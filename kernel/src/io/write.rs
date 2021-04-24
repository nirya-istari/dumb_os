use super::{Error, ErrorKind, Result};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::usize;

pub trait Write {
    fn write(&mut self, data: &[u8]) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;

    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        write_all(self, data)
    }

    fn write_fmt(&mut self, fmt: core::fmt::Arguments<'_>) -> Result<()> {
        struct Adaptor<'a, W: ?Sized + 'a> {
            inner: &'a mut W,
            result: Result<()>,
        }

        impl<'a, W: Write + ?Sized> core::fmt::Write for Adaptor<'a, W> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let res = self.inner.write_all(s.as_bytes());
                self.result = res.map(|_| ());
                match &self.result {
                    Ok(_) => Ok(()),
                    Err(_) => Err(core::fmt::Error),
                }
            }
        }

        let mut w = Adaptor {
            inner: self,
            result: Ok(()),
        };
        match core::fmt::write(&mut w, fmt) {
            Ok(()) => Ok(()),
            Err(_) => {
                if w.result.is_ok() {
                    w.result
                } else {
                    Err(Error::new(ErrorKind::Other, "formatting error"))
                }
            }
        }
    }

    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

fn write_all<W>(w: &mut W, buf: &[u8]) -> Result<()>
where
    W: Write + ?Sized,
{
    if buf.len() == 0 {
        return Ok(());
    }

    let mut remaining = buf;
    while !remaining.is_empty() {
        let n = w.write(remaining)?;
        remaining = &remaining[n..];
    }
    Ok(())
}

#[derive(Debug, Default)]
pub struct Sink {
    _private: (),
}
pub fn sink() -> Sink {
    Sink { _private: () }
}

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    fn write_all(&mut self, _: &[u8]) -> Result<()> {
        Ok(())
    }

    fn write_fmt(&mut self, _: core::fmt::Arguments<'_>) -> Result<()> {
        Ok(())
    }
}
impl Write for &Sink {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    fn write_all(&mut self, _: &[u8]) -> Result<()> {
        Ok(())
    }

    fn write_fmt(&mut self, _: core::fmt::Arguments<'_>) -> Result<()> {
        Ok(())
    }
}

impl Write for &mut [u8] {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let amt = usize::min(buf.len(), self.len());

        let (a, b) = core::mem::replace(self, &mut []).split_at_mut(amt);

        a.copy_from_slice(&buf[..amt]);
        *self = b;
        Ok(amt)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        if self.write(buf)? == buf.len() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::WriteZero,
                "Failed to write whole buffer",
            ))
        }
    }
}

impl Write for Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl<W: Write + ?Sized> Write for &mut W {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        (**self).write_all(buf)
    }

    fn write_fmt(&mut self, fmt: core::fmt::Arguments<'_>) -> Result<()> {
        (**self).write_fmt(fmt)
    }
}

impl<W: Write + ?Sized> Write for Box<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        (**self).write_all(buf)
    }

    fn write_fmt(&mut self, fmt: core::fmt::Arguments<'_>) -> Result<()> {
        (**self).write_fmt(fmt)
    }
}
