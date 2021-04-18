use super::{mpsc::Sender, Task, TaskId};
use crate::prelude::*;
use alloc::{collections::BTreeMap, sync::Arc};
use alloc::{prelude::v1::*, task::Wake};
use conquer_once::spin::OnceCell;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use crossbeam::queue::ArrayQueue;
use futures::Future;
use x86_64::instructions::interrupts;

pub struct Executor {
    task_queue: Arc<ArrayQueue<TaskId>>,
    tasks: BTreeMap<TaskId, Task>,
    waker_cache: BTreeMap<TaskId, Waker>,
    new_tasks: Arc<ArrayQueue<Task>>,
}

static NEW_TASK_QUEUE: OnceCell<Arc<ArrayQueue<Task>>> = OnceCell::uninit();

pub fn spawn_task(ts: Task) {
    NEW_TASK_QUEUE
        .get()
        .expect("Task queue not inialized")
        .push(ts)
        .expect("Task queue full");
}

pub fn spawn(fut: impl Future<Output = ()> + Send + 'static, desc: impl ToString) {
    spawn_task(Task::new(fut, desc));
}

impl Executor {
    pub fn new() -> Box<Executor> {
        let queue = NEW_TASK_QUEUE.get_or_init(|| Arc::new(ArrayQueue::new(128)));

        Box::new(Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(128)),
            waker_cache: BTreeMap::new(),
            new_tasks: queue.clone(),
        })
    }

    /// Attempt to spawn a task or return it back to the caller.
    pub fn spawn_task(&self, task: Task) -> Result<(), Task> {
        self.new_tasks.push(task)
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    pub fn sleep_if_idle(&mut self) {
        interrupts::disable();
        if self.task_queue.is_empty() && self.new_tasks.is_empty() {
            interrupts::enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }

    pub fn run_ready_tasks(&mut self) {
        let Self {
            tasks,
            task_queue,
            waker_cache,
            new_tasks,
        } = self;
        
        while let Some(task) = new_tasks.pop() {

            let task_id = task.id;
            if let Some(ref desc) = task.desc {
                serial_println!("new task: {}", desc);
            } else {
                serial_println!("new task: {:?}", task_id)
            }
            tasks.insert(task_id, task);
            task_queue.push(task_id).expect("Task queue overflowwing");
        }

        
        while let Some(task_id) = task_queue.pop() {            
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue,
            };
            if let Some(ref desc) = task.desc {
                serial_println!("running task: {}", desc);
            }
            let waker: &mut Waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));
            let mut context = Context::from_waker(&waker);

            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    if let Some(ref desc) = task.desc {
                        serial_println!("task completed: {}", desc);
                    } else {
                        serial_println!("task completed: {:?}", task_id);
                    }
                    // I think NLL saves us here. But I have no idea how this works re. lifetimes.

                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }

    pub fn run_simple(&mut self) {
        while let Some(task_id) = self.task_queue.pop() {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);

            let task = self.tasks.get_mut(&task_id).expect("Task missing");

            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    serial_println!("Task {:?} completed.", task_id);
                }
                Poll::Pending => {
                    self.task_queue.push(task_id).expect("Tasks queue is full.");
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpawnHandle {
    tx: Sender<Task>,
}
impl SpawnHandle {
    pub fn spawn(&mut self, future: impl Future<Output = ()> + Send + 'static, desc: impl ToString) {
        let task = Task::new(future, desc);
        self.tx.try_send(task).expect("Spawn channel closed");
    }
}

pub fn yield_task() -> impl Future<Output = ()> + 'static {
    Yield { yielded: false }
}

#[derive(Debug)]
struct Yield {
    yielded: bool,
}
impl Future for Yield {
    type Output = ();

    fn poll(mut self: core::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}

impl TaskWaker {
    fn wake_task(&self) {
        self.task_queue.push(self.task_id).ok();
    }

    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }
}

fn no_op(_: *const ()) {}
fn clone(_: *const ()) -> RawWaker {
    dummy_raw_waker()
}
static DUMMY_RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(clone, no_op, no_op, no_op);

fn dummy_raw_waker() -> RawWaker {
    // 0x4 is the address represening allocated zero sized types.
    RawWaker::new(0x4 as *const (), &DUMMY_RAW_WAKER_VTABLE)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}
