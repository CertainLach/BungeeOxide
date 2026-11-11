use super::items;
use crate::gen::convert::{v1_8, v1_9, Version};
use crate::pds::convert::{unconvertible, Result};
use crate::session::Session;

type C8 = v1_8::play_to_client_packet::Ty;

const SECTION_BLOCKS: usize = 4096;
const SECTION_LIGHT: usize = 2048;
const BIOMES: usize = 256;

struct Section8 {
	states: Vec<u16>,
	block_light: Vec<u8>,
	sky_light: Option<Vec<u8>>,
}

fn read_varint(data: &[u8], at: &mut usize) -> Option<u32> {
	let mut value = 0u32;
	for shift in (0..35).step_by(7) {
		let b = *data.get(*at)?;
		*at += 1;
		value |= u32::from(b & 0x7f) << shift;
		if b & 0x80 == 0 {
			return Some(value);
		}
	}
	None
}

fn sections_1_9(data: &[u8], count: usize, sky: bool) -> Option<(Vec<Section8>, usize)> {
	let mut at = 0;
	let mut out = Vec::with_capacity(count);
	for _ in 0..count {
		let bits = usize::from(*data.get(at)?);
		at += 1;
		if bits == 0 || bits > 32 {
			return None;
		}
		let palette_len = read_varint(data, &mut at)? as usize;
		let mut palette = Vec::with_capacity(palette_len.min(4096));
		for _ in 0..palette_len {
			palette.push(read_varint(data, &mut at)?);
		}
		let longs_len = read_varint(data, &mut at)? as usize;
		if longs_len * 64 < SECTION_BLOCKS * bits {
			return None;
		}
		let raw = data.get(at..at + longs_len * 8)?;
		at += longs_len * 8;
		let longs: Vec<u64> = raw
			.chunks_exact(8)
			.map(|c| u64::from_be_bytes(c.try_into().expect("8 bytes")))
			.collect();
		let mask = (1u64 << bits) - 1;
		let states = (0..SECTION_BLOCKS)
			.map(|i| {
				let bit = i * bits;
				let (word, off) = (bit / 64, bit % 64);
				let mut v = longs[word] >> off;
				if off + bits > 64 {
					v |= longs[word + 1] << (64 - off);
				}
				let v = (v & mask) as u32;
				let state = if palette.is_empty() {
					v
				} else {
					palette.get(v as usize).copied().unwrap_or(0)
				};
				items::packed_back(Version::V1_8, state as i32) as u16
			})
			.collect();
		let block_light = data.get(at..at + SECTION_LIGHT)?.to_vec();
		at += SECTION_LIGHT;
		let sky_light = if sky {
			let s = data.get(at..at + SECTION_LIGHT)?.to_vec();
			at += SECTION_LIGHT;
			Some(s)
		} else {
			None
		};
		out.push(Section8 {
			states,
			block_light,
			sky_light,
		});
	}
	Some((out, at))
}

fn column_1_8(
	x: i32,
	z: i32,
	full: bool,
	bitmap: u16,
	sections: &[Section8],
	biomes: Option<&[u8]>,
) -> C8 {
	let mut data =
		Vec::with_capacity(sections.len() * (SECTION_BLOCKS * 2 + SECTION_LIGHT * 2) + BIOMES);
	for s in sections {
		for state in &s.states {
			data.extend_from_slice(&state.to_le_bytes());
		}
	}
	for s in sections {
		data.extend_from_slice(&s.block_light);
	}
	for s in sections {
		if let Some(sky) = &s.sky_light {
			data.extend_from_slice(sky);
		}
	}
	if let Some(biomes) = biomes {
		data.extend_from_slice(biomes);
	}
	C8::MapChunk(v1_8::play_to_client_packet_map_chunk::Ty {
		x,
		z,
		ground_up: full,
		bit_map: bitmap,
		chunk_data: data,
	})
}

pub fn legacy_down_map_chunk_1_8(
	p: v1_9::play_to_client_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<C8>> {
	let bitmap = (p.bit_map & 0xffff) as u16;
	let count = bitmap.count_ones() as usize;
	let biomes_len = if p.ground_up { BIOMES } else { 0 };
	let parsed = [true, false].into_iter().find_map(|sky| {
		let (sections, used) = sections_1_9(&p.chunk_data, count, sky)?;
		(used + biomes_len == p.chunk_data.len()).then_some((sections, used, sky))
	});
	let Some((mut sections, used, sky)) = parsed else {
		return unconvertible(format!(
			"1.9 chunk {} {} with bitmap {bitmap:#x} has {} bytes that match no section layout",
			p.x,
			p.z,
			p.chunk_data.len()
		));
	};
	let biomes = p.ground_up.then(|| &p.chunk_data[used..]);
	if p.ground_up && bitmap == 0 {
		sections.push(Section8 {
			states: vec![0; SECTION_BLOCKS],
			block_light: vec![0; SECTION_LIGHT],
			sky_light: sky.then(|| vec![0xff; SECTION_LIGHT]),
		});
		return Ok(vec![column_1_8(p.x, p.z, true, 1, &sections, biomes)]);
	}
	Ok(vec![column_1_8(
		p.x,
		p.z,
		p.ground_up,
		bitmap,
		&sections,
		biomes,
	)])
}

pub fn legacy_down_unload_chunk_1_8(
	p: v1_9::play_to_client_packet_unload_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<C8>> {
	Ok(vec![column_1_8(
		p.chunk_x,
		p.chunk_z,
		true,
		0,
		&[],
		Some(&[0; BIOMES]),
	)])
}
