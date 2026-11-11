//! pds runtime: byte-level input/output the generated code is built against.

use std::cell::Cell;

pub mod convert;
pub mod natives;
pub mod protobuf;
pub mod stack;
pub mod view;

pub struct Input<'i> {
	data: &'i [u8],
	offset: Cell<usize>,
	mask: Cell<Option<(usize, u8)>>,
	needed: Cell<usize>,
}
impl<'i> Input<'i> {
	pub fn new(data: &'i [u8]) -> Self {
		Self {
			data,
			offset: Cell::new(0),
			mask: Cell::new(None),
			needed: Cell::new(0),
		}
	}
	pub fn offset(&self) -> usize {
		self.offset.get()
	}
	pub fn remaining(&self) -> usize {
		self.data.len().saturating_sub(self.offset())
	}
	pub fn needed(&self) -> usize {
		self.needed.get()
	}
	pub fn short(&self, n: usize) -> Error {
		self.needed.set(self.offset().saturating_add(n));
		Error::NotEnoughData
	}
	pub fn expect(&self, n: usize) -> Result<()> {
		if n > self.remaining() {
			return Err(self.short(n));
		}
		Ok(())
	}
	pub fn mask_next(&self, and: u8) {
		self.mask.set(Some((self.offset(), and)));
	}
	pub fn peek(&self) -> Result<u8> {
		let offset = self.offset();
		let byte = self
			.data
			.get(offset)
			.copied()
			.ok_or_else(|| self.short(1))?;
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
	pub fn exact<const N: usize>(&self) -> Result<[u8; N]> {
		let start = self.offset();
		let mut out: [u8; N] = self.bytes(N)?.try_into().expect("bytes returns N bytes");
		if let Some((at, and)) = self.mask.get() {
			if let Some(b) = at.checked_sub(start).and_then(|i| out.get_mut(i)) {
				*b &= and;
			}
		}
		Ok(out)
	}
	pub fn take(&self, n: usize) -> Result<Input<'i>> {
		Ok(Input::new(self.bytes(n)?))
	}
	pub fn bytes(&self, n: usize) -> Result<&'i [u8]> {
		self.expect(n)?;
		let start = self.offset();
		self.offset.set(start + n);
		Ok(&self.data[start..start + n])
	}
	pub fn rest(&self) -> &'i [u8] {
		let start = self.offset().min(self.data.len());
		self.offset.set(self.data.len());
		&self.data[start..]
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn shortfalls_record_how_many_bytes_the_input_needs() {
		let input = Input::new(&[1, 2, 3]);
		input.read().unwrap();
		assert!(matches!(input.bytes(10), Err(Error::NotEnoughData)));
		assert_eq!(input.needed(), 11);
		input.bytes(2).unwrap();
		assert!(matches!(input.peek(), Err(Error::NotEnoughData)));
		assert_eq!(input.needed(), 4);
		let input = Input::new(&[0; 8]);
		input.read().unwrap();
		input.expect(7).unwrap();
		assert!(matches!(input.expect(3 * 4), Err(Error::NotEnoughData)));
		assert_eq!(input.needed(), 13);
		assert_eq!(input.offset(), 1);
	}

	#[test]
	fn exact_reads_whole_arrays_and_keeps_the_mask() {
		let input = Input::new(&[0x81, 0x02, 0x03]);
		input.mask_next(0x7f);
		assert_eq!(input.exact::<2>().unwrap(), [0x01, 0x02]);
		assert!(matches!(input.exact::<2>(), Err(Error::NotEnoughData)));
		assert_eq!(input.needed(), 4);
		assert_eq!(input.exact::<1>().unwrap(), [0x03]);
	}
}
