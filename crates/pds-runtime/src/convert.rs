use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
	Dropped,
	Unconvertible(String),
	Encoding(String),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::Dropped => write!(f, "the packet is dropped"),
			Error::Unconvertible(why) => write!(f, "can't convert: {why}"),
			Error::Encoding(why) => write!(f, "can't encode: {why}"),
		}
	}
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub fn unconvertible<T>(why: impl Into<String>) -> Result<T> {
	Err(Error::Unconvertible(why.into()))
}
