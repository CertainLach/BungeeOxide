use ::bungeeoxide_nbt::json::{format_uuid, parse_uuid};
use ::bungeeoxide_nbt::{CompoundExt, CompoundVecExt, Entry, Payload, Payload as P};

use super::chunk::{self, F1_15_2, F1_16};
use super::meta::{self, Meta, Slot};
use super::tag_data::NEW_TAGS_1_16;
use super::*;
use crate::gen::convert::{v1_15_2 as old, v1_16 as new};
use crate::pds::convert::Result;
use crate::session::Session;

const FROM: Version = Version::V1_15_2;
const TO: Version = Version::V1_16;

const ATTRIBUTES: &[(&str, &str)] = &[
	("generic.maxHealth", "minecraft:generic.max_health"),
	(
		"zombie.spawnReinforcements",
		"minecraft:zombie.spawn_reinforcements",
	),
	("horse.jumpStrength", "minecraft:horse.jump_strength"),
	("generic.followRange", "minecraft:generic.follow_range"),
	(
		"generic.knockbackResistance",
		"minecraft:generic.knockback_resistance",
	),
	("generic.movementSpeed", "minecraft:generic.movement_speed"),
	("generic.flyingSpeed", "minecraft:generic.flying_speed"),
	("generic.attackDamage", "minecraft:generic.attack_damage"),
	(
		"generic.attackKnockback",
		"minecraft:generic.attack_knockback",
	),
	("generic.attackSpeed", "minecraft:generic.attack_speed"),
	(
		"generic.armorToughness",
		"minecraft:generic.armor_toughness",
	),
];

fn valid_key(key: &str) -> bool {
	let (ns, path) = key.split_once(':').unwrap_or(("minecraft", key));
	let ok = |s: &str, extra: &str| {
		s.chars().all(|c| {
			c.is_ascii_lowercase() || c.is_ascii_digit() || "_-.".contains(c) || extra.contains(c)
		})
	};
	!path.is_empty() && ok(ns, "") && ok(path, "/")
}

fn attribute_up(key: &str) -> Option<String> {
	if let Some((_, new)) = ATTRIBUTES.iter().find(|(old, _)| *old == key) {
		return Some((*new).to_owned());
	}
	let key = if key.contains(':') {
		key.to_owned()
	} else {
		format!("minecraft:{key}")
	};
	valid_key(&key).then_some(key)
}

fn attribute_down(key: &str) -> String {
	let full = if key.contains(':') {
		key.to_owned()
	} else {
		format!("minecraft:{key}")
	};
	ATTRIBUTES
		.iter()
		.find(|(_, new)| *new == full)
		.map_or_else(|| strip(key).to_owned(), |(old, _)| (*old).to_owned())
}

fn uuid_to_ints(c: &mut [Entry], name: &str) {
	if let Some(mut v) = c.field_mut(name) {
		if let Some(uuid) = v.as_str().and_then(parse_uuid) {
			*v = Payload::uuid(uuid);
		}
	}
}

fn uuid_to_string(c: &mut [Entry], name: &str) {
	if let Some(mut v) = c.field_mut(name) {
		if let Some(uuid) = v.as_uuid() {
			*v = P::String(format_uuid(uuid));
		}
	}
}

fn fix_all(c: &mut Vec<Entry>, name: &str, up: bool) {
	let fix = |v: &mut P| {
		if let P::String(s) = v {
			*s = if up {
				attribute_up(s).unwrap_or_else(|| s.clone())
			} else {
				attribute_down(s)
			};
		}
	};
	if let Some(mut v) = c.field_mut(name) {
		fix(&mut v);
	}
}

fn item_up(name: &str, tag: &mut Vec<Entry>) {
	if name == "player_head" {
		if let Some(owner) = tag.compound_mut("SkullOwner") {
			uuid_to_ints(owner, "Id");
		}
	}
	if let Some(list) = tag.compounds_mut("AttributeModifiers") {
		for m in list {
			fix_all(m, "AttributeName", true);
			fix_all(m, "Name", true);
			let most = m.field("UUIDMost").and_then(Payload::as_i64);
			let least = m.field("UUIDLeast").and_then(Payload::as_i64);
			if let (Some(most), Some(least)) = (most, least) {
				m.take("UUIDMost");
				m.take("UUIDLeast");
				let uuid = ((most as u64 as u128) << 64) | least as u64 as u128;
				m.put("UUID", Payload::uuid(uuid));
			}
		}
	}
}

fn legacy_texts(tag: &mut [Entry]) {
	if let Some(display) = tag.compound_mut("display") {
		if let Some(P::String(name)) = display.field_mut("Name").as_deref_mut() {
			*name = text::legacy(name);
		}
		if let Some(P::List(::bungeeoxide_nbt::List {
			items: ::bungeeoxide_nbt::ListItems::String(lines),
			..
		})) = display.field_mut("Lore").as_deref_mut()
		{
			lines.iter_mut().for_each(|l| *l = text::legacy(l));
		}
	}
}

fn item_down(name: &str, tag: &mut Vec<Entry>) {
	legacy_texts(tag);
	if name == "written_book" {
		if let Some(P::List(::bungeeoxide_nbt::List {
			items: ::bungeeoxide_nbt::ListItems::String(pages),
			..
		})) = tag.field_mut("pages").as_deref_mut()
		{
			pages.iter_mut().for_each(|l| *l = text::legacy(l));
		}
	}
	if name == "player_head" {
		if let Some(owner) = tag.compound_mut("SkullOwner") {
			uuid_to_string(owner, "Id");
		}
	}
	if let Some(list) = tag.compounds_mut("AttributeModifiers") {
		for m in list {
			fix_all(m, "AttributeName", false);
			fix_all(m, "Name", false);
			if let Some(uuid) = m.field("UUID").and_then(Payload::as_uuid) {
				m.take("UUID");
				m.put("UUIDMost", P::Long((uuid >> 64) as u64 as i64));
				m.put("UUIDLeast", P::Long(uuid as u64 as i64));
			}
		}
	}
}

fn slot_up(v: Slot) -> Slot {
	meta::slot(v, FROM, TO, item_up)
}

fn slot_down(v: Slot) -> Slot {
	meta::slot(v, TO, FROM, item_down)
}

fn block_entity(mut e: Entry, up: bool) -> Entry {
	let P::Compound(c) = &mut e.value else {
		return e;
	};
	let id = c
		.field("id")
		.and_then(Payload::as_str)
		.map(|s| strip(s).to_owned())
		.unwrap_or_default();
	match (id.as_str(), up) {
		("conduit", true) => {
			if let Some(uuid) = c
				.take("target_uuid")
				.as_ref()
				.and_then(Payload::as_str)
				.and_then(parse_uuid)
			{
				c.put("Target", Payload::uuid(uuid));
			}
		}
		("conduit", false) => {
			if let Some(target @ P::IntArray(_)) = c.take("Target") {
				if let Some(uuid) = target.as_uuid() {
					c.put("target_uuid", P::String(format_uuid(uuid)));
				}
			}
		}
		("skull", true) => {
			if let Some(P::Compound(mut owner)) = c.take("Owner") {
				uuid_to_ints(&mut owner, "Id");
				c.put("SkullOwner", P::Compound(owner));
			}
		}
		("skull", false) => {
			if let Some(P::Compound(mut owner)) = c.take("SkullOwner") {
				uuid_to_string(&mut owner, "Id");
				c.put("Owner", P::Compound(owner));
			}
		}
		("sign", false) => {
			for line in ["Text1", "Text2", "Text3", "Text4"] {
				if let Some(P::String(t)) = c.field_mut(line).as_deref_mut() {
					*t = text::legacy(t);
				}
			}
		}
		("mob_spawner", _) => {
			let (from, to) = if up {
				("minecraft:zombie_pigman", "minecraft:zombified_piglin")
			} else {
				("minecraft:zombified_piglin", "minecraft:zombie_pigman")
			};
			if let Some(P::String(id)) = c
				.compound_mut("SpawnData")
				.and_then(|d| d.field_mut("id"))
				.as_deref_mut()
			{
				if id == from {
					*id = to.to_owned();
				}
			}
		}
		_ => {}
	}
	e
}

fn dimension_name(id: i32) -> &'static str {
	match id {
		-1 => "minecraft:the_nether",
		1 => "minecraft:the_end",
		_ => "minecraft:overworld",
	}
}

fn dimension_id(name: &str) -> i32 {
	match name {
		"minecraft:the_nether" => -1,
		"minecraft:the_end" => 1,
		_ => 0,
	}
}

fn level_type(flat: bool) -> String {
	if flat { "flat" } else { "default" }.to_owned()
}

pub fn nether_slot_1_16(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot_up(v))
}

pub fn nether_slot_1_15_2(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot_down(v))
}

pub fn nether_uuid_1_16(v: String, _cx: &mut Session) -> Result<u128> {
	match parse_uuid(&v) {
		Some(uuid) => Ok(uuid),
		None => unconvertible(format!("login uuid {v:?} is not a UUID")),
	}
}

pub fn nether_uuid_1_15_2(v: u128, _cx: &mut Session) -> Result<String> {
	Ok(format_uuid(v))
}

pub fn nether_chat_1_16(
	v: old::play_to_client_packet_chat::Ty,
	_cx: &mut Session,
) -> Result<new::play_to_client_packet_chat::Ty> {
	Ok(new::play_to_client_packet_chat::Ty {
		message: v.message,
		position: v.position,
		sender: 0,
	})
}

pub fn nether_chat_1_15_2(
	v: new::play_to_client_packet_chat::Ty,
	_cx: &mut Session,
) -> Result<old::play_to_client_packet_chat::Ty> {
	Ok(old::play_to_client_packet_chat::Ty {
		message: text::legacy(&v.message),
		position: v.position,
	})
}

pub fn nether_spawn_entity_weather_1_16(
	p: old::play_to_client_packet_spawn_entity_weather::Ty,
	cx: &mut Session,
) -> Result<Vec<new::play_to_client_packet::Ty>> {
	let Some(ty) = entity_id(TO, "lightning_bolt").filter(|_| p.r#type == 1) else {
		return Ok(vec![]);
	};
	track(
		cx,
		TO,
		p.entity_id,
		Some(Kind {
			name: "lightning_bolt",
			original: "lightning_bolt",
			substitute: false,
		}),
	);
	Ok(vec![new::play_to_client_packet::Ty::SpawnEntity(
		new::play_to_client_packet_spawn_entity::Ty {
			entity_id: p.entity_id,
			object_uuid: 0x7064_7332_0000_4000_8000_0001_0000_0000 | p.entity_id as u32 as u128,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			pitch: 0,
			yaw: 0,
			object_data: 0,
			velocity: new::vec3i16::Ty { x: 0, y: 0, z: 0 },
		},
	)])
}

pub fn nether_entity_equipment_1_16(
	p: old::play_to_client_packet_entity_equipment::Ty,
	_cx: &mut Session,
) -> Result<Vec<new::play_to_client_packet::Ty>> {
	Ok(vec![new::play_to_client_packet::Ty::EntityEquipment(
		new::play_to_client_packet_entity_equipment::Ty {
			entity_id: p.entity_id,
			equipments: vec![
				new::play_to_client_packet_entity_equipment_equipments_items::Ty {
					slot: p.slot as i8,
					item: slot_up(p.item),
				},
			],
		},
	)])
}

pub fn nether_entity_equipment_1_15_2(
	p: new::play_to_client_packet_entity_equipment::Ty,
	_cx: &mut Session,
) -> Result<Vec<old::play_to_client_packet::Ty>> {
	Ok(p.equipments
		.into_iter()
		.map(|e| {
			old::play_to_client_packet::Ty::EntityEquipment(
				old::play_to_client_packet_entity_equipment::Ty {
					entity_id: p.entity_id,
					slot: e.slot as i32,
					item: slot_down(e.item),
				},
			)
		})
		.collect())
}

pub fn nether_spawn_entity_1_16(
	p: SpawnObject,
	cx: &mut Session,
) -> Result<Vec<new::play_to_client_packet::Ty>> {
	Ok(spawn_object(p, FROM, TO, cx)
		.map(new::play_to_client_packet::Ty::SpawnEntity)
		.into_iter()
		.collect())
}

pub fn nether_spawn_entity_1_15_2(
	p: SpawnObject,
	cx: &mut Session,
) -> Result<Vec<old::play_to_client_packet::Ty>> {
	if entity_name(TO, p.r#type) == Some("lightning_bolt") {
		track(cx, TO, p.entity_id, None);
		return Ok(vec![old::play_to_client_packet::Ty::SpawnEntityWeather(
			old::play_to_client_packet_spawn_entity_weather::Ty {
				entity_id: p.entity_id,
				r#type: 1,
				x: p.x,
				y: p.y,
				z: p.z,
			},
		)]);
	}
	Ok(spawn_object(p, TO, FROM, cx)
		.map(old::play_to_client_packet::Ty::SpawnEntity)
		.into_iter()
		.collect())
}

pub fn nether_spawn_entity_living_1_16(p: SpawnMob, cx: &mut Session) -> Result<SpawnMob> {
	spawn_mob(p, FROM, TO, cx)
}

pub fn nether_spawn_entity_living_1_15_2(p: SpawnMob, cx: &mut Session) -> Result<SpawnMob> {
	spawn_mob(p, TO, FROM, cx)
}

pub fn nether_named_entity_spawn_1_16(
	p: new::play_to_client_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<new::play_to_client_packet_named_entity_spawn::Ty> {
	player(cx, TO, p.entity_id);
	Ok(p)
}

pub fn nether_named_entity_spawn_1_15_2(
	p: new::play_to_client_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<new::play_to_client_packet_named_entity_spawn::Ty> {
	player(cx, TO, p.entity_id);
	Ok(p)
}

pub fn nether_entity_destroy_1_16(
	p: Vec<i32>,
	cx: &mut Session,
) -> Result<Vec<new::play_to_client_packet::Ty>> {
	untrack(cx, &p);
	Ok(vec![new::play_to_client_packet::Ty::EntityDestroy(p)])
}

pub fn nether_entity_destroy_1_15_2(
	p: Vec<i32>,
	cx: &mut Session,
) -> Result<Vec<old::play_to_client_packet::Ty>> {
	untrack(cx, &p);
	Ok(vec![old::play_to_client_packet::Ty::EntityDestroy(p)])
}

pub fn nether_statistics_1_16(
	p: Vec<Stat>,
	_cx: &mut Session,
) -> Result<Vec<new::play_to_client_packet::Ty>> {
	Ok(vec![new::play_to_client_packet::Ty::Statistics(
		statistics(p, FROM, TO),
	)])
}

pub fn nether_statistics_1_15_2(
	p: Vec<Stat>,
	_cx: &mut Session,
) -> Result<Vec<old::play_to_client_packet::Ty>> {
	Ok(vec![old::play_to_client_packet::Ty::Statistics(
		statistics(p, TO, FROM),
	)])
}

pub fn nether_login_1_16(
	v: old::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<new::play_to_client_packet_login::Ty> {
	player(cx, TO, v.entity_id);
	let name = dimension_name(v.dimension).to_owned();
	Ok(new::play_to_client_packet_login::Ty {
		entity_id: v.entity_id,
		game_mode: v.game_mode,
		previous_game_mode: 255,
		world_names: [-1, 0, 1].map(|d| dimension_name(d).to_owned()).to_vec(),
		dimension_codec: codec(TO)?,
		dimension: name.clone(),
		world_name: name,
		hashed_seed: v.hashed_seed,
		max_players: v.max_players,
		view_distance: v.view_distance,
		reduced_debug_info: v.reduced_debug_info,
		enable_respawn_screen: v.enable_respawn_screen,
		is_debug: false,
		is_flat: v.level_type == "flat",
	})
}

#[derive(Default)]
struct Client15 {
	entity: i32,
	dimension: Option<i32>,
	world: String,
	attributes: Vec<new::play_to_client_packet_entity_update_attributes_properties_items::Ty>,
	sneaking: bool,
}

pub fn nether_login_1_15_2(
	v: new::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<old::play_to_client_packet_login::Ty> {
	player(cx, TO, v.entity_id);
	*cx.state::<Client15>() = Client15 {
		entity: v.entity_id,
		dimension: Some(dimension_id(&v.dimension)),
		world: v.world_name.clone(),
		..Client15::default()
	};
	Ok(old::play_to_client_packet_login::Ty {
		entity_id: v.entity_id,
		game_mode: v.game_mode,
		dimension: dimension_id(&v.dimension),
		hashed_seed: v.hashed_seed,
		max_players: v.max_players,
		level_type: level_type(v.is_flat),
		view_distance: v.view_distance,
		reduced_debug_info: v.reduced_debug_info,
		enable_respawn_screen: v.enable_respawn_screen,
	})
}

fn respawn_15(
	dimension: i32,
	v: &new::play_to_client_packet_respawn::Ty,
) -> old::play_to_client_packet::Ty {
	old::play_to_client_packet::Ty::Respawn(old::play_to_client_packet_respawn::Ty {
		dimension,
		hashed_seed: v.hashed_seed,
		gamemode: v.gamemode,
		level_type: level_type(v.is_flat),
	})
}

pub fn nether_respawn_1_16(
	v: old::play_to_client_packet_respawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<new::play_to_client_packet::Ty>> {
	let name = dimension_name(v.dimension).to_owned();
	Ok(vec![new::play_to_client_packet::Ty::Respawn(
		new::play_to_client_packet_respawn::Ty {
			dimension: name.clone(),
			world_name: name,
			hashed_seed: v.hashed_seed,
			gamemode: v.gamemode,
			previous_gamemode: 255,
			is_debug: false,
			is_flat: v.level_type == "flat",
			copy_metadata: true,
		},
	)])
}

pub fn nether_respawn_1_15_2(
	v: new::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Vec<old::play_to_client_packet::Ty>> {
	let dimension = dimension_id(&v.dimension);
	let state = cx.state::<Client15>();
	let mut out = Vec::new();
	if state.dimension == Some(dimension) && state.world != v.world_name {
		out.push(respawn_15(if dimension == 0 { -1 } else { 0 }, &v));
	}
	out.push(respawn_15(dimension, &v));
	if v.copy_metadata && !state.attributes.is_empty() {
		out.push(old::play_to_client_packet::Ty::EntityUpdateAttributes(
			old::play_to_client_packet_entity_update_attributes::Ty {
				entity_id: state.entity,
				properties: state.attributes.clone(),
			},
		));
	} else {
		state.attributes.clear();
	}
	state.dimension = Some(dimension);
	state.world = v.world_name;
	Ok(out)
}

pub fn nether_block_entity_1_16(e: Entry, up: bool) -> Entry {
	block_entity(e, up)
}

pub fn nether_heightmaps_1_16(heightmaps: &mut Entry, up: bool) {
	match up {
		true => chunk::heightmaps(heightmaps, (9, true), (9, false), 0, 256),
		false => chunk::heightmaps(heightmaps, (9, false), (9, true), 0, 256),
	}
}

pub fn nether_map_chunk_1_16(
	v: old::play_to_client_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<new::play_to_client_packet_map_chunk::Ty> {
	use new::play_to_client_packet_map_chunk_biomes::Ty as B;
	let mut heightmaps = v.heightmaps;
	nether_heightmaps_1_16(&mut heightmaps, true);
	let present = (v.bit_map as u32).count_ones() as usize;
	Ok(new::play_to_client_packet_map_chunk::Ty {
		x: v.x,
		z: v.z,
		ignore_old_data: matches!(
			v.biomes,
			old::play_to_client_packet_map_chunk_biomes::Ty::True(_)
		),
		bit_map: v.bit_map,
		heightmaps,
		biomes: match v.biomes {
			old::play_to_client_packet_map_chunk_biomes::Ty::True(b) => B::True(b),
			old::play_to_client_packet_map_chunk_biomes::Ty::False => B::False,
		},
		chunk_data: chunk::convert_sections(&v.chunk_data, present, F1_15_2, F1_16)?,
		block_entities: v
			.block_entities
			.into_iter()
			.map(|e| block_entity(e, true))
			.collect(),
	})
}

pub fn nether_map_chunk_1_15_2(
	v: new::play_to_client_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<old::play_to_client_packet_map_chunk::Ty> {
	use old::play_to_client_packet_map_chunk_biomes::Ty as B;
	let mut heightmaps = v.heightmaps;
	nether_heightmaps_1_16(&mut heightmaps, false);
	let present = (v.bit_map as u32).count_ones() as usize;
	Ok(old::play_to_client_packet_map_chunk::Ty {
		x: v.x,
		z: v.z,
		bit_map: v.bit_map,
		heightmaps,
		biomes: match v.biomes {
			new::play_to_client_packet_map_chunk_biomes::Ty::True(b) => B::True(b),
			new::play_to_client_packet_map_chunk_biomes::Ty::False => B::False,
		},
		chunk_data: chunk::convert_sections(&v.chunk_data, present, F1_16, F1_15_2)?,
		block_entities: v
			.block_entities
			.into_iter()
			.map(|e| block_entity(e, false))
			.collect(),
	})
}

pub fn nether_world_particles_1_16(
	v: old::play_to_client_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<new::play_to_client_packet_world_particles::Ty> {
	let p = kept(meta::particle(
		meta::particle_15(v.particle_id, v.data),
		FROM,
		TO,
		slot_up,
	))?;
	Ok(new::play_to_client_packet_world_particles::Ty {
		particle_id: p.id,
		data: kept(meta::data_16(p))?,
		long_distance: v.long_distance,
		x: v.x,
		y: v.y,
		z: v.z,
		offset_x: v.offset_x,
		offset_y: v.offset_y,
		offset_z: v.offset_z,
		particle_data: v.particle_data,
		particles: v.particles,
	})
}

pub fn nether_world_particles_1_15_2(
	v: new::play_to_client_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<old::play_to_client_packet_world_particles::Ty> {
	let p = kept(meta::particle(
		meta::particle_16(v.particle_id, v.data),
		TO,
		FROM,
		slot_down,
	))?;
	Ok(old::play_to_client_packet_world_particles::Ty {
		particle_id: p.id,
		data: kept(meta::data_15(p))?,
		long_distance: v.long_distance,
		x: v.x,
		y: v.y,
		z: v.z,
		offset_x: v.offset_x,
		offset_y: v.offset_y,
		offset_z: v.offset_z,
		particle_data: v.particle_data,
		particles: v.particles,
	})
}

pub fn nether_entity_metadata_1_16(
	v: old::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<new::play_to_client_packet_entity_metadata::Ty> {
	let kind = kind_of(cx, TO, v.entity_id).name;
	let mut items = meta::remap_values(meta::decode_15(v.metadata), FROM, TO, &mut slot_up);
	if meta::is_arrow(kind) {
		items.retain(|(k, _)| *k != 8);
		for (k, _) in items.iter_mut().filter(|(k, _)| *k > 8) {
			*k -= 1;
		}
	}
	if kind == "wolf" {
		if let Some(flags) = items.iter().find_map(|(k, m)| match (k, m) {
			(16, Meta::Byte(f)) => Some(*f),
			_ => None,
		}) {
			items.push((20, Meta::VarInt(if flags & 2 != 0 { i32::MAX } else { 0 })));
		}
	}
	meta::minecart_block(&mut items, kind, 10, FROM, TO);
	Ok(new::play_to_client_packet_entity_metadata::Ty {
		entity_id: v.entity_id,
		metadata: meta::encode_16(items),
	})
}

pub fn nether_entity_metadata_1_15_2(
	v: new::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<old::play_to_client_packet_entity_metadata::Ty> {
	let tracked = kind_of(cx, TO, v.entity_id);
	let kind = tracked.name;
	let mut items = meta::remap_values(meta::decode_16(v.metadata), TO, FROM, &mut slot_down);
	if meta::is_arrow(kind) {
		for (k, _) in items.iter_mut().filter(|(k, _)| *k >= 8) {
			*k += 1;
		}
	}
	if kind == "fishing_bobber" {
		items.retain(|(k, _)| *k != 8);
	}
	if kind == "wolf" {
		let angry = items
			.iter()
			.any(|(k, m)| matches!((k, m), (20, Meta::VarInt(t)) if *t > 0));
		items.retain(|(k, _)| *k != 20);
		for (k, m) in items.iter_mut() {
			if let (16, Meta::Byte(f)) = (*k, &mut *m) {
				*f = if angry { *f | 2 } else { *f & !2 };
			}
		}
	}
	meta::minecart_block(&mut items, kind, 10, TO, FROM);
	meta::substituted(&mut items, tracked);
	meta::legacy_texts(&mut items);
	Ok(old::play_to_client_packet_entity_metadata::Ty {
		entity_id: v.entity_id,
		metadata: meta::encode_15(items),
	})
}

pub fn nether_block_change_1_16(v: BlockChange, _cx: &mut Session) -> Result<BlockChange> {
	Ok(block_change(v, FROM, TO))
}

pub fn nether_block_change_1_15_2(v: BlockChange, _cx: &mut Session) -> Result<BlockChange> {
	Ok(block_change(v, TO, FROM))
}

type MultiBlock = new::play_to_client_packet_multi_block_change::Ty;

fn multi_block(mut v: MultiBlock, from: Version, to: Version) -> MultiBlock {
	for r in &mut v.records {
		r.block_id = state(from, to, r.block_id);
	}
	v
}

pub fn nether_multi_block_change_1_16(v: MultiBlock, _cx: &mut Session) -> Result<MultiBlock> {
	Ok(multi_block(v, FROM, TO))
}

pub fn nether_multi_block_change_1_15_2(v: MultiBlock, _cx: &mut Session) -> Result<MultiBlock> {
	Ok(multi_block(v, TO, FROM))
}

pub fn nether_acknowledge_player_digging_1_16(v: Digging, _cx: &mut Session) -> Result<Digging> {
	Ok(digging(v, FROM, TO))
}

pub fn nether_acknowledge_player_digging_1_15_2(v: Digging, _cx: &mut Session) -> Result<Digging> {
	Ok(digging(v, TO, FROM))
}

pub fn nether_block_action_1_16(v: BlockAction, _cx: &mut Session) -> Result<BlockAction> {
	block_action(v, FROM, TO)
}

pub fn nether_block_action_1_15_2(v: BlockAction, _cx: &mut Session) -> Result<BlockAction> {
	block_action(v, TO, FROM)
}

pub fn nether_world_event_1_16(v: WorldEvent, _cx: &mut Session) -> Result<WorldEvent> {
	world_event(v, FROM, TO)
}

pub fn nether_world_event_1_15_2(v: WorldEvent, _cx: &mut Session) -> Result<WorldEvent> {
	world_event(v, TO, FROM)
}

pub fn nether_sound_effect_1_16(v: Sound, _cx: &mut Session) -> Result<Sound> {
	sound_effect(v, FROM, TO)
}

pub fn nether_sound_effect_1_15_2(v: Sound, _cx: &mut Session) -> Result<Sound> {
	sound_effect(v, TO, FROM)
}

pub fn nether_entity_sound_effect_1_16(v: EntitySound, _cx: &mut Session) -> Result<EntitySound> {
	entity_sound(v, FROM, TO)
}

pub fn nether_entity_sound_effect_1_15_2(v: EntitySound, _cx: &mut Session) -> Result<EntitySound> {
	entity_sound(v, TO, FROM)
}

pub fn nether_set_cooldown_1_16(v: Cooldown, _cx: &mut Session) -> Result<Cooldown> {
	cooldown(v, FROM, TO)
}

pub fn nether_set_cooldown_1_15_2(v: Cooldown, _cx: &mut Session) -> Result<Cooldown> {
	cooldown(v, TO, FROM)
}

pub fn nether_tags_1_16(v: Tags, _cx: &mut Session) -> Result<Tags> {
	Ok(all_tags(v, FROM, TO, NEW_TAGS_1_16, &[]))
}

pub fn nether_tags_1_15_2(v: Tags, _cx: &mut Session) -> Result<Tags> {
	Ok(all_tags(v, TO, FROM, &[], &[]))
}

type OpenWindow = new::play_to_client_packet_open_window::Ty;

pub fn nether_open_window_1_16(mut v: OpenWindow, _cx: &mut Session) -> Result<OpenWindow> {
	if v.inventory_type >= 20 {
		v.inventory_type += 1;
	}
	Ok(v)
}

pub fn nether_open_window_1_15_2(mut v: OpenWindow, _cx: &mut Session) -> Result<OpenWindow> {
	v.window_title = text::legacy(&v.window_title);
	match v.inventory_type {
		20 => v.inventory_type = 7,
		t if t > 20 => v.inventory_type -= 1,
		_ => {}
	}
	Ok(v)
}

type ContainerData = new::play_to_client_packet_craft_progress_bar::Ty;

fn enchantment_hint(mut v: ContainerData, from: Version, to: Version) -> ContainerData {
	if (4..=6).contains(&v.property) && v.value >= 0 {
		v.value = direct("enchantments", from, to, v.value as i32).map_or(-1, |e| e as i16);
	}
	v
}

pub fn nether_craft_progress_bar_1_16(
	v: ContainerData,
	_cx: &mut Session,
) -> Result<ContainerData> {
	Ok(enchantment_hint(v, FROM, TO))
}

pub fn nether_craft_progress_bar_1_15_2(
	v: ContainerData,
	_cx: &mut Session,
) -> Result<ContainerData> {
	Ok(enchantment_hint(v, TO, FROM))
}

type Attributes = new::play_to_client_packet_entity_update_attributes::Ty;

pub fn nether_entity_update_attributes_1_16(
	mut v: Attributes,
	_cx: &mut Session,
) -> Result<Attributes> {
	v.properties = v
		.properties
		.into_iter()
		.filter_map(|mut p| {
			p.key = attribute_up(&p.key)?;
			Some(p)
		})
		.collect();
	Ok(v)
}

pub fn nether_entity_update_attributes_1_15_2(
	mut v: Attributes,
	cx: &mut Session,
) -> Result<Attributes> {
	for p in &mut v.properties {
		p.key = attribute_down(&p.key);
	}
	let state = cx.state::<Client15>();
	if v.entity_id == state.entity {
		for p in &v.properties {
			state.attributes.retain(|a| a.key != p.key);
			state.attributes.push(p.clone());
		}
	}
	Ok(v)
}

type TileEntity = new::play_to_client_packet_tile_entity_data::Ty;

pub fn nether_tile_entity_data_1_16(mut v: TileEntity, _cx: &mut Session) -> Result<TileEntity> {
	v.nbt_data = v.nbt_data.map(|e| block_entity(e, true));
	Ok(v)
}

pub fn nether_tile_entity_data_1_15_2(mut v: TileEntity, _cx: &mut Session) -> Result<TileEntity> {
	v.nbt_data = v.nbt_data.map(|e| block_entity(e, false));
	Ok(v)
}

#[derive(Clone, Copy)]
struct Abilities {
	flying: f32,
	walking: f32,
}

impl Default for Abilities {
	fn default() -> Self {
		Self {
			flying: 0.05,
			walking: 0.1,
		}
	}
}

type ClientAbilities = new::play_to_client_packet_abilities::Ty;

pub fn nether_abilities_1_16(v: ClientAbilities, cx: &mut Session) -> Result<ClientAbilities> {
	*cx.state::<Abilities>() = Abilities {
		flying: v.flying_speed,
		walking: v.walking_speed,
	};
	Ok(v)
}

pub fn nether_abilities_flags_1_16(
	v: old::play_to_server_packet_abilities::Ty,
	_cx: &mut Session,
) -> Result<i8> {
	Ok(v.flags)
}

pub fn nether_abilities_1_15_2(
	flags: i8,
	cx: &mut Session,
) -> Result<old::play_to_server_packet_abilities::Ty> {
	let a = *cx.state::<Abilities>();
	Ok(old::play_to_server_packet_abilities::Ty {
		flags,
		flying_speed: a.flying,
		walking_speed: a.walking,
	})
}

pub fn nether_kick_disconnect_1_16(
	reason: String,
	_cx: &mut Session,
) -> Result<Vec<new::play_to_client_packet::Ty>> {
	Ok(vec![new::play_to_client_packet::Ty::KickDisconnect(reason)])
}

pub fn nether_kick_disconnect_1_15_2(
	reason: String,
	_cx: &mut Session,
) -> Result<Vec<old::play_to_client_packet::Ty>> {
	Ok(vec![old::play_to_client_packet::Ty::KickDisconnect(
		text::legacy(&reason),
	)])
}

pub fn nether_login_disconnect_1_16(
	reason: String,
	_cx: &mut Session,
) -> Result<Vec<new::login_to_client_packet::Ty>> {
	Ok(vec![new::login_to_client_packet::Ty::Disconnect(reason)])
}

pub fn nether_login_disconnect_1_15_2(
	reason: String,
	_cx: &mut Session,
) -> Result<Vec<old::login_to_client_packet::Ty>> {
	Ok(vec![old::login_to_client_packet::Ty::Disconnect(
		text::legacy(&reason),
	)])
}

type Title = new::play_to_client_packet_title::Ty;

pub fn nether_title_1_15_2(v: Title, _cx: &mut Session) -> Result<Title> {
	Ok(match v {
		Title::V0(t) => Title::V0(text::legacy(&t)),
		Title::V1(t) => Title::V1(text::legacy(&t)),
		Title::V2(t) => Title::V2(text::legacy(&t)),
		v => v,
	})
}

type Header = new::play_to_client_packet_playerlist_header::Ty;

pub fn nether_playerlist_header_1_15_2(v: Header, _cx: &mut Session) -> Result<Header> {
	Ok(Header {
		header: text::legacy(&v.header),
		footer: text::legacy(&v.footer),
	})
}

type BossBar = new::play_to_client_packet_boss_bar::Ty;

pub fn nether_boss_bar_1_15_2(mut v: BossBar, _cx: &mut Session) -> Result<BossBar> {
	use new::play_to_client_packet_boss_bar_action::Ty as A;
	match &mut v.action {
		A::V0(add) => add.title = text::legacy(&add.title),
		A::V3(title) => *title = text::legacy(title),
		_ => {}
	}
	Ok(v)
}

type Combat = new::play_to_client_packet_combat_event::Ty;

pub fn nether_combat_event_1_15_2(mut v: Combat, _cx: &mut Session) -> Result<Combat> {
	if let Combat::V2(death) = &mut v {
		death.message = text::legacy(&death.message);
	}
	Ok(v)
}

type Objective = new::play_to_client_packet_scoreboard_objective::Ty;

pub fn nether_scoreboard_objective_1_15_2(
	mut v: Objective,
	_cx: &mut Session,
) -> Result<Objective> {
	use new::play_to_client_packet_scoreboard_objective_action::Ty as A;
	match &mut v.action {
		A::V0(a) => a.display_text = text::legacy(&a.display_text),
		A::V2(a) => a.display_text = text::legacy(&a.display_text),
		_ => {}
	}
	Ok(v)
}

type Teams = new::play_to_client_packet_teams::Ty;

pub fn nether_teams_1_15_2(mut v: Teams, _cx: &mut Session) -> Result<Teams> {
	use new::play_to_client_packet_teams_mode::Ty as M;
	match &mut v.mode {
		M::V0(t) => {
			(t.name, t.prefix, t.suffix) = (
				text::legacy(&t.name),
				text::legacy(&t.prefix),
				text::legacy(&t.suffix),
			)
		}
		M::V2(t) => {
			(t.name, t.prefix, t.suffix) = (
				text::legacy(&t.name),
				text::legacy(&t.prefix),
				text::legacy(&t.suffix),
			)
		}
		_ => {}
	}
	Ok(v)
}

type PlayerInfo = new::play_to_client_packet_player_info::Ty;

pub fn nether_player_info_1_15_2(mut v: PlayerInfo, _cx: &mut Session) -> Result<PlayerInfo> {
	use new::play_to_client_packet_player_info_data_items_anon::Ty as A;
	for d in &mut v.data {
		match &mut d.anon {
			A::AddPlayer(p) => p.display_name = text::legacy_opt(p.display_name.take()),
			A::UpdateDisplayName(name) => *name = text::legacy_opt(name.take()),
			_ => {}
		}
	}
	Ok(v)
}

const MAP_COLORS_1_15_2: [u8; 28] = [
	113, 114, 114, 112, 152, 83, 83, 155, 143, 115, 115, 143, 127, 127, 127, 95, 127, 127, 124, 95,
	187, 155, 184, 187, 127, 124, 125, 127,
];

type Map = new::play_to_client_packet_map::Ty;

pub fn nether_map_1_15_2(mut v: Map, _cx: &mut Session) -> Result<Map> {
	for icon in &mut v.icons {
		icon.display_name = text::legacy_opt(icon.display_name.take());
	}
	if let new::play_to_client_packet_map_columns::Ty::Default(_, c) = &mut v.columns {
		for b in &mut c.data {
			if let Some(m) = (*b as usize)
				.checked_sub(208)
				.and_then(|i| MAP_COLORS_1_15_2.get(i))
			{
				*b = *m;
			}
		}
	}
	Ok(v)
}

type Commands = new::play_to_client_packet_declare_commands::Ty;

pub fn nether_declare_commands_1_15_2(mut v: Commands, _cx: &mut Session) -> Result<Commands> {
	use new::command_node_extra_node_data::Ty as E;
	use new::command_node_extra_node_data_v2_properties::Ty as Props;
	for node in &mut v.nodes {
		if let E::V2(arg) = &mut node.extra_node_data {
			if matches!(arg.properties, Props::MinecraftUuid) {
				arg.parser = "brigadier:string".to_owned();
				arg.properties = Props::BrigadierString(new::command_node_extra_node_data_v2_properties_brigadier_string::Ty::SingleWord);
			}
		}
	}
	Ok(v)
}

type EntityAction = old::play_to_server_packet_entity_action::Ty;

pub fn nether_entity_action_1_16(v: EntityAction, cx: &mut Session) -> Result<EntityAction> {
	use old::play_to_server_packet_entity_action_action_id::Ty as A;
	match v.action_id {
		A::StartSneaking => cx.state::<Client15>().sneaking = true,
		A::StopSneaking => cx.state::<Client15>().sneaking = false,
		_ => {}
	}
	Ok(v)
}

pub fn nether_sneaking_1_16(
	_src: &old::play_to_server_packet_use_entity::Ty,
	cx: &mut Session,
) -> Result<bool> {
	Ok(cx.state::<Client15>().sneaking)
}
