// src/allocator/mod.rs

use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

pub const HEAP_START: u64 = 0x_4444_4444_0000;
pub const HEAP_SIZE: u64 = 128 * 1024; // 128 KiB

#[global_allocator]
static ALLOCATOR: EpsilonAllocatorLocked = EpsilonAllocatorLocked::new();

struct DummyAllocator;

unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("Should not happen");
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout);
}

pub unsafe fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let heap_start = VirtAddr::new(HEAP_START);
    let page_range = {
        let heap_last_addr = VirtAddr::new(HEAP_START + HEAP_SIZE - 1);
        let heap_start_page = Page::containing_address(heap_start);
        let heap_last_page = Page::containing_address(heap_last_addr);
        Page::range_inclusive(heap_start_page, heap_last_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        mapper.map_to(page, frame, flags, frame_allocator)?.flush();
    }

    let start = page_range.start.start_address();

    {
        let allocator: EpsilonAllocator = EpsilonAllocator {
            _start: start.as_u64(),
            next: start.as_u64(),
            remaining: HEAP_SIZE,
        };
        let mut lock = ALLOCATOR.alloc.lock();
        *lock = Some(allocator);
    }

    Ok(())
}

struct EpsilonAllocatorLocked {
    alloc: Mutex<Option<EpsilonAllocator>>,
}
impl EpsilonAllocatorLocked {
    const fn new() -> EpsilonAllocatorLocked {
        EpsilonAllocatorLocked {
            alloc: Mutex::new(None),
        }
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
