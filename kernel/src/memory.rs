// src/memory.rs


use crate::prelude::*;

use bootloader::{BootInfo, boot_info::MemoryRegionKind};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageSize, PageTable, PageTableFlags,
        PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

pub unsafe fn init(physical_addr_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_page_table = active_level_4_table(physical_addr_offset);
    OffsetPageTable::new(level_4_page_table, physical_addr_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _flags) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt: VirtAddr = physical_memory_offset + phys.as_u64();

    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr // Unsafe
}

pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    let mapping_result = unsafe { mapper.map_to(page, frame, flags, frame_allocator) };

    mapping_result.expect("map_to failed").flush();
}

pub fn print_memory(physical_memory_offset: u64) {
    // Print all the used non-leaf tables to serial.
    let page_table = unsafe { active_level_4_table(VirtAddr::new(physical_memory_offset)) };

    for (i, entry) in page_table.iter().enumerate() {
        if entry.is_unused() == false {
            println!("L4 {}: {:?}", i, entry);

            let phys = entry.frame().unwrap().start_address();
            let virt = phys.as_u64() + physical_memory_offset;
            let ptr = VirtAddr::new(virt).as_mut_ptr();
            let l3_table: &PageTable = unsafe { &*ptr };

            for (j, l3_entry) in l3_table.iter().enumerate() {
                if !l3_entry.is_unused() {
                    println!("  L3  {}: {:?}", j, l3_entry);

                    let phys = l3_entry.frame().unwrap().start_address();
                    let virt = phys.as_u64() + physical_memory_offset;
                    let ptr = VirtAddr::new(virt).as_mut_ptr();
                    let l2_table: &PageTable = unsafe { &*ptr };

                    for (k, l2_entry) in l2_table.iter().enumerate() {
                        if !l2_entry.is_unused()
                            && !l2_entry.flags().contains(PageTableFlags::HUGE_PAGE)
                        {
                            println!("    L2 {}: {:?}", k, l2_entry);
                        }
                    }
                }
            }
        }
    }
}

pub struct EmptyFrameAllocator;
unsafe impl<S> FrameAllocator<S> for EmptyFrameAllocator
where
    S: PageSize,
{
    fn allocate_frame(&mut self) -> Option<PhysFrame<S>> {
        None
    }
}

pub struct BootInfoBumpAllocator {
    bootinfo: &'static BootInfo,
    next: usize,
}

impl BootInfoBumpAllocator {
    pub unsafe fn init(bootinfo: &'static BootInfo) -> BootInfoBumpAllocator {
        BootInfoBumpAllocator { bootinfo, next: 0 }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // iter over all memory regions
        self.bootinfo.memory_regions.iter()
            // Filter by Usuable memory regions
            .filter(|region| region.kind == MemoryRegionKind::Usable)
            // Extract the ranges and iterator over frame numbers
            .flat_map(|region| region.start .. region.end)
            // Convert frame numbers to addresses
            .map(|addr| PhysAddr::new(addr))
            // Create PhysFrame value from address
            .map(|address|  PhysFrame::containing_address(address))        
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoBumpAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let next = self.usable_frames().nth(self.next);
        if next.is_some() {            
            self.next += 1;
        }
        next
    }
}

