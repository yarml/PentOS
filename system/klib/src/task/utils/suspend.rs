use {
    alloc::vec::Vec,
    core::{
        pin::Pin,
        task::{Context, Poll, Waker},
    },
    spinlocks::mutex::Mutex,
    x64::interrupts,
};

pub fn suspend() -> Suspender {
    Suspender::new()
}

pub struct Suspender {
    registered: bool,
}

impl Suspender {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { registered: false }
    }
}

impl Future for Suspender {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.registered {
            Poll::Ready(())
        } else {
            self.get_mut().registered = true;
            interrupts::with_disabled(|| {
                let mut wakers = WAKERS.lock();
                if wakers.iter().all(|w| !w.will_wake(cx.waker())) {
                    wakers.push(cx.waker().clone());
                }
            });
            Poll::Pending
        }
    }
}

static WAKERS: Mutex<Vec<Waker>> = Mutex::new(Vec::new());

pub(crate) fn wake() {
    let wakers = interrupts::with_disabled(|| {
        let mut wakers = WAKERS.lock();
        core::mem::take(&mut *wakers)
    });
    for waker in wakers {
        waker.wake();
    }
}
