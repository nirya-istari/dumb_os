// src/allocator/mod.rs

pub mod dummy;
pub mod epsilon;
// pub mod list_allocator;

#[cfg(epsilon)]
use epsilon::EpsilonAllocatorLocked;
use linked_list_allocator::LockedHeap;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

pub const HEAP_START: u64 = 0x4444_4444_0000;
pub const HEAP_SIZE: u64 = 64 * 1024; // 64 KiB

#[cfg(feature = "linked_list_allocator")]
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[cfg(feature = "epsilon_allocator")]
#[global_allocator]
static ALLOCATOR: EpsilonAllocatorLocked = EpsilonAllocatorLocked::new();

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout);
}

pub fn init_heap(
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
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush();
        }
    }

    let start = page_range.start.start_address();
    unsafe {
        ALLOCATOR
            .lock()
            .init(start.as_u64() as usize, HEAP_SIZE as usize);
    }

    Ok(())
}
