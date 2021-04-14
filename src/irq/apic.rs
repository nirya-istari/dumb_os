
use x86_64::VirtAddr;
use volatile::Volatile;


// src/apic.rs

#[repr(align(16))]
struct Reserved {
    _dont: Volatile<u32>,
    _padding: [u32; 3],
}

#[repr(align(16))]
pub struct ReadOnlyRegister {
    reg: Volatile<u32>,
    _padding: [u32; 3],
}

impl ReadOnlyRegister {
    /// IO registers can have effects when the read. So this is protected with mut
    fn read(&mut self) -> u32 { self.reg.read() }
}

#[repr(align(16))]
pub struct WriteOnlyRegister {
    reg: Volatile<u32>,
    _padding: [u32; 3],
}

impl WriteOnlyRegister {
    fn write(&mut self, value: u32) { self.reg.write(value) }
}

#[repr(align(16))]
pub struct ReadWriteRegister {
    reg: Volatile<u32>,
    _padding: [u32; 3],
}

impl ReadWriteRegister {
    fn read(&mut self) -> u32 { self.reg.read() }
    fn write(&mut self, value: u32) { self.reg.write(value) }
}

#[repr(C)]
pub struct MappedRegisters {
    /* 000 */ _reserved00:                              Reserved,
    /* 010 */ _reserved01:                              Reserved,
    /* 020 */ id:                                       ReadWriteRegister,
    /* 030 */ version:                                  ReadOnlyRegister,
    /* 040 */ _reserved02:                              Reserved,
    /* 050 */ _reserved03:                              Reserved,
    /* 060 */ _reserved04:                              Reserved,
    /* 070 */ _reserved05:                              Reserved,
    /* 080 */ task_priority:                            ReadWriteRegister,
    /* 090 */ arbitration_priority:                     ReadOnlyRegister,
    /* 0a0 */ processor_priority:                       ReadOnlyRegister,
    /* 0b0 */ end_of_interrupt:                         WriteOnlyRegister,
    /* 0c0 */ remote_read:                              ReadOnlyRegister,
    /* 0d0 */ logical_destination:                      ReadWriteRegister,
    /* 0e0 */ destination_format:                       ReadWriteRegister,
    /* 0f0 */ spurious_interrupt_vector:                ReadWriteRegister,
    /* 100 */ in_service:                              [ReadOnlyRegister; 8],
    /* 180 */ trigger_mode:                            [ReadOnlyRegister; 8],
    /* 200 */ interrupt_request:                       [ReadOnlyRegister; 8],
    /* 280 */ error_status:                             ReadOnlyRegister,
    /* 290 */ _reserved06:                              Reserved,
    /* 2a0 */ _reserved07:                              Reserved,
    /* 2b0 */ _reserved08:                              Reserved,
    /* 2c0 */ _reserved09:                              Reserved,
    /* 2d0 */ _reserved10:                              Reserved,
    /* 2e0 */ _reserved11:                              Reserved,
    /* 2f0 */ lvt_corrected_machine_check_interrupt:    ReadWriteRegister,
    /* 300 */ interrupt_command:                       [ReadWriteRegister; 2],
    /* 320 */ lvt_timer:                                ReadWriteRegister,
    /* 330 */ lvt_thermal_sensor:                       ReadWriteRegister,
    /* 340 */ lvt_performance_counter:                  ReadWriteRegister,
    /* 350 */ lvt_lint0:                                ReadWriteRegister,
    /* 360 */ lvt_lint1:                                ReadWriteRegister,
    /* 370 */ lvt_error:                                ReadWriteRegister,
    /* 380 */ timer_initial_count:                      ReadWriteRegister,
    /* 390 */ timer_current_count:                      ReadOnlyRegister,
    /* 3a0 */ _reserved12:                              Reserved,
    /* 3b0 */ _reserved13:                              Reserved,
    /* 3c0 */ _reserved14:                              Reserved,
    /* 3d0 */ _reserved15:                              Reserved,
    /* 3eo */ timer_divide_configuration:               ReadWriteRegister,
    /* 3f0 */ _reserved16:                              Reserved,
    _private: ()
}

#[repr(C)]
struct IoApicRegisters {
    select: Volatile<u32>,
    _padding: [u32; 3],
    data: Volatile<u32>,
}

impl IoApicRegisters {
    fn read(&mut self, select: u8) -> u32 {
        self.select.write(select as u32);
        self.data.read()
    }

    fn write(&mut self, register: u8, value: u32) {
        self.select.write(register as u32);
        self.data.write(value);
    }
}

struct Apic {
    // This is fine because we need a Mutable reference to Apic to perform mutations.
    registers: &'static mut MappedRegisters,
}

impl Apic {
    fn new(physical_memory_offset: VirtAddr) -> Apic {
        // TODO: There's a MSR that controls the offset but I don't see a reason
        // to contol that here.

        todo!()

    }

    fn enable(&mut self) {
        self.registers.spurious_interrupt_vector.write(
            0b_00000000_00000001_11111111_11111111
        );
    }
}

#[cfg(test)]
mod test {
    use field_offset::offset_of;
    use super::*;
    
#[test_case]
fn check_apic_register_alignment() {
    assert_eq!(0x000, offset_of!(MappedRegisters => _reserved00).get_byte_offset(), "_reserved00");
    assert_eq!(0x010, offset_of!(MappedRegisters => _reserved01).get_byte_offset(), "_reserved01");
    assert_eq!(0x020, offset_of!(MappedRegisters => id).get_byte_offset(), "id");
    assert_eq!(0x030, offset_of!(MappedRegisters => version).get_byte_offset(), "version");
    assert_eq!(0x040, offset_of!(MappedRegisters => _reserved02).get_byte_offset(), "_reserved02");
    assert_eq!(0x050, offset_of!(MappedRegisters => _reserved03).get_byte_offset(), "_reserved03");
    assert_eq!(0x060, offset_of!(MappedRegisters => _reserved04).get_byte_offset(), "_reserved04");
    assert_eq!(0x070, offset_of!(MappedRegisters => _reserved05).get_byte_offset(), "_reserved05");
    assert_eq!(0x080, offset_of!(MappedRegisters => task_priority).get_byte_offset(), "task_priority");
    assert_eq!(0x090, offset_of!(MappedRegisters => arbitration_priority).get_byte_offset(), "arbitration_priority");
    assert_eq!(0x0a0, offset_of!(MappedRegisters => processor_priority).get_byte_offset(), "processor_priority");
    assert_eq!(0x0b0, offset_of!(MappedRegisters => end_of_interrupt).get_byte_offset(), "end_of_interrupt");
    assert_eq!(0x0c0, offset_of!(MappedRegisters => remote_read).get_byte_offset(), "remote_read");
    assert_eq!(0x0d0, offset_of!(MappedRegisters => logical_destination).get_byte_offset(), "logical_destination");
    assert_eq!(0x0e0, offset_of!(MappedRegisters => destination_format).get_byte_offset(), "destination_format");
    assert_eq!(0x0f0, offset_of!(MappedRegisters => spurious_interrupt_vector).get_byte_offset(), "spurious_interrupt_vector");
    assert_eq!(0x100, offset_of!(MappedRegisters => in_service).get_byte_offset(), "in_service");
    assert_eq!(0x180, offset_of!(MappedRegisters => trigger_mode).get_byte_offset(), "trigger_mode");
    assert_eq!(0x200, offset_of!(MappedRegisters => interrupt_request).get_byte_offset(), "interrupt_request");
    assert_eq!(0x280, offset_of!(MappedRegisters => error_status).get_byte_offset(), "error_status");
    assert_eq!(0x290, offset_of!(MappedRegisters => _reserved06).get_byte_offset(), "_reserved06");
    assert_eq!(0x2a0, offset_of!(MappedRegisters => _reserved07).get_byte_offset(), "_reserved07");
    assert_eq!(0x2b0, offset_of!(MappedRegisters => _reserved08).get_byte_offset(), "_reserved08");
    assert_eq!(0x2c0, offset_of!(MappedRegisters => _reserved09).get_byte_offset(), "_reserved09");
    assert_eq!(0x2d0, offset_of!(MappedRegisters => _reserved10).get_byte_offset(), "_reserved10");
    assert_eq!(0x2e0, offset_of!(MappedRegisters => _reserved11).get_byte_offset(), "_reserved11");
    assert_eq!(0x2f0, offset_of!(MappedRegisters => lvt_corrected_machine_check_interrupt).get_byte_offset(), "lvt_corrected_machine_check_interrupt");
    assert_eq!(0x300, offset_of!(MappedRegisters => interrupt_command).get_byte_offset(), "interrupt_command");
    assert_eq!(0x320, offset_of!(MappedRegisters => lvt_timer).get_byte_offset(), "lvt_timer");
    assert_eq!(0x330, offset_of!(MappedRegisters => lvt_thermal_sensor).get_byte_offset(), "lvt_thermal_sensor");
    assert_eq!(0x340, offset_of!(MappedRegisters => lvt_performance_counter).get_byte_offset(), "lvt_performance_counter");
    assert_eq!(0x350, offset_of!(MappedRegisters => lvt_lint0).get_byte_offset(), "lvt_lint0");
    assert_eq!(0x360, offset_of!(MappedRegisters => lvt_lint1).get_byte_offset(), "lvt_lint1");
    assert_eq!(0x370, offset_of!(MappedRegisters => lvt_error).get_byte_offset(), "lvt_error");
    assert_eq!(0x380, offset_of!(MappedRegisters => timer_initial_count).get_byte_offset(), "timer_initial_count");
    assert_eq!(0x390, offset_of!(MappedRegisters => timer_current_count).get_byte_offset(), "timer_current_count");
    assert_eq!(0x3a0, offset_of!(MappedRegisters => _reserved12).get_byte_offset(), "_reserved12");
    assert_eq!(0x3b0, offset_of!(MappedRegisters => _reserved13).get_byte_offset(), "_reserved13");
    assert_eq!(0x3c0, offset_of!(MappedRegisters => _reserved14).get_byte_offset(), "_reserved14");
    assert_eq!(0x3d0, offset_of!(MappedRegisters => _reserved15).get_byte_offset(), "_reserved15");
    assert_eq!(0x3e0, offset_of!(MappedRegisters => timer_divide_configuration).get_byte_offset(), "timer_divide_configuration");
    assert_eq!(0x3f0, offset_of!(MappedRegisters => _reserved16).get_byte_offset(), "_reserved16");
}


#[test_case]
fn check_io_apic_register_alignment() {
    assert_eq!(0, offset_of!(IoApicRegisters => select).get_byte_offset(), "select");
    assert_eq!(0x10, offset_of!(IoApicRegisters => data).get_byte_offset(), "data");
}



}