//! Adapted from crossbeam's `ArrayQueue`, which is itself based on Dmitry Vyukov's bounded MPMC
//! queue. Unlike `ArrayQueue`, this implementation uses a static inline buffer (no `alloc`) and
//! takes the capacity as a const generic parameter `N`.
//!
//! Original source:
//!   - <https://github.com/crossbeam-rs/crossbeam/blob/master/crossbeam-queue/src/array_queue.rs>
//!   - <http://www.1024cores.net/home/lock-free-algorithms/queues/bounded-mpmc-queue>

use core::{
    cell::UnsafeCell,
    fmt,
    mem::{self, MaybeUninit},
    ops::{Deref, DerefMut},
    panic::{RefUnwindSafe, UnwindSafe},
    sync::atomic::{self, AtomicUsize, Ordering},
};

/// Pads a value to the length of a cache line (64 bytes) to avoid false sharing.
#[repr(align(64))]
struct CachePadded<T>(T);

impl<T> Deref for CachePadded<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for CachePadded<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> CachePadded<T> {
    const fn new(val: T) -> Self {
        Self(val)
    }
}

struct Backoff(u32);

impl Backoff {
    #[inline]
    fn new() -> Self {
        Self(0)
    }

    #[inline]
    fn spin(&mut self) {
        for _ in 0..(1 << self.0.min(6)) {
            core::hint::spin_loop();
        }
        self.0 = self.0.saturating_add(1);
    }

    #[inline]
    fn snooze(&mut self) {
        for _ in 0..(1 << self.0.min(10)) {
            core::hint::spin_loop();
        }
        self.0 = self.0.saturating_add(1);
    }
}

/// A slot in the queue buffer.
struct Slot<T> {
    /// Sequence stamp used to coordinate producers and consumers.
    ///
    /// - `stamp == tail`     → slot is empty and ready to be written.
    /// - `stamp == head + 1` → slot holds a value and is ready to be read.
    stamp: AtomicUsize,

    value: UnsafeCell<MaybeUninit<T>>,
}

/// A bounded, lock-free multi-producer multi-consumer queue backed by a
/// **static inline buffer** of `N` elements.
///
/// Because the buffer lives inside the struct itself (no heap allocation),
/// `N` must be known at compile time. The queue cannot hold more than `N`
/// elements; [`push`](LockFreeQueue::push) returns `Err` when full.
/// [`force_push`](LockFreeQueue::force_push) can be used to treat the queue
/// as a ring-buffer, evicting the oldest element when full.
///
/// # Examples
///
/// ```rust
/// let q = LockFreeQueue::<char, 4>::new();
///
/// assert_eq!(q.push('a'), Ok(()));
/// assert_eq!(q.push('b'), Ok(()));
/// assert_eq!(q.pop(), Some('a'));
/// ```
pub struct LockFreeQueue<T, const N: usize> {
    /// Points to the next slot to be read from.
    head: CachePadded<AtomicUsize>,

    /// Points to the next slot to be written to.
    tail: CachePadded<AtomicUsize>,

    /// Inline slot buffer — no heap allocation required.
    buffer: [Slot<T>; N],

    /// A lap-increment value equal to the smallest power of two greater than `N`.
    one_lap: usize,
}

// SAFETY: All interior mutation goes through atomics or is guarded by the
//         stamp protocol; `T: Send` is sufficient for cross-thread safety.
unsafe impl<T: Send, const N: usize> Sync for LockFreeQueue<T, N> {}
unsafe impl<T: Send, const N: usize> Send for LockFreeQueue<T, N> {}

impl<T, const N: usize> UnwindSafe for LockFreeQueue<T, N> {}
impl<T, const N: usize> RefUnwindSafe for LockFreeQueue<T, N> {}

impl<T, const N: usize> LockFreeQueue<T, N> {
    /// Creates a new, empty `LockFreeQueue`.
    ///
    /// # Panics
    ///
    /// Panics if `N` is zero.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let q = LockFreeQueue::<i32, 16>::new();
    /// ```
    pub fn new() -> Self {
        assert!(N > 0, "LockFreeQueue capacity N must be non-zero");

        let one_lap = (N + 1).next_power_of_two();

        // SAFETY: `Slot` contains only atomics and `MaybeUninit`, both of
        //         which are valid in any bit-pattern. We initialise the stamp
        //         values below via `forget`-and-rewrite rather than
        //         `MaybeUninit::zeroed` so that the stamp for slot `i` starts
        //         at `i` (not zero).
        //
        // We build the array via a manual loop because `Slot` is not `Copy`
        // and const-generic array initialisation with non-trivial values
        // requires unsafe.
        let buffer = {
            let mut uninit: MaybeUninit<[Slot<T>; N]> = MaybeUninit::uninit();
            let ptr = uninit.as_mut_ptr() as *mut Slot<T>;
            for i in 0..N {
                unsafe {
                    ptr.add(i).write(Slot {
                        stamp: AtomicUsize::new(i),
                        value: UnsafeCell::new(MaybeUninit::uninit()),
                    });
                }
            }
            unsafe { uninit.assume_init() }
        };

        Self {
            buffer,
            one_lap,
            head: CachePadded::new(AtomicUsize::new(0)),
            tail: CachePadded::new(AtomicUsize::new(0)),
        }
    }

    /// Returns the capacity of this queue (always equal to `N`).
    #[inline]
    pub const fn capacity(&self) -> usize {
        N
    }

    fn push_or_else<F>(&self, mut value: T, f: F) -> Result<(), T>
    where
        F: Fn(T, usize, usize, &Slot<T>) -> Result<T, T>,
    {
        let mut backoff = Backoff::new();
        let mut tail = self.tail.load(Ordering::Relaxed);

        loop {
            let index = tail & (self.one_lap - 1);
            let lap = tail & !(self.one_lap - 1);

            let new_tail = if index + 1 < N {
                tail + 1
            } else {
                lap.wrapping_add(self.one_lap)
            };

            debug_assert!(index < self.buffer.len());
            let slot = unsafe { self.buffer.get_unchecked(index) };
            let stamp = slot.stamp.load(Ordering::Acquire);

            if tail == stamp {
                // Slot is ready to be written — try to claim it.
                match self.tail.compare_exchange_weak(
                    tail,
                    new_tail,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        unsafe { slot.value.get().write(MaybeUninit::new(value)) };
                        slot.stamp.store(tail + 1, Ordering::Release);
                        return Ok(());
                    }
                    Err(t) => {
                        tail = t;
                        backoff.spin();
                    }
                }
            } else if stamp.wrapping_add(self.one_lap) == tail + 1 {
                // Slot is one full lap behind — potentially full.
                atomic::fence(Ordering::SeqCst);
                value = f(value, tail, new_tail, slot)?;
                backoff.spin();
                tail = self.tail.load(Ordering::Relaxed);
            } else {
                // Another thread is mid-operation; wait for it to finish.
                backoff.snooze();
                tail = self.tail.load(Ordering::Relaxed);
            }
        }
    }

    /// Attempts to push `value` into the queue.
    ///
    /// Returns `Err(value)` if the queue is full.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let q = LockFreeQueue::<i32, 1>::new();
    /// assert_eq!(q.push(1), Ok(()));
    /// assert_eq!(q.push(2), Err(2));
    /// ```
    pub fn push(&self, value: T) -> Result<(), T> {
        self.push_or_else(value, |v, tail, _, _| {
            let head = self.head.load(Ordering::Relaxed);
            if head.wrapping_add(self.one_lap) == tail {
                Err(v) // queue is full
            } else {
                Ok(v) // spurious; retry
            }
        })
    }

    /// Pushes `value` into the queue using an exclusive reference.
    ///
    /// Because `&mut self` guarantees exclusive access, all atomic operations
    /// are elided.  Returns `Err(value)` if the queue is full.
    pub fn push_mut(&mut self, value: T) -> Result<(), T> {
        let tail = *self.tail.get_mut();
        let head = *self.head.get_mut();

        if head.wrapping_add(self.one_lap) == tail {
            return Err(value);
        }

        let index = tail & (self.one_lap - 1);
        let lap = tail & !(self.one_lap - 1);
        let new_tail = if index + 1 < N {
            tail + 1
        } else {
            lap.wrapping_add(self.one_lap)
        };

        *self.tail.get_mut() = new_tail;

        let slot = unsafe { self.buffer.get_unchecked_mut(index) };
        unsafe { slot.value.get().write(MaybeUninit::new(value)) };
        *slot.stamp.get_mut() = tail + 1;

        Ok(())
    }

    /// Pushes `value` into the queue, evicting the oldest element if full.
    ///
    /// Returns the evicted element if the queue was full, or `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let q = LockFreeQueue::<i32, 2>::new();
    /// assert_eq!(q.force_push(1), None);
    /// assert_eq!(q.force_push(2), None);
    /// assert_eq!(q.force_push(3), Some(1)); // evicts 1
    /// assert_eq!(q.pop(), Some(2));
    /// ```
    pub fn force_push(&self, value: T) -> Option<T> {
        self.push_or_else(value, |v, tail, new_tail, slot| {
            let head = tail.wrapping_sub(self.one_lap);
            let new_head = new_tail.wrapping_sub(self.one_lap);

            if self
                .head
                .compare_exchange_weak(head, new_head, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                self.tail.store(new_tail, Ordering::SeqCst);

                let old = unsafe { slot.value.get().replace(MaybeUninit::new(v)).assume_init() };
                slot.stamp.store(tail + 1, Ordering::Release);

                Err(old) // signals "evicted old value"
            } else {
                Ok(v) // CAS failed; retry outer loop
            }
        })
        .err()
    }

    /// Attempts to pop an element from the queue.
    ///
    /// Returns `None` if the queue is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let q = LockFreeQueue::<i32, 4>::new();
    /// q.push(42).unwrap();
    /// assert_eq!(q.pop(), Some(42));
    /// assert_eq!(q.pop(), None);
    /// ```
    pub fn pop(&self) -> Option<T> {
        let mut backoff = Backoff::new();
        let mut head = self.head.load(Ordering::Relaxed);

        loop {
            let index = head & (self.one_lap - 1);
            let lap = head & !(self.one_lap - 1);

            debug_assert!(index < self.buffer.len());
            let slot = unsafe { self.buffer.get_unchecked(index) };
            let stamp = slot.stamp.load(Ordering::Acquire);

            if head + 1 == stamp {
                // Slot holds a value — try to claim it.
                let new = if index + 1 < N {
                    head + 1
                } else {
                    lap.wrapping_add(self.one_lap)
                };

                match self.head.compare_exchange_weak(
                    head,
                    new,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        let value = unsafe { slot.value.get().read().assume_init() };
                        slot.stamp
                            .store(head.wrapping_add(self.one_lap), Ordering::Release);
                        return Some(value);
                    }
                    Err(h) => {
                        head = h;
                        backoff.spin();
                    }
                }
            } else if stamp == head {
                atomic::fence(Ordering::SeqCst);
                let tail = self.tail.load(Ordering::Relaxed);
                if tail == head {
                    return None; // queue is empty
                }
                backoff.spin();
                head = self.head.load(Ordering::Relaxed);
            } else {
                backoff.snooze();
                head = self.head.load(Ordering::Relaxed);
            }
        }
    }

    /// Attempts to pop an element using an exclusive reference.
    ///
    /// Atomic operations are elided due to `&mut self`.
    /// Returns `None` if the queue is empty.
    pub fn pop_mut(&mut self) -> Option<T> {
        let head = *self.head.get_mut();
        let tail = *self.tail.get_mut();

        if tail == head {
            return None;
        }

        let index = head & (self.one_lap - 1);
        let lap = head & !(self.one_lap - 1);

        debug_assert!(index < self.buffer.len());
        let slot = unsafe { self.buffer.get_unchecked_mut(index) };

        let value = unsafe { slot.value.get().read().assume_init() };
        *slot.stamp.get_mut() = head.wrapping_add(self.one_lap);

        *self.head.get_mut() = if index + 1 < N {
            head + 1
        } else {
            lap.wrapping_add(self.one_lap)
        };

        Some(value)
    }

    /// Returns `true` if the queue currently contains no elements.
    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);
        tail == head
    }

    /// Returns `true` if the queue is at capacity.
    pub fn is_full(&self) -> bool {
        let tail = self.tail.load(Ordering::SeqCst);
        let head = self.head.load(Ordering::SeqCst);
        head.wrapping_add(self.one_lap) == tail
    }

    /// Returns the number of elements currently in the queue.
    ///
    /// This is a best-effort snapshot; the value may be stale in a
    /// concurrent context by the time it is used.
    pub fn len(&self) -> usize {
        loop {
            let tail = self.tail.load(Ordering::SeqCst);
            let head = self.head.load(Ordering::SeqCst);

            if self.tail.load(Ordering::SeqCst) == tail {
                let hix = head & (self.one_lap - 1);
                let tix = tail & (self.one_lap - 1);

                return if hix < tix {
                    tix - hix
                } else if hix > tix {
                    N - hix + tix
                } else if tail == head {
                    0
                } else {
                    N
                };
            }
        }
    }
}

impl<T, const N: usize> Default for LockFreeQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Drop for LockFreeQueue<T, N> {
    fn drop(&mut self) {
        if mem::needs_drop::<T>() {
            let head = *self.head.get_mut();
            let tail = *self.tail.get_mut();

            let hix = head & (self.one_lap - 1);
            let tix = tail & (self.one_lap - 1);

            let len = if hix < tix {
                tix - hix
            } else if hix > tix {
                N - hix + tix
            } else if tail == head {
                0
            } else {
                N
            };

            for i in 0..len {
                let index = if hix + i < N { hix + i } else { hix + i - N };
                unsafe {
                    debug_assert!(index < self.buffer.len());
                    let slot = self.buffer.get_unchecked_mut(index);
                    (*slot.value.get()).assume_init_drop();
                }
            }
        }
    }
}

impl<T, const N: usize> fmt::Debug for LockFreeQueue<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("LockFreeQueue { .. }")
    }
}

impl<T, const N: usize> IntoIterator for LockFreeQueue<T, N> {
    type Item = T;
    type IntoIter = IntoIter<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { value: self }
    }
}

/// Consuming iterator over a [`LockFreeQueue`].
#[derive(Debug)]
pub struct IntoIter<T, const N: usize> {
    value: LockFreeQueue<T, N>,
}

impl<T, const N: usize> Iterator for IntoIter<T, N> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.value.pop_mut()
    }
}
