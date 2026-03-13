mod ps2_impl;

pub(crate) use ps2_impl::{init, on_key_event};
use {
    crate::{
        dev::{ps2::ps2_impl::KEYS_PRESS_MAP, timer},
        task::stream::Stream,
    },
    alloc::vec::Vec,
    config::dev::ps2::KEY_EVENT_QUEUE_SIZE,
    core::{
        pin::Pin,
        sync::atomic::{AtomicUsize, Ordering},
        task::{Context, Poll, Waker},
    },
    keys::{Key, KeyEvent},
    log::warn,
    spinlocks::{mutex::SpinMutex, rwlock::SpinRwLock},
    utils::collections::broadcast_queue::{BroadcastCursor, BroadcastQueue, ReadResult},
    x64::interrupts,
};

pub mod keys {
    pub use keys::*;
}

static LAST_UPDATE_TIMESTAMP: AtomicUsize = AtomicUsize::new(0);

static KEY_EVENT_QUEUE: SpinRwLock<BroadcastQueue<KeyEvent, KEY_EVENT_QUEUE_SIZE>> =
    SpinRwLock::new(BroadcastQueue::new());
static KEY_EVENT_WAKERS: SpinMutex<Vec<Waker>> = SpinMutex::new(Vec::new());

pub fn keyboard_update() -> KeyUpdateFuture {
    KeyUpdateFuture {
        stream: key_event_stream(),
    }
}
pub fn key_event_stream() -> KeyEventStream {
    let cursor = interrupts::with_disabled(|| KEY_EVENT_QUEUE.read().subscribe());

    KeyEventStream {
        cursor,
        registered: false,
    }
}

pub fn is_down(key: Key) -> bool {
    KEYS_PRESS_MAP[key.id].load(Ordering::Relaxed)
}

pub struct KeyUpdateFuture {
    stream: KeyEventStream,
}

impl Future for KeyUpdateFuture {
    type Output = KeyEvent;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.stream).poll_next(cx) {
            Poll::Ready(Some(ev)) => Poll::Ready(ev),
            Poll::Ready(None) => unreachable!("KeyEventStream never gives finishes"),
            _ => Poll::Pending,
        }
    }
}

pub struct KeyEventStream {
    cursor: BroadcastCursor,
    registered: bool,
}

impl Stream for KeyEventStream {
    type Item = KeyEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Try to read before registering the waker, in case events arrived
        // between the last poll and now.

        let read_result = interrupts::with_disabled(|| self.cursor.read(&*KEY_EVENT_QUEUE.read()));

        match read_result {
            ReadResult::Event(event) => {
                self.registered = false;
                return Poll::Ready(Some(event));
            }
            ReadResult::Lagged { missed, val } => {
                warn!("missed {missed} keyboard events");
                self.registered = false;
                return Poll::Ready(Some(val));
            }
            ReadResult::Pending => {}
        }

        // No events available, register waker and return Pending.
        if !self.registered {
            interrupts::with_disabled(|| {
                let mut wakers = KEY_EVENT_WAKERS.lock();
                if wakers.iter().all(|w| !w.will_wake(cx.waker())) {
                    wakers.push(cx.waker().clone());
                }
            });
            self.registered = true;
        }

        Poll::Pending
    }
}

fn keyboard_update_wake(event: KeyEvent) {
    let event_wakers = interrupts::with_disabled(|| {
        let current_time = timer::get_timestamp();
        LAST_UPDATE_TIMESTAMP.store(current_time, Ordering::Relaxed);

        KEY_EVENT_QUEUE.write().push(event);

        core::mem::take(&mut *KEY_EVENT_WAKERS.lock())
    });

    for waker in event_wakers {
        waker.wake();
    }
}
