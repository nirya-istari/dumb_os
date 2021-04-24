use super::{Read, Result, Write};
use alloc::vec::Vec;

pub fn copy<R: ?Sized, W: ?Sized>(reader: &mut R, writer: &mut W) -> Result<usize>
where
    R: Read,
    W: Write,
{
    // Because we're not std and don't have access to Rust 2077 features.
    // We just have to do with pretentind BufRead doesn't exists and making our
    // own buffer.
    // Here we allocate a buffer. Stack allocating buffer in no_std envirments
    // could easily lead to stack overflows. By using the heap we can raid any
    // out of memory errors sensibly.
    let mut buffer: Vec<u8> = Vec::new();
    buffer.resize(4096, 0);
    let mut total = 0;

    loop {
        let amt = reader.read(buffer.as_mut())?;
        if amt == 0 {
            return Ok(total);
        }
        writer.write_all(&buffer[..amt])?;
        total += amt;
    }
}
