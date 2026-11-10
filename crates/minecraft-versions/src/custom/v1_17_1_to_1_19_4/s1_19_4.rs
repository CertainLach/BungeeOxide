use super::chunk;
use super::meta::{self, Kind, Meta, MINECARTS};
use super::s1_19::{dropped_keys, minecart_block, section_records, world_event, world_particles};
use super::s1_19_3::{block_entities, spawn};
use super::*;
use crate::gen::convert::{v1_19_3, v1_19_4};

type Client193 = v1_19_3::play_toClient_packet::Ty;
type Client194 = v1_19_4::play_toClient_packet::Ty;
type Properties193 = v1_19_3::command_node__extraNodeData__v2__properties::Ty;

const OLD: V = V::V1_19_3;
const NEW: V = V::V1_19_4;

pub fn caves_slot_up_1_19_4(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot(OLD, NEW, v))
}

pub fn caves_slot_down_1_19_3(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot(NEW, OLD, v))
}

pub fn caves_particle_up_1_19_4(
	v: v1_19_3::particle::Ty,
	_cx: &mut Session,
) -> Result<v1_19_4::particle::Ty> {
	particle(
		OLD,
		NEW,
		&v,
		v1_19_3::particle::serialize,
		v1_19_4::particle::deserialize,
	)
}

pub fn caves_particle_down_1_19_3(
	v: v1_19_4::particle::Ty,
	_cx: &mut Session,
) -> Result<v1_19_3::particle::Ty> {
	particle(
		NEW,
		OLD,
		&v,
		v1_19_4::particle::serialize,
		v1_19_3::particle::deserialize,
	)
}

world_particles!(caves_world_particles_up_1_19_4, v1_19_3, v1_19_4, OLD, NEW);
world_particles!(
	caves_world_particles_down_1_19_3,
	v1_19_4,
	v1_19_3,
	NEW,
	OLD
);

pub fn caves_heightmap_parser_down_1_19_3(_p: (), _cx: &mut Session) -> Result<Properties193> {
	Ok(Properties193::brigadier_string(
		v1_19_3::command_node__extraNodeData__v2__properties__brigadier_string::Ty::SINGLE_WORD,
	))
}

pub fn caves_time_parser_down_1_19_3(_min: i32, _cx: &mut Session) -> Result<Properties193> {
	Ok(Properties193::minecraft_time)
}

fn registry_id(version: V, registry: &str, name: &str) -> Option<i32> {
	let Payload::compound(reg) = vanilla_registry(version, registry)? else {
		return None;
	};
	let Some(Payload::list(List {
		items: ListItems::compound(list),
		..
	})) = reg.field("value")
	else {
		return None;
	};
	let e = list
		.iter()
		.find(|e| e.field("name").and_then(Payload::as_str) == Some(name))?;
	e.field("id").and_then(Payload::as_i64).map(|i| i as i32)
}

fn damage_type(status: i8) -> Option<&'static str> {
	Some(match status {
		2 | 44 => "minecraft:generic",
		33 => "minecraft:thorns",
		36 => "minecraft:drown",
		37 => "minecraft:on_fire",
		57 => "minecraft:freeze",
		_ => return None,
	})
}

pub fn caves_entity_status_up_1_19_4(
	p: v1_19_3::play_toClient_packet_entity_status::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client194>> {
	let Some(ty) =
		damage_type(p.entityStatus).and_then(|n| registry_id(NEW, "minecraft:damage_type", n))
	else {
		return Ok(vec![Client194::entity_status(p)]);
	};
	Ok(vec![Client194::damage_event(
		v1_19_4::play_toClient_packet_damage_event::Ty {
			entityId: p.entityId,
			sourceTypeId: ty,
			sourceCauseId: 0,
			sourceDirectId: 0,
			sourcePosition: None,
		},
	)])
}

pub fn caves_damage_event_down_1_19_3(
	p: v1_19_4::play_toClient_packet_damage_event::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client193>> {
	Ok(vec![Client193::entity_status(
		v1_19_3::play_toClient_packet_entity_status::Ty {
			entityId: p.entityId,
			entityStatus: 2,
		},
	)])
}

pub fn caves_animation_up_1_19_4(
	p: v1_19_3::play_toClient_packet_animation::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client194>> {
	Ok(vec![match p.animation {
		1 => Client194::hurt_animation(v1_19_4::play_toClient_packet_hurt_animation::Ty {
			entityId: p.entityId,
			yaw: 0.0,
		}),
		_ => Client194::animation(p),
	}])
}

pub fn caves_hurt_animation_down_1_19_3(
	p: v1_19_4::play_toClient_packet_hurt_animation::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client193>> {
	Ok(vec![Client193::animation(
		v1_19_3::play_toClient_packet_animation::Ty {
			entityId: p.entityId,
			animation: 1,
		},
	)])
}

pub fn caves_position_up_1_19_4(
	p: v1_19_3::play_toClient_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<Client194>> {
	let mut out = Vec::new();
	if p.dismountVehicle {
		if let Some(vehicle) = world(cx).vehicle.take() {
			out.push(Client194::set_passengers(
				v1_19_4::play_toClient_packet_set_passengers::Ty {
					entityId: vehicle,
					passengers: vec![],
				},
			));
		}
	}
	out.push(Client194::position(
		v1_19_4::play_toClient_packet_position::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			flags: p.flags,
			teleportId: p.teleportId,
		},
	));
	Ok(out)
}

pub fn caves_position_down_1_19_3(
	p: v1_19_4::play_toClient_packet_position::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client193>> {
	Ok(vec![Client193::position(
		v1_19_3::play_toClient_packet_position::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			flags: p.flags,
			teleportId: p.teleportId,
			dismountVehicle: false,
		},
	)])
}

pub fn caves_set_passengers_up_1_19_4(
	p: v1_19_3::play_toClient_packet_set_passengers::Ty,
	cx: &mut Session,
) -> Result<v1_19_4::play_toClient_packet_set_passengers::Ty> {
	let w = world(cx);
	if w.vehicle == Some(p.entityId) {
		w.vehicle = None;
	}
	if p.passengers.contains(&w.player) {
		w.vehicle = Some(p.entityId);
	}
	Ok(p)
}

pub fn caves_respawn_up_1_19_4(
	p: v1_19_3::play_toClient_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<v1_19_4::play_toClient_packet_respawn::Ty> {
	world(cx).vehicle = None;
	Ok(p)
}

pub fn caves_codec_up_1_19_4(
	src: &v1_19_3::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	let w = world(cx);
	w.player = src.entityId;
	w.vehicle = None;
	w.enter(&src.dimensionCodec, &src.worldType);
	let mut codec = src.dimensionCodec.clone();
	for b in registry_entries(&mut codec, "minecraft:worldgen/biome")
		.into_iter()
		.flatten()
	{
		if let Some(element) = b.compound_mut("element") {
			let rain = element
				.field("precipitation")
				.and_then(Payload::as_str)
				.is_some_and(|p| p != "none");
			element.put("has_precipitation", Payload::byte(rain as i8));
		}
	}
	for name in [
		"minecraft:damage_type",
		"minecraft:trim_material",
		"minecraft:trim_pattern",
	] {
		put_registry(&mut codec, NEW, name)?;
	}
	Ok(codec)
}

pub fn caves_codec_down_1_19_3(
	src: &v1_19_4::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	world(cx).enter(&src.dimensionCodec, &src.worldType);
	let mut codec = src.dimensionCodec.clone();
	for b in registry_entries(&mut codec, "minecraft:worldgen/biome")
		.into_iter()
		.flatten()
	{
		if let Some(element) = b.compound_mut("element") {
			if element.field("precipitation").is_none() {
				let rain = element
					.field("has_precipitation")
					.and_then(Payload::as_i64)
					.unwrap_or(1) != 0;
				element.put(
					"precipitation",
					Payload::string(if rain { "rain" } else { "none" }.into()),
				);
			}
		}
	}
	if let Payload::compound(root) = &mut codec.value {
		root.retain(|e| {
			!matches!(
				e.name.as_str(),
				"minecraft:damage_type" | "minecraft:trim_material" | "minecraft:trim_pattern"
			)
		});
	}
	Ok(codec)
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const ICON_PREFIX: &str = "data:image/png;base64,";

fn base64_decode(text: &str) -> Option<Vec<u8>> {
	let mut out = Vec::with_capacity(text.len() * 3 / 4);
	let (mut acc, mut bits) = (0u32, 0);
	for c in text
		.bytes()
		.filter(|c| !c.is_ascii_whitespace() && *c != b'=')
	{
		acc = (acc << 6) | BASE64.iter().position(|b| *b == c)? as u32;
		bits += 6;
		if bits >= 8 {
			bits -= 8;
			out.push((acc >> bits) as u8);
		}
	}
	Some(out)
}

fn base64_encode(data: &[u8]) -> String {
	let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
	for chunk in data.chunks(3) {
		let n = chunk
			.iter()
			.enumerate()
			.fold(0u32, |n, (i, b)| n | (*b as u32) << (16 - 8 * i));
		for i in 0..4 {
			match i <= chunk.len() {
				true => out.push(BASE64[(n >> (18 - 6 * i) & 63) as usize] as char),
				false => out.push('='),
			}
		}
	}
	out
}

pub fn caves_icon_up_1_19_4(
	src: &v1_19_3::play_toClient_packet_server_data::Ty,
	_cx: &mut Session,
) -> Result<Option<Vec<u8>>> {
	Ok(src
		.icon
		.as_deref()
		.and_then(|i| base64_decode(i.strip_prefix(ICON_PREFIX).unwrap_or(i))))
}

pub fn caves_icon_down_1_19_3(
	src: &v1_19_4::play_toClient_packet_server_data::Ty,
	_cx: &mut Session,
) -> Result<Option<String>> {
	Ok(src
		.iconBytes
		.as_deref()
		.map(|b| format!("{ICON_PREFIX}{}", base64_encode(b))))
}

pub fn caves_motd_up_1_19_4(
	src: &v1_19_3::play_toClient_packet_server_data::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(src.motd.clone().unwrap_or_else(|| chat_text("")))
}

pub fn caves_motd_down_1_19_3(
	src: &v1_19_4::play_toClient_packet_server_data::Ty,
	_cx: &mut Session,
) -> Result<Option<String>> {
	Ok(Some(src.motd.clone()))
}

const HORSES: &[&str] = &[
	"horse",
	"donkey",
	"mule",
	"llama",
	"trader_llama",
	"skeleton_horse",
	"zombie_horse",
	"camel",
];

pub fn caves_metadata_up_1_19_4(
	src: &v1_19_3::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<v1_19_4::entityMetadata::Ty> {
	let w = world(cx);
	let (horse, boat) = (
		w.is(src.entityId, HORSES),
		w.is(src.entityId, &["boat", "chest_boat"]),
	);
	let mut minecart = minecart_block(OLD, NEW, w.is(src.entityId, MINECARTS));
	let bytes = meta::convert(
		OLD,
		NEW,
		&encode(&src.metadata, v1_19_3::entityMetadata::serialize)?,
		|m| match m.key {
			18 if horse => Ok(None),
			k if horse && k > 18 => Ok(Some(Meta { key: k - 1, ..m })),
			11 if boat => Ok(Some(match m.int() {
				Some(t) if t > 4 => Meta::varint(11, m.kind, t + 1),
				_ => m,
			})),
			_ => minecart(m),
		},
	)?;
	decode(&bytes, v1_19_4::entityMetadata::deserialize)
}

pub fn caves_metadata_down_1_19_3(
	src: &v1_19_4::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<v1_19_3::entityMetadata::Ty> {
	let w = world(cx);
	let (horse, boat) = (
		w.is(src.entityId, HORSES),
		w.is(src.entityId, &["boat", "chest_boat"]),
	);
	let dropped = dropped_keys(w.name(src.entityId), &[("sniffer", &[17, 18])]);
	let mut minecart = minecart_block(NEW, OLD, w.is(src.entityId, MINECARTS));
	let bytes = meta::convert(
		NEW,
		OLD,
		&encode(&src.metadata, v1_19_4::entityMetadata::serialize)?,
		|m| match (m.key, m.kind) {
			(_, Kind::BlockState | Kind::Sniffer | Kind::Vector3 | Kind::Quaternion) => Ok(None),
			(k, _) if dropped.contains(&k) => Ok(None),
			(k, _) if horse && k >= 18 => Ok(Some(Meta { key: k + 1, ..m })),
			(11, _) if boat => Ok(Some(match m.int() {
				Some(5) => Meta::varint(11, m.kind, 0),
				Some(t) if t > 5 => Meta::varint(11, m.kind, t - 1),
				_ => m,
			})),
			_ => minecart(m),
		},
	)?;
	decode(&bytes, v1_19_3::entityMetadata::deserialize)
}

pub fn caves_map_chunk_up_1_19_4(
	mut p: v1_19_3::play_toClient_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<v1_19_4::play_toClient_packet_map_chunk::Ty> {
	p.chunkData = chunk::remap_states(&p.chunkData, world(cx).known_sections(), OLD, NEW)?;
	Ok(p)
}

pub fn caves_map_chunk_down_1_19_3(
	mut p: v1_19_4::play_toClient_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<v1_19_3::play_toClient_packet_map_chunk::Ty> {
	p.chunkData = chunk::remap_states(&p.chunkData, world(cx).known_sections(), NEW, OLD)?;
	p.blockEntities = block_entities(NEW, OLD, p.blockEntities, |b| &mut b.r#type);
	Ok(p)
}

pub fn caves_block_change_up_1_19_4(
	p: v1_19_3::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client194>> {
	Ok(vec![Client194::block_change(
		v1_19_4::play_toClient_packet_block_change::Ty {
			location: p.location,
			r#type: state(OLD, NEW, p.r#type),
		},
	)])
}

pub fn caves_block_change_down_1_19_3(
	p: v1_19_4::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client193>> {
	Ok(vec![Client193::block_change(
		v1_19_3::play_toClient_packet_block_change::Ty {
			location: p.location,
			r#type: state(NEW, OLD, p.r#type),
		},
	)])
}

pub fn caves_section_records_up_1_19_4(
	src: &v1_19_3::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(section_records(OLD, NEW, &src.records)
		.into_iter()
		.map(|r| r as i32)
		.collect())
}

pub fn caves_section_records_down_1_19_3(
	src: &v1_19_4::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<i64>> {
	let records: Vec<i64> = src.records.iter().map(|r| *r as u32 as i64).collect();
	Ok(section_records(NEW, OLD, &records))
}

pub fn caves_spawn_entity_up_1_19_4(
	p: v1_19_3::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<v1_19_4::play_toClient_packet_spawn_entity::Ty> {
	spawn(OLD, NEW, p, cx)
}

pub fn caves_spawn_entity_down_1_19_3(
	p: v1_19_4::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<v1_19_3::play_toClient_packet_spawn_entity::Ty> {
	spawn(NEW, OLD, p, cx)
}

pub fn caves_world_event_data_up_1_19_4(
	src: &v1_19_3::play_toClient_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(world_event(OLD, NEW, src.effectId, src.data))
}

pub fn caves_world_event_data_down_1_19_3(
	src: &v1_19_4::play_toClient_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(world_event(NEW, OLD, src.effectId, src.data))
}

pub fn caves_block_action_id_up_1_19_4(
	src: &v1_19_3::play_toClient_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block(OLD, NEW, src.blockId).ok_or(Error::Dropped)
}

pub fn caves_block_action_id_down_1_19_3(
	src: &v1_19_4::play_toClient_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block(NEW, OLD, src.blockId).ok_or(Error::Dropped)
}

pub fn caves_cooldown_item_up_1_19_4(
	src: &v1_19_3::play_toClient_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	item(OLD, NEW, src.itemID).ok_or(Error::Dropped)
}

pub fn caves_cooldown_item_down_1_19_3(
	src: &v1_19_4::play_toClient_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	item(NEW, OLD, src.itemID).ok_or(Error::Dropped)
}

fn sound_holder(
	from: V,
	to: V,
	holder: &v1_19_3::ItemSoundHolder::Ty,
) -> Result<v1_19_4::ItemSoundHolder::Ty> {
	use v1_19_3::ItemSoundHolder::Ty as Old;
	use v1_19_4::ItemSoundHolder::Ty as New;
	Ok(match holder {
		Old::Inline(e) => New::Inline(e.clone()),
		Old::Id(id) => match sound(from, to, *id) {
			Ok(id) => New::Id(id),
			Err(_) => New::Inline(v1_19_4::ItemSoundEvent::Ty {
				soundName: format!(
					"minecraft:{}",
					registry::names(from, "sounds")
						.and_then(|n| n.name(*id as u32))
						.ok_or(Error::Dropped)?
				),
				fixedRange: None,
			}),
		},
	})
}

pub fn caves_sound_holder_up_1_19_4(
	src: &v1_19_3::play_toClient_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<v1_19_4::ItemSoundHolder::Ty> {
	sound_holder(OLD, NEW, &src.sound)
}

pub fn caves_sound_holder_down_1_19_3(
	src: &v1_19_4::play_toClient_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<v1_19_3::ItemSoundHolder::Ty> {
	sound_holder(NEW, OLD, &src.sound)
}

fn sound_event(
	from: V,
	to: V,
	event: &v1_19_3::play_toClient_packet_entity_sound_effect__soundEvent::Ty,
) -> Result<v1_19_4::play_toClient_packet_entity_sound_effect__soundEvent::Ty> {
	use v1_19_3::play_toClient_packet_entity_sound_effect__soundEvent::Ty as Old;
	use v1_19_4::play_toClient_packet_entity_sound_effect__soundEvent::Ty as New;
	Ok(match event {
		Old::v0(e) => New::v0(e.clone()),
		Old::default(tag) => New::default(sound(from, to, tag - 1)? + 1),
	})
}

pub fn caves_entity_sound_event_up_1_19_4(
	src: &v1_19_3::play_toClient_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<v1_19_4::play_toClient_packet_entity_sound_effect__soundEvent::Ty> {
	sound_event(OLD, NEW, &src.soundEvent)
}

pub fn caves_entity_sound_event_down_1_19_3(
	src: &v1_19_4::play_toClient_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<v1_19_3::play_toClient_packet_entity_sound_effect__soundEvent::Ty> {
	sound_event(NEW, OLD, &src.soundEvent)
}

pub fn caves_tags_up_1_19_4(p: Tags, _cx: &mut Session) -> Result<Vec<Client194>> {
	Ok(vec![Client194::tags(tags(
		OLD,
		NEW,
		p,
		&[],
		super::new_tags::NEW_1_19_4,
	))])
}

pub fn caves_tags_down_1_19_3(p: Tags, _cx: &mut Session) -> Result<Vec<Client193>> {
	Ok(vec![Client193::tags(tags(NEW, OLD, p, &[], &[]))])
}

pub fn caves_statistics_up_1_19_4(p: Stats, _cx: &mut Session) -> Result<Vec<Client194>> {
	Ok(vec![Client194::statistics(statistics(OLD, NEW, p))])
}

pub fn caves_statistics_down_1_19_3(p: Stats, _cx: &mut Session) -> Result<Vec<Client193>> {
	Ok(vec![Client193::statistics(statistics(NEW, OLD, p))])
}
