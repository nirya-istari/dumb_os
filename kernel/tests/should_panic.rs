// tests/should_panic.rs

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use dumb_os::{qemu};
use dumb_os::prelude::*;
use qemu::{exit_qemu, ExitCode};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_panic();
    println!("[did not panic]");
    exit_qemu(ExitCode::Failed);
    loop {}
}

fn test_panic() {
    print!("should_panic::test_panic...");
    
    panic!("test panic works");
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("[ok]");
    qemu::exit_qemu(qemu::ExitCode::Success);
    loop {}
}
