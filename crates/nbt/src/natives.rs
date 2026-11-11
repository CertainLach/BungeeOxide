pub mod pds_nbt {
	pub use crate::gen::pds_Entry::*;
}

pub mod pds_optionalNbt {
	use crate::gen::pds_Entry;
	use crate::pds::{Input, Output, Result};

	pub type Ty = Option<pds_Entry::Ty>;

	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		match _0 {
			Some(tag) => pds_Entry::serialize(tag, output),
			None => {
				output.write(&[0]);
				Ok(())
			}
		}
	}

	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		if input.peek()? == 0 {
			input.read()?;
			return Ok(None);
		}
		Ok(Some(pds_Entry::deserialize(input)?))
	}

	pub fn size_of(_0: &Ty) -> usize {
		_0.as_ref().map_or(1, pds_Entry::size_of)
	}
}

pub mod pds_anonymousNbt {
	pub use crate::gen::pds_Tag::*;
}

pub mod pds_anonOptionalNbt {
	use crate::gen::pds_Tag;
	use crate::pds::{Input, Output, Result};

	pub type Ty = Option<pds_Tag::Ty>;

	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		match _0 {
			Some(tag) => pds_Tag::serialize(tag, output),
			None => {
				output.write(&[0]);
				Ok(())
			}
		}
	}

	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		if input.peek()? == 0 {
			input.read()?;
			return Ok(None);
		}
		Ok(Some(pds_Tag::deserialize(input)?))
	}

	pub fn size_of(_0: &Ty) -> usize {
		_0.as_ref().map_or(1, pds_Tag::size_of)
	}
}

pub mod pds_compressedNbt {
	use crate::pds::{Error, Input, Output, Result};

	pub type Ty = Option<Vec<u8>>;

	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		match _0 {
			Some(gzip) => {
				let len = i16::try_from(gzip.len()).map_err(|_| Error::TooLargeInteger)?;
				output.write(&len.to_be_bytes());
				output.write(gzip);
			}
			None => output.write(&(-1i16).to_be_bytes()),
		}
		Ok(())
	}

	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		let len = i16::from_be_bytes([input.read()?, input.read()?]);
		if len < 0 {
			return Ok(None);
		}
		(0..len)
			.map(|_| input.read())
			.collect::<Result<Vec<u8>>>()
			.map(Some)
	}

	pub fn size_of(_0: &Ty) -> usize {
		2 + _0.as_ref().map_or(0, Vec::len)
	}
}

pub mod view {
	macro_rules! optional {
		($name:ident, $inner:ident) => {
			pub mod $name {
				use crate::gen::view::$inner;
				use crate::pds::{Input, Output, Result};

				pub type Ty<'i> = Option<$inner::Ty<'i>>;

				pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
					match _0 {
						Some(tag) => $inner::serialize(tag, output),
						None => {
							output.write(&[0]);
							Ok(())
						}
					}
				}

				pub fn deserialize<'i>(input: &mut Input<'i>) -> Result<Ty<'i>> {
					if input.peek()? == 0 {
						input.read()?;
						return Ok(None);
					}
					Ok(Some($inner::deserialize(input)?))
				}

				pub fn size_of(_0: &Ty) -> usize {
					_0.as_ref().map_or(1, $inner::size_of)
				}

				pub fn view(_0: &super::super::$name::Ty) -> Ty<'_> {
					_0.as_ref().map($inner::view)
				}

				pub fn into_view(_0: &mut super::super::$name::Ty) -> Ty<'_> {
					_0.as_mut().map($inner::into_view)
				}
			}
		};
	}

	pub mod pds_nbt {
		pub use crate::gen::view::pds_Entry::*;
	}

	pub mod pds_anonymousNbt {
		pub use crate::gen::view::pds_Tag::*;
	}

	optional!(pds_optionalNbt, pds_Entry);
	optional!(pds_anonOptionalNbt, pds_Tag);
}

#[cfg(test)]
mod tests {
	use super::{pds_anonOptionalNbt, pds_compressedNbt, pds_optionalNbt};
	use crate::pds::{Input, Output};
	use crate::{Entry, Payload, Tag};

	fn round<T: PartialEq + std::fmt::Debug>(
		value: T,
		ser: fn(&T, &mut Output) -> crate::pds::Result<()>,
		de: fn(&mut Input) -> crate::pds::Result<T>,
		size: fn(&T) -> usize,
	) -> Vec<u8> {
		let mut out = Output::new();
		ser(&value, &mut out).unwrap();
		let bytes = out.into_inner();
		assert_eq!(bytes.len(), size(&value));
		let mut input = Input::new(&bytes);
		assert_eq!(de(&mut input).unwrap(), value);
		assert_eq!(input.remaining(), 0);
		bytes
	}

	#[test]
	fn optional_nbt_writes_an_end_tag_for_none() {
		let none = round(
			None,
			pds_optionalNbt::serialize,
			pds_optionalNbt::deserialize,
			pds_optionalNbt::size_of,
		);
		assert_eq!(none, [0]);
		let entry = Entry::new(
			"",
			Payload::from(vec![Entry::new("a", Payload::from(7i32))]),
		);
		round(
			Some(entry),
			pds_optionalNbt::serialize,
			pds_optionalNbt::deserialize,
			pds_optionalNbt::size_of,
		);
		let tag = Tag::from(Entry::new("", Payload::from("x")));
		round(
			Some(tag),
			pds_anonOptionalNbt::serialize,
			pds_anonOptionalNbt::deserialize,
			pds_anonOptionalNbt::size_of,
		);
	}

	#[test]
	fn compressed_nbt_uses_a_signed_length() {
		assert_eq!(
			round(
				None,
				pds_compressedNbt::serialize,
				pds_compressedNbt::deserialize,
				pds_compressedNbt::size_of
			),
			[0xff, 0xff]
		);
		let bytes = round(
			Some(vec![0x1f, 0x8b, 8]),
			pds_compressedNbt::serialize,
			pds_compressedNbt::deserialize,
			pds_compressedNbt::size_of,
		);
		assert_eq!(bytes, [0, 3, 0x1f, 0x8b, 8]);
	}
}
