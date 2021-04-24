use super::{Chain, Error, ErrorKind, Result, Take};
use alloc::string::String;
use alloc::vec::Vec;

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        read_to_end(self, buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        append_to_string(buf, |b| read_to_end(self, b))
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        read_exact(self, buf)
    }

    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    fn bytes(self) -> Bytes<Self>
    where
        Self: Sized,
    {
        Bytes { inner: self }
    }

    fn chain<Other>(self, next: Other) -> Chain<Self, Other>
    where
        Self: Sized,
        Other: Read + Sized,
    {
        Chain {
            first: self,
            second: next,
            done_first: false,
        }
    }

    fn take(self, limit: u64) -> Take<Self>
    where
        Self: Sized,
    {
        Take { inner: self, limit }
    }
}

// Guard shrinks buffer in case of panic.
struct Guard<'a> {
    buf: &'a mut Vec<u8>,
    len: usize,
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        unsafe {
            self.buf.set_len(self.len);
        }
    }
}

fn append_to_string<F>(buf: &mut String, f: F) -> Result<usize>
where
    F: FnOnce(&mut Vec<u8>) -> Result<usize>,
{
    // Copied directly from std because of unsafty.

    unsafe {
        let mut g = Guard {
            len: buf.len(),
            buf: buf.as_mut_vec(),
        };
        let ret = f(g.buf);
        if core::str::from_utf8(&g.buf[g.len..]).is_err() {
            ret.and_then(|_| {
                Err(Error::new(
                    ErrorKind::InvalidData,
                    "stream did not contain valid UTF-8",
                ))
            })
        } else {
            g.len = g.buf.len();
            ret
        }
    }
}

pub struct Bytes<R> {
    inner: R,
}

impl<R: Read> Iterator for Bytes<R> {
    type Item = Result<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf: [u8; 1] = [0];
        match self.inner.read(&mut buf) {
            Ok(0) => None,
            Ok(_) => Some(Ok(buf[0])),
            Err(err) => Some(Err(err)),
        }
    }
}

fn read_to_end<R>(r: &mut R, output: &mut Vec<u8>) -> Result<usize>
where
    R: Read + ?Sized,
{
    // My fantabulous no-unsafe version of the std read_to_end.
    // This is partly because we don't have the whole nightly initialize api.
    let start_len = output.len();
    loop {
        output.reserve(32);
        let buf_start = output.len();
        output.resize(output.capacity(), 0);

        let buf = &mut output[buf_start..];
        match r.read(buf) {
            Ok(0) => {
                // EOF. We're done here.
                output.truncate(buf_start);
                return Ok(output.len() - start_len);
            }
            Ok(n) => {
                // We got something. Chop off extra.
                output.truncate(buf_start + n);
            }
            Err(err) if err.kind() == ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
    }
}

pub fn read_exact<R>(r: &mut R, buf: &mut [u8]) -> Result<()>
where
    R: Read + ?Sized,
{
    if buf.is_empty() {
        return Ok(());
    }

    let mut left = buf;
    loop {
        match r.read(left) {
            Ok(0) => {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "read_exact: reached EOF before end of buffer.",
                ));
            }
            Ok(n) if n == left.len() => {
                return Ok(());
            }
            Ok(n) => {
                left = &mut left[n..];
            }
            Err(err) if err.kind() == ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
    }
}

impl Read for &[u8] {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let len = usize::min(self.len(), buf.len());

        let src = &(*self)[..len];
        let dest = &mut buf[..len];
        dest.copy_from_slice(src);

        *self = &(*self)[len..];

        Ok(len)
    }
}

pub struct Repeat {
    byte: u8,
}

pub fn repeat(byte: u8) -> Repeat {
    Repeat { byte }
}

impl Read for Repeat {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        for b in buf.iter_mut() {
            *b = self.byte
        }
        Ok(buf.len())
    }
}

impl<R: Read + ?Sized> Read for &mut R {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).read(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        (**self).read_exact(buf)
    }
}

impl<R: Read + ?Sized> Read for alloc::boxed::Box<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).read(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        (**self).read_exact(buf)
    }
}
