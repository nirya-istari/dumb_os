// main.rs

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(dumb_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use dumb_os::{memory::{self, BootInfoBumpAllocator, create_example_mapping, print_memory}, prelude::*};
use x86_64::{VirtAddr, structures::paging::{Page, Translate}};

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
    serial_println!("{:#?}", bootinfo);

    let mut mapper = unsafe { memory::init(physical_memory_offset) };

    let addresses = [
        // VGA buffer
        0xb8000,
        // Some random code page
        0x201008,
        // Somewhere on the stack
        0x0100_0020_1a10,
        // The physical memory map.
        bootinfo.physical_memory_offset,
    ];

    for &address in addresses.iter() {
        let virt = VirtAddr::new(address);

        let phys = mapper.translate_addr(virt);
        serial_println!("{:?} -> {:?}", virt, phys);
    }

    let mut frame_allocator = unsafe { 
        BootInfoBumpAllocator::init(bootinfo) 
    };

    print_memory(physical_memory_offset.as_u64());

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
