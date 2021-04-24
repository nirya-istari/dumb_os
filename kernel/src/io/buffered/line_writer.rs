use crate::io::{BufWriter, IntoInnerError, Result, Write};
use core::{fmt, slice::memchr::memchr};

pub struct LineWriter<W: Write> {
    inner: BufWriter<W>,
}

impl<W: Write> LineWriter<W> {
    pub fn new(inner: W) -> LineWriter<W> {
        LineWriter::with_capacity(1024, inner)
    }

    pub fn with_capacity(capacity: usize, inner: W) -> LineWriter<W> {
        LineWriter {
            inner: BufWriter::with_capacity(capacity, inner),
        }
    }

    pub fn get_ref(&self) -> &W {
        self.inner.get_ref()
    }

    pub fn get_mut(&mut self) -> &mut W {
        self.inner.get_mut()
    }

    pub fn into_inner(self) -> core::result::Result<W, IntoInnerError<LineWriter<W>>> {
        self.inner
            .into_inner()
            .map_err(|err| err.new_wrapped(|inner| LineWriter { inner }))
    }
}

impl<W: Write> Write for LineWriter<W> {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let n = self.inner.write(data)?;
        let index = memchr(b'\n', self.inner.buffer());
        if index.is_some() {
            self.inner.flush()?;
        }
        Ok(n)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

impl<W: Write> fmt::Debug for LineWriter<W>
where
    W: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("LineWriter")
            .field("writer", &self.get_ref())
            .field(
                "buffer",
                &format_args!("{}/{}", self.inner.buffer().len(), self.inner.capacity()),
            )
            .finish()
    }
}
