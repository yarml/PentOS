use {
    crate::{get_timestamp, task::futures::ManualFuture}, alloc::collections::binary_heap::BinaryHeap, core::{
        cmp::Ordering,
        pin::Pin,
        task::{Context, Poll, Waker},
    }, sync::{AsyncMutex, AsyncMutexGuard},
};

struct SleepingWaker {
    waker: Waker,
    end_time: usize,
}

static WAKERS: AsyncMutex<BinaryHeap<SleepingWaker>> = AsyncMutex::new(BinaryHeap::new());

pub fn sleep(ms: usize) -> Sleeper {
    Sleeper::for_duration(ms)
}

pub struct Sleeper {
    end_time: usize,
    wakers: ManualFuture<AsyncMutexGuard<'static, BinaryHeap<SleepingWaker>>>,
}

impl Sleeper {
    pub fn for_duration(ms: usize) -> Self {
        let ticks = ms.div_ceil(10);
        let current_time = get_timestamp();
        let end_time = current_time + ticks;
        Self {
            end_time,
            wakers: ManualFuture::make(WAKERS.lock()),
        }
    }
}

impl Future for Sleeper {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let current = get_timestamp();
        if current >= self.end_time {
            Poll::Ready(())
        } else {
            if self.wakers.poll(cx).is_none() {
                return Poll::Pending;
            }
            let mut wakers = self.wakers.remake(WAKERS.lock()).unwrap();

            let end_time = self.end_time;
            if wakers.iter().all(|w| !w.waker.will_wake(cx.waker())) {
                wakers.push(SleepingWaker {
                    end_time,
                    waker: cx.waker().clone(),
                });
            }
            Poll::Pending
        }
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
pub(crate) async fn wake() {
    let current = get_timestamp();
    let mut wakers = WAKERS.lock().await;
    while wakers
        .peek()
        .filter(|waker| current >= waker.end_time)
        .is_some()
    {
        wakers.pop().unwrap().waker.wake();
    }
}
