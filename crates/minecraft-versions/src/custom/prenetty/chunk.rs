use crate::gen::convert::Version;
use std::io::{Read, Write};

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use super::ids;
use crate::pds::convert::{unconvertible, Error, Result};

pub const BLOCKS: usize = 4096;
pub const HALF: usize = 2048;
pub const BIOMES: usize = 256;
const OLD_HEIGHT: usize = 128;
const OLD_COLUMN: usize = 16 * 16 * OLD_HEIGHT;

pub fn inflate(data: &[u8]) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	ZlibDecoder::new(data)
		.read_to_end(&mut out)
		.map_err(|e| Error::Encoding(format!("bad chunk data: {e}")))?;
	Ok(out)
}

pub fn deflate(data: &[u8]) -> Result<Vec<u8>> {
	let mut z = ZlibEncoder::new(
		Vec::with_capacity(data.len() / 4 + 64),
		Compression::default(),
	);
	z.write_all(data)
		.map_err(|e| Error::Encoding(e.to_string()))?;
	z.finish().map_err(|e| Error::Encoding(e.to_string()))
}

#[derive(Debug, Clone, Copy)]
pub struct Shape {
	pub bitmap: u16,
	pub add: u16,
	pub sky: bool,
	pub full: bool,
}

impl Shape {
	pub fn sections(&self) -> usize {
		self.bitmap.count_ones() as usize
	}

	pub fn len(&self) -> usize {
		let n = self.sections();
		let m = (self.add & self.bitmap).count_ones() as usize;
		n * (BLOCKS + HALF * if self.sky { 3 } else { 2 })
			+ m * HALF
			+ if self.full { BIOMES } else { 0 }
	}

	fn check(&self, data: &[u8]) -> Result<()> {
		if self.sections() == 0 {
			return Ok(());
		}
		if data.len() < self.len() {
			return unconvertible(format!(
				"chunk data is {} bytes, expected {}",
				data.len(),
				self.len()
			));
		}
		Ok(())
	}
}

fn nibble(arr: &[u8], i: usize) -> u8 {
	arr[i / 2] >> ((i & 1) * 4) & 15
}

fn set_nibble(arr: &mut [u8], i: usize, v: u8) {
	let shift = (i & 1) * 4;
	arr[i / 2] = arr[i / 2] & !(15 << shift) | (v & 15) << shift;
}

pub fn remap(data: &mut [u8], shape: Shape, to: Version) -> Result<()> {
	shape.check(data)?;
	let n = shape.sections();
	let needs: Vec<bool> = (0..256u16)
		.map(|id| ids::block(id, 0, to).0 != id)
		.collect();
	let (blocks, rest) = data.split_at_mut(n * BLOCKS);
	let metas = &mut rest[..n * HALF];
	for i in 0..blocks.len() {
		let id = blocks[i];
		if needs[id as usize] {
			let (nid, nmeta) = ids::block(id as u16, nibble(metas, i), to);
			blocks[i] = nid as u8;
			set_nibble(metas, i, nmeta);
		}
	}
	Ok(())
}

pub fn remap_compressed(data: &[u8], shape: Shape, to: Version) -> Result<Vec<u8>> {
	if !ids::block_changes(to) || shape.bitmap == 0 {
		return Ok(data.to_vec());
	}
	let mut raw = inflate(data)?;
	remap(&mut raw, shape, to)?;
	deflate(&raw)
}

pub fn with_sky(data: &[u8], shape: Shape) -> Result<Vec<u8>> {
	if shape.sky {
		return Ok(data.to_vec());
	}
	shape.check(data)?;
	let n = shape.sections();
	let light_end = n * (BLOCKS + 2 * HALF);
	let mut out = Vec::with_capacity(data.len() + n * HALF);
	out.extend_from_slice(&data[..light_end]);
	out.resize(light_end + n * HALF, 0);
	out.extend_from_slice(&data[light_end..shape.len()]);
	Ok(out)
}

pub fn split(
	data: &[u8],
	columns: &[(i32, i32, u16, u16)],
	sky: bool,
) -> Result<Vec<(i32, i32, u16, u16, Vec<u8>)>> {
	let mut at = 0;
	let mut out = Vec::with_capacity(columns.len());
	for &(x, z, bitmap, add) in columns {
		let shape = Shape {
			bitmap,
			add,
			sky,
			full: true,
		};
		let end = at + shape.len();
		if end > data.len() {
			return unconvertible(format!(
				"bulk chunk data ends at {}, column needs {end}",
				data.len()
			));
		}
		out.push((x, z, bitmap, add, data[at..end].to_vec()));
		at = end;
	}
	Ok(out)
}

pub fn empty_column(sky: bool) -> Vec<u8> {
	let shape = Shape {
		bitmap: 1,
		add: 0,
		sky,
		full: true,
	};
	let mut out = vec![0; shape.len()];
	if sky {
		let start = BLOCKS + 2 * HALF;
		out[start..start + HALF].fill(0xff);
	}
	let len = out.len();
	out[len - BIOMES..].fill(1);
	out
}

pub struct Old {
	pub x: i32,
	pub y: i16,
	pub z: i32,
	pub size_x: u8,
	pub size_y: u8,
	pub size_z: u8,
	pub data: Vec<u8>,
}

fn old_index(x: usize, y: usize, z: usize) -> usize {
	x << 11 | z << 7 | y
}

pub fn old_full(chunk_x: i32, chunk_z: i32, data: &[u8], shape: Shape, to: Version) -> Result<Old> {
	shape.check(data)?;
	let n = shape.sections();
	let mut blocks = vec![0u8; OLD_COLUMN];
	let mut metas = vec![0u8; OLD_COLUMN / 2];
	let mut block_light = vec![0u8; OLD_COLUMN / 2];
	let mut sky_light = vec![if shape.sky { 0xff } else { 0 }; OLD_COLUMN / 2];
	let (ids, rest) = data.split_at(n * BLOCKS);
	let (meta, rest) = rest.split_at(n * HALF);
	let (light, rest) = rest.split_at(n * HALF);
	let sky = if shape.sky {
		&rest[..n * HALF]
	} else {
		&[][..]
	};
	for (k, s) in (0..16).filter(|s| shape.bitmap & 1 << s != 0).enumerate() {
		if s * 16 >= OLD_HEIGHT {
			continue;
		}
		for i in 0..BLOCKS {
			let (x, z, y) = (i & 15, i >> 4 & 15, s * 16 + (i >> 8));
			let o = old_index(x, y, z);
			let j = k * BLOCKS + i;
			let (id, m) = ids::block(ids[j] as u16, nibble(meta, j), to);
			blocks[o] = id as u8;
			set_nibble(&mut metas, o, m);
			set_nibble(&mut block_light, o, nibble(light, j));
			if shape.sky {
				set_nibble(&mut sky_light, o, nibble(sky, j));
			}
		}
	}
	let mut raw = blocks;
	raw.extend_from_slice(&metas);
	raw.extend_from_slice(&block_light);
	raw.extend_from_slice(&sky_light);
	Ok(Old {
		x: chunk_x * 16,
		y: 0,
		z: chunk_z * 16,
		size_x: 15,
		size_y: (OLD_HEIGHT - 1) as u8,
		size_z: 15,
		data: deflate(&raw)?,
	})
}

pub fn old_sections(
	chunk_x: i32,
	chunk_z: i32,
	data: &[u8],
	shape: Shape,
	to: Version,
) -> Result<Vec<Old>> {
	shape.check(data)?;
	let n = shape.sections();
	let (ids, rest) = data.split_at(n * BLOCKS);
	let (meta, rest) = rest.split_at(n * HALF);
	let (light, rest) = rest.split_at(n * HALF);
	let sky = if shape.sky {
		&rest[..n * HALF]
	} else {
		&[][..]
	};
	let mut out = Vec::new();
	for (k, s) in (0..16).filter(|s| shape.bitmap & 1 << s != 0).enumerate() {
		if s * 16 >= OLD_HEIGHT {
			continue;
		}
		let mut blocks = vec![0u8; BLOCKS];
		let mut metas = vec![0u8; HALF];
		let mut block_light = vec![0u8; HALF];
		let mut sky_light = vec![0u8; HALF];
		for i in 0..BLOCKS {
			let (x, z, y) = (i & 15, i >> 4 & 15, i >> 8);
			let o = x << 8 | z << 4 | y;
			let j = k * BLOCKS + i;
			let (id, m) = ids::block(ids[j] as u16, nibble(meta, j), to);
			blocks[o] = id as u8;
			set_nibble(&mut metas, o, m);
			set_nibble(&mut block_light, o, nibble(light, j));
			if shape.sky {
				set_nibble(&mut sky_light, o, nibble(sky, j));
			}
		}
		let mut raw = blocks;
		raw.extend_from_slice(&metas);
		raw.extend_from_slice(&block_light);
		raw.extend_from_slice(&sky_light);
		out.push(Old {
			x: chunk_x * 16,
			y: (s * 16) as i16,
			z: chunk_z * 16,
			size_x: 15,
			size_y: 15,
			size_z: 15,
			data: deflate(&raw)?,
		});
	}
	Ok(out)
}

pub struct Cuboid {
	pub blocks: Vec<(i32, i32, i32, u8, u8)>,
	pub full: Option<(i32, i32, Vec<u8>, u16)>,
}

pub fn from_old(old: &Old, sky: bool) -> Result<Cuboid> {
	let (sx, sy, sz) = (
		old.size_x as usize + 1,
		old.size_y as usize + 1,
		old.size_z as usize + 1,
	);
	let volume = sx * sy * sz;
	let raw = inflate(&old.data)?;
	if raw.len() < volume * 5 / 2 {
		return unconvertible(format!(
			"1.1 chunk data is {} bytes, expected {}",
			raw.len(),
			volume * 5 / 2
		));
	}
	let (ids, rest) = raw.split_at(volume);
	let (metas, rest) = rest.split_at(volume / 2);
	let (block_light, sky_light) = rest.split_at(volume / 2);
	let full =
		old.x & 15 == 0 && old.z & 15 == 0 && old.y == 0 && (sx, sy, sz) == (16, OLD_HEIGHT, 16);
	if full {
		let mut bitmap = 0u16;
		for s in 0..OLD_HEIGHT / 16 {
			let any = (0..16).any(|x| {
				(0..16).any(|z| (s * 16..s * 16 + 16).any(|y| ids[old_index(x, y, z)] != 0))
			});
			if any {
				bitmap |= 1 << s;
			}
		}
		let n = bitmap.count_ones() as usize;
		let shape = Shape {
			bitmap,
			add: 0,
			sky,
			full: true,
		};
		let mut out = vec![0u8; shape.len()];
		let (new_ids, rest) = out.split_at_mut(n * BLOCKS);
		let (new_meta, rest) = rest.split_at_mut(n * HALF);
		let (new_light, rest) = rest.split_at_mut(n * HALF);
		let (new_sky, biomes) = rest.split_at_mut(if sky { n * HALF } else { 0 });
		for (k, s) in (0..16).filter(|s| bitmap & 1 << s != 0).enumerate() {
			for i in 0..BLOCKS {
				let o = old_index(i & 15, s * 16 + (i >> 8), i >> 4 & 15);
				let j = k * BLOCKS + i;
				new_ids[j] = ids[o];
				set_nibble(new_meta, j, nibble(metas, o));
				set_nibble(new_light, j, nibble(block_light, o));
				if sky {
					set_nibble(new_sky, j, nibble(sky_light, o));
				}
			}
		}
		biomes[..BIOMES].fill(1);
		return Ok(Cuboid {
			blocks: vec![],
			full: Some((old.x >> 4, old.z >> 4, out, bitmap)),
		});
	}
	let mut blocks = Vec::with_capacity(volume);
	let mut i = 0;
	for x in 0..sx {
		for z in 0..sz {
			for y in 0..sy {
				blocks.push((
					old.x + x as i32,
					old.y as i32 + y as i32,
					old.z + z as i32,
					ids[i],
					nibble(metas, i),
				));
				i += 1;
			}
		}
	}
	Ok(Cuboid { blocks, full: None })
}

#[cfg(test)]
mod tests {
	use super::*;

	fn column(bitmap: u16, sky: bool, id: impl Fn(usize, usize) -> u8) -> Vec<u8> {
		let shape = Shape {
			bitmap,
			add: 0,
			sky,
			full: true,
		};
		let n = shape.sections();
		let mut out = vec![0u8; shape.len()];
		for (k, s) in (0..16).filter(|s| bitmap & 1 << s != 0).enumerate() {
			for i in 0..BLOCKS {
				out[k * BLOCKS + i] = id(s, i);
				set_nibble(&mut out[n * BLOCKS..], k * BLOCKS + i, (i % 7) as u8);
			}
		}
		out
	}

	#[test]
	fn columns_survive_the_1_1_format_below_y_128() {
		let id = |s: usize, i: usize| ((s * 31 + i * 7) % 90) as u8;
		let data = column(0b0000_0000_1000_0101, true, id);
		let shape = Shape {
			bitmap: 0b0000_0000_1000_0101,
			add: 0,
			sky: true,
			full: true,
		};
		let old = old_full(2, -3, &data, shape, Version::V1_2_3).unwrap();
		assert_eq!((old.x, old.z, old.size_y), (32, -48, 127));
		let back = from_old(&old, true).unwrap();
		let (cx, cz, raw, bitmap) = back.full.unwrap();
		assert_eq!((cx, cz), (2, -3));
		assert_eq!(bitmap & 0b101, 0b101);
		let k = |s: u16| (bitmap & ((1 << s) - 1)).count_ones() as usize;
		for i in 0..BLOCKS {
			assert_eq!(raw[k(2) * BLOCKS + i], id(2, i));
		}
	}

	#[test]
	fn remap_replaces_new_blocks_in_place() {
		let mut data = column(1, false, |_, i| if i == 5 { 162 } else { 1 });
		let shape = Shape {
			bitmap: 1,
			add: 0,
			sky: false,
			full: true,
		};
		set_nibble(&mut data[BLOCKS..], 5, 13);
		remap(&mut data, shape, Version::V1_6_4).unwrap();
		assert_eq!((data[5], nibble(&data[BLOCKS..], 5)), (17, 12));
		assert_eq!(data[6], 1);
	}

	#[test]
	fn sky_light_is_inserted_after_block_light() {
		let shape = Shape {
			bitmap: 0b11,
			add: 0,
			sky: false,
			full: true,
		};
		let data: Vec<u8> = (0..shape.len()).map(|i| (i % 251) as u8).collect();
		let out = with_sky(&data, shape).unwrap();
		let sky = Shape { sky: true, ..shape };
		assert_eq!(out.len(), sky.len());
		let light_end = 2 * (BLOCKS + 2 * HALF);
		assert_eq!(&out[..light_end], &data[..light_end]);
		assert!(out[light_end..light_end + 2 * HALF].iter().all(|b| *b == 0));
		assert_eq!(&out[light_end + 2 * HALF..], &data[light_end..]);
	}
}
