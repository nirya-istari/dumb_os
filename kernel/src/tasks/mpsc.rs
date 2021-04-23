use alloc::sync::Arc;
use core::{fmt::Display, result::Result, sync::atomic::{AtomicBool, Ordering}, task::{Context, Poll}};
use crossbeam::queue::{ArrayQueue};
use futures::{Stream, future::poll_fn, task::AtomicWaker};

pub fn channel<T>(buffer: usize) -> (Sender<T>, Receiver<T>) {
    let channel = Arc::new(Channel {
        queue: ArrayQueue::new(buffer),
        waker: AtomicWaker::new(),
        closed: AtomicBool::default(),
    });
    (
        Sender {
            channel: channel.clone(),
        },
        Receiver { channel },
    )
}

struct Channel<T> {
    queue: ArrayQueue<T>,
    waker: AtomicWaker,
    closed: AtomicBool,
}

impl<T> core::fmt::Debug for Channel<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Channel")
            .field("queue", &format_args!("{}/{}", self.queue.len(), self.queue.capacity()))
            .field("waker", &self.waker)
            .finish()
    }
}

pub struct Sender<T> {
    channel: Arc<Channel<T>>,
}

impl<T> core::fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Sender")
         .field("channel", &self.channel)
         .finish()
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender { channel: self.channel.clone() }
    }
}

impl<T> Sender<T> {
    pub fn try_send(&mut self, message: T) -> Result<(), TrySendError<T>> {
        let channel = self.channel.as_ref();
        if channel.closed.load(core::sync::atomic::Ordering::Relaxed) {
            Err(TrySendError::Closed(message))
        } else {
            channel.queue.push(message).map_err(TrySendError::Full)?;
            channel.waker.wake();
            Ok(())
        }
    }

    pub fn is_closed(&self) -> bool {
        self.channel.closed.load(Ordering::Relaxed)
    }

    pub fn same_channel(&self, o: &Self) -> bool {
        Arc::as_ptr(&self.channel) == Arc::as_ptr(&o.channel)
    }

    pub fn capacity(&self) -> usize {
        self.channel.queue.capacity()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TrySendError<T>{
    Full(T),
    Closed(T)
}

impl<T> Display for TrySendError<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {                        
            TrySendError::Full(_) => write!(f, "channel full"),
            TrySendError::Closed(_) => write!(f, "channel closed")
        }
    }
}

// TODO. Implmented my std::error::Error wrapper from no-std-io

#[derive(Debug)]
pub struct Receiver<T> {
    channel: Arc<Channel<T>>,
}

impl<T> Receiver<T> {
    pub async fn recv(&mut self) -> Option<T> {
        poll_fn(|cx| self.poll_recv(cx)).await
    }

    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let channel = self.channel.as_ref();
        let closed =  channel.closed.load(Ordering::Relaxed);

        if let Some(item) = channel.queue.pop() {
            Poll::Ready(Some(item))
        } else if closed {
            Poll::Ready(None)
        } else {
            channel.waker.register(&cx.waker());
            if let Some(item) = channel.queue.pop() {
                channel.waker.take();
                Poll::Ready(Some(item))
            } else {
                Poll::Pending
            }
        }
    }

    pub fn into_stream(self) -> ReceiverStream<T> {
        ReceiverStream::new(self)
    }

    pub fn close(&mut self) {
        self.channel.closed.store(true, Ordering::Relaxed)
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.channel.closed.store(true, Ordering::Relaxed)
    }
}
impl<T> Unpin for Receiver<T> {}

#[derive(Debug)]
pub struct ReceiverStream<T> {
    receiver: Receiver<T>,
}

impl<T> ReceiverStream<T> {
    pub fn new (receiver: Receiver<T>) -> Self {
        Self { receiver }
    }    
    pub fn into_inner(self) -> Receiver<T> {
        self.receiver
    }    
}

impl<T> AsMut<Receiver<T>> for ReceiverStream<T> {
    fn as_mut(&mut self) -> &mut Receiver<T> {
        &mut self.receiver
    }
}

impl<T> AsRef<Receiver<T>> for ReceiverStream<T> {
    fn as_ref(&self) -> &Receiver<T> {
        &self.receiver
    }
}

impl<T> Stream for ReceiverStream<T> {
    type Item = T;

    fn poll_next(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {        
        self.as_mut().receiver.poll_recv(cx)
    }
}