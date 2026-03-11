mod ps2_impl;

use {
    crate::{dev::timer, task::stream::Stream},
    alloc::vec::Vec,
    config::dev::ps2::KEY_EVENT_QUEUE_SIZE,
    core::{
        pin::Pin,
        sync::atomic::{AtomicUsize, Ordering},
        task::{Context, Poll, Waker},
    },
    keys::KeyEvent,
    spinlocks::mutex::Mutex,
    utils::collections::broadcast_queue::{BroadcastCursor, BroadcastQueue, ReadResult},
    x64::interrupts,
};

pub(crate) use ps2_impl::{init, on_key_event};

static LAST_UPDATE_TIMESTAMP: AtomicUsize = AtomicUsize::new(0);

static KEY_EVENT_QUEUE: Mutex<BroadcastQueue<KeyEvent, KEY_EVENT_QUEUE_SIZE>> =
    Mutex::new(BroadcastQueue::new());
static KEY_EVENT_WAKERS: Mutex<Vec<Waker>> = Mutex::new(Vec::new());

pub fn keyboard_update() -> KeyUpdateFuture {
    KeyUpdateFuture {
        stream: key_event_stream(),
    }
}
pub fn key_event_stream() -> KeyEventStream {
    KeyEventStream {
        cursor: KEY_EVENT_QUEUE.lock().subscribe(),
        registered: false,
    }
}

pub struct KeyUpdateFuture {
    stream: KeyEventStream,
}

impl Future for KeyUpdateFuture {
    type Output = KeyEvent;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.stream).poll_next(cx) {
            Poll::Ready(KeyEventItem::Event(ev)) => Poll::Ready(ev),
            _ => Poll::Pending,
        }
    }
}

pub struct KeyEventStream {
    cursor: BroadcastCursor,
    registered: bool,
}

pub enum KeyEventItem {
    Event(KeyEvent),
    /// Some events were missed because this consumer was too slow.
    /// The inner value is the number of missed events.
    Lagged(usize),
}

impl Stream for KeyEventStream {
    type Item = KeyEventItem;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Item> {
        // Try to read before registering the waker, in case events arrived
        // between the last poll and now.
        match self.cursor.read(&*KEY_EVENT_QUEUE.lock()) {
            ReadResult::Event(event) => {
                self.registered = false;
                return Poll::Ready(KeyEventItem::Event(event));
            }
            ReadResult::Lagged(missed) => {
                self.registered = false;
                return Poll::Ready(KeyEventItem::Lagged(missed));
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

        KEY_EVENT_QUEUE.lock().push(event);

        core::mem::take(&mut *KEY_EVENT_WAKERS.lock())
    });

    for waker in event_wakers {
        waker.wake();
    }
}
