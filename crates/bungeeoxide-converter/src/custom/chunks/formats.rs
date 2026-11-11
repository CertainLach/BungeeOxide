use crate::gen::convert::Version;
use crate::version_range;
use std::io::{Read, Write};
use std::sync::OnceLock;

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use super::bits::{bits_for, long_count, pack, palette, put_longs, put_varint, unpack, Reader};
use super::model::{Biomes, Column, Light, Nibbles, Section, BLOCKS, CELLS, NIBBLES};
use crate::pds::convert::{unconvertible, Error, Result};
use crate::registry;

const MAX_STATE: u32 = 1 << 20;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Format {
	Legacy7,
	Raw8,
	Pal9,
	Flat13,
	Count14,
	Packed16,
	Paletted18,
}

pub(super) fn format(version: Version) -> Format {
	match version {
		Version::V1_7 => Format::Legacy7,
		Version::V1_8 => Format::Raw8,
		version_range!(V1_9..V1_13) => Format::Pal9,
		version_range!(V1_13..V1_14) => Format::Flat13,
		version_range!(V1_14..V1_16) => Format::Count14,
		version_range!(V1_16..V1_18) => Format::Packed16,
		_ => Format::Paletted18,
	}
}

fn cached<T: Send + Sync>(
	cells: &'static OnceLock<Vec<OnceLock<T>>>,
	version: Version,
	make: impl FnOnce() -> T,
) -> &'static T {
	cells.get_or_init(|| (0..Version::ALL.len()).map(|_| OnceLock::new()).collect())
		[version.index()]
	.get_or_init(make)
}

pub(super) fn global_bits(version: Version) -> u32 {
	static CACHE: OnceLock<Vec<OnceLock<u32>>> = OnceLock::new();
	*cached(&CACHE, version, || match format(version) {
		Format::Legacy7 | Format::Raw8 => 16,
		Format::Pal9 => 13,
		Format::Flat13 | Format::Count14 => 14,
		_ => registry::blocks(version).map_or(15, |b| bits_for(b.states() as usize)),
	})
}

pub(super) fn domain(version: Version) -> usize {
	1 << global_bits(version)
}

fn air(version: Version) -> &'static [u32] {
	static CACHE: OnceLock<Vec<OnceLock<Vec<u32>>>> = OnceLock::new();
	cached(&CACHE, version, || {
		let b = registry::blocks(version);
		let out: Vec<u32> = ["air", "cave_air", "void_air"]
			.iter()
			.filter_map(|n| b?.block(n).map(|x| x.default))
			.collect();
		if out.is_empty() {
			vec![0]
		} else {
			out
		}
	})
}

pub(super) fn non_air(version: Version, states: &[u32]) -> i16 {
	match format(version) {
		Format::Legacy7 | Format::Raw8 | Format::Pal9 => {
			states.iter().filter(|s| **s >= 16).count() as i16
		}
		_ => {
			let air = air(version);
			states.iter().filter(|s| !air.contains(s)).count() as i16
		}
	}
}

fn count_of(version: Version, section: &Section) -> i16 {
	section
		.count
		.unwrap_or_else(|| non_air(version, &section.states))
}

pub(super) fn biome_bits(version: Version) -> u32 {
	static CACHE: OnceLock<Vec<OnceLock<u32>>> = OnceLock::new();
	*cached(&CACHE, version, || {
		let n = registry::names(version, "biomes").map_or(64, |n| n.names.len());
		bits_for(n.max(2))
	})
}

fn check(state: u32) -> Result<u32> {
	match state < MAX_STATE {
		true => Ok(state),
		false => unconvertible(format!("block state {state} is out of range")),
	}
}

pub(super) fn deflate(data: &[u8]) -> Result<Vec<u8>> {
	let mut z = ZlibEncoder::new(Vec::with_capacity(data.len() / 4), Compression::default());
	z.write_all(data)
		.map_err(|e| Error::Encoding(e.to_string()))?;
	z.finish().map_err(|e| Error::Encoding(e.to_string()))
}

pub(super) fn inflate(data: &[u8]) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	ZlibDecoder::new(data)
		.read_to_end(&mut out)
		.map_err(|e| Error::Encoding(e.to_string()))?;
	Ok(out)
}

fn present(mask: u32) -> impl Iterator<Item = usize> {
	(0..16).filter(move |s| mask & 1 << s != 0)
}

fn nibble(a: &[u8], i: usize) -> u32 {
	((a[i >> 1] >> ((i & 1) * 4)) & 15) as u32
}

fn set_nibble(a: &mut [u8], i: usize, v: u32) {
	a[i >> 1] |= ((v & 15) as u8) << ((i & 1) * 4);
}

pub(super) struct Bitmask {
	pub(super) mask: u32,
	pub(super) add: u32,
}

pub(super) fn size_7(mask: u32, add: u32, sky: bool, full: bool) -> usize {
	let n = mask.count_ones() as usize;
	let m = (add & mask).count_ones() as usize;
	n * (BLOCKS + NIBBLES * if sky { 3 } else { 2 }) + m * NIBBLES + if full { 256 } else { 0 }
}

pub(super) fn size_8(mask: u32, sky: bool, full: bool) -> usize {
	let n = mask.count_ones() as usize;
	n * (BLOCKS * 2 + NIBBLES * if sky { 2 } else { 1 }) + if full { 256 } else { 0 }
}

pub(super) fn decode_7(col: &mut Column, data: &[u8], m: Bitmask, sky: bool) -> Result<()> {
	let n = m.mask.count_ones() as usize;
	if data.len() < size_7(m.mask, m.add, sky, col.full) {
		return unconvertible(format!(
			"1.7 chunk data is {} bytes, expected {}",
			data.len(),
			size_7(m.mask, m.add, sky, col.full)
		));
	}
	let (ids, rest) = data.split_at(n * BLOCKS);
	let (metas, rest) = rest.split_at(n * NIBBLES);
	let (blocks, rest) = rest.split_at(n * NIBBLES);
	let (skies, rest) = rest.split_at(if sky { n * NIBBLES } else { 0 });
	let mut adds = rest;
	let mut light = Light::new(16, sky);
	col.sections = vec![None; 16];
	for (k, s) in present(m.mask).enumerate() {
		let add = if m.add & 1 << s != 0 {
			let (a, r) = adds.split_at(NIBBLES);
			adds = r;
			Some(a)
		} else {
			None
		};
		let ids = &ids[k * BLOCKS..(k + 1) * BLOCKS];
		let metas = &metas[k * NIBBLES..(k + 1) * NIBBLES];
		let states = (0..BLOCKS)
			.map(|i| {
				let high = add.map_or(0, |a| nibble(a, i));
				(high << 8 | ids[i] as u32) << 4 | nibble(metas, i)
			})
			.collect();
		col.sections[s] = Some(Section::new(states, None));
		light.block[s + 1] = Nibbles::Data(blocks[k * NIBBLES..(k + 1) * NIBBLES].to_vec());
		if sky {
			light.sky[s + 1] = Nibbles::Data(skies[k * NIBBLES..(k + 1) * NIBBLES].to_vec());
		}
	}
	col.light = Some(light);
	if col.full {
		col.biomes = Biomes::Flat(adds[..256].iter().map(|b| *b as i32).collect());
	}
	Ok(())
}

pub(super) fn light_array(light: &Option<Light>, s: usize, sky: bool) -> Vec<u8> {
	let fill = if sky { 0xff } else { 0 };
	let arrays = light.as_ref().map(|l| if sky { &l.sky } else { &l.block });
	match arrays.and_then(|a| a.get(s + 1)) {
		Some(Nibbles::Data(d)) if d.len() == NIBBLES => d.clone(),
		Some(Nibbles::Empty) => vec![0; NIBBLES],
		_ => vec![fill; NIBBLES],
	}
}

pub(super) fn has_sky(col: &Column) -> bool {
	col.light.as_ref().is_none_or(|l| l.has_sky)
}

fn listed(col: &Column) -> Vec<(usize, &Section)> {
	col.sections
		.iter()
		.enumerate()
		.take(16)
		.filter_map(|(i, s)| Some((i, s.as_ref()?)))
		.collect()
}

fn flat_biomes(col: &Column) -> Vec<i32> {
	match &col.biomes {
		Biomes::Flat(b) if b.len() >= 256 => b[..256].to_vec(),
		_ => vec![0; 256],
	}
}

pub(super) fn encode_7(col: &Column) -> (Bitmask, Vec<u8>) {
	let list = listed(col);
	let sky = has_sky(col);
	let mut mask = 0;
	let mut add = 0;
	for (s, section) in &list {
		mask |= 1 << s;
		if section.states.iter().any(|v| *v >> 12 != 0) {
			add |= 1 << s;
		}
	}
	let mut out = Vec::with_capacity(size_7(mask, add, sky, col.full));
	for (_, section) in &list {
		out.extend(section.states.iter().map(|v| (v >> 4) as u8));
	}
	for (_, section) in &list {
		let mut metas = vec![0u8; NIBBLES];
		for (i, v) in section.states.iter().enumerate() {
			set_nibble(&mut metas, i, *v);
		}
		out.extend(metas);
	}
	for (s, _) in &list {
		out.extend(light_array(&col.light, *s, false));
	}
	if sky {
		for (s, _) in &list {
			out.extend(light_array(&col.light, *s, true));
		}
	}
	for (s, section) in &list {
		if add & 1 << s != 0 {
			let mut a = vec![0u8; NIBBLES];
			for (i, v) in section.states.iter().enumerate() {
				set_nibble(&mut a, i, v >> 12);
			}
			out.extend(a);
		}
	}
	if col.full {
		out.extend(flat_biomes(col).iter().map(|b| *b as u8));
	}
	(Bitmask { mask, add }, out)
}

pub(super) fn decode_8(col: &mut Column, data: &[u8], mask: u32, sky: bool) -> Result<()> {
	let n = mask.count_ones() as usize;
	if data.len() < size_8(mask, sky, col.full) {
		return unconvertible(format!(
			"1.8 chunk data is {} bytes, expected {}",
			data.len(),
			size_8(mask, sky, col.full)
		));
	}
	let (blocks, rest) = data.split_at(n * BLOCKS * 2);
	let (block_light, rest) = rest.split_at(n * NIBBLES);
	let (sky_light, rest) = rest.split_at(if sky { n * NIBBLES } else { 0 });
	let mut light = Light::new(16, sky);
	col.sections = vec![None; 16];
	for (k, s) in present(mask).enumerate() {
		let states = blocks[k * BLOCKS * 2..(k + 1) * BLOCKS * 2]
			.as_chunks::<2>()
			.0
			.iter()
			.map(|p| u16::from_le_bytes(*p) as u32)
			.collect();
		col.sections[s] = Some(Section::new(states, None));
		light.block[s + 1] = Nibbles::Data(block_light[k * NIBBLES..(k + 1) * NIBBLES].to_vec());
		if sky {
			light.sky[s + 1] = Nibbles::Data(sky_light[k * NIBBLES..(k + 1) * NIBBLES].to_vec());
		}
	}
	col.light = Some(light);
	if col.full {
		col.biomes = Biomes::Flat(rest[..256].iter().map(|b| *b as i32).collect());
	}
	Ok(())
}

pub(super) fn encode_8(col: &Column) -> (u32, Vec<u8>) {
	let list = listed(col);
	let sky = has_sky(col);
	let mask = list.iter().fold(0, |m, (s, _)| m | 1 << s);
	let mut out = Vec::with_capacity(size_8(mask, sky, col.full));
	for (_, section) in &list {
		for v in &section.states {
			out.extend_from_slice(&(*v as u16).to_le_bytes());
		}
	}
	for (s, _) in &list {
		out.extend(light_array(&col.light, *s, false));
	}
	if sky {
		for (s, _) in &list {
			out.extend(light_array(&col.light, *s, true));
		}
	}
	if col.full {
		out.extend(flat_biomes(col).iter().map(|b| *b as u8));
	}
	(mask, out)
}

fn read_spanning(r: &mut Reader, pal9: bool) -> Result<Vec<u32>> {
	let bits = r.u8()? as u32;
	let palette = if bits <= 8 || pal9 {
		let n = r.varint()?.max(0) as usize;
		let entries = (0..n)
			.map(|_| r.varint().map(|v| v as u32))
			.collect::<Result<Vec<_>>>()?;
		(bits <= 8).then_some(entries)
	} else {
		None
	};
	if bits == 0 || bits > 32 {
		return unconvertible(format!("chunk section with {bits} bits per block"));
	}
	let n = r.varint()?.max(0) as usize;
	if n * 64 < BLOCKS * bits as usize {
		return unconvertible(format!("chunk section has {n} longs for {bits} bits"));
	}
	let longs = r.longs(n)?;
	let values = unpack(&longs, bits, BLOCKS, true);
	match palette {
		Some(p) => values
			.into_iter()
			.map(|i| {
				p.get(i as usize)
					.copied()
					.map_or_else(|| unconvertible("palette index out of range"), check)
			})
			.collect(),
		None => values.into_iter().map(check).collect(),
	}
}

fn write_spanning(out: &mut Vec<u8>, states: &[u32], global: u32, pal9: bool) {
	match palette(states, 256) {
		Some(p) => {
			let bits = bits_for(p.values.len()).max(4);
			out.push(bits as u8);
			put_varint(out, p.values.len() as i32);
			for v in &p.values {
				put_varint(out, *v as i32);
			}
			put_longs(out, &pack(p.indices.into_iter(), bits, true), true);
		}
		None => {
			out.push(global as u8);
			if pal9 {
				put_varint(out, 0);
			}
			put_longs(out, &pack(states.iter().copied(), global, true), true);
		}
	}
}

fn decode_9_with(col: &mut Column, data: &[u8], mask: u32, sky: bool, pal9: bool) -> Result<()> {
	let mut r = Reader::new(data);
	let mut light = Light::new(16, sky);
	let mut sections = vec![None; 16];
	for s in present(mask) {
		sections[s] = Some(Section::new(read_spanning(&mut r, pal9)?, None));
		light.block[s + 1] = Nibbles::Data(r.bytes(NIBBLES)?.to_vec());
		if sky {
			light.sky[s + 1] = Nibbles::Data(r.bytes(NIBBLES)?.to_vec());
		}
	}
	let biomes = match (col.full, pal9) {
		(false, _) => Biomes::Absent,
		(true, true) => Biomes::Flat(r.bytes(256)?.iter().map(|b| *b as i32).collect()),
		(true, false) => Biomes::Flat((0..256).map(|_| r.i32()).collect::<Result<_>>()?),
	};
	if r.remaining() != 0 {
		return unconvertible(format!("{} bytes left after chunk data", r.remaining()));
	}
	col.sections = sections;
	col.light = Some(light);
	col.biomes = biomes;
	Ok(())
}

pub(super) fn decode_9(col: &mut Column, data: &[u8], mask: u32, pal9: bool) -> Result<()> {
	match decode_9_with(col, data, mask, true, pal9) {
		Ok(()) => Ok(()),
		Err(e) => decode_9_with(col, data, mask, false, pal9).map_err(|_| e),
	}
}

pub(super) fn encode_9(col: &Column, version: Version) -> (u32, Vec<u8>) {
	let pal9 = format(version) == Format::Pal9;
	let global = global_bits(version);
	let list = listed(col);
	let sky = has_sky(col);
	let mut mask = 0;
	let mut out = Vec::new();
	for (s, section) in &list {
		mask |= 1 << s;
		write_spanning(&mut out, &section.states, global, pal9);
		out.extend(light_array(&col.light, *s, false));
		if sky {
			out.extend(light_array(&col.light, *s, true));
		}
	}
	if col.full {
		for b in flat_biomes(col) {
			match pal9 {
				true => out.push(b as u8),
				false => out.extend_from_slice(&b.to_be_bytes()),
			}
		}
	}
	(mask, out)
}

pub(super) fn decode_14(
	col: &mut Column,
	data: &[u8],
	mask: u32,
	biomes_inside: bool,
) -> Result<()> {
	let mut r = Reader::new(data);
	let mut sections = vec![None; 16];
	for s in present(mask) {
		let count = r.i16()?;
		sections[s] = Some(Section::new(read_spanning(&mut r, false)?, Some(count)));
	}
	if biomes_inside && col.full {
		col.biomes = Biomes::Flat((0..256).map(|_| r.i32()).collect::<Result<_>>()?);
	}
	if r.remaining() != 0 {
		return unconvertible(format!("{} bytes left after chunk data", r.remaining()));
	}
	col.sections = sections;
	Ok(())
}

pub(super) fn encode_14(col: &Column, version: Version, biomes_inside: bool) -> (u32, Vec<u8>) {
	let mut mask = 0;
	let mut out = Vec::new();
	for (s, section) in listed(col) {
		mask |= 1 << s;
		out.extend_from_slice(&count_of(version, section).to_be_bytes());
		write_spanning(&mut out, &section.states, 14, false);
	}
	if biomes_inside && col.full {
		for b in flat_biomes(col) {
			out.extend_from_slice(&b.to_be_bytes());
		}
	}
	(mask, out)
}

pub(super) fn decode_16(
	col: &mut Column,
	data: &[u8],
	present: &[usize],
	total: usize,
) -> Result<()> {
	let mut r = Reader::new(data);
	let mut sections = vec![None; total];
	for s in present {
		let count = r.i16()?;
		let bits = r.u8()? as u32;
		let palette = match bits <= 8 {
			true => Some(
				(0..r.varint()?.max(0))
					.map(|_| r.varint().map(|v| v as u32))
					.collect::<Result<Vec<_>>>()?,
			),
			false => None,
		};
		let n = r.varint()?.max(0) as usize;
		let longs = r.longs(n)?;
		if bits == 0 || bits > 32 {
			return unconvertible(format!("chunk section with {bits} bits per block"));
		}
		let values = unpack(&longs, bits, BLOCKS, false);
		let states = match palette {
			Some(p) => values
				.into_iter()
				.map(|i| {
					p.get(i as usize)
						.copied()
						.map_or_else(|| unconvertible("palette index out of range"), check)
				})
				.collect::<Result<_>>()?,
			None => values.into_iter().map(check).collect::<Result<_>>()?,
		};
		if *s >= sections.len() {
			sections.resize(*s + 1, None);
		}
		sections[*s] = Some(Section::new(states, Some(count)));
	}
	if r.remaining() != 0 {
		return unconvertible(format!("{} bytes left after chunk data", r.remaining()));
	}
	col.sections = sections;
	Ok(())
}

fn write_packed(out: &mut Vec<u8>, states: &[u32], global: u32) {
	match palette(states, 256) {
		Some(p) => {
			let bits = bits_for(p.values.len()).max(4);
			out.push(bits as u8);
			put_varint(out, p.values.len() as i32);
			for v in &p.values {
				put_varint(out, *v as i32);
			}
			put_longs(out, &pack(p.indices.into_iter(), bits, false), true);
		}
		None => {
			out.push(global as u8);
			put_longs(out, &pack(states.iter().copied(), global, false), true);
		}
	}
}

pub(super) fn encode_16(col: &Column, version: Version) -> (Vec<usize>, Vec<u8>) {
	let global = global_bits(version);
	let mut present = Vec::new();
	let mut out = Vec::new();
	let limit = if version < Version::V1_17 {
		16
	} else {
		usize::MAX
	};
	for (s, section) in col.sections.iter().enumerate().take(limit) {
		let Some(section) = section else { continue };
		present.push(s);
		out.extend_from_slice(&count_of(version, section).to_be_bytes());
		write_packed(&mut out, &section.states, global);
	}
	(present, out)
}

pub(super) struct Layout {
	pub(super) lengths: bool,
	pub(super) fluids: bool,
}

pub(super) fn layout(version: Version) -> Layout {
	Layout {
		lengths: version <= Version::V1_21_4,
		fluids: version >= Version::V26_1,
	}
}

fn read_container(
	r: &mut Reader,
	entries: usize,
	blocks: bool,
	lengths: bool,
	seen: &mut Option<u32>,
) -> Result<Vec<u32>> {
	let bits = r.u8()? as u32;
	if bits == 0 {
		let v = check(r.varint()? as u32)?;
		if lengths {
			let n = r.varint()?.max(0) as usize;
			r.longs(n)?;
		}
		return Ok(vec![v; entries]);
	}
	let indirect = if blocks { 8 } else { 3 };
	let palette = match bits <= indirect {
		true => Some(
			(0..r.varint()?.max(0))
				.map(|_| r.varint().map(|v| v as u32))
				.collect::<Result<Vec<_>>>()?,
		),
		false => {
			*seen = Some(bits);
			None
		}
	};
	if bits > 32 {
		return unconvertible(format!("paletted container with {bits} bits"));
	}
	let mut storage = if blocks && palette.is_some() {
		bits.max(4)
	} else {
		bits
	};
	let n = if lengths {
		let n = r.varint()?.max(0) as usize;
		if n != long_count(entries, storage, false) && n == long_count(entries, bits, false) {
			storage = bits;
		}
		n
	} else {
		long_count(entries, storage, false)
	};
	let longs = r.longs(n)?;
	let values = unpack(&longs, storage, entries, false);
	match palette {
		Some(p) => values
			.into_iter()
			.map(|i| {
				p.get(i as usize)
					.copied()
					.map_or_else(|| unconvertible("palette index out of range"), check)
			})
			.collect(),
		None => values.into_iter().map(check).collect(),
	}
}

fn write_container(out: &mut Vec<u8>, values: &[u32], blocks: bool, global: u32, lengths: bool) {
	let indirect = if blocks { 8 } else { 3 };
	match palette(values, 1 << indirect) {
		Some(p) if p.values.len() == 1 => {
			out.push(0);
			put_varint(out, p.values[0] as i32);
			if lengths {
				put_varint(out, 0);
			}
		}
		Some(p) => {
			let bits = if blocks {
				bits_for(p.values.len()).max(4)
			} else {
				bits_for(p.values.len())
			};
			out.push(bits as u8);
			put_varint(out, p.values.len() as i32);
			for v in &p.values {
				put_varint(out, *v as i32);
			}
			put_longs(out, &pack(p.indices.into_iter(), bits, false), lengths);
		}
		None => {
			let global = global.max(indirect + 1);
			out.push(global as u8);
			put_longs(out, &pack(values.iter().copied(), global, false), lengths);
		}
	}
}

pub(crate) fn padded(version: Version) -> bool {
	(Version::V1_18..=Version::V1_19_4).contains(&version)
}

pub(crate) fn padding(
	version: Version,
	rest: &[u8],
	read: usize,
	single: usize,
	sections: Option<usize>,
) -> bool {
	padded(version)
		&& rest.iter().all(|b| *b == 0)
		&& sections.map_or(rest.len() == single, |n| read >= n)
}

pub(super) fn decode_18(
	col: &mut Column,
	data: &[u8],
	version: Version,
	known: Option<usize>,
) -> Result<()> {
	let l = layout(version);
	let mut r = Reader::new(data);
	let mut sections = Vec::new();
	let mut biomes = Vec::new();
	let mut seen = None;
	let mut single = 0;
	while !r.done() && !padding(version, r.rest(), sections.len(), single, known) {
		let count = r.i16()?;
		if l.fluids {
			r.i16()?;
		}
		single += usize::from(r.rest().first() == Some(&0));
		let states = read_container(&mut r, BLOCKS, true, l.lengths, &mut None)?;
		sections.push(Some(Section::new(states, Some(count))));
		biomes.extend(
			read_container(&mut r, CELLS, false, l.lengths, &mut seen)?
				.into_iter()
				.map(|b| b as i32),
		);
	}
	col.sections = sections;
	col.biomes = Biomes::Cells(biomes);
	if seen.is_some() {
		col.biome_bits = seen;
	}
	Ok(())
}

pub(super) fn encode_18(col: &Column, version: Version) -> Vec<u8> {
	let l = layout(version);
	let global = global_bits(version);
	let biome_global = col.biome_bits.unwrap_or_else(|| biome_bits(version));
	let cells = match &col.biomes {
		Biomes::Cells(c) => c.as_slice(),
		_ => &[],
	};
	let mut out = Vec::new();
	let air = Section::filled(0, Some(0));
	for (i, s) in col.sections.iter().enumerate() {
		let section = s.as_ref().unwrap_or(&air);
		let count = count_of(version, section);
		out.extend_from_slice(&count.to_be_bytes());
		if l.fluids {
			out.extend_from_slice(
				&crate::custom::fluid_count_26_1(count, &section.states).to_be_bytes(),
			);
		}
		write_container(&mut out, &section.states, true, global, l.lengths);
		let biomes: Vec<u32> = (0..CELLS)
			.map(|c| {
				cells
					.get(i * CELLS + c)
					.or(cells.last())
					.copied()
					.unwrap_or(0)
					.max(0) as u32
			})
			.collect();
		write_container(&mut out, &biomes, false, biome_global, l.lengths);
	}
	out
}
