mod executor;
mod task_impl;
mod utils;

use {crate::task::executor::Executor, spinlocks::once::Once};

pub use utils::*;

static MAIN_EXECUTOR: Once<Executor> = Once::new();

pub(crate) fn init() {
    MAIN_EXECUTOR.init(Executor::new);
}

fn main_executor() -> &'static Executor {
    MAIN_EXECUTOR.poll().expect("Main executor not initialized")
}

pub fn run() -> ! {
    main_executor().run()
}

pub fn spawn(future: impl Future<Output = ()> + 'static + Send) {
    main_executor().spawn(future);
}
