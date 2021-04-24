use crate::io::{BufRead, Read, Result};

#[derive(Debug)]
pub struct Chain<A, B> {
    // Drops first when EOF.
    pub(crate) first: A,
    pub(crate) second: B,
    pub(crate) done_first: bool,
}

impl<A, B> Chain<A, B> {
    pub fn into_inner(self) -> (A, B) {
        (self.first, self.second)
    }

    pub fn get_ref(&self) -> (&A, &B) {
        (&self.first, &self.second)
    }

    pub fn get_mut(&mut self) -> (&mut A, &mut B) {
        (&mut self.first, &mut self.second)
    }
}

impl<A, B> Read for Chain<A, B>
where
    A: Read,
    B: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.done_first {
            match self.first.read(buf) {
                Ok(0) => {
                    self.done_first = true;
                }
                Ok(n) => {
                    assert!(n <= buf.len());
                    return Ok(n);
                }
                Err(err) => return Err(err),
            }
        }
        self.second.read(buf)
    }
}

impl<A, B> BufRead for Chain<A, B>
where
    A: BufRead,
    B: BufRead,
{
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if !self.done_first {
            match self.first.fill_buf() {
                Ok(buf) if buf.is_empty() => {
                    self.done_first = true;
                }
                other => return other,
            }
        }
        self.second.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        if !self.done_first {
            self.first.consume(amt)
        } else {
            self.second.consume(amt)
        }
    }
}
