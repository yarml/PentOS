use {
    alloc::collections::vec_deque::VecDeque,
    core::{
        cell::UnsafeCell,
        ops::{Deref, DerefMut},
        pin::Pin,
        sync::atomic::{AtomicBool, Ordering},
        task::{Context, Poll, Waker},
    },
    spinlocks::mutex::SpinMutex,
    x64::interrupts,
};

pub struct AsyncMutex<T: ?Sized> {
    locked: AtomicBool,
    waiters: SpinMutex<VecDeque<Waker>>,
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
    queued: bool,
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
            queued: false,
        }
    }
}

impl<'mutex, T: ?Sized> Future for AsyncMutexLockFuture<'mutex, T> {
    type Output = AsyncMutexGuard<'mutex, T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        interrupts::with_disabled(|| {
            let mut waiters = this.mutex.waiters.lock();

            if this.queued
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

            this.queued = true;
            waiters.push_back(cx.waker().clone());
            Poll::Pending
        })
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
        interrupts::with_disabled(|| {
            let mut waiters = self.mutex.waiters.lock();
            if waiters.is_empty() {
                self.mutex.locked.store(false, Ordering::Release);
            } else {
                // It already is true, we're setting it again to tru just for the Release ordering
                // In case we have waiters, we never set locked to false, we directly pass it
                // to the next task waiting for it.
                self.mutex.locked.store(true, Ordering::Release);
                waiters.pop_front().unwrap().wake();
            }
        });
    }
}
