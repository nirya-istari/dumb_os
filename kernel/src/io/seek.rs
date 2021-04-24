use super::Result;
use alloc::boxed::Box;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

pub trait Seek {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64>;

    fn stream_len(&mut self) -> Result<u64> {
        let orig_position = self.stream_position()?;
        self.seek(SeekFrom::End(0))?;
        let end_positon = self.stream_position()?;
        self.seek(SeekFrom::Start(orig_position))?;
        Ok(end_positon)
    }

    fn stream_position(&mut self) -> Result<u64> {
        self.seek(SeekFrom::Current(0))
    }
}

impl<S: Seek + ?Sized> Seek for &mut S {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        (**self).seek(pos)
    }

    fn stream_position(&mut self) -> Result<u64> {
        (**self).stream_position()
    }
}

impl<S: Seek + ?Sized> Seek for Box<S> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        (**self).seek(pos)
    }

    fn stream_position(&mut self) -> Result<u64> {
        (**self).stream_position()
    }
}
