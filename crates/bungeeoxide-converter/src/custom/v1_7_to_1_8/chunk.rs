use std::io::{Read, Write};

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use super::item::state_1_7;
use crate::gen::convert::{v1_7, v1_8};
use crate::pds::convert::{unconvertible, Error, Result};

const BLOCKS: usize = 4096;
const HALF: usize = 2048;
const BIOMES: usize = 256;

fn deflate(data: &[u8]) -> Result<Vec<u8>> {
	let mut z = ZlibEncoder::new(Vec::with_capacity(data.len() / 4), Compression::default());
	z.write_all(data)
		.map_err(|e| Error::Encoding(e.to_string()))?;
	z.finish().map_err(|e| Error::Encoding(e.to_string()))
}

fn inflate(data: &[u8]) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	ZlibDecoder::new(data)
		.read_to_end(&mut out)
		.map_err(|e| Error::Encoding(e.to_string()))?;
	Ok(out)
}

fn size_1_8(bitmap: u16, sky: bool, full: bool) -> usize {
	let n = bitmap.count_ones() as usize;
	n * (BLOCKS * 2 + HALF + if sky { HALF } else { 0 }) + if full { BIOMES } else { 0 }
}

fn size_1_7(bitmap: u16, add: u16, sky: bool, full: bool) -> usize {
	let n = bitmap.count_ones() as usize;
	let m = (add & bitmap).count_ones() as usize;
	n * (BLOCKS + 2 * HALF + if sky { HALF } else { 0 }) + m * HALF + if full { BIOMES } else { 0 }
}

fn empty(full: bool, data: &[u8], out: &mut Vec<u8>) {
	if full {
		out.extend_from_slice(data.get(..BIOMES).unwrap_or(&[0; BIOMES]));
	}
}

fn column_1_7(bitmap: u16, sky: bool, full: bool, data: &[u8], out: &mut Vec<u8>) -> Result<()> {
	if bitmap == 0 {
		empty(full, data, out);
		return Ok(());
	}
	let n = bitmap.count_ones() as usize;
	if data.len() < size_1_8(bitmap, sky, full) {
		return unconvertible(format!(
			"chunk data is {} bytes, expected {}",
			data.len(),
			size_1_8(bitmap, sky, full)
		));
	}
	let (blocks, rest) = data.split_at(n * BLOCKS * 2);
	let start = out.len();
	out.resize(start + n * (BLOCKS + HALF), 0);
	let (ids, metas) = out[start..].split_at_mut(n * BLOCKS);
	for (i, pair) in blocks.chunks_exact(2).enumerate() {
		let state = state_1_7(u16::from_le_bytes([pair[0], pair[1]]));
		ids[i] = (state >> 4) as u8;
		metas[i / 2] |= ((state & 15) as u8) << ((i & 1) * 4);
	}
	let light = n * HALF * if sky { 2 } else { 1 };
	out.extend_from_slice(&rest[..light]);
	if full {
		out.extend_from_slice(&rest[light..light + BIOMES]);
	}
	Ok(())
}

fn column_1_8(
	bitmap: u16,
	add: u16,
	sky: bool,
	full: bool,
	data: &[u8],
	out: &mut Vec<u8>,
) -> Result<()> {
	if bitmap == 0 {
		empty(full, data, out);
		return Ok(());
	}
	let n = bitmap.count_ones() as usize;
	if data.len() < size_1_7(bitmap, add, sky, full) {
		return unconvertible(format!(
			"chunk data is {} bytes, expected {}",
			data.len(),
			size_1_7(bitmap, add, sky, full)
		));
	}
	let (ids, rest) = data.split_at(n * BLOCKS);
	let (metas, rest) = rest.split_at(n * HALF);
	let light = n * HALF * if sky { 2 } else { 1 };
	let (lights, rest) = rest.split_at(light);
	let mut adds = rest;
	let mut msb = Vec::with_capacity(n);
	for s in (0..16).filter(|s| bitmap & 1 << s != 0) {
		if add & 1 << s != 0 {
			let (a, r) = adds.split_at(HALF);
			msb.push(Some(a));
			adds = r;
		} else {
			msb.push(None);
		}
	}
	out.reserve(n * BLOCKS * 2 + light + BIOMES);
	for (i, id) in ids.iter().enumerate() {
		let nibble = |arr: &[u8], i: usize| (arr[i / 2] >> ((i & 1) * 4)) as u16 & 15;
		let high = msb[i / BLOCKS].map_or(0, |a| nibble(a, i % BLOCKS));
		let state = (high << 8 | *id as u16) << 4 | nibble(metas, i);
		out.extend_from_slice(&state.to_le_bytes());
	}
	out.extend_from_slice(lights);
	if full {
		out.extend_from_slice(&adds[..BIOMES]);
	}
	Ok(())
}

pub(super) fn single_1_7(
	p: v1_8::play_to_client_packet_map_chunk::Ty,
) -> Result<v1_7::play_to_client_packet_map_chunk::Ty> {
	let sky = p.chunk_data.len() == size_1_8(p.bit_map, true, p.ground_up);
	let mut raw = Vec::with_capacity(p.chunk_data.len());
	column_1_7(p.bit_map, sky, p.ground_up, &p.chunk_data, &mut raw)?;
	Ok(v1_7::play_to_client_packet_map_chunk::Ty {
		x: p.x,
		z: p.z,
		ground_up: p.ground_up,
		bit_map: p.bit_map,
		add_bit_map: 0,
		compressed_chunk_data: deflate(&raw)?,
	})
}

pub(super) fn bulk_1_7(
	p: v1_8::play_to_client_packet_map_chunk_bulk::Ty,
) -> Result<v1_7::play_to_client_packet_map_chunk_bulk::Ty> {
	let mut raw = Vec::with_capacity(p.data.len());
	let mut at = 0;
	let mut meta = Vec::with_capacity(p.meta.len());
	for m in &p.meta {
		let size = size_1_8(m.bit_map, p.sky_light_sent, true);
		let Some(data) = p.data.get(at..at + size) else {
			return unconvertible("map_chunk_bulk data is shorter than its columns");
		};
		column_1_7(m.bit_map, p.sky_light_sent, true, data, &mut raw)?;
		at += size;
		meta.push(v1_7::play_to_client_packet_map_chunk_bulk_meta_items::Ty {
			x: m.x,
			z: m.z,
			bit_map: m.bit_map,
			add_bit_map: 0,
		});
	}
	Ok(v1_7::play_to_client_packet_map_chunk_bulk::Ty {
		sky_light_sent: p.sky_light_sent,
		compressed_chunk_data: deflate(&raw)?,
		meta,
	})
}

pub(super) fn single_1_8(
	p: v1_7::play_to_client_packet_map_chunk::Ty,
) -> Result<v1_8::play_to_client_packet_map_chunk::Ty> {
	let data = inflate(&p.compressed_chunk_data)?;
	let sky = data.len() == size_1_7(p.bit_map, p.add_bit_map, true, p.ground_up);
	let mut chunk_data = Vec::new();
	column_1_8(
		p.bit_map,
		p.add_bit_map,
		sky,
		p.ground_up,
		&data,
		&mut chunk_data,
	)?;
	Ok(v1_8::play_to_client_packet_map_chunk::Ty {
		x: p.x,
		z: p.z,
		ground_up: p.ground_up,
		bit_map: p.bit_map,
		chunk_data,
	})
}

pub(super) fn bulk_1_8(
	p: v1_7::play_to_client_packet_map_chunk_bulk::Ty,
) -> Result<v1_8::play_to_client_packet_map_chunk_bulk::Ty> {
	let raw = inflate(&p.compressed_chunk_data)?;
	let mut data = Vec::new();
	let mut at = 0;
	let mut meta = Vec::with_capacity(p.meta.len());
	for m in &p.meta {
		let size = size_1_7(m.bit_map, m.add_bit_map, p.sky_light_sent, true);
		let Some(column) = raw.get(at..at + size) else {
			return unconvertible("map_chunk_bulk data is shorter than its columns");
		};
		column_1_8(
			m.bit_map,
			m.add_bit_map,
			p.sky_light_sent,
			true,
			column,
			&mut data,
		)?;
		at += size;
		meta.push(v1_8::play_to_client_packet_map_chunk_bulk_meta_items::Ty {
			x: m.x,
			z: m.z,
			bit_map: m.bit_map,
		});
	}
	Ok(v1_8::play_to_client_packet_map_chunk_bulk::Ty {
		sky_light_sent: p.sky_light_sent,
		meta,
		data,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn column(bitmap: u16, state: impl Fn(usize, usize) -> u16) -> Vec<u8> {
		let n = bitmap.count_ones() as usize;
		let mut data = Vec::new();
		for s in 0..n {
			for i in 0..BLOCKS {
				data.extend_from_slice(&state(s, i).to_le_bytes());
			}
		}
		for s in 0..n {
			data.extend(std::iter::repeat(s as u8).take(HALF));
		}
		for s in 0..n {
			data.extend(std::iter::repeat(0xf0 | s as u8).take(HALF));
		}
		data.extend((0..BIOMES).map(|b| b as u8));
		data
	}

	#[test]
	fn chunk_round_trips_through_1_7() {
		let bitmap = 0b1000_0000_0010_0001u16;
		let state =
			|s: usize, i: usize| (((s * 7 + i * 13) % 60) as u16 + 2) << 4 | (i % 16) as u16;
		let data = column(bitmap, state);
		let p = v1_8::play_to_client_packet_map_chunk::Ty {
			x: 3,
			z: -7,
			ground_up: true,
			bit_map: bitmap,
			chunk_data: data.clone(),
		};
		let old = single_1_7(p.clone()).unwrap();
		assert_eq!(
			(old.x, old.z, old.ground_up, old.bit_map, old.add_bit_map),
			(3, -7, true, bitmap, 0)
		);
		let raw = inflate(&old.compressed_chunk_data).unwrap();
		assert_eq!(raw.len(), size_1_7(bitmap, 0, true, true));
		assert_eq!(raw[BLOCKS + 5], (state(1, 5) >> 4) as u8);
		assert_eq!(raw[3 * BLOCKS + 2] & 15, (state(0, 4) & 15) as u8);
		assert_eq!(raw[3 * BLOCKS + 2] >> 4, (state(0, 5) & 15) as u8);
		assert_eq!(single_1_8(old).unwrap(), p);
	}

	#[test]
	fn new_blocks_become_1_7_blocks() {
		let data = column(1, |_, i| if i == 0 { 169 << 4 } else { 1 << 4 | 2 });
		let bulk = v1_8::play_to_client_packet_map_chunk_bulk::Ty {
			sky_light_sent: true,
			meta: vec![
				v1_8::play_to_client_packet_map_chunk_bulk_meta_items::Ty {
					x: 0,
					z: 1,
					bit_map: 1,
				},
				v1_8::play_to_client_packet_map_chunk_bulk_meta_items::Ty {
					x: 0,
					z: 2,
					bit_map: 1,
				},
			],
			data: [data.clone(), data].concat(),
		};
		let old = bulk_1_7(bulk).unwrap();
		let raw = inflate(&old.compressed_chunk_data).unwrap();
		let size = size_1_7(1, 0, true, true);
		assert_eq!(raw.len(), 2 * size);
		assert_eq!((raw[0], raw[1], raw[size + 1]), (89, 1, 1));
		assert_eq!(raw[BLOCKS], 0);
		let back = bulk_1_8(old).unwrap();
		assert_eq!(back.meta.len(), 2);
		assert_eq!(u16::from_le_bytes([back.data[0], back.data[1]]), 89 << 4);
		assert_eq!(u16::from_le_bytes([back.data[2], back.data[3]]), 1 << 4);
	}

	#[test]
	fn unloads_tolerate_missing_biomes() {
		let p = v1_8::play_to_client_packet_map_chunk::Ty {
			x: 1,
			z: 2,
			ground_up: true,
			bit_map: 0,
			chunk_data: vec![],
		};
		let old = single_1_7(p).unwrap();
		assert_eq!(
			inflate(&old.compressed_chunk_data).unwrap(),
			vec![0; BIOMES]
		);
		assert_eq!(single_1_8(old).unwrap().chunk_data, vec![0; BIOMES]);
	}

	#[test]
	fn add_arrays_extend_block_ids() {
		let mut raw = vec![0u8; size_1_7(1, 1, false, false)];
		raw[0] = 0x34;
		raw[BLOCKS] = 0x05;
		raw[BLOCKS + 2 * HALF] = 0x02;
		let mut out = Vec::new();
		column_1_8(1, 1, false, false, &raw, &mut out).unwrap();
		assert_eq!(u16::from_le_bytes([out[0], out[1]]), 0x234 << 4 | 5);
	}
}
