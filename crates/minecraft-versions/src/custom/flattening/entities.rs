use std::collections::HashMap;

use super::blocks::{packed_of_state, state_earlier, state_later, state_of_packed};
use super::items::{slot_down, slot_earlier, slot_later, slot_up};
use super::text::{json_to_legacy, legacy_to_json};
use super::world::{particle_data_down, particle_data_earlier, particle_data_later, particle_up};
use crate::gen::convert::{v1_12_2 as a, v1_13 as b};
use crate::pds::convert::{Error, Result};
use crate::session::Session;

type M12 = a::entity_metadata_item::Ty;
type M13 = b::entity_metadata_item::Ty;
type Meta12 = a::entity_metadata::Ty;
type Meta13 = b::entity_metadata::Ty;

pub(super) const MOBS: &[(i32, &str, i32)] = &[
	(1, "item", 32),
	(2, "xp_orb", 22),
	(3, "area_effect_cloud", 0),
	(4, "elder_guardian", 15),
	(5, "wither_skeleton", 84),
	(6, "stray", 71),
	(7, "egg", 74),
	(8, "leash_knot", 35),
	(9, "painting", 49),
	(10, "arrow", 2),
	(11, "snowball", 67),
	(12, "fireball", 34),
	(13, "small_fireball", 65),
	(14, "ender_pearl", 75),
	(15, "eye_of_ender_signal", 23),
	(16, "potion", 77),
	(17, "xp_bottle", 76),
	(18, "item_frame", 33),
	(19, "wither_skull", 85),
	(20, "tnt", 55),
	(21, "falling_block", 24),
	(22, "fireworks_rocket", 25),
	(23, "husk", 30),
	(24, "spectral_arrow", 68),
	(25, "shulker_bullet", 60),
	(26, "dragon_fireball", 13),
	(27, "zombie_villager", 89),
	(28, "skeleton_horse", 63),
	(29, "zombie_horse", 88),
	(30, "armor_stand", 1),
	(31, "donkey", 11),
	(32, "mule", 46),
	(33, "evocation_fangs", 20),
	(34, "evocation_illager", 21),
	(35, "vex", 78),
	(36, "vindication_illager", 81),
	(37, "illusion_illager", 31),
	(40, "commandblock_minecart", 41),
	(41, "boat", 5),
	(42, "minecart", 39),
	(43, "chest_minecart", 40),
	(44, "furnace_minecart", 42),
	(45, "tnt_minecart", 45),
	(46, "hopper_minecart", 43),
	(47, "spawner_minecart", 44),
	(50, "creeper", 10),
	(51, "skeleton", 62),
	(52, "spider", 69),
	(53, "giant", 27),
	(54, "zombie", 87),
	(55, "slime", 64),
	(56, "ghast", 26),
	(57, "zombie_pigman", 53),
	(58, "enderman", 18),
	(59, "cave_spider", 6),
	(60, "silverfish", 61),
	(61, "blaze", 4),
	(62, "magma_cube", 38),
	(63, "ender_dragon", 17),
	(64, "wither", 83),
	(65, "bat", 3),
	(66, "witch", 82),
	(67, "endermite", 19),
	(68, "guardian", 28),
	(69, "shulker", 59),
	(90, "pig", 51),
	(91, "sheep", 58),
	(92, "cow", 9),
	(93, "chicken", 7),
	(94, "squid", 70),
	(95, "wolf", 86),
	(96, "mooshroom", 47),
	(97, "snowman", 66),
	(98, "ocelot", 48),
	(99, "villager_golem", 80),
	(100, "horse", 29),
	(101, "rabbit", 56),
	(102, "polar_bear", 54),
	(103, "llama", 36),
	(104, "llama_spit", 37),
	(105, "parrot", 50),
	(120, "villager", 79),
	(200, "ender_crystal", 16),
];

const NEW_MOBS: &[(i32, i32)] = &[
	(14, 27),
	(8, 94),
	(57, 94),
	(52, 94),
	(72, 94),
	(12, 94),
	(90, 105),
	(73, 98),
];

const ENTITY_NAMES: [(&str, &str); 12] = [
	("commandblock_minecart", "command_block_minecart"),
	("ender_crystal", "end_crystal"),
	("evocation_fangs", "evoker_fangs"),
	("evocation_illager", "evoker"),
	("eye_of_ender_signal", "eye_of_ender"),
	("fireworks_rocket", "firework_rocket"),
	("illusion_illager", "illusioner"),
	("snowman", "snow_golem"),
	("villager_golem", "iron_golem"),
	("vindication_illager", "vindicator"),
	("xp_bottle", "experience_bottle"),
	("xp_orb", "experience_orb"),
];

fn entity_name(name: &str, up: bool) -> String {
	let bare = name
		.strip_prefix("minecraft:")
		.unwrap_or(name)
		.to_ascii_lowercase();
	let found = ENTITY_NAMES
		.iter()
		.find(|(old, new)| if up { *old == bare } else { *new == bare });
	format!(
		"minecraft:{}",
		found.map_or(bare.as_str(), |(old, new)| if up { new } else { old })
	)
}

pub(super) fn flat_entity_name(name: &str) -> String {
	entity_name(name, true)
}

pub(super) fn legacy_entity_name(name: &str) -> String {
	entity_name(name, false)
}

pub(super) fn legacy_mob_name(id: i32) -> Option<&'static str> {
	MOBS.iter()
		.find(|(old, _, _)| *old == id)
		.map(|(_, n, _)| *n)
}

fn mob_down(id: i32) -> Option<i32> {
	MOBS.iter()
		.find(|(_, _, new)| *new == id)
		.map(|(old, _, _)| *old)
		.or_else(|| {
			NEW_MOBS
				.iter()
				.find(|(new, _)| *new == id)
				.map(|(_, old)| *old)
		})
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
	Player,
	Mob(i32),
	Object(i8),
}

#[derive(Default)]
pub(super) struct Entities {
	kinds: HashMap<i32, Kind>,
}

#[derive(Default)]
pub(super) struct FlatEntities {
	kinds: HashMap<i32, Kind>,
}

#[derive(Default)]
pub(super) struct LaterEntities {
	objects: HashMap<i32, i8>,
}

#[derive(Default)]
pub(super) struct Position {
	pub x: f64,
	pub y: f64,
	pub z: f64,
}

fn kind(cx: &mut Session, id: i32) -> Option<Kind> {
	cx.state::<Entities>().kinds.get(&id).copied()
}

fn remember(cx: &mut Session, id: i32, k: Kind) {
	cx.state::<Entities>().kinds.insert(id, k);
}

fn flat_kind(cx: &mut Session, id: i32) -> Option<Kind> {
	cx.state::<FlatEntities>().kinds.get(&id).copied()
}

fn remember_flat(cx: &mut Session, id: i32, k: Kind) {
	cx.state::<FlatEntities>().kinds.insert(id, k);
}

fn type_13(v: &M13) -> i32 {
	match v {
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
	}
}

fn type_12(v: &M12) -> i8 {
	match v {
		M12::V0(_) => 0,
		M12::V1(_) => 1,
		M12::V2(_) => 2,
		M12::V3(_) => 3,
		M12::V4(_) => 4,
		M12::V5(_) => 5,
		M12::V6(_) => 6,
		M12::V7(_) => 7,
		M12::V8(_) => 8,
		M12::V9(_) => 9,
		M12::V10(_) => 10,
		M12::V11(_) => 11,
		M12::V12(_) => 12,
		M12::V13(_) => 13,
		M12::Default(t) => *t,
	}
}

fn entry_13(key: u8, value: M13) -> b::entity_metadata_items::Ty {
	b::entity_metadata_items::Ty {
		key,
		r#type: type_13(&value),
		value,
	}
}

fn entry_12(key: u8, value: M12) -> a::entity_metadata_items::Ty {
	a::entity_metadata_items::Ty {
		key,
		r#type: type_12(&value),
		value,
	}
}

fn value_up(v: M12) -> M13 {
	match v {
		M12::V0(x) => M13::V0(x),
		M12::V1(x) => M13::V1(x),
		M12::V2(x) => M13::V2(x),
		M12::V3(x) => M13::V3(x),
		M12::V4(x) => M13::V4(x),
		M12::V5(s) => M13::V6(slot_up(s)),
		M12::V6(x) => M13::V7(x),
		M12::V7(r) => M13::V8(b::entity_metadata_item_v8::Ty {
			pitch: r.pitch,
			yaw: r.yaw,
			roll: r.roll,
		}),
		M12::V8(p) => M13::V9(p),
		M12::V9(p) => M13::V10(p),
		M12::V10(d) => M13::V11(d),
		M12::V11(u) => M13::V12(u),
		M12::V12(s) => M13::V13(state_of_packed(s) as i32),
		M12::V13(n) => M13::V14(n),
		M12::Default(t) => M13::Default(if t > 4 { t as i32 + 1 } else { t as i32 }),
	}
}

fn value_down(v: M13) -> Option<M12> {
	Some(match v {
		M13::V0(x) => M12::V0(x),
		M13::V1(x) => M12::V1(x),
		M13::V2(x) => M12::V2(x),
		M13::V3(x) => M12::V3(x),
		M13::V4(x) => M12::V4(x),
		M13::V5(text) => M12::V3(text.map(|t| json_to_legacy(&t)).unwrap_or_default()),
		M13::V15(_) => return None,
		M13::V6(s) => M12::V5(slot_down(s)),
		M13::V7(x) => M12::V6(x),
		M13::V8(r) => M12::V7(a::entity_metadata_item_v7::Ty {
			pitch: r.pitch,
			yaw: r.yaw,
			roll: r.roll,
		}),
		M13::V9(p) => M12::V8(p),
		M13::V10(p) => M12::V9(p),
		M13::V11(d) => M12::V10(d),
		M13::V12(u) => M12::V11(u),
		M13::V13(s) => M12::V12(if s == 0 {
			0
		} else {
			packed_of_state(s.max(0) as u32)
		}),
		M13::V14(n) => M12::V13(n),
		M13::Default(t) => M12::Default(if t > 5 { t - 1 } else { t } as i8),
	})
}

const ZOMBIES: [i32; 4] = [23, 27, 54, 57];
const FLAT_ZOMBIES: [i32; 5] = [87, 89, 30, 53, 14];
const FISH: [i32; 4] = [8, 57, 52, 72];

fn int_12(v: &M12) -> i32 {
	match v {
		M12::V0(n) => *n as i32,
		M12::V1(n) => *n,
		_ => 0,
	}
}

fn meta_up(kind: Option<Kind>, items: Meta12) -> Meta13 {
	let zombie = matches!(kind, Some(Kind::Mob(m)) if ZOMBIES.contains(&m));
	let wolf = kind == Some(Kind::Mob(95));
	let minecart = kind == Some(Kind::Object(10));
	let cloud = kind == Some(Kind::Object(3));
	let mut particle: Option<(i32, [i32; 2])> = None;
	let mut out = Vec::with_capacity(items.len());
	for e in items {
		let key = e.key;
		if cloud && key >= 9 {
			let p = particle.get_or_insert((0, [0, 0]));
			match key {
				9 => p.0 = int_12(&e.value),
				10 => p.1[0] = int_12(&e.value),
				11 => p.1[1] = int_12(&e.value),
				_ => {}
			}
			continue;
		}
		let value = match (key, e.value) {
			(0, M12::V0(flags)) => M13::V0(flags & !0x10),
			(2, M12::V3(name)) => M13::V5((!name.is_empty()).then(|| legacy_to_json(&name, false))),
			(17, M12::V1(color)) if wolf => M13::V1(15 - color),
			(9, M12::V1(block)) if minecart => M13::V1(state_of_packed(block) as i32),
			(_, v) => value_up(v),
		};
		let key = if zombie && key >= 15 { key + 1 } else { key };
		out.push(entry_13(key, value));
	}
	if let Some((id, args)) = particle {
		if let Some((particle_id, data)) = particle_up(id, &args) {
			out.push(entry_13(9, M13::V15(b::particle::Ty { particle_id, data })));
		}
	}
	out
}

fn dropped_down(kind: Option<Kind>, key: u8) -> bool {
	match kind {
		Some(Kind::Mob(73)) => (13..=18).contains(&key),
		Some(Kind::Mob(m)) if FISH.contains(&m) => key == 12 || key == 13,
		Some(Kind::Mob(12)) => (12..=14).contains(&key),
		Some(Kind::Mob(90)) => key == 12,
		Some(Kind::Object(1)) => key == 12,
		Some(Kind::Object(94)) => key == 7,
		_ => false,
	}
}

fn meta_down(kind: Option<Kind>, items: Meta13) -> Meta12 {
	let zombie = matches!(kind, Some(Kind::Mob(m)) if FLAT_ZOMBIES.contains(&m));
	let wolf = kind == Some(Kind::Mob(86));
	let minecart = kind == Some(Kind::Object(10));
	let cloud = kind == Some(Kind::Object(3));
	let mut out = Vec::with_capacity(items.len());
	for e in items {
		let key = e.key;
		if dropped_down(kind, key) || (zombie && key == 15) {
			continue;
		}
		let key = if zombie && key > 15 { key - 1 } else { key };
		let value = match (key, e.value) {
			(0, M13::V0(flags)) => M12::V0(flags & !0x10),
			(17, M13::V1(color)) if wolf => M12::V1(15 - color),
			(9, M13::V1(state)) if minecart => M12::V1(packed_of_state(state.max(0) as u32)),
			(9, M13::V15(p)) if cloud => {
				let (id, args) = particle_data_down(p.data, p.particle_id).unwrap_or((0, vec![]));
				let first = match (id, args.first()) {
					(37 | 38 | 46, Some(0)) => 102,
					(_, a) => a.copied().unwrap_or(0),
				};
				out.push(entry_12(9, M12::V1(id)));
				out.push(entry_12(10, M12::V1(first)));
				out.push(entry_12(11, M12::V1(args.get(1).copied().unwrap_or(0))));
				continue;
			}
			(_, v) => match value_down(v) {
				Some(v) => v,
				None => continue,
			},
		};
		out.push(entry_12(key, value));
	}
	out
}

pub fn flattening_object_data_1_13(
	src: &a::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<i32> {
	remember(cx, src.entity_id, Kind::Object(src.r#type));
	Ok(match (src.r#type, src.object_data) {
		(70, data) => state_of_packed(data) as i32,
		(71, 0) => 3,
		(71, 1) => 4,
		(71, 3) => 5,
		(_, data) => data,
	})
}

pub fn flattening_object_data_1_12_2(
	src: &b::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<i32> {
	remember_flat(cx, src.entity_id, Kind::Object(src.r#type));
	Ok(match (src.r#type, src.object_data) {
		(70, data) => packed_of_state(data.max(0) as u32),
		(71, 3) => 0,
		(71, 4) => 1,
		(71, 5) => 3,
		(_, data) => data,
	})
}

pub fn flattening_object_type_1_12_2(
	src: &b::play_to_client_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<i8> {
	Ok(if src.r#type == 94 { 60 } else { src.r#type })
}

pub fn flattening_mob_type_1_13(
	src: &a::play_to_client_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<i32> {
	remember(cx, src.entity_id, Kind::Mob(src.r#type));
	MOBS.iter()
		.find(|(old, _, _)| *old == src.r#type)
		.map(|(_, _, new)| *new)
		.ok_or(Error::Dropped)
}

pub fn flattening_mob_type_1_12_2(
	src: &b::play_to_client_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<i32> {
	remember_flat(cx, src.entity_id, Kind::Mob(src.r#type));
	mob_down(src.r#type).ok_or(Error::Dropped)
}

pub fn flattening_mob_metadata_1_13(
	src: &a::play_to_client_packet_spawn_entity_living::Ty,
	_cx: &mut Session,
) -> Result<Meta13> {
	Ok(meta_up(Some(Kind::Mob(src.r#type)), src.metadata.clone()))
}

pub fn flattening_mob_metadata_1_12_2(
	src: &b::play_to_client_packet_spawn_entity_living::Ty,
	_cx: &mut Session,
) -> Result<Meta12> {
	let mut out = meta_down(Some(Kind::Mob(src.r#type)), src.metadata.clone());
	if src.r#type == 90 {
		out.push(entry_12(15, M12::V1(3)));
	}
	Ok(out)
}

pub fn flattening_player_metadata_1_13(
	src: &a::play_to_client_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Meta13> {
	remember(cx, src.entity_id, Kind::Player);
	Ok(meta_up(Some(Kind::Player), src.metadata.clone()))
}

pub fn flattening_player_metadata_1_12_2(
	src: &b::play_to_client_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Meta12> {
	remember_flat(cx, src.entity_id, Kind::Player);
	Ok(meta_down(Some(Kind::Player), src.metadata.clone()))
}

pub fn flattening_entity_metadata_1_13(
	src: &a::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Meta13> {
	let k = kind(cx, src.entity_id);
	Ok(meta_up(k, src.metadata.clone()))
}

pub fn flattening_entity_metadata_1_12_2(
	src: &b::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Meta12> {
	let k = flat_kind(cx, src.entity_id);
	Ok(meta_down(k, src.metadata.clone()))
}

pub fn flattening_entity_destroy_1_13(
	ids: Vec<i32>,
	cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	let kinds = &mut cx.state::<Entities>().kinds;
	for id in &ids {
		kinds.remove(id);
	}
	Ok(vec![b::play_to_client_packet::Ty::EntityDestroy(ids)])
}

pub fn flattening_entity_destroy_1_12_2(
	ids: Vec<i32>,
	cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	let kinds = &mut cx.state::<FlatEntities>().kinds;
	for id in &ids {
		kinds.remove(id);
	}
	Ok(vec![a::play_to_client_packet::Ty::EntityDestroy(ids)])
}

const ARROWS: [i8; 3] = [60, 91, 94];

fn meta_later(object: Option<i8>, items: Meta13, up: bool) -> Meta13 {
	let arrow = object.is_some_and(|o| ARROWS.contains(&o));
	let minecart = object == Some(10);
	let state = if up { state_later } else { state_earlier };
	let mut out = Vec::with_capacity(items.len());
	for e in items {
		let mut key = e.key;
		if arrow && key >= 7 {
			match up {
				true => key += 1,
				false if key == 7 => continue,
				false => key -= 1,
			}
		}
		let value = match (key, e.value) {
			(9, M13::V1(s)) if minecart => M13::V1(state(s.max(0) as u32) as i32),
			(_, M13::V13(s)) => M13::V13(state(s.max(0) as u32) as i32),
			(_, M13::V6(slot)) => M13::V6(if up {
				slot_later(slot)
			} else {
				slot_earlier(slot)
			}),
			(_, M13::V15(p)) => M13::V15(b::particle::Ty {
				particle_id: p.particle_id,
				data: if up {
					particle_data_later(p.data)
				} else {
					particle_data_earlier(p.data)
				},
			}),
			(_, v) => v,
		};
		out.push(entry_13(key, value));
	}
	out
}

fn object(cx: &mut Session, id: i32) -> Option<i8> {
	cx.state::<LaterEntities>().objects.get(&id).copied()
}

fn spawn_object(
	src: &b::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
	f: fn(u32) -> u32,
) -> i32 {
	cx.state::<LaterEntities>()
		.objects
		.insert(src.entity_id, src.r#type);
	match src.r#type {
		70 => f(src.object_data.max(0) as u32) as i32,
		_ => src.object_data,
	}
}

pub fn flattening_spawn_object_data_1_13_1(
	src: &b::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<i32> {
	Ok(spawn_object(src, cx, state_later))
}

pub fn flattening_spawn_object_data_1_13(
	src: &b::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<i32> {
	Ok(spawn_object(src, cx, state_earlier))
}

pub fn flattening_spawn_mob_metadata_1_13_1(
	src: &b::play_to_client_packet_spawn_entity_living::Ty,
	_cx: &mut Session,
) -> Result<Meta13> {
	Ok(meta_later(None, src.metadata.clone(), true))
}

pub fn flattening_spawn_mob_metadata_1_13(
	src: &b::play_to_client_packet_spawn_entity_living::Ty,
	_cx: &mut Session,
) -> Result<Meta13> {
	Ok(meta_later(None, src.metadata.clone(), false))
}

pub fn flattening_spawn_player_metadata_1_13_1(
	src: &b::play_to_client_packet_named_entity_spawn::Ty,
	_cx: &mut Session,
) -> Result<Meta13> {
	Ok(meta_later(None, src.metadata.clone(), true))
}

pub fn flattening_spawn_player_metadata_1_13(
	src: &b::play_to_client_packet_named_entity_spawn::Ty,
	_cx: &mut Session,
) -> Result<Meta13> {
	Ok(meta_later(None, src.metadata.clone(), false))
}

pub fn flattening_update_metadata_1_13_1(
	src: &b::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Meta13> {
	let o = object(cx, src.entity_id);
	Ok(meta_later(o, src.metadata.clone(), true))
}

pub fn flattening_update_metadata_1_13(
	src: &b::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Meta13> {
	let o = object(cx, src.entity_id);
	Ok(meta_later(o, src.metadata.clone(), false))
}

pub fn flattening_forget_entities_1_13_1(
	ids: Vec<i32>,
	cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	let objects = &mut cx.state::<LaterEntities>().objects;
	for id in &ids {
		objects.remove(id);
	}
	Ok(vec![b::play_to_client_packet::Ty::EntityDestroy(ids)])
}

pub fn flattening_forget_entities_1_13(
	ids: Vec<i32>,
	cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	flattening_forget_entities_1_13_1(ids, cx)
}

pub(super) fn forget_later(cx: &mut Session) {
	cx.state::<LaterEntities>().objects.clear();
}

pub(super) fn forget_flat(cx: &mut Session) {
	cx.state::<Entities>().kinds.clear();
	cx.state::<FlatEntities>().kinds.clear();
}

fn track(cx: &mut Session, x: f64, y: f64, z: f64) {
	let p = cx.state::<Position>();
	(p.x, p.y, p.z) = (x, y, z);
}

pub fn flattening_moved_1_13(
	src: &a::play_to_server_packet_position::Ty,
	cx: &mut Session,
) -> Result<f64> {
	track(cx, src.x, src.y, src.z);
	Ok(src.x)
}

pub fn flattening_moved_look_1_13(
	src: &a::play_to_server_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<f64> {
	track(cx, src.x, src.y, src.z);
	Ok(src.x)
}

pub fn flattening_vehicle_moved_1_13(
	src: &a::play_to_server_packet_vehicle_move::Ty,
	cx: &mut Session,
) -> Result<f64> {
	track(cx, src.x, src.y, src.z);
	Ok(src.x)
}

pub fn flattening_teleported_1_12_2(
	src: &b::play_to_client_packet_position::Ty,
	cx: &mut Session,
) -> Result<f64> {
	let p = cx.state::<Position>();
	let rel = |bit: i8, old: f64, new: f64| if src.flags & bit != 0 { old + new } else { new };
	(p.x, p.y, p.z) = (rel(1, p.x, src.x), rel(2, p.y, src.y), rel(4, p.z, src.z));
	Ok(src.x)
}

pub fn flattening_face_player_1_12_2(
	p: b::play_to_client_packet_face_player::Ty,
	cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	let at = cx.state::<Position>();
	let eyes = if p.feet_eyes == 1 { 1.62 } else { 0.0 };
	let (dx, dy, dz) = (p.x - at.x, p.y - (at.y + eyes), p.z - at.z);
	let r = (dx * dx + dy * dy + dz * dz).sqrt();
	let yaw = -dx.atan2(dz).to_degrees();
	let yaw = if yaw < 0.0 { 360.0 + yaw } else { yaw } as f32;
	let pitch = -(dy / r).asin().to_degrees() as f32;
	Ok(vec![a::play_to_client_packet::Ty::Position(
		a::play_to_client_packet_position::Ty {
			x: 0.0,
			y: 0.0,
			z: 0.0,
			yaw,
			pitch,
			flags: 7,
			teleport_id: -1,
		},
	)])
}
