// src/tasks.rs

pub mod executor;
pub mod keyboard;
pub mod timer;
pub mod mpsc;

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
    desc: Option<String>,
    future: Pin<Box<dyn Future<Output = ()> + Send>>
}

impl core::fmt::Debug for Task {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl Task {    
    pub fn new(ts: impl Future<Output = ()> + Send + 'static, desc: impl ToString) -> Task {
        Task {
            id: TaskId::new(),
            future: Box::pin(ts),
            desc: Some(desc.to_string()),
        }
    }

    pub fn no_desc(ts: impl Future<Output = ()> + Send + 'static) -> Task {
        Task {
            id: TaskId::new(),
            future: Box::pin(ts),
            desc: None
        }
    }
    

    pub fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

pub fn yield_task() -> impl Future<Output = ()> {
    executor::yield_task()
}