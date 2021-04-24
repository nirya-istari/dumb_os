use super::{BufRead, Error, ErrorKind, Read, Result, Seek, SeekFrom};
use alloc::{vec::Vec, string::String};

#[derive(Debug)]
pub struct Empty {
    _private: (),
}

pub fn empty() -> Empty {
    Empty { _private: () }
}


impl BufRead for Empty {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        Ok(&[])
    }

    fn consume(&mut self, _: usize) {}

    fn read_until(&mut self, _byte: u8, _buf: &mut Vec<u8>) -> Result<usize> {
        Ok(0)
    }

    fn read_line(&mut self, _: &mut String) -> Result<usize> {
        Ok(0)
    }
}



impl Read for Empty {
    fn read(&mut self, _: &mut [u8]) -> Result<usize> {
        Ok(0)
    }

    fn read_to_end(&mut self, _: &mut Vec<u8>) -> Result<usize> {
        Ok(0)
    }

    fn read_to_string(&mut self, _: &mut String) -> Result<usize> {
        Ok(0)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        if buf.len() > 0 {
            Err(Error::new(ErrorKind::UnexpectedEof, "read_exact: Empty does not contain any data."))
        } else {
            Ok(())
        }
    }
}


impl Seek for Empty {
    fn seek(&mut self, _: SeekFrom) -> Result<u64> {
        Ok(0)
    }

    fn stream_len(&mut self) -> Result<u64> {
        Ok(0)
    }

    fn stream_position(&mut self) -> Result<u64> {
        Ok(0)
    }
}