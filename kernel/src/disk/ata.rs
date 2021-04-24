

use core::{alloc::Layout, fmt, iter::{self, once}, sync::atomic::{Ordering, AtomicBool}, task::{Context, Poll}};

use alloc::{collections::TryReserveError, prelude::v1::*};
use bitflags::bitflags;
use futures::{future::poll_fn, task::AtomicWaker};
use smallvec::SmallVec;
use ux::{u28, u4};
use x86_64::{instructions::{interrupts::without_interrupts, port::{Port, PortReadOnly, PortWriteOnly}}, structures::port::{PortRead, PortWrite}};

use crate::{prelude::*, tasks::timer::current_tick};
use crate::irq::InterruptIndex;

const SECTOR_SIZE: usize = 512;

#[derive(Debug)]
struct Bus {
    kind: BusKind,
    io_port_base: u16,
    dc_port_base: u16,
    selected: DriveSelector,    
    interrupt: InterruptIndex,
    drives: SmallVec<[Drive; 2]>
}

#[derive(Debug)]
struct Drive {
    selector: DriveSelector,
    kind: DriveKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DriveKind {
    PataPi,
    SataPi,
    Pata,
    Sata,    
}

impl DriveKind {
    fn identify(cylinder: u16) -> Option<Self> {
        match cylinder {
            0xEB14 => Some(Self::PataPi),
            0x9669 => Some(Self::SataPi),
            0x0000 => Some(Self::Pata),
            0xc33c => Some(Self::Sata),
            // Unkown
            _ => None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BusKind {
    Primary,
    Secondary, // There are possibly 4 busses. But we don't have interrups for them.
}

impl fmt::Display for BusKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
        })
    }
}

impl BusKind {

    fn waker<'a>(&'a self) -> &'static AtomicWaker {
        match self {
            BusKind::Primary => &PRIMARY_WAKER,
            BusKind::Secondary => &SECONDARY_WAKER,
        }
    }

    fn interrupt_flag<'a>(&'a self) -> &'static AtomicBool {
        match self {
            BusKind::Primary => &PRIMARY_INTERRUPT_FLAG,
            BusKind::Secondary => &SECONDARY_INTERRUPT_FLAG,
        }
    }
    
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DriveSelector {
    /// First drive on the bus. Often called Master
    First,
    /// Second drive on the bus. Often called Slave.
    Second,
}

impl DriveSelector {
    fn iter() -> impl Iterator<Item = DriveSelector> {
        once(DriveSelector::First).chain(once(DriveSelector::Second))
    }
}

// Use LBA 28-bit. Maximum of 128GiB. If we want more go to SATA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct LbaAddr(u28);

impl LbaAddr {
    fn zero() -> Self {
        Self(u28::default())
    }

    fn split(&self) -> (u8, u8, u8, u4) {
        let int = u32::from(self.0);
        let [a, b, c, d] = int.to_le_bytes();
        (a, b, c, u4::new(d))
    }
}

pub static PRIMARY_WAKER: AtomicWaker = AtomicWaker::new();
pub static PRIMARY_INTERRUPT_FLAG: AtomicBool = AtomicBool::new(false);
pub static SECONDARY_WAKER: AtomicWaker = AtomicWaker::new();
pub static SECONDARY_INTERRUPT_FLAG: AtomicBool = AtomicBool::new(false);

pub fn interrupt(bus: BusKind) {
    match bus {
        BusKind::Primary => { 
            PRIMARY_WAKER.wake();
            PRIMARY_INTERRUPT_FLAG.store(true, Ordering::SeqCst);
        }
        BusKind::Secondary => {
            SECONDARY_WAKER.wake();
            SECONDARY_INTERRUPT_FLAG.store(true, Ordering::SeqCst);
        }
    }
}

fn standard_busses() -> impl Iterator<Item = Bus> {
    // Dumbest way to create a iterator without cloning.
    iter::once(
    Bus {
        kind: BusKind::Primary,
        io_port_base: 0x1F0,
        dc_port_base: 0x3F6,
        selected: DriveSelector::First,        
        interrupt: InterruptIndex::PrimaryATA,
        drives: SmallVec::new(),
    }).chain(iter::once(
    Bus {
        kind: BusKind::Secondary,
        io_port_base: 0x170,
        dc_port_base: 0x376,
        selected: DriveSelector::First,        
        interrupt: InterruptIndex::SecondaryATA,
        drives: SmallVec::new(),
    }))
}

pub async fn ata_main() {   
    let mut buses: Vec<Bus> = standard_busses()
        .filter(|p| !p.floating())        
        .collect::<Vec<Bus>>();        

    let mut selected = None;

    for bus in buses.iter_mut() {
        
        bus.identify_drives();
        for drive in &bus.drives {
            if selected.is_none() {
                selected = Some((bus.kind, drive.selector));
            }
        }

        let error = unsafe { bus.error_register().read() };
        let status = unsafe { bus.status().read() };        
        let drive_address = unsafe { bus.drive_address().read() };
        let drive_head_register = unsafe { bus.drive_head_register().read() };
        
        println!("ATA: {:?}:\n  error: {:?}\n  status: {:?}\n  drive_address: {:?}\n  drive/head register: {:?}", 
            bus.kind,
            error,
            status,
            drive_address,
            drive_head_register
        );
    }
    println!("Busses: {:#?}", buses);
    let (kind, drive) = selected.expect("not drives avaliable");
    println!("Using {:?}, {:?}", kind, drive);
    let bus = buses.iter_mut().find(|b| b.kind == kind)
        .expect("WAT");

    match bus.read_sector(drive, LbaAddr::zero()).await {
        Ok(vec) => dump_hex(&vec),
        Err(err) => println!("Error reading: {}", err)
    };
}

fn dump_hex<B>(data: &B) 
    where B: AsRef<[u8]>
{
    let data = data.as_ref();
    let chunk_size = 16;

    for (r, row) in data.chunks(chunk_size).enumerate() {
        print!("{:08x}: ", r * chunk_size);
        for word in row.chunks(2) {
            print!("{:02x}{:02x} ", word[0], word[1]);
        }
        println!("  ");
        for byte in row {
            let ch = if !byte.is_ascii() || byte.is_ascii_control() {
                '.'
            } else {
                *byte as char
            };
            print!("{}", ch);
        }
        println!();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum AtaCommand  {
    /// READ SECTORS
    ReadSectors = 0x20,
}

impl PortWrite for AtaCommand {
    unsafe fn write_to_port(port: u16, value: Self) {
        PortWrite::write_to_port(port, value as u8)
    }
}

impl Bus {
    fn poll_interrupt(&mut self, cx: &mut Context) -> Poll<()> {
        let waker = self.kind.waker();
        let interrupt_flag = self.kind.interrupt_flag();

        without_interrupts(|| {
            if interrupt_flag.swap(false, Ordering::SeqCst) == true {
                Poll::Ready(())
            } else {
                waker.register(&cx.waker());
                Poll::Pending             
            }
        })
    }

    async fn wait_for_interrupt(&mut self) {
        poll_fn(|cx| self.poll_interrupt(cx)).await
    }

    async fn wait_for_drq(&mut self) -> Result<(), ErrorRegister> {
        self.wait_for_interrupt().await;
        let status = unsafe { self.status().read() };
        if status.contains(StatusRegister::ERR) {
            self.check_error()?;
        }
        Ok(())
    }

    /// Primary: 0x1F0
    /// Secondary: 0x170
    fn data_register(&self) -> Port<u16> { Port::new(self.io_port_base) }
    /// Primary: 0x1F1
    /// Secondary: 0x171
    fn error_register(&self) -> PortReadOnly<ErrorRegister> { PortReadOnly::new(self.io_port_base + 1) }
    /// Primary: 0x1F1
    /// Secondary: 0x171
    fn features_register(&self) -> PortWriteOnly<u8> { PortWriteOnly::new(self.io_port_base + 1) }
    /// Primary: 0x1F2
    /// Secondary: 0x172
    fn sector_count_register(&self) -> Port<SectorCountRegister> { Port::new(self.io_port_base + 2) }
    /// Primary: 0x1F3
    /// Secondary: 0x173
    fn sector_number_register(&self) -> Port<u8> { Port::new(self.io_port_base + 3) }
    /// Primary: 0x1F4
    /// Secondary: 0x174
    fn cylinder_low_register(&self) -> Port<u8> { Port::new(self.io_port_base + 4) }
    /// Primary: 0x1F5
    /// Secondary: 0x175
    fn cylinder_hi_register(&self) -> Port<u8> { Port::new(self.io_port_base + 5) }

    /// Read cylinder_low to cylinder_hi and convert to u16
    fn read_cylinder(&self) -> u16 {
        unsafe {
            u16::from_le_bytes([
                self.cylinder_low_register().read(),
                self.cylinder_hi_register().read(),
            ])
        }
    }

    /// Primary: 0x1F6
    /// Secondary: 0x176
    fn drive_head_register(&self) -> Port<DriveHeadRegister> { Port::new(self.io_port_base + 6) }
    /// Primary: 0x1F7
    /// Secondary: 0x177
    fn status(&self) -> PortReadOnly<StatusRegister> { PortReadOnly::new(self.io_port_base + 7) }
    /// Primary: 0x1F7
    /// Secondary: 0x177
    fn command(&self) -> PortWriteOnly<AtaCommand> { PortWriteOnly::new(self.io_port_base + 7) }
    /// Primary: 0x3F6
    /// Secondary: 0x376
    fn alt_status(&self) -> PortReadOnly<StatusRegister> { PortReadOnly::new(self.dc_port_base + 0) }
    /// Primary: 0x3F6
    /// Secondary: 0x376
    fn device_control_register(&self) -> PortWriteOnly<DeviceControlRegister> { PortWriteOnly::new(self.dc_port_base + 0) }
    /// Primary: 0x3F7
    /// Secondary: 0x377
    fn drive_address(&self) -> PortReadOnly<DriveAddressRegister> { PortReadOnly::new(self.dc_port_base + 1) }

    fn floating(&self) -> bool {
        unsafe { self.status().read() == StatusRegister::FLOATING }
    }

    fn wait_for_bsy(&mut self) -> Result<(), ()> {
        let start_ticks = current_tick();
        let mut port = self.alt_status();
        loop {
            let flag = unsafe { port.read() };
            
            if flag.contains(StatusRegister::RDY) && !flag.contains(StatusRegister::BSY) {
                return Ok(());
            } else if current_tick() - start_ticks > 1 {
                return Err(())
            }
        }
    }

    fn soft_reset(&mut self) {
        unsafe { 
            println!("soft resetting bus {}", self.kind);
            self.device_control_register().write(DeviceControlRegister::SRST);
            crate::delay(5);
            self.device_control_register().write(DeviceControlRegister::empty());
            self.wait_for_bsy().ok();
        }
    }
    
    fn identify_drives(&mut self) {
        self.soft_reset();
        for selector in DriveSelector::iter() {

            unsafe {
                self.drive_head_register().write(DriveHeadRegister::new(LbaAddr::zero(), selector));
                for _ in 0..4 {
                    // Wait 400 ns
                    self.alt_status().read();
                }
                let cylinder = self.read_cylinder();
                if let Some(kind) = DriveKind::identify(cylinder) {
                    self.drives.push(Drive {
                        selector,
                        kind,
                    });
                }
            }
        }
    }


    async fn read_sector(&mut self, drive: DriveSelector, position: LbaAddr) -> Result<Vec<u8>, ReadError> {
        let sector_count = 1;
        let bytes = sector_count as usize * SECTOR_SIZE;
        let mut sectors: Vec<u8> = Vec::new();
        sectors.try_reserve_exact(bytes)?;
        println!("allocated: {}", sectors.len());

        unsafe {
            sectors.set_len(sectors.capacity());
        }
        
        let status = unsafe { self.alt_status().read() };
        if status.contains(StatusRegister::BSY) || status.contains(StatusRegister::DRQ) {
            return Err(ReadError::DiskBusy);
        }
        println!("status: {:?}", status);


        unsafe {
            self.drive_head_register().write(
                DriveHeadRegister::new(position, drive)
            );
            // Induce delay
            self.features_register().write(0);
        }        
        
        let status = unsafe { self.alt_status().read() };        
        println!("after drive select. status: {:?}", status);

        unsafe {
            
            let (sector_number, cylinder_low, cylinder_hi, _,) = position.split();

            self.sector_count_register().write(SectorCountRegister(sector_count));
            let status =  { self.alt_status().read() };        
            println!("after sector_count_register. status: {:?}", status);

            self.sector_number_register().write(sector_number);
            let status =  { self.alt_status().read() };        
            println!("after sector_number_register. status: {:?}", status);
            
            self.cylinder_low_register().write(cylinder_low);
            let status =  { self.alt_status().read() };        
            println!("after cylinder_low_register. status: {:?}", status);
            
            self.cylinder_hi_register().write(cylinder_hi);
            let status =  { self.alt_status().read() };        
            println!("after cylinder_hi_register. status: {:?}", status);
            

            self.command().write(AtaCommand::ReadSectors);
            println!("sent READ SECTORS command");
            let status = { self.alt_status().read() };        
            println!("after command . status: {:?}", status);
            

            self.check_error()?;

            self.wait_for_drq().await?;

            read_sector(self.data_register(), &mut sectors[..]);

        }


        Err(ReadError::_NotImplemented)
    }

    fn check_error(&mut self) -> Result<(), ErrorRegister> {
        
        let error_code = unsafe { self.error_register().read() };
        if error_code.is_empty() {
            Ok(())
        } else {
            Err(error_code)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
    DiskBusy,
    ReqReadTooBig,
    OutOfMemory(Layout),
    AtaError(ErrorRegister),
    // Instead of using todo!()
    _NotImplemented,
}
impl From<TryReserveError> for ReadError {
    fn from(error: TryReserveError) -> Self {
        match error {
            TryReserveError::CapacityOverflow => ReadError::ReqReadTooBig,
            TryReserveError::AllocError { layout, .. } => ReadError::OutOfMemory(layout),
        }
    }
}

impl From<ErrorRegister> for ReadError {
    fn from(error: ErrorRegister) -> Self {
        Self::AtaError(error)
    }
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {        
        match self {
            ReadError::DiskBusy => write!(f, "disk is busy"),
            ReadError::ReqReadTooBig => write!(f, "requested read was too big"),
            ReadError::OutOfMemory(reqiured) => write!(f, "not enough memory for read (required {}B)", reqiured.size()),
            ReadError::AtaError(error) => write!(f, "ata error {:?}", error),
            ReadError::_NotImplemented => write!(f, "not implemented")
        }
    }
}

/// Read 512 bytes for Data port.
unsafe fn read_sector(port: Port<u16>, buffer: &mut [u8]) {
    assert!(buffer.len() >= SECTOR_SIZE);
    // Port has not getting for it's only field.
    let port_no: &u16 = core::mem::transmute(&port);
    let ptr = buffer.as_mut_ptr();
    asm! {
        "rep insw",
        in("dx")  port_no,
        in("rdi") ptr,
        in("rax") SECTOR_SIZE / 2,        
    };
}

bitflags! {
    pub struct ErrorRegister: u8 {
        /// Address mark not found.
        const AMNF = 1 << 0;
        /// Track zero not found.
        const TKZNF = 1 << 1;
        /// Aborted command.
        const ABRT = 1 << 2;
        /// Media change request.
        const MCR = 1 << 3;
        /// ID not found.
        const IDNF = 1 << 4;
        /// Media changed.
        const MC = 1 << 5;
        /// Uncorrectable data error.
        const UNC = 1 << 6;
        /// Bad Block detected. 
        const BBK = 1 << 7;
    }
}
impl PortRead for ErrorRegister {
    unsafe fn read_from_port(port: u16) -> Self {        
        Self::from_bits_unchecked(PortRead::read_from_port(port))
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SectorCountRegister(u8);

impl PortRead for SectorCountRegister {
    unsafe fn read_from_port(port: u16) -> Self {
        SectorCountRegister(PortRead::read_from_port(port))
    }
}

impl PortWrite for SectorCountRegister {
    unsafe fn write_to_port(port: u16, value: Self) {
        PortWrite::write_to_port(port, value.0)
    }
}

impl DriveHeadRegister {
    fn new(addr: LbaAddr, drive: DriveSelector) -> Self {        
        let mut bits = DriveHeadRegister::from_bits_truncate(
            DriveHeadRegister::LBA_ADDR.bits | 
            u8::from(addr.split().3)
        );
        if drive == DriveSelector::Second {
            bits |= DriveHeadRegister::DRV;
        }
        // 0xE0: SET_x are requred, and we're not using CHS so LBA is always set.        
        bits |= DriveHeadRegister::LBA | DriveHeadRegister::SET_1 | DriveHeadRegister::SET_2;
        bits
    }
}

bitflags!  {
    struct DriveHeadRegister: u8 {
        // 0x00
        const LBA24 = 1 << 0;
        // 0x01
        const LBA25 = 1 << 1;
        // 0x02
        const LBA26 = 1 << 2;
        // 0x04
        const LBA27 = 1 << 3;        
        const LBA_ADDR = (Self::LBA24.bits | Self::LBA25.bits | Self::LBA26.bits | Self::LBA27.bits);
        // 0x08
        const DRV = 1 << 4;
        // 0x20
        const SET_1 = 1 << 5;
        // 0x40
        const LBA = 1 << 6;
        // 0x80
        const SET_2 = 1 << 7;
    }
}

impl PortWrite for DriveHeadRegister {
    unsafe fn write_to_port(port: u16, value: Self) {        
        PortWrite::write_to_port(port, value.bits);
    }
}

impl PortRead for DriveHeadRegister {
    unsafe fn read_from_port(port: u16) -> Self {
        DriveHeadRegister::from_bits_unchecked(PortRead::read_from_port(port))
    }
}

bitflags! {
    struct StatusRegister: u8 {
        const ERR = 1 << 0;
        const IDX = 1 << 1;
        const CORR = 1 << 2;
        const DRQ = 1 << 3;
        const SRV = 1 << 4;
        const DF = 1 << 5;
        const RDY = 1 << 6;
        const BSY = 1 << 7;
        const FLOATING = 0xFF;
    }
}

impl PortRead for StatusRegister {
    unsafe fn read_from_port(port: u16) -> Self {        
        Self::from_bits_unchecked(PortRead::read_from_port(port))
    }
}

bitflags! {
    struct DeviceControlRegister: u8 {
        const NIEN = 1 << 1;
        const SRST = 1 << 2;
        const HOB  = 1 << 7;
    }
}

impl PortWrite for DeviceControlRegister {
    unsafe fn write_to_port(port: u16, value: Self) {
        PortWrite::write_to_port(port, value.bits)
    }
}

bitflags! {
    struct DriveAddressRegister: u8 {
        const DS0 = 1 << 0;
        const DS1 = 1 << 1;
        const HS0 = 1 << 2;
        const HS1 = 1 << 3;
        const HS2 = 1 << 4;
        const HS3 = 1 << 5;
        const WTG = 1 << 6;        
        // const _RES = 1 << 7;
    }
}

impl PortRead for DriveAddressRegister {
    unsafe fn read_from_port(port: u16) -> Self {        
        Self::from_bits_truncate(PortRead::read_from_port(port))
    }
}
