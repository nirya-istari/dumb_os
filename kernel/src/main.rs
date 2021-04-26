// src/main.rs

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(alloc_prelude)]
#![test_runner(dumb_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::format;
use alloc::prelude::v1::*;

use core::panic::PanicInfo;

use bootloader::{
    boot_info::{BootInfo, Optional},
    entry_point,
};
use rand::prelude::*;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;
use x86_64::VirtAddr;

use dumb_os::allocator::HEAP_SIZE;
use dumb_os::memory::BootInfoBumpAllocator;
use dumb_os::tasks::executor::Executor;
use dumb_os::tasks::keyboard::print_keypresses;
use dumb_os::tasks::timer;
use dumb_os::tasks::Task;
use dumb_os::{
    allocator,
    tasks::{executor::spawn, timer::sleep},
};
use dumb_os::{print, println};

entry_point!(kernel_main);

fn kernel_main(bootinfo: &'static mut BootInfo) -> ! {
    let mut rng = Pcg64::new(0xcafef00dd15ea5e5, 0xa02bdbf7bb3c0a7ac28fa16a64abf96);

    dumb_os::init();
    check_bootinfo(bootinfo);

    let physical_memory_offset = VirtAddr::new(
        bootinfo
            .physical_memory_offset
            .into_option()
            .expect("no physical memory offset"),
    );

    print!("Filling in frame buffer...");
    let mut fb = core::mem::replace(&mut bootinfo.framebuffer, Optional::None)
        .into_option()
        .unwrap();
    let info = fb.info();

    for (r, row) in fb.buffer_mut().chunks_mut(info.stride).enumerate() {
        for (c, pixel) in row
            .chunks_mut(info.bytes_per_pixel)
            .take(info.vertical_resolution)
            .enumerate()
        {
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

    print!("Initializing heap");
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap allocation failed");
    println!(" OK");

    #[cfg(test)]
    test_main();

    let acpi = dumb_os::acpi::init(physical_memory_offset, bootinfo)
        .map_err(|err| panic!("Failed to inialized acpi: {:?}", err));

    let mut executor = Executor::new();

    let (timer_task, _timer_handle) = unsafe { timer::init() };

    executor.spawn_task(timer_task).unwrap();
    executor
        .spawn_task(Task::new(print_keypresses(), "print keypresses"))
        .unwrap();
    // executor.spawn_task(Task::new(disk_main(), "disk main")).unwrap();

    executor
        .spawn_task(Task::new(example_task::<Pcg64>(rng.gen()), "example task"))
        .unwrap();
    // executor
    // .spawn_task(Task::new(example_timer(rng.gen() ), "example timer"))
    // .unwrap();

    executor.run()
}

fn check_bootinfo(bootinfo: &BootInfo) -> () {
    if bootinfo.physical_memory_offset.as_ref().is_none() {
        panic!("physical memory offset is required");
    }
    if bootinfo.rsdp_addr.as_ref().is_none() {
        panic!("rsdp addr is required");
    }
    let mut usuable_bytes = 0;

    for (i, region) in bootinfo.memory_regions.iter().enumerate() {
        println!("Region {}: {:?}", i, region);
        if region.kind == bootloader::boot_info::MemoryRegionKind::Usable {
            usuable_bytes += region.end - region.start;
        }
    }
    if usuable_bytes < HEAP_SIZE {
        panic!(
            "Not enough usuable memory. Require {}, have {}",
            HEAP_SIZE, usuable_bytes
        );
    }
    println!("Have {} bytes of memory", usuable_bytes)
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
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use dumb_os::io::{stdout, Write};
    // We're panicing nothing else will be printing.
    let stdout = stdout();
    let mut out = unsafe { stdout.break_lock() };
    writeln!(out, "{}", info).ok();
    dumb_os::halt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    dumb_os::test_panic_handler(info)
}
