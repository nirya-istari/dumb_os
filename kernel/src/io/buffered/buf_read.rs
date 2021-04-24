use crate::io::{Error, ErrorKind, Read, Result};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

pub trait BufRead: Read {
    fn fill_buf(&mut self) -> Result<&[u8]>;
    fn consume(&mut self, amt: usize);

    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        read_until(self, byte, buf)
    }

    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        read_line(self, buf)
    }

    fn split(self, byte: u8) -> Split<Self>
    where
        Self: Sized,
    {
        Split { inner: self, byte }
    }

    fn lines(self) -> Lines<Self>
    where
        Self: Sized,
    {
        Lines { inner: self }
    }
}

fn read_until<R: BufRead + ?Sized>(r: &mut R, byte: u8, output: &mut Vec<u8>) -> Result<usize> {
    let start_size = output.len();
    loop {
        let result = r.fill_buf();
        match result {
            Ok(buf) if buf.is_empty() => return Ok(output.len() - start_size),
            Ok(buf) => {
                let index = buf.iter().position(|b| *b == byte);
                if let Some(index) = index {
                    output.extend_from_slice(&buf[..=index]);
                    r.consume(index);
                    return Ok(output.len() - start_size);
                } else {
                    output.extend_from_slice(buf);
                    let len = buf.len();
                    r.consume(len);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

fn read_line<R: BufRead + ?Sized>(r: &mut R, output: &mut String) -> Result<usize> {
    let start_size = output.len();
    loop {
        let result = r.fill_buf();
        match result {
            Ok(buf) if buf.is_empty() => return Ok(output.len() - start_size),
            Ok(buf) => {
                let (buf, done) = match buf.iter().position(|b| *b == b'\n') {
                    Some(index) => (&buf[..=index], true),
                    None => (buf, false),
                };
                let s: &str = match core::str::from_utf8(buf) {
                    Ok(s) => s,
                    Err(err) => return Err(Error::new(ErrorKind::InvalidInput, err)),
                };
                output.push_str(s);
                let len = buf.len();
                r.consume(len);
                if done {
                    return Ok(output.len() - start_size);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

pub struct Split<R> {
    inner: R,
    byte: u8,
}
impl<R> Iterator for Split<R>
where
    R: BufRead,
{
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut output = Vec::new();
        match self.inner.read_until(self.byte, &mut output) {
            Ok(0) => None,
            Ok(_) => Some(Ok(output)),
            Err(err) => Some(Err(err)),
        }
    }
}

pub struct Lines<R> {
    inner: R,
}
impl<R> Iterator for Lines<R>
where
    R: BufRead,
{
    type Item = core::result::Result<String, crate::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buffer = String::new();
        match self.inner.read_line(&mut buffer) {
            Ok(0) => None,
            Ok(_) => Some(Ok(buffer)),
            Err(err) => Some(Err(err)),
        }
    }
}

impl BufRead for &[u8] {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        Ok(&**self)
    }

    fn consume(&mut self, amt: usize) {
        *self = &self[amt..];
    }
}

impl<R> BufRead for &mut R
where
    R: BufRead + ?Sized,
{
    fn fill_buf(&mut self) -> Result<&[u8]> {
        (**self).fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        (**self).consume(amt)
    }

    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_until(byte, buf)
    }

    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_line(buf)
    }
}

impl<R> BufRead for Box<R>
where
    R: BufRead + ?Sized,
{
    fn fill_buf(&mut self) -> Result<&[u8]> {
        (**self).fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        (**self).consume(amt)
    }

    fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        (**self).read_until(byte, buf)
    }

    fn read_line(&mut self, buf: &mut String) -> Result<usize> {
        (**self).read_line(buf)
    }
}
