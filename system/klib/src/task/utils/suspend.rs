use core::{
    pin::Pin,
    task::{Context, Poll},
};

pub fn suspend() -> Suspender {
    Suspender::new()
}

pub struct Suspender {
    polled: bool,
}

impl Suspender {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { polled: false }
    }
}

impl Future for Suspender {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.polled {
            Poll::Ready(())
        } else {
            self.get_mut().polled = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
