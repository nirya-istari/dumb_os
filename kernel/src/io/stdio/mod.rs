use core::fmt::{self};

use alloc::prelude::v1::*;

use conquer_once::spin::{OnceCell};
use interrupts::without_interrupts;
use smallvec::SmallVec;
use spin::{Mutex, MutexGuard};
use x86_64::instructions::interrupts;

use crate::uart::SerialPort;
use crate::io::{self, Write};

use super::{BufRead, Read};

static STDIO: OnceCell<Mutex<Stdio>> = OnceCell::uninit();

pub fn stdio_init() {
    STDIO.init_once(|| {
        use core::fmt::Write;
        let mut serial = unsafe { SerialPort::new(0x3f8) };
        serial.init();
        write!(serial, "uart initialized\n").expect("Write failed");
        Mutex::new(Stdio {
            serial: serial,
            read_buffer_pos: 0,            
            read_buffer: SmallVec::new()
        })
    })    
}

fn stdio() -> &'static Mutex<Stdio> {
    STDIO.get().unwrap_or_else(|| panic!("STDIO has not been inialized"))
}

pub fn stdin() -> Stdin {
    Stdin { _private: () }
}

pub fn stdout() -> Stdout {
    Stdout { _private: () }
}

/// Actual std io handler.
struct Stdio {
    serial: SerialPort,
    read_buffer_pos: usize,
    read_buffer: SmallVec<[u8; 1024]>
}

impl fmt::Debug for Stdio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stdio")
            .field("serial", &"SerialPort")
            .finish()
    }
}


impl Write for Stdio {
    fn write(&mut self, data: &[u8]) -> super::Result<usize> {
        for b in data {            
            self.serial.send(*b);
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> super::Result<()> {
        Ok(())
    }
}

impl Read for Stdio {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() { return Ok(0); }
        let mut blocked = true;
        
        // Read at least 1 byte
        for (i, b) in buf.iter_mut().enumerate() {            
            loop {
                if let Some(v) = self.serial.try_receive() {
                    *b = v;
                    blocked = false;                    
                } else if !blocked {
                    // We've read something.
                    return Ok(i)
                }
            }
        }
        Ok(buf.len())
    }
}

impl BufRead for Stdio {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.read_buffer.len() == self.read_buffer.capacity() {
            return Ok(&self.read_buffer[self.read_buffer_pos..])
        }

        // Due to the semantics of Read we can't return an empty buffer.
        // so we poll until we get at least 1 byte. Then return.
        while self.read_buffer.len() < self.read_buffer.capacity()         
            && (self.read_buffer.is_empty() || self.serial.data_avaliable())
        {
            if let Some(b) = self.serial.try_receive() {
                self.read_buffer.push(b);
            }            
        }
        
        Ok(&self.read_buffer[self.read_buffer_pos..])
    }

    fn consume(&mut self, amt: usize) {
        self.read_buffer_pos = self.read_buffer_pos.saturating_add(amt);
        if self.read_buffer_pos >= self.read_buffer.len() {
            self.read_buffer.clear();
            self.read_buffer_pos = 0;
        }
    }
}

#[derive(Debug)]
pub struct Stdout {
    _private: ()
}

#[derive(Debug)]
pub struct StdoutLock<'a> {
    mutex: MutexGuard<'a, Stdio>
}

impl Stdout {
    pub fn lock(&self) -> StdoutLock<'_> {
        StdoutLock {
            mutex: stdio().lock()
        }
    }

    pub unsafe fn break_lock(&self) -> StdoutLock<'_> {        
        stdio().force_unlock();
        self.lock()
    }
}

impl Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.lock().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.lock().flush()
    }

    fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        self.lock().write_all(data)
    }

    fn write_fmt(&mut self, fmt: core::fmt::Arguments<'_>) -> io::Result<()> {
        self.lock().write_fmt(fmt)
    }
}

impl Write for StdoutLock<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.mutex.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.mutex.flush()
    }

    fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        self.mutex.write_all(data)
    }

    fn write_fmt(&mut self, fmt: core::fmt::Arguments<'_>) -> io::Result<()> {
        self.mutex.write_fmt(fmt)
    }
}

#[derive(Debug)]
pub struct Stdin {
    _private: ()
}

#[derive(Debug)]
pub struct StdinLock<'a> {
    mutex: MutexGuard<'a, Stdio>
}
impl Stdin {
    pub fn lock(&self) -> StdinLock<'_> {
        StdinLock {
            mutex: stdio().lock()
        }
    }

    pub fn read_line(&self, buf: &mut String) -> io::Result<usize> {
        stdio().lock().read_line(buf)
    }
}


#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    without_interrupts(|| {
        let mut lock = stdio().lock();    
        fmt::Write::write_fmt(&mut lock.serial, args)        
    }).expect("writing to stdout failed");
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::io::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => ( $crate::io::_print(format_args!("\n")) );
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(
        concat!($fmt, "\n"), $($arg)*));   
}

#[test_case]
fn test_println() {
    println!("Hello, world");
}

#[test_case]
fn test_println_many() {
    for i in 0..=200 {
        println!("line: {}", i);
    }
}
