use core::{
    borrow::BorrowMut,
    cell::UnsafeCell,
    hint,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Mutex<T: ?Sized> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct MutexGuard<'mutex, T: 'mutex + ?Sized> {
    lock: &'mutex AtomicBool,
    data: &'mutex mut T,
}

/// # Safety
/// With `T: Send`, borrow checker will prevent any move when Mutex is locked
/// There is no issue moving an unlocked Mutex between harts.
/// The `UnsafeCell<T>` within the `Mutex<T>` is only accessible to 1 hart at most
/// at any point in time.
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

/// # Safety
/// The `UnsafeCell<T>` is not directly accessible to harts unless they lock the entire
/// `Mutex<T>`.
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

/// # Safety
/// No problem dropping a `MutexGuard<T>` in a hart it was not locked in, or
/// accessing the data behind the mutex.
unsafe impl<T: ?Sized + Send> Send for MutexGuard<'_, T> {}

/// # Safety
/// Borrow checker will prevent any mutable aliasing.
unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    pub fn lock(&self) -> MutexGuard<'_, T> {
        loop {
            if let Some(guard) = self.try_lock() {
                return guard;
            }

            while self.is_locked() {
                hint::spin_loop();
            }
        }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            let data = unsafe {
                // # Safety
                // We host the data, so we know it is in a valid memory location
                // The lock also guarentees exclusivity of the unique reference
                self.data.get().as_mut_unchecked()
            };
            Some(MutexGuard {
                lock: &self.lock,
                data,
            })
        } else {
            None
        }
    }

    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }
}

impl<'lock, T: ?Sized> MutexGuard<'lock, T> {
    pub fn map_borrow<U: ?Sized>(self) -> MutexGuard<'lock, U>
    where
        T: BorrowMut<U>,
    {
        let mut mself = ManuallyDrop::new(self);
        let mself = unsafe {
            // SAFETY: ManullyDrop<T> is guarenteed to have the same layout as T
            &mut *(&mut mself as *mut _ as *mut Self)
        };

        let lock = mself.lock;
        let orig_data: &mut T = mself.data;
        let borrowed_data: &mut U = orig_data.borrow_mut();

        MutexGuard {
            lock,
            data: borrowed_data,
        }
    }
}

impl<'lock, T: ?Sized> Drop for MutexGuard<'lock, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}

impl<'lock, T: ?Sized> Deref for MutexGuard<'lock, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'lock, T: ?Sized> DerefMut for MutexGuard<'lock, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}
