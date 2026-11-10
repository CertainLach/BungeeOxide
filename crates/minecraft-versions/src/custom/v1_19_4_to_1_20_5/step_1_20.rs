use super::common::{self, block, block_state, encode, item, name_id, sound, ParticleMap, Sound};
use crate::gen::convert::Version;
use crate::gen::convert::{v1_19_4 as old, v1_20 as new};
use crate::pds::convert::{Error, Result};
use crate::Session;
use nbt::{CompoundExt, CompoundVecExt, Entry, List, Payload, PayloadMut};

const OLD: Version = Version::V1_19_4;
const NEW: Version = Version::V1_20;

type Slot = old::slot::Ty;
type Client = new::play_toClient_packet::Ty;
type ClientOld = old::play_toClient_packet::Ty;

const NEW_TRIM_PATTERNS: &[&str] = &["host", "raiser", "silence", "shaper", "wayfinder"];
const STASHED_TRIM: &str = "pds2:Trim";

fn stash_trim(nbt: &mut Option<Entry>, down: bool) {
	let Some(Entry {
		value: Payload::compound(c),
		..
	}) = nbt
	else {
		return;
	};
	if !down {
		c.rename(STASHED_TRIM, "Trim");
		return;
	}
	let pattern = c
		.field("Trim")
		.and_then(Payload::as_compound)
		.and_then(|t| t.field("pattern").and_then(Payload::as_str));
	if pattern
		.is_some_and(|p| NEW_TRIM_PATTERNS.contains(&p.strip_prefix("minecraft:").unwrap_or(p)))
	{
		c.rename("Trim", STASHED_TRIM);
	}
}

pub(super) fn remap_slot(v: Slot, from: Version, to: Version) -> Slot {
	match v {
		Slot::r#true(mut s) => match item(from, to, s.itemId) {
			Some(id) => {
				s.itemId = id;
				stash_trim(&mut s.nbtData, !common::later(from, to));
				Slot::r#true(s)
			}
			None => Slot::r#false,
		},
		v => v,
	}
}

pub fn trails_slot_1_19_4_to_1_20(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(remap_slot(v, OLD, NEW))
}

pub fn trails_slot_1_20_to_1_19_4(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(remap_slot(v, NEW, OLD))
}

fn slot_bytes(from: Version, to: Version) -> impl Fn(&[u8]) -> Result<Vec<u8>> {
	move |b: &[u8]| {
		let s = common::decode(b, |i| old::slot::deserialize(i))?;
		encode(&remap_slot(s, from, to), old::slot::serialize)
	}
}

fn split_particle(bytes: &[u8]) -> Result<(i32, Vec<u8>)> {
	let (id, rest) = common::read_varint(bytes)?;
	Ok((id, rest.to_vec()))
}

fn join_particle(id: i32, payload: &[u8]) -> Vec<u8> {
	let mut out = Vec::new();
	common::varint(&mut out, id);
	out.extend_from_slice(payload);
	out
}

pub(super) fn particle_1_19_4_to_1_20(
	v: old::particle::Ty,
	_cx: &mut Session,
) -> Result<new::particle::Ty> {
	let (id, payload) = split_particle(&encode(&v, old::particle::serialize)?)?;
	let item = slot_bytes(OLD, NEW);
	let (id, payload) = common::particle(
		&ParticleMap {
			from: OLD,
			to: NEW,
			item: &item,
		},
		id,
		&payload,
	)?;
	common::decode(&join_particle(id, &payload), |i| {
		new::particle::deserialize(i)
	})
}

pub(super) fn particle_1_20_to_1_19_4(
	v: new::particle::Ty,
	_cx: &mut Session,
) -> Result<old::particle::Ty> {
	let (id, payload) = split_particle(&encode(&v, new::particle::serialize)?)?;
	let item = slot_bytes(NEW, OLD);
	let (id, payload) = common::particle(
		&ParticleMap {
			from: NEW,
			to: OLD,
			item: &item,
		},
		id,
		&payload,
	)?;
	common::decode(&join_particle(id, &payload), |i| {
		old::particle::deserialize(i)
	})
}

pub(super) fn optional_state(from: Version, to: Version, s: i32) -> i32 {
	match s {
		0 => 0,
		s => block_state(from, to, s - 1) + 1,
	}
}

fn rotate_y180(q: old::vec4f::Ty) -> old::vec4f::Ty {
	old::vec4f::Ty {
		x: -q.z,
		y: q.w,
		z: q.x,
		w: -q.y,
	}
}

fn display_rotation(cx: &mut Session, version: Version, entity: i32) -> Option<(u8, bool)> {
	let kind = common::entity_kind(cx, entity)?;
	if kind != "item_display" {
		return None;
	}
	let key = common::meta_index(version, &kind, "left_rotation")?;
	Some((key, common::state(cx).displays.insert(entity)))
}

macro_rules! metadata {
	($name:ident, $root:ty, $src:ident, $dst:ident, $from:expr, $to:expr, $particle:ident) => {
		arm!(
			$name,
			$root,
			entity_metadata,
			$src::play_toClient_packet_entity_metadata::Ty,
			|p, cx| {
				use $dst::entityMetadataItem::Ty as N;
				use $src::entityMetadataItem::Ty as V;
				let rotation = display_rotation(cx, $from, p.entityId);
				let mut rotated = false;
				let mut out = Vec::with_capacity(p.metadata.len() + 1);
				for e in p.metadata {
					let ty = common::decode(
						&encode(&e.r#type, $src::entityMetadata__items__type::serialize)?,
						|i| $dst::entityMetadata__items__type::deserialize(i),
					)?;
					let value = match e.value {
						V::particle(p) => match $particle(p, cx) {
							Ok(p) => N::particle(p),
							Err(Error::Dropped) => continue,
							Err(e) => return Err(e),
						},
						V::quaternion(q) if rotation.is_some_and(|(key, _)| key == e.key) => {
							rotated = true;
							N::quaternion(rotate_y180(q))
						}
						V::block_state(s) => N::block_state(block_state($from, $to, s)),
						V::optional_block_state(s) => {
							N::optional_block_state(optional_state($from, $to, s))
						}
						V::item_stack(s) => N::item_stack(remap_slot(s, $from, $to)),
						v => common::decode(
							&encode(&v, $src::entityMetadataItem::serialize)?,
							|i| {
								$dst::entityMetadataItem::deserialize(
									i,
									$dst::entityMetadataItem::Args { compareTo: &ty },
								)
							},
						)?,
					};
					out.push($dst::entityMetadata__items::Ty {
						key: e.key,
						r#type: ty,
						value,
					});
				}
				if let Some((key, true)) = rotation.filter(|_| !rotated) {
					out.push($dst::entityMetadata__items::Ty {
						key,
						r#type: $dst::entityMetadata__items__type::Ty::quaternion,
						value: N::quaternion(old::vec4f::Ty {
							x: 0.0,
							y: 1.0,
							z: 0.0,
							w: 0.0,
						}),
					});
				}
				$dst::play_toClient_packet_entity_metadata::Ty {
					entityId: p.entityId,
					metadata: out,
				}
			}
		);
	};
}

metadata!(
	trails_entity_metadata_1_19_4_to_1_20,
	Client,
	old,
	new,
	OLD,
	NEW,
	particle_1_19_4_to_1_20
);
metadata!(
	trails_entity_metadata_1_20_to_1_19_4,
	ClientOld,
	new,
	old,
	NEW,
	OLD,
	particle_1_20_to_1_19_4
);

fn damage_type(name: &str, id: i32) -> Payload {
	Payload::compound(vec![
		Entry::new("name", Payload::string(name.to_owned())),
		Entry::new("id", Payload::int(id)),
		Entry::new(
			"element",
			Payload::compound(vec![
				Entry::new("scaling", Payload::string("always".to_owned())),
				Entry::new("exhaustion", Payload::float(0.0)),
				Entry::new("message_id", Payload::string("badRespawnPoint".to_owned())),
			]),
		),
	])
}

pub(super) fn registry_entries<'a>(
	codec: &'a mut Payload,
	registry: &str,
) -> Option<PayloadMut<'a>> {
	let reg = codec
		.as_compound_mut()?
		.compound_mut(&format!("minecraft:{registry}"))?;
	reg.field_mut("value")
		.filter(|v| matches!(**v, Payload::list(_)))
}

pub fn trails_codec_1_19_4_to_1_20(mut v: Entry, _cx: &mut Session) -> Result<Entry> {
	if let Some(mut l) = registry_entries(&mut v.value, "damage_type") {
		let mut entries = l.values();
		let names: Vec<String> = entries
			.iter()
			.filter_map(|e| {
				e.as_compound()
					.and_then(|c| c.field("name").and_then(Payload::as_str))
					.map(str::to_owned)
			})
			.collect();
		let mut next = entries
			.iter()
			.filter_map(|e| {
				e.as_compound()
					.and_then(|c| c.field("id").and_then(Payload::to_i32))
			})
			.max()
			.unwrap_or(-1)
			+ 1;
		for name in ["minecraft:outside_border", "minecraft:generic_kill"] {
			if !names
				.iter()
				.any(|n| n == name || format!("minecraft:{n}") == name)
			{
				entries.push(damage_type(name, next));
				next += 1;
			}
		}
		*l = List::of(entries).into();
	}
	Ok(v)
}

pub fn trails_codec_1_20_to_1_19_4(mut v: Entry, _cx: &mut Session) -> Result<Entry> {
	if let Payload::compound(root) = &mut v.value {
		if root.field("minecraft:trim_pattern").is_none() {
			if let Ok(nbt::Tag {
				value: Payload::compound(vanilla),
				..
			}) = super::codec::vanilla(NEW)
			{
				if let Some(p) = vanilla.field("minecraft:trim_pattern") {
					root.put("minecraft:trim_pattern", p.clone());
				}
			}
		}
	}
	if let Some(mut l) = registry_entries(&mut v.value, "trim_pattern") {
		let entries = l
			.values()
			.into_iter()
			.map(|mut e| {
				if let Payload::compound(c) = &mut e {
					if let Some(element) = c.compound_mut("element") {
						let template = element
							.field("template_item")
							.and_then(Payload::as_str)
							.unwrap_or_default();
						let name = template
							.strip_prefix("minecraft:")
							.unwrap_or(template)
							.strip_suffix("_armor_trim_smithing_template")
							.unwrap_or_default();
						if NEW_TRIM_PATTERNS.contains(&name) {
							element.put(
								"template_item",
								Payload::string(
									"minecraft:spire_armor_trim_smithing_template".to_owned(),
								),
							);
						}
					}
				}
				e
			})
			.collect();
		*l = List::of(entries).into();
	}
	Ok(v)
}

const UPDATE_1_20: &str = "minecraft:update_1_20";

pub fn trails_feature_flags_1_19_4_to_1_20(
	mut p: Vec<String>,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	p.retain(|f| f != UPDATE_1_20);
	Ok(vec![Client::feature_flags(p)])
}

pub fn trails_feature_flags_1_20_to_1_19_4(
	mut p: Vec<String>,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	if !p.iter().any(|f| f == UPDATE_1_20) {
		p.push(UPDATE_1_20.to_owned());
	}
	Ok(vec![ClientOld::feature_flags(p)])
}

pub(super) fn block_entity_type(from: Version, to: Version, t: i32) -> Option<i32> {
	match (from, to) {
		(OLD, NEW) => Some(if t >= 35 { t + 1 } else { t }),
		(NEW, OLD) => match t {
			35 => None,
			t if t > 35 => Some(t - 1),
			t => Some(t),
		},
		_ => Some(t),
	}
}

fn empty_line() -> Payload {
	Payload::string(r#"{"text":""}"#.to_owned())
}

pub(super) fn sign_1_19_4_to_1_20(c: &mut Vec<Entry>) {
	let messages: Vec<Payload> = (1..=4)
		.map(|i| match c.take(&format!("Text{i}")) {
			Some(p @ Payload::string(_)) => p,
			_ => empty_line(),
		})
		.collect();
	let filtered: Vec<Payload> = (1..=4)
		.map(|i| match c.take(&format!("FilteredText{i}")) {
			Some(p @ Payload::string(_)) => p,
			_ => messages[i - 1].clone(),
		})
		.collect();
	let mut front = vec![Entry::new("messages", List::of(messages.clone()))];
	if filtered != messages {
		front.push(Entry::new("filtered_messages", List::of(filtered)));
	}
	if let Some(color) = c.take("Color") {
		front.push(Entry::new("color", color));
	}
	if let Some(glowing) = c.take("GlowingText") {
		front.push(Entry::new("has_glowing_text", glowing));
	}
	c.put("front_text", Payload::compound(front));
}

pub(super) fn sign_1_20_to_1_19_4(c: &mut Vec<Entry>) {
	let Some(Payload::compound(front)) = c.take("front_text") else {
		return;
	};
	c.take("back_text");
	c.take("is_waxed");
	let messages = front
		.field("messages")
		.map(Payload::values)
		.unwrap_or_default();
	let filtered = front
		.field("filtered_messages")
		.map(Payload::values)
		.unwrap_or_default();
	for i in 0..4 {
		let line = messages.get(i).cloned().unwrap_or_else(empty_line);
		c.put(&format!("Text{}", i + 1), line.clone());
		if let Some(f) = filtered.get(i).filter(|f| **f != line) {
			c.put(&format!("FilteredText{}", i + 1), f.clone());
		}
	}
	if let Some(color) = front.field("color") {
		c.put("Color", color.clone());
	}
	if let Some(glowing) = front.field("has_glowing_text") {
		c.put("GlowingText", glowing.clone());
	}
}

fn block_entity_nbt(from: Version, to: Version, kind: i32, nbt: &mut Option<Entry>) {
	if let Some(Entry {
		value: Payload::compound(c),
		..
	}) = nbt
	{
		match (from, to, kind) {
			(OLD, NEW, 7 | 8) => sign_1_19_4_to_1_20(c),
			(NEW, OLD, 7 | 8) => sign_1_20_to_1_19_4(c),
			_ => {}
		}
	}
}

pub fn trails_chunk_entities_1_20(
	list: Vec<new::chunkBlockEntity::Ty>,
	up: bool,
) -> Vec<new::chunkBlockEntity::Ty> {
	let (from, to) = if up { (OLD, NEW) } else { (NEW, OLD) };
	list.into_iter()
		.filter_map(|mut b| {
			let kind = block_entity_type(from, to, b.r#type)?;
			block_entity_nbt(
				from,
				to,
				if up { b.r#type.min(kind) } else { b.r#type },
				&mut b.nbtData,
			);
			b.r#type = kind;
			Some(b)
		})
		.collect()
}

fn map_chunk(
	from: Version,
	to: Version,
	p: old::play_toClient_packet_map_chunk::Ty,
	known: Option<usize>,
) -> Result<new::play_toClient_packet_map_chunk::Ty> {
	let state = |s: u32| block_state(from, to, s as i32) as u32;
	let biome = |b: u32| b;
	let remap = super::chunk::Remap {
		state: &state,
		states: common::states(to),
		biome: &biome,
		biomes: None,
	};
	let chunkData = super::chunk::sections(&p.chunkData, &remap, from.index(), known)?;
	let blockEntities = trails_chunk_entities_1_20(p.blockEntities, (from, to) == (OLD, NEW));
	Ok(new::play_toClient_packet_map_chunk::Ty {
		x: p.x,
		z: p.z,
		heightmaps: p.heightmaps,
		chunkData,
		blockEntities,
		skyLightMask: p.skyLightMask,
		blockLightMask: p.blockLightMask,
		emptySkyLightMask: p.emptySkyLightMask,
		emptyBlockLightMask: p.emptyBlockLightMask,
		skyLight: p.skyLight,
		blockLight: p.blockLight,
	})
}

pub fn trails_map_chunk_1_19_4_to_1_20(
	p: old::play_toClient_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	Ok(vec![Client::map_chunk(map_chunk(
		OLD,
		NEW,
		p,
		crate::custom::caves_known_sections(cx),
	)?)])
}

pub fn trails_map_chunk_1_20_to_1_19_4(
	p: new::play_toClient_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let state = |s: u32| block_state(NEW, OLD, s as i32) as u32;
	let biome = |b: u32| b;
	let remap = super::chunk::Remap {
		state: &state,
		states: common::states(OLD),
		biome: &biome,
		biomes: None,
	};
	let chunkData = super::chunk::sections(
		&p.chunkData,
		&remap,
		NEW.index(),
		crate::custom::caves_known_sections(cx),
	)?;
	let blockEntities = trails_chunk_entities_1_20(p.blockEntities, false);
	Ok(vec![ClientOld::map_chunk(
		old::play_toClient_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			heightmaps: p.heightmaps,
			chunkData,
			blockEntities,
			trustEdges: true,
			skyLightMask: p.skyLightMask,
			blockLightMask: p.blockLightMask,
			emptySkyLightMask: p.emptySkyLightMask,
			emptyBlockLightMask: p.emptyBlockLightMask,
			skyLight: p.skyLight,
			blockLight: p.blockLight,
		},
	)])
}

pub(super) fn record(from: Version, to: Version, r: i64) -> i64 {
	let s = block_state(from, to, (r >> 12) as i32) as i64;
	(s << 12) | (r & 0xfff)
}

pub fn trails_multi_block_change_1_19_4_to_1_20(
	p: old::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	let c = p.chunkCoordinates;
	Ok(vec![Client::multi_block_change(
		new::play_toClient_packet_multi_block_change::Ty {
			chunkCoordinates: new::play_toClient_packet_multi_block_change__chunkCoordinates::Ty {
				x: c.x,
				z: c.z,
				y: c.y,
			},
			records: p
				.records
				.into_iter()
				.map(|r| record(OLD, NEW, r as i64) as i32)
				.collect(),
		},
	)])
}

pub fn trails_multi_block_change_1_20_to_1_19_4(
	p: new::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let c = p.chunkCoordinates;
	Ok(vec![ClientOld::multi_block_change(
		old::play_toClient_packet_multi_block_change::Ty {
			chunkCoordinates: old::play_toClient_packet_multi_block_change__chunkCoordinates::Ty {
				x: c.x,
				z: c.z,
				y: c.y,
			},
			suppressLightUpdates: false,
			records: p
				.records
				.into_iter()
				.map(|r| record(NEW, OLD, r as i64) as i32)
				.collect(),
		},
	)])
}

pub fn trails_block_change_1_19_4_to_1_20(
	mut p: old::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	p.r#type = block_state(OLD, NEW, p.r#type);
	Ok(vec![Client::block_change(p)])
}

pub fn trails_block_change_1_20_to_1_19_4(
	mut p: new::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	p.r#type = block_state(NEW, OLD, p.r#type);
	Ok(vec![ClientOld::block_change(p)])
}

pub fn trails_block_action_1_19_4_to_1_20(
	mut p: old::play_toClient_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	p.blockId = block(OLD, NEW, p.blockId).ok_or(Error::Dropped)?;
	Ok(vec![Client::block_action(p)])
}

pub fn trails_block_action_1_20_to_1_19_4(
	mut p: new::play_toClient_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	p.blockId = block(NEW, OLD, p.blockId).ok_or(Error::Dropped)?;
	Ok(vec![ClientOld::block_action(p)])
}

pub fn trails_tile_entity_data_1_19_4_to_1_20(
	mut p: old::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	block_entity_nbt(OLD, NEW, p.action, &mut p.nbtData);
	p.action = block_entity_type(OLD, NEW, p.action).ok_or(Error::Dropped)?;
	Ok(vec![Client::tile_entity_data(p)])
}

pub fn trails_tile_entity_data_1_20_to_1_19_4(
	mut p: new::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	block_entity_nbt(NEW, OLD, p.action, &mut p.nbtData);
	p.action = block_entity_type(NEW, OLD, p.action).ok_or(Error::Dropped)?;
	Ok(vec![ClientOld::tile_entity_data(p)])
}

pub(super) fn world_event_data(from: Version, to: Version, effect: i32, data: i32) -> i32 {
	match effect {
		2001 => block_state(from, to, data),
		_ => data,
	}
}

pub fn trails_world_event_1_19_4_to_1_20(
	mut p: old::play_toClient_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	p.data = world_event_data(OLD, NEW, p.effectId, p.data);
	Ok(vec![Client::world_event(p)])
}

pub fn trails_world_event_1_20_to_1_19_4(
	mut p: new::play_toClient_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	p.data = world_event_data(NEW, OLD, p.effectId, p.data);
	Ok(vec![ClientOld::world_event(p)])
}

pub fn trails_world_particles_1_19_4_to_1_20(
	p: old::play_toClient_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	let payload = encode(&p.data, old::particleData::serialize)?;
	let item = slot_bytes(OLD, NEW);
	let (id, payload) = common::particle(
		&ParticleMap {
			from: OLD,
			to: NEW,
			item: &item,
		},
		p.particleId,
		&payload,
	)?;
	let data = common::decode(&payload, |i| {
		new::particleData::deserialize(i, new::particleData::Args { compareTo: &id })
	})?;
	Ok(vec![Client::world_particles(
		new::play_toClient_packet_world_particles::Ty {
			particleId: id,
			longDistance: p.longDistance,
			x: p.x,
			y: p.y,
			z: p.z,
			offsetX: p.offsetX,
			offsetY: p.offsetY,
			offsetZ: p.offsetZ,
			particleData: p.particleData,
			particles: p.particles,
			data,
		},
	)])
}

pub fn trails_world_particles_1_20_to_1_19_4(
	p: new::play_toClient_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let payload = encode(&p.data, new::particleData::serialize)?;
	let item = slot_bytes(NEW, OLD);
	let (id, payload) = common::particle(
		&ParticleMap {
			from: NEW,
			to: OLD,
			item: &item,
		},
		p.particleId,
		&payload,
	)?;
	let data = common::decode(&payload, |i| {
		old::particleData::deserialize(i, old::particleData::Args { compareTo: &id })
	})?;
	Ok(vec![ClientOld::world_particles(
		old::play_toClient_packet_world_particles::Ty {
			particleId: id,
			longDistance: p.longDistance,
			x: p.x,
			y: p.y,
			z: p.z,
			offsetX: p.offsetX,
			offsetY: p.offsetY,
			offsetZ: p.offsetZ,
			particleData: p.particleData,
			particles: p.particles,
			data,
		},
	)])
}

pub(super) fn spawn_object_data(from: Version, to: Version, kind: &str, data: i32) -> i32 {
	match kind {
		"falling_block" => block_state(from, to, data),
		_ => data,
	}
}

fn spawn_entity(
	from: Version,
	to: Version,
	mut p: old::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<old::play_toClient_packet_spawn_entity::Ty> {
	common::spawned(cx, from, p.entityId, p.r#type);
	let kind = crate::registry::names(from, "entities")
		.and_then(|n| n.name(p.r#type as u32))
		.unwrap_or_default();
	p.objectData = spawn_object_data(from, to, kind, p.objectData);
	p.r#type = name_id("entities", from, to, p.r#type).ok_or(Error::Dropped)?;
	Ok(p)
}

pub fn trails_spawn_entity_1_19_4_to_1_20(
	p: old::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	Ok(vec![Client::spawn_entity(spawn_entity(OLD, NEW, p, cx)?)])
}

pub fn trails_spawn_entity_1_20_to_1_19_4(
	p: new::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	Ok(vec![ClientOld::spawn_entity(spawn_entity(
		NEW, OLD, p, cx,
	)?)])
}

pub fn trails_set_cooldown_1_19_4_to_1_20(
	mut p: old::play_toClient_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	p.itemID = item(OLD, NEW, p.itemID).ok_or(Error::Dropped)?;
	Ok(vec![Client::set_cooldown(p)])
}

pub fn trails_set_cooldown_1_20_to_1_19_4(
	mut p: new::play_toClient_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	p.itemID = item(NEW, OLD, p.itemID).ok_or(Error::Dropped)?;
	Ok(vec![ClientOld::set_cooldown(p)])
}

pub(super) fn sound_holder(
	from: Version,
	to: Version,
	s: old::ItemSoundHolder::Ty,
) -> Result<old::ItemSoundHolder::Ty> {
	use old::ItemSoundHolder::Ty as H;
	Ok(match s {
		H::Id(id) => match sound(from, to, id)? {
			Sound::Id(id) => H::Id(id),
			Sound::Named(soundName) => H::Inline(old::ItemSoundEvent::Ty {
				soundName,
				fixedRange: None,
			}),
		},
		inline => inline,
	})
}

pub fn trails_sound_effect_1_19_4_to_1_20(
	mut p: old::play_toClient_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	p.sound = sound_holder(OLD, NEW, p.sound)?;
	Ok(vec![Client::sound_effect(p)])
}

pub fn trails_sound_effect_1_20_to_1_19_4(
	mut p: new::play_toClient_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	p.sound = sound_holder(NEW, OLD, p.sound)?;
	Ok(vec![ClientOld::sound_effect(p)])
}

pub(super) fn entity_sound(
	from: Version,
	to: Version,
	s: old::play_toClient_packet_entity_sound_effect__soundEvent::Ty,
) -> Result<old::play_toClient_packet_entity_sound_effect__soundEvent::Ty> {
	use old::play_toClient_packet_entity_sound_effect__soundEvent::Ty as E;
	Ok(match s {
		E::default(tag) => match sound(from, to, tag - 1)? {
			Sound::Id(id) => E::default(id + 1),
			Sound::Named(resource) => E::v0(
				old::play_toClient_packet_entity_sound_effect__soundEvent__v0::Ty {
					resource,
					range: None,
				},
			),
		},
		inline => inline,
	})
}

pub fn trails_entity_sound_effect_1_19_4_to_1_20(
	mut p: old::play_toClient_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	p.soundEvent = entity_sound(OLD, NEW, p.soundEvent)?;
	Ok(vec![Client::entity_sound_effect(p)])
}

pub fn trails_entity_sound_effect_1_20_to_1_19_4(
	mut p: new::play_toClient_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	p.soundEvent = entity_sound(NEW, OLD, p.soundEvent)?;
	Ok(vec![ClientOld::entity_sound_effect(p)])
}

pub(super) fn tag_entries(
	registry: &str,
	from: Version,
	to: Version,
	entries: Vec<i32>,
) -> Vec<i32> {
	let map: &dyn Fn(i32) -> Option<i32> =
		match registry.strip_prefix("minecraft:").unwrap_or(registry) {
			"block" => &|id| block(from, to, id),
			"item" => &|id| item(from, to, id),
			"entity_type" => &|id| name_id("entities", from, to, id),
			_ => &|id| Some(id),
		};
	entries.into_iter().filter_map(map).collect()
}

pub(super) fn remap_tags(
	from: Version,
	to: Version,
	mut tags: Vec<old::play_toClient_packet__tags__items::Ty>,
) -> Vec<old::play_toClient_packet__tags__items::Ty> {
	for registry in &mut tags {
		for t in &mut registry.tags {
			t.entries = tag_entries(&registry.tagType, from, to, std::mem::take(&mut t.entries));
		}
	}
	tags
}

pub fn trails_tags_1_19_4_to_1_20(
	p: Vec<old::play_toClient_packet__tags__items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	Ok(vec![Client::tags(remap_tags(OLD, NEW, p))])
}

pub fn trails_tags_1_20_to_1_19_4(
	p: Vec<new::play_toClient_packet__tags__items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let mut tags = remap_tags(NEW, OLD, p);
	for registry in tags.iter_mut().filter(|r| r.tagType == "minecraft:block") {
		if !registry
			.tags
			.iter()
			.any(|t| t.tagName == "minecraft:replaceable_plants")
		{
			let entries = registry
				.tags
				.iter()
				.find(|t| t.tagName == "minecraft:replaceable_by_trees")
				.map(|t| t.entries.clone())
				.unwrap_or_default();
			registry
				.tags
				.push(old::play_toClient_packet__tags__items__tags__items::Ty {
					tagName: "minecraft:replaceable_plants".to_owned(),
					entries,
				});
		}
	}
	Ok(vec![ClientOld::tags(tags)])
}

pub fn trails_end_combat_event_1_19_4_to_1_20(
	p: old::play_toClient_packet_end_combat_event::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	Ok(vec![Client::end_combat_event(p.duration)])
}

pub fn trails_end_combat_event_1_20_to_1_19_4(
	duration: i32,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	Ok(vec![ClientOld::end_combat_event(
		old::play_toClient_packet_end_combat_event::Ty {
			duration,
			entityId: -1,
		},
	)])
}

pub fn trails_open_sign_entity_1_19_4_to_1_20(
	location: old::position::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	Ok(vec![Client::open_sign_entity(
		new::play_toClient_packet_open_sign_entity::Ty {
			location,
			isFrontText: true,
		},
	)])
}

pub fn trails_open_sign_entity_1_20_to_1_19_4(
	p: new::play_toClient_packet_open_sign_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	common::state(cx).back_sign = match p.isFrontText {
		true => None,
		false => Some(encode(&p.location, new::position::serialize)?),
	};
	Ok(vec![ClientOld::open_sign_entity(p.location)])
}

pub fn trails_sign_front_1_19_4_to_1_20(
	src: &old::play_toServer_packet_update_sign::Ty,
	cx: &mut Session,
) -> Result<bool> {
	let at = encode(&src.location, old::position::serialize)?;
	Ok(common::state(cx).back_sign.take().is_none_or(|b| b != at))
}

pub fn trails_update_sign_1_20_to_1_19_4(
	p: new::play_toServer_packet_update_sign::Ty,
	_cx: &mut Session,
) -> Result<Vec<old::play_toServer_packet::Ty>> {
	Ok(match p.isFrontText {
		true => vec![old::play_toServer_packet::Ty::update_sign(
			old::play_toServer_packet_update_sign::Ty {
				location: p.location,
				text1: p.text1,
				text2: p.text2,
				text3: p.text3,
				text4: p.text4,
			},
		)],
		false => vec![],
	})
}
