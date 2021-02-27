// qemu.rs

#[derive(Copy, Clone)]
pub enum ExitStatusCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: ExitStatusCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);        
    }
}
