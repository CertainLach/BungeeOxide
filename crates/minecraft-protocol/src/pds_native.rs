//! Hand-written primitive natives the generated code forwards to.

use crate::pds::{Error, Input, Output, Result};

pub use pds_runtime::natives::{
	pds_bool, pds_f32, pds_f64, pds_i16, pds_i32, pds_i64, pds_i8, pds_u16, pds_u32, pds_u64,
	pds_u8, pds_void,
};

/// Protobuf-style LEB128 varint, as used by the minecraft protocol (32 bit).
pub mod pds_varint {
	use super::*;
	pub type Ty = i32;
	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		let mut value = *_0 as u32;
		loop {
			let mut b = (value & 0x7f) as u8;
			value >>= 7;
			if value != 0 {
				b |= 0x80;
			}
			output.write(&[b]);
			if value == 0 {
				return Ok(());
			}
		}
	}
	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		let mut out = 0u32;
		let mut shift = 0;
		loop {
			if shift >= 35 {
				return Err(Error::TooLargeInteger);
			}
			let b = input.read()?;
			out |= ((b & 0x7f) as u32) << shift;
			shift += 7;
			if b & 0x80 == 0 {
				return Ok(out as i32);
			}
		}
	}
	pub fn size_of(_0: &Ty) -> usize {
		let mut size = 1;
		let mut value = (*_0 as u32) >> 7;
		while value != 0 {
			size += 1;
			value >>= 7;
		}
		size
	}
}

/// 64-bit LEB128 varint.
pub mod pds_varlong {
	use super::*;
	pub type Ty = i64;
	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		let mut value = *_0 as u64;
		loop {
			let mut b = (value & 0x7f) as u8;
			value >>= 7;
			if value != 0 {
				b |= 0x80;
			}
			output.write(&[b]);
			if value == 0 {
				return Ok(());
			}
		}
	}
	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		let mut out = 0u64;
		let mut shift = 0;
		loop {
			if shift >= 70 {
				return Err(Error::TooLargeInteger);
			}
			let b = input.read()?;
			out |= ((b & 0x7f) as u64) << shift;
			shift += 7;
			if b & 0x80 == 0 {
				return Ok(out as i64);
			}
		}
	}
	pub fn size_of(_0: &Ty) -> usize {
		let mut size = 1;
		let mut value = (*_0 as u64) >> 7;
		while value != 0 {
			size += 1;
			value >>= 7;
		}
		size
	}
}

pub mod pds_UUID {
	use super::*;
	pub type Ty = u128;
	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		output.write(&_0.to_be_bytes());
		Ok(())
	}
	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		let mut buf = [0u8; 16];
		for b in buf.iter_mut() {
			*b = input.read()?;
		}
		Ok(u128::from_be_bytes(buf))
	}
	pub fn size_of(_0: &Ty) -> usize {
		16
	}
}

pub mod pds_anonymousNbt {
	pub use nbt::gen::pds_Tag::*;
}

pub mod pds_anonOptionalNbt {
	pub use nbt::gen::pds_Tag::*;
}

pub mod pds_lpVec3 {
	use super::*;

	#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
	pub struct Ty {
		pub scale: u64,
		pub x: u16,
		pub y: u16,
		pub z: u16,
	}
	impl Ty {
		pub fn to_f64(&self) -> [f64; 3] {
			let unpack =
				|q: u16| (f64::from(q.min(32766)) * 2.0 / 32766.0 - 1.0) * self.scale as f64;
			[unpack(self.x), unpack(self.y), unpack(self.z)]
		}
	}

	fn extended(_0: &Ty) -> Result<Option<i32>> {
		if _0.scale <= 3 {
			return Ok(None);
		}
		let high = u32::try_from(_0.scale >> 2).map_err(|_| Error::InvalidValue)?;
		Ok(Some(high as i32))
	}

	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		if _0.scale == 0 {
			output.write(&[0]);
			return Ok(());
		}
		let high = extended(_0)?;
		let markers = (_0.scale & 3) | if high.is_some() { 4 } else { 0 };
		let packed = markers
			| (u64::from(_0.x & 0x7fff) << 3)
			| (u64::from(_0.y & 0x7fff) << 18)
			| (u64::from(_0.z & 0x7fff) << 33);
		output.write(&[packed as u8, (packed >> 8) as u8]);
		output.write(&((packed >> 16) as u32).to_be_bytes());
		if let Some(high) = high {
			pds_varint::serialize(&high, output)?;
		}
		Ok(())
	}
	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		let a = input.read()?;
		if a == 0 {
			return Ok(Ty::default());
		}
		let b = input.read()?;
		let mut c = 0u64;
		for _ in 0..4 {
			c = (c << 8) | u64::from(input.read()?);
		}
		let packed = (c << 16) | (u64::from(b) << 8) | u64::from(a);
		let mut scale = u64::from(a & 3);
		if a & 4 != 0 {
			scale |= u64::from(pds_varint::deserialize(input)? as u32) << 2;
		}
		Ok(Ty {
			scale,
			x: ((packed >> 3) & 0x7fff) as u16,
			y: ((packed >> 18) & 0x7fff) as u16,
			z: ((packed >> 33) & 0x7fff) as u16,
		})
	}
	pub fn size_of(_0: &Ty) -> usize {
		if _0.scale == 0 {
			return 1;
		}
		match extended(_0) {
			Ok(Some(high)) => 6 + pds_varint::size_of(&high),
			_ => 6,
		}
	}
}

/// All bytes until the end of the input.
pub mod pds_restBuffer {
	use super::*;
	pub type Ty = Vec<u8>;
	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		output.write(_0);
		Ok(())
	}
	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		let mut out = Vec::new();
		while let Ok(b) = input.read() {
			out.push(b);
		}
		Ok(out)
	}
	pub fn size_of(_0: &Ty) -> usize {
		_0.len()
	}
}
