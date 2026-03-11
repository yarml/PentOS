use core::{
    cell::UnsafeCell,
    hint,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

type AtomicWord = AtomicUsize;
type Word = usize;

pub struct SpinRwLock<T: ?Sized> {
    lock: AtomicWord,
    data: UnsafeCell<T>,
}

pub struct SpinRwLockReadGuard<'lock, T: 'lock + ?Sized> {
    lock: &'lock AtomicWord,
    data: &'lock T,
}

pub struct SpinRwLockWriteGuard<'lock, T: 'lock + ?Sized> {
    lock: &'lock AtomicWord,
    data: &'lock mut T,
}

pub struct SpinRwLockDeferredGuard<'lock, T: 'lock + ?Sized> {
    lock: &'lock AtomicWord,
    data: *mut T, // Keeping a mutable pointer to be able to change to writer, not keeping a &mut T to respect aliasing rules
}

// LSB 2 bits of lock stores the state
pub enum RwLockState {
    Open,
    ReadOnly,
    WriteOnly,
    Locked,
}

/// # Safety
/// Borrow checker will stop moves across thread boundaries if there is any reader
/// or writer. So `T: Send` should allow `RwLock<T>: Send`
unsafe impl<T: ?Sized + Send> Send for SpinRwLock<T> {}

/// # Safety
/// Requiring T: Send in addition to T: Sync because RwLock can give mutable
/// references to T.
unsafe impl<T: ?Sized + Send + Sync> Sync for SpinRwLock<T> {}

/// # Safety
/// `RwLockReadGuard<T>` is equivalent to &T
unsafe impl<T: ?Sized + Sync> Send for SpinRwLockReadGuard<'_, T> {}
/// # Safety
/// `RwLockReadGuard<T>` is equivalent to &T
unsafe impl<T: ?Sized + Sync> Sync for SpinRwLockReadGuard<'_, T> {}

/// # Safety
/// `RwLockWriteuard<T>` is equivalent to &mut T
/// `&RwLockWriteuard<T>` is equivalent to &&mut T, which is equivalent to &T
unsafe impl<T: ?Sized + Send + Sync> Send for SpinRwLockWriteGuard<'_, T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for SpinRwLockWriteGuard<'_, T> {}

/// # Safety
/// Alothough RwLockDeferredGuard is like RwLockReadGuard, it can be converted
/// to a RwLockWriteGuard, as such it has the same T: Send + Sync conditions.
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

impl<T: ?Sized> SpinRwLock<T> {
    pub fn reader_count(&self) -> Word {
        reader_count_from(self.lock.load(Ordering::Relaxed))
    }
    pub fn state(&self) -> RwLockState {
        // No panic: 2 bits taken always gives valid RwLockState
        state_from(self.lock.load(Ordering::Relaxed))
    }
}

impl<T: ?Sized> SpinRwLock<T> {
    pub fn try_read(&self) -> Option<SpinRwLockReadGuard<'_, T>> {
        if self
            .lock
            .fetch_update(Ordering::Acquire, Ordering::Relaxed, |lock| {
                let state = state_from(lock);
                let readers = reader_count_from(lock);
                if matches!(state, RwLockState::Open | RwLockState::ReadOnly) {
                    Some(make_lock(state, readers + 1))
                } else {
                    None
                }
            })
            .is_ok()
        {
            let data = unsafe {
                // SAFETY: Pointer is valid since we host the data
                // Aliasing rules are checked on runtime
                &*self.data.get()
            };
            Some(SpinRwLockReadGuard {
                lock: &self.lock,
                data,
            })
        } else {
            None
        }
    }
    pub fn try_write(&self) -> Option<SpinRwLockWriteGuard<'_, T>> {
        if self
            .lock
            .fetch_update(Ordering::Acquire, Ordering::Relaxed, |lock| {
                let state = state_from(lock);
                let readers = reader_count_from(lock);
                if matches!(state, RwLockState::Open | RwLockState::WriteOnly) && readers == 0 {
                    Some(make_lock(RwLockState::Locked, 0))
                } else {
                    None
                }
            })
            .is_ok()
        {
            let data = unsafe {
                // SAFETY: Pointer is valid since we host the data
                // Aliasing rules are checked on runtime
                &mut *self.data.get()
            };
            Some(SpinRwLockWriteGuard {
                lock: &self.lock,
                data,
            })
        } else {
            None
        }
    }
    pub fn try_deferred_write(&self) -> Option<SpinRwLockDeferredGuard<'_, T>> {
        if self
            .lock
            .fetch_update(Ordering::Acquire, Ordering::Relaxed, |lock| {
                let state = state_from(lock);
                let readers = reader_count_from(lock);
                if matches!(state, RwLockState::Open) {
                    Some(make_lock(RwLockState::WriteOnly, readers + 1))
                } else {
                    None
                }
            })
            .is_ok()
        {
            let data = self.data.get();
            Some(SpinRwLockDeferredGuard {
                lock: &self.lock,
                data,
            })
        } else {
            None
        }
    }
}

impl<T: ?Sized> SpinRwLock<T> {
    pub fn read(&self) -> SpinRwLockReadGuard<'_, T> {
        loop {
            if let Some(guard) = self.try_read() {
                return guard;
            }
            hint::spin_loop();
        }
    }
    pub fn write(&self) -> SpinRwLockWriteGuard<'_, T> {
        loop {
            if let Some(guard) = self.try_write() {
                return guard;
            }
            hint::spin_loop();
        }
    }
    pub fn deferred_write(&self) -> SpinRwLockDeferredGuard<'_, T> {
        loop {
            if let Some(guard) = self.try_deferred_write() {
                return guard;
            }
            hint::spin_loop();
        }
    }
}

impl<T: Default> Default for SpinRwLock<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<'lock, T: ?Sized> SpinRwLockDeferredGuard<'lock, T> {
    #[must_use = "check the guard was upgraded or not, otherwise it will drop"]
    pub fn try_write(self) -> Result<SpinRwLockWriteGuard<'lock, T>, SpinRwLockDeferredGuard<'lock, T>> {
        let mut mself = ManuallyDrop::new(self);
        if mself
            .lock
            .fetch_update(Ordering::Acquire, Ordering::Relaxed, |lock| {
                let readers = reader_count_from(lock);
                if readers == 0 {
                    Some(make_lock(RwLockState::Locked, 0))
                } else {
                    None
                }
            })
            .is_ok()
        {
            let data = unsafe { &mut *mself.data };
            let lock = mself.lock;
            Ok(SpinRwLockWriteGuard { lock, data })
        } else {
            Err(ManuallyDrop::into_inner(mself))
        }
    }
}

impl<'lock, T: ?Sized> SpinRwLockDeferredGuard<'lock, T> {
    pub fn write(mut self) -> SpinRwLockWriteGuard<'lock, T> {
        loop {
            match self.try_write() {
                Ok(write_guard) => return write_guard,
                Err(deferred_guard) => self = deferred_guard,
            }
            hint::spin_loop();
        }
    }
}

impl<'lock, T: ?Sized> SpinRwLockWriteGuard<'lock, T> {
    pub fn read(self) -> SpinRwLockReadGuard<'lock, T> {
        let mself = ManuallyDrop::new(self);
        unsafe {
            // SAFETY: We always return Some in the set function.
            mself
                .lock
                .fetch_update(Ordering::Acquire, Ordering::Relaxed, |_| {
                    Some(make_lock(RwLockState::Open, 1))
                })
                .unwrap_unchecked();
        }
        let lock = mself.lock;
        let data = unsafe {
            // SAFETY: No write guard can race a mutable access to T since we already set the reader count to 1
            &*(mself.data as *const T)
        };
        SpinRwLockReadGuard { lock, data }
    }
    // Wouldn't make sense to convert a write guard to a deferred guard, since it will
    // be an exclusive reader inhibiting any further readers from entering the lock
    // The write guard already displays that behaviour.
}

impl<'lock, T: ?Sized> SpinRwLockReadGuard<'lock, T> {
    /// # Safety
    /// Must guarentee that T and Q have the same size and are bit compatible
    pub unsafe fn reinterpret<Q>(self) -> SpinRwLockReadGuard<'lock, Q> {
        let mself = ManuallyDrop::new(self);
        let lock = mself.lock;
        let data = unsafe {
            // SAFETY: Guarenteed if T and Q have the same size and are bit compatible
            core::mem::transmute_copy(&mself.data)
        };
        SpinRwLockReadGuard { lock, data }
    }
}

impl<'lock, T: ?Sized> SpinRwLockWriteGuard<'lock, T> {
    /// # Safety
    /// Must guarentee that T and Q have the same size and are bit compatible
    pub unsafe fn reinterpret<Q>(self) -> SpinRwLockWriteGuard<'lock, Q> {
        let mself = ManuallyDrop::new(self);
        let lock = mself.lock;
        let data = unsafe {
            // SAFETY: Guarenteed if T and Q have the same size and are bit compatible
            core::mem::transmute_copy(&mself.data)
        };
        SpinRwLockWriteGuard { lock, data }
    }
}

impl<'lock, T: ?Sized> SpinRwLockDeferredGuard<'lock, T> {
    /// # Safety
    /// Must guarentee that T and Q have the same size and are bit compatible
    pub unsafe fn reinterpret<Q>(self) -> SpinRwLockDeferredGuard<'lock, Q> {
        let mself = ManuallyDrop::new(self);
        let lock = mself.lock;
        let data = unsafe {
            // SAFETY: Guarenteed if T and Q have the same size and are bit compatible
            core::mem::transmute_copy(&mself.data)
        };
        SpinRwLockDeferredGuard { lock, data }
    }
}

impl<T: ?Sized> Deref for SpinRwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T: ?Sized> Deref for SpinRwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T: ?Sized> DerefMut for SpinRwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<T: ?Sized> Deref for SpinRwLockDeferredGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            // SAFETY: Data is hosted within a RwLock, so pointer is valid
            // Aliasing rules are runtime guarenteed with said lock
            &*self.data
        }
    }
}

impl<T: ?Sized> Drop for SpinRwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: Always retuning Some from the set function.
            self.lock
                .fetch_update(Ordering::Release, Ordering::Relaxed, |lock| {
                    let state = state_from(lock);
                    let readers = reader_count_from(lock);
                    Some(make_lock(state, readers - 1))
                })
                .unwrap_unchecked();
        }
    }
}

impl<T: ?Sized> Drop for SpinRwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: Always retuning Some from the set function.
            self.lock
                .fetch_update(Ordering::Release, Ordering::Relaxed, |_| {
                    Some(make_lock(RwLockState::Open, 0))
                })
                .unwrap_unchecked();
        }
    }
}

impl<T: ?Sized> Drop for SpinRwLockDeferredGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: Always retuning Some from the set function.
            self.lock
                .fetch_update(Ordering::Release, Ordering::Relaxed, |lock| {
                    let readers = reader_count_from(lock);
                    Some(make_lock(RwLockState::Open, readers - 1))
                })
                .unwrap_unchecked();
        }
    }
}

// TODO: make a proc macro for this
impl TryFrom<Word> for RwLockState {
    type Error = Word;

    fn try_from(value: Word) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Open),
            1 => Ok(Self::ReadOnly),
            2 => Ok(Self::WriteOnly),
            3 => Ok(Self::Locked),
            other => Err(other),
        }
    }
}
impl From<RwLockState> for Word {
    fn from(value: RwLockState) -> Self {
        match value {
            RwLockState::Open => 0,
            RwLockState::ReadOnly => 1,
            RwLockState::WriteOnly => 2,
            RwLockState::Locked => 3,
        }
    }
}

fn state_from(lock: Word) -> RwLockState {
    RwLockState::try_from(lock & 0x3).unwrap()
}
const fn reader_count_from(lock: Word) -> Word {
    lock >> 2
}
fn make_lock(state: RwLockState, readers: Word) -> Word {
    Word::from(state) | readers << 2
}
