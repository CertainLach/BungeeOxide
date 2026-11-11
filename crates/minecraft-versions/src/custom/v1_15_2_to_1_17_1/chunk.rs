use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use ::nbt::Entry;
use ::nbt::Payload as P;

use crate::gen::convert::Version;
use crate::pds::convert::{unconvertible, Result};
use crate::registry;

pub(super) const BLOCKS: usize = 4096;

pub(super) struct Reader<'a> {
	data: &'a [u8],
	at: usize,
}

impl<'a> Reader<'a> {
	pub(super) fn new(data: &'a [u8]) -> Self {
		Self { data, at: 0 }
	}

	pub(super) fn take(&mut self, n: usize) -> Result<&'a [u8]> {
		let Some(bytes) = self.data.get(self.at..self.at + n) else {
			return unconvertible("chunk data is truncated");
		};
		self.at += n;
		Ok(bytes)
	}

	pub(super) fn u8(&mut self) -> Result<u8> {
		Ok(self.take(1)?[0])
	}

	pub(super) fn i16(&mut self) -> Result<i16> {
		Ok(i16::from_be_bytes(
			self.take(2)?.try_into().expect("two bytes"),
		))
	}

	pub(super) fn i64(&mut self) -> Result<i64> {
		Ok(i64::from_be_bytes(
			self.take(8)?.try_into().expect("eight bytes"),
		))
	}

	pub(super) fn varint(&mut self) -> Result<i32> {
		let mut v = 0u32;
		for shift in (0..35).step_by(7) {
			let b = self.u8()?;
			v |= ((b & 0x7f) as u32) << shift;
			if b & 0x80 == 0 {
				return Ok(v as i32);
			}
		}
		unconvertible("varint in chunk data is too long")
	}

	pub(super) fn rest(&self) -> &'a [u8] {
		&self.data[self.at..]
	}
}

pub(super) fn write_varint(out: &mut Vec<u8>, v: i32) {
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

pub(super) fn bits_for(n: usize) -> u32 {
	if n <= 1 {
		0
	} else {
		usize::BITS - (n - 1).leading_zeros()
	}
}

pub(super) fn unpack(longs: &[i64], bits: u32, count: usize, spanning: bool) -> Vec<u32> {
	let bits = bits.max(1) as usize;
	let mask = (1u64 << bits) - 1;
	let word = |i: usize| longs.get(i).copied().unwrap_or(0) as u64;
	(0..count)
		.map(|i| {
			if spanning {
				let bit = i * bits;
				let (w, off) = (bit / 64, bit % 64);
				let mut v = word(w) >> off;
				if off + bits > 64 {
					v |= word(w + 1) << (64 - off);
				}
				(v & mask) as u32
			} else {
				let per = 64 / bits;
				((word(i / per) >> ((i % per) * bits)) & mask) as u32
			}
		})
		.collect()
}

pub(super) fn pack(values: &[u32], bits: u32, spanning: bool) -> Vec<i64> {
	let bits = bits.max(1) as usize;
	let mask = (1u64 << bits) - 1;
	if spanning {
		let mut out = vec![0u64; (values.len() * bits).div_ceil(64)];
		for (i, v) in values.iter().enumerate() {
			let v = *v as u64 & mask;
			let bit = i * bits;
			let (w, off) = (bit / 64, bit % 64);
			out[w] |= v << off;
			if off + bits > 64 {
				out[w + 1] |= v >> (64 - off);
			}
		}
		out.into_iter().map(|v| v as i64).collect()
	} else {
		let per = 64 / bits;
		let mut out = vec![0u64; values.len().div_ceil(per)];
		for (i, v) in values.iter().enumerate() {
			out[i / per] |= (*v as u64 & mask) << ((i % per) * bits);
		}
		out.into_iter().map(|v| v as i64).collect()
	}
}

#[derive(Clone, Copy)]
pub(super) struct Format {
	pub(super) version: Version,
	pub(super) spanning: bool,
}

pub(super) const F1_15_2: Format = Format {
	version: Version::V1_15_2,
	spanning: true,
};
pub(super) const F1_16: Format = Format {
	version: Version::V1_16,
	spanning: false,
};
pub(super) const F1_16_2: Format = Format {
	version: Version::V1_16_2,
	spanning: false,
};
pub(super) const F1_17: Format = Format {
	version: Version::V1_17,
	spanning: false,
};

struct Blocks {
	global_bits: u32,
	air: Vec<u32>,
}

fn blocks(version: Version) -> &'static Blocks {
	static CACHE: OnceLock<Mutex<HashMap<Version, &'static Blocks>>> = OnceLock::new();
	let mut map = CACHE
		.get_or_init(Default::default)
		.lock()
		.expect("block cache poisoned");
	map.entry(version).or_insert_with(|| {
		let b = registry::blocks(version);
		let air = ["air", "cave_air", "void_air"]
			.iter()
			.filter_map(|n| b.and_then(|b| b.block(n)).map(|x| x.default))
			.collect();
		Box::leak(Box::new(Blocks {
			global_bits: bits_for(b.map_or(1 << 15, |b| b.states() as usize)),
			air,
		}))
	})
}

pub(super) fn airs(version: Version) -> &'static [u32] {
	&blocks(version).air
}

pub(super) fn state_table(from: Version, to: Version) -> Option<&'static [u32]> {
	super::states(from, to)
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Section {
	pub(super) count: i16,
	pub(super) bits: u8,
	pub(super) palette: Option<Vec<u32>>,
	pub(super) data: Vec<i64>,
}

impl Section {
	pub(super) fn read(r: &mut Reader) -> Result<Self> {
		let count = r.i16()?;
		let bits = r.u8()?;
		let palette = if bits <= 8 {
			let n = r.varint()?.max(0) as usize;
			Some(
				(0..n)
					.map(|_| r.varint().map(|v| v as u32))
					.collect::<Result<Vec<_>>>()?,
			)
		} else {
			None
		};
		let n = r.varint()?.max(0) as usize;
		let data = (0..n).map(|_| r.i64()).collect::<Result<Vec<_>>>()?;
		Ok(Self {
			count,
			bits,
			palette,
			data,
		})
	}

	pub(super) fn write(&self, out: &mut Vec<u8>) {
		out.extend_from_slice(&self.count.to_be_bytes());
		out.push(self.bits);
		if let Some(p) = &self.palette {
			write_varint(out, p.len() as i32);
			for v in p {
				write_varint(out, *v as i32);
			}
		}
		write_varint(out, self.data.len() as i32);
		for l in &self.data {
			out.extend_from_slice(&l.to_be_bytes());
		}
	}

	pub(super) fn states(&self, spanning: bool) -> Vec<u32> {
		let values = unpack(&self.data, self.bits as u32, BLOCKS, spanning);
		match &self.palette {
			Some(p) => values
				.into_iter()
				.map(|i| p.get(i as usize).copied().unwrap_or(0))
				.collect(),
			None => values,
		}
	}

	pub(super) fn encode(states: &[u32], format: Format) -> Self {
		let info = blocks(format.version);
		let count = states.iter().filter(|s| !info.air.contains(s)).count() as i16;
		let mut palette = Vec::new();
		let mut index = HashMap::new();
		for s in states {
			if !index.contains_key(s) {
				index.insert(*s, palette.len() as u32);
				palette.push(*s);
				if palette.len() > 256 {
					break;
				}
			}
		}
		if palette.len() <= 256 {
			let bits = bits_for(palette.len()).max(4);
			let indices: Vec<u32> = states.iter().map(|s| index[s]).collect();
			Self {
				count,
				bits: bits as u8,
				palette: Some(palette),
				data: pack(&indices, bits, format.spanning),
			}
		} else {
			Self {
				count,
				bits: info.global_bits as u8,
				palette: None,
				data: pack(states, info.global_bits, format.spanning),
			}
		}
	}

	pub(super) fn convert(&self, from: Format, to: Format) -> Self {
		let table = state_table(from.version, to.version);
		if table.is_none() && from.spanning == to.spanning {
			return self.clone();
		}
		let air = blocks(to.version).air.first().copied().unwrap_or(0);
		let states: Vec<u32> = self
			.states(from.spanning)
			.into_iter()
			.map(|s| match table {
				Some(t) => t.get(s as usize).copied().unwrap_or(air),
				None => s,
			})
			.collect();
		Self::encode(&states, to)
	}
}

pub(super) fn read_sections(data: &[u8], present: usize) -> Result<(Vec<Section>, Vec<u8>)> {
	let mut r = Reader::new(data);
	let sections = (0..present)
		.map(|_| Section::read(&mut r))
		.collect::<Result<Vec<_>>>()?;
	Ok((sections, r.rest().to_vec()))
}

pub(super) fn convert_sections(
	data: &[u8],
	present: usize,
	from: Format,
	to: Format,
) -> Result<Vec<u8>> {
	if state_table(from.version, to.version).is_none() && from.spanning == to.spanning {
		return Ok(data.to_vec());
	}
	let (sections, rest) = read_sections(data, present)?;
	let mut out = Vec::with_capacity(data.len());
	for s in &sections {
		s.convert(from, to).write(&mut out);
	}
	out.extend_from_slice(&rest);
	Ok(out)
}

pub(super) fn heightmaps(
	nbt: &mut Entry,
	from: (u32, bool),
	to: (u32, bool),
	shift: i32,
	max: i32,
) {
	let P::Compound(maps) = &mut nbt.value else {
		return;
	};
	for e in maps.iter_mut() {
		if let P::LongArray(longs) = &e.value {
			let values: Vec<u32> = unpack(longs, from.0, 256, from.1)
				.into_iter()
				.map(|v| (v as i32 - shift).clamp(0, max) as u32)
				.collect();
			e.value = P::LongArray(pack(&values, to.0, to.1));
		}
	}
}

pub(super) fn read_light(r: &mut Reader, mask: u64) -> Result<Vec<(usize, Vec<u8>)>> {
	(0..64)
		.filter(|i| mask & (1 << i) != 0)
		.map(|i| {
			let n = r.varint()?.max(0) as usize;
			Ok((i, r.take(n)?.to_vec()))
		})
		.collect()
}
