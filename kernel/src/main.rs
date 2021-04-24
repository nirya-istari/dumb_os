// src/main.rs

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(alloc_prelude)]
#![test_runner(dumb_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::prelude::v1::*;
use alloc::format;

use core::panic::PanicInfo;

use bootloader::{entry_point, boot_info::{BootInfo, Optional}};
use rand::{Rng, SeedableRng};
use rand::prelude::*;
use rand_pcg::Pcg64;
use x86_64::VirtAddr;

use dumb_os::io::{Write, stdout};
use dumb_os::{memory::BootInfoBumpAllocator};
use dumb_os::{print, println};
use dumb_os::tasks::executor::Executor;
use dumb_os::tasks::keyboard::print_keypresses;
use dumb_os::tasks::timer;
use dumb_os::tasks::Task;
use dumb_os::{
    allocator,
    tasks::{executor::spawn, timer::sleep},
};
use dumb_os::disk::disk_main;



entry_point!(kernel_main);
/* #[export_name = "_start"]
pub extern "C" fn __impl_start(boot_info: &'static bootloader::boot_info::BootInfo) -> ! {
    let f: fn(&'static bootloader::boot_info::BootInfo) -> ! = kernel_main;
    f(boot_info)
}*/

fn kernel_main(bootinfo: &'static mut BootInfo) -> ! {
    let mut rng = Pcg64::new(0xcafef00dd15ea5e5, 0xa02bdbf7bb3c0a7ac28fa16a64abf96);
    
    let physical_memory_offset = VirtAddr::new(
        bootinfo
            .physical_memory_offset
            .into_option()
            .expect("no physical memory offset")
    );
    

    dumb_os::init();

    print!("Filling in frame buffer...");
    let mut fb = core::mem::replace(&mut bootinfo.framebuffer, Optional::None)
        .into_option().unwrap();
    let info = fb.info();

    for (r, row) in fb.buffer_mut().chunks_mut(info.stride).enumerate() {
        for (c, pixel) in row.chunks_mut(info.bytes_per_pixel).take(info.vertical_resolution).enumerate() {
            pixel[0] = ((r * 255) / info.vertical_resolution) as u8;
            pixel[1] = ((c * 255) / info.horizontal_resolution) as u8;
        }
    }
    println!(" OK");

    // Same seed for testing
    



    use x86_64::registers::control::Cr3;

    let (level_4_page_table, _) = Cr3::read();
    println!(
        "Level 4 page table at:{:?}",
        level_4_page_table.start_address()
    );
    // println!("{:#?}", bootinfo);

    print!("Initializing Page mapper...");
    let mut mapper = unsafe { dumb_os::memory::init(physical_memory_offset) };
    println!(" OK");

    print!("Initializing frame allocator");
    let mut frame_allocator = unsafe { BootInfoBumpAllocator::init(bootinfo) };
    println!(" OK");

    panic!("Testing backtrace");

    print!("Initializing heap");
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap allocation failed");
    println!(" OK");

    #[cfg(test)]
    test_main();

    let mut executor = Executor::new();

    let (timer_task, _timer_handle) = unsafe { timer::init() };

    executor.spawn_task(timer_task).unwrap();
    executor.spawn_task(Task::new(print_keypresses(), "print keypresses")).unwrap();
    executor.spawn_task(Task::new(disk_main(), "disk main")).unwrap();

    executor
        .spawn_task(Task::new(example_task::<Pcg64>(rng.gen()), "example task" ))
        .unwrap();
    /* executor
        .spawn_task(Task::new(example_timer(rng.gen() ), "example timer"))
        .unwrap();     */

    executor.run()
}

async fn async_number() -> u32 {
    42
}

async fn example_task<R: Rng + SeedableRng>(seed: R::Seed) {
    let mut rng = R::from_seed(seed);
    let id = rng.next_u32();
    println!("{}. async number: {}", id, async_number().await);
}

#[allow(dead_code)]
async fn example_timer(seed: <Pcg64 as SeedableRng>::Seed) {
    let mut rng = Pcg64::from_seed(seed);

    let mut arr: Vec<u64> = (1..=20).map(|i| i * 10).collect();

    arr.shuffle(&mut rng);

    for i in arr {
        spawn(wait_and_print(i), format!("wait and print: {}", i));
    }
}

async fn wait_and_print(i: u64) {
    sleep(i).await;
    println!("Waited {} ticks", i);
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // We're panicing nothing else will be printing.
    let stdout = stdout();
    let mut out = unsafe { stdout.break_lock() };

    writeln!(out, "{}", info).ok();
    writeln!(out, "Stack track:").ok();
    dumb_os::halt_loop();
}


/* #[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    dumb_os::test_panic_handler(info)
}
 */
