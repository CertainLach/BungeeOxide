use std::sync::OnceLock;

use super::formats::{has_sky, light_array};
use super::model::{Biomes, BlockEntity, Column, HeightKind, Light, Nibbles, Section, CELLS};
use super::packets::{
	body, decode, encode_chunk, encode_light, encode_unload, entities_18, from_light_14,
	from_light_17, heightmap_id, heightmap_name, heightmaps_entry, heightmaps_named, light_14,
	light_17, list_18, list_202, named_entities, named_list, Packet,
};
use super::tables::{settle, step_table};
use super::Virtual;
use crate::custom as m;
use crate::gen::convert::{
	step, v1_13, v1_14, v1_17, v1_18, v1_20_2, v1_21_3, Carrier, Root, Version,
};
use crate::pds::convert::{Error, Result};
use crate::Session;

type Modern = v1_20_2::chunk_block_entity::Ty;

fn wire(bytes: Vec<u8>) -> Carrier {
	Carrier::Wire(bytes)
}

fn chunk(col: Column) -> Carrier {
	Carrier::Virtual(Virtual::Chunk(Box::new(col)))
}

fn present(col: &mut Column) -> Vec<(usize, Vec<u32>)> {
	col.sections
		.iter_mut()
		.enumerate()
		.take(16)
		.filter_map(|(i, s)| Some((i, std::mem::take(&mut s.as_mut()?.states))))
		.collect()
}

fn flattening(a: Version, b: Version, mut col: Column, cx: &mut Session) -> Result<Vec<Carrier>> {
	settle(&mut col, a);
	let at = m::FlatChunk {
		x: col.x,
		z: col.z,
		full: col.full,
		sky: has_sky(&col),
	};
	let sections = present(&mut col);
	let entities = named_list(&col.entities);
	let biomes = match (&mut col.biomes, col.full) {
		(Biomes::Flat(b), true) => Some(b),
		_ => None,
	};
	let flat = m::flattening_column_1_13(cx, at, sections, biomes, entities);
	for (i, states) in flat.sections {
		if let Some(Some(s)) = col.sections.get_mut(i) {
			s.states = states;
		}
	}
	col.entities = named_entities(flat.block_entities);
	col.space = b;
	col.at = b;
	let mut out = vec![chunk(col)];
	for p in flat.neighbors {
		out.push(wire(body(&p, v1_13::play_to_client_packet::serialize)?));
	}
	Ok(out)
}

fn village_up(a: Version, b: Version, mut col: Column, cx: &mut Session) -> Result<Vec<Carrier>> {
	settle(&mut col, a);
	let sky = has_sky(&col);
	let sections = present(&mut col)
		.into_iter()
		.map(|(y, states)| m::VillageSection {
			y,
			states,
			block: light_array(&col.light, y, false),
			sky: sky.then(|| light_array(&col.light, y, true)),
		})
		.collect();
	let village = m::village_column_1_14(cx, col.x, col.z, col.full, sections)?;
	for (s, count) in village.sections.into_iter().zip(village.counts) {
		if let Some(Some(section)) = col.sections.get_mut(s.y) {
			*section = Section::new(s.states, Some(count));
		}
	}
	col.light = Some(from_light_14(&village.light)?);
	col.heightmaps = Some(heightmaps_named(village.heightmaps.value));
	col.space = b;
	col.at = b;
	let mut out = Vec::with_capacity(village.before.len() + 1);
	for p in village.before {
		out.push(wire(body(&p, v1_14::play_to_client_packet::serialize)?));
	}
	out.push(chunk(col));
	Ok(out)
}

fn village_down(col: &mut Column, cx: &mut Session) -> Result<()> {
	if let Some(light) = col.light.take() {
		m::village_update_light_1_13_2(light_14(col.x, col.z, &light), cx)?;
	}
	let ys: Vec<usize> = col
		.sections
		.iter()
		.enumerate()
		.take(16)
		.filter(|(_, s)| s.is_some())
		.map(|(i, _)| i)
		.collect();
	let arrays = m::village_section_light_1_13_2(cx, col.x, col.z, &ys);
	let mut light = Light::new(16, arrays.first().is_none_or(|(_, sky)| sky.is_some()));
	for (y, (block, sky)) in ys.into_iter().zip(arrays) {
		light.block[y + 1] = Nibbles::Data(block);
		if let Some(sky) = sky {
			light.sky[y + 1] = Nibbles::Data(sky);
		}
	}
	col.light = Some(light);
	col.heightmaps = None;
	Ok(())
}

fn nether_partial(a: Version, b: Version, mut col: Column) -> Result<Vec<Carrier>> {
	settle(&mut col, a);
	let table = step_table(a, b);
	let mut out = Vec::new();
	if let Some(light) = &col.light {
		out.push(wire(encode_light(b, col.x, col.z, light)?));
	}
	for (y, s) in col.sections.iter().enumerate().take(16) {
		let Some(s) = s else { continue };
		let states: Vec<u32> = s
			.states
			.iter()
			.map(|s| table.get(*s as usize).copied().unwrap_or(0))
			.collect();
		out.push(wire(body(
			&m::nether_section_change_1_17(col.x, col.z, y as u32, &states),
			v1_17::play_to_client_packet::serialize,
		)?));
	}
	Ok(out)
}

fn caves_up(col: &mut Column, cx: &mut Session) -> Result<Vec<Carrier>> {
	let (x, z) = (col.x, col.z);
	let mut before = Vec::new();
	if let Some(light) = col.light.take() {
		for p in m::caves_update_light_1_18(light_17(x, z, &light), cx)? {
			before.push(wire(body(&p, v1_18::play_to_client_packet::serialize)?));
		}
	}
	let caves = m::caves_column_1_18(cx, x, z);
	let n = caves.sections;
	col.sections.resize(n, None);
	for s in col.sections.iter_mut() {
		if s.is_none() {
			*s = Some(Section::filled(0, Some(0)));
		}
	}
	let old = match &col.biomes {
		Biomes::Cells(c) => c.as_slice(),
		_ => &[],
	};
	col.biomes = Biomes::Cells(
		(0..n * CELLS)
			.map(|i| old.get(i).map_or(0, |b| (*b).max(0)))
			.collect(),
	);
	col.biome_bits = Some(caves.biome_bits);
	col.light = Some(from_light_17(n, caves.light));
	col.entities = entities_18(
		x,
		z,
		m::caves_block_entities_1_18(named_list(&col.entities)),
	);
	Ok(before)
}

fn bundle_1_21_3() -> Result<Vec<u8>> {
	static BUNDLE: OnceLock<Vec<u8>> = OnceLock::new();
	if let Some(b) = BUNDLE.get() {
		return Ok(b.clone());
	}
	let bytes = body(
		&v1_21_3::play_to_client_packet::Ty::BundleDelimiter,
		v1_21_3::play_to_client_packet::serialize,
	)?;
	Ok(BUNDLE.get_or_init(|| bytes).clone())
}

fn kept(result: Result<()>) -> Result<bool> {
	match result {
		Ok(()) => Ok(true),
		Err(Error::Dropped) => Ok(false),
		Err(e) => Err(e),
	}
}

fn each_modern(
	col: &mut Column,
	cx: &mut Session,
	f: impl Fn(&mut BlockEntity, &Modern, &mut Session) -> Result<()>,
) -> Result<()> {
	let typed = list_202(&col.entities);
	let mut out = Vec::with_capacity(typed.len());
	for (mut e, t) in std::mem::take(&mut col.entities).into_iter().zip(typed) {
		if kept(f(&mut e, &t, cx))? {
			out.push(e);
		}
	}
	col.entities = out;
	Ok(())
}

fn entities(a: Version, b: Version, col: &mut Column, cx: &mut Session) -> Result<()> {
	if col.entities.is_empty() {
		return Ok(());
	}
	let (x, z, up) = (col.x, col.z, b > a);
	match (a, b) {
		(Version::V1_9_4, Version::V1_9_2) => col.entities.clear(),
		(Version::V1_15_2, Version::V1_16) | (Version::V1_16, Version::V1_15_2) => {
			col.entities = named_entities(
				named_list(&col.entities)
					.into_iter()
					.map(|e| m::nether_block_entity_1_16(e, up))
					.collect(),
			);
		}
		(Version::V1_18, Version::V1_17_1) => {
			col.entities =
				named_entities(m::caves_block_entities_1_17_1(x, z, list_18(&col.entities)))
		}
		(Version::V1_19, Version::V1_18_2) => {
			col.entities = entities_18(x, z, m::caves_chunk_entities_1_18_2(list_18(&col.entities)))
		}
		(Version::V1_19_2, Version::V1_19_3)
		| (Version::V1_19_3, Version::V1_19_2)
		| (Version::V1_19_4, Version::V1_19_3) => {
			col.entities = entities_18(x, z, m::caves_chunk_entities(a, b, list_18(&col.entities)));
		}
		(Version::V1_19_4, Version::V1_20) | (Version::V1_20, Version::V1_19_4) => {
			col.entities = entities_18(
				x,
				z,
				m::trails_chunk_entities_1_20(list_18(&col.entities), up),
			)
		}
		(Version::V1_20, Version::V1_20_2) | (Version::V1_20_2, Version::V1_20) => {
			col.entities.iter_mut().for_each(|e| e.name.clear())
		}
		(Version::V1_20_3, Version::V1_20_5) | (Version::V1_20_5, Version::V1_20_3) => {
			for e in col.entities.iter_mut() {
				m::trails_chunk_entity_nbt_1_20_5(e.kind.unwrap_or(0), &mut e.nbt, up);
			}
		}
		(Version::V1_21_1, Version::V1_21_3) => each_modern(col, cx, |e, t, cx| {
			Ok(e.kind = Some(m::tricky_block_entity_1_21_3(t, cx)?))
		})?,
		(Version::V1_21_3, Version::V1_21_1) => each_modern(col, cx, |e, t, cx| {
			Ok(e.kind = Some(m::tricky_block_entity_down_1_21_1(t, cx)?))
		})?,
		(Version::V1_21_4, Version::V1_21_5) => each_modern(col, cx, |e, t, cx| {
			Ok(e.nbt = m::late_chunk_block_entity_up_1_21_5(t, cx)?)
		})?,
		(Version::V1_21_5, Version::V1_21_4) => each_modern(col, cx, |e, t, cx| {
			Ok(e.nbt = m::late_chunk_block_entity_down_1_21_4(t, cx)?)
		})?,
		(Version::V1_21_8, Version::V1_21_9) => each_modern(col, cx, |e, t, cx| {
			Ok(e.kind = Some(m::late_chunk_block_entity_up_1_21_9(t, cx)?))
		})?,
		(Version::V1_21_9, Version::V1_21_8) => each_modern(col, cx, |e, t, cx| {
			Ok(e.kind = Some(m::late_chunk_block_entity_down_1_21_8(t, cx)?))
		})?,
		_ => {}
	}
	Ok(())
}

fn rename_heightmaps(col: &mut Column, ids: bool) {
	if let Some(maps) = &mut col.heightmaps {
		maps.retain_mut(|h| {
			let kind = match ids {
				true => heightmap_id(&h.kind).map(HeightKind::Id),
				false => heightmap_name(&h.kind).map(HeightKind::Named),
			};
			match kind {
				Some(k) => {
					h.kind = k;
					true
				}
				None => false,
			}
		});
	}
}

fn repack_heightmaps(col: &mut Column, up: bool) {
	if col.heightmaps.is_some() {
		let mut entry = heightmaps_entry(&col.heightmaps);
		m::nether_heightmaps_1_16(&mut entry, up);
		col.heightmaps = Some(heightmaps_named(entry.value));
	}
}

fn recounted(a: Version, b: Version) -> bool {
	matches!(
		(a, b),
		(Version::V1_15_2, Version::V1_16)
			| (Version::V1_16, Version::V1_15_2)
			| (Version::V1_16_1, Version::V1_16_2)
			| (Version::V1_16_2, Version::V1_16_1)
			| (Version::V1_16_2, Version::V1_17)
	)
}

fn envelope(a: Version, b: Version, col: &mut Column, cx: &mut Session) {
	if recounted(a, b) {
		col.sections
			.iter_mut()
			.flatten()
			.for_each(|s| s.count = None);
	}
	match (a, b) {
		(Version::V1_14_4, Version::V1_15) => {
			if let Biomes::Flat(old) = &col.biomes {
				if old.len() == 256 {
					col.biomes = Biomes::Cells(m::village_biomes_1_15(old));
				}
			}
		}
		(Version::V1_15, Version::V1_14_4) => {
			if let Biomes::Cells(cells) = &col.biomes {
				if cells.len() == 1024 {
					col.biomes = Biomes::Flat(m::village_biomes_1_14_4(cells));
				}
			}
		}
		(Version::V1_15_2, Version::V1_16) => {
			repack_heightmaps(col, true);
			if let Some(l) = &mut col.light {
				l.trust = true;
			}
		}
		(Version::V1_16, Version::V1_15_2) => repack_heightmaps(col, false),
		(Version::V1_16_2, Version::V1_16_1) => {
			if let Biomes::Cells(cells) = &mut col.biomes {
				*cells = m::nether_biomes_1_16_1(cx, std::mem::take(cells));
			}
		}
		(Version::V1_18, Version::V1_17_1) => {
			for s in col.sections.iter_mut() {
				if s.as_ref().is_some_and(|s| s.count == Some(0)) {
					*s = None;
				}
			}
		}
		(Version::V1_20, Version::V1_19_4) => {
			if let Some(l) = &mut col.light {
				l.trust = true;
			}
		}
		(Version::V1_21_4, Version::V1_21_5) => rename_heightmaps(col, true),
		(Version::V1_21_5, Version::V1_21_4) => rename_heightmaps(col, false),
		(Version::V1_9, Version::V1_8) => {
			if col.full && col.sections.iter().take(16).all(Option::is_none) {
				if col.sections.is_empty() {
					col.sections.push(None);
				}
				col.sections[0] = Some(Section::filled(0, None));
				let light = col.light.get_or_insert_with(|| Light::new(16, true));
				if light.block.len() < 2 {
					light.block.resize(2, Nibbles::Missing);
					light.sky.resize(2, Nibbles::Missing);
				}
				light.block[1] = Nibbles::Data(vec![0; 2048]);
				light.sky[1] = Nibbles::Data(vec![0xff; 2048]);
				light.has_sky = true;
			}
		}
		_ => {}
	}
}

pub(super) fn advance(
	a: Version,
	b: Version,
	mut col: Column,
	cx: &mut Session,
) -> Result<Vec<Carrier>> {
	match (a, b) {
		(Version::V1_17, Version::V1_16_2) => return m::nether_column_1_16_2(col, cx),
		(Version::V1_12_2, Version::V1_13) => return flattening(a, b, col, cx),
		(Version::V1_13_2, Version::V1_14) => return village_up(a, b, col, cx),
		(Version::V1_16_2, Version::V1_17) if !col.full => return nether_partial(a, b, col),
		(Version::V1_8, Version::V1_9) if col.full && col.sections.iter().all(Option::is_none) => {
			return Ok(vec![wire(encode_unload(b, col.x, col.z)?)])
		}
		_ => {}
	}
	let mut before = Vec::new();
	let mut after = Vec::new();
	match (a, b) {
		(Version::V1_14, Version::V1_13_2) => village_down(&mut col, cx)?,
		(Version::V1_17_1, Version::V1_18) => before = caves_up(&mut col, cx)?,
		_ => entities(a, b, &mut col, cx)?,
	}
	if (a, b) == (Version::V1_21_1, Version::V1_21_3) && m::tricky_resend_1_21_3(cx, col.x, col.z) {
		before.push(wire(bundle_1_21_3()?));
		before.push(wire(encode_unload(b, col.x, col.z)?));
		after.push(wire(bundle_1_21_3()?));
	}
	envelope(a, b, &mut col, cx);
	col.at = b;
	before.push(chunk(col));
	before.extend(after);
	Ok(before)
}

pub(super) fn advance_bulk(
	a: Version,
	b: Version,
	mut columns: Vec<Column>,
	sky: bool,
) -> Result<Vec<Carrier>> {
	if (a, b) != (Version::V1_8, Version::V1_9) {
		for c in &mut columns {
			c.at = b;
		}
		return Ok(vec![Carrier::Virtual(Virtual::Bulk(columns, sky))]);
	}
	let mut out = Vec::with_capacity(columns.len());
	for mut c in columns {
		if c.sections.iter().all(Option::is_none) {
			out.push(wire(encode_unload(b, c.x, c.z)?));
			continue;
		}
		c.at = b;
		out.push(chunk(c));
	}
	Ok(out)
}
