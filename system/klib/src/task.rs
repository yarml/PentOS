pub mod stream;

mod executor;
mod task_impl;
mod urgent_task;
mod utils;

use {
    crate::task::{executor::Executor, urgent_task::UrgentTask},
    core::pin::Pin,
    spinlocks::once::Once,
};

pub use utils::*;

static MAIN_EXECUTOR: Once<Executor> = Once::new();

pub(crate) fn init() {
    MAIN_EXECUTOR.init(Executor::new);
}

fn main_executor() -> Pin<&'static Executor> {
    Pin::static_ref(MAIN_EXECUTOR.poll().expect("Main executor not initialized"))
}

pub fn run() -> ! {
    main_executor().run()
}

pub fn spawn(future: impl Future<Output = ()> + 'static + Send) {
    main_executor().spawn(future)
}

pub fn spawn_urgent(urgent: UrgentTask) {
    main_executor().spawn_urgent(urgent)
}
