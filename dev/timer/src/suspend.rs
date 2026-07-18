use {
    crate::task::futures::ManualFuture,
    alloc::vec::Vec,
    core::{
        pin::Pin,
        task::{Context, Poll, Waker},
    },
    sync::{AsyncMutex, AsyncMutexGuard},
};

pub fn suspend() -> Suspender {
    Suspender::new()
}

pub struct Suspender {
    wakers: ManualFuture<AsyncMutexGuard<'static, Vec<Waker>>>,
    registered: bool,
}

impl Suspender {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            wakers: ManualFuture::make(WAKERS.lock()),
            registered: false,
        }
    }
}

impl Future for Suspender {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.registered {
            Poll::Ready(())
        } else {
            if self.wakers.poll(cx).is_none() {
                return Poll::Pending;
            }
            let mut wakers = self.wakers.remake(WAKERS.lock()).unwrap();
            if wakers.iter().all(|w| !w.will_wake(cx.waker())) {
                wakers.push(cx.waker().clone());
            }
            self.registered = true;
            Poll::Pending
        }
    }
}

static WAKERS: AsyncMutex<Vec<Waker>> = AsyncMutex::new(Vec::new());

pub(crate) async fn wake() {
    let wakers = core::mem::take(&mut *WAKERS.lock().await);

    for waker in wakers {
        waker.wake();
    }
}
