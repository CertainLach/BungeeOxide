use crate::{Error, Input, Output, Result};

pub const VARINT: u8 = 0;
pub const I64: u8 = 1;
pub const LEN: u8 = 2;
pub const I32: u8 = 5;

pub fn read_varint(input: &Input) -> Result<u64> {
	let mut out = 0u64;
	for i in 0..10 {
		let b = input.read()?;
		out |= u64::from(b & 0x7f) << (7 * i);
		if b & 0x80 == 0 {
			return Ok(out);
		}
	}
	Err(Error::TooLargeInteger)
}

pub fn write_varint(output: &mut Output, mut v: u64) {
	while v >= 0x80 {
		output.write(&[(v as u8) | 0x80]);
		v >>= 7;
	}
	output.write(&[v as u8]);
}

pub fn varint_size(v: u64) -> usize {
	(64 - (v | 1).leading_zeros() as usize).div_ceil(7)
}

pub fn read_key(input: &Input) -> Result<(u32, u8)> {
	let key = read_varint(input)?;
	let number = u32::try_from(key >> 3).map_err(|_| Error::InvalidValue)?;
	Ok((number, (key & 7) as u8))
}

pub fn write_key(output: &mut Output, number: u32, wire: u8) {
	write_varint(output, (u64::from(number) << 3) | u64::from(wire))
}

pub fn key_size(number: u32) -> usize {
	varint_size(u64::from(number) << 3)
}

pub fn expect(wire: u8, expected: u8) -> Result<()> {
	if wire == expected {
		Ok(())
	} else {
		Err(Error::InvalidValue)
	}
}

pub fn len<'i>(input: &Input<'i>) -> Result<Input<'i>> {
	let n = usize::try_from(read_varint(input)?).map_err(|_| Error::TooLargeInteger)?;
	input.take(n)
}

pub fn write_len(output: &mut Output, n: usize) {
	write_varint(output, n as u64)
}

pub fn len_size(n: usize) -> usize {
	varint_size(n as u64) + n
}

pub fn skip(input: &Input, wire: u8) -> Result<()> {
	match wire {
		VARINT => {
			read_varint(input)?;
		}
		I64 => {
			input.take(8)?;
		}
		LEN => {
			len(input)?;
		}
		I32 => {
			input.take(4)?;
		}
		_ => return Err(Error::InvalidValue),
	}
	Ok(())
}

pub mod natives {
	use super::*;

	macro_rules! varint_native {
		($name:ident, $ty:ty, $decode:expr, $encode:expr) => {
			pub mod $name {
				use super::*;
				pub type Ty = $ty;
				pub fn deserialize(input: &mut Input) -> Result<Ty> {
					let decode: fn(u64) -> Ty = $decode;
					Ok(decode(read_varint(input)?))
				}
				fn encode(v: Ty) -> u64 {
					let encode: fn(Ty) -> u64 = $encode;
					encode(v)
				}
				pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
					write_varint(output, encode(*_0));
					Ok(())
				}
				pub fn size_of(_0: &Ty) -> usize {
					varint_size(encode(*_0))
				}
			}
		};
	}

	varint_native!(pds_int32, i32, |v| v as i32, |v| v as i64 as u64);
	varint_native!(pds_int64, i64, |v| v as i64, |v| v as u64);
	varint_native!(pds_uint32, u32, |v| v as u32, u64::from);
	varint_native!(pds_uint64, u64, |v| v, |v| v);
	varint_native!(
		pds_sint32,
		i32,
		|v| ((v >> 1) as u32 as i32) ^ -((v & 1) as i32),
		|v| ((v << 1) ^ (v >> 31)) as u32 as u64
	);
	varint_native!(
		pds_sint64,
		i64,
		|v| ((v >> 1) as i64) ^ -((v & 1) as i64),
		|v| ((v << 1) ^ (v >> 63)) as u64
	);
	varint_native!(pds_bool, bool, |v| v != 0, |v| v as u64);

	macro_rules! fixed_native {
		($name:ident, $ty:ty) => {
			pub mod $name {
				use super::*;
				pub type Ty = $ty;
				pub fn deserialize(input: &mut Input) -> Result<Ty> {
					let mut buf = [0u8; std::mem::size_of::<$ty>()];
					for b in buf.iter_mut() {
						*b = input.read()?;
					}
					Ok(<$ty>::from_le_bytes(buf))
				}
				pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
					output.write(&_0.to_le_bytes());
					Ok(())
				}
				pub fn size_of(_0: &Ty) -> usize {
					std::mem::size_of::<$ty>()
				}
			}
		};
	}

	fixed_native!(pds_fixed32, u32);
	fixed_native!(pds_fixed64, u64);
	fixed_native!(pds_sfixed32, i32);
	fixed_native!(pds_sfixed64, i64);
	fixed_native!(pds_float, f32);
	fixed_native!(pds_double, f64);

	fn rest(input: &Input) -> Result<Vec<u8>> {
		let mut out = Vec::with_capacity(input.remaining());
		while input.remaining() > 0 {
			out.push(input.read()?);
		}
		Ok(out)
	}

	pub mod pds_string {
		use super::*;
		pub type Ty = String;
		pub fn deserialize(input: &mut Input) -> Result<Ty> {
			String::from_utf8(rest(input)?).map_err(|_| Error::InvalidValue)
		}
		pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
			output.write(_0.as_bytes());
			Ok(())
		}
		pub fn size_of(_0: &Ty) -> usize {
			_0.len()
		}
	}

	pub mod pds_bytes {
		use super::*;
		pub type Ty = Vec<u8>;
		pub fn deserialize(input: &mut Input) -> Result<Ty> {
			rest(input)
		}
		pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
			output.write(_0);
			Ok(())
		}
		pub fn size_of(_0: &Ty) -> usize {
			_0.len()
		}
	}

	pub mod view {
		pub mod pds_string {
			use crate::{Error, Input, Output, Result};
			pub type Ty<'i> = &'i str;
			pub fn deserialize<'i>(input: &mut Input<'i>) -> Result<Ty<'i>> {
				std::str::from_utf8(input.rest()).map_err(|_| Error::InvalidValue)
			}
			pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
				output.write(_0.as_bytes());
				Ok(())
			}
			pub fn size_of(_0: &Ty) -> usize {
				_0.len()
			}
			pub fn view(_0: &String) -> Ty<'_> {
				_0
			}
		}

		pub mod pds_bytes {
			use crate::{Input, Output, Result};
			pub type Ty<'i> = &'i [u8];
			pub fn deserialize<'i>(input: &mut Input<'i>) -> Result<Ty<'i>> {
				Ok(input.rest())
			}
			pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
				output.write(_0);
				Ok(())
			}
			pub fn size_of(_0: &Ty) -> usize {
				_0.len()
			}
			pub fn view(_0: &Vec<u8>) -> Ty<'_> {
				_0
			}
		}
	}
}
