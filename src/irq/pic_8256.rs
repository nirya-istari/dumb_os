// src/irq/pic_8256.rs
use pic8259_simple::ChainedPics;
use x86_64::instructions::{interrupts, port::Port};

pub const PIC1_OFFSET: u8 = 32;
pub const PIC2_OFFSET: u8 = PIC1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC1_OFFSET, PIC2_OFFSET) });

pub unsafe fn disable() {
    // Well uhhh probably don't want interrups while PIC's are being turned off.
    interrupts::disable();
    
    let mut port2: Port<u8> = Port::new(0xa1);
    let mut port1: Port<u8> = Port::new(0x21);
    
    port2.write(0xff);
    port1.write(0xff);    
    
}
