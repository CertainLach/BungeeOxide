use crate::{Input, Output, Result};

macro_rules! int_native {
	($name:ident, $ty:ty) => {
		pub mod $name {
			use super::*;
			pub type Ty = $ty;
			pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
				output.write(&_0.to_be_bytes());
				Ok(())
			}
			pub fn deserialize(input: &mut Input) -> Result<Ty> {
				let mut buf = [0u8; std::mem::size_of::<$ty>()];
				for b in buf.iter_mut() {
					*b = input.read()?;
				}
				Ok(<$ty>::from_be_bytes(buf))
			}
			pub fn size_of(_0: &Ty) -> usize {
				std::mem::size_of::<$ty>()
			}
		}
	};
}

int_native!(pds_u8, u8);
int_native!(pds_u16, u16);
int_native!(pds_u32, u32);
int_native!(pds_u64, u64);
int_native!(pds_i8, i8);
int_native!(pds_i16, i16);
int_native!(pds_i32, i32);
int_native!(pds_i64, i64);
int_native!(pds_f32, f32);
int_native!(pds_f64, f64);

pub mod pds_bool {
	use super::*;
	pub type Ty = bool;
	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		output.write(&[*_0 as u8]);
		Ok(())
	}
	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		Ok(input.read()? != 0)
	}
	pub fn size_of(_0: &Ty) -> usize {
		1
	}
}

pub mod pds_void {
	use super::*;
	pub type Ty = ();
	pub fn serialize(_0: &Ty, _output: &mut Output) -> Result<()> {
		Ok(())
	}
	pub fn deserialize(_input: &mut Input) -> Result<Ty> {
		Ok(())
	}
	pub fn size_of(_0: &Ty) -> usize {
		0
	}
}
