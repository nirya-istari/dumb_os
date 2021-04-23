// src/vga_buffer.rs

use core::{
    fmt::{self, Write},
    mem::swap,
};
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;
use x86_64::instructions::interrupts;

use crate::tasks::timer::current_tick;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    const fn new(forground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (forground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct BufferChar {
    ascii_char: u8,
    color_code: ColorCode,
}

// Status bar on actual first line.
const FIRST_LINE: usize = 1;
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<BufferChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    row_position: usize,
    color_code: ColorCode,
    ticks: u64,
    buffer: &'static mut Buffer,
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(unsafe { Writer::init() });
}

impl Writer {
    unsafe fn init() -> Writer {
        let mut writer = Writer {
            column_position: 0,
            row_position: BUFFER_HEIGHT - 1,
            color_code: ColorCode::new(Color::Yellow, Color::Black),
            ticks: 0,
            buffer: { &mut *(0xb8000 as *mut Buffer) },
        };

        for row in 0..FIRST_LINE {
            for cell in writer.buffer.chars[row].iter_mut() {
                cell.write(BufferChar {
                    ascii_char: b' ',
                    color_code: ColorCode::new(Color::Black, Color::Yellow),
                });
            }
        }

        writer.update_status_line();

        writer
    }

    unsafe fn update_status_line(&mut self) {
        let mut orig_row_positon = 0;
        let mut orig_column_position = 0;
        let mut orig_color_code = ColorCode::new(Color::Black, Color::Yellow);
        swap(&mut orig_row_positon, &mut self.row_position);
        swap(&mut orig_column_position, &mut self.column_position);
        swap(&mut orig_color_code, &mut self.color_code);

        // TODO: make this not go over line count.
        let ticks = self.ticks;
        let cycles: u64 = core::arch::x86_64::_rdtsc();

        write!(self, "tick: {}, cycles: {}", ticks, cycles).ok();

        swap(&mut orig_row_positon, &mut self.row_position);
        swap(&mut orig_column_position, &mut self.column_position);
        swap(&mut orig_color_code, &mut self.color_code);
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = self.row_position;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(BufferChar {
                    ascii_char: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    fn new_line(&mut self) {
        for from in (FIRST_LINE + 1)..BUFFER_HEIGHT {
            let to = from - 1;
            for col in 0..BUFFER_WIDTH {
                let ch = self.buffer.chars[from][col].read();
                self.buffer.chars[to][col].write(ch);
            }
        }
        self.clear_line(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_line(&mut self, row: usize) {
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(BufferChar {
                ascii_char: b' ',
                color_code: self.color_code,
            });
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    interrupts::without_interrupts(|| {
        let mut guard = WRITER.lock();
        guard.write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
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

#[test_case]
fn test_println_output() {
    interrupts::without_interrupts(|| {
        let s = "Some test string that fits on a single line";
        println!("{}", s);
        for (i, c) in s.chars().enumerate() {
            let screen_char = WRITER.lock().buffer.chars[BUFFER_HEIGHT - 2][i].read();
            assert_eq!(char::from(screen_char.ascii_char), c);
        }
    })
}

pub fn update_ticks() {
    let ticks = current_tick();
    let mut guard = WRITER.lock();        
    guard.ticks = ticks;
    unsafe {
        guard.update_status_line();
    }
}
