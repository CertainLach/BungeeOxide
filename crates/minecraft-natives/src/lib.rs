#![allow(non_snake_case)]
use pds_bytes_runtime::{Error, Input, Output, Result};

pub use bungeeoxide_nbt::natives::*;

mod classic;
pub use classic::*;

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
		Ok(u128::from_be_bytes(input.exact()?))
	}
	pub fn size_of(_0: &Ty) -> usize {
		16
	}
}

pub mod pds_restBuffer {
	use super::*;
	pub type Ty = Vec<u8>;
	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		output.write(_0);
		Ok(())
	}
	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		Ok(input.rest().to_vec())
	}
	pub fn size_of(_0: &Ty) -> usize {
		_0.len()
	}
}

pub mod pds_string16 {
	use super::*;
	pub type Ty = String;
	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		let len = i16::try_from(_0.encode_utf16().count()).map_err(|_| Error::TooLargeInteger)?;
		output.write(&len.to_be_bytes());
		for unit in _0.encode_utf16() {
			output.write(&unit.to_be_bytes());
		}
		Ok(())
	}
	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		let len = i16::from_be_bytes(input.exact()?);
		let len = usize::try_from(len).map_err(|_| Error::InvalidValue)?;
		let units: Vec<u16> = input
			.bytes(len * 2)?
			.chunks_exact(2)
			.map(|c| u16::from_be_bytes([c[0], c[1]]))
			.collect();
		String::from_utf16(&units).map_err(|_| Error::InvalidValue)
	}
	pub fn size_of(_0: &Ty) -> usize {
		2 + 2 * _0.encode_utf16().count()
	}
}

pub mod pds_legacySlot {
	use super::*;
	#[derive(Debug, Clone, PartialEq)]
	pub struct Item {
		pub id: i16,
		pub itemCount: i8,
		pub itemDamage: i16,
		pub nbtData: Option<Vec<u8>>,
	}
	pub type Ty = Option<Item>;
	pub fn has_nbt(id: i16) -> bool {
		matches!(id, 256..=259 | 261 | 267..=279 | 283..=286 | 290..=294 | 298..=317 | 346 | 359)
	}
	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		let Some(item) = _0 else {
			output.write(&(-1i16).to_be_bytes());
			return Ok(());
		};
		if item.id < 0 || (item.nbtData.is_some() && !has_nbt(item.id)) {
			return Err(Error::InvalidValue);
		}
		output.write(&item.id.to_be_bytes());
		output.write(&item.itemCount.to_be_bytes());
		output.write(&item.itemDamage.to_be_bytes());
		if has_nbt(item.id) {
			pds_compressedNbt::serialize(&item.nbtData, output)?;
		}
		Ok(())
	}
	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		let id = i16::from_be_bytes(input.exact()?);
		if id < 0 {
			return Ok(None);
		}
		let itemCount = i8::from_be_bytes(input.exact()?);
		let itemDamage = i16::from_be_bytes(input.exact()?);
		let nbtData = if has_nbt(id) {
			pds_compressedNbt::deserialize(input)?
		} else {
			None
		};
		Ok(Some(Item {
			id,
			itemCount,
			itemDamage,
			nbtData,
		}))
	}
	pub fn size_of(_0: &Ty) -> usize {
		match _0 {
			None => 2,
			Some(item) if has_nbt(item.id) => 5 + pds_compressedNbt::size_of(&item.nbtData),
			Some(_) => 5,
		}
	}
}

pub mod pds_lpVec3 {
	use super::*;
	pub type Ty = Vec<u8>;
	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		output.write(_0);
		Ok(())
	}
	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		let first = input.read()?;
		let mut out = vec![first];
		if first == 0 {
			return Ok(out);
		}
		out.extend(input.exact::<5>()?);
		if first & 4 != 0 {
			let scale = pds_varint::deserialize(input)?;
			let mut o = Output::new();
			pds_varint::serialize(&scale, &mut o)?;
			out.extend(o.into_inner());
		}
		Ok(out)
	}
	pub fn size_of(_0: &Ty) -> usize {
		_0.len()
	}
}

pub mod view {
	use pds_bytes_runtime::{Input, Output, Result};

	pub use bungeeoxide_nbt::natives::view::*;

	pub mod pds_restBuffer {
		use super::*;
		pub type Ty<'i> = &'i [u8];
		pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
			output.write(_0);
			Ok(())
		}
		pub fn deserialize<'i>(input: &mut Input<'i>) -> Result<Ty<'i>> {
			Ok(input.rest())
		}
		pub fn size_of(_0: &Ty) -> usize {
			_0.len()
		}
		pub fn view(_0: &Vec<u8>) -> Ty<'_> {
			_0
		}
	}

	pub mod pds_lpVec3 {
		use super::*;
		pub type Ty<'i> = &'i [u8];
		pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
			output.write(_0);
			Ok(())
		}
		pub fn deserialize<'i>(input: &mut Input<'i>) -> Result<Ty<'i>> {
			let start = input.offset();
			let first = input.read()?;
			if first != 0 {
				input.bytes(5)?;
				if first & 4 != 0 {
					crate::pds_varint::deserialize(input)?;
				}
			}
			let end = input.offset();
			input.rewind(start);
			input.bytes(end - start)
		}
		pub fn size_of(_0: &Ty) -> usize {
			_0.len()
		}
		pub fn view(_0: &Vec<u8>) -> Ty<'_> {
			_0
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn encode<T>(value: &T, serialize: fn(&T, &mut Output) -> Result<()>) -> Vec<u8> {
		let mut out = Output::new();
		serialize(value, &mut out).unwrap();
		out.into_inner()
	}

	fn decode<T>(bytes: &[u8], deserialize: fn(&mut Input) -> Result<T>) -> Result<T> {
		let mut input = Input::new(bytes);
		let value = deserialize(&mut input)?;
		assert_eq!(input.remaining(), 0);
		Ok(value)
	}

	#[test]
	fn string16_round_trips_utf16_code_units() {
		use pds_string16::*;
		for (text, bytes) in [
			("", vec![0, 0]),
			("Hi", vec![0, 2, 0, b'H', 0, b'i']),
			("\u{a7}a", vec![0, 2, 0, 0xa7, 0, b'a']),
			("\u{1f600}", vec![0, 2, 0xd8, 0x3d, 0xde, 0x00]),
		] {
			let text = text.to_owned();
			assert_eq!(encode(&text, serialize), bytes);
			assert_eq!(size_of(&text), bytes.len());
			assert_eq!(decode(&bytes, deserialize).unwrap(), text);
		}
	}

	#[test]
	fn string16_rejects_bad_input() {
		use pds_string16::*;
		assert!(matches!(
			decode(&[0xff, 0xff], deserialize),
			Err(Error::InvalidValue)
		));
		assert!(matches!(
			decode(&[0, 1, 0xd8, 0x3d], deserialize),
			Err(Error::InvalidValue)
		));
		assert!(matches!(
			decode(&[0, 2, 0, b'a'], deserialize),
			Err(Error::NotEnoughData)
		));
		let mut out = Output::new();
		assert!(matches!(
			serialize(&"a".repeat(0x8000), &mut out),
			Err(Error::TooLargeInteger)
		));
	}

	#[test]
	fn legacy_slot_reads_nbt_only_for_damageable_items() {
		use pds_legacySlot::*;
		let cases: [(Ty, Vec<u8>); 4] = [
			(None, vec![0xff, 0xff]),
			(
				Some(Item {
					id: 1,
					itemCount: 64,
					itemDamage: 0,
					nbtData: None,
				}),
				vec![0, 1, 64, 0, 0],
			),
			(
				Some(Item {
					id: 276,
					itemCount: 1,
					itemDamage: 12,
					nbtData: None,
				}),
				vec![0x01, 0x14, 1, 0, 12, 0xff, 0xff],
			),
			(
				Some(Item {
					id: 359,
					itemCount: 1,
					itemDamage: 0,
					nbtData: Some(vec![0x1f, 0x8b, 0x08]),
				}),
				vec![0x01, 0x67, 1, 0, 0, 0, 3, 0x1f, 0x8b, 0x08],
			),
		];
		for (item, bytes) in cases {
			assert_eq!(encode(&item, serialize), bytes);
			assert_eq!(size_of(&item), bytes.len());
			assert_eq!(decode(&bytes, deserialize).unwrap(), item);
		}
		assert_eq!(decode(&[0xff, 0xfe], deserialize).unwrap(), None);
		assert!(has_nbt(261) && has_nbt(298) && has_nbt(317) && has_nbt(346));
		assert!(!has_nbt(260) && !has_nbt(280) && !has_nbt(318) && !has_nbt(2256));
	}

	#[test]
	fn legacy_slot_refuses_nbt_it_cannot_write() {
		use pds_legacySlot::*;
		let item = Some(Item {
			id: 264,
			itemCount: 1,
			itemDamage: 0,
			nbtData: Some(vec![1]),
		});
		assert!(matches!(
			serialize(&item, &mut Output::new()),
			Err(Error::InvalidValue)
		));
	}
}
