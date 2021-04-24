use crate::io::{BufRead, Read, Result, Seek, SeekFrom};
use alloc::boxed::Box;
use alloc::vec;
use core::fmt;

pub struct BufReader<R> {
    inner: R,
    buffer: Box<[u8]>,
    pos: usize,
    cap: usize,
}

impl<R: Read> BufReader<R> {
    pub fn new(inner: R) -> Self {
        Self::with_capacity(4096, inner)
    }

    pub fn with_capacity(capacity: usize, inner: R) -> BufReader<R> {
        BufReader {
            inner,
            buffer: vec![0; capacity].into_boxed_slice(),
            pos: 0,
            cap: 0,
        }
    }
}

impl<R> BufReader<R> {
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buffer[self.pos..self.cap]
    }

    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    pub fn into_inner(self) -> R {
        self.inner
    }

    fn discard_buffer(&mut self) {
        self.pos = 0;
        self.cap = 0;
    }
}

impl<R: Read> Read for BufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut my_buf = self.fill_buf()?;

        let amt = my_buf.read(buf)?;
        self.consume(amt);
        Ok(amt)
    }
}

impl<R: Read> BufRead for BufReader<R> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.cap == self.pos {
            self.inner.read(&mut self.buffer)?;
        }
        Ok(self.buffer())
    }

    fn consume(&mut self, amt: usize) {
        if self.pos + amt > self.cap {
            panic!("Invalid consume length")
        }
        self.pos += amt;
        if self.pos == self.cap {
            self.discard_buffer();
        }
    }
}

impl<R: Seek> Seek for BufReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let result: u64;
        if let SeekFrom::Current(n) = pos {
            let remainder = (self.cap - self.pos) as i64;
            // it should be safe to assume that remainder fits within an i64 as the alternative
            // means we managed to allocate 8 exbibytes and that's absurd.
            // But it's not out of the realm of possibility for some weird underlying reader to
            // support seeking by i64::MIN so we need to handle underflow when subtracting
            // remainder.
            if let Some(offset) = n.checked_sub(remainder) {
                result = self.inner.seek(SeekFrom::Current(offset))?;
            } else {
                // seek backwards by our remainder, and then by the offset
                self.inner.seek(SeekFrom::Current(-remainder))?;
                self.discard_buffer();
                result = self.inner.seek(SeekFrom::Current(n))?;
            }
        } else {
            // Seeking with Start/End doesn't care about our buffer length.
            result = self.inner.seek(pos)?;
        }
        self.discard_buffer();
        Ok(result)
    }

    fn stream_len(&mut self) -> Result<u64> {
        self.inner.stream_len()
    }

    fn stream_position(&mut self) -> Result<u64> {
        let remainder = (self.cap - self.pos) as u64;
        Ok(self
            .inner
            .stream_position()?
            .checked_sub(remainder)
            .expect("overflow when subtracting remaining buffer size from inner stream position"))
    }
}

impl<R> fmt::Debug for BufReader<R>
where
    R: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("BufReader")
            .field("reader", &self.inner)
            .field(
                "buffer",
                &format_args!("{}/{}", self.cap - self.pos, self.buffer.len()),
            )
            .finish()
    }
}
