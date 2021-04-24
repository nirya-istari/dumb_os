
mod buffered;
mod chain;
mod copy;
mod cursor;
mod error;
mod read;
mod seek;
mod take;
mod write;
mod empty;
mod stdio;

pub mod prelude;

const DEFAULT_BUF_CAPACITY: usize = 1024;

pub use buffered::*;
pub use chain::*;
pub use copy::*;
pub use cursor::*;
pub use error::*;
pub use read::*;
pub use seek::*;
pub use take::*;
pub use write::*;
pub use empty::*;
pub use stdio::*;

pub type Result<T> = core::result::Result<T, self::Error>;
