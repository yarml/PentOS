use {
    crate::task::task_impl::{Task, TaskId, TaskWaker},
    alloc::{collections::btree_map::BTreeMap, sync::Arc},
    config::task::MAX_TASK_COUNT,
    core::task::{Context, Poll, Waker},
    spinlocks::mutex::Mutex,
    utils::collections::lock_free_queue::LockFreeQueue,
    x64::interrupts,
};

pub type TaskQueue = LockFreeQueue<TaskId, MAX_TASK_COUNT>;

pub(super) struct Executor {
    queue: Arc<TaskQueue>,
    tasks: Mutex<BTreeMap<TaskId, Task>>,
    waker_cache: Mutex<BTreeMap<TaskId, Waker>>,
}

impl Executor {
    pub(super) fn new() -> Self {
        Executor {
            queue: Arc::new(TaskQueue::new()),
            tasks: Mutex::new(BTreeMap::new()),
            waker_cache: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let task = Task::new(future);
        interrupts::with_disabled(|| {
            let mut tasks = self.tasks.lock();
            let task_id = task.id();
            if tasks.insert(task.id(), task).is_some() {
                panic!("task with same ID already exists");
            }
            self.queue.push(task_id).expect("queue full");
        })
    }

    pub fn run(&self) -> ! {
        loop {
            while let Some(task_id) = self.queue.pop() {
                let Some(task) = interrupts::with_disabled(|| {
                    let mut tasks = self.tasks.lock();
                    tasks.get_mut(&task_id).map(|task_mut| unsafe {
                        // SAFETY: Queue contains an ID that is not repeated twice
                        // in the queue for another hart to get a reference to the same task
                        &mut *(task_mut as *mut Task)
                    })
                }) else {
                    continue;
                };

                let waker = interrupts::with_disabled(|| {
                    let mut waker_cache = self.waker_cache.lock();
                    unsafe {
                        // SAFETY: Queue contains an ID that is not repeated twice
                        // in the queue for another hart to get a reference to the same task
                        &*(waker_cache
                            .entry(task_id)
                            .or_insert_with(|| TaskWaker::waker(task_id, self.queue.clone()))
                            as *const Waker)
                    }
                });

                let mut context = Context::from_waker(waker);

                match task.poll(&mut context) {
                    Poll::Ready(()) => interrupts::with_disabled(|| {
                        let mut tasks = self.tasks.lock();
                        tasks.remove(&task_id);
                    }),
                    Poll::Pending => {}
                }
            }

            interrupts::disable();
            if self.queue.is_empty() {
                interrupts::enable_and_halt();
            } else {
                interrupts::enable();
            }
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}
