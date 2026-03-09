use {
    crate::interrupts::get_timestamp,
    core::{
        pin::Pin,
        task::{Context, Poll},
    },
};

pub struct Sleeper {
    end_time: usize,
}

impl Sleeper {
    pub fn for_duration(ms: usize) -> Self {
        let ticks = ms.div_ceil(10);
        let current_time = get_timestamp();
        let end_time = current_time + ticks;
        Self { end_time }
    }
}

impl Future for Sleeper {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let current = get_timestamp();
        if current > self.end_time {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

pub fn sleep(ms: usize) -> Sleeper {
    Sleeper::for_duration(ms)
}
