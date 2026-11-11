use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::gen::convert::{v1_13_2, v1_14, Version};
use crate::pds::convert::{unconvertible, Error, Result};
use crate::pds::{Input, Output};
use crate::pds_native::{pds_bool, pds_i32, pds_u8, pds_varint};
use crate::registry;
use crate::session::Session;
use bungeeoxide_nbt::{CompoundExt, Entry as Nbt, Payload as Tag};

type Client13 = v1_13_2::play_to_client_packet::Ty;
type Client14 = v1_14::play_to_client_packet::Ty;
type Server13 = v1_13_2::play_to_server_packet::Ty;
type Slot = v1_13_2::slot::Ty;
type M13 = v1_13_2::entity_metadata_item::Ty;
type M14 = v1_14::entity_metadata_item::Ty;
type E13 = v1_13_2::entity_metadata_items::Ty;
type E14 = v1_14::entity_metadata_items::Ty;
type P13 = v1_13_2::particle_data::Ty;
type P14 = v1_14::particle_data::Ty;

const OLD: Version = Version::V1_13_2;
const NEW: Version = Version::V1_14;
const CUSTOM_STATISTICS: i32 = 58;
pub(super) const VIEW_DISTANCE: i32 = 64;
const GLOBAL_BITS: usize = 14;
const FULL_LIGHT: [u8; 2048] = [0xff; 2048];

pub(super) fn remap(kind: &str, from: Version, to: Version, id: i32) -> Option<i32> {
	let id = u32::try_from(id).ok()?;
	registry::remap_name(kind, from, to, id).map(|v| v as i32)
}

pub(super) fn state(from: Version, to: Version, id: i32) -> i32 {
	registry::remap_state(from, to, id as u32) as i32
}

pub(super) fn entity_name(version: Version, id: i32) -> &'static str {
	registry::names(version, "entities")
		.and_then(|n| n.name(id as u32))
		.unwrap_or("")
}

pub(super) fn entity_id(version: Version, name: &str) -> Option<i32> {
	registry::names(version, "entities")
		.and_then(|n| n.id(name))
		.map(|i| i as i32)
}

pub(super) fn item_id(version: Version, name: &str) -> Option<i32> {
	registry::names(version, "items")
		.and_then(|n| n.id(name))
		.map(|i| i as i32)
}

pub(super) fn item_name(version: Version, slot: &Slot) -> &'static str {
	match slot {
		Slot::True(s) => registry::names(version, "items")
			.and_then(|n| n.name(s.item_id as u32))
			.unwrap_or(""),
		_ => "",
	}
}

pub(super) fn item(from: Version, to: Version, slot: Slot) -> Slot {
	match slot {
		Slot::True(mut s) => match remap("items", from, to, s.item_id) {
			Some(id) => {
				s.item_id = id;
				Slot::True(s)
			}
			None => Slot::False,
		},
		empty => empty,
	}
}

fn lore(slot: &mut Slot, f: fn(&str) -> String) {
	let Slot::True(s) = slot else { return };
	let Some(Nbt {
		value: Tag::Compound(root),
		..
	}) = &mut s.nbt_data
	else {
		return;
	};
	let Some(display) = root.compound_mut("display") else {
		return;
	};
	if let Some(Tag::List(list)) = display.field_mut("Lore").as_deref_mut() {
		for line in list.strings_mut().into_iter().flatten() {
			*line = f(line);
		}
	}
}

fn lore_1_14(line: &str) -> String {
	serde_json::json!({ "text": line }).to_string()
}

#[derive(Clone, Default, PartialEq)]
struct Style {
	color: Option<char>,
	formats: [bool; 5],
}

const FORMATS: [(&str, char); 5] = [
	("obfuscated", 'k'),
	("bold", 'l'),
	("strikethrough", 'm'),
	("underlined", 'n'),
	("italic", 'o'),
];

const COLORS: [(&str, char); 16] = [
	("black", '0'),
	("dark_blue", '1'),
	("dark_green", '2'),
	("dark_aqua", '3'),
	("dark_red", '4'),
	("dark_purple", '5'),
	("gold", '6'),
	("gray", '7'),
	("dark_gray", '8'),
	("blue", '9'),
	("green", 'a'),
	("aqua", 'b'),
	("red", 'c'),
	("light_purple", 'd'),
	("yellow", 'e'),
	("white", 'f'),
];

fn legacy(value: &serde_json::Value, parent: &Style, last: &mut Style, out: &mut String) {
	use serde_json::Value;
	let mut style = parent.clone();
	let text = match value {
		Value::String(s) => s.clone(),
		Value::Array(parts) => {
			for p in parts {
				legacy(p, parent, last, out);
			}
			return;
		}
		Value::Object(o) => {
			if let Some(c) = o.get("color").and_then(Value::as_str) {
				style.color = COLORS.iter().find(|(n, _)| *n == c).map(|(_, code)| *code);
			}
			for (i, (name, _)) in FORMATS.iter().enumerate() {
				if let Some(on) = o.get(*name).and_then(Value::as_bool) {
					style.formats[i] = on;
				}
			}
			let own = match (
				o.get("text").and_then(Value::as_str),
				o.get("translate").and_then(Value::as_str),
			) {
				(Some(t), _) => t.to_owned(),
				(None, Some(key)) => key.to_owned(),
				_ => String::new(),
			};
			emit(&own, &style, last, out);
			for p in o
				.get("extra")
				.and_then(Value::as_array)
				.into_iter()
				.flatten()
			{
				legacy(p, &style, last, out);
			}
			return;
		}
		Value::Number(n) => n.to_string(),
		Value::Bool(b) => b.to_string(),
		Value::Null => return,
	};
	emit(&text, &style, last, out);
}

fn emit(text: &str, style: &Style, last: &mut Style, out: &mut String) {
	if text.is_empty() {
		return;
	}
	if style != last {
		let dropped = last
			.formats
			.iter()
			.zip(style.formats)
			.any(|(was, is)| *was && !is);
		if style.color != last.color || dropped {
			match style.color {
				Some(c) => out.extend(['\u{a7}', c]),
				None => out.push_str("\u{a7}r"),
			}
			last.formats = [false; 5];
		}
		for (i, (_, code)) in FORMATS.iter().enumerate() {
			if style.formats[i] && !last.formats[i] {
				out.extend(['\u{a7}', *code]);
			}
		}
		*last = style.clone();
	}
	out.push_str(text);
}

fn lore_1_13_2(line: &str) -> String {
	match serde_json::from_str::<serde_json::Value>(line) {
		Ok(value) => {
			let mut out = String::new();
			legacy(&value, &Style::default(), &mut Style::default(), &mut out);
			out
		}
		Err(_) => line.to_owned(),
	}
}

fn slot14(s: Slot) -> Slot {
	let mut s = item(OLD, NEW, s);
	lore(&mut s, lore_1_14);
	s
}

fn slot13(s: Slot) -> Slot {
	let mut s = item(NEW, OLD, s);
	lore(&mut s, lore_1_13_2);
	s
}

pub fn village_slot_1_14(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot14(v))
}

pub fn village_slot_1_13_2(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot13(v))
}

pub(super) struct Bytes<'a> {
	data: &'a [u8],
	at: usize,
}

impl<'a> Bytes<'a> {
	pub(super) fn new(data: &'a [u8]) -> Self {
		Self { data, at: 0 }
	}

	pub(super) fn take(&mut self, n: usize) -> Result<&'a [u8]> {
		let out = self
			.data
			.get(self.at..self.at.saturating_add(n))
			.ok_or_else(|| Error::Unconvertible("chunk data ends early".into()))?;
		self.at += n;
		Ok(out)
	}

	pub(super) fn varint(&mut self) -> Result<i32> {
		let mut v = 0u32;
		for shift in (0..35).step_by(7) {
			let b = self.take(1)?[0];
			v |= ((b & 0x7f) as u32) << shift;
			if b & 0x80 == 0 {
				return Ok(v as i32);
			}
		}
		unconvertible("varint is too long")
	}

	pub(super) fn rest(&self) -> &'a [u8] {
		&self.data[self.at..]
	}
}

pub(super) fn put_varint(out: &mut Vec<u8>, v: i32) {
	let mut v = v as u32;
	while v >= 0x80 {
		out.push(v as u8 | 0x80);
		v >>= 7;
	}
	out.push(v as u8);
}

pub(super) fn read_states(r: &mut Bytes) -> Result<Vec<u32>> {
	let bits = r.take(1)?[0] as usize;
	let palette = if bits <= 8 {
		let n = r.varint()?;
		(0..n)
			.map(|_| r.varint().map(|v| v as u32))
			.collect::<Result<Vec<_>>>()?
	} else {
		Vec::new()
	};
	let bits = match bits {
		0..=4 => 4,
		5..=8 => bits,
		_ => GLOBAL_BITS,
	};
	let n = r.varint()? as usize;
	if n != 64 * bits {
		return unconvertible(format!("a section has {n} longs for {bits} bits"));
	}
	let longs: Vec<u64> = r
		.take(n * 8)?
		.chunks_exact(8)
		.map(|c| u64::from_be_bytes(c.try_into().expect("8 bytes")))
		.collect();
	let mask = (1u64 << bits) - 1;
	(0..4096)
		.map(|i| {
			let (word, off) = (i * bits / 64, i * bits % 64);
			let mut v = longs[word] >> off;
			if off + bits > 64 {
				v |= longs[word + 1] << (64 - off);
			}
			let v = (v & mask) as u32;
			match palette.is_empty() {
				true => Ok(v),
				false => palette
					.get(v as usize)
					.copied()
					.ok_or_else(|| Error::Unconvertible(format!("palette index {v} out of range"))),
			}
		})
		.collect()
}

pub(super) fn write_states(out: &mut Vec<u8>, states: &[u32]) {
	let mut palette = Vec::new();
	let mut index = HashMap::new();
	for s in states {
		index.entry(*s).or_insert_with(|| {
			palette.push(*s);
			palette.len() as u64 - 1
		});
	}
	let local = palette.len() <= 256;
	let bits = match local {
		true => (usize::BITS - (palette.len() - 1).leading_zeros()).max(4) as usize,
		false => GLOBAL_BITS,
	};
	out.push(bits as u8);
	if local {
		put_varint(out, palette.len() as i32);
		for p in &palette {
			put_varint(out, *p as i32);
		}
	}
	let mut longs = vec![0u64; 64 * bits];
	for (i, s) in states.iter().enumerate() {
		let v = if local { index[s] } else { *s as u64 };
		let (word, off) = (i * bits / 64, i * bits % 64);
		longs[word] |= v << off;
		if off + bits > 64 {
			longs[word + 1] |= v >> (64 - off);
		}
	}
	put_varint(out, longs.len() as i32);
	for l in longs {
		out.extend(l.to_be_bytes());
	}
}

fn nibble(a: &[u8], i: usize) -> u8 {
	(a[i >> 1] >> ((i & 1) * 4)) & 15
}

fn set_nibble(a: &mut [u8], i: usize, v: u8) {
	let s = (i & 1) * 4;
	a[i >> 1] = (a[i >> 1] & !(15 << s)) | (v << s);
}

const SOLID: u8 = 1;
const MOTION: u8 = 2;
const NON_FULL: u8 = 4;

const PASSABLE: &[&str] = &[
	"grass",
	"fern",
	"dead_bush",
	"tall_grass",
	"large_fern",
	"dandelion",
	"poppy",
	"blue_orchid",
	"allium",
	"azure_bluet",
	"oxeye_daisy",
	"cornflower",
	"lily_of_the_valley",
	"wither_rose",
	"sunflower",
	"lilac",
	"rose_bush",
	"peony",
	"brown_mushroom",
	"red_mushroom",
	"wheat",
	"carrots",
	"potatoes",
	"beetroots",
	"melon_stem",
	"pumpkin_stem",
	"attached_melon_stem",
	"attached_pumpkin_stem",
	"sugar_cane",
	"nether_wart",
	"vine",
	"lily_pad",
	"cocoa",
	"sweet_berry_bush",
	"bamboo_sapling",
	"chorus_plant",
	"chorus_flower",
	"snow",
	"cobweb",
	"fire",
	"nether_portal",
	"end_portal",
	"end_gateway",
	"tripwire",
	"tripwire_hook",
	"redstone_wire",
	"repeater",
	"comparator",
	"lever",
	"ladder",
	"end_rod",
	"flower_pot",
	"structure_void",
	"scaffolding",
	"torch",
	"wall_torch",
	"redstone_torch",
	"redstone_wall_torch",
	"rail",
	"powered_rail",
	"detector_rail",
	"activator_rail",
];

fn block_flags(name: &str, values: &[(&str, &str)]) -> u8 {
	if matches!(name, "air" | "cave_air" | "void_air") {
		return 0;
	}
	let wet = matches!(
		name,
		"water" | "lava" | "bubble_column" | "seagrass" | "tall_seagrass" | "kelp" | "kelp_plant"
	) || values.contains(&("waterlogged", "true"));
	let passable = PASSABLE.contains(&name)
		|| name.starts_with("potted_")
		|| [
			"_sapling", "_tulip", "_carpet", "_button", "_head", "_skull",
		]
		.iter()
		.any(|s| name.ends_with(s));
	let non_full = (name.ends_with("_slab") && !values.contains(&("type", "double")))
		|| name.ends_with("_stairs")
		|| matches!(name, "farmland" | "grass_path");
	SOLID | if wet || !passable { MOTION } else { 0 } | if non_full { NON_FULL } else { 0 }
}

fn flags_1_14() -> &'static [u8] {
	static FLAGS: OnceLock<Vec<u8>> = OnceLock::new();
	FLAGS.get_or_init(|| {
		let Some(b) = registry::blocks(NEW) else {
			return vec![];
		};
		(0..b.states())
			.map(|s| {
				b.owner(s).map_or(SOLID | MOTION, |block| {
					block_flags(&block.name, &block.values(s))
				})
			})
			.collect()
	})
}

fn flag(s: u32) -> u8 {
	flags_1_14()
		.get(s as usize)
		.copied()
		.unwrap_or(SOLID | MOTION)
}

pub struct VillageSection {
	pub y: usize,
	pub states: Vec<u32>,
	pub block: Vec<u8>,
	pub sky: Option<Vec<u8>>,
}

fn sections_1_13_2(
	data: &[u8],
	bitmap: i32,
	full: bool,
	sky: bool,
) -> Result<(Vec<VillageSection>, Vec<u8>)> {
	let mut r = Bytes::new(data);
	let mut out = Vec::new();
	for y in (0..16).filter(|y| bitmap & 1 << y != 0) {
		let states = read_states(&mut r)?;
		let block = r.take(2048)?.to_vec();
		let sky = match sky {
			true => Some(r.take(2048)?.to_vec()),
			false => None,
		};
		out.push(VillageSection {
			y,
			states,
			block,
			sky,
		});
	}
	let biomes = if full {
		r.take(1024)?.to_vec()
	} else {
		Vec::new()
	};
	if !r.rest().is_empty() {
		return unconvertible(format!(
			"{} bytes follow the chunk sections",
			r.rest().len()
		));
	}
	Ok((out, biomes))
}

const FACES: [(i32, i32, i32); 6] = [
	(0, -1, 0),
	(0, 1, 0),
	(0, 0, -1),
	(0, 0, 1),
	(-1, 0, 0),
	(1, 0, 0),
];

fn fix_light(sections: &mut [VillageSection]) {
	let mut at = [usize::MAX; 16];
	for (i, s) in sections.iter().enumerate() {
		at[s.y] = i;
	}
	for si in 0..sections.len() {
		for idx in 0..4096 {
			if flag(sections[si].states[idx]) & NON_FULL == 0 {
				continue;
			}
			let (x, z, y) = ((idx & 15) as i32, (idx >> 4 & 15) as i32, (idx >> 8) as i32);
			let (mut sky, mut block) = (0u8, 0u8);
			for (dx, dy, dz) in FACES {
				let (nx, nz) = (x + dx, z + dz);
				if !(0..16).contains(&nx) || !(0..16).contains(&nz) {
					continue;
				}
				let sy = sections[si].y as i32;
				let (ns, ny) = match y + dy {
					-1 => (sy - 1, 15),
					16 => (sy + 1, 0),
					n => (sy, n),
				};
				let Some(&ni) = usize::try_from(ns)
					.ok()
					.and_then(|s| at.get(s))
					.filter(|i| **i != usize::MAX)
				else {
					continue;
				};
				let n_idx = (ny << 8 | nz << 4 | nx) as usize;
				let nb = nibble(&sections[ni].block, n_idx);
				if nb == 15 {
					block = 14;
				} else if nb > block {
					block = nb - 1;
				}
				if let Some(l) = &sections[ni].sky {
					let n = nibble(l, n_idx);
					if sky != 15 && n == 15 {
						sky = if dy == 1 { 15 } else { 14 };
					} else if sky != 15 && n > sky {
						sky = n - 1;
					}
				}
			}
			let sec = &mut sections[si];
			if block != 0 {
				set_nibble(&mut sec.block, idx, block);
			}
			if let (Some(l), true) = (&mut sec.sky, sky != 0) {
				set_nibble(l, idx, sky);
			}
		}
	}
}

fn open_sky(y: usize, surface: &[i64; 256]) -> Vec<u8> {
	let mut out = vec![0u8; 2048];
	for i in 0..4096 {
		if (y * 16 + i / 256) as i64 >= surface[i & 255] {
			set_nibble(&mut out, i, 15);
		}
	}
	out
}

fn light_1_14(
	x: i32,
	z: i32,
	full: bool,
	sections: &[VillageSection],
	surface: &[i64; 256],
) -> v1_14::play_to_client_packet_update_light::Ty {
	let mut sky_mask = if full { 0x3ffff } else { 0 };
	let mut block_mask = 0;
	for s in sections {
		if !full && s.sky.is_some() {
			sky_mask |= 1 << (s.y + 1);
		}
		block_mask |= 1 << (s.y + 1);
	}
	let mut data = Vec::new();
	let put = |data: &mut Vec<u8>, bytes: &[u8]| {
		put_varint(data, bytes.len() as i32);
		data.extend_from_slice(bytes);
	};
	if full {
		put(&mut data, &FULL_LIGHT);
		for y in 0..16 {
			match sections.iter().find(|s| s.y == y) {
				Some(VillageSection { sky: Some(l), .. }) => put(&mut data, l),
				Some(_) => put(&mut data, &FULL_LIGHT),
				None => put(&mut data, &open_sky(y, surface)),
			}
		}
		put(&mut data, &FULL_LIGHT);
	} else {
		for l in sections.iter().filter_map(|s| s.sky.as_ref()) {
			put(&mut data, l);
		}
	}
	for s in sections {
		put(&mut data, &s.block);
	}
	v1_14::play_to_client_packet_update_light::Ty {
		chunk_x: x,
		chunk_z: z,
		sky_light_mask: sky_mask,
		block_light_mask: block_mask,
		empty_sky_light_mask: 0,
		empty_block_light_mask: 0,
		data,
	}
}

pub(super) fn pack_heights(h: &[i64; 256]) -> Vec<i64> {
	let mut longs = vec![0u64; 36];
	for (i, v) in h.iter().enumerate() {
		let (v, bit) = (*v as u64 & 0x1ff, i * 9);
		let (word, off) = (bit / 64, bit % 64);
		longs[word] |= v << off;
		if off + 9 > 64 {
			longs[word + 1] |= v >> (64 - off);
		}
	}
	longs.into_iter().map(|l| l as i64).collect()
}

fn heightmaps(full: bool, motion: &[i64; 256], surface: &[i64; 256]) -> Nbt {
	let maps = match full {
		true => vec![
			Nbt::new("MOTION_BLOCKING", Tag::LongArray(pack_heights(motion))),
			Nbt::new("WORLD_SURFACE", Tag::LongArray(pack_heights(surface))),
		],
		false => vec![],
	};
	Nbt::new("", Tag::Compound(maps))
}

#[derive(Default)]
struct Light13 {
	sky: Vec<Option<Vec<u8>>>,
	block: Vec<Option<Vec<u8>>>,
}

#[derive(Default)]
struct State14 {
	self_id: i32,
	no_sky: bool,
	center: Option<(i32, i32)>,
	pos: Option<(f64, f64)>,
	types: HashMap<i32, &'static str>,
	mob_flags: HashMap<i32, i8>,
	player_flags: HashMap<i32, i8>,
	sleeping: HashSet<i32>,
	riptide: HashSet<i32>,
	trade_window: u8,
	actions: i16,
	difficulty: u8,
	light: HashMap<(i32, i32), Light13>,
	down_types: HashMap<i32, &'static str>,
	positions: HashMap<i32, (f64, f64, f64)>,
}

fn chunk_of(x: f64, z: f64) -> (i32, i32) {
	((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
}

impl State14 {
	fn recenter(&mut self, chunk: Option<(i32, i32)>, out: &mut Vec<Client14>) {
		let far = |(a, b): (i32, i32), (c, d): (i32, i32)| {
			(a - c).abs() >= VIEW_DISTANCE || (b - d).abs() >= VIEW_DISTANCE
		};
		let player = self.pos.map(|(x, z)| chunk_of(x, z));
		let target = match (chunk, player) {
			(Some(c), Some(p)) if !far(c, p) => p,
			(Some(c), _) if self.center.is_some_and(|cur| !far(c, cur)) => return,
			(Some(c), _) => c,
			(None, Some(p)) => p,
			(None, None) => return,
		};
		if self.center != Some(target) {
			self.center = Some(target);
			out.push(Client14::UpdateViewPosition(
				v1_14::play_to_client_packet_update_view_position::Ty {
					chunk_x: target.0,
					chunk_z: target.1,
				},
			));
		}
	}

	fn forget(&mut self, id: i32) {
		self.types.remove(&id);
		self.mob_flags.remove(&id);
		self.player_flags.remove(&id);
		self.sleeping.remove(&id);
		self.riptide.remove(&id);
		self.down_types.remove(&id);
		self.positions.remove(&id);
	}

	fn reset(&mut self, self_id: i32, dimension: i32) {
		self.types.clear();
		self.mob_flags.clear();
		self.player_flags.clear();
		self.sleeping.clear();
		self.riptide.clear();
		self.down_types.clear();
		self.light.clear();
		self.positions.clear();
		self.self_id = self_id;
		self.types.insert(self_id, "player");
		self.down_types.insert(self_id, "player");
		self.no_sky = dimension != 0;
		self.center = None;
		self.pos = None;
	}

	fn pose(&self, id: i32) -> i32 {
		let f = self.player_flags.get(&id).copied().unwrap_or(0) as u8;
		match () {
			_ if f & 0x80 != 0 => 1,
			_ if self.sleeping.contains(&id) => 2,
			_ if f & 0x10 != 0 => 3,
			_ if self.riptide.contains(&id) => 4,
			_ if f & 0x02 != 0 => 5,
			_ => 0,
		}
	}

	fn aggressive(&mut self, id: i32, on: bool) {
		let f = self.mob_flags.entry(id).or_default();
		*f = if on { *f | 4 } else { *f & !4 };
	}

	fn metadata_1_14(
		&mut self,
		id: i32,
		items: Vec<E13>,
		extra: &mut Vec<Client14>,
	) -> Result<Vec<E14>> {
		let name = self.types.get(&id).copied().unwrap_or("");
		let mut out = Vec::with_capacity(items.len() + 2);
		let (mut flags, mut pose) = (false, false);
		for e in items {
			let k = e.key;
			let mut value = value_1_14(e.value)?;
			match (name, k, &value) {
				("player", 0, M14::V0(b)) => {
					self.player_flags.insert(id, *b);
					pose = true;
				}
				("player", 6, M14::V0(b)) => {
					match b & 4 != 0 {
						true => self.riptide.insert(id),
						false => self.riptide.remove(&id),
					};
					pose = true;
				}
				(n, 7, M14::V2(h)) if is_living(n) && h.is_nan() => value = M14::V2(1.0),
				(n, 9, M14::V1(s)) if n.ends_with("minecart") => {
					value = M14::V1(state(OLD, NEW, *s))
				}
				("firework_rocket", 7, M14::V1(e)) => {
					value = M14::V17(if *e == 0 { 0 } else { e + 1 })
				}
				("villager", 13, M14::V1(p)) | ("zombie_villager", 17, M14::V1(p)) => {
					value = villager_data(*p)
				}
				_ => {}
			}
			match key_1_14(name, k) {
				Key::To(13) if is_mob(name) => {
					if let M14::V0(b) = value {
						let old = self.mob_flags.get(&id).copied().unwrap_or(0);
						self.mob_flags.insert(id, (b & !4) | (old & 4));
						flags = true;
					}
				}
				Key::To(key) => out.push(entry_14(key, value)),
				Key::Aggressive => {
					let on = match value {
						M14::V7(b) => b,
						M14::V0(b) => b != 0,
						_ => false,
					};
					self.aggressive(id, on);
					flags = true;
				}
				Key::Armor => extra.push(horse_armor_1_14(id, &value)),
				Key::Gone => {}
			}
		}
		if flags {
			out.push(entry_14(
				13,
				M14::V0(self.mob_flags.get(&id).copied().unwrap_or(0)),
			));
		}
		if pose && name == "player" && id != self.self_id {
			out.push(entry_14(6, M14::V18(self.pose(id))));
		}
		Ok(out)
	}

	fn metadata_1_13_2(
		&mut self,
		id: i32,
		items: Vec<E14>,
		extra: &mut Vec<Client13>,
	) -> Result<Vec<E13>> {
		let name = self.down_types.get(&id).copied().unwrap_or("");
		let mut out = Vec::with_capacity(items.len() + 1);
		for e in items {
			let k = e.key;
			let replaced = match (name, k) {
				("pillager" | "ravager", 14..) | ("fox" | "panda", 15..) | ("cat", 18..) => true,
				("player", 12) => {
					if let M14::V10(Some(location)) = &e.value {
						extra.push(Client13::Bed(v1_13_2::play_to_client_packet_bed::Ty {
							entity_id: id,
							location: pos13(location.clone()),
						}));
					}
					true
				}
				_ => false,
			};
			if replaced {
				continue;
			}
			if is_mob(name) && k == 13 {
				let M14::V0(b) = e.value else { continue };
				out.push(entry_13(11, M13::V0(b & !4)));
				let on = b & 4 != 0;
				match aggressive_key(name) {
					Some(14) => out.push(entry_13(14, M13::V7(on))),
					Some(12) if matches!(name, "skeleton" | "stray" | "wither_skeleton") => {
						out.push(entry_13(12, M13::V7(on)))
					}
					Some(key) => out.push(entry_13(key, M13::V0(on as i8))),
					None => {}
				}
				continue;
			}
			let Some(key) = key_1_13_2(name, k) else {
				continue;
			};
			let value = match (name, key, e.value) {
				(n, 9, M14::V1(s)) if n.ends_with("minecart") => M14::V1(state(NEW, OLD, s)),
				("firework_rocket", 7, M14::V17(v)) => M14::V1((v - 1).max(0)),
				("cat", 15, M14::V1(v)) => M14::V1(ocelot_variant(v)),
				(_, _, M14::V16(d)) => M14::V1(profession_1_13_2(d.villager_profession)),
				(_, _, v) => v,
			};
			if let Some(value) = value_1_13_2(value) {
				out.push(entry_13(key, value));
			}
		}
		Ok(out)
	}
}

pub(super) fn is_living(name: &str) -> bool {
	!name.is_empty()
		&& !name.ends_with("minecart")
		&& !matches!(
			name,
			"area_effect_cloud"
				| "arrow" | "boat"
				| "dragon_fireball"
				| "end_crystal"
				| "evoker_fangs"
				| "experience_orb"
				| "eye_of_ender"
				| "falling_block"
				| "firework_rocket"
				| "item" | "item_frame"
				| "fireball" | "leash_knot"
				| "llama_spit"
				| "painting" | "tnt"
				| "shulker_bullet"
				| "small_fireball"
				| "snowball" | "spectral_arrow"
				| "egg" | "ender_pearl"
				| "experience_bottle"
				| "potion" | "trident"
				| "wither_skull"
				| "lightning_bolt"
				| "fishing_bobber"
		)
}

fn is_mob(name: &str) -> bool {
	is_living(name) && !matches!(name, "player" | "armor_stand")
}

const ZOMBIES: &[&str] = &[
	"zombie",
	"husk",
	"drowned",
	"zombie_pigman",
	"zombie_villager",
];

fn aggressive_key(name: &str) -> Option<u8> {
	match name {
		n if ZOMBIES.contains(&n) => Some(14),
		"skeleton" | "stray" | "wither_skeleton" | "vindicator" | "evoker" | "illusioner" => {
			Some(12)
		}
		_ => None,
	}
}

#[derive(PartialEq)]
enum Key {
	To(u8),
	Aggressive,
	Armor,
	Gone,
}

fn key_1_14(name: &str, k: u8) -> Key {
	if k <= 5 {
		return Key::To(k);
	}
	if !is_living(name) {
		return match name {
			"arrow" | "spectral_arrow" | "trident" if k >= 8 => Key::To(k + 2),
			_ => Key::To(k + 1),
		};
	}
	if k <= 10 {
		return Key::To(k + 1);
	}
	if is_mob(name) && k == 11 {
		return Key::To(13);
	}
	if aggressive_key(name) == Some(k) {
		return Key::Aggressive;
	}
	match (name, k) {
		(n, 15..) if ZOMBIES.contains(&n) => Key::To(k + 1),
		("witch", 12) => Key::To(15),
		("horse", 16) => Key::Armor,
		("ocelot", 13 | 14 | 15) => Key::Gone,
		_ => Key::To(k + 2),
	}
}

fn key_1_13_2(name: &str, k: u8) -> Option<u8> {
	match (name, k) {
		(
			"snowball" | "egg" | "ender_pearl" | "experience_bottle" | "eye_of_ender" | "fireball"
			| "small_fireball",
			7,
		)
		| ("firework_rocket", 9) => None,
		_ => (0..=k).find(|old| key_1_14(name, *old) == Key::To(k)),
	}
}

fn villager_data(profession: i32) -> M14 {
	let profession = match profession {
		0 => 5,
		1 => 9,
		2 => 4,
		3 => 1,
		4 => 2,
		5 => 11,
		_ => 0,
	};
	M14::V16(v1_14::entity_metadata_item_v16::Ty {
		villager_type: 2,
		villager_profession: profession,
		level: 1,
	})
}

fn profession_1_13_2(profession: i32) -> i32 {
	match profession {
		1 | 10 | 13 | 14 => 3,
		2 | 8 => 4,
		3 | 9 => 1,
		4 => 2,
		5 | 6 | 7 | 12 => 0,
		_ => 5,
	}
}

fn ocelot_variant(cat: i32) -> i32 {
	match cat {
		0..=3 => cat,
		10 => 1,
		5 | 6 => 2,
		4 | 7 | 8 | 9 => 3,
		_ => 0,
	}
}

const REPLACED: &[(&str, &str)] = &[
	("fox", "Fox"),
	("panda", "Panda"),
	("trader_llama", "Trader Llama"),
	("pillager", "Pillager"),
	("wandering_trader", "Wandering Trader"),
	("ravager", "Ravager"),
];

fn replacement_name(name: &str, metadata: &[E13]) -> Option<E13> {
	let (_, display) = REPLACED.iter().find(|(n, _)| *n == name)?;
	(!metadata.iter().any(|e| e.key == 2)).then(|| {
		entry_13(
			2,
			M13::V5(Some(serde_json::json!({ "text": display }).to_string())),
		)
	})
}

fn horse_armor_1_14(id: i32, value: &M14) -> Client14 {
	let name = match value {
		M14::V1(1) => "iron_horse_armor",
		M14::V1(2) => "golden_horse_armor",
		M14::V1(3) => "diamond_horse_armor",
		_ => "",
	};
	let item = item_id(NEW, name).map_or(Slot::False, |item_id| {
		Slot::True(v1_14::slot_true::Ty {
			item_id,
			item_count: 1,
			nbt_data: None,
		})
	});
	Client14::EntityEquipment(v1_14::play_to_client_packet_entity_equipment::Ty {
		entity_id: id,
		slot: 4,
		item,
	})
}

fn type_14(v: &M14) -> i32 {
	match v {
		M14::V0(_) => 0,
		M14::V1(_) => 1,
		M14::V2(_) => 2,
		M14::V3(_) => 3,
		M14::V4(_) => 4,
		M14::V5(_) => 5,
		M14::V6(_) => 6,
		M14::V7(_) => 7,
		M14::V8(_) => 8,
		M14::V9(_) => 9,
		M14::V10(_) => 10,
		M14::V11(_) => 11,
		M14::V12(_) => 12,
		M14::V13(_) => 13,
		M14::V14(_) => 14,
		M14::V15(_) => 15,
		M14::V16(_) => 16,
		M14::V17(_) => 17,
		M14::V18(_) => 18,
		M14::Default(t) => *t,
	}
}

pub(super) fn entry_14(key: u8, value: M14) -> E14 {
	E14 {
		key,
		r#type: type_14(&value),
		value,
	}
}

fn entry_13(key: u8, value: M13) -> E13 {
	let ty = match &value {
		M13::V0(_) => 0,
		M13::V1(_) => 1,
		M13::V2(_) => 2,
		M13::V3(_) => 3,
		M13::V4(_) => 4,
		M13::V5(_) => 5,
		M13::V6(_) => 6,
		M13::V7(_) => 7,
		M13::V8(_) => 8,
		M13::V9(_) => 9,
		M13::V10(_) => 10,
		M13::V11(_) => 11,
		M13::V12(_) => 12,
		M13::V13(_) => 13,
		M13::V14(_) => 14,
		M13::V15(_) => 15,
		M13::Default(t) => *t,
	};
	E13 {
		key,
		r#type: ty,
		value,
	}
}

fn pos14(p: v1_13_2::position::Ty) -> v1_14::position::Ty {
	v1_14::position::Ty {
		x: p.x,
		y: p.y,
		z: p.z,
	}
}

fn pos13(p: v1_14::position::Ty) -> v1_13_2::position::Ty {
	v1_13_2::position::Ty {
		x: p.x,
		y: p.y,
		z: p.z,
	}
}

fn value_1_14(v: M13) -> Result<M14> {
	Ok(match v {
		M13::V0(b) => M14::V0(b),
		M13::V1(n) => M14::V1(n),
		M13::V2(f) => M14::V2(f),
		M13::V3(s) => M14::V3(s),
		M13::V4(s) => M14::V4(s),
		M13::V5(s) => M14::V5(s),
		M13::V6(s) => M14::V6(slot14(s)),
		M13::V7(b) => M14::V7(b),
		M13::V8(r) => M14::V8(v1_14::entity_metadata_item_v8::Ty {
			pitch: r.pitch,
			yaw: r.yaw,
			roll: r.roll,
		}),
		M13::V9(p) => M14::V9(pos14(p)),
		M13::V10(p) => M14::V10(p.map(pos14)),
		M13::V11(d) => M14::V11(d),
		M13::V12(u) => M14::V12(u),
		M13::V13(s) => M14::V13(state(OLD, NEW, s)),
		M13::V14(n) => M14::V14(n),
		M13::V15(p) => M14::V15(particle_1_14(p)?),
		M13::Default(t) => M14::Default(t),
	})
}

fn value_1_13_2(v: M14) -> Option<M13> {
	Some(match v {
		M14::V0(b) => M13::V0(b),
		M14::V1(n) => M13::V1(n),
		M14::V2(f) => M13::V2(f),
		M14::V3(s) => M13::V3(s),
		M14::V4(s) => M13::V4(s),
		M14::V5(s) => M13::V5(s),
		M14::V6(s) => M13::V6(slot13(s)),
		M14::V7(b) => M13::V7(b),
		M14::V8(r) => M13::V8(v1_13_2::entity_metadata_item_v8::Ty {
			pitch: r.pitch,
			yaw: r.yaw,
			roll: r.roll,
		}),
		M14::V9(p) => M13::V9(pos13(p)),
		M14::V10(p) => M13::V10(p.map(pos13)),
		M14::V11(d) => M13::V11(d),
		M14::V12(u) => M13::V12(u),
		M14::V13(s) => M13::V13(state(NEW, OLD, s)),
		M14::V14(n) => M13::V14(n),
		M14::V15(p) => M13::V15(particle_1_13_2(p).ok()?),
		M14::V16(_) | M14::V17(_) | M14::V18(_) => return None,
		M14::Default(t) => M13::Default(t),
	})
}

fn particle_data_1_14(id: i32, data: P13) -> Result<(i32, P14)> {
	let id = remap("particles", OLD, NEW, id).ok_or(Error::Dropped)?;
	let data = match data {
		P13::V3(s) => P14::V3(state(OLD, NEW, s)),
		P13::V11(d) => P14::V14(v1_14::particle_data_v14::Ty {
			red: d.red,
			green: d.green,
			blue: d.blue,
			scale: d.scale,
		}),
		P13::V20(s) => P14::V23(state(OLD, NEW, s)),
		P13::V27(s) => P14::V32(slot14(s)),
		P13::Default(_) => P14::Default(id),
	};
	Ok((id, data))
}

fn particle_data_1_13_2(id: i32, data: P14) -> Result<(i32, P13)> {
	let id = remap("particles", NEW, OLD, id).ok_or(Error::Dropped)?;
	let data = match data {
		P14::V3(s) => P13::V3(state(NEW, OLD, s)),
		P14::V14(d) => P13::V11(v1_13_2::particle_data_v11::Ty {
			red: d.red,
			green: d.green,
			blue: d.blue,
			scale: d.scale,
		}),
		P14::V23(s) => P13::V20(state(NEW, OLD, s)),
		P14::V32(s) => P13::V27(slot13(s)),
		P14::Default(_) => P13::Default(id),
	};
	Ok((id, data))
}

fn particle_1_14(p: v1_13_2::particle::Ty) -> Result<v1_14::particle::Ty> {
	let (particle_id, data) = particle_data_1_14(p.particle_id, p.data)?;
	Ok(v1_14::particle::Ty { particle_id, data })
}

fn particle_1_13_2(p: v1_14::particle::Ty) -> Result<v1_13_2::particle::Ty> {
	let (particle_id, data) = particle_data_1_13_2(p.particle_id, p.data)?;
	Ok(v1_13_2::particle::Ty { particle_id, data })
}

pub fn village_world_particles_1_14(
	p: v1_13_2::play_to_client_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<v1_14::play_to_client_packet_world_particles::Ty> {
	let (particle_id, data) = particle_data_1_14(p.particle_id, p.data)?;
	Ok(v1_14::play_to_client_packet_world_particles::Ty {
		particle_id,
		long_distance: p.long_distance,
		x: p.x,
		y: p.y,
		z: p.z,
		offset_x: p.offset_x,
		offset_y: p.offset_y,
		offset_z: p.offset_z,
		particle_data: p.particle_data,
		particles: p.particles,
		data,
	})
}

pub fn village_world_particles_1_13_2(
	p: v1_14::play_to_client_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<v1_13_2::play_to_client_packet_world_particles::Ty> {
	let (particle_id, data) = particle_data_1_13_2(p.particle_id, p.data)?;
	Ok(v1_13_2::play_to_client_packet_world_particles::Ty {
		particle_id,
		long_distance: p.long_distance,
		x: p.x,
		y: p.y,
		z: p.z,
		offset_x: p.offset_x,
		offset_y: p.offset_y,
		offset_z: p.offset_z,
		particle_data: p.particle_data,
		particles: p.particles,
		data,
	})
}

type Node = v1_13_2::command_node::Ty;

fn rename_parser(mut node: Node, up: bool) -> Node {
	use v1_13_2::command_node_extra_node_data::Ty as Extra;
	use v1_13_2::command_node_extra_node_data_v2_properties::Ty as Parser;
	use v1_13_2::command_node_extra_node_data_v2_properties_brigadier_integer as Integer;
	let Extra::V2(arg) = &mut node.extra_node_data else {
		return node;
	};
	let (parser, properties) = match (up, &arg.properties) {
		(true, Parser::MinecraftNbt) => ("minecraft:nbt_compound_tag", Parser::MinecraftNbtCompoundTag),
		(false, Parser::MinecraftNbtCompoundTag) => ("minecraft:nbt", Parser::MinecraftNbt),
		(false, Parser::Default(p)) if p == "minecraft:nbt_tag" => (
			"brigadier:string",
			Parser::BrigadierString(v1_13_2::command_node_extra_node_data_v2_properties_brigadier_string::Ty::GreedyPhrase),
		),
		(false, Parser::MinecraftTime) => (
			"brigadier:integer",
			Parser::BrigadierInteger(Integer::Ty {
				flags: v1_13_2::command_node_extra_node_data_v2_properties_brigadier_integer_flags::Ty {
					unused: 0,
					max_present: 0,
					min_present: 1,
				},
				min: v1_13_2::command_node_extra_node_data_v2_properties_brigadier_integer_min::Ty::V1(0),
				max: v1_13_2::command_node_extra_node_data_v2_properties_brigadier_integer_max::Ty::Default(0),
			}),
		),
		_ => return node,
	};
	arg.parser = parser.into();
	arg.properties = properties;
	node
}

pub fn village_command_node_1_14(node: Node, _cx: &mut Session) -> Result<Node> {
	Ok(rename_parser(node, true))
}

pub fn village_command_node_1_13_2(node: Node, _cx: &mut Session) -> Result<Node> {
	Ok(rename_parser(node, false))
}

type Explosion = v1_13_2::play_to_client_packet_explosion::Ty;

pub fn village_explosion_1_14(p: Explosion, _cx: &mut Session) -> Result<Explosion> {
	let origin = |v: f32| if v < 0.0 { v.trunc() } else { v };
	Ok(Explosion {
		x: origin(p.x),
		y: origin(p.y),
		z: origin(p.z),
		..p
	})
}

pub fn village_explosion_1_13_2(p: Explosion, _cx: &mut Session) -> Result<Explosion> {
	let origin = |v: f32| if v < 0.0 { v.floor() } else { v };
	Ok(Explosion {
		x: origin(p.x),
		y: origin(p.y),
		z: origin(p.z),
		..p
	})
}

fn difficulty_14(d: u8) -> v1_14::play_to_client_packet_difficulty_difficulty::Ty {
	use v1_14::play_to_client_packet_difficulty_difficulty::Ty as D;
	match d {
		0 => D::Peaceful,
		1 => D::Easy,
		2 => D::Normal,
		3 => D::Hard,
		n => D::Default(n),
	}
}

fn difficulty_packet_14(d: u8) -> Client14 {
	Client14::Difficulty(v1_14::play_to_client_packet_difficulty::Ty {
		difficulty: difficulty_14(d),
		difficulty_locked: false,
	})
}

pub fn village_difficulty_1_14(
	d: v1_13_2::play_to_client_packet_difficulty::Ty,
	_cx: &mut Session,
) -> Result<v1_14::play_to_client_packet_difficulty::Ty> {
	Ok(v1_14::play_to_client_packet_difficulty::Ty {
		difficulty: difficulty_14(v1_13_2::play_to_client_packet_difficulty::virtual_tag(&d)),
		difficulty_locked: false,
	})
}

pub fn village_difficulty_1_13_2(
	d: v1_14::play_to_client_packet_difficulty::Ty,
	cx: &mut Session,
) -> Result<v1_13_2::play_to_client_packet_difficulty::Ty> {
	use v1_13_2::play_to_client_packet_difficulty::Ty as D;
	let value = v1_14::play_to_client_packet_difficulty_difficulty::virtual_tag(&d.difficulty);
	cx.state::<State14>().difficulty = value;
	Ok(match value {
		0 => D::Peaceful,
		1 => D::Easy,
		2 => D::Normal,
		3 => D::Hard,
		n => D::Default(n),
	})
}

pub fn village_login_1_14(
	p: v1_13_2::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	cx.state::<State14>().reset(p.entity_id, p.dimension);
	Ok(vec![
		Client14::Login(v1_14::play_to_client_packet_login::Ty {
			entity_id: p.entity_id,
			game_mode: p.game_mode,
			dimension: p.dimension,
			max_players: p.max_players,
			level_type: p.level_type,
			view_distance: VIEW_DISTANCE,
			reduced_debug_info: p.reduced_debug_info,
		}),
		Client14::UpdateViewDistance(VIEW_DISTANCE),
		difficulty_packet_14(p.difficulty),
	])
}

pub fn village_login_1_13_2(
	p: v1_14::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let st = cx.state::<State14>();
	st.reset(p.entity_id, p.dimension);
	Ok(vec![Client13::Login(
		v1_13_2::play_to_client_packet_login::Ty {
			entity_id: p.entity_id,
			game_mode: p.game_mode,
			dimension: p.dimension,
			difficulty: st.difficulty,
			max_players: p.max_players,
			level_type: p.level_type,
			reduced_debug_info: p.reduced_debug_info,
		},
	)])
}

pub fn village_respawn_1_14(
	p: v1_13_2::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	let st = cx.state::<State14>();
	let id = st.self_id;
	st.reset(id, p.dimension);
	Ok(vec![
		Client14::Respawn(v1_14::play_to_client_packet_respawn::Ty {
			dimension: p.dimension,
			gamemode: p.gamemode,
			level_type: p.level_type,
		}),
		difficulty_packet_14(p.difficulty),
		Client14::UpdateViewDistance(VIEW_DISTANCE),
	])
}

pub fn village_respawn_1_13_2(
	p: v1_14::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let st = cx.state::<State14>();
	let id = st.self_id;
	st.reset(id, p.dimension);
	Ok(vec![Client13::Respawn(
		v1_13_2::play_to_client_packet_respawn::Ty {
			dimension: p.dimension,
			difficulty: st.difficulty,
			gamemode: p.gamemode,
			level_type: p.level_type,
		},
	)])
}

pub fn village_position_1_14(
	p: v1_13_2::play_to_client_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	let st = cx.state::<State14>();
	let (ox, oz) = st.pos.unwrap_or((0.0, 0.0));
	let x = if p.flags & 0x01 != 0 { ox + p.x } else { p.x };
	let z = if p.flags & 0x04 != 0 { oz + p.z } else { p.z };
	st.pos = Some((x, z));
	let mut out = Vec::new();
	st.recenter(None, &mut out);
	out.push(Client14::Position(p));
	Ok(out)
}

pub fn village_position_1_13_2(
	p: v1_14::play_to_server_packet_position::Ty,
	cx: &mut Session,
) -> Result<v1_13_2::play_to_server_packet_position::Ty> {
	cx.state::<State14>().pos = Some((p.x, p.z));
	Ok(p)
}

pub fn village_position_look_1_13_2(
	p: v1_14::play_to_server_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<v1_13_2::play_to_server_packet_position_look::Ty> {
	cx.state::<State14>().pos = Some((p.x, p.z));
	Ok(p)
}

pub fn village_vehicle_move_1_13_2(
	p: v1_14::play_to_server_packet_vehicle_move::Ty,
	cx: &mut Session,
) -> Result<v1_13_2::play_to_server_packet_vehicle_move::Ty> {
	cx.state::<State14>().pos = Some((p.x, p.z));
	Ok(p)
}

pub struct VillageColumn {
	pub sections: Vec<VillageSection>,
	pub counts: Vec<i16>,
	pub heightmaps: Nbt,
	pub light: v1_14::play_to_client_packet_update_light::Ty,
	pub before: Vec<Client14>,
}

pub fn village_column_1_14(
	cx: &mut Session,
	x: i32,
	z: i32,
	full: bool,
	mut sections: Vec<VillageSection>,
) -> Result<VillageColumn> {
	let table = registry::state_table(OLD, NEW)
		.ok_or_else(|| Error::Unconvertible("no 1.13.2 -> 1.14 block states".into()))?;
	let (mut surface, mut motion) = ([0i64; 256], [0i64; 256]);
	let mut counts = Vec::with_capacity(sections.len());
	for sec in &mut sections {
		let mut count = 0i16;
		for (i, s) in sec.states.iter_mut().enumerate() {
			*s = table.get(*s as usize).copied().unwrap_or(0);
			let f = flag(*s);
			let top = (sec.y * 16 + i / 256 + 1) as i64;
			if f & SOLID != 0 {
				count += 1;
				surface[i & 255] = top;
			}
			if f & MOTION != 0 {
				motion[i & 255] = top;
			}
		}
		counts.push(count);
	}
	fix_light(&mut sections);
	let light = light_1_14(x, z, full, &sections, &surface);
	let mut before = Vec::new();
	cx.state::<State14>().recenter(Some((x, z)), &mut before);
	Ok(VillageColumn {
		sections,
		counts,
		heightmaps: heightmaps(full, &motion, &surface),
		light,
		before,
	})
}

pub fn village_map_chunk_1_14(
	p: v1_13_2::play_to_client_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	let sky = !cx.state::<State14>().no_sky;
	let (sections, biomes) = sections_1_13_2(&p.chunk_data, p.bit_map, p.ground_up, sky)
		.or_else(|e| sections_1_13_2(&p.chunk_data, p.bit_map, p.ground_up, !sky).map_err(|_| e))?;
	let column = village_column_1_14(cx, p.x, p.z, p.ground_up, sections)?;
	let mut data = Vec::new();
	for (sec, count) in column.sections.iter().zip(&column.counts) {
		data.extend(count.to_be_bytes());
		write_states(&mut data, &sec.states);
	}
	data.extend(&biomes);
	let mut out = column.before;
	out.push(Client14::UpdateLight(column.light));
	out.push(Client14::MapChunk(
		v1_14::play_to_client_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			ground_up: p.ground_up,
			bit_map: p.bit_map,
			heightmaps: column.heightmaps,
			chunk_data: data,
			block_entities: p.block_entities,
		},
	));
	Ok(out)
}

pub fn village_update_light_1_13_2(
	p: v1_14::play_to_client_packet_update_light::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let entry = cx
		.state::<State14>()
		.light
		.entry((p.chunk_x, p.chunk_z))
		.or_default();
	entry.sky.resize(18, None);
	entry.block.resize(18, None);
	let mut r = Bytes::new(&p.data);
	for (mask, empty, arrays) in [
		(p.sky_light_mask, p.empty_sky_light_mask, &mut entry.sky),
		(
			p.block_light_mask,
			p.empty_block_light_mask,
			&mut entry.block,
		),
	] {
		for (i, a) in arrays.iter_mut().enumerate() {
			if mask & 1 << i != 0 {
				let n = r.varint()? as usize;
				*a = Some(r.take(n)?.to_vec());
			} else if empty & 1 << i != 0 {
				*a = Some(vec![0; 2048]);
			}
		}
	}
	Ok(vec![])
}

pub fn village_unload_chunk_1_13_2(
	p: v1_14::play_to_client_packet_unload_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	cx.state::<State14>().light.remove(&(p.chunk_x, p.chunk_z));
	Ok(vec![Client13::UnloadChunk(p)])
}

pub fn village_section_light_1_13_2(
	cx: &mut Session,
	x: i32,
	z: i32,
	ys: &[usize],
) -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
	let st = cx.state::<State14>();
	let light = st.light.remove(&(x, z)).unwrap_or_default();
	let array = |arrays: &[Option<Vec<u8>>], y: usize, fill: u8| {
		arrays
			.get(y + 1)
			.cloned()
			.flatten()
			.filter(|a| a.len() == 2048)
			.unwrap_or_else(|| vec![fill; 2048])
	};
	ys.iter()
		.map(|y| {
			(
				array(&light.block, *y, 0),
				(!st.no_sky).then(|| array(&light.sky, *y, 0xff)),
			)
		})
		.collect()
}

pub fn village_map_chunk_1_13_2(
	p: v1_14::play_to_client_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let ys: Vec<usize> = (0..16).filter(|y| p.bit_map & 1 << y != 0).collect();
	let light = village_section_light_1_13_2(cx, p.x, p.z, &ys);
	let table = registry::state_table(NEW, OLD)
		.ok_or_else(|| Error::Unconvertible("no 1.14 -> 1.13.2 block states".into()))?;
	let mut r = Bytes::new(&p.chunk_data);
	let mut data = Vec::new();
	for (block, sky) in light {
		r.take(2)?;
		let mut states = read_states(&mut r)?;
		for s in &mut states {
			*s = table.get(*s as usize).copied().unwrap_or(0);
		}
		write_states(&mut data, &states);
		data.extend(block);
		if let Some(sky) = sky {
			data.extend(sky);
		}
	}
	if p.ground_up {
		data.extend(r.take(1024)?);
	}
	if !r.rest().is_empty() {
		return unconvertible(format!(
			"{} bytes follow the chunk sections",
			r.rest().len()
		));
	}
	Ok(vec![Client13::MapChunk(
		v1_13_2::play_to_client_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			ground_up: p.ground_up,
			bit_map: p.bit_map,
			chunk_data: data,
			block_entities: p.block_entities,
		},
	)])
}

const MINECARTS: [&str; 7] = [
	"minecart",
	"chest_minecart",
	"furnace_minecart",
	"tnt_minecart",
	"spawner_minecart",
	"hopper_minecart",
	"command_block_minecart",
];

fn object_1_14(ty: i8, data: i32) -> Option<&'static str> {
	match registry::names(OLD, "objects")?.name(ty as u8 as u32)? {
		"minecart" => Some(
			usize::try_from(data)
				.ok()
				.and_then(|d| MINECARTS.get(d))
				.copied()
				.unwrap_or("minecart"),
		),
		name => Some(name),
	}
}

fn object_1_13_2(name: &str) -> Option<(i8, Option<i32>)> {
	let objects = registry::names(OLD, "objects")?;
	match MINECARTS.iter().position(|m| *m == name) {
		Some(variant) => Some((objects.id("minecart")? as i8, Some(variant as i32))),
		None => Some((objects.id(name)? as i8, None)),
	}
}

fn is_arrow(name: &str) -> bool {
	matches!(name, "arrow" | "spectral_arrow" | "trident")
}

pub fn village_spawn_entity_1_14(
	p: v1_13_2::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	let Some(name) = object_1_14(p.r#type, p.object_data) else {
		return Ok(vec![]);
	};
	let Some(ty) = entity_id(NEW, name) else {
		return Ok(vec![]);
	};
	cx.state::<State14>().types.insert(p.entity_id, name);
	let data = match name {
		"falling_block" => state(OLD, NEW, p.object_data),
		n if is_arrow(n) => p.object_data - 1,
		_ => p.object_data,
	};
	let velocity = (is_arrow(name) || name == "item" && p.object_data > 0).then(|| {
		Client14::EntityVelocity(v1_14::play_to_client_packet_entity_velocity::Ty {
			entity_id: p.entity_id,
			velocity: p.velocity.clone(),
		})
	});
	let mut out = vec![Client14::SpawnEntity(
		v1_14::play_to_client_packet_spawn_entity::Ty {
			entity_id: p.entity_id,
			object_uuid: p.object_uuid,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			pitch: p.pitch,
			yaw: p.yaw,
			object_data: data,
			velocity: p.velocity,
		},
	)];
	out.extend(velocity);
	Ok(out)
}

pub fn village_spawn_entity_1_13_2(
	p: v1_14::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let name = entity_name(NEW, p.r#type);
	let Some((ty, fixed)) = object_1_13_2(name) else {
		return Ok(vec![]);
	};
	let st = cx.state::<State14>();
	st.down_types.insert(p.entity_id, name);
	st.positions.insert(p.entity_id, (p.x, p.y, p.z));
	let data = match name {
		"falling_block" => state(NEW, OLD, p.object_data),
		n if is_arrow(n) => p.object_data + 1,
		_ => fixed.unwrap_or(p.object_data),
	};
	Ok(vec![Client13::SpawnEntity(
		v1_13_2::play_to_client_packet_spawn_entity::Ty {
			entity_id: p.entity_id,
			object_uuid: p.object_uuid,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			pitch: p.pitch,
			yaw: p.yaw,
			object_data: data,
			velocity: p.velocity,
		},
	)])
}

pub fn village_spawn_entity_living_1_14(
	p: v1_13_2::play_to_client_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	let Some(mut ty) = remap("entities", OLD, NEW, p.r#type) else {
		return Ok(vec![]);
	};
	let mut name = entity_name(NEW, ty);
	let tamed = p
		.metadata
		.iter()
		.any(|e| e.key == 15 && matches!(e.value, M13::V1(v) if v != 0));
	if let (true, Some(cat)) = (name == "ocelot" && tamed, entity_id(NEW, "cat")) {
		(ty, name) = (cat, "cat");
	}
	let st = cx.state::<State14>();
	st.forget(p.entity_id);
	st.types.insert(p.entity_id, name);
	let mut extra = Vec::new();
	let metadata = st.metadata_1_14(p.entity_id, p.metadata, &mut extra)?;
	let mut out = vec![Client14::SpawnEntityLiving(
		v1_14::play_to_client_packet_spawn_entity_living::Ty {
			entity_id: p.entity_id,
			entity_uuid: p.entity_uuid,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			head_pitch: p.head_pitch,
			velocity: p.velocity,
			metadata,
		},
	)];
	out.extend(extra);
	Ok(out)
}

pub fn village_spawn_entity_living_1_13_2(
	p: v1_14::play_to_client_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let Some(ty) = remap("entities", NEW, OLD, p.r#type) else {
		return Ok(vec![]);
	};
	let name = entity_name(NEW, p.r#type);
	let st = cx.state::<State14>();
	st.forget(p.entity_id);
	st.down_types.insert(p.entity_id, name);
	st.positions.insert(p.entity_id, (p.x, p.y, p.z));
	let mut extra = Vec::new();
	let mut metadata = st.metadata_1_13_2(p.entity_id, p.metadata, &mut extra)?;
	metadata.extend(replacement_name(name, &metadata));
	let mut out = vec![Client13::SpawnEntityLiving(
		v1_13_2::play_to_client_packet_spawn_entity_living::Ty {
			entity_id: p.entity_id,
			entity_uuid: p.entity_uuid,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			head_pitch: p.head_pitch,
			velocity: p.velocity,
			metadata,
		},
	)];
	out.extend(extra);
	Ok(out)
}

pub fn village_named_entity_spawn_1_14(
	p: v1_13_2::play_to_client_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	let st = cx.state::<State14>();
	st.forget(p.entity_id);
	st.types.insert(p.entity_id, "player");
	let mut extra = Vec::new();
	let metadata = st.metadata_1_14(p.entity_id, p.metadata, &mut extra)?;
	let mut out = vec![Client14::NamedEntitySpawn(
		v1_14::play_to_client_packet_named_entity_spawn::Ty {
			entity_id: p.entity_id,
			player_uuid: p.player_uuid,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			metadata,
		},
	)];
	out.extend(extra);
	Ok(out)
}

pub fn village_named_entity_spawn_1_13_2(
	p: v1_14::play_to_client_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let st = cx.state::<State14>();
	st.forget(p.entity_id);
	st.down_types.insert(p.entity_id, "player");
	st.positions.insert(p.entity_id, (p.x, p.y, p.z));
	let mut extra = Vec::new();
	let metadata = st.metadata_1_13_2(p.entity_id, p.metadata, &mut extra)?;
	let mut out = vec![Client13::NamedEntitySpawn(
		v1_13_2::play_to_client_packet_named_entity_spawn::Ty {
			entity_id: p.entity_id,
			player_uuid: p.player_uuid,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			metadata,
		},
	)];
	out.extend(extra);
	Ok(out)
}

pub fn village_entity_metadata_1_14(
	p: v1_13_2::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	let mut extra = Vec::new();
	let metadata = cx
		.state::<State14>()
		.metadata_1_14(p.entity_id, p.metadata, &mut extra)?;
	let mut out = vec![Client14::EntityMetadata(
		v1_14::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
			metadata,
		},
	)];
	out.extend(extra);
	Ok(out)
}

pub fn village_entity_metadata_1_13_2(
	p: v1_14::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let mut extra = Vec::new();
	let metadata = cx
		.state::<State14>()
		.metadata_1_13_2(p.entity_id, p.metadata, &mut extra)?;
	let mut out = vec![Client13::EntityMetadata(
		v1_13_2::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
			metadata,
		},
	)];
	out.extend(extra);
	Ok(out)
}

pub fn village_entity_equipment_1_13_2(
	p: v1_14::play_to_client_packet_entity_equipment::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let item = slot13(p.item);
	let horse = cx.state::<State14>().down_types.get(&p.entity_id).copied() == Some("horse");
	if !horse {
		return Ok(vec![Client13::EntityEquipment(
			v1_13_2::play_to_client_packet_entity_equipment::Ty {
				entity_id: p.entity_id,
				slot: p.slot,
				item,
			},
		)]);
	}
	let armor = match item_name(OLD, &item) {
		"" => 0,
		"iron_horse_armor" => 1,
		"golden_horse_armor" => 2,
		"diamond_horse_armor" => 3,
		_ => return Ok(vec![]),
	};
	Ok(vec![Client13::EntityMetadata(
		v1_13_2::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
			metadata: vec![entry_13(16, M13::V1(armor))],
		},
	)])
}

pub fn village_entity_status_1_13_2(
	p: v1_14::play_to_client_packet_entity_status::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let st = cx.state::<State14>();
	if p.entity_status != 3 {
		return Ok(vec![Client13::EntityStatus(p)]);
	}
	if p.entity_id == st.self_id {
		return Ok(vec![]);
	}
	let player = st.down_types.get(&p.entity_id).copied() == Some("player");
	let id = p.entity_id;
	let mut out = vec![Client13::EntityStatus(p)];
	if player {
		out.extend((0..6).map(|slot| {
			Client13::EntityEquipment(v1_13_2::play_to_client_packet_entity_equipment::Ty {
				entity_id: id,
				slot,
				item: Slot::False,
			})
		}));
	}
	Ok(out)
}

fn moved(cx: &mut Session, id: i32, to: impl FnOnce((f64, f64, f64)) -> (f64, f64, f64)) {
	let positions = &mut cx.state::<State14>().positions;
	if let Some(at) = positions.get_mut(&id) {
		*at = to(*at);
	}
}

fn relative(d: i16) -> f64 {
	d as f64 / 4096.0
}

pub fn village_entity_teleport_1_13_2(
	p: v1_14::play_to_client_packet_entity_teleport::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	cx.state::<State14>()
		.positions
		.insert(p.entity_id, (p.x, p.y, p.z));
	Ok(vec![Client13::EntityTeleport(p)])
}

pub fn village_rel_entity_move_1_13_2(
	p: v1_14::play_to_client_packet_rel_entity_move::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	moved(cx, p.entity_id, |(x, y, z)| {
		(
			x + relative(p.d_x),
			y + relative(p.d_y),
			z + relative(p.d_z),
		)
	});
	Ok(vec![Client13::RelEntityMove(p)])
}

pub fn village_entity_move_look_1_13_2(
	p: v1_14::play_to_client_packet_entity_move_look::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	moved(cx, p.entity_id, |(x, y, z)| {
		(
			x + relative(p.d_x),
			y + relative(p.d_y),
			z + relative(p.d_z),
		)
	});
	Ok(vec![Client13::EntityMoveLook(p)])
}

pub fn village_entity_sound_effect_1_13_2(
	p: v1_14::play_to_client_packet_entity_sound_effect::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let Some(sound_id) = remap("sounds", NEW, OLD, p.sound_id) else {
		return Ok(vec![]);
	};
	let Some((x, y, z)) = cx.state::<State14>().positions.get(&p.entity_id).copied() else {
		return Ok(vec![]);
	};
	Ok(vec![Client13::SoundEffect(
		v1_13_2::play_to_client_packet_sound_effect::Ty {
			sound_id,
			sound_category: p.sound_category,
			x: (x * 8.0) as i32,
			y: (y * 8.0) as i32,
			z: (z * 8.0) as i32,
			volume: p.volume,
			pitch: p.pitch,
		},
	)])
}

fn self_moved(cx: &mut Session, x: f64, y: f64, z: f64) {
	let st = cx.state::<State14>();
	let id = st.self_id;
	st.positions.insert(id, (x, y, z));
}

pub fn village_player_position_1_14(
	p: v1_13_2::play_to_server_packet_position::Ty,
	cx: &mut Session,
) -> Result<v1_14::play_to_server_packet_position::Ty> {
	self_moved(cx, p.x, p.y, p.z);
	Ok(p)
}

pub fn village_player_position_look_1_14(
	p: v1_13_2::play_to_server_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<v1_14::play_to_server_packet_position_look::Ty> {
	self_moved(cx, p.x, p.y, p.z);
	Ok(p)
}

pub fn village_teleport_1_13_2(
	p: v1_14::play_to_client_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let st = cx.state::<State14>();
	let id = st.self_id;
	let (ox, oy, oz) = st.positions.get(&id).copied().unwrap_or_default();
	let axis = |bit: i8, old: f64, new: f64| if p.flags & bit != 0 { old + new } else { new };
	st.positions
		.insert(id, (axis(1, ox, p.x), axis(2, oy, p.y), axis(4, oz, p.z)));
	Ok(vec![Client13::Position(p)])
}

fn forget_all(ids: &[i32], cx: &mut Session) {
	let st = cx.state::<State14>();
	for id in ids {
		st.forget(*id);
	}
}

pub fn village_entity_destroy_1_14(ids: Vec<i32>, cx: &mut Session) -> Result<Vec<Client14>> {
	forget_all(&ids, cx);
	Ok(vec![Client14::EntityDestroy(ids)])
}

pub fn village_entity_destroy_1_13_2(ids: Vec<i32>, cx: &mut Session) -> Result<Vec<Client13>> {
	forget_all(&ids, cx);
	Ok(vec![Client13::EntityDestroy(ids)])
}

pub fn village_bed_1_14(
	p: v1_13_2::play_to_client_packet_bed::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	let st = cx.state::<State14>();
	st.sleeping.insert(p.entity_id);
	let mut metadata = vec![entry_14(12, M14::V10(Some(pos14(p.location))))];
	if p.entity_id != st.self_id {
		metadata.push(entry_14(6, M14::V18(st.pose(p.entity_id))));
	}
	Ok(vec![Client14::EntityMetadata(
		v1_14::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
			metadata,
		},
	)])
}

pub fn village_animation_1_14(
	p: v1_13_2::play_to_client_packet_animation::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	let id = p.entity_id;
	let wake = p.animation == 2;
	let mut out = vec![Client14::Animation(p)];
	if wake {
		let st = cx.state::<State14>();
		st.sleeping.remove(&id);
		let mut metadata = vec![entry_14(12, M14::V10(None))];
		if id != st.self_id {
			metadata.push(entry_14(6, M14::V18(st.pose(id))));
		}
		out.push(Client14::EntityMetadata(
			v1_14::play_to_client_packet_entity_metadata::Ty {
				entity_id: id,
				metadata,
			},
		));
	}
	Ok(out)
}

fn window_type_1_14(kind: &str, slots: u8) -> Option<i32> {
	Some(match kind.strip_prefix("minecraft:").unwrap_or(kind) {
		"crafting_table" => 11,
		"furnace" => 13,
		"dropper" | "dispenser" => 6,
		"enchanting_table" => 12,
		"brewing_stand" => 10,
		"villager" => 18,
		"beacon" => 8,
		"anvil" => 7,
		"hopper" => 15,
		"shulker_box" => 19,
		_ if (9..=54).contains(&slots) && slots % 9 == 0 => slots as i32 / 9 - 1,
		_ => return None,
	})
}

fn window_type_1_13_2(id: i32) -> Option<(&'static str, u8, Option<&'static str>)> {
	Some(match id {
		0..=5 => ("minecraft:container", (id as u8 + 1) * 9, None),
		6 => ("minecraft:dropper", 9, None),
		7 => ("minecraft:anvil", 0, None),
		8 => ("minecraft:beacon", 1, None),
		9 => ("minecraft:furnace", 3, Some("Blast Furnace")),
		10 => ("minecraft:brewing_stand", 5, None),
		11 => ("minecraft:crafting_table", 0, None),
		12 => ("minecraft:enchanting_table", 0, None),
		13 => ("minecraft:furnace", 3, None),
		14 => ("minecraft:furnace", 3, Some("Grindstone")),
		15 => ("minecraft:hopper", 5, None),
		18 => ("minecraft:villager", 0, None),
		19 => ("minecraft:shulker_box", 27, None),
		20 => ("minecraft:furnace", 3, Some("Smoker")),
		21 => ("minecraft:anvil", 0, Some("Cartography Table")),
		_ => return None,
	})
}

fn window_title_1_13_2(id: i32, title: String, fallback: Option<&str>) -> String {
	let translated = serde_json::from_str::<serde_json::Value>(&title)
		.ok()
		.and_then(|v| {
			v.get("translate")
				.and_then(|t| t.as_str())
				.map(str::to_owned)
		});
	match (id, translated, fallback) {
		(2, Some(key), _) if key == "container.barrel" => {
			serde_json::json!({ "text": "Barrel" }).to_string()
		}
		(_, Some(_), Some(name)) => serde_json::json!({ "text": name }).to_string(),
		_ => title,
	}
}

pub fn village_open_window_1_14(
	p: v1_13_2::play_to_client_packet_open_window::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	use v1_13_2::play_to_client_packet_open_window_entity_id::Ty as Horse;
	if let Horse::EntityHorse(entity_id) = p.entity_id {
		return Ok(vec![Client14::OpenHorseWindow(
			v1_14::play_to_client_packet_open_horse_window::Ty {
				window_id: p.window_id,
				nb_slots: p.slot_count as i32,
				entity_id,
			},
		)]);
	}
	let Some(ty) = window_type_1_14(&p.inventory_type, p.slot_count) else {
		return unconvertible(format!(
			"no 1.14 window for {} with {} slots",
			p.inventory_type, p.slot_count
		));
	};
	if ty == 18 {
		cx.state::<State14>().trade_window = p.window_id;
	}
	Ok(vec![Client14::OpenWindow(
		v1_14::play_to_client_packet_open_window::Ty {
			window_id: p.window_id as i32,
			inventory_type: ty,
			window_title: p.window_title,
		},
	)])
}

pub fn village_open_window_1_13_2(
	p: v1_14::play_to_client_packet_open_window::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let Some((kind, slots, fallback)) = window_type_1_13_2(p.inventory_type) else {
		return Ok(vec![]);
	};
	if kind == "minecraft:villager" {
		cx.state::<State14>().trade_window = p.window_id as u8;
	}
	Ok(vec![Client13::OpenWindow(
		v1_13_2::play_to_client_packet_open_window::Ty {
			window_id: p.window_id as u8,
			entity_id: v1_13_2::play_to_client_packet_open_window_entity_id::Ty::Default(
				kind.to_owned(),
			),
			inventory_type: kind.to_owned(),
			window_title: window_title_1_13_2(p.inventory_type, p.window_title, fallback),
			slot_count: slots,
		},
	)])
}

pub fn village_open_horse_window_1_13_2(
	p: v1_14::play_to_client_packet_open_horse_window::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client13>> {
	Ok(vec![Client13::OpenWindow(
		v1_13_2::play_to_client_packet_open_window::Ty {
			window_id: p.window_id,
			inventory_type: "EntityHorse".to_owned(),
			window_title: "{\"translate\":\"entity.minecraft.horse\"}".to_owned(),
			slot_count: p.nb_slots as u8,
			entity_id: v1_13_2::play_to_client_packet_open_window_entity_id::Ty::EntityHorse(
				p.entity_id,
			),
		},
	)])
}

fn bad(e: crate::pds::Error) -> Error {
	Error::Unconvertible(format!("malformed payload: {e:?}"))
}

fn trade_list_1_14(
	data: &[u8],
	cx: &mut Session,
) -> Result<v1_14::play_to_client_packet_trade_list::Ty> {
	let mut input = Input::new(data);
	let window_id = pds_i32::deserialize(&mut input).map_err(bad)?;
	let count = pds_u8::deserialize(&mut input).map_err(bad)?;
	let slot = |input: &mut Input| v1_13_2::slot::deserialize(input).map(slot14).map_err(bad);
	let trades = (0..count)
		.map(|_| {
			let input_item1 = slot(&mut input)?;
			let output_item = slot(&mut input)?;
			let input_item2 = match pds_bool::deserialize(&mut input).map_err(bad)? {
				true => Some(slot(&mut input)?),
				false => None,
			};
			Ok(v1_14::play_to_client_packet_trade_list_trades_items::Ty {
				input_item1,
				output_item,
				input_item2,
				trade_disabled: pds_bool::deserialize(&mut input).map_err(bad)?,
				nb_trade_uses: pds_i32::deserialize(&mut input).map_err(bad)?,
				maximum_nb_trade_uses: pds_i32::deserialize(&mut input).map_err(bad)?,
				xp: 0,
				special_price: 0,
				price_multiplier: 0.0,
			})
		})
		.collect::<Result<Vec<_>>>()?;
	cx.state::<State14>().trade_window = window_id as u8;
	Ok(v1_14::play_to_client_packet_trade_list::Ty {
		window_id,
		trades,
		villager_level: 0,
		experience: 0,
		is_regular_villager: false,
	})
}

pub fn village_custom_payload_1_14(
	p: v1_13_2::play_to_client_packet_custom_payload::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	Ok(vec![
		match p.channel.strip_prefix("minecraft:").unwrap_or(&p.channel) {
			"trader_list" => Client14::TradeList(trade_list_1_14(&p.data, cx)?),
			"book_open" => {
				Client14::OpenBook(pds_varint::deserialize(&mut Input::new(&p.data)).map_err(bad)?)
			}
			_ => Client14::CustomPayload(p),
		},
	])
}

fn payload_13(channel: &str, data: Vec<u8>) -> Client13 {
	Client13::CustomPayload(v1_13_2::play_to_client_packet_custom_payload::Ty {
		channel: channel.to_owned(),
		data,
	})
}

pub fn village_open_book_1_13_2(hand: i32, _cx: &mut Session) -> Result<Vec<Client13>> {
	let mut o = Output::new();
	pds_varint::serialize(&hand, &mut o).map_err(bad)?;
	Ok(vec![payload_13("minecraft:book_open", o.into_inner())])
}

pub fn village_trade_list_1_13_2(
	p: v1_14::play_to_client_packet_trade_list::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client13>> {
	let mut o = Output::new();
	pds_i32::serialize(&p.window_id, &mut o).map_err(bad)?;
	pds_u8::serialize(&(p.trades.len() as u8), &mut o).map_err(bad)?;
	for t in p.trades {
		v1_13_2::slot::serialize(&slot13(t.input_item1), &mut o).map_err(bad)?;
		v1_13_2::slot::serialize(&slot13(t.output_item), &mut o).map_err(bad)?;
		pds_bool::serialize(&t.input_item2.is_some(), &mut o).map_err(bad)?;
		if let Some(s) = t.input_item2 {
			v1_13_2::slot::serialize(&slot13(s), &mut o).map_err(bad)?;
		}
		pds_bool::serialize(&t.trade_disabled, &mut o).map_err(bad)?;
		pds_i32::serialize(&t.nb_trade_uses, &mut o).map_err(bad)?;
		pds_i32::serialize(&t.maximum_nb_trade_uses, &mut o).map_err(bad)?;
	}
	Ok(vec![payload_13("minecraft:trader_list", o.into_inner())])
}

pub fn village_select_trade_1_13_2(slot: i32, cx: &mut Session) -> Result<Vec<Server13>> {
	let st = cx.state::<State14>();
	st.actions = st.actions.wrapping_add(1);
	let stone = item_id(OLD, "stone").unwrap_or(1);
	let resync = Nbt::new(
		"",
		Tag::Compound(vec![Nbt::new("force_resync", Tag::Double(f64::NAN))]),
	);
	Ok(vec![
		Server13::SelectTrade(slot),
		Server13::WindowClick(v1_13_2::play_to_server_packet_window_click::Ty {
			window_id: st.trade_window,
			slot: -999,
			mouse_button: 2,
			action: i16::MIN.wrapping_add(st.actions),
			mode: 5,
			item: Slot::True(v1_13_2::slot_true::Ty {
				item_id: stone,
				item_count: 1,
				nbt_data: Some(resync),
			}),
		}),
	])
}

type Tags13 = Vec<v1_13_2::play_to_client_packet_tags_block_tags_items::Ty>;
type Tags14 = Vec<v1_14::play_to_client_packet_tags_block_tags_items::Ty>;

fn tags_up(tags: Tags13, f: impl Fn(i32) -> Option<i32>) -> Tags14 {
	tags.into_iter()
		.map(|t| v1_14::play_to_client_packet_tags_block_tags_items::Ty {
			tag_name: t.tag_name,
			entries: t.entries.into_iter().filter_map(&f).collect(),
		})
		.collect()
}

fn tags_down(tags: Tags14, f: impl Fn(i32) -> Option<i32>) -> Tags13 {
	tags.into_iter()
		.map(
			|t| v1_13_2::play_to_client_packet_tags_block_tags_items::Ty {
				tag_name: t.tag_name,
				entries: t.entries.into_iter().filter_map(&f).collect(),
			},
		)
		.collect()
}

pub fn village_tags_1_14(
	p: v1_13_2::play_to_client_packet_tags::Ty,
	_cx: &mut Session,
) -> Result<v1_14::play_to_client_packet_tags::Ty> {
	Ok(v1_14::play_to_client_packet_tags::Ty {
		block_tags: tags_up(p.block_tags, |id| remap("blocks", OLD, NEW, id)),
		item_tags: tags_up(p.item_tags, |id| remap("items", OLD, NEW, id)),
		fluid_tags: tags_up(p.fluid_tags, Some),
		entity_tags: Vec::new(),
	})
}

pub fn village_tags_1_13_2(
	p: v1_14::play_to_client_packet_tags::Ty,
	_cx: &mut Session,
) -> Result<v1_13_2::play_to_client_packet_tags::Ty> {
	Ok(v1_13_2::play_to_client_packet_tags::Ty {
		block_tags: tags_down(p.block_tags, |id| remap("blocks", NEW, OLD, id)),
		item_tags: tags_down(p.item_tags, |id| remap("items", NEW, OLD, id)),
		fluid_tags: tags_down(p.fluid_tags, Some),
	})
}

type Recipe13 = v1_13_2::play_to_client_packet_declare_recipes_items::Ty;
type Recipe14 = v1_14::play_to_client_packet_declare_recipes_items::Ty;

pub fn village_recipe_1_14(r: Recipe13, _cx: &mut Session) -> Result<Recipe14> {
	use v1_13_2::play_to_client_packet_declare_recipes_items_data::Ty as D13;
	use v1_14::play_to_client_packet_declare_recipes_items_data::Ty as D14;
	let items = |v: Vec<Slot>| v.into_iter().map(slot14).collect::<Vec<_>>();
	let data = match r.data {
		D13::CraftingShaped(s) => {
			D14::MinecraftCraftingShaped(v1_14::play_to_client_packet_declare_recipes_items_data_minecraft_crafting_shaped::Ty {
				width: s.width,
				height: s.height,
				group: s.group,
				ingredients: s.ingredients.into_iter().map(|c| c.into_iter().map(items).collect()).collect(),
				result: slot14(s.result),
			})
		}
		D13::CraftingShapeless(s) => {
			D14::MinecraftCraftingShapeless(v1_14::play_to_client_packet_declare_recipes_items_data_minecraft_crafting_shapeless::Ty {
				group: s.group,
				ingredients: s.ingredients.into_iter().map(items).collect(),
				result: slot14(s.result),
			})
		}
		D13::Smelting(s) => D14::MinecraftSmelting(v1_14::minecraft_smelting_format::Ty {
			group: s.group,
			ingredient: items(s.ingredient),
			result: slot14(s.result),
			experience: s.experience,
			cook_time: s.cook_time,
		}),
		D13::CraftingSpecialArmordye => D14::MinecraftCraftingSpecialArmordye,
		D13::CraftingSpecialBannerduplicate => D14::MinecraftCraftingSpecialBannerduplicate,
		D13::CraftingSpecialBookcloning => D14::MinecraftCraftingSpecialBookcloning,
		D13::CraftingSpecialFireworkRocket => D14::MinecraftCraftingSpecialFireworkRocket,
		D13::CraftingSpecialFireworkStar => D14::MinecraftCraftingSpecialFireworkStar,
		D13::CraftingSpecialFireworkStarFade => D14::MinecraftCraftingSpecialFireworkStarFade,
		D13::CraftingSpecialMapcloning => D14::MinecraftCraftingSpecialMapcloning,
		D13::CraftingSpecialMapextending => D14::MinecraftCraftingSpecialMapextending,
		D13::CraftingSpecialShielddecoration => D14::MinecraftCraftingSpecialShielddecoration,
		D13::CraftingSpecialShulkerboxcoloring => D14::MinecraftCraftingSpecialShulkerboxcoloring,
		D13::CraftingSpecialTippedarrow => D14::MinecraftCraftingSpecialTippedarrow,
		D13::CraftingSpecialBanneraddpattern | D13::CraftingSpecialRepairitem | D13::Default(_) => {
			return Err(Error::Dropped)
		}
	};
	Ok(Recipe14 {
		r#type: format!("minecraft:{}", r.r#type.trim_start_matches("minecraft:")),
		recipe_id: r.recipe_id,
		data,
	})
}

pub fn village_recipe_1_13_2(r: Recipe14, _cx: &mut Session) -> Result<Recipe13> {
	use v1_13_2::play_to_client_packet_declare_recipes_items_data::Ty as D13;
	use v1_14::play_to_client_packet_declare_recipes_items_data::Ty as D14;
	let items = |v: Vec<Slot>| v.into_iter().map(slot13).collect::<Vec<_>>();
	let data = match r.data {
		D14::MinecraftCraftingShaped(s) => D13::CraftingShaped(
			v1_13_2::play_to_client_packet_declare_recipes_items_data_crafting_shaped::Ty {
				width: s.width,
				height: s.height,
				group: s.group,
				ingredients: s
					.ingredients
					.into_iter()
					.map(|c| c.into_iter().map(items).collect())
					.collect(),
				result: slot13(s.result),
			},
		),
		D14::MinecraftCraftingShapeless(s) => D13::CraftingShapeless(
			v1_13_2::play_to_client_packet_declare_recipes_items_data_crafting_shapeless::Ty {
				group: s.group,
				ingredients: s.ingredients.into_iter().map(items).collect(),
				result: slot13(s.result),
			},
		),
		D14::MinecraftSmelting(s) => D13::Smelting(
			v1_13_2::play_to_client_packet_declare_recipes_items_data_smelting::Ty {
				group: s.group,
				ingredient: items(s.ingredient),
				result: slot13(s.result),
				experience: s.experience,
				cook_time: s.cook_time,
			},
		),
		D14::MinecraftCraftingSpecialArmordye => D13::CraftingSpecialArmordye,
		D14::MinecraftCraftingSpecialBanneraddpattern => D13::CraftingSpecialBanneraddpattern,
		D14::MinecraftCraftingSpecialBannerduplicate => D13::CraftingSpecialBannerduplicate,
		D14::MinecraftCraftingSpecialBookcloning => D13::CraftingSpecialBookcloning,
		D14::MinecraftCraftingSpecialFireworkRocket => D13::CraftingSpecialFireworkRocket,
		D14::MinecraftCraftingSpecialFireworkStar => D13::CraftingSpecialFireworkStar,
		D14::MinecraftCraftingSpecialFireworkStarFade => D13::CraftingSpecialFireworkStarFade,
		D14::MinecraftCraftingSpecialMapcloning => D13::CraftingSpecialMapcloning,
		D14::MinecraftCraftingSpecialMapextending => D13::CraftingSpecialMapextending,
		D14::MinecraftCraftingSpecialRepairitem => D13::CraftingSpecialRepairitem,
		D14::MinecraftCraftingSpecialShielddecoration => D13::CraftingSpecialShielddecoration,
		D14::MinecraftCraftingSpecialShulkerboxcoloring => D13::CraftingSpecialShulkerboxcoloring,
		D14::MinecraftCraftingSpecialTippedarrow => D13::CraftingSpecialTippedarrow,
		_ => return Err(Error::Dropped),
	};
	Ok(Recipe13 {
		recipe_id: r.recipe_id,
		r#type: r.r#type.trim_start_matches("minecraft:").to_owned(),
		data,
	})
}

type Statistic = v1_13_2::play_to_client_packet_statistics_items::Ty;

pub(super) fn statistic(
	s: Statistic,
	from: Version,
	to: Version,
	custom: i32,
) -> Result<Statistic> {
	let id = match s.category_id {
		0 => remap("blocks", from, to, s.statistic_id),
		1..=5 => remap("items", from, to, s.statistic_id),
		6 | 7 => remap("entities", from, to, s.statistic_id),
		_ => (0..custom)
			.contains(&s.statistic_id)
			.then_some(s.statistic_id),
	};
	Ok(Statistic {
		statistic_id: id.ok_or(Error::Dropped)?,
		..s
	})
}

pub fn village_statistic_1_14(s: Statistic, _cx: &mut Session) -> Result<Statistic> {
	statistic(s, OLD, NEW, CUSTOM_STATISTICS)
}

pub fn village_statistic_1_13_2(s: Statistic, _cx: &mut Session) -> Result<Statistic> {
	statistic(s, NEW, OLD, CUSTOM_STATISTICS)
}

pub fn village_block_change_1_14(
	p: &v1_13_2::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(OLD, NEW, p.r#type))
}

pub fn village_block_change_1_13_2(
	p: &v1_14::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(NEW, OLD, p.r#type))
}

pub fn village_multi_block_change_1_14(
	r: &v1_13_2::play_to_client_packet_multi_block_change_records_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(OLD, NEW, r.block_id))
}

pub fn village_multi_block_change_1_13_2(
	r: &v1_14::play_to_client_packet_multi_block_change_records_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(NEW, OLD, r.block_id))
}

pub fn village_block_action_1_14(
	p: &v1_13_2::play_to_client_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("blocks", OLD, NEW, p.block_id).ok_or(Error::Dropped)
}

pub fn village_block_action_1_13_2(
	p: &v1_14::play_to_client_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("blocks", NEW, OLD, p.block_id).ok_or(Error::Dropped)
}

pub(super) fn world_event(effect: i32, data: i32, from: Version, to: Version) -> Result<i32> {
	Ok(match effect {
		1010 if data != 0 => remap("items", from, to, data).ok_or(Error::Dropped)?,
		2001 => state(from, to, data),
		_ => data,
	})
}

pub fn village_world_event_1_14(
	p: &v1_13_2::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	world_event(p.effect_id, p.data, OLD, NEW)
}

pub fn village_world_event_1_13_2(
	p: &v1_14::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	world_event(p.effect_id, p.data, NEW, OLD)
}

pub fn village_sound_effect_1_14(
	p: &v1_13_2::play_to_client_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("sounds", OLD, NEW, p.sound_id).ok_or(Error::Dropped)
}

pub fn village_sound_effect_1_13_2(
	p: &v1_14::play_to_client_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("sounds", NEW, OLD, p.sound_id).ok_or(Error::Dropped)
}

pub fn village_set_cooldown_1_14(
	p: &v1_13_2::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("items", OLD, NEW, p.item_id).ok_or(Error::Dropped)
}

pub fn village_set_cooldown_1_13_2(
	p: &v1_14::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("items", NEW, OLD, p.item_id).ok_or(Error::Dropped)
}
