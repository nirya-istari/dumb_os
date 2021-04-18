

use core::{pin::Pin, sync::atomic::{AtomicU64, Ordering}, task::{Context, Poll}};
use alloc::{collections::BinaryHeap, sync::Arc};
use conquer_once::spin::OnceCell;
use futures::{Future, Stream, StreamExt, stream::select, task::{AtomicWaker}};
use crate::prelude::*;

use super::{Task, mpsc::{self, Receiver, Sender}};

static CURRENT_TICK: AtomicU64 = AtomicU64::new(0);

/// Get the last tick that occurred.
pub fn current_tick() -> u64 {
    CURRENT_TICK.load(Ordering::SeqCst)
}

/// Called from interrupt.
pub(crate) fn next_tick() {
    CURRENT_TICK.fetch_add(1, Ordering::SeqCst);
    MASTER_WAKER.wake();
}

#[derive(Debug)]
enum TimerEvent {
    Tick(u64),
    NewTask(PendingTimer)
}

static MASTER_WAKER: AtomicWaker = AtomicWaker::new();
static SHARED_HANDLE: OnceCell<TimerHandle> = OnceCell::uninit();

pub unsafe fn init() -> (Task, TimerHandle) {
    let master_stream = MasterTickStream { last_tick: 0 };
    let (tx, rx) = mpsc::channel(32);

    let handle = TimerHandle { send: tx };
    SHARED_HANDLE.init_once(|| handle.clone());

    (Task::no_desc(timer_main(master_stream,  rx)), handle)
}

async fn timer_main(
    ticks: MasterTickStream,
    new_tasks: Receiver<PendingTimer>
) {    
    let mut stream = select(
        ticks.map(TimerEvent::Tick) ,
        new_tasks.into_stream().map(TimerEvent::NewTask)
        );

    let mut queue: BinaryHeap<PendingTimer> = BinaryHeap::new();

    while let Some(ev) = stream.next().await {
        match ev {
            TimerEvent::Tick(tick) => {
                print!(".");
                while queue.peek().map(|t| t.tick <= tick).unwrap_or(false) {
                    let t = queue.pop().unwrap();
                    t.waker.wake();
                }
            }
            TimerEvent::NewTask(task) => {
                queue.push(task)
            }
        }

    }
}

#[derive(Debug)]
struct PendingTimer {
    tick: u64,
    waker: Arc<AtomicWaker>,
}

impl PendingTimer {
    fn new(tick: u64) -> PendingTimer {
        PendingTimer {
            tick,
            waker: Arc::new(AtomicWaker::new()),
        }
    }
}

impl Ord for PendingTimer {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // Reversed ordering as alloc's binary heap is a Max-heap and we need a min-heap
        other.tick.cmp(&self.tick)        
    }
}
impl PartialOrd for PendingTimer {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for PendingTimer {}
impl PartialEq for PendingTimer {
    fn eq(&self, other: &Self) -> bool {
        self.tick == other.tick
    }
}

#[derive(Debug, Clone)]
pub struct TimerHandle {
    send: Sender<PendingTimer>
}

impl TimerHandle {
    pub fn sleep(&mut self, ticks: u64) -> impl Future<Output = ()> {
        let sleep = Sleep::new(ticks, self.send.clone());
        sleep
    }
}

pub fn sleep(ticks: u64) -> impl Future<Output = ()> {
    SHARED_HANDLE.get().expect("Timer task not initalized")
        .clone()
        .sleep(ticks)
}

struct MasterTickStream {
    last_tick: u64,
}
impl Stream for MasterTickStream {
    type Item = u64;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let tick = current_tick();

        if self.last_tick < tick {
            self.get_mut().last_tick = tick;
            Poll::Ready(Some(tick))
        } else {
            MASTER_WAKER.register(&cx.waker());
            // Race conditions... YAY!
            let tick = current_tick();
            if self.last_tick < tick {
                MASTER_WAKER.take();
                self.get_mut().last_tick = tick;
                Poll::Ready(Some(tick))
            } else {
                Poll::Pending
            }
        }
    }
}


struct Sleep {
    tick: u64,
    waker: Arc<AtomicWaker>
}

impl Sleep {
    fn new(ticks: u64, mut register: Sender<PendingTimer>) -> Sleep {
        let now = current_tick();
        let tick = now + ticks;
        let pending_timer = PendingTimer::new(tick);
        let res = Sleep { tick, waker: pending_timer.waker.clone() };
        register.try_send(pending_timer).unwrap();
        res
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let tick = current_tick();
        if self.tick <= tick {
            Poll::Ready(())
        } else {
            self.waker.register(&cx.waker());
            Poll::Pending
        }
    }
}
