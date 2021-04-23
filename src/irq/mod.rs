// src/irq/irq.rs

pub mod pic_8256;
// pub mod apic;

use crate::{gdt, halt_loop, tasks::timer};
use crate::prelude::*;
use crate::disk::ata;
use lazy_static::lazy_static;
use x86_64::instructions::{port::Port};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use self::pic_8256::PICS;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);

        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);

        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);

        idt[InterruptIndex::PrimaryATA.as_usize()].set_handler_fn(primary_ata_handler);
        idt[InterruptIndex::SecondaryATA.as_usize()].set_handler_fn(secondary_ata_handler);

        idt
    };
}

pub fn init() {
    IDT.load();
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    // PIC1
    Timer = pic_8256::PIC1_OFFSET,
    Keyboard,
    SecondaryPic,
    SerialPort2,
    SerialPort1,
    ParallelPort23,
    FloppyDisk,
    ParallelPort1,
    // PIC2
    RTC,
    ACPI,
    _Availabe1,
    _Availabe2,
    Mouse,
    CoProcessor,
    PrimaryATA,
    SecondaryATA,
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
    pub fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: Breakpoint\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!("DOUBLE FAULT. Code {}:\n{:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:?}", stack_frame);

    serial_println!("EXCEPTION: PAGE FAULT");
    serial_println!("Accessed Address: {:?}", Cr2::read());
    serial_println!("Error Code: {:?}", error_code);
    serial_println!("{:?}", stack_frame);

    halt_loop();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    timer::next_tick();

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Always take your locks together.
    let mut pics = PICS.lock();

    let mut port: Port<u8> = Port::new(0x60);

    let scancode = unsafe { port.read() };
    crate::tasks::keyboard::add_scancode(scancode);

    unsafe {
        pics.notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn primary_ata_handler(_stack_frame: InterruptStackFrame) {
    let mut pics = PICS.lock();
    serial_println!("primary ata handler");
    ata::interrupt( ata::BusKind::Primary); 
    
    unsafe {

        pics.notify_end_of_interrupt(InterruptIndex::PrimaryATA.as_u8());
    }
}

extern "x86-interrupt" fn secondary_ata_handler(_stack_frame: InterruptStackFrame) {
    let mut pics = PICS.lock();
    serial_println!("secondary ata handler");
    unsafe {
        pics.notify_end_of_interrupt(InterruptIndex::SecondaryATA.as_u8());
    }
}

// PICS

#[test_case]
fn test_breakpoint_exception() {
    // Invoke a breakpoint.
    x86_64::instructions::interrupts::int3();
}
