// panic.rs

#[cfg(not(test))]
use crate::println;
#[cfg(test)]
use crate::serial_println;
    

use crate::qemu;
use core::panic::PanicInfo;


#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[failed]");
    serial_println!("Paniced: {}", info);

    qemu::exit_qemu(qemu::ExitStatusCode::Failed);
    loop {}
}
