use {
    crate::task::{self, sleep, suspend},
    core::sync::atomic::{AtomicUsize, Ordering},
};

static TIMESTAMP: AtomicUsize = AtomicUsize::new(0);

pub fn get_timestamp() -> usize {
    TIMESTAMP.load(Ordering::Relaxed)
}

pub(crate) fn on_tick() {
    TIMESTAMP.fetch_add(1, Ordering::Relaxed);
    task::spawn(wake_all());
}

async fn wake_all() {
    sleep::wake().await;
    suspend::wake().await;
}
