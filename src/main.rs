// src/main.rs

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(dumb_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;
use core::panic::PanicInfo;
use dumb_os::allocator;
use dumb_os::tasks::Task;
use dumb_os::tasks::executor::Executor;
use dumb_os::tasks::keyboard::print_keypresses;
use dumb_os::prelude::*;
use dumb_os::memory::BootInfoBumpAllocator;
use x86_64::VirtAddr;

entry_point!(kernel_main);

fn kernel_main(bootinfo: &'static BootInfo) -> ! {

    // Same seed for testing
    let mut rng = Pcg64::new(0xcafef00dd15ea5e5, 0xa02bdbf7bb3c0a7ac28fa16a64abf96);

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

    let mut frame_allocator = unsafe { BootInfoBumpAllocator::init(bootinfo) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap allocation failed");

    #[cfg(test)]
    test_main();

    let mut executor = Executor::new();
    executor.spawn(Task::new(print_keypresses()));
    executor.spawn(Task::new(example_task::<Pcg64>(rng.gen() )));
    executor.run()
}

async fn async_number() -> u32 {
    42
}

async fn example_task<R: Rng+SeedableRng>(seed: R::Seed) {
    let mut rng = R::from_seed(seed);
    let id = rng.next_u32();    
    println!("{}. async number: {}", id, async_number().await);    
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    dumb_os::halt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    dumb_os::test_panic_handler(info)
}
