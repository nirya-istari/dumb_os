// src/tasks.rs

pub mod executor;
pub mod keyboard;

use core::{future::Future, pin::Pin, sync::atomic::{AtomicU64, Ordering}, task::{Context, Poll}};
use alloc::prelude::v1::*;

static TASK_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
struct TaskId(u64);
impl TaskId {
    fn new() -> TaskId {
        TaskId(TASK_COUNTER.fetch_add(1, Ordering::Relaxed))        
    }
}

pub struct Task {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()>>>
}

impl core::fmt::Debug for Task {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl Task {    
    pub fn new(ts: impl Future<Output = ()> + 'static) -> Task {
        Task {
            id: TaskId::new(),
            future: Box::pin(ts)
        }
    }

    pub fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

pub fn yield_task() -> impl Future<Output = ()> {
    executor::Executor::yield_task()
}