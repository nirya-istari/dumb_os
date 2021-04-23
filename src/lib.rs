// src/lib.rs

// TMP:
#![allow(dead_code)]

#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(alloc_prelude)]
#![feature(asm)]
#![feature(try_reserve)]

extern crate alloc;

use core::panic::PanicInfo;

use x86_64::instructions::port::Port;

pub mod allocator;
pub mod disk;
pub mod gdt;
pub mod irq;
pub mod memory;
pub mod prelude;
pub mod qemu;
pub mod serial;
pub mod sync;
pub mod tasks;
pub mod vga_buffer;

pub fn init() {
    gdt::init();
    irq::init();
    unsafe { irq::pic_8256::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) -> () {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    qemu::exit_qemu(qemu::ExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    qemu::exit_qemu(qemu::ExitCode::Failed);
    halt_loop();
}

pub fn halt_loop() -> ! {
    use x86_64::instructions::hlt;
    loop {
        hlt();
    }
}

/// Entry point for `cargo test`
#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    init();
    test_main();
    halt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

/// Delays a short time writing to Port 0x80.
pub fn delay(microseconds: u32) {
    unsafe {
        // Writing to port 0x80 does nothing but wastes time.
        for _ in 0..microseconds {
            Port::<u8>::new(0x80).write(0);
        }
    }
}