// main.rs

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

// defines println! must be first.
mod vga_buffer;
mod serial;
mod panic;
mod qemu;

// static HELLO: &[u8] = b"Hello World!";

#[no_mangle]
extern "C" fn _start() -> ! {
    println!("Hello, world{}", '!');

    #[cfg(test)]
    test_main();

    halt();
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


#[test_case]
fn trivial_test() {
    assert_eq!(2 + 2, 4);
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }

    qemu::exit_qemu(qemu::ExitStatusCode::Success);
}

fn halt() -> ! {
    use x86_64::instructions::hlt;
    loop {
        hlt();
    }
}
