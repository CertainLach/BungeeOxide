use crate::gen::convert::Version;
use crate::version_range;
use std::sync::OnceLock;

use bungeeoxide_nbt::{Entry, Payload, Tag};

use super::bits::{put_varint, Reader};
use super::formats::{
	decode_14, decode_16, decode_18, decode_7, decode_8, decode_9, deflate, encode_14, encode_16,
	encode_18, encode_7, encode_8, encode_9, inflate, size_7, size_8, Bitmask,
};
use super::model::{Biomes, BlockEntity, Column, HeightKind, Heightmap, Light, Nibbles, CELLS};
use crate::gen::convert::{
	inspect, v1_14, v1_15, v1_16, v1_16_2, v1_17, v1_18, v1_20, v1_20_2, v1_21_5, v1_21_6, v1_7,
	v1_8, v1_9, v1_9_4, Root,
};
use crate::pds::convert::{unconvertible, Error, Result};
use crate::pds::{Input, Output};

type C7 = v1_7::play_to_client_packet_map_chunk::Ty;
type B7 = v1_7::play_to_client_packet_map_chunk_bulk::Ty;
type C8 = v1_8::play_to_client_packet_map_chunk::Ty;
type B8 = v1_8::play_to_client_packet_map_chunk_bulk::Ty;
type C9 = v1_9::play_to_client_packet_map_chunk::Ty;
type C94 = v1_9_4::play_to_client_packet_map_chunk::Ty;
type C14 = v1_14::play_to_client_packet_map_chunk::Ty;
type C15 = v1_15::play_to_client_packet_map_chunk::Ty;
type C16 = v1_16::play_to_client_packet_map_chunk::Ty;
type C162 = v1_16_2::play_to_client_packet_map_chunk::Ty;
type C17 = v1_17::play_to_client_packet_map_chunk::Ty;
type C18 = v1_18::play_to_client_packet_map_chunk::Ty;
type C20 = v1_20::play_to_client_packet_map_chunk::Ty;
type C202 = v1_20_2::play_to_client_packet_map_chunk::Ty;
type C215 = v1_21_5::play_to_client_packet_map_chunk::Ty;
type C216 = v1_21_6::play_to_client_packet_map_chunk::Ty;
type L14 = v1_14::play_to_client_packet_update_light::Ty;
type L16 = v1_16::play_to_client_packet_update_light::Ty;
type L17 = v1_17::play_to_client_packet_update_light::Ty;
type L20 = v1_20::play_to_client_packet_update_light::Ty;
type U9 = v1_9::play_to_client_packet_unload_chunk::Ty;
type U202 = v1_20_2::play_to_client_packet_unload_chunk::Ty;
type E18 = v1_18::chunk_block_entity::Ty;
type E202 = v1_20_2::chunk_block_entity::Ty;

pub enum Packet {
	Chunk(Column),
	Bulk(Vec<Column>, bool),
	Light(i32, i32, Light),
	Unload(i32, i32),
}

fn encoding(e: crate::pds::Error) -> Error {
	Error::Encoding(format!("{e:?}"))
}

pub(super) fn body<T>(
	value: &T,
	ser: fn(&T, &mut Output) -> crate::pds::Result<()>,
) -> Result<Vec<u8>> {
	let mut o = Output::new();
	ser(value, &mut o).map_err(encoding)?;
	Ok(o.into_inner())
}

fn read<T>(data: &[u8], de: fn(&mut Input) -> crate::pds::Result<T>) -> Result<T> {
	let mut input = Input::new(data);
	let out = de(&mut input).map_err(encoding)?;
	match input.remaining() {
		0 => Ok(out),
		n => unconvertible(format!("{n} bytes left after a chunk packet")),
	}
}

fn framed(id: i32, body: Vec<u8>) -> Vec<u8> {
	let mut out = Vec::with_capacity(body.len() + 5);
	put_varint(&mut out, id);
	out.extend(body);
	out
}

#[derive(Default, Debug)]
pub struct Ids {
	pub chunk: Option<i32>,
	pub bulk: Option<i32>,
	pub light: Option<i32>,
	pub unload: Option<i32>,
}

fn find_id(version: Version, name: &str, body: &[u8]) -> Option<i32> {
	(0..256).find(|id| {
		let data = framed(*id, body.to_vec());
		inspect(Root::PlayToClient, version, &data).is_ok_and(|n| n == name)
	})
}

pub fn ids(version: Version) -> &'static Ids {
	static IDS: OnceLock<Vec<OnceLock<Ids>>> = OnceLock::new();
	IDS.get_or_init(|| (0..Version::ALL.len()).map(|_| OnceLock::new()).collect())[version.index()]
		.get_or_init(|| {
			if version < Version::V1_7 {
				return Ids::default();
			}
			let mut col = Column::new(0, 0, true, version);
			col.biomes = match version {
				version_range!(..V1_15) => Biomes::Flat(vec![0; 256]),
				_ => Biomes::Cells(vec![0; 1024]),
			};
			col.heightmaps = Some(Vec::new());
			let chunk = chunk_body(version, &col)
				.ok()
				.and_then(|b| find_id(version, "map_chunk", &b));
			let bulk = match version {
				version_range!(..V1_9) => bulk_body(version, std::slice::from_ref(&col), true)
					.ok()
					.and_then(|b| find_id(version, "map_chunk_bulk", &b)),
				_ => None,
			};
			let light = match version {
				version_range!(V1_14..) => light_body(version, 0, 0, &Light::new(16, true))
					.ok()
					.and_then(|b| find_id(version, "update_light", &b)),
				_ => None,
			};
			let unload = match version {
				version_range!(V1_9..) => unload_body(version, 0, 0)
					.ok()
					.and_then(|b| find_id(version, "unload_chunk", &b)),
				_ => None,
			};
			Ids {
				chunk,
				bulk,
				light,
				unload,
			}
		})
}

pub fn packet_id(data: &[u8]) -> Option<i32> {
	Reader::new(data).varint().ok()
}

pub(super) fn named_entities(list: Vec<Entry>) -> Vec<BlockEntity> {
	list.into_iter()
		.map(|e| {
			let at = |k: &str| {
				e.value
					.field(k)
					.and_then(Payload::as_i64)
					.map_or(0, |v| v as i32)
			};
			BlockEntity {
				x: at("x"),
				y: at("y"),
				z: at("z"),
				kind: None,
				name: e.name.clone(),
				nbt: Some(Tag::from(e)),
			}
		})
		.collect()
}

pub(super) fn named_list(list: &[BlockEntity]) -> Vec<Entry> {
	list.iter()
		.map(|b| {
			b.nbt
				.clone()
				.unwrap_or_else(|| Tag::new(Vec::new()))
				.named(b.name.clone())
		})
		.collect()
}

pub(super) fn entities_18(cx: i32, cz: i32, list: Vec<E18>) -> Vec<BlockEntity> {
	list.into_iter()
		.map(|b| BlockEntity {
			x: cx * 16 + b.anon.x as i32,
			y: b.y as i32,
			z: cz * 16 + b.anon.z as i32,
			kind: Some(b.r#type),
			name: b
				.nbt_data
				.as_ref()
				.map_or_else(String::new, |e| e.name.clone()),
			nbt: b.nbt_data.map(Tag::from),
		})
		.collect()
}

pub(super) fn list_18(list: &[BlockEntity]) -> Vec<E18> {
	list.iter()
		.map(|b| E18 {
			anon: v1_18::chunk_block_entity_anon::Ty {
				x: (b.x & 15) as u8,
				z: (b.z & 15) as u8,
			},
			y: b.y as i16,
			r#type: b.kind.unwrap_or(0),
			nbt_data: b.nbt.clone().map(|t| t.named(b.name.clone())),
		})
		.collect()
}

pub(super) fn entities_202(cx: i32, cz: i32, list: Vec<E202>) -> Vec<BlockEntity> {
	list.into_iter()
		.map(|b| BlockEntity {
			x: cx * 16 + b.anon.x as i32,
			y: b.y as i32,
			z: cz * 16 + b.anon.z as i32,
			kind: Some(b.r#type),
			name: String::new(),
			nbt: b.nbt_data,
		})
		.collect()
}

pub(super) fn list_202(list: &[BlockEntity]) -> Vec<E202> {
	list.iter()
		.map(|b| E202 {
			anon: v1_20_2::chunk_block_entity_anon::Ty {
				x: (b.x & 15) as u8,
				z: (b.z & 15) as u8,
			},
			y: b.y as i16,
			r#type: b.kind.unwrap_or(0),
			nbt_data: b.nbt.clone(),
		})
		.collect()
}

pub(super) fn heightmaps_named(value: Payload) -> Vec<Heightmap> {
	match value {
		Payload::Compound(c) => c
			.into_iter()
			.filter_map(|e| match e.value {
				Payload::LongArray(data) => Some(Heightmap {
					kind: HeightKind::Named(e.name),
					data,
				}),
				_ => None,
			})
			.collect(),
		_ => Vec::new(),
	}
}

pub(super) fn heightmap_name(kind: &HeightKind) -> Option<String> {
	match kind {
		HeightKind::Named(n) => Some(n.clone()),
		HeightKind::Id(i) => crate::custom::late_heightmap_name_1_21_4(*i).map(str::to_owned),
	}
}

pub(super) fn heightmap_id(kind: &HeightKind) -> Option<i32> {
	match kind {
		HeightKind::Named(n) => crate::custom::late_heightmap_type_1_21_5(n),
		HeightKind::Id(i) => Some(*i),
	}
}

fn compound_of(maps: &Option<Vec<Heightmap>>) -> Payload {
	Payload::Compound(
		maps.iter()
			.flatten()
			.filter_map(|h| {
				Some(Entry::new(
					heightmap_name(&h.kind)?,
					Payload::LongArray(h.data.clone()),
				))
			})
			.collect(),
	)
}

pub(super) fn heightmaps_entry(maps: &Option<Vec<Heightmap>>) -> Entry {
	Entry::new("", compound_of(maps))
}

fn heightmaps_tag(maps: &Option<Vec<Heightmap>>) -> Tag {
	Tag::new(compound_of(maps))
}

fn type_216(id: i32) -> v1_21_6::play_to_client_packet_map_chunk_heightmaps_items_type::Ty {
	use v1_21_6::play_to_client_packet_map_chunk_heightmaps_items_type::Ty as T;
	match id {
		0 => T::WorldSurfaceWg,
		1 => T::WorldSurface,
		2 => T::OceanFloorWg,
		3 => T::OceanFloor,
		4 => T::MotionBlocking,
		5 => T::MotionBlockingNoLeaves,
		n => T::Default(n),
	}
}

fn bit(mask: &[i64], i: usize) -> bool {
	mask.get(i / 64)
		.is_some_and(|w| (*w as u64 >> (i % 64)) & 1 != 0)
}

fn set_bit(mask: &mut Vec<i64>, i: usize) {
	if mask.len() <= i / 64 {
		mask.resize(i / 64 + 1, 0);
	}
	mask[i / 64] |= 1i64 << (i % 64);
}

fn mask_len(masks: &[&[i64]]) -> usize {
	masks
		.iter()
		.map(|m| {
			m.iter()
				.rposition(|w| *w != 0)
				.map_or(0, |w| w * 64 + 64 - (m[w] as u64).leading_zeros() as usize)
		})
		.max()
		.unwrap_or(0)
}

fn arrays(
	len: usize,
	mask: &[i64],
	empty: &[i64],
	mut data: impl Iterator<Item = Vec<u8>>,
) -> Vec<Nibbles> {
	(0..len)
		.map(|i| {
			if bit(mask, i) {
				data.next().map_or(Nibbles::Missing, Nibbles::Data)
			} else if bit(empty, i) {
				Nibbles::Empty
			} else {
				Nibbles::Missing
			}
		})
		.collect()
}

struct Masks {
	sky: Vec<i64>,
	block: Vec<i64>,
	empty_sky: Vec<i64>,
	empty_block: Vec<i64>,
	sky_data: Vec<Vec<u8>>,
	block_data: Vec<Vec<u8>>,
}

fn masks(light: &Light) -> Masks {
	let mut m = Masks {
		sky: vec![],
		block: vec![],
		empty_sky: vec![],
		empty_block: vec![],
		sky_data: vec![],
		block_data: vec![],
	};
	for (i, n) in light.sky.iter().enumerate() {
		match n {
			Nibbles::Data(d) => {
				set_bit(&mut m.sky, i);
				m.sky_data.push(d.clone());
			}
			Nibbles::Empty => set_bit(&mut m.empty_sky, i),
			Nibbles::Missing => {}
		}
	}
	for (i, n) in light.block.iter().enumerate() {
		match n {
			Nibbles::Data(d) => {
				set_bit(&mut m.block, i);
				m.block_data.push(d.clone());
			}
			Nibbles::Empty => set_bit(&mut m.empty_block, i),
			Nibbles::Missing => {}
		}
	}
	m
}

macro_rules! masks_of {
	($p:ident) => {
		Masks {
			sky: $p.sky_light_mask,
			block: $p.block_light_mask,
			empty_sky: $p.empty_sky_light_mask,
			empty_block: $p.empty_block_light_mask,
			sky_data: $p.sky_light,
			block_data: $p.block_light,
		}
	};
}

fn light_from_masks(sections: usize, m: Masks, trust: bool) -> Light {
	let len = mask_len(&[&m.sky, &m.block, &m.empty_sky, &m.empty_block]).max(sections + 2);
	let has_sky = m.sky.iter().chain(&m.empty_sky).any(|w| *w != 0);
	Light {
		sky: arrays(len, &m.sky, &m.empty_sky, m.sky_data.into_iter()),
		block: arrays(len, &m.block, &m.empty_block, m.block_data.into_iter()),
		has_sky,
		trust,
	}
}

fn light_from_rest(
	sky: i32,
	block: i32,
	empty_sky: i32,
	empty_block: i32,
	data: &[u8],
	trust: bool,
) -> Result<Light> {
	let mut r = Reader::new(data);
	let mut take = |mask: i32| -> Result<Vec<Vec<u8>>> {
		(0..32)
			.filter(|i| mask as u32 & 1 << i != 0)
			.map(|_| {
				let n = r.varint()?.max(0) as usize;
				Ok(r.bytes(n)?.to_vec())
			})
			.collect()
	};
	let sky_data = take(sky)?;
	let block_data = take(block)?;
	let w = |m: i32| vec![m as u32 as i64];
	let m = Masks {
		sky: w(sky),
		block: w(block),
		empty_sky: w(empty_sky),
		empty_block: w(empty_block),
		sky_data,
		block_data,
	};
	Ok(light_from_masks(16, m, trust))
}

fn rest_of(m: &Masks) -> (i32, i32, i32, i32, Vec<u8>) {
	let first = |v: &[i64]| v.first().copied().unwrap_or(0) as i32;
	let mut data = Vec::new();
	for a in m.sky_data.iter().chain(&m.block_data) {
		put_varint(&mut data, a.len() as i32);
		data.extend_from_slice(a);
	}
	(
		first(&m.sky),
		first(&m.block),
		first(&m.empty_sky),
		first(&m.empty_block),
		data,
	)
}

fn light_or_empty(col: &Column) -> Light {
	col.light.clone().unwrap_or_else(|| {
		let n = col.sections.len();
		let mut l = Light::new(n, true);
		l.sky.iter_mut().for_each(|s| *s = Nibbles::Empty);
		l.block.iter_mut().for_each(|s| *s = Nibbles::Empty);
		l
	})
}

fn mask_17(col: &Column) -> Vec<i64> {
	let mut mask = Vec::new();
	for (i, s) in col.sections.iter().enumerate() {
		if s.is_some() {
			set_bit(&mut mask, i);
		}
	}
	mask
}

fn cells_i32(col: &Column, n: usize) -> Vec<i32> {
	match &col.biomes {
		Biomes::Cells(c) => (0..n)
			.map(|i| c.get(i).or(c.last()).copied().unwrap_or(0))
			.collect(),
		_ => vec![0; n],
	}
}

fn cells_len(col: &Column) -> usize {
	match &col.biomes {
		Biomes::Cells(c) if !c.is_empty() => c.len(),
		_ => col.sections.len().max(16) * CELLS,
	}
}

pub fn chunk_body(version: Version, col: &Column) -> Result<Vec<u8>> {
	let (x, z) = (col.x, col.z);
	match version {
		Version::V1_7 => {
			let (m, raw) = encode_7(col);
			body(
				&C7 {
					x,
					z,
					ground_up: col.full,
					bit_map: m.mask as u16,
					add_bit_map: m.add as u16,
					compressed_chunk_data: deflate(&raw)?,
				},
				v1_7::play_to_client_packet_map_chunk::serialize,
			)
		}
		Version::V1_8 => {
			let (mask, chunk_data) = encode_8(col);
			body(
				&C8 {
					x,
					z,
					ground_up: col.full,
					bit_map: mask as u16,
					chunk_data,
				},
				v1_8::play_to_client_packet_map_chunk::serialize,
			)
		}
		version_range!(V1_9..V1_9_4) => {
			let (mask, chunk_data) = encode_9(col, version);
			body(
				&C9 {
					x,
					z,
					ground_up: col.full,
					bit_map: mask as i32,
					chunk_data,
				},
				v1_9::play_to_client_packet_map_chunk::serialize,
			)
		}
		version_range!(V1_9_4..V1_14) => {
			let (mask, chunk_data) = encode_9(col, version);
			let block_entities = named_list(&col.entities);
			body(
				&C94 {
					x,
					z,
					ground_up: col.full,
					bit_map: mask as i32,
					chunk_data,
					block_entities,
				},
				v1_9_4::play_to_client_packet_map_chunk::serialize,
			)
		}
		version_range!(V1_14..V1_15) => {
			let (mask, chunk_data) = encode_14(col, version, true);
			let p = C14 {
				x,
				z,
				ground_up: col.full,
				bit_map: mask as i32,
				heightmaps: heightmaps_entry(&col.heightmaps),
				chunk_data,
				block_entities: named_list(&col.entities),
			};
			body(&p, v1_14::play_to_client_packet_map_chunk::serialize)
		}
		version_range!(V1_15..V1_16) => {
			use v1_15::play_to_client_packet_map_chunk_biomes::Ty as B;
			let (mask, chunk_data) = encode_14(col, version, false);
			let biomes = if col.full {
				B::True(cells_i32(col, 1024))
			} else {
				B::False
			};
			let p = C15 {
				x,
				z,
				bit_map: mask as i32,
				heightmaps: heightmaps_entry(&col.heightmaps),
				biomes,
				chunk_data,
				block_entities: named_list(&col.entities),
			};
			body(&p, v1_15::play_to_client_packet_map_chunk::serialize)
		}
		version_range!(V1_16..V1_16_2) => {
			use v1_16::play_to_client_packet_map_chunk_biomes::Ty as B;
			let (present, chunk_data) = encode_16(col, version);
			let mask = present.iter().fold(0i32, |m, s| m | 1 << s);
			let biomes = if col.full {
				B::True(cells_i32(col, 1024))
			} else {
				B::False
			};
			let p = C16 {
				x,
				z,
				ignore_old_data: col.full,
				bit_map: mask,
				heightmaps: heightmaps_entry(&col.heightmaps),
				biomes,
				chunk_data,
				block_entities: named_list(&col.entities),
			};
			body(&p, v1_16::play_to_client_packet_map_chunk::serialize)
		}
		Version::V1_16_2 => {
			use v1_16_2::play_to_client_packet_map_chunk_biomes::Ty as B;
			let (present, chunk_data) = encode_16(col, version);
			let mask = present.iter().fold(0i32, |m, s| m | 1 << s);
			let biomes = if col.full {
				B::True(cells_i32(col, 1024))
			} else {
				B::False
			};
			let p = C162 {
				x,
				z,
				bit_map: mask,
				heightmaps: heightmaps_entry(&col.heightmaps),
				biomes,
				chunk_data,
				block_entities: named_list(&col.entities),
			};
			body(&p, v1_16_2::play_to_client_packet_map_chunk::serialize)
		}
		version_range!(V1_17..V1_18) => {
			let (_, chunk_data) = encode_16(col, version);
			let p = C17 {
				x,
				z,
				bit_map: mask_17(col),
				heightmaps: heightmaps_entry(&col.heightmaps),
				biomes: cells_i32(col, cells_len(col)),
				chunk_data,
				block_entities: named_list(&col.entities),
			};
			body(&p, v1_17::play_to_client_packet_map_chunk::serialize)
		}
		version_range!(V1_18..V1_20) => {
			let m = masks(&light_or_empty(col));
			let p = C18 {
				x,
				z,
				heightmaps: heightmaps_entry(&col.heightmaps),
				chunk_data: encode_18(col, version),
				block_entities: list_18(&col.entities),
				trust_edges: col.light.as_ref().is_some_and(|l| l.trust),
				sky_light_mask: m.sky,
				block_light_mask: m.block,
				empty_sky_light_mask: m.empty_sky,
				empty_block_light_mask: m.empty_block,
				sky_light: m.sky_data,
				block_light: m.block_data,
			};
			body(&p, v1_18::play_to_client_packet_map_chunk::serialize)
		}
		Version::V1_20 => {
			let m = masks(&light_or_empty(col));
			let p = C20 {
				x,
				z,
				heightmaps: heightmaps_entry(&col.heightmaps),
				chunk_data: encode_18(col, version),
				block_entities: list_18(&col.entities),
				sky_light_mask: m.sky,
				block_light_mask: m.block,
				empty_sky_light_mask: m.empty_sky,
				empty_block_light_mask: m.empty_block,
				sky_light: m.sky_data,
				block_light: m.block_data,
			};
			body(&p, v1_20::play_to_client_packet_map_chunk::serialize)
		}
		version_range!(V1_20_2..V1_21_5) => {
			let m = masks(&light_or_empty(col));
			let p = C202 {
				x,
				z,
				heightmaps: heightmaps_tag(&col.heightmaps),
				chunk_data: encode_18(col, version),
				block_entities: list_202(&col.entities),
				sky_light_mask: m.sky,
				block_light_mask: m.block,
				empty_sky_light_mask: m.empty_sky,
				empty_block_light_mask: m.empty_block,
				sky_light: m.sky_data,
				block_light: m.block_data,
			};
			body(&p, v1_20_2::play_to_client_packet_map_chunk::serialize)
		}
		Version::V1_21_5 => {
			let m = masks(&light_or_empty(col));
			let heightmaps = col
				.heightmaps
				.iter()
				.flatten()
				.filter_map(|h| {
					Some(
						v1_21_5::play_to_client_packet_map_chunk_heightmaps_items::Ty {
							r#type: heightmap_id(&h.kind)?,
							data: h.data.clone(),
						},
					)
				})
				.collect();
			let p = C215 {
				x,
				z,
				heightmaps,
				chunk_data: encode_18(col, version),
				block_entities: list_202(&col.entities),
				sky_light_mask: m.sky,
				block_light_mask: m.block,
				empty_sky_light_mask: m.empty_sky,
				empty_block_light_mask: m.empty_block,
				sky_light: m.sky_data,
				block_light: m.block_data,
			};
			body(&p, v1_21_5::play_to_client_packet_map_chunk::serialize)
		}
		_ => {
			let m = masks(&light_or_empty(col));
			let heightmaps = col
				.heightmaps
				.iter()
				.flatten()
				.filter_map(|h| {
					Some(
						v1_21_6::play_to_client_packet_map_chunk_heightmaps_items::Ty {
							r#type: type_216(heightmap_id(&h.kind)?),
							data: h.data.clone(),
						},
					)
				})
				.collect();
			let p = C216 {
				x,
				z,
				heightmaps,
				chunk_data: encode_18(col, version),
				block_entities: list_202(&col.entities),
				sky_light_mask: m.sky,
				block_light_mask: m.block,
				empty_sky_light_mask: m.empty_sky,
				empty_block_light_mask: m.empty_block,
				sky_light: m.sky_data,
				block_light: m.block_data,
			};
			body(&p, v1_21_6::play_to_client_packet_map_chunk::serialize)
		}
	}
}

pub fn bulk_body(version: Version, columns: &[Column], sky: bool) -> Result<Vec<u8>> {
	let mut columns = columns.to_vec();
	for c in &mut columns {
		c.full = true;
		if let Some(l) = &mut c.light {
			l.has_sky = sky;
		}
	}
	match version {
		Version::V1_7 => {
			let mut raw = Vec::new();
			let mut meta = Vec::new();
			for c in &columns {
				let (m, data) = encode_7(c);
				raw.extend(data);
				meta.push(v1_7::play_to_client_packet_map_chunk_bulk_meta_items::Ty {
					x: c.x,
					z: c.z,
					bit_map: m.mask as u16,
					add_bit_map: m.add as u16,
				});
			}
			body(
				&B7 {
					sky_light_sent: sky,
					compressed_chunk_data: deflate(&raw)?,
					meta,
				},
				v1_7::play_to_client_packet_map_chunk_bulk::serialize,
			)
		}
		_ => {
			let mut data = Vec::new();
			let mut meta = Vec::new();
			for c in &columns {
				let (mask, d) = encode_8(c);
				data.extend(d);
				meta.push(v1_8::play_to_client_packet_map_chunk_bulk_meta_items::Ty {
					x: c.x,
					z: c.z,
					bit_map: mask as u16,
				});
			}
			body(
				&B8 {
					sky_light_sent: sky,
					meta,
					data,
				},
				v1_8::play_to_client_packet_map_chunk_bulk::serialize,
			)
		}
	}
}

pub(super) fn light_14(x: i32, z: i32, light: &Light) -> L14 {
	let (sky, block, empty_sky, empty_block, data) = rest_of(&masks(light));
	L14 {
		chunk_x: x,
		chunk_z: z,
		sky_light_mask: sky,
		block_light_mask: block,
		empty_sky_light_mask: empty_sky,
		empty_block_light_mask: empty_block,
		data,
	}
}

pub(super) fn from_light_14(p: &L14) -> Result<Light> {
	light_from_rest(
		p.sky_light_mask,
		p.block_light_mask,
		p.empty_sky_light_mask,
		p.empty_block_light_mask,
		&p.data,
		false,
	)
}

pub(super) fn light_17(x: i32, z: i32, light: &Light) -> L17 {
	let m = masks(light);
	L17 {
		chunk_x: x,
		chunk_z: z,
		trust_edges: light.trust,
		sky_light_mask: m.sky,
		block_light_mask: m.block,
		empty_sky_light_mask: m.empty_sky,
		empty_block_light_mask: m.empty_block,
		sky_light: m.sky_data,
		block_light: m.block_data,
	}
}

pub(super) fn from_light_17(sections: usize, p: L17) -> Light {
	let trust = p.trust_edges;
	light_from_masks(sections, masks_of!(p), trust)
}

pub fn light_body(version: Version, x: i32, z: i32, light: &Light) -> Result<Vec<u8>> {
	let m = masks(light);
	match version {
		version_range!(V1_14..V1_16) => body(
			&light_14(x, z, light),
			v1_14::play_to_client_packet_update_light::serialize,
		),
		version_range!(V1_16..V1_17) => {
			let (sky, block, empty_sky, empty_block, data) = rest_of(&m);
			let p = L16 {
				chunk_x: x,
				chunk_z: z,
				trust_edges: light.trust,
				sky_light_mask: sky,
				block_light_mask: block,
				empty_sky_light_mask: empty_sky,
				empty_block_light_mask: empty_block,
				data,
			};
			body(&p, v1_16::play_to_client_packet_update_light::serialize)
		}
		version_range!(V1_17..V1_20) => body(
			&light_17(x, z, light),
			v1_17::play_to_client_packet_update_light::serialize,
		),
		_ => {
			let p = L20 {
				chunk_x: x,
				chunk_z: z,
				sky_light_mask: m.sky,
				block_light_mask: m.block,
				empty_sky_light_mask: m.empty_sky,
				empty_block_light_mask: m.empty_block,
				sky_light: m.sky_data,
				block_light: m.block_data,
			};
			body(&p, v1_20::play_to_client_packet_update_light::serialize)
		}
	}
}

pub fn unload_body(version: Version, x: i32, z: i32) -> Result<Vec<u8>> {
	match version {
		version_range!(..V1_20_2) => body(
			&U9 {
				chunk_x: x,
				chunk_z: z,
			},
			v1_9::play_to_client_packet_unload_chunk::serialize,
		),
		_ => body(
			&U202 {
				chunk_z: z,
				chunk_x: x,
			},
			v1_20_2::play_to_client_packet_unload_chunk::serialize,
		),
	}
}

fn id_of(version: Version, id: Option<i32>, what: &str) -> Result<i32> {
	id.ok_or_else(|| Error::Encoding(format!("no {what} packet in {version}")))
}

pub fn encode_chunk(version: Version, col: &Column) -> Result<Vec<Vec<u8>>> {
	let ids = ids(version);
	let mut out = Vec::with_capacity(2);
	if (Version::V1_14..Version::V1_18).contains(&version) {
		if let Some(light) = &col.light {
			out.push(framed(
				id_of(version, ids.light, "update_light")?,
				light_body(version, col.x, col.z, light)?,
			));
		}
	}
	out.push(framed(
		id_of(version, ids.chunk, "map_chunk")?,
		chunk_body(version, col)?,
	));
	Ok(out)
}

pub fn encode_bulk(version: Version, columns: &[Column], sky: bool) -> Result<Vec<u8>> {
	Ok(framed(
		id_of(version, ids(version).bulk, "map_chunk_bulk")?,
		bulk_body(version, columns, sky)?,
	))
}

pub fn encode_light(version: Version, x: i32, z: i32, light: &Light) -> Result<Vec<u8>> {
	Ok(framed(
		id_of(version, ids(version).light, "update_light")?,
		light_body(version, x, z, light)?,
	))
}

pub fn encode_unload(version: Version, x: i32, z: i32) -> Result<Vec<u8>> {
	Ok(framed(
		id_of(version, ids(version).unload, "unload_chunk")?,
		unload_body(version, x, z)?,
	))
}

fn decode_chunk_body(version: Version, data: &[u8], sections: Option<usize>) -> Result<Column> {
	match version {
		Version::V1_7 => {
			let p = read(data, v1_7::play_to_client_packet_map_chunk::deserialize)?;
			let raw = inflate(&p.compressed_chunk_data)?;
			let mut col = Column::new(p.x, p.z, p.ground_up, version);
			let (mask, add) = (p.bit_map as u32, p.add_bit_map as u32);
			let sky = raw.len() == size_7(mask, add, true, p.ground_up);
			decode_7(&mut col, &raw, Bitmask { mask, add }, sky)?;
			Ok(col)
		}
		Version::V1_8 => {
			let p = read(data, v1_8::play_to_client_packet_map_chunk::deserialize)?;
			let mut col = Column::new(p.x, p.z, p.ground_up, version);
			let mask = p.bit_map as u32;
			let sky = p.chunk_data.len() == size_8(mask, true, p.ground_up);
			decode_8(&mut col, &p.chunk_data, mask, sky)?;
			Ok(col)
		}
		version_range!(V1_9..V1_9_4) => {
			let p = read(data, v1_9::play_to_client_packet_map_chunk::deserialize)?;
			let mut col = Column::new(p.x, p.z, p.ground_up, version);
			decode_9(&mut col, &p.chunk_data, p.bit_map as u32, true)?;
			Ok(col)
		}
		version_range!(V1_9_4..V1_14) => {
			let p = read(data, v1_9_4::play_to_client_packet_map_chunk::deserialize)?;
			let mut col = Column::new(p.x, p.z, p.ground_up, version);
			decode_9(
				&mut col,
				&p.chunk_data,
				p.bit_map as u32,
				version < Version::V1_13,
			)?;
			col.entities = named_entities(p.block_entities);
			Ok(col)
		}
		version_range!(V1_14..V1_15) => {
			let p = read(data, v1_14::play_to_client_packet_map_chunk::deserialize)?;
			let mut col = Column::new(p.x, p.z, p.ground_up, version);
			decode_14(&mut col, &p.chunk_data, p.bit_map as u32, true)?;
			col.heightmaps = Some(heightmaps_named(p.heightmaps.value));
			col.entities = named_entities(p.block_entities);
			Ok(col)
		}
		version_range!(V1_15..V1_16) => {
			use v1_15::play_to_client_packet_map_chunk_biomes::Ty as B;
			let p = read(data, v1_15::play_to_client_packet_map_chunk::deserialize)?;
			let mut col = Column::new(p.x, p.z, matches!(p.biomes, B::True(_)), version);
			decode_14(&mut col, &p.chunk_data, p.bit_map as u32, false)?;
			if let B::True(b) = p.biomes {
				col.biomes = Biomes::Cells(b);
			}
			col.heightmaps = Some(heightmaps_named(p.heightmaps.value));
			col.entities = named_entities(p.block_entities);
			Ok(col)
		}
		version_range!(V1_16..V1_16_2) => {
			use v1_16::play_to_client_packet_map_chunk_biomes::Ty as B;
			let p = read(data, v1_16::play_to_client_packet_map_chunk::deserialize)?;
			let mut col = Column::new(p.x, p.z, matches!(p.biomes, B::True(_)), version);
			let present: Vec<usize> = (0..16).filter(|s| p.bit_map & 1 << s != 0).collect();
			decode_16(&mut col, &p.chunk_data, &present, 16)?;
			if let B::True(b) = p.biomes {
				col.biomes = Biomes::Cells(b);
			}
			col.heightmaps = Some(heightmaps_named(p.heightmaps.value));
			col.entities = named_entities(p.block_entities);
			Ok(col)
		}
		Version::V1_16_2 => {
			use v1_16_2::play_to_client_packet_map_chunk_biomes::Ty as B;
			let p = read(data, v1_16_2::play_to_client_packet_map_chunk::deserialize)?;
			let mut col = Column::new(p.x, p.z, matches!(p.biomes, B::True(_)), version);
			let present: Vec<usize> = (0..16).filter(|s| p.bit_map & 1 << s != 0).collect();
			decode_16(&mut col, &p.chunk_data, &present, 16)?;
			if let B::True(b) = p.biomes {
				col.biomes = Biomes::Cells(b);
			}
			col.heightmaps = Some(heightmaps_named(p.heightmaps.value));
			col.entities = named_entities(p.block_entities);
			Ok(col)
		}
		version_range!(V1_17..V1_18) => {
			let p = read(data, v1_17::play_to_client_packet_map_chunk::deserialize)?;
			let mut col = Column::new(p.x, p.z, true, version);
			let present: Vec<usize> = (0..p.bit_map.len() * 64)
				.filter(|i| bit(&p.bit_map, *i))
				.collect();
			decode_16(&mut col, &p.chunk_data, &present, p.biomes.len() / CELLS)?;
			col.biomes = Biomes::Cells(p.biomes);
			col.heightmaps = Some(heightmaps_named(p.heightmaps.value));
			col.entities = named_entities(p.block_entities);
			Ok(col)
		}
		version_range!(V1_18..V1_20) => {
			let p = read(data, v1_18::play_to_client_packet_map_chunk::deserialize)?;
			let mut col = Column::new(p.x, p.z, true, version);
			decode_18(&mut col, &p.chunk_data, version, sections)?;
			let n = col.sections.len();
			col.light = Some(light_from_masks(n, masks_of!(p), p.trust_edges));
			col.heightmaps = Some(heightmaps_named(p.heightmaps.value));
			col.entities = entities_18(p.x, p.z, p.block_entities);
			Ok(col)
		}
		Version::V1_20 => {
			let p = read(data, v1_20::play_to_client_packet_map_chunk::deserialize)?;
			let mut col = Column::new(p.x, p.z, true, version);
			decode_18(&mut col, &p.chunk_data, version, sections)?;
			let n = col.sections.len();
			col.light = Some(light_from_masks(n, masks_of!(p), false));
			col.heightmaps = Some(heightmaps_named(p.heightmaps.value));
			col.entities = entities_18(p.x, p.z, p.block_entities);
			Ok(col)
		}
		version_range!(V1_20_2..V1_21_5) => {
			let p = read(data, v1_20_2::play_to_client_packet_map_chunk::deserialize)?;
			let mut col = Column::new(p.x, p.z, true, version);
			decode_18(&mut col, &p.chunk_data, version, sections)?;
			let n = col.sections.len();
			col.light = Some(light_from_masks(n, masks_of!(p), false));
			col.heightmaps = Some(heightmaps_named(p.heightmaps.value));
			col.entities = entities_202(p.x, p.z, p.block_entities);
			Ok(col)
		}
		Version::V1_21_5 => {
			let p = read(data, v1_21_5::play_to_client_packet_map_chunk::deserialize)?;
			let mut col = Column::new(p.x, p.z, true, version);
			decode_18(&mut col, &p.chunk_data, version, sections)?;
			let n = col.sections.len();
			col.light = Some(light_from_masks(n, masks_of!(p), false));
			col.heightmaps = Some(
				p.heightmaps
					.into_iter()
					.map(|h| Heightmap {
						kind: HeightKind::Id(h.r#type),
						data: h.data,
					})
					.collect(),
			);
			col.entities = entities_202(p.x, p.z, p.block_entities);
			Ok(col)
		}
		_ => {
			let p = read(data, v1_21_6::play_to_client_packet_map_chunk::deserialize)?;
			let mut col = Column::new(p.x, p.z, true, version);
			decode_18(&mut col, &p.chunk_data, version, sections)?;
			let n = col.sections.len();
			col.light = Some(light_from_masks(n, masks_of!(p), false));
			col.heightmaps = Some(
				p.heightmaps
					.into_iter()
					.map(|h| Heightmap { kind: HeightKind::Id(v1_21_6::play_to_client_packet_map_chunk_heightmaps_items_type::virtual_tag(&h.r#type)), data: h.data })
					.collect(),
			);
			col.entities = entities_202(p.x, p.z, p.block_entities);
			Ok(col)
		}
	}
}

fn decode_bulk_body(version: Version, data: &[u8]) -> Result<(Vec<Column>, bool)> {
	let mut out = Vec::new();
	match version {
		Version::V1_7 => {
			let p = read(
				data,
				v1_7::play_to_client_packet_map_chunk_bulk::deserialize,
			)?;
			let raw = inflate(&p.compressed_chunk_data)?;
			let mut at = 0;
			for m in &p.meta {
				let (mask, add) = (m.bit_map as u32, m.add_bit_map as u32);
				let size = size_7(mask, add, p.sky_light_sent, true);
				let Some(chunk) = raw.get(at..at + size) else {
					return unconvertible("map_chunk_bulk data is shorter than its columns");
				};
				let mut col = Column::new(m.x, m.z, true, version);
				decode_7(&mut col, chunk, Bitmask { mask, add }, p.sky_light_sent)?;
				out.push(col);
				at += size;
			}
			Ok((out, p.sky_light_sent))
		}
		_ => {
			let p = read(
				data,
				v1_8::play_to_client_packet_map_chunk_bulk::deserialize,
			)?;
			let mut at = 0;
			for m in &p.meta {
				let mask = m.bit_map as u32;
				let size = size_8(mask, p.sky_light_sent, true);
				let Some(chunk) = p.data.get(at..at + size) else {
					return unconvertible("map_chunk_bulk data is shorter than its columns");
				};
				let mut col = Column::new(m.x, m.z, true, version);
				decode_8(&mut col, chunk, mask, p.sky_light_sent)?;
				out.push(col);
				at += size;
			}
			Ok((out, p.sky_light_sent))
		}
	}
}

fn decode_light_body(version: Version, data: &[u8]) -> Result<(i32, i32, Light)> {
	match version {
		version_range!(V1_14..V1_16) => {
			let p = read(data, v1_14::play_to_client_packet_update_light::deserialize)?;
			let l = light_from_rest(
				p.sky_light_mask,
				p.block_light_mask,
				p.empty_sky_light_mask,
				p.empty_block_light_mask,
				&p.data,
				false,
			)?;
			Ok((p.chunk_x, p.chunk_z, l))
		}
		version_range!(V1_16..V1_17) => {
			let p = read(data, v1_16::play_to_client_packet_update_light::deserialize)?;
			let l = light_from_rest(
				p.sky_light_mask,
				p.block_light_mask,
				p.empty_sky_light_mask,
				p.empty_block_light_mask,
				&p.data,
				p.trust_edges,
			)?;
			Ok((p.chunk_x, p.chunk_z, l))
		}
		version_range!(V1_17..V1_20) => {
			let p = read(data, v1_17::play_to_client_packet_update_light::deserialize)?;
			let l = light_from_masks(0, masks_of!(p), p.trust_edges);
			Ok((p.chunk_x, p.chunk_z, l))
		}
		_ => {
			let p = read(data, v1_20::play_to_client_packet_update_light::deserialize)?;
			let l = light_from_masks(0, masks_of!(p), false);
			Ok((p.chunk_x, p.chunk_z, l))
		}
	}
}

fn decode_unload_body(version: Version, data: &[u8]) -> Result<(i32, i32)> {
	match version {
		version_range!(..V1_20_2) => {
			let p = read(data, v1_9::play_to_client_packet_unload_chunk::deserialize)?;
			Ok((p.chunk_x, p.chunk_z))
		}
		_ => {
			let p = read(
				data,
				v1_20_2::play_to_client_packet_unload_chunk::deserialize,
			)?;
			Ok((p.chunk_x, p.chunk_z))
		}
	}
}

pub fn decode(version: Version, data: &[u8]) -> Result<Option<Packet>> {
	decode_in(version, data, None)
}

pub fn decode_in(version: Version, data: &[u8], sections: Option<usize>) -> Result<Option<Packet>> {
	let mut r = Reader::new(data);
	let Ok(id) = r.varint() else { return Ok(None) };
	let body = &data[data.len() - r.remaining()..];
	let ids = ids(version);
	if Some(id) == ids.chunk {
		return decode_chunk_body(version, body, sections).map(|c| Some(Packet::Chunk(c)));
	}
	if Some(id) == ids.bulk {
		return decode_bulk_body(version, body).map(|(c, sky)| Some(Packet::Bulk(c, sky)));
	}
	if Some(id) == ids.light {
		return decode_light_body(version, body).map(|(x, z, l)| Some(Packet::Light(x, z, l)));
	}
	if Some(id) == ids.unload {
		return decode_unload_body(version, body).map(|(x, z)| Some(Packet::Unload(x, z)));
	}
	Ok(None)
}
