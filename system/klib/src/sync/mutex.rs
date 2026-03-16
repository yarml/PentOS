use {
    alloc::{collections::vec_deque::VecDeque, sync::Arc},
    core::{
        cell::UnsafeCell,
        ops::{Deref, DerefMut},
        pin::Pin,
        sync::atomic::{AtomicBool, AtomicU8, Ordering},
        task::{Context, Poll, Waker},
    },
    spinlocks::mutex::SpinMutex,
    x64::interrupts,
};

const QUEUE_STATE_QUEUED: u8 = 0;
const QUEUE_STATE_WOKEN: u8 = 1;
const QUEUE_STATE_CANCELLED: u8 = 2;

pub struct AsyncMutex<T: ?Sized> {
    locked: AtomicBool,
    waiters: SpinMutex<VecDeque<WakerSlot>>,
    data: UnsafeCell<T>,
}

/// Returned by [`AsyncMutex::lock`]. Holds the mutex until dropped, then
/// wakes the next waiter in line.
pub struct AsyncMutexGuard<'mutex, T: 'mutex + ?Sized> {
    mutex: &'mutex AsyncMutex<T>,
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
pub struct AsyncMutexLockFuture<'mutex, T: 'mutex + ?Sized> {
    mutex: &'mutex AsyncMutex<T>,
    queue_state: Option<Arc<AtomicU8>>,
}

struct WakerSlot {
    waker: Waker,
    queue_state: Arc<AtomicU8>,
}

/// # Safety
/// `T: Send` is sufficient for the same reason as `Mutex<T>`: the borrow
/// checker prevents moving a locked `AsyncMutex` across hart boundaries.
unsafe impl<T: ?Sized + Send> Send for AsyncMutex<T> {}

/// # Safety
/// `T: Send` is required (not just `Sync`) because `AsyncMutex` can produce
/// `&mut T`.
unsafe impl<T: ?Sized + Send> Sync for AsyncMutex<T> {}

/// # Safety
/// `AsyncMutexGuard<T>` is morally `&mut T`: sending it to another hart is safe
/// when `T: Send`.
unsafe impl<T: ?Sized + Send> Send for AsyncMutexGuard<'_, T> {}

/// # Safety
/// `&AsyncMutexGuard<T>` is morally `&&mut T`, i.e. `&T`.  Sharing requires
/// `T: Sync`.
unsafe impl<T: ?Sized + Sync> Sync for AsyncMutexGuard<'_, T> {}

/// # Safety
/// The future holds only a shared reference to the mutex and a `bool`; both
/// are safe to send.
unsafe impl<T: ?Sized + Send> Send for AsyncMutexLockFuture<'_, T> {}
unsafe impl<T: ?Sized + Send> Sync for AsyncMutexLockFuture<'_, T> {}

impl<T> AsyncMutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            waiters: SpinMutex::new(VecDeque::new()),
            data: UnsafeCell::new(data),
            locked: AtomicBool::new(false),
        }
    }
}

impl<T: Default> Default for AsyncMutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: ?Sized> AsyncMutex<T> {
    pub fn lock(&self) -> AsyncMutexLockFuture<'_, T> {
        AsyncMutexLockFuture {
            mutex: self,
            queue_state: None,
        }
    }

    pub fn wake_next(&self) {
        interrupts::with_disabled(|| {
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
}

impl<'mutex, T: ?Sized> Future for AsyncMutexLockFuture<'mutex, T> {
    type Output = AsyncMutexGuard<'mutex, T>;

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

impl<'mutex, T: ?Sized> Drop for AsyncMutexLockFuture<'mutex, T> {
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

impl<T: ?Sized> Deref for AsyncMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data
    }
}

impl<T: ?Sized> DerefMut for AsyncMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<T: ?Sized> Drop for AsyncMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.wake_next();
    }
}
