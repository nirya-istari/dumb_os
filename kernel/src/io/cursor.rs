use super::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use alloc::{boxed::Box, vec::Vec};
use core::{convert::TryInto, usize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor<B> {
    inner: B,
    position: u64,
}

impl<B> Cursor<B> {
    pub fn new(inner: B) -> Cursor<B> {
        Cursor { inner, position: 0 }
    }

    pub fn into_inner(self) -> B {
        self.inner
    }

    pub fn get_ref(&self) -> &B {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut B {
        &mut self.inner
    }

    pub fn position(&self) -> u64 {
        self.position
    }

    pub fn set_position(&mut self, position: u64) {
        self.position = position;
    }
}

impl<B> Cursor<B>
where
    B: AsRef<[u8]>,
{
    fn get_from_position(&self) -> &[u8] {
        let buffer = self.inner.as_ref();
        let start = usize::min(buffer.len(), self.position.try_into().unwrap_or(usize::MAX));
        &buffer[start..]
    }
}

impl<B> Read for Cursor<B>
where
    B: AsRef<[u8]>,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let data = self.get_from_position();
        if data.is_empty() {
            return Ok(0);
        }
        let read_len = usize::min(data.len(), buf.len());
        let src = &data[..read_len];
        let dest = &mut buf[..read_len];
        dest.copy_from_slice(src);
        self.position += read_len as u64;
        Ok(dest.len())
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        // We can improve performance as we have all the data avaliable now.
        let data = self.get_from_position();
        buf.extend_from_slice(data);
        Ok(data.len())
    }
}

impl<B> Seek for Cursor<B>
where
    B: AsRef<[u8]>,
{
    fn seek(&mut self, whence: SeekFrom) -> Result<u64> {
        let (base_pos, offset) = match whence {
            SeekFrom::Start(n) => {
                self.position = n;
                return Ok(n);
            }
            SeekFrom::End(n) => (self.inner.as_ref().len() as u64, n),
            SeekFrom::Current(n) => (self.position, n),
        };
        let new_pos = if offset >= 0 {
            base_pos.checked_add(offset as u64)
        } else {
            base_pos.checked_sub((offset.wrapping_neg()) as u64)
        };

        match new_pos {
            Some(n) => {
                self.position = n;
                Ok(self.position)
            }
            None => Err(Error::new(
                ErrorKind::InvalidInput,
                "invalid seek to a negative or overflowing position",
            )),
        }
    }

    fn stream_len(&mut self) -> Result<u64> {
        Ok(self.inner.as_ref().len() as u64)
    }

    fn stream_position(&mut self) -> Result<u64> {
        Ok(u64::min(self.position, self.inner.as_ref().len() as u64))
    }
}

fn slice_write(pos_mut: &mut u64, slice: &mut [u8], data: &[u8]) -> Result<usize> {
    let pos = u64::min(*pos_mut, slice.len() as u64) as usize;
    let amt = (&mut slice[pos..]).write(data)?;
    *pos_mut += amt as u64;
    Ok(amt)
}

impl Write for Cursor<&mut [u8]> {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        slice_write(&mut self.position, self.inner.as_mut(), data)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for Cursor<Box<[u8]>> {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        slice_write(&mut self.position, self.inner.as_mut(), data)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

fn vec_write(pos_mut: &mut u64, vec: &mut Vec<u8>, data: &[u8]) -> Result<usize> {
    let pos: usize = (*pos_mut).try_into().map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            "cursor position exceed maximum possible vector length",
        )
    })?;

    if vec.len() < pos {
        vec.resize(pos, 0);
    }

    let space = vec.len() - pos;
    let (left, right) = data.split_at(usize::min(space, data.len()));
    vec[pos..pos + left.len()].copy_from_slice(left);
    vec.extend_from_slice(right);

    *pos_mut = (pos + data.len()) as u64;
    Ok(data.len())
}

impl Write for Cursor<Vec<u8>> {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        vec_write(&mut self.position, &mut self.inner, data)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Write for Cursor<&mut Vec<u8>> {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        vec_write(&mut self.position, self.inner, data)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
