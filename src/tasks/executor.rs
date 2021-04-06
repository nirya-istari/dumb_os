use super::{Task, TaskId};
use crate::prelude::*;
use alloc::{collections::BTreeMap, sync::Arc};
use alloc::{prelude::v1::*, task::Wake};
use futures::Future;
use x86_64::instructions::interrupts;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use crossbeam::queue::ArrayQueue;

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn new() -> Box<Executor> {
        Box::new(Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(128)),
            waker_cache: BTreeMap::new(),
        })
    }

    pub fn spawn(&mut self, task: Task) {
        serial_println!("Task {:?} spawned.", task.id);
        let task_id = task.id;
        self.tasks
            .insert(task_id, task)
            .expect_none("Task already spawned");
        self.task_queue.push(task_id).ok();
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    pub fn sleep_if_idle(&mut self)  {
        interrupts::disable();
        if self.task_queue.is_empty() {
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
        } = self;

        while let Some(task_id) = task_queue.pop() {
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue,
            };
            let waker: &mut Waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));
            let mut context = Context::from_waker(&waker);

            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    serial_println!("Task {:?} completed", task_id);
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
            let waker = dummy_water();
            let mut context = Context::from_waker(&waker);

            let task = self.tasks.get_mut(&task_id).expect("Task missing");

            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    serial_println!("Task {:?} completed.", task_id);
                }
                Poll::Pending => {
                    self.task_queue
                        .push(task_id)
                        .expect("Tasks queue is full.");
                }
            }
        }
    }

    pub(super) fn yield_task() -> impl Future<Output = ()> {
        Yield { yielded: false }
    }
}

struct Yield {
    yielded: bool
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

fn dummy_water() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}
