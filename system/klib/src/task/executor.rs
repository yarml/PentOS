use {
    crate::task::{
        task_impl::{Task, TaskId, TaskWaker},
        urgent_task::UrgentTask,
    },
    alloc::{
        collections::{btree_map::BTreeMap, vec_deque::VecDeque},
        sync::Arc,
    },
    config::task::MAX_URGENT_TASK_COUNT,
    core::{
        mem,
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
    tasks: SpinMutex<BTreeMap<TaskId, Arc<SpinMutex<Task>>>>,
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
            let mut queue = self.queue.lock();
            let mut tasks = self.tasks.lock();
            let task_id = task.id();
            if tasks
                .insert(task.id(), Arc::new(SpinMutex::new(task)))
                .is_some()
            {
                panic!("task with same ID already exists");
            }
            queue.push_back(task_id);
        })
    }

    pub fn schedule(&self, task_id: TaskId) {
        interrupts::with_disabled(|| {
            let mut queue = self.queue.lock();
            let tasks = self.tasks.lock();
            if tasks.get(&task_id).is_none() {
                return;
            }
            queue.push_back(task_id);
        })
    }

    pub fn run(self: Pin<&'static Self>) -> ! {
        loop {
            while let Some(urgent_task) = self.urgent_queue.pop() {
                urgent_task()
            }

            let task_lock = interrupts::with_disabled(|| {
                let mut queue = self.queue.lock();
                let tasks = self.tasks.lock();

                let (index, task_lock) =
                    queue.iter().enumerate().find_map(|(index, task_id)| {
                        // task_id not obsolete
                        let task_lock = tasks.get(task_id)?;
                        let task_lock = task_lock.clone();

                        // if already locked, it's being executed by another core currently.
                        let task = task_lock.try_lock()?;

                        // Forget so that the task stays locked for a guard that we will recreate later
                        mem::forget(task);

                        // task not running
                        Some((index, task_lock))
                    })?;
                queue.remove(index);

                Some(task_lock)
            });

            if let Some(task_lock) = task_lock {
                let mut task = unsafe {
                    // SAFETY: we previously forgot the lock, now we remember it again
                    task_lock.force_lock()
                };

                let waker = interrupts::with_disabled(|| {
                    let mut waker_cache = self.waker_cache.lock();

                    waker_cache
                        .entry(task.id())
                        .or_insert_with(|| TaskWaker::waker(task.id(), self))
                        .clone()
                });
                let mut cx = Context::from_waker(&waker);

                match task.poll(&mut cx) {
                    Poll::Ready(()) => {
                        let mut tasks = self.tasks.lock();
                        let mut waker_cache = self.waker_cache.lock();
                        tasks.remove(&task.id());
                        waker_cache.remove(&task.id());
                    }
                    Poll::Pending => {}
                }
            }

            interrupts::disable();
            let queue_empty = {
                let queue = self.queue.lock();
                queue.is_empty()
            };
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
