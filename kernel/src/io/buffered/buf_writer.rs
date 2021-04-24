use crate::io::{Error, Result, Write, DEFAULT_BUF_CAPACITY};
use alloc::vec::Vec;
use core::fmt::Display;

#[derive(Debug)]
pub struct BufWriter<W> {
    inner: W,
    buffer: Vec<u8>,
}

impl<W: Write> BufWriter<W> {
    pub fn new(inner: W) -> BufWriter<W> {
        BufWriter::with_capacity(DEFAULT_BUF_CAPACITY, inner)
    }

    pub fn with_capacity(capacity: usize, inner: W) -> BufWriter<W> {
        BufWriter {
            inner: inner,
            buffer: Vec::with_capacity(capacity),
        }
    }

    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buffer[..]
    }

    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    pub fn into_inner(mut self) -> core::result::Result<W, IntoInnerError<BufWriter<W>>> {
        match self.flush_buffer() {
            Ok(_) => Ok(self.inner),
            Err(err) => Err(IntoInnerError(self, err)),
        }
    }

    fn flush_buffer(&mut self) -> Result<()> {
        let res = self.inner.write_all(self.buffer.as_ref())?;
        self.buffer.truncate(0);
        Ok(res)
    }
}

impl<W> Write for BufWriter<W>
where
    W: Write,
{
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        if self.buffer.len() + data.len() > self.buffer.capacity() {
            self.flush_buffer()?;
        }
        if data.len() >= self.buffer.capacity() {
            self.get_mut().write(data)
        } else {
            self.buffer.extend_from_slice(data);
            Ok(data.len())
        }
    }

    fn flush(&mut self) -> Result<()> {
        self.flush_buffer().map(|_| ())
    }

    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        if self.buffer.len() + data.len() > self.buffer.capacity() {
            self.flush_buffer()?;
        }

        if data.len() >= self.buffer.capacity() {
            self.get_mut().write_all(data)
        } else {
            self.buffer.extend_from_slice(data);
            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct IntoInnerError<W>(W, Error);

impl<W> IntoInnerError<W> {
    /// Construct a new IntoInnerError
    pub(crate) fn new(writer: W, error: Error) -> Self {
        Self(writer, error)
    }

    /// Helper to construct a new IntoInnerError; intended to help with
    /// adapters that wrap other adapters
    pub(crate) fn new_wrapped<W2>(self, f: impl FnOnce(W) -> W2) -> IntoInnerError<W2> {
        let Self(writer, error) = self;
        IntoInnerError::new(f(writer), error)
    }

    pub fn error(&self) -> &Error {
        &self.1
    }

    pub fn into_inner(self) -> W {
        self.0
    }
}

impl<W> Display for IntoInnerError<W> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self.error(), f)
    }
}

impl<W> crate::error::Error for IntoInnerError<W>
where
    W: Send + core::fmt::Debug,
{
    fn source(&self) -> Option<&(dyn crate::error::Error + 'static)> {
        Some(&self.1)
    }
}

impl<W> From<IntoInnerError<W>> for Error {
    fn from(i: IntoInnerError<W>) -> Self {
        i.1
    }
}
