use super::{BufRead, Read, Result};

pub struct Take<R> {
    pub(crate) inner: R,
    pub(crate) limit: u64
}

impl<R: Read> Read for Take<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if self.limit == 0 { return Ok(0); }


        let len = u64::min(self.limit, buf.len() as u64) as usize;

        let buf = &mut buf[..len];
        match self.inner.read(buf) {            
            Ok(n) => {
                assert!(buf.len() >= n);
                self.limit -= n as u64;
                Ok(n)
            }
            err => err
        }
    }
}

impl<R> BufRead for Take<R> where R: BufRead {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.limit == 0 {
            return Ok(&[]);
        }

        let res = self.inner.fill_buf();
        if let Ok(buf) = res {
            let len = u64::min(self.limit, buf.len() as u64) as usize;
            self.limit -= len as u64;
            Ok(&buf[..len])
        } else {
            res
        }
    }

    fn consume(&mut self, amt: usize) {
        let len = u64::min(self.limit, amt as u64) as usize;
        self.limit -= len as u64;
        self.inner.consume(len);
    }
}
