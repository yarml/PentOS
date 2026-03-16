mod ps2_impl;

use crate::sync::mutex::AsyncMutexGuard;
pub(crate) use ps2_impl::{init, on_scancode};

use {
    crate::{
        dev::{ps2::ps2_impl::KEYS_PRESS_MAP, timer},
        sync::mutex::AsyncMutex,
        task::{futures::ManualFuture, stream::Stream},
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
    utils::collections::broadcast_queue::{BroadcastCursor, BroadcastQueue, ReadResult},
};

pub mod keys {
    pub use keys::*;
}

static LAST_UPDATE_TIMESTAMP: AtomicUsize = AtomicUsize::new(0);

static KEY_EVENT_QUEUE: AsyncMutex<BroadcastQueue<KeyEvent, KEY_EVENT_QUEUE_SIZE>> =
    AsyncMutex::new(BroadcastQueue::new());
static KEY_EVENT_WAKERS: AsyncMutex<Vec<Waker>> = AsyncMutex::new(Vec::new());

pub async fn keyboard_update() -> KeyUpdateFuture {
    KeyUpdateFuture {
        stream: key_event_stream().await,
    }
}
pub async fn key_event_stream() -> KeyEventStream {
    let event_queue = KEY_EVENT_QUEUE.lock().await;
    let cursor = event_queue.subscribe();

    KeyEventStream {
        cursor,
        event_queue: ManualFuture::make(KEY_EVENT_QUEUE.lock()),
        wakers: ManualFuture::make(KEY_EVENT_WAKERS.lock()),
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
    event_queue:
        ManualFuture<AsyncMutexGuard<'static, BroadcastQueue<KeyEvent, KEY_EVENT_QUEUE_SIZE>>>,
    wakers: ManualFuture<AsyncMutexGuard<'static, Vec<Waker>>>,
    registered: bool,
}

impl Stream for KeyEventStream {
    type Item = KeyEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Try to read before registering the waker, in case events arrived
        // between the last poll and now.

        if self.event_queue.poll(cx).is_none() {
            return Poll::Pending;
        }
        let event_queue = self.event_queue.remake(KEY_EVENT_QUEUE.lock()).unwrap();
        let read_result = self.cursor.read(&event_queue);

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
            if self.wakers.poll(cx).is_none() {
                return Poll::Pending;
            }
            let mut wakers = self.wakers.remake(KEY_EVENT_WAKERS.lock()).unwrap();
            if wakers.iter().all(|w| !w.will_wake(cx.waker())) {
                wakers.push(cx.waker().clone());
            }
            self.registered = true;
        }

        Poll::Pending
    }
}

async fn keyboard_update_wake(event: KeyEvent) {
    let current_time = timer::get_timestamp();
    LAST_UPDATE_TIMESTAMP.store(current_time, Ordering::Relaxed);
    KEY_EVENT_QUEUE.lock().await.push(event);

    let event_wakers = core::mem::take(&mut *KEY_EVENT_WAKERS.lock().await);

    for waker in event_wakers {
        waker.wake();
    }
}
