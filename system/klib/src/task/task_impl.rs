use {
    crate::task::Executor,
    alloc::{boxed::Box, sync::Arc, task::Wake},
    core::{
        ops::Deref,
        pin::Pin,
        sync::atomic::{AtomicUsize, Ordering},
        task::{Context, Poll, Waker},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(usize);

pub(super) struct Task {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
}

pub(super) struct TaskWaker {
    task_id: TaskId,
    executor: Pin<&'static Executor>,
}

impl Deref for TaskId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static + Send) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed)),
            future: Box::pin(future),
        }
    }
    pub fn poll(&mut self, cx: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(cx)
    }

    pub fn id(&self) -> TaskId {
        self.id
    }
}

impl TaskWaker {
    pub fn waker(task_id: TaskId, executor: Pin<&'static Executor>) -> Waker {
        Waker::from(Arc::new(Self { task_id, executor }))
    }

    fn wake_task(&self) {
        self.executor.schedule(self.task_id);
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
