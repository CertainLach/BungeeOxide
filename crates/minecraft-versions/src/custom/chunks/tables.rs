use std::sync::OnceLock;

use super::formats::{domain, format, Format};
use super::model::{Biomes, Column, Light, Section, BLOCKS};
use super::packets::{decode, encode_chunk, Packet};
use crate::gen::convert::{step, Root, Version};
use crate::registry;
use crate::version_range;
use crate::Session;

type Cache = OnceLock<Vec<OnceLock<Box<[u32]>>>>;

fn slot(cache: &'static Cache, a: Version, b: Version) -> &'static OnceLock<Box<[u32]>> {
	let n = Version::ALL.len();
	&cache.get_or_init(|| (0..n * n).map(|_| OnceLock::new()).collect())[a.index() * n + b.index()]
}

pub fn probe_column(version: Version) -> Column {
	let n = domain(version);
	let mut col = Column::new(0, 0, true, version);
	col.sections = (0..n / BLOCKS)
		.map(|k| {
			Some(Section::new(
				(k * BLOCKS..(k + 1) * BLOCKS).map(|s| s as u32).collect(),
				None,
			))
		})
		.collect();
	col.biomes = match format(version) {
		Format::Legacy7 | Format::Raw8 | Format::Pal9 | Format::Flat13 => {
			Biomes::Flat(vec![0; 256])
		}
		_ if version < Version::V1_15 => Biomes::Flat(vec![0; 256]),
		_ => Biomes::Cells(vec![0; col.sections.len().max(16) * 64]),
	};
	col.light = match version {
		version_range!(V1_14..V1_18) => None,
		_ => Some(Light::new(col.sections.len().max(16), true)),
	};
	col.heightmaps = Some(Vec::new());
	col
}

pub fn probe(a: Version, b: Version) -> Option<Vec<u32>> {
	let col = probe_column(a);
	let packets = encode_chunk(a, &col).ok()?;
	let mut cx = Session::new();
	let outs = step(Root::PlayToClient, a, b, packets.last()?, &mut cx).ok()?;
	let out = outs.iter().find_map(|o| match decode(b, o) {
		Ok(Some(Packet::Chunk(c))) => Some(c),
		_ => None,
	})?;
	let mut table = Vec::with_capacity(domain(a));
	for (k, s) in col.sections.iter().enumerate() {
		let Some(s) = s else { continue };
		let got = out.sections.get(k).and_then(Option::as_ref);
		match got {
			Some(g) => table.extend_from_slice(&g.states),
			None => table.extend(s.states.iter().map(|_| 0)),
		}
	}
	Some(table)
}

fn legacy_down(state: u32) -> u32 {
	let known = |id: u32| {
		registry::blocks(Version::V1_8).is_some_and(|b| b.list.iter().any(|x| x.id == id))
	};
	let (id, meta) = (state >> 4, state & 15);
	if known(id) {
		return state;
	}
	let (id, meta) = match id {
		198 => (50, 5),
		199 | 200 => (0, 0),
		201 | 202 | 204 => (155, 0),
		203 => (156, meta),
		205 => (44, 7 | (meta & 8)),
		206 => (121, 0),
		207 => (59, meta.min(7)),
		208 => (2, 0),
		209 => (119, 0),
		210 | 211 => (137, 0),
		212 => (79, 0),
		213 => (87, 0),
		214 => (159, 14),
		215 => (112, 0),
		216 => (155, 2),
		217 => (0, 0),
		218 => (23, meta),
		219..=234 => (159, id - 219),
		235..=250 => (159, id - 235),
		251 => (35, meta),
		252 => (12, 0),
		_ => (1, 0),
	};
	id << 4 | meta
}

pub fn step_table(a: Version, b: Version) -> &'static [u32] {
	static CACHE: Cache = OnceLock::new();
	slot(&CACHE, a, b).get_or_init(|| match probe(a, b) {
		Some(t) => t.into_boxed_slice(),
		None if (a, b) == (Version::V1_9, Version::V1_8) => {
			(0..domain(a) as u32).map(legacy_down).collect()
		}
		None => (0..domain(a) as u32).collect(),
	})
}

pub fn composed(from: Version, to: Version) -> &'static [u32] {
	static CACHE: Cache = OnceLock::new();
	slot(&CACHE, from, to).get_or_init(|| {
		let Some(next) = (if to > from { from.next() } else { from.prev() }) else {
			return Box::default();
		};
		let first = step_table(from, next);
		if next == to {
			return first.into();
		}
		let rest = composed(next, to);
		first
			.iter()
			.map(|s| rest.get(*s as usize).copied().unwrap_or(0))
			.collect()
	})
}

pub fn settle(col: &mut Column, to: Version) {
	if col.space == to {
		return;
	}
	let table = composed(col.space, to);
	for s in col.sections.iter_mut().flatten() {
		for state in s.states.iter_mut() {
			*state = table.get(*state as usize).copied().unwrap_or(0);
		}
	}
	col.space = to;
}
