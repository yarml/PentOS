use {
    crate::task::{
        task_impl::{Task, TaskId, TaskState, TaskWaker},
        urgent_task::UrgentTask,
    },
    alloc::collections::{btree_map::BTreeMap, vec_deque::VecDeque},
    config::task::MAX_URGENT_TASK_COUNT,
    core::{
        pin::Pin,
        task::{Context, Poll, Waker},
    },
    spinlocks::mutex::SpinMutex,
    utils::collections::lock_free_queue::LockFreeQueue,
    x64::interrupts,
};

pub(super) struct Executor {
    urgent_queue: LockFreeQueue<UrgentTask, MAX_URGENT_TASK_COUNT>,
    queue: SpinMutex<VecDeque<TaskId>>,
    tasks: SpinMutex<BTreeMap<TaskId, Task>>,
    waker_cache: SpinMutex<BTreeMap<TaskId, Waker>>,
}

impl Executor {
    pub(super) fn new() -> Self {
        Executor {
            urgent_queue: LockFreeQueue::new(),
            queue: SpinMutex::new(VecDeque::new()),
            tasks: SpinMutex::new(BTreeMap::new()),
            waker_cache: SpinMutex::new(BTreeMap::new()),
        }
    }

    /// Lock free urgent task spawning. Urgent tasks are normal functions
    /// they are not supposed to be async and must complete quickly
    ///
    /// urgent tasks should not be fired at a high rate, or else they might
    /// starve compute time from normal tasks.
    pub fn spawn_urgent(&self, urgent: UrgentTask) {
        self.urgent_queue
            .push(urgent)
            .expect("too many urgent tasks at once");
    }

    pub fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let task = Task::new(future);
        interrupts::with_disabled(|| {
            let mut tasks = self.tasks.lock();
            let mut queue = self.queue.lock();
            let task_id = task.id();
            if tasks.insert(task.id(), task).is_some() {
                panic!("task with same ID already exists");
            }
            queue.push_back(task_id);
        })
    }

    pub fn schedule(&self, task_id: TaskId) {
        interrupts::with_disabled(|| {
            let mut tasks = self.tasks.lock();
            let Some(task) = tasks.get_mut(&task_id) else {
                // Maybe we have a waker left whose task somehow already finished execution???
                return;
            };
            if task.state() != TaskState::Pending {
                // Already scheduled apparently
                return;
            }
            task.set_state(TaskState::Scheduled);
            let mut queue = self.queue.lock();
            queue.push_back(task_id);
        })
    }

    pub fn run(self: Pin<&'static Self>) -> ! {
        loop {
            while let Some(urgent_task) = self.urgent_queue.pop() {
                urgent_task()
            }

            while let Some(task_id) = interrupts::with_disabled(|| {
                let mut queue = self.queue.lock();
                queue.pop_front()
            }) {
                let Some(task) = interrupts::with_disabled(|| {
                    let mut tasks = self.tasks.lock();
                    tasks.get_mut(&task_id).map(|task_mut| unsafe {
                        task_mut.set_state(TaskState::Running);
                        // SAFETY: This is safe due to two combined invariants:
                        // 1. BTreeMap guarantees stable node addresses — inserting or removing
                        //    *other* keys does not move or invalidate existing values in place,
                        //    so the pointer remains valid for the lifetime of the task entry.
                        // 2. We set the task state to `Running` while still holding the `tasks`
                        //    lock. `schedule()` only re-enqueues a task if its state is `Pending`,
                        //    so no other hart can observe this task_id in the queue while it is
                        //    `Running`, making the &mut unique.
                        // Together these ensure the &mut alias is exclusive and the pointee lives
                        // at least until we call tasks.remove() in the Poll::Ready branch.
                        &mut *(task_mut as *mut Task)
                    })
                }) else {
                    continue;
                };

                let waker = interrupts::with_disabled(|| {
                    let mut waker_cache = self.waker_cache.lock();

                    waker_cache
                        .entry(task_id)
                        .or_insert_with(|| TaskWaker::waker(task_id, self))
                        .clone()
                });

                let mut context = Context::from_waker(&waker);

                match task.poll(&mut context) {
                    Poll::Ready(()) => interrupts::with_disabled(|| {
                        let mut tasks = self.tasks.lock();
                        let mut waker_cache = self.waker_cache.lock();
                        tasks.remove(&task_id);
                        waker_cache.remove(&task_id);
                    }),
                    Poll::Pending => {
                        task.set_state(TaskState::Pending);
                    }
                }
            }

            let queue_empty = interrupts::with_disabled(|| {
                let queue = self.queue.lock();
                queue.is_empty()
            });

            if queue_empty && self.urgent_queue.is_empty() {
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
