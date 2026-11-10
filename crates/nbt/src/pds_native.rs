pub use pds_runtime::natives::{
	pds_f32, pds_f64, pds_i16, pds_i32, pds_i64, pds_i8, pds_u8, pds_void,
};

pub mod pds_string {
	use crate::pds::{Error, Input, Output, Result};

	pub type Ty = String;

	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		let bytes = encode(_0);
		let len = u16::try_from(bytes.len()).map_err(|_| Error::InvalidValue)?;
		output.write(&len.to_be_bytes());
		output.write(&bytes);
		Ok(())
	}

	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		let len = u16::from_be_bytes([input.read()?, input.read()?]) as usize;
		let mut bytes = Vec::with_capacity(len);
		for _ in 0..len {
			bytes.push(input.read()?);
		}
		decode(&bytes)
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
