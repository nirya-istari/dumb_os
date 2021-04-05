// main.rs

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(dumb_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use dumb_os::{allocator, memory::{BootInfoBumpAllocator}, prelude::*};
use x86_64::VirtAddr;

entry_point!(kernel_main);

fn kernel_main(bootinfo: &'static BootInfo) -> ! {
    let physical_memory_offset = VirtAddr::new(bootinfo.physical_memory_offset);

    println!("Hello World{}", "!");

    dumb_os::init();

    use x86_64::registers::control::Cr3;

    let (level_4_page_table, _) = Cr3::read();
    println!(
        "Level 4 page table at:{:?}",
        level_4_page_table.start_address()
    );
    // serial_println!("{:#?}", bootinfo);

    let mut mapper = unsafe { dumb_os::memory::init(physical_memory_offset) };

    let mut frame_allocator = unsafe { 
        BootInfoBumpAllocator::init(bootinfo) 
    };

    unsafe {
        allocator::init_heap(&mut mapper, &mut frame_allocator)
            .expect("Heap allocation failed");
    }

    // print_memory(physical_memory_offset.as_u64());

    #[cfg(test)]
    test_main();

    println!("It did not crash");

    dumb_os::halt();
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    dumb_os::halt();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    dumb_os::test_panic_handler(info)
}
