//! Lock word encoding:
//! a single `usize` represents the full lock state:
//!
//!   0              : unlocked
//!   1..=MAX_READERS: that many readers currently hold a read guard
//!   WRITER_BIT     : one writer holds a write guard (no readers allowed)
//!   WRITER_BIT + n : one writer holds a write guard, n pending readers are
//!                    queued via `deferred_write` and are waiting to be woken
//!                    (n counts deferred-write guards currently alive, each of
//!                    which represents one future reader)
//!
//! WRITER_BIT is the highest bit. Reader counts occupy the lower bits.
//! Reader count can therefore reach WRITER_BIT - 1 before overflowing into the
//! writer bit, which is large and safe to treat as unreachable.

use core::{
    cell::UnsafeCell,
    hint,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

const WRITER_BIT: usize = 1 << (usize::BITS - 1);

// Maximum number of simultaneous readers. Attempting to acquire a read guard
// beyond this limit will spin as though we're attempting to lock a Mutex for reading.
const MAX_READERS: usize = WRITER_BIT - 1;

pub struct SpinRwLock<T: ?Sized> {
    lock: AtomicUsize,
    data: UnsafeCell<T>,
}

/// Holds a `&T`. Multiple read guards may coexist.
pub struct SpinRwLockReadGuard<'lock, T: 'lock + ?Sized> {
    lock: &'lock AtomicUsize,
    data: &'lock T,
}

/// Holds an `&mut T`. No other guard may coexist.
pub struct SpinRwLockWriteGuard<'lock, T: 'lock + ?Sized> {
    lock: &'lock AtomicUsize,
    data: &'lock mut T,
}

/// A *deferred* write guard. Like a read guard it increments the reader count,
/// which prevents new write guards from being acquired, but it does NOT yet give
/// access to `&mut T`. Once all other readers have released their guards you
/// can atomically upgrade this into a full `RwLockWriteGuard` via `write()` /
/// `try_write()`.
///
/// The raw `*mut T` is kept instead of `&mut T` because we must not assert
/// exclusive ownership until the upgrade succeeds.
pub struct SpinRwLockDeferredGuard<'lock, T: 'lock + ?Sized> {
    lock: &'lock AtomicUsize,
    data: *mut T,
}

/// # Safety
/// `T: Send` is sufficient: the borrow checker prevents moving a locked
/// `RwLock` across hart boundaries.
unsafe impl<T: ?Sized + Send> Send for SpinRwLock<T> {}

/// # Safety
/// `T: Send + Sync` is required because `RwLock` can produce `&mut T`.
unsafe impl<T: ?Sized + Send + Sync> Sync for SpinRwLock<T> {}

/// # Safety
/// `RwLockReadGuard<T>` is equivalent to `&T`.
unsafe impl<T: ?Sized + Sync> Send for SpinRwLockReadGuard<'_, T> {}
/// # Safety
/// `RwLockReadGuard<T>` is equivalent to `&T`.
unsafe impl<T: ?Sized + Sync> Sync for SpinRwLockReadGuard<'_, T> {}

/// # Safety
/// `RwLockWriteGuard<T>` is equivalent to `&mut T`.
/// `&RwLockWriteGuard<T>` is equivalent to `&&mut T`, i.e. `&T`.
unsafe impl<T: ?Sized + Send + Sync> Send for SpinRwLockWriteGuard<'_, T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for SpinRwLockWriteGuard<'_, T> {}

/// # Safety
/// `RwLockDeferredGuard` can be upgraded to `RwLockWriteGuard`, so it requires
/// the same bounds as `RwLockWriteGuard`.
unsafe impl<T: ?Sized + Send + Sync> Send for SpinRwLockDeferredGuard<'_, T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for SpinRwLockDeferredGuard<'_, T> {}

impl<T> SpinRwLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            lock: AtomicUsize::new(0),
            data: UnsafeCell::new(value),
        }
    }
}

impl<T: Default> Default for SpinRwLock<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: ?Sized> SpinRwLock<T> {
    pub fn reader_count(&self) -> usize {
        let word = self.lock.load(Ordering::Relaxed);
        word & !WRITER_BIT
    }

    pub fn is_write_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed) & WRITER_BIT != 0
    }
}

impl<T: ?Sized> SpinRwLock<T> {
    /// Try to acquire a read guard without spinning.
    ///
    /// Succeeds when no writer holds the lock. The uncontended path is
    /// a single `fetch_add` followed by a cheap branch.
    pub fn try_read(&self) -> Option<SpinRwLockReadGuard<'_, T>> {
        let prev = self.lock.fetch_add(1, Ordering::Acquire);

        if prev & WRITER_BIT == 0 && prev < MAX_READERS {
            let data = unsafe {
                // SAFETY: WRITER_BIT was clear, so no `&mut T` exists.
                // Our reader count increment serialises us with any writer
                // that tries to set WRITER_BIT after us.
                &*self.data.get()
            };
            Some(SpinRwLockReadGuard {
                lock: &self.lock,
                data,
            })
        } else {
            // A writer is present (or the reader count is astronomically large).
            self.lock.fetch_sub(1, Ordering::Release);
            None
        }
    }

    /// Try to acquire a write guard without spinning.
    ///
    /// Succeeds only when the lock word is exactly 0 (no readers, no writer).
    pub fn try_write(&self) -> Option<SpinRwLockWriteGuard<'_, T>> {
        self.lock
            .compare_exchange(0, WRITER_BIT, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| {
                let data = unsafe {
                    // SAFETY: CAS from 0 means no readers and no other writer.
                    &mut *self.data.get()
                };
                SpinRwLockWriteGuard {
                    lock: &self.lock,
                    data,
                }
            })
    }

    /// Try to acquire a deferred-write guard without spinning.
    ///
    /// A deferred-write guard increments the reader count (like a read guard)
    /// and can later be upgraded to a full write guard once all concurrent
    /// readers have released. Acquiring one blocks any new *writers* from
    /// entering, but does not block ongoing or new *readers*.
    ///
    /// Fails if a writer already holds the lock.
    pub fn try_deferred_write(&self) -> Option<SpinRwLockDeferredGuard<'_, T>> {
        // Same optimistic fetch_add strategy as try_read.
        let prev = self.lock.fetch_add(1, Ordering::Acquire);

        if prev & WRITER_BIT == 0 && prev < MAX_READERS {
            Some(SpinRwLockDeferredGuard {
                lock: &self.lock,
                data: self.data.get(),
            })
        } else {
            self.lock.fetch_sub(1, Ordering::Release);
            None
        }
    }

    /// Spin until a read guard is available.
    pub fn read(&self) -> SpinRwLockReadGuard<'_, T> {
        loop {
            if let Some(guard) = self.try_read() {
                return guard;
            }
            while self.lock.load(Ordering::Relaxed) & WRITER_BIT != 0 {
                hint::spin_loop();
            }
        }
    }

    /// Spin until a write guard is available.
    pub fn write(&self) -> SpinRwLockWriteGuard<'_, T> {
        loop {
            if let Some(guard) = self.try_write() {
                return guard;
            }
            while self.lock.load(Ordering::Relaxed) != 0 {
                hint::spin_loop();
            }
        }
    }

    /// Spin until a deferred-write guard is available.
    pub fn deferred_write(&self) -> SpinRwLockDeferredGuard<'_, T> {
        loop {
            if let Some(guard) = self.try_deferred_write() {
                return guard;
            }
            while self.lock.load(Ordering::Relaxed) & WRITER_BIT != 0 {
                hint::spin_loop();
            }
        }
    }
}

impl<'lock, T: ?Sized> SpinRwLockDeferredGuard<'lock, T> {
    /// Attempt to upgrade to a write guard in one step.
    ///
    /// Succeeds only if this is the *sole* remaining reader (i.e. the reader
    /// count, which includes this guard's own +1, equals exactly 1).  When
    /// successful the guard is consumed and WRITER_BIT is set atomically.
    ///
    /// Returns `Err(self)` if other readers are still active, leaving the
    /// deferred guard alive so the caller can retry.
    #[must_use = "if Err, the deferred guard is returned and must not be forgotten"]
    pub fn try_write(self) -> Result<SpinRwLockWriteGuard<'lock, T>, Self> {
        let mut mself = ManuallyDrop::new(self);
        // We hold exactly one reader slot. The CAS succeeds iff the count is 1
        // and WRITER_BIT is clear, meaning we are the only reader.
        // We swap in WRITER_BIT (reader count 0, writer present).
        mself
            .lock
            .compare_exchange(1, WRITER_BIT, Ordering::Acquire, Ordering::Relaxed)
            .map(|_| {
                let data = unsafe {
                    // SAFETY: CAS confirmed we were the sole reader; WRITER_BIT
                    // is now set, blocking all other acquisitions.
                    &mut *mself.data
                };
                SpinRwLockWriteGuard {
                    lock: mself.lock,
                    data,
                }
            })
            .map_err(|_| ManuallyDrop::into_inner(mself))
    }

    pub fn write(mut self) -> SpinRwLockWriteGuard<'lock, T> {
        loop {
            match self.try_write() {
                Ok(guard) => return guard,
                Err(deferred) => {
                    self = deferred;
                    while self.lock.load(Ordering::Relaxed) & !WRITER_BIT > 1 {
                        hint::spin_loop();
                    }
                }
            }
        }
    }
}

impl<'lock, T: ?Sized> SpinRwLockWriteGuard<'lock, T> {
    /// Downgrade the write guard to a read guard without releasing the lock.
    ///
    /// Atomically clears WRITER_BIT and sets the reader count to 1, so no
    /// window exists during which the lock appears to be free.
    pub fn read(self) -> SpinRwLockReadGuard<'lock, T> {
        let mself = ManuallyDrop::new(self);
        mself.lock.store(1, Ordering::Release);
        let data = unsafe {
            // SAFETY: We just set reader count to 1; the old &mut T becomes &T.
            &*(mself.data as *const T)
        };
        SpinRwLockReadGuard {
            lock: mself.lock,
            data,
        }
    }
}

impl<'lock, T: ?Sized> SpinRwLockReadGuard<'lock, T> {
    /// Reinterpret the inner reference as type `Q`.
    ///
    /// # Safety
    /// `T` and `Q` must have the same size and be bit-compatible.
    pub unsafe fn reinterpret<Q>(self) -> SpinRwLockReadGuard<'lock, Q> {
        let mself = ManuallyDrop::new(self);
        let lock = mself.lock;
        let data = unsafe { core::mem::transmute_copy(&mself.data) };
        SpinRwLockReadGuard { lock, data }
    }
}

impl<'lock, T: ?Sized> SpinRwLockWriteGuard<'lock, T> {
    /// Reinterpret the inner reference as type `Q`.
    ///
    /// # Safety
    /// `T` and `Q` must have the same size and be bit-compatible.
    pub unsafe fn reinterpret<Q>(self) -> SpinRwLockWriteGuard<'lock, Q> {
        let mself = ManuallyDrop::new(self);
        let lock = mself.lock;
        let data = unsafe { core::mem::transmute_copy(&mself.data) };
        SpinRwLockWriteGuard { lock, data }
    }
}

impl<'lock, T: ?Sized> SpinRwLockDeferredGuard<'lock, T> {
    /// Reinterpret the inner pointer as type `Q`.
    ///
    /// # Safety
    /// `T` and `Q` must have the same size and be bit-compatible.
    pub unsafe fn reinterpret<Q>(self) -> SpinRwLockDeferredGuard<'lock, Q> {
        let mself = ManuallyDrop::new(self);
        let lock = mself.lock;
        let data = unsafe { core::mem::transmute_copy(&mself.data) };
        SpinRwLockDeferredGuard { lock, data }
    }
}

impl<T: ?Sized> Deref for SpinRwLockReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<T: ?Sized> Deref for SpinRwLockWriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<T: ?Sized> DerefMut for SpinRwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<T: ?Sized> Deref for SpinRwLockDeferredGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe {
            // SAFETY: The lock word has at least our own reader count, so no
            // writer can hold `&mut T` concurrently.
            &*self.data
        }
    }
}

impl<T: ?Sized> Drop for SpinRwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.fetch_sub(1, Ordering::Release);
    }
}

impl<T: ?Sized> Drop for SpinRwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.fetch_and(!WRITER_BIT, Ordering::Release);
    }
}

impl<T: ?Sized> Drop for SpinRwLockDeferredGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.fetch_sub(1, Ordering::Release);
    }
}
