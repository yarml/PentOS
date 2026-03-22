#![no_std]
extern crate alloc;

use {
    alloc::{collections::vec_deque::VecDeque, sync::Arc},
    core::{
        cell::UnsafeCell,
        hint,
        marker::PhantomData,
        ops::{Deref, DerefMut},
        pin::Pin,
        sync::atomic::{AtomicBool, AtomicU8, Ordering},
        task::{Context, Poll, Waker},
    },
    spinlocks::mutex::SpinMutex,
    x64::interrupts,
};

pub trait CriticalSection {
    fn with<R>(f: impl FnOnce() -> R) -> R;
}

pub struct NoCriticalSection;
impl CriticalSection for NoCriticalSection {
    fn with<R>(f: impl FnOnce() -> R) -> R {
        f()
    }
}

pub struct InterruptDisabled;
impl CriticalSection for InterruptDisabled {
    fn with<R>(f: impl FnOnce() -> R) -> R {
        x64::interrupts::with_disabled(f)
    }
}

#[cfg(target_os = "none")]
pub type DefaultCriticalSection = InterruptDisabled;

#[cfg(not(target_os = "none"))]
pub type DefaultCriticalSection = NoCriticalSection;

const QUEUE_STATE_QUEUED: u8 = 0;
const QUEUE_STATE_WOKEN: u8 = 1;
const QUEUE_STATE_CANCELLED: u8 = 2;

pub struct AsyncMutex<T: ?Sized, CS: CriticalSection = DefaultCriticalSection> {
    locked: AtomicBool,
    waiters: SpinMutex<VecDeque<WakerSlot>>,
    _phantom: PhantomData<CS>,
    data: UnsafeCell<T>,
}

/// Returned by [`AsyncMutex::lock`]. Holds the mutex until dropped, then
/// wakes the next waiter in line.
pub struct AsyncMutexGuard<'mutex, T: 'mutex + ?Sized, CS: CriticalSection = DefaultCriticalSection> {
    mutex: &'mutex AsyncMutex<T, CS>,
    data: &'mutex mut T,
}

/// The future returned by [`AsyncMutex::lock`].
///
/// On the first `poll` it tries to acquire the lock immediately. If the lock
/// is held by someone else it registers the current task's waker and returns
/// `Poll::Pending`. Once the previous holder drops its guard it calls
/// `waker.wake()`, which re-schedules this task. On the next `poll` the
/// lock is guaranteed to be free (no other task can have taken it in between,
/// because the waker queue is FIFO and the guard drops only one waker at a
/// time) and the guard also does not mark the mutex as unlocked for a
/// task to skip the queue and hold the lock before everyone else.
pub struct AsyncMutexLockFuture<'mutex, T: 'mutex + ?Sized, CS: CriticalSection = DefaultCriticalSection>
{
    mutex: &'mutex AsyncMutex<T, CS>,
    queue_state: Option<Arc<AtomicU8>>,
}

struct WakerSlot {
    waker: Waker,
    queue_state: Arc<AtomicU8>,
}

/// # Safety
/// `T: Send` is sufficient for the same reason as `Mutex<T>`: the borrow
/// checker prevents moving a locked `AsyncMutex` across hart boundaries.
unsafe impl<T: ?Sized + Send, CS: CriticalSection> Send for AsyncMutex<T, CS> {}

/// # Safety
/// `T: Send` is required (not just `Sync`) because `AsyncMutex` can produce
/// `&mut T`.
unsafe impl<T: ?Sized + Send, CS: CriticalSection> Sync for AsyncMutex<T, CS> {}

/// # Safety
/// `AsyncMutexGuard<T>` is morally `&mut T`: sending it to another hart is safe
/// when `T: Send`.
unsafe impl<T: ?Sized + Send, CS: CriticalSection> Send for AsyncMutexGuard<'_, T, CS> {}

/// # Safety
/// `&AsyncMutexGuard<T>` is morally `&&mut T`, i.e. `&T`.  Sharing requires
/// `T: Sync`.
unsafe impl<T: ?Sized + Sync, CS: CriticalSection> Sync for AsyncMutexGuard<'_, T, CS> {}

/// # Safety
/// The future holds only a shared reference to the mutex and a `bool`; both
/// are safe to send.
unsafe impl<T: ?Sized + Send, CS: CriticalSection> Send for AsyncMutexLockFuture<'_, T, CS> {}
unsafe impl<T: ?Sized + Send, CS: CriticalSection> Sync for AsyncMutexLockFuture<'_, T, CS> {}

impl<T, CS: CriticalSection> AsyncMutex<T, CS> {
    pub const fn new(data: T) -> Self {
        Self {
            waiters: SpinMutex::new(VecDeque::new()),
            data: UnsafeCell::new(data),
            locked: AtomicBool::new(false),
            _phantom: PhantomData,
        }
    }
}

impl<T: Default, CS: CriticalSection> Default for AsyncMutex<T, CS> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: ?Sized, CS: CriticalSection> AsyncMutex<T, CS> {
    pub fn lock(&self) -> AsyncMutexLockFuture<'_, T, CS> {
        AsyncMutexLockFuture {
            mutex: self,
            queue_state: None,
        }
    }

    pub fn wake_next(&self) {
        CS::with(|| {
            let mark_unlocked = || {
                self.locked.store(false, Ordering::Release);
            };
            let mut waiters = self.waiters.lock();
            if waiters.is_empty() {
                mark_unlocked();
            } else {
                loop {
                    let slot = waiters.pop_front();
                    if slot.is_none() {
                        mark_unlocked();
                        break;
                    }
                    let slot = slot.unwrap();
                    if slot
                        .queue_state
                        .compare_exchange(
                            QUEUE_STATE_QUEUED,
                            QUEUE_STATE_WOKEN,
                            Ordering::Release,
                            Ordering::Relaxed,
                        )
                        .is_ok()
                    {
                        slot.waker.wake();
                        break;
                    }
                }
            }
        })
    }

    pub fn lock_sync(&self) -> AsyncMutexGuard<'_, T, CS> {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            hint::spin_loop();
        }
        let data = unsafe {
            // SAFETY: invariant of self.queue, or the CAS
            &mut *self.data.get()
        };
        AsyncMutexGuard { mutex: self, data }
    }
}

impl<'mutex, T: ?Sized, CS: CriticalSection> Future for AsyncMutexLockFuture<'mutex, T, CS> {
    type Output = AsyncMutexGuard<'mutex, T, CS>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        interrupts::with_disabled(|| {
            let mut waiters = this.mutex.waiters.lock();
            let queue_state = this
                .queue_state
                .as_ref()
                .map(|qs| qs.load(Ordering::Acquire));

            if queue_state == Some(QUEUE_STATE_WOKEN)
                || (waiters.is_empty()
                    && this
                        .mutex
                        .locked
                        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                        .is_ok())
            {
                let data = unsafe {
                    // SAFETY: invariant of self.queue, or the CAS
                    &mut *this.mutex.data.get()
                };
                return Poll::Ready(AsyncMutexGuard {
                    mutex: this.mutex,
                    data,
                });
            }

            let queue_state = Arc::new(AtomicU8::new(QUEUE_STATE_QUEUED));
            this.queue_state = Some(queue_state.clone());
            waiters.push_back(WakerSlot {
                waker: cx.waker().clone(),
                queue_state,
            });
            Poll::Pending
        })
    }
}

impl<'mutex, T: ?Sized, CS: CriticalSection> Drop for AsyncMutexLockFuture<'mutex, T, CS> {
    fn drop(&mut self) {
        if let Some(queue_state) = self.queue_state.clone()
            && queue_state
                .compare_exchange(
                    QUEUE_STATE_QUEUED,
                    QUEUE_STATE_CANCELLED,
                    Ordering::Release,
                    Ordering::Relaxed,
                )
                .is_ok()
        {}
    }
}

impl<T: ?Sized, CS: CriticalSection> Deref for AsyncMutexGuard<'_, T, CS> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data
    }
}

impl<T: ?Sized, CS: CriticalSection> DerefMut for AsyncMutexGuard<'_, T, CS> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<T: ?Sized, CS: CriticalSection> Drop for AsyncMutexGuard<'_, T, CS> {
    fn drop(&mut self) {
        self.mutex.wake_next();
    }
}
