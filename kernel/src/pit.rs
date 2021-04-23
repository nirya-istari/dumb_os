use bitflags::bitflags;
use spin::Mutex;
use x86_64::instructions::{interrupts, port::{Port, PortWriteOnly}};

struct Pit {
    channel_0: Port<u8>,
    _channel_1: Port<u8>,
    _channel_2: Port<u8>,
    command: PortWriteOnly<u8>
}

static PIT: Mutex<Pit> = Mutex::new(
    Pit {
        channel_0: Port::new(0x40),
        _channel_1: Port::new(0x41),
        _channel_2: Port::new(0x42),
        command: PortWriteOnly::new(0x43),
    }
);

bitflags! {
    struct CommandFlags: u8 {
        // Channel select
        const CHANNEL_0 = 0b00_00_000_0;
        const CHANNEL_1 = 0b01_00_000_0;
        const CHANNEL_2 = 0b10_00_000_0;
        const READ_BACK = 0b11_00_000_0;
        // Access mode
        const ACCESS_LATCH_COUNT_VALUE_COMMAND = 0b00_00_000_0;
        const ACCESS_MODE_LOBYTE_ONLY = 0b00_01_000_0;
        const ACCESS_MODE_HIBYTE_ONLY = 0b00_10_000_0;
        const ACCESS_MODE_BOTH = 0b00_11_000_0;
        // Operating mode
        // interrupt on terminal count
        const OPERATING_MODE_0 = 0b00_00_000_0;
        // hardware re-triggerable one-shot
        const OPERATING_MODE_1 = 0b00_00_001_0;
        // rate generator
        const OPERATING_MODE_2 = 0b00_00_010_0;
        // square wave generator
        const OPERATING_MODE_3 = 0b00_00_011_0;
        // software triggered strobe
        const OPERATING_MODE_4 = 0b00_00_100_0;
        // hardware triggered strobe
        const OPERATING_MODE_5 = 0b00_00_101_0;
        // rate generator, same as 010b
        const OPERATING_MODE_2_ALIAS = 0b00_00_110_0;
        // square wave generator, same as 011b
        const OPERATING_MODE_3_ALIAS = 0b00_00_111_0;

        const BINARY_MODE = 0b00_00_000_0;
        const BCD_MODE = 0b00_00_000_1;
    }
}

pub fn current_count() -> u16 {
    let mut pit = PIT.lock();
    interrupts::without_interrupts(|| {
        
        let count = unsafe {
            pit.command.write( CommandFlags::CHANNEL_0.bits() );

            let count_lo = pit.channel_0.read();
            let count_hi = pit.channel_0.read();
            u16::from_le_bytes([count_lo, count_hi])
        };

        count 
    })
}

fn set_reload_value(value: u16) {
    let mut pit = PIT.lock();
    interrupts::without_interrupts(|| {
        let [lo, hi] = value.to_le_bytes();
        unsafe {
            pit.channel_0.write(lo);
            pit.channel_0.write(hi);
        }
    })
}
