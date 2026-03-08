use {
    alloc::{
        boxed::Box,
        collections::{btree_map::BTreeMap, vec_deque::VecDeque},
        sync::Arc,
        task::Wake,
    },
    core::{
        ops::Deref,
        pin::Pin,
        sync::atomic::{AtomicU8, AtomicUsize, Ordering},
        task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    },
    klib_macros::klib_hart_local,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(usize);

const TASK_STATE_READY: u8 = 0;
const TASK_STATE_WAITING: u8 = 1;
const TASK_STATE_RUNNING: u8 = 2;

// static GLOBAL_QUEUE: VecDeque<Arc<Task>> = VecDeque::new();

pub struct Executor {
    // tasks: BTreeMap<TaskId, Task>,
    // queue: VecDeque<Task>,
    // waker_cache: BTreeMap<TaskId, Waker>,
    queue: VecDeque<Task>,
}

pub struct Task {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Deref for TaskId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed)),
            future: Box::pin(future),
        }
    }
    fn poll(&mut self, cx: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(cx)
    }
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            queue: VecDeque::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        self.queue.push_back(task)
    }

    pub fn run(&mut self) {
        while let Some(mut task) = self.queue.pop_front() {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {}
                Poll::Pending => self.queue.push_back(task),
            }
        }
    }
}

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(0 as *const (), vtable)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}

// pub fn run() {
//     loop {
//         while let Some(task) = GLOBAL_QUEUE.with_mut(|q| q.pop_front()) {
//             //task.state.store(TASK_STATE_RUNNING, Ordering::Relaxed);

//             let waker = Waker::from(task.clone());
//             let mut cx = Context::from_waker(&waker);

//             let future = unsafe { task.future.as_mut() };

//             match future.poll(&mut cx) {
//                 Poll::Ready(()) => {}
//                 Poll::Pending => {
//                     task.state.store(TASK_STATE_WAITING, Ordering::Relaxed);
//                 }
//             }
//         }

//         unsafe { core::arch::asm!("hlt") }
//     }
// }
