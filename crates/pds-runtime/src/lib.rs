//! pds runtime: byte-level input/output the generated code is built against.

use std::cell::Cell;

pub mod convert;
pub mod natives;
pub mod protobuf;
pub mod stack;

pub struct Input<'i> {
	data: &'i [u8],
	offset: Cell<usize>,
	mask: Cell<Option<(usize, u8)>>,
}
impl<'i> Input<'i> {
	pub fn new(data: &'i [u8]) -> Self {
		Self {
			data,
			offset: Cell::new(0),
			mask: Cell::new(None),
		}
	}
	pub fn offset(&self) -> usize {
		self.offset.get()
	}
	pub fn remaining(&self) -> usize {
		self.data.len().saturating_sub(self.offset())
	}
	pub fn mask_next(&self, and: u8) {
		self.mask.set(Some((self.offset(), and)));
	}
	pub fn peek(&self) -> Result<u8> {
		let offset = self.offset();
		let byte = self.data.get(offset).copied().ok_or(Error::NotEnoughData)?;
		Ok(match self.mask.get() {
			Some((at, and)) if at == offset => byte & and,
			_ => byte,
		})
	}
	pub fn read(&self) -> Result<u8> {
		let value = self.peek()?;
		self.skip();
		Ok(value)
	}
	pub fn skip(&self) {
		self.offset.set(self.offset.get() + 1)
	}
	pub fn rewind(&self, offset: usize) {
		self.offset.set(offset)
	}
	pub fn take(&self, n: usize) -> Result<Input<'i>> {
		if n > self.remaining() {
			return Err(Error::NotEnoughData);
		}
		let start = self.offset();
		self.offset.set(start + n);
		Ok(Input::new(&self.data[start..start + n]))
	}
}

#[derive(Default)]
pub struct Output {
	out: Vec<u8>,
}
impl Output {
	pub fn new() -> Self {
		Self::default()
	}
	pub fn write(&mut self, v: &[u8]) {
		self.out.extend_from_slice(v);
	}
	pub fn into_inner(self) -> Vec<u8> {
		self.out
	}
}

#[derive(Debug)]
pub enum Error {
	NotEnoughData,
	TooLargeInteger,
	InvalidValue,
	TooDeep,
}
impl From<stack::TooDeep> for Error {
	fn from(_: stack::TooDeep) -> Self {
		Error::TooDeep
	}
}
pub type Result<T, E = Error> = std::result::Result<T, E>;
