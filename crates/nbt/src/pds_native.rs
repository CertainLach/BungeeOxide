pub use pds_runtime::natives::{
	pds_f32, pds_f64, pds_i16, pds_i32, pds_i64, pds_i8, pds_u8, pds_void,
};

pub mod pds_string {
	use crate::pds::{Error, Input, Output, Result};

	pub type Ty = String;

	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		write(_0, output)
	}

	pub fn write(s: &str, output: &mut Output) -> Result<()> {
		let bytes = encode(s);
		let len = u16::try_from(bytes.len()).map_err(|_| Error::InvalidValue)?;
		output.write(&len.to_be_bytes());
		output.write(&bytes);
		Ok(())
	}

	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		let len = u16::from_be_bytes(input.exact()?) as usize;
		decode(input.bytes(len)?)
	}

	pub fn size_of(_0: &Ty) -> usize {
		2 + encode(_0).len()
	}

	pub fn encode(s: &str) -> Vec<u8> {
		let mut out = Vec::with_capacity(s.len());
		for unit in s.encode_utf16() {
			match unit {
				0x0001..=0x007f => out.push(unit as u8),
				0x0000 | 0x0080..=0x07ff => {
					out.push(0xc0 | (unit >> 6) as u8);
					out.push(0x80 | (unit & 0x3f) as u8);
				}
				_ => {
					out.push(0xe0 | (unit >> 12) as u8);
					out.push(0x80 | ((unit >> 6) & 0x3f) as u8);
					out.push(0x80 | (unit & 0x3f) as u8);
				}
			}
		}
		out
	}

	pub fn decode(b: &[u8]) -> Result<String> {
		let cont = |i: usize| {
			b.get(i)
				.filter(|x| *x & 0xc0 == 0x80)
				.map(|x| (*x & 0x3f) as u16)
				.ok_or(Error::InvalidValue)
		};
		let mut units = Vec::with_capacity(b.len());
		let mut i = 0;
		while i < b.len() {
			let c = b[i] as u16;
			if c & 0x80 == 0 {
				units.push(c);
				i += 1;
			} else if c & 0xe0 == 0xc0 {
				units.push(((c & 0x1f) << 6) | cont(i + 1)?);
				i += 2;
			} else if c & 0xf0 == 0xe0 {
				units.push(((c & 0x0f) << 12) | (cont(i + 1)? << 6) | cont(i + 2)?);
				i += 3;
			} else {
				return Err(Error::InvalidValue);
			}
		}
		String::from_utf16(&units).map_err(|_| Error::InvalidValue)
	}
}

pub mod view {
	pub mod pds_string {
		use std::borrow::Cow;

		use crate::pds::{Input, Output, Result};
		use crate::pds_native::pds_string::{decode, encode, write};

		pub type Ty<'i> = Cow<'i, str>;

		pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
			write(_0, output)
		}

		pub fn deserialize<'i>(input: &mut Input<'i>) -> Result<Ty<'i>> {
			let len = u16::from_be_bytes(input.exact()?) as usize;
			let bytes = input.bytes(len)?;
			match std::str::from_utf8(bytes) {
				Ok(s) if bytes.iter().all(|b| *b < 0xf0) => Ok(Cow::Borrowed(s)),
				_ => decode(bytes).map(Cow::Owned),
			}
		}

		pub fn size_of(_0: &Ty) -> usize {
			2 + encode(_0).len()
		}

		pub fn view(_0: &String) -> Ty<'_> {
			Cow::Borrowed(_0)
		}
	}
}
