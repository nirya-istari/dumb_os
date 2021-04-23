use core::{cell::UnsafeCell, fmt, ops::{Deref, DerefMut}, sync::atomic::{AtomicBool, Ordering}, task::Poll};

use alloc::sync::{Arc, Weak};
use crossbeam::queue::SegQueue;
use futures::{future::poll_fn, task::AtomicWaker};

#[derive(Default)]
pub struct Mutex<T> {
    locked: AtomicBool,
    wakers: SegQueue<Weak<AtomicWaker>>,
    data: UnsafeCell<T>,
}

pub struct MutexGuard<'a, T> {
    lock: &'a Mutex<T>,
}

pub struct OwnedMutexGuard<T> {
    lock: Arc<Mutex<T>>,
}

#[derive(Debug, Clone, Copy)]
pub enum TryLockError {
    Locked,
}

impl<T> Mutex<T> {
    pub const fn new(t: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            wakers: SegQueue::new(),
            data: UnsafeCell::new(t),
        }
    }

    fn inner_try_lock_(&self) -> Result<bool, bool> {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
    }
    fn inner_unlock_(&self) {
        self.locked.store(false, Ordering::Release);
    }

    /// Async function that returns once it's obtained the lock.
    /// Will attempt at least once before queue to save on memory.
    async fn wait_and_lock(&self) {
        if self.inner_try_lock_().is_ok() {
            // We have the lock!
            return;
        }

        let waker = Arc::new(AtomicWaker::new());
        let queue_waker = Arc::downgrade(&waker);
        let fut = poll_fn(|cx| {
            match self.inner_try_lock_() {
                Err(_) => {
                    waker.register(&cx.waker());                    
                    if self.inner_try_lock_().is_ok() {
                        waker.take();
                        Poll::Ready(())
                    } else {
                        Poll::Pending
                    }
                }
                Ok(_) => {
                    Poll::Ready(())
                }
            }
        });
        self.wakers.push(queue_waker);
        fut.await
    }

    pub async fn lock(&self) -> MutexGuard<'_, T> {
        self.wait_and_lock().await;
        MutexGuard { lock: self }        
    }

    pub async fn lock_owned(self: Arc<Self>) -> OwnedMutexGuard<T> {
        self.wait_and_lock().await;
        OwnedMutexGuard { lock: self }
    }

    pub fn try_lock(&self) -> Result<MutexGuard<'_, T>, TryLockError> {
        match self.inner_try_lock_() {
            Ok(_) => Ok(MutexGuard { lock: self }),
            Err(_) => Err(TryLockError::Locked),
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }

    pub fn try_lock_owned(self: Arc<Self>) -> Result<OwnedMutexGuard<T>, TryLockError> {
        match self.inner_try_lock_() {
            Ok(_) => Ok(OwnedMutexGuard { lock: self }),
            Err(_) => Err(TryLockError::Locked),
        }
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T> From<T> for Mutex<T> {
    fn from(t: T) -> Self {
        Self::new(t)
    }
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.inner_unlock_()
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: fmt::Debug> fmt::Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: fmt::Display> fmt::Display for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T> Drop for OwnedMutexGuard<T> {
    fn drop(&mut self) {
        self.lock.inner_unlock_()
    }
}

impl<T> Deref for OwnedMutexGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for OwnedMutexGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: fmt::Debug> fmt::Debug for OwnedMutexGuard<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: fmt::Display> fmt::Display for OwnedMutexGuard<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl fmt::Display for TryLockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TryLockError::Locked => write!(f, "mutex was locked"),
        }
    }
}
