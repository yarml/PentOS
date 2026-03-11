mod ps2_impl;

use {
    crate::dev::timer,
    alloc::vec::Vec,
    core::{
        pin::Pin,
        sync::atomic::{AtomicUsize, Ordering},
        task::{Context, Poll, Waker},
    },
    spinlocks::mutex::Mutex,
    x64::interrupts,
};

pub(crate) use ps2_impl::{init, on_key_event};

static LAST_UPDATE_TIMESTAMP: AtomicUsize = AtomicUsize::new(0);

static KEYBOARD_UPDATE_WAKERS: Mutex<Vec<Waker>> = Mutex::new(Vec::new());

pub fn keyboard_update() -> KeyUpdateFuture {
    KeyUpdateFuture {
        start_timestamp: timer::get_timestamp(),
    }
}

pub struct KeyUpdateFuture {
    start_timestamp: usize,
}

impl Future for KeyUpdateFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if LAST_UPDATE_TIMESTAMP.load(Ordering::Relaxed) > self.start_timestamp {
            Poll::Ready(())
        } else {
            interrupts::with_disabled(|| {
                let mut wakers = KEYBOARD_UPDATE_WAKERS.lock();
                if wakers.iter().all(|w| !w.will_wake(cx.waker())) {
                    wakers.push(cx.waker().clone());
                }
            });
            Poll::Pending
        }
    }
}

fn keyboard_update_wake() {
    let wakers = interrupts::with_disabled(|| {
        let current_time = timer::get_timestamp();
        LAST_UPDATE_TIMESTAMP.store(current_time, Ordering::Relaxed);
        let mut wakers = KEYBOARD_UPDATE_WAKERS.lock();
        core::mem::take(&mut *wakers)
    });
    for waker in wakers {
        waker.wake();
    }
}
