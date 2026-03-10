use {
    crate::task::{sleep, suspend},
    core::sync::atomic::{AtomicUsize, Ordering},
};

static TIMESTAMP: AtomicUsize = AtomicUsize::new(0);

pub fn get_timestamp() -> usize {
    TIMESTAMP.load(Ordering::Relaxed)
}

pub(crate) fn on_tick() {
    TIMESTAMP.fetch_add(1, Ordering::Relaxed);
    sleep::wake();
    suspend::wake();
}
