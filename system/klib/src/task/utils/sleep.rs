use {
    crate::dev::timer::get_timestamp,
    alloc::collections::binary_heap::BinaryHeap,
    core::{
        cmp::Ordering,
        pin::Pin,
        task::{Context, Poll, Waker},
    },
    spinlocks::mutex::Mutex,
    x64::interrupts,
};

struct SleepingWaker {
    waker: Waker,
    end_time: usize,
}

static WAKERS: Mutex<BinaryHeap<SleepingWaker>> = Mutex::new(BinaryHeap::new());

pub fn sleep(ms: usize) -> Sleeper {
    Sleeper::for_duration(ms)
}

pub struct Sleeper {
    end_time: usize,
    registered: bool,
}

impl Sleeper {
    pub fn for_duration(ms: usize) -> Self {
        let ticks = ms.div_ceil(10);
        let current_time = get_timestamp();
        let end_time = current_time + ticks;
        Self {
            end_time,
            registered: false,
        }
    }
}

impl Future for Sleeper {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        interrupts::with_disabled(|| {
            let current = get_timestamp();
            if current >= self.end_time {
                Poll::Ready(())
            } else {
                if !self.registered {
                    let end_time = self.end_time;
                    self.get_mut().registered = true;
                    let mut wakers = WAKERS.lock();
                    wakers.push(SleepingWaker {
                        end_time,
                        waker: cx.waker().clone(),
                    });
                }
                Poll::Pending
            }
        })
    }
}

impl Ord for SleepingWaker {
    fn cmp(&self, other: &Self) -> Ordering {
        other.end_time.cmp(&self.end_time)
    }
}
impl PartialOrd for SleepingWaker {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for SleepingWaker {
    fn eq(&self, other: &Self) -> bool {
        self.end_time == other.end_time
    }
}
impl Eq for SleepingWaker {}

/// Scheduled as urgent task at timer_interrupt
pub fn wake() {
    let current = get_timestamp();
    interrupts::with_disabled(|| {
        let mut wakers = WAKERS.lock();
        while wakers
            .peek()
            .filter(|waker| current >= waker.end_time)
            .is_some()
        {
            wakers.pop().unwrap().waker.wake();
        }
    })
}
