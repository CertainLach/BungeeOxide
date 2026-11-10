use crate::pds::convert::{Error, Result};
use crate::pds::{Input, Output};

pub(super) fn encoding(e: crate::pds::Error) -> Error {
	Error::Encoding(format!("{e:?}"))
}

pub(super) struct Reader<'a>(pub Input<'a>);

impl<'a> Reader<'a> {
	pub fn new(data: &'a [u8]) -> Self {
		Self(Input::new(data))
	}

	pub fn remaining(&self) -> usize {
		self.0.remaining()
	}

	pub fn u8(&mut self) -> Result<u8> {
		self.0.read().map_err(encoding)
	}

	pub fn i32(&mut self) -> Result<i32> {
		Ok(i32::from_be_bytes(self.array()?))
	}

	pub fn u64(&mut self) -> Result<u64> {
		Ok(u64::from_be_bytes(self.array()?))
	}

	pub fn varint(&mut self) -> Result<i32> {
		crate::pds_native::pds_varint::deserialize(&mut self.0).map_err(encoding)
	}

	pub fn bytes(&mut self, n: usize) -> Result<Vec<u8>> {
		(0..n).map(|_| self.u8()).collect()
	}

	pub fn string(&mut self) -> Result<String> {
		let n = self.varint()?;
		let bytes = self.bytes(
			usize::try_from(n).map_err(|_| Error::Encoding("negative string length".into()))?,
		)?;
		String::from_utf8(bytes).map_err(|e| Error::Encoding(e.to_string()))
	}

	pub fn rest(&mut self) -> Result<Vec<u8>> {
		self.bytes(self.remaining())
	}

	pub fn with<T>(&mut self, f: fn(&mut Input<'a>) -> crate::pds::Result<T>) -> Result<T> {
		f(&mut self.0).map_err(encoding)
	}

	fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
		let mut out = [0; N];
		for b in &mut out {
			*b = self.u8()?;
		}
		Ok(out)
	}
}

pub(super) fn put_varint(out: &mut Vec<u8>, v: i32) {
	let mut v = v as u32;
	loop {
		if v & !0x7f == 0 {
			out.push(v as u8);
			return;
		}
		out.push((v as u8 & 0x7f) | 0x80);
		v >>= 7;
	}
}

pub(super) fn put_string(out: &mut Vec<u8>, s: &str) {
	put_varint(out, s.len() as i32);
	out.extend_from_slice(s.as_bytes());
}

pub(super) fn put<T>(
	out: &mut Vec<u8>,
	v: &T,
	f: fn(&T, &mut Output) -> crate::pds::Result<()>,
) -> Result<()> {
	let mut o = Output::new();
	f(v, &mut o).map_err(encoding)?;
	out.extend(o.into_inner());
	Ok(())
}
