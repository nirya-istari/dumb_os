use core::{fmt, mem, str::{self, Utf8Error}};

use bootloader::{boot_info::Optional, BootInfo};
use mem::size_of;
use x86_64::VirtAddr;
use field_offset::offset_of;

use crate::prelude::*;

mod tables;

/// For documenatantive reasons.
/// To copy a `Copy` field in Rust you simple put it in a block all to itself:
/// ```
/// let x: i32 = 42;
/// let y: i32 = {val};
/// assert_eq!(x, y);
/// ```
/// It's not *obvious* that this is why the braces are there. To make it obvious
/// we create a mostly redeundent macro that describes why this is done.
/// (The unaligned part is because calling `.clone()` on a unaligned field is
/// undefined behavior. )
macro_rules! copy_unaligned_field {
    ($v:expr) => {{
        $v
    }};
}

pub fn init(physical_offset: VirtAddr, bootinfo: &mut BootInfo) -> Result<Acpi, AcpiInitError> {
    if let Some(rdsp_addr) = mem::replace(&mut bootinfo.rsdp_addr, Optional::None).into_option() {
        println!("rdsp_addr = {}", rdsp_addr);
        let rsdp_ptr = (physical_offset.as_u64() + rdsp_addr) as *const RsdpDescriptor;

        let rsdp = unsafe { RsdpDescriptor::validate(rsdp_ptr) }?;
        println!("rsdp = {:?}", rsdp);

        let rsdt = rsdp.rsdt(physical_offset);
        println!("rsdt = {:?}", rsdt);

        for entry in rsdt.get_entries() {
            let header = DescriptionHeader::from_addr(physical_offset, *entry);
            println!("header = {:?}", header);
        }

        todo!()
    } else {
        Err(AcpiInitError::NoRsdbAddr)
    }
}
#[derive(Debug)]
pub enum AcpiInitError {
    NoRsdbAddr,
    InvalidRsdpSignatureUtf8Error(Utf8Error),
    IncorrectRsdbSignature(&'static str),
    // So orignally Acpi 1.0 wasn't going to be supported, but it turns out QEMU uses it.
    Acpi10NotSupported,
    RsdpChecksumInvalid,
}

#[repr(packed)]
struct RsdpDescriptor {
    // 1.0
    signature: [u8; 8], /* offset: 0 */
    checksum: u8,       /* offset: 8 */
    oem_id: [u8; 6],    /* offset: 9 */
    revision: u8,       /* offset: 15 */
    rsdt_address: u32,  /* offset 16 */
    // 2.0 and later
    length: u32,           /* offset: 20 */
    xsdt_address: u64,     /* offset: 24 */
    extended_checksum: u8, /* offset: 32 */
    _reserved: [u8; 3],    /* offset: 33 */
}

#[test_case]
fn rsdp_descriptor_layout() {
    assert_eq!(0, offset_of!(RsdpDescriptor => signature).get_byte_offset());
    assert_eq!(8, offset_of!(RsdpDescriptor => checksum).get_byte_offset());
    assert_eq!(9, offset_of!(RsdpDescriptor => oem_id).get_byte_offset());
    assert_eq!(15, offset_of!(RsdpDescriptor => revision).get_byte_offset());
    assert_eq!(16, offset_of!(RsdpDescriptor => rsdt_address).get_byte_offset());
    assert_eq!(20, offset_of!(RsdpDescriptor => length).get_byte_offset());
    assert_eq!(24, offset_of!(RsdpDescriptor => xsdt_address).get_byte_offset());
    assert_eq!(32, offset_of!(RsdpDescriptor => extended_checksum).get_byte_offset());
    assert_eq!(33, offset_of!(RsdpDescriptor => _reserved).get_byte_offset());
}


impl fmt::Debug for RsdpDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = f.debug_struct("RsdpDescriptor");
        builder
            .field("signature", unsafe {
                // SAFETY: signature is checked in Self::validate so can safely and
                // convert it.
                &str::from_utf8_unchecked(&self.signature)
            })
            .field("checksum", &self.checksum)
            .field("oem_id", &self.oem_id())
            .field("revision", &self.revision)
            .field(
                "rsdt_address",
                &format_args!("0x{:x}", copy_unaligned_field!(self.rsdt_address)),
            );
        if self.revision >= 2 {
            builder
                .field("length", &copy_unaligned_field!(self.length))
                .field(
                    "xdt_address",
                    &format_args!("0x{:x}", copy_unaligned_field!(self.xsdt_address)),
                )
                .field("extended_checksum", &self.extended_checksum)
                .finish()
        } else {
            // Don't print 2.0 parts of a 1.0 header.
            builder.finish_non_exhaustive()
        }
    }
}

impl RsdpDescriptor {
    unsafe fn validate(
        ptr: *const RsdpDescriptor,
    ) -> Result<&'static RsdpDescriptor, AcpiInitError> {
        let rsdp: &'static RsdpDescriptor =  ptr.as_ref().unwrap();

        let signature = str::from_utf8(&rsdp.signature[..])
            .map_err(|err| AcpiInitError::InvalidRsdpSignatureUtf8Error(err))?;
        if signature != "RSD PTR " {
            return Err(AcpiInitError::IncorrectRsdbSignature(signature));
        }

        // Need to copy unaligned fields. Ref (&'s) cannot reference unaligned fields.
        let checksum_1 = checksum_bytes(rsdp.signature)
            .wrapping_add(rsdp.checksum)
            .wrapping_add(checksum_bytes(rsdp.oem_id))
            .wrapping_add(rsdp.revision)
            .wrapping_add(checksum_bytes(copy_unaligned_field!(rsdp.rsdt_address).as_ne_bytes()));
        if checksum_1 != 0 {
            return Err(AcpiInitError::RsdpChecksumInvalid);
        }

        if rsdp.revision >= 2 {
            // We technically need to add checksum_1. But it should be zero now.
            let checksum_2 = checksum_bytes(copy_unaligned_field!(rsdp.length).as_ne_bytes())
                .wrapping_add(checksum_bytes(copy_unaligned_field!(rsdp.xsdt_address).as_ne_bytes()))
                .wrapping_add(rsdp.extended_checksum)
                .wrapping_add(checksum_bytes(rsdp._reserved));
            if checksum_2 != 0 {
                return Err(AcpiInitError::RsdpChecksumInvalid);
            }
        }

        Ok(rsdp)
    }

    /// Get a reference to the rsdp descriptor's oem id.
    fn oem_id(&self) -> Result<&str, &[u8]> {
        str_or_bytes(&self.oem_id)        
    }

    unsafe fn unsafe_rsdt(&'static self, physical_offset: VirtAddr) -> &'static Rdst {
        let ptr = (physical_offset.as_u64() + self.rsdt_address as u64) as *const Rdst;
        
        ptr.as_ref().expect("Rdst cannot be mapped to 0x0")
    }

    fn rsdt(&'static self, physical_offset: VirtAddr) -> &'static Rdst {
        let rsdt = unsafe { self.unsafe_rsdt(physical_offset) };
        if &rsdt.signature != b"RSDT" {
            panic!("rsdt signature invalid");
        }

        let ptr = rsdt as *const _ as *const u8;
        let slice = unsafe {
            core::slice::from_raw_parts(ptr, rsdt.length as usize)
        };
        let checksum = checksum_bytes(slice);
        if checksum != 0 {
            panic!("RSDT checksum is incorrect {}", checksum);
        }        

        rsdt
    }
}

fn str_or_bytes(b: &[u8]) -> Result<&str, &[u8]> 
{
    let r: &[u8] = b.as_ref();
    match str::from_utf8(r) {
        Ok(s) => Ok(s),
        Err(_) => Err(r),
    }
}

fn checksum_bytes<B: AsRef<[u8]>>(b: B) -> u8 {
    b.as_ref().iter().fold(0, |a, b| u8::wrapping_add(a, *b))
}

#[repr(C)]
struct Rdst {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: [u8; 4],
    creator_revision: u32,
    // So this is start of an array of many values. But here we're 'pretend' it's length 1 because DST's are hard.
    // and I don't want the compiler doing anything smart with a zero sized type.
    entries: [u32; 1],
}

#[test_case]
fn rsdt_layout() {
    assert_eq!(0, offset_of!(Rdst => signature).get_byte_offset(), "signature");
    assert_eq!(4, offset_of!(Rdst => length).get_byte_offset(), "length");
    assert_eq!(8, offset_of!(Rdst => revision).get_byte_offset(), "revision");
    assert_eq!(9, offset_of!(Rdst => checksum).get_byte_offset(), "checksum");
    assert_eq!(10, offset_of!(Rdst => oem_id).get_byte_offset(), "oem_id");
    assert_eq!(16, offset_of!(Rdst => oem_table_id).get_byte_offset(), "oem_table_id");
    assert_eq!(24, offset_of!(Rdst => oem_revision).get_byte_offset(), "oem_revision");
    assert_eq!(28, offset_of!(Rdst => creator_id).get_byte_offset(), "creator_id");
    assert_eq!(32, offset_of!(Rdst => creator_revision).get_byte_offset(), "creator_revision");
    assert_eq!(36, offset_of!(Rdst => entries).get_byte_offset(), "entries");
}

#[test_case]
fn rsdt_entries_offset() {
    const NUM_ENTRIES: usize = 1;
    let test_table = Rdst {
        signature: *b"RDST",
        length: 36 + (NUM_ENTRIES * size_of::<u32>()) as u32, 
        revision: 1,
        checksum: 0x69,
        oem_id: *b"TESTIN",
        oem_table_id: *b"dumb_os ",
        oem_revision: 12,
        creator_id: *b"ME  ",
        creator_revision: 1,
        entries: [42],
    };
    let test_table_addr = (&test_table as *const Rdst) as i64;
    let entries = test_table.get_entries();
    let entries_addr = entries.as_ptr() as i64;
    assert_eq!(36, entries_addr - test_table_addr, ".get_entries()");
    assert_eq!(1, entries.len());
}

impl Rdst {

    fn get_entries(&self) -> &[u32] {        

        let entry_count = (self.length as usize - offset_of!(Rdst => entries ).get_byte_offset()) / size_of::<u32>();
        // 36 is aligned so this fine.
        let entry_ptr = {&self.entries[0]} as *const u32;
        assert_eq!((self as *const _ as u64) % 4, 0);
        assert_eq!((self as *const _ as u64) + 36, entry_ptr as u64);
        assert_eq!((entry_ptr as u64) % 4, 0);
        
        
        unsafe {            
            core::slice::from_raw_parts(entry_ptr, entry_count)
        }
    }

}

impl fmt::Debug for Rdst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rdst")
            .field("signature", &str::from_utf8(&self.signature))
            .field("length", &{self.length} )
            .field("revision", &self.revision)
            .field("checksum", &self.checksum)
            .field("oem_id", &str_or_bytes(&self.oem_id))
            .field("oem_table_id", &str_or_bytes(&self.oem_table_id))
            .field("oem_revision", &{self.oem_revision})
            .field("creator_id", &str_or_bytes(&self.creator_id))
            .field("creator_revision", &{self.creator_revision})
            .field("entries", &self.get_entries())
            .finish()
    }
}

/// Unique object in charge oc ACPI.
#[derive(Debug)]
pub struct Acpi {
    rsdp: &'static RsdpDescriptor,
    rsdt: &'static Rdst,
}


#[repr(C)]
struct DescriptionHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: [u8; 4],
    creator_revision: u32,
}

#[test_case]
fn description_header_layout() {
    assert_eq!(0, offset_of!(DescriptionHeader => signature).get_byte_offset(), "signature");
    assert_eq!(4, offset_of!(DescriptionHeader => length).get_byte_offset(), "length");
    assert_eq!(8, offset_of!(DescriptionHeader => revision).get_byte_offset(), "revision");
    assert_eq!(9, offset_of!(DescriptionHeader => checksum).get_byte_offset(), "checksum");
    assert_eq!(10, offset_of!(DescriptionHeader => oem_id).get_byte_offset(), "oem_id");
    assert_eq!(16, offset_of!(DescriptionHeader => oem_table_id).get_byte_offset(), "oem_table_id");
    assert_eq!(24, offset_of!(DescriptionHeader => oem_revision).get_byte_offset(), "oem_revision");
    assert_eq!(28, offset_of!(DescriptionHeader => creator_id).get_byte_offset(), "creator_id");
    assert_eq!(32, offset_of!(DescriptionHeader => creator_revision).get_byte_offset(), "creator_revision");
}

impl fmt::Debug for DescriptionHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptionHeader")
            .field("signature", &str_or_bytes(&self.signature))
            .field("length", &self.length)
            .field("revision", &self.revision)
            .field("checksum", &self.checksum)
            .field("oem_id", &str_or_bytes(&self.oem_id))
            .field("oem_table_id", &str_or_bytes(&self.oem_table_id))
            .field("oem_revision", &self.revision)
            .field("creator_id", &str_or_bytes(&self.creator_id))
            .field("creator_revision", &self.creator_revision)
            .finish()
    }
}


impl DescriptionHeader {
    fn from_addr(physical_offset: VirtAddr, addr: u32) -> &'static DescriptionHeader {
        let ptr = (physical_offset.as_u64() + addr as u64) as *const DescriptionHeader;
        

        let header = unsafe {ptr.as_ref()}.expect("description header at 0x0");
        let checksum = unsafe {
            checksum_bytes(&core::slice::from_raw_parts(ptr as *const u8, header.length as usize))
        };
        if checksum != 0 {
            panic!("Descriotion header at {:x} checksum is invalid. signature = {:?}", addr, str_or_bytes(&header.signature));
        }

        header
    }
}



