// src/allocator/epsilon.rs

use core::alloc::{GlobalAlloc, Layout};
use spin::lock_api::Mutex;
use x86_64::VirtAddr;

pub struct EpsilonAllocatorLocked {
    alloc: Mutex<Option<EpsilonAllocator>>,
}
impl EpsilonAllocatorLocked {
    pub const fn new() -> EpsilonAllocatorLocked {
        EpsilonAllocatorLocked {
            alloc: Mutex::const_new(spin::Mutex::new(()),  None),
        }
    }

    pub fn init(&self, addr: VirtAddr, size: u64) {
        let mut g = self.alloc.lock();
        if g.is_some() {
            panic!("Allocator already initialized");
        }
        *g = Some(EpsilonAllocator {
            _start: addr.as_u64(),
            next: addr.as_u64(),
            remaining: size,
        });
    }
}

struct EpsilonAllocator {
    _start: u64,
    next: u64,
    remaining: u64,
}

impl EpsilonAllocator {
    unsafe fn bump(&mut self, size: u64) -> bool {
        let next = self.next as *mut u8;
        if size > self.remaining {
            false
        } else {
            self.next = next.offset(size as isize) as u64;
            self.remaining -= size;
            true
        }
    }
}

unsafe impl GlobalAlloc for EpsilonAllocatorLocked {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut guard = self.alloc.lock();
        match *guard {
            Some(ref mut a) => {
                let next = a.next as *mut u8;
                let align_skip = next.align_offset(layout.align());

                if a.bump(align_skip as u64) {
                    let result = a.next as *mut u8;
                    if a.bump(layout.size() as u64) {
                        result
                    } else {
                        core::ptr::null_mut()
                    }
                } else {
                    core::ptr::null_mut()
                }
            }
            None => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // YOLO.
    }
}
