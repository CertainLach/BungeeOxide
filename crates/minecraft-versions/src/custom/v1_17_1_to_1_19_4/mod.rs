use std::collections::{HashMap, HashSet};

use nbt::{CompoundExt, CompoundVecExt, Entry, List, ListItems, Payload};

use super::chat_types;
use crate::gen::convert::{v1_13_2, v1_17_1, Version as V};
use crate::pds::convert::{unconvertible, Error, Result};
use crate::pds::{Input, Output};
use crate::registry;
use crate::session::Session;

mod chunk;
mod meta;
mod new_tags;
mod s1_18;
mod s1_19;
mod s1_19_3;
mod s1_19_4;

pub use s1_18::*;
pub use s1_19::*;
pub use s1_19_3::*;
pub use s1_19_4::*;

type Slot = v1_13_2::slot::Ty;
type Light = v1_17_1::play_to_client_packet_update_light::Ty;
type Tags = Vec<v1_17_1::play_to_client_packet_tags_items::Ty>;
type Stats = Vec<v1_17_1::play_to_client_packet_statistics_items::Ty>;

struct Painting {
	uuid: u128,
	position: (f64, f64, f64),
	direction: i32,
}

#[derive(Default)]
struct World {
	min_y: i32,
	height: i32,
	biomes: usize,
	light: HashMap<(i32, i32), Light>,
	loaded: HashSet<(i32, i32)>,
	dimensions: Vec<(String, Vec<Entry>)>,
	ack: Option<(i32, bool)>,
	entities: HashMap<i32, String>,
	paintings: HashMap<i32, Painting>,
	player: i32,
	vehicle: Option<i32>,
	seed: u64,
	sequence: i32,
}

impl World {
	fn random(&mut self) -> i64 {
		self.seed = self.seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
		let mut z = self.seed;
		z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
		z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
		(z ^ (z >> 31)) as i64
	}

	fn dimension_key(&self, dimension: &Entry) -> String {
		let Some(c) = dimension.value.as_compound() else {
			return "minecraft:overworld".into();
		};
		let score = |e: &[Entry]| c.iter().filter(|x| e.iter().any(|y| y == *x)).count();
		self.dimensions
			.iter()
			.max_by_key(|(_, e)| (score(e), usize::MAX - e.len().abs_diff(c.len())))
			.map_or_else(|| "minecraft:overworld".into(), |(name, _)| name.clone())
	}

	fn dimension(&self, key: &str) -> Option<Entry> {
		let (_, e) = self.dimensions.iter().find(|(n, _)| n == key)?;
		Some(Entry::new("", Payload::Compound(e.clone())))
	}

	fn sections(&self) -> usize {
		self.known_sections().unwrap_or(16)
	}

	fn known_sections(&self) -> Option<usize> {
		(self.height > 0).then_some((self.height / 16) as usize)
	}

	fn biome_bits(&self) -> u32 {
		ceil_log2(if self.biomes > 0 { self.biomes } else { 64 })
	}

	fn track_dimension(&mut self, dimension: &Entry) {
		let Some(c) = dimension.value.as_compound() else {
			return;
		};
		if let (Some(min_y), Some(height)) = (
			c.field("min_y").and_then(Payload::as_i64),
			c.field("height").and_then(Payload::as_i64),
		) {
			self.min_y = min_y as i32;
			self.height = height as i32;
		}
	}

	fn track_codec(&mut self, codec: &Entry) {
		let Some(root) = codec.value.as_compound() else {
			return;
		};
		let list = |name: &str| match root.field(name) {
			Some(Payload::Compound(reg)) => match reg.field("value") {
				Some(Payload::List(List {
					items: ListItems::Compound(list),
					..
				})) => Some(list),
				_ => None,
			},
			_ => None,
		};
		if let Some(biomes) = list("minecraft:worldgen/biome") {
			self.biomes = biomes.len();
		}
		if let Some(dims) = list("minecraft:dimension_type") {
			self.dimensions = dims
				.iter()
				.filter_map(|d| {
					let name = d.field("name").and_then(Payload::as_str)?.to_owned();
					let Some(Payload::Compound(element)) = d.field("element") else {
						return None;
					};
					Some((name, element.clone()))
				})
				.collect();
		}
	}

	fn enter(&mut self, codec: &Entry, dimension: &str) {
		self.track_codec(codec);
		self.track_dimension_key(dimension);
	}

	fn track_dimension_key(&mut self, key: &str) {
		if let Some(d) = self.dimension(key) {
			self.track_dimension(&d);
		}
	}

	fn forget_chunks(&mut self) {
		self.light.clear();
		self.loaded.clear();
	}

	fn spawned(&mut self, entity: i32, version: V, ty: i32) {
		if let Some(name) = registry::names(version, "entities").and_then(|n| n.name(ty as u32)) {
			self.entities.insert(entity, name.to_owned());
		}
	}

	fn is(&self, entity: i32, names: &[&str]) -> bool {
		self.entities
			.get(&entity)
			.is_some_and(|n| names.contains(&n.as_str()))
	}

	fn name(&self, entity: i32) -> Option<&str> {
		self.entities.get(&entity).map(String::as_str)
	}
}

fn world(cx: &mut Session) -> &mut World {
	cx.state::<World>()
}

pub fn caves_known_sections(cx: &mut Session) -> Option<usize> {
	world(cx).known_sections()
}

pub fn caves_track_codec(codec: &mut Entry, _from: V, _to: V, cx: &mut Session) -> Result<()> {
	world(cx).track_codec(codec);
	Ok(())
}

pub fn caves_track_dimension(
	dimension: &mut Entry,
	_from: V,
	_to: V,
	cx: &mut Session,
) -> Result<()> {
	world(cx).track_dimension(dimension);
	Ok(())
}

pub fn caves_track_dimension_key(
	key: &mut String,
	_from: V,
	_to: V,
	cx: &mut Session,
) -> Result<()> {
	world(cx).track_dimension_key(key);
	Ok(())
}

fn ceil_log2(n: usize) -> u32 {
	match n {
		0 | 1 => 0,
		n => usize::BITS - (n - 1).leading_zeros(),
	}
}

fn encoding(e: crate::pds::Error) -> Error {
	Error::Encoding(format!("{e:?}"))
}

fn encode<A>(v: &A, ser: fn(&A, &mut Output) -> crate::pds::Result<()>) -> Result<Vec<u8>> {
	let mut o = Output::new();
	ser(v, &mut o).map_err(encoding)?;
	Ok(o.into_inner())
}

fn decode<B>(bytes: &[u8], de: fn(&mut Input) -> crate::pds::Result<B>) -> Result<B> {
	let mut input = Input::new(bytes);
	let v = de(&mut input).map_err(encoding)?;
	match input.remaining() {
		0 => Ok(v),
		n => unconvertible(format!("{n} bytes left after re-decoding")),
	}
}

fn read_varint(data: &[u8], at: &mut usize) -> Result<i32> {
	let mut v: u32 = 0;
	for shift in (0..35).step_by(7) {
		let Some(&b) = data.get(*at) else {
			return unconvertible("truncated varint");
		};
		*at += 1;
		v |= ((b & 0x7f) as u32) << shift;
		if b & 0x80 == 0 {
			return Ok(v as i32);
		}
	}
	unconvertible("varint is too long")
}

fn write_varint(out: &mut Vec<u8>, v: i32) {
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

fn take<'a>(data: &'a [u8], at: &mut usize, n: usize) -> Result<&'a [u8]> {
	let Some(s) = data.get(*at..*at + n) else {
		return unconvertible("truncated data");
	};
	*at += n;
	Ok(s)
}

fn item(from: V, to: V, id: i32) -> Option<i32> {
	if id == 0 {
		return Some(0);
	}
	registry::remap_name("items", from, to, id as u32).map(|v| v as i32)
}

fn slot(from: V, to: V, v: Slot) -> Slot {
	match v {
		Slot::True(mut s) => match item(from, to, s.item_id) {
			Some(id) => {
				s.item_id = id;
				Slot::True(s)
			}
			None => Slot::False,
		},
		empty => empty,
	}
}

fn entity_type(from: V, to: V, id: i32) -> Option<i32> {
	registry::remap_name("entities", from, to, id as u32).map(|v| v as i32)
}

fn state(from: V, to: V, id: i32) -> i32 {
	registry::remap_state(from, to, id as u32) as i32
}

fn block(from: V, to: V, id: i32) -> Option<i32> {
	let (a, b) = (registry::blocks(from)?, registry::blocks(to)?);
	let old = a
		.list
		.get(id as usize)
		.filter(|x| x.id == id as u32)
		.or_else(|| a.list.iter().find(|x| x.id == id as u32))?;
	let renamed = registry::renames::BLOCKS
		.iter()
		.find(|(v, o, _)| *v == to && *o == old.name);
	b.block(renamed.map_or(old.name.as_str(), |r| r.2))
		.map(|x| x.id as i32)
}

fn named(kind: &str, from: V, to: V, id: i32) -> Option<i32> {
	registry::remap_name(kind, from, to, id as u32).map(|v| v as i32)
}

fn particle_name(version: V, id: i32) -> Option<&'static str> {
	registry::names(version, "particles")?.name(id as u32)
}

fn particle_id(version: V, name: &str) -> Option<i32> {
	registry::names(version, "particles")?
		.id(name)
		.map(|v| v as i32)
}

#[derive(Clone, Copy)]
enum Source {
	Block([u8; 8]),
	Entity(i32, f32),
}

enum Data {
	None,
	State(i32),
	Raw(Vec<u8>),
	Item(Slot),
	Vibration(Option<[u8; 8]>, Option<Source>, i32),
}

fn vibration_has_origin(version: V) -> bool {
	version < V::V1_19
}

fn unpatched(version: V) -> bool {
	version >= V::V1_19_4
}

fn read_data(version: V, name: &str, data: &[u8]) -> Result<Data> {
	let mut at = 0;
	Ok(match name {
		"block" | "block_marker" | "falling_dust" => Data::State(read_varint(data, &mut at)?),
		"item" => Data::Item(decode(data, v1_13_2::slot::deserialize)?),
		"vibration" => {
			let origin = match vibration_has_origin(version) {
				true => Some(take(data, &mut at, 8)?.try_into().expect("8 bytes")),
				false => None,
			};
			let n = read_varint(data, &mut at)? as usize;
			let kind = std::str::from_utf8(take(data, &mut at, n)?)
				.unwrap_or_default()
				.trim_start_matches("minecraft:")
				.to_owned();
			let source = match kind.as_str() {
				"block" => Some(Source::Block(
					take(data, &mut at, 8)?.try_into().expect("8 bytes"),
				)),
				"entity" if vibration_has_origin(version) => {
					Some(Source::Entity(read_varint(data, &mut at)?, 0.0))
				}
				"entity" if unpatched(version) => {
					let id = read_varint(data, &mut at)?;
					read_varint(data, &mut at)?;
					read_varint(data, &mut at)?;
					Some(Source::Entity(id, 0.0))
				}
				"entity" => {
					let id = read_varint(data, &mut at)?;
					Some(Source::Entity(
						id,
						f32::from_be_bytes(take(data, &mut at, 4)?.try_into().expect("4 bytes")),
					))
				}
				_ => None,
			};
			Data::Vibration(origin, source, read_varint(data, &mut at)?)
		}
		"dust" | "dust_color_transition" | "sculk_charge" | "shriek" => Data::Raw(data.to_vec()),
		_ => Data::None,
	})
}

fn write_data(from: V, to: V, name: &str, data: Data) -> Result<Option<Vec<u8>>> {
	let mut out = Vec::new();
	match (name, data) {
		("block" | "block_marker" | "falling_dust", Data::State(s)) => {
			write_varint(&mut out, state(from, to, s))
		}
		("item", Data::Item(s)) => out = encode(&slot(from, to, s), v1_13_2::slot::serialize)?,
		("vibration", Data::Vibration(origin, Some(source), ticks)) => {
			if vibration_has_origin(to) {
				let Some(origin) = origin else {
					return Ok(None);
				};
				out.extend_from_slice(&origin);
			}
			let (kind, body) = match source {
				Source::Block(pos) => ("minecraft:block", pos.to_vec()),
				Source::Entity(_, _) if unpatched(to) => return Ok(None),
				Source::Entity(id, eye) => {
					let mut b = Vec::new();
					write_varint(&mut b, id);
					if !vibration_has_origin(to) {
						b.extend_from_slice(&eye.to_be_bytes());
					}
					("minecraft:entity", b)
				}
			};
			write_varint(&mut out, kind.len() as i32);
			out.extend_from_slice(kind.as_bytes());
			out.extend(body);
			write_varint(&mut out, ticks);
		}
		("dust" | "dust_color_transition" | "sculk_charge" | "shriek", Data::Raw(raw)) => out = raw,
		(
			"block"
			| "block_marker"
			| "falling_dust"
			| "item"
			| "vibration"
			| "dust"
			| "dust_color_transition"
			| "sculk_charge"
			| "shriek",
			_,
		) => return Ok(None),
		_ => {}
	}
	Ok(Some(out))
}

fn particle_data(from: V, to: V, id: i32, data: &[u8]) -> Result<Option<(i32, Vec<u8>)>> {
	let Some(name) = particle_name(from, id) else {
		return Ok(None);
	};
	let source = read_data(from, name, data)?;
	let target = match (name, &source) {
		("barrier" | "light", _) if particle_id(to, name).is_none() => {
			let Some(marker) = particle_id(to, "block_marker") else {
				return Ok(None);
			};
			let state = registry::blocks(to)
				.and_then(|b| b.block(name))
				.map_or(0, |b| b.default as i32);
			let mut out = Vec::new();
			write_varint(&mut out, state);
			return Ok(Some((marker, out)));
		}
		("block_marker", Data::State(s)) if particle_id(to, name).is_none() => {
			let light = registry::blocks(from)
				.and_then(|b| b.owner(*s as u32))
				.is_some_and(|b| b.name == "light");
			particle_id(to, if light { "light" } else { "barrier" })
		}
		_ => named("particles", from, to, id),
	};
	let Some(target) = target else {
		return Ok(None);
	};
	let Some(target_name) = particle_name(to, target) else {
		return Ok(None);
	};
	Ok(write_data(from, to, target_name, source)?.map(|d| (target, d)))
}

fn particle<A, B>(
	from: V,
	to: V,
	v: &A,
	ser: fn(&A, &mut Output) -> crate::pds::Result<()>,
	de: fn(&mut Input) -> crate::pds::Result<B>,
) -> Result<B> {
	let bytes = encode(v, ser)?;
	let mut at = 0;
	let id = read_varint(&bytes, &mut at)?;
	let Some((id, data)) = particle_data(from, to, id, &bytes[at..])? else {
		return Err(Error::Dropped);
	};
	let mut out = Vec::new();
	write_varint(&mut out, id);
	out.extend(data);
	decode(&out, de)
}

fn sound(from: V, to: V, id: i32) -> Result<i32> {
	named("sounds", from, to, id).ok_or(Error::Dropped)
}

fn tag_entries(version: V, kind: &str, names: &[&str]) -> Vec<i32> {
	names
		.iter()
		.filter_map(|n| match kind.trim_start_matches("minecraft:") {
			"block" => registry::blocks(version)?.block(n).map(|b| b.id as i32),
			"item" => registry::names(version, "items")?.id(n).map(|i| i as i32),
			"entity_type" => registry::names(version, "entities")?
				.id(n)
				.map(|i| i as i32),
			_ => None,
		})
		.collect()
}

fn tags(
	from: V,
	to: V,
	list: Tags,
	renamed: &[(&str, &str)],
	added: &[(&str, &str, &[&str])],
) -> Tags {
	use v1_17_1::play_to_client_packet_tags_items_tags_items::Ty as Tag;
	let mut list: Tags = list
		.into_iter()
		.map(|mut kind| {
			let map = |id: i32| -> Option<i32> {
				match kind.tag_type.trim_start_matches("minecraft:") {
					"item" => item(from, to, id),
					"block" => block(from, to, id),
					"entity_type" => entity_type(from, to, id),
					_ => Some(id),
				}
			};
			kind.tags = std::mem::take(&mut kind.tags)
				.into_iter()
				.map(|mut t| {
					let mut entries: Vec<i32> = t.entries.iter().filter_map(|e| map(*e)).collect();
					entries.dedup();
					t.entries = entries;
					if let Some((_, new)) = renamed.iter().find(|(old, _)| *old == t.tag_name) {
						t.tag_name = (*new).to_owned();
					}
					t
				})
				.collect();
			kind
		})
		.collect();
	for (kind, name, values) in added {
		if let Some(k) = list.iter_mut().find(|k| k.tag_type == *kind) {
			if !k.tags.iter().any(|t| t.tag_name == *name) {
				k.tags.push(Tag {
					tag_name: (*name).to_owned(),
					entries: tag_entries(to, kind, values),
				});
			}
		}
	}
	list
}

fn statistics(from: V, to: V, list: Stats) -> Stats {
	list.into_iter()
		.filter_map(|mut s| {
			s.statistic_id = match s.category_id {
				0 => block(from, to, s.statistic_id)?,
				1..=5 => item(from, to, s.statistic_id)?,
				6 | 7 => entity_type(from, to, s.statistic_id)?,
				_ => s.statistic_id,
			};
			Some(s)
		})
		.collect()
}

fn registry_entries<'a>(codec: &'a mut Entry, name: &str) -> Option<&'a mut Vec<Vec<Entry>>> {
	codec
		.value
		.as_compound_mut()?
		.compound_mut(name)?
		.compounds_mut("value")
}

fn vanilla_registry(version: V, name: &str) -> Option<Payload> {
	let json: serde_json::Value = serde_json::from_str(registry::login_packet(version)?).ok()?;
	nbt::json::typed::read(json.get("dimensionCodec")?.get("value")?.get(name)?)
}

fn put_registry(codec: &mut Entry, version: V, name: &str) -> Result<()> {
	let Some(reg) = vanilla_registry(version, name) else {
		return unconvertible(format!("no vanilla {name} registry for {}", version.name()));
	};
	let Payload::Compound(root) = &mut codec.value else {
		return unconvertible("the registry codec is not a compound");
	};
	root.put(name, reg);
	Ok(())
}

fn block_entity_types(version: V) -> Vec<&'static str> {
	let mut v = vec![
		"furnace",
		"chest",
		"trapped_chest",
		"ender_chest",
		"jukebox",
		"dispenser",
		"dropper",
		"sign",
		"mob_spawner",
		"piston",
		"brewing_stand",
		"enchanting_table",
		"end_portal",
		"beacon",
		"skull",
		"daylight_detector",
		"hopper",
		"comparator",
		"banner",
		"structure_block",
		"end_gateway",
		"command_block",
		"shulker_box",
		"bed",
		"conduit",
		"barrel",
		"smoker",
		"blast_furnace",
		"lectern",
		"bell",
		"jigsaw",
		"campfire",
		"beehive",
		"sculk_sensor",
	];
	if version >= V::V1_19 {
		v.extend(["sculk_catalyst", "sculk_shrieker"]);
	}
	if version >= V::V1_19_3 {
		v.insert(8, "hanging_sign");
		v.push("chiseled_bookshelf");
	}
	if version >= V::V1_19_4 {
		v.extend(["suspicious_sand", "decorated_pot"]);
	}
	v
}

fn block_entity_fallback(name: &str) -> &str {
	match name {
		"hanging_sign" => "sign",
		"chiseled_bookshelf" | "decorated_pot" | "suspicious_sand" => "",
		other => other,
	}
}

fn block_entity_type(from: V, to: V, id: i32) -> Option<i32> {
	let name = *block_entity_types(from).get(id as usize)?;
	let types = block_entity_types(to);
	types
		.iter()
		.position(|n| *n == name)
		.or_else(|| types.iter().position(|n| *n == block_entity_fallback(name)))
		.map(|i| i as i32)
}

fn chat_text(text: &str) -> String {
	serde_json::json!({ "text": text }).to_string()
}
