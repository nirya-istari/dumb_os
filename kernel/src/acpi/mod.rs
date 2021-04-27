use alloc::{prelude::v1::*, sync::Arc};
use core::{fmt, mem, ptr::NonNull, result::Result::{Err, Ok}};
use spin::lock_api::Mutex;

use acpi::{sdt::Signature, AcpiHandler, AcpiTables, PciConfigRegions, PhysicalMapping, Sdt};
use alloc::collections::BTreeMap;
use bootloader::{boot_info::Optional, BootInfo};
use volatile::Volatile;
use x86_64::{PhysAddr, VirtAddr, structures::paging::{Mapper, Page, PageSize, PageTableFlags, PhysFrame, Size4KiB, mapper::MapToError}};

use crate::{memory_manager::MemoryManager, prelude::*};

const ACPI_OFFSET: u64 = 0x500000000000;

/// maps region with NO_CACHE set.
#[derive(Debug, Clone)]
struct MapAcpiAddr {
    physical_offset: *const u8,
    memory_manager: Arc<Mutex<MemoryManager>>,
}

impl AcpiHandler for MapAcpiAddr {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        let mut lock = self.memory_manager.lock();
        let MemoryManager {
            ref mut mapper,
            ref mut frame_allocator,
        } = *lock;
        let start_addr = PhysAddr::new(physical_address as u64);
        let last_addr = PhysAddr::new((physical_address + size - 1) as u64);

        let first_frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(start_addr);
        let last_frame = PhysFrame::containing_address(last_addr);

        let range = PhysFrame::range_inclusive(first_frame, last_frame);

        // println!("Mappin {:?}", range);
        for frame in range {
            let addr = VirtAddr::from_ptr(self.physical_offset) + frame.start_address().as_u64();
            let page = Page::containing_address(addr);

            // println!("ACPI: mapping {:x} to {:x}", frame.start_address(), page.start_address());
            let map_result = mapper
                .map_to_with_table_flags(
                    page,
                    frame,
                    PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        | PageTableFlags::NO_CACHE
                        | PageTableFlags::NO_EXECUTE,
                    PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        | PageTableFlags::NO_CACHE
                        | PageTableFlags::NO_EXECUTE,
                    frame_allocator,
                );
                match map_result {
                    Ok(flush) => { flush.flush(); }
                    Err(MapToError::PageAlreadyMapped(_)) => {
                        // println!("{:x} already mapped", frame.start_address())
                    }
                    Err(err) => {
                        panic!("Error mapping: {:?}", err);
                    }
                }
                
        }

        PhysicalMapping {
            physical_start: physical_address,
            virtual_start: NonNull::new(
                self.physical_offset.offset(physical_address as isize) as *mut T
            )
            .expect("physical_address mapped to 0x0"),
            region_length: size,
            mapped_length: range.count() * Size4KiB::SIZE as usize,
            handler: self.clone(),
        }
    }

    fn unmap_physical_region<T>(&self, _: &acpi::PhysicalMapping<Self, T>) {
        // All memory is mapped so we don't need to unmap it.
    }
}

struct DebugAcpiTables<'a>(&'a AcpiTables<MapAcpiAddr>);
impl fmt::Debug for DebugAcpiTables<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tables = self.0;
        f.debug_struct("AcpiTables")
            .field("revision", &tables.revision)
            .field("sdts", &DebugSdtMap(&tables.sdts))
            .field("dsdt", &tables.dsdt)
            .field("ssdts", &tables.ssdts)
            .finish()
    }
}

struct DebugSdtMap<'a>(&'a BTreeMap<Signature, Sdt>);
impl fmt::Debug for DebugSdtMap<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_map();
        for (signature, v) in self.0.iter() {
            d.entry(signature, &DebugSdt(v));
        }
        d.finish()
    }
}

struct DebugSdt<'a>(&'a Sdt);
impl fmt::Debug for DebugSdt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sdt")
            .field(
                "physical_addr",
                &format_args!("{:x}", self.0.physical_address),
            )
            .field("length", &self.0.length)
            .field("validated", &self.0.validated)
            .finish()
    }
}

pub fn init(
    bootinfo: &mut BootInfo,
    memory_manager: Arc<Mutex<MemoryManager>>,
) -> Result<Acpi, AcpiInitError> {
    if let Some(rdsp_addr) = mem::replace(&mut bootinfo.rsdp_addr, Optional::None).into_option() {
        let acpi_mapper = MapAcpiAddr {
            physical_offset: ACPI_OFFSET as *const u8,
            memory_manager: memory_manager.clone(),
        };

        let tables = unsafe {
            AcpiTables::from_rsdp(acpi_mapper.clone(), rdsp_addr as usize)
            .expect("failed to get ACPI tables")
        };
    

        println!("{:#?}", DebugAcpiTables(&tables));

        let pci_regions = PciConfigRegions::new(&tables).expect("Failed to get PCI regions");
        println!("Enumerating PCI config regions");
        let mut pci_devices: Vec<PciDevice> = Vec::new();

        for segment in 0..32 {
            for bus in 0..u8::MAX {
                if let Some(addr) = pci_regions.physical_address(segment, bus, 0, 0) {
                    println!(
                        "  segment = {:x}, bus = {:x}, device = {:x}, addr = {:x}",
                        segment, bus, 0, addr
                    );
                    for device in 0..=u8::MAX {
                        if let Some(device) = check(
                            &acpi_mapper,
                            &pci_regions,
                            segment,
                            bus,
                            device,
                        ) {
                            println!("    {:?}", device);
                            pci_devices.push(device);
                        }
                    }
                }
            }
        }
        pci_devices.shrink_to_fit();

        Ok(Acpi { pci_devices })
    } else {
        Err(AcpiInitError::NoRsdbAddr)
    }
}
#[derive(Debug)]
pub enum AcpiInitError {
    NoRsdbAddr,
}

#[derive(Debug)]
pub struct Acpi {
    pci_devices: Vec<PciDevice>,
}

#[derive(Debug)]
struct PciDevice {
    vendor_id: u16,
    device_id: u16,
    root_header_type: u8,
    functions: Vec<PciFunction>,
}

#[derive(Debug)]
struct PciFunction {
    classs: u8,
    subclass: u8,
    prog_if: u8,
    revision_id: u8,
    config_space: PhysFrame<Size4KiB>,
}

/// Memory mapped PCI config space.
#[repr(C, align(4096))]
struct PciConfigSpace {
    // Register 0
    vendor_id: Volatile<u16>,
    device_id: Volatile<u16>,
    // Register 1
    command: Volatile<u16>,
    status: Volatile<u16>,
    // Register 2
    revision_id: Volatile<u8>,
    prog_if: Volatile<u8>,
    subclass: Volatile<u8>,
    class_code: Volatile<u8>,
    // Register 3
    cache_line_size: Volatile<u8>,
    latency_timer: Volatile<u8>,
    header_type: Volatile<u8>,
    bist: Volatile<u8>,
    // Base Addresses
    base_addresses: [Volatile<u32>; 5],
    cardbus_cis_pointer: Volatile<u32>,
    // Register 0b
    subsystem_vendor_id: Volatile<u16>,
    subsystem_id: Volatile<u16>,
    // Register 0c
    expansion_rom_base_address: Volatile<u32>,
    // Register 0d
    capabilites_pointer: Volatile<u8>,
    _reserved_0d_1: Volatile<u8>,
    _reserved_0d_2: Volatile<u16>,
    _reserved_0e: Volatile<u32>,
    // Register 0f
    interrupt_line: Volatile<u8>,
    interrupt_pin: Volatile<u8>,
    min_grant: Volatile<u8>,
    max_latency: Volatile<u8>,
}

fn check(
    acpi_handler: &impl AcpiHandler,
    pci: &PciConfigRegions,
    segment_group_no: u16,
    bus: u8,
    device: u8,
) -> Option<PciDevice> {
    let mut dev = PciDevice {
        vendor_id: 0,
        device_id: 0,
        root_header_type: 0,
        functions: Vec::new()        
    };

    for function in 0..=u8::MAX {
        let address = pci.physical_address(segment_group_no, bus, device, function)?;    
        let frame = PhysFrame::from_start_address( PhysAddr::new(address) )
            .expect("PCI address not at start of frame");

        let config_space = unsafe { 
            acpi_handler.map_physical_region::<PciConfigSpace>(address as usize, 4096)
        };
        
        
        let vendor_id = config_space.vendor_id.read();
        if vendor_id == 0xFFFF {
            return None;
        }
        let device_id = config_space.device_id.read();
        let header_type = config_space.header_type.read();
        dev.functions.push(PciFunction {
            classs: config_space.class_code.read(),
            subclass: config_space.subclass.read(),
            prog_if: config_space.prog_if.read(),
            revision_id: config_space.revision_id.read(),
            config_space: frame,            
        });

        if function == 0 {
            dev.vendor_id = vendor_id;
            dev.device_id = device_id;            
            dev.root_header_type = header_type;
            // Check if it's a multi function device.
            if header_type & 0x80 == 0 {                
                break;
            }
        }
    }
    // TODO: Find extra functions.

    Some(dev)
}
