// memory.rs

use crate::prelude::*;
use x86_64::{VirtAddr, registers::control::Cr3, structures::paging::{OffsetPageTable, PageTable, PageTableFlags}};

pub unsafe fn init(physical_addr_offset: VirtAddr) -> OffsetPageTable<'static>
{
    let level_4_page_table = active_level_4_table(physical_addr_offset);
    OffsetPageTable::new(level_4_page_table, physical_addr_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) 
    -> &'static mut PageTable 
{    
    let (level_4_table_frame, _flags) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt: VirtAddr = physical_memory_offset + phys.as_u64();

    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr // Unsafe
}







fn print_memory(physical_memory_offset: u64) 
{
        // Print all the used non-leaf tables to serial.
        let page_table = unsafe { 
            active_level_4_table(VirtAddr::new(physical_memory_offset)) 
        };

        for (i, entry) in page_table.iter().enumerate() {
            if entry.is_unused() == false {
                serial_println!("L4 {}: {:?}", i, entry);
    
                let phys = entry.frame().unwrap().start_address();
                let virt = phys.as_u64() + physical_memory_offset;
                let ptr = VirtAddr::new(virt).as_mut_ptr();
                let l3_table: &PageTable = unsafe {&*ptr};
    
                for (j, l3_entry) in l3_table.iter().enumerate() {
                    if !l3_entry.is_unused() {
                        serial_println!("  L3  {}: {:?}", j, l3_entry);
    
                        let phys = l3_entry.frame().unwrap().start_address();
                        let virt = phys.as_u64() + physical_memory_offset;
                        let ptr = VirtAddr::new(virt).as_mut_ptr();
                        let l2_table: &PageTable = unsafe {&*ptr};
    
                        for (k, l2_entry) in l2_table.iter().enumerate() {
                            if !l2_entry.is_unused() && !l2_entry.flags().contains(PageTableFlags::HUGE_PAGE) {                            
                                serial_println!("    L2 {}: {:?}", k, l2_entry);
                            }
                        }
                    }
                }
    
            }
        }
}