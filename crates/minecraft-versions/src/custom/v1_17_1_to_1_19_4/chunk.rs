use std::collections::HashMap;

use super::{ceil_log2, read_varint, take, unconvertible, write_varint, Result, V};
use crate::custom::chunks::padding;

pub(super) const BLOCKS: usize = 4096;
pub(super) const BIOMES: usize = 64;

pub(super) struct Section {
	pub count: i16,
	pub blocks: Vec<u32>,
	pub biomes: Vec<u32>,
	pub raw_biomes: Vec<u8>,
}

pub(super) fn global_bits(version: V) -> u32 {
	ceil_log2(crate::registry::blocks(version).map_or(1 << 15, |b| b.states() as usize))
}

fn read_longs(data: &[u8], at: &mut usize) -> Result<Vec<u64>> {
	let n = read_varint(data, at)? as usize;
	let bytes = take(data, at, n * 8)?;
	Ok(bytes
		.as_chunks::<8>()
		.0
		.iter()
		.map(|c| u64::from_be_bytes(*c))
		.collect())
}

fn unpack(longs: &[u64], bits: u32, size: usize) -> Result<Vec<u32>> {
	let per = (64 / bits) as usize;
	if longs.len() < size.div_ceil(per) {
		return unconvertible(format!(
			"paletted container has {} longs, {bits} bits need {}",
			longs.len(),
			size.div_ceil(per)
		));
	}
	let mask = (1u64 << bits) - 1;
	Ok((0..size)
		.map(|i| ((longs[i / per] >> ((i % per) as u32 * bits)) & mask) as u32)
		.collect())
}

fn pack(out: &mut Vec<u8>, values: impl ExactSizeIterator<Item = u32>, bits: u32) {
	let per = (64 / bits) as usize;
	let mut longs = vec![0u64; values.len().div_ceil(per)];
	for (i, v) in values.enumerate() {
		longs[i / per] |= (v as u64) << ((i % per) as u32 * bits);
	}
	write_varint(out, longs.len() as i32);
	for l in longs {
		out.extend_from_slice(&l.to_be_bytes());
	}
}

struct Kind {
	size: usize,
	min: u32,
	max: u32,
	single: bool,
}

const STATES: Kind = Kind {
	size: BLOCKS,
	min: 4,
	max: 8,
	single: true,
};
const STATES_1_17: Kind = Kind {
	size: BLOCKS,
	min: 4,
	max: 8,
	single: false,
};
const BIOME_CELLS: Kind = Kind {
	size: BIOMES,
	min: 1,
	max: 3,
	single: true,
};

fn read_container(data: &[u8], at: &mut usize, kind: &Kind) -> Result<Vec<u32>> {
	let bits = take(data, at, 1)?[0] as u32;
	if bits == 0 && kind.single {
		let v = read_varint(data, at)? as u32;
		read_longs(data, at)?;
		return Ok(vec![v; kind.size]);
	}
	if bits <= kind.max {
		let n = read_varint(data, at)? as usize;
		let palette = (0..n)
			.map(|_| read_varint(data, at).map(|v| v as u32))
			.collect::<Result<Vec<_>>>()?;
		let longs = read_longs(data, at)?;
		return unpack(&longs, bits.max(kind.min), kind.size)?
			.into_iter()
			.map(|i| match palette.get(i as usize) {
				Some(v) => Ok(*v),
				None => unconvertible(format!("palette index {i} out of {n}")),
			})
			.collect();
	}
	let longs = read_longs(data, at)?;
	unpack(&longs, bits, kind.size)
}

fn write_container(out: &mut Vec<u8>, values: &[u32], kind: &Kind, global: u32) {
	let mut palette = Vec::new();
	let mut index = HashMap::new();
	for v in values {
		index.entry(*v).or_insert_with(|| {
			palette.push(*v);
			palette.len() as u32 - 1
		});
	}
	let bits = ceil_log2(palette.len());
	if bits == 0 && kind.single {
		out.push(0);
		write_varint(out, palette[0] as i32);
		write_varint(out, 0);
		return;
	}
	let bits = bits.max(kind.min);
	if bits <= kind.max {
		out.push(bits as u8);
		write_varint(out, palette.len() as i32);
		for p in &palette {
			write_varint(out, *p as i32);
		}
		pack(out, values.iter().map(|v| index[v]), bits);
	} else {
		out.push(global as u8);
		pack(out, values.iter().copied(), global);
	}
}

pub(super) type Sections17 = Vec<Option<(i16, Vec<u32>)>>;

pub(super) fn read_1_17(data: &[u8], mask: &[i64], sections: usize) -> Result<Sections17> {
	let mut at = 0;
	(0..sections)
		.map(|s| {
			if mask.get(s / 64).is_none_or(|m| m >> (s % 64) & 1 == 0) {
				return Ok(None);
			}
			let count = i16::from_be_bytes(take(data, &mut at, 2)?.try_into().expect("2 bytes"));
			Ok(Some((count, read_container(data, &mut at, &STATES_1_17)?)))
		})
		.collect()
}

pub(super) fn write_1_17(sections: &[Section], global: u32) -> (Vec<i64>, Vec<u8>) {
	let mut mask = vec![0i64; sections.len().div_ceil(64)];
	let mut out = Vec::new();
	for (s, section) in sections.iter().enumerate() {
		if section.count == 0 {
			continue;
		}
		mask[s / 64] |= 1 << (s % 64);
		out.extend_from_slice(&section.count.to_be_bytes());
		write_container(&mut out, &section.blocks, &STATES_1_17, global);
	}
	while mask.last() == Some(&0) {
		mask.pop();
	}
	(mask, out)
}

fn read_section(data: &[u8], at: &mut usize) -> Result<Section> {
	let count = i16::from_be_bytes(take(data, at, 2)?.try_into().expect("2 bytes"));
	let blocks = read_container(data, at, &STATES)?;
	let start = *at;
	let biomes = read_container(data, at, &BIOME_CELLS)?;
	Ok(Section {
		count,
		blocks,
		biomes,
		raw_biomes: data[start..*at].to_vec(),
	})
}

pub(super) fn read_1_18(data: &[u8], version: V, sections: Option<usize>) -> Result<Vec<Section>> {
	let mut at = 0;
	let mut out = Vec::new();
	let mut single = 0;
	while at < data.len() && !padding(version, &data[at..], out.len(), single, sections) {
		single += usize::from(data.get(at + 2) == Some(&0));
		out.push(read_section(data, &mut at)?);
	}
	Ok(out)
}

pub(super) fn write_1_18(sections: &[Section], global: u32, biome_bits: u32) -> Vec<u8> {
	let mut out = Vec::new();
	for s in sections {
		out.extend_from_slice(&s.count.to_be_bytes());
		write_container(&mut out, &s.blocks, &STATES, global);
		match s.raw_biomes.is_empty() {
			true => write_container(&mut out, &s.biomes, &BIOME_CELLS, biome_bits),
			false => out.extend_from_slice(&s.raw_biomes),
		}
	}
	out
}

pub(super) fn remap_states(
	data: &[u8],
	sections: Option<usize>,
	from: V,
	to: V,
) -> Result<Vec<u8>> {
	let mut sections = read_1_18(data, from, sections)?;
	let table = crate::registry::state_table(from, to).unwrap_or_default();
	for s in &mut sections {
		for b in &mut s.blocks {
			*b = table.get(*b as usize).copied().unwrap_or(0);
		}
	}
	Ok(write_1_18(&sections, global_bits(to), 0))
}
