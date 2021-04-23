// tests/stack_overflow.rs

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use volatile::Volatile;
use dumb_os::serial_print;
use dumb_os::qemu::{ExitCode, exit_qemu};
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(test_double_fault_handler)
                .set_stack_index(dumb_os::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt
    };
}
fn init_test_idt() {
    TEST_IDT.load();
}
extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_print!("[ok]\n");
    exit_qemu(ExitCode::Success);
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_print!("stack_overflow::stack_overflow... ");

    dumb_os::gdt::init();
    init_test_idt();

    stack_overflow();

    panic!("Continued after stack_overflow");
    
}

#[allow(unconditional_recursion)]
pub fn stack_overflow() {
    // Use some memory.
    let x = Volatile::new(0);
    x.read();
    stack_overflow();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    dumb_os::test_panic_handler(info)
}

