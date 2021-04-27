
use x86_64::structures::paging::{OffsetPageTable};

use crate::memory::BootInfoBumpAllocator;


#[derive(Debug)]
pub struct MemoryManager {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BootInfoBumpAllocator,
}
