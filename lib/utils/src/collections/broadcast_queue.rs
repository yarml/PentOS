use core::mem::MaybeUninit;

pub struct BroadcastQueue<T: Copy, const N: usize> {
    buf: [MaybeUninit<T>; N],
    tail: usize,
}

pub struct BroadcastCursor {
    head: usize,
}

pub enum ReadResult<T> {
    /// An event was read, cursor has been advanced.
    Event(T),
    /// The consumer was too slow and missed some events.
    /// The cursor has been fast-forwarded to the oldest available event
    /// and read from there.
    /// The inner value is the number of missed events & the oldest
    /// still available entry.
    Lagged { missed: usize, val: T },
    /// No new events available.
    Pending,
}

impl<T: Copy, const N: usize> BroadcastQueue<T, N> {
    pub const fn new() -> Self {
        assert!(N > 0, "BroadcastQueue capacity must be non-zero");
        Self {
            buf: [MaybeUninit::uninit(); N],
            tail: 0,
        }
    }

    pub fn push(&mut self, val: T) {
        self.buf[self.tail % N].write(val);
        self.tail += 1;
    }

    pub fn subscribe(&self) -> BroadcastCursor {
        BroadcastCursor { head: self.tail }
    }

    fn read_inner(&self, cursor: &mut BroadcastCursor) -> ReadResult<T> {
        if cursor.head == self.tail {
            return ReadResult::Pending;
        }

        let missed = if self.tail - cursor.head > N {
            let missed = self.tail - N - cursor.head;
            cursor.head = self.tail - N;
            missed
        } else {
            0
        };

        // SAFETY: this slot has been written since head < tail
        // and we verified it hasn't been overwritten (tail - head <= N)
        let val = unsafe { self.buf[cursor.head % N].assume_init() };
        cursor.head += 1;

        if missed > 0 {
            ReadResult::Lagged { missed, val }
        } else {
            ReadResult::Event(val)
        }
    }
}

impl BroadcastCursor {
    pub fn read<T: Copy, const N: usize>(&mut self, queue: &BroadcastQueue<T, N>) -> ReadResult<T> {
        queue.read_inner(self)
    }
}

impl<T: Copy, const N: usize> Default for BroadcastQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ReadResult<T> {
    pub fn value(self) -> Option<T> {
        match self {
            ReadResult::Event(val) => Some(val),
            ReadResult::Lagged { val, .. } => Some(val),
            ReadResult::Pending => None,
        }
    }
}
