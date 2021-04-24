use alloc::{borrow::Cow, boxed::Box};
use alloc::string::String;
use core::{fmt::{self, Debug, Display}, str};

pub trait Error: Debug + Display {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    #[cfg(features = "error_description")]
    #[deprecated(note = "use Display impl or to_string()")]
    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    #[deprecated(note = "Replaced with StdError::source")]
    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl<'a, E: Error + Send + Sync + 'a> From<E> for Box<dyn Error + Send + Sync + 'a> {
    fn from(err: E) -> Self {
        Box::new(err)
    }
}

#[allow(deprecated)]
impl<'a, T: Error + ?Sized> Error for &'a T {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        (**self).source()
    }

    #[cfg(features = "error_description")]
    fn description(&self) -> &str {
        (**self).description()
    }

    fn cause(&self) -> Option<&dyn Error> {
        (**self).cause()
    }
}

#[allow(deprecated)]
impl<'a, T: Error> Error for Box<T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        (**self).source()
    }

    #[cfg(features = "error_description")]
    #[allow(deprecated)]
    fn description(&self) -> &str {
        (**self).description()
    }

    fn cause(&self) -> Option<&dyn Error> {
        (**self).cause()
    }
}



struct StringError(String);
impl Error for StringError {

    #[cfg(features = "error_description")]
    #[allow(deprecated)]
    fn description(&self) -> &str {
        &self.0
    }
}

impl Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl From<String> for Box<dyn Error + Send + Sync> {
    fn from(error_message: String) -> Self {
        Box::new(StringError(error_message))
    }
}

impl From<String> for Box<dyn Error> {
    fn from(error_message: String) -> Self {
        Box::new(StringError(error_message))
    }
}

impl<'a> From<&str> for Box<dyn Error+Send+Sync+'a> {
    fn from(err: &str) -> Self {
        From::from(String::from(err))
    }
}

impl From<&str> for Box<dyn Error> {
    fn from(err: &'_ str) -> Self {
        From::from(String::from(err))
    }
}

impl<'a, 'b> From<Cow<'b, str>> for Box<dyn Error + Send + Sync + 'a> {
    fn from(error_message: Cow<'b, str>) -> Self {
        From::from(String::from(error_message))
    }
}

impl<'a> From<Cow<'a, str>> for Box<dyn Error> {
    fn from(err: Cow<'a, str>) -> Box<dyn Error> {
        From::from(String::from(err))
    }
}

impl Error for core::array::TryFromSliceError {}
impl Error for core::cell::BorrowError {}
impl Error for core::cell::BorrowMutError {}
impl Error for core::char::CharTryFromError {}
impl Error for core::char::ParseCharError {}
impl Error for core::fmt::Error {}
impl Error for core::num::ParseFloatError {}
impl Error for core::num::ParseIntError {}
impl Error for core::str::ParseBoolError {}
impl Error for core::str::Utf8Error {}

impl Error for alloc::string::FromUtf8Error {}
impl Error for alloc::string::FromUtf16Error {}
impl Error for alloc::string::ParseError {}
