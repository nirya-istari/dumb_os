// src/task/keyboard.rs

use core::{pin::Pin, task::Poll};
use conquer_once::spin::OnceCell;
use crossbeam::queue::ArrayQueue;
use futures::{stream::{Stream, StreamExt}, task::AtomicWaker};
use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts::Us104Key};
use crate::prelude::*;


static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING scancode queue is full. Dropping input");
        } else {
            WAKER.wake();
        }
    } else {
        println!("WARNING scancode queue uninitialized.");
    }
}

pub struct ScanCodeStream {
    _private: (),
}

impl ScanCodeStream {
    pub fn new() -> ScanCodeStream {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("Failed to initialze scancode stream");
        ScanCodeStream { _private: () }
    }
}

impl Stream for ScanCodeStream {
    type Item = u8;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {                

        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("SCANCODE_QUEUE not initialized");

        // Fast path.
        if let Some(code) = queue.pop() {            
            Poll::Ready(Some(code))
        } else {
            WAKER.register(&cx.waker());
            if let Some(code) = queue.pop() {                
                WAKER.take();
                Poll::Ready(Some(code))
            } else {
                Poll::Pending
            }
        }
    }
}

pub async fn print_keypresses() {
    let mut scancodes = ScanCodeStream::new();
    let mut keyboard = Keyboard::new(Us104Key, ScancodeSet1, HandleControl::Ignore);    

    while let Some(code) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(code) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::RawKey(key) => print!("<{:?}>", key),
                    DecodedKey::Unicode(character) => print!("{}", character),
                }
            }
        }
    }
}