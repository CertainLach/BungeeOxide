use super::blocks::{
	item_earlier, item_later, new_items, packed_of_state, state_earlier, state_later,
	state_of_packed, FLAT, OLD,
};
use super::items::{
	item_down, item_up, legacy_item_name, legacy_of_item, slot_earlier, slot_later,
};
use crate::gen::convert::{v1_12_2 as a, v1_13 as b};
use crate::pds::convert::{Error, Result};
use crate::registry::{self, renames};
use crate::session::Session;

type Data13 = b::particleData::Ty;

#[derive(Clone, Copy, PartialEq)]
enum Arg {
	None,
	Dust,
	Item,
	Block,
	Falling,
}

const PARTICLES: [(i32, Arg); 49] = [
	(34, Arg::None),
	(19, Arg::None),
	(18, Arg::None),
	(21, Arg::None),
	(4, Arg::None),
	(43, Arg::None),
	(22, Arg::None),
	(42, Arg::None),
	(32, Arg::None),
	(6, Arg::None),
	(14, Arg::None),
	(37, Arg::None),
	(30, Arg::None),
	(12, Arg::None),
	(26, Arg::None),
	(17, Arg::None),
	(0, Arg::None),
	(44, Arg::None),
	(10, Arg::None),
	(9, Arg::None),
	(1, Arg::None),
	(24, Arg::None),
	(32, Arg::None),
	(33, Arg::None),
	(35, Arg::None),
	(15, Arg::None),
	(23, Arg::None),
	(31, Arg::None),
	(-1, Arg::None),
	(5, Arg::None),
	(11, Arg::Dust),
	(29, Arg::None),
	(34, Arg::None),
	(28, Arg::None),
	(25, Arg::None),
	(2, Arg::None),
	(27, Arg::Item),
	(3, Arg::Block),
	(3, Arg::Block),
	(36, Arg::None),
	(-1, Arg::None),
	(13, Arg::None),
	(8, Arg::None),
	(16, Arg::None),
	(7, Arg::None),
	(40, Arg::None),
	(20, Arg::Falling),
	(41, Arg::None),
	(38, Arg::None),
];

const LEGACY_PARTICLES: [i32; 50] = [
	16, 20, 35, 37, 4, 29, 9, 44, 42, 19, 18, 30, 13, 41, 10, 25, 43, 15, 2, 1, 46, 3, 6, 26, 21,
	34, 14, 36, 33, 31, 12, 27, 22, 23, 0, 24, 39, 11, 48, 12, 45, 47, 7, 5, 17, 4, 4, 4, 18, 18,
];

fn dust(red: f32, green: f32, blue: f32) -> Data13 {
	Data13::v11(b::particleData__v11::Ty {
		red,
		green,
		blue,
		scale: 1.0,
	})
}

pub(super) fn particle_up(id: i32, args: &[i32]) -> Option<(i32, Data13)> {
	let (new, arg) = *PARTICLES.get(usize::try_from(id).ok()?)?;
	if new < 0 {
		return None;
	}
	let first = args.first().copied().unwrap_or(0);
	let data = match arg {
		Arg::None => Data13::default(new),
		Arg::Dust => dust(1.0, 0.0, 0.0),
		Arg::Item => {
			let (item, nbtData) =
				item_up(first as i16, args.get(1).copied().unwrap_or(0) as i16, None);
			Data13::v27(b::slot::Ty::default(
				item,
				b::slot__default::Ty {
					itemCount: 1,
					nbtData,
				},
			))
		}
		Arg::Block => Data13::v3(state_of_packed(first) as i32),
		Arg::Falling => {
			let first = if first & 4095 == 160 {
				first & !4095 | 95
			} else {
				first
			};
			Data13::v20(state_of_packed(first) as i32)
		}
	};
	Some((new, data))
}

pub(super) fn particle_data_down(data: Data13, id: i32) -> Option<(i32, Vec<i32>)> {
	let old = *LEGACY_PARTICLES.get(usize::try_from(id).ok()?)?;
	Some(match data {
		Data13::v3(state) | Data13::v20(state) => (old, vec![packed_of_state(state.max(0) as u32)]),
		Data13::v27(b::slot::Ty::default(item, s)) => {
			let (item, damage, _) = item_down(item, s.nbtData);
			(old, vec![item as i32, damage as i32])
		}
		Data13::v27(b::slot::Ty::_1) => (old, vec![0, 0]),
		Data13::v11(_) | Data13::default(_) => (old, vec![]),
	})
}

fn dust_color(speed: f32, offset: f32, red: bool) -> f32 {
	let color = speed * offset;
	if red && color == 0.0 {
		return 1.0;
	}
	((color * 255.0) as i32 & 255) as f32 / 255.0
}

pub fn flattening_world_particles_1_13(
	p: a::play_toClient_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<b::play_toClient_packet_world_particles::Ty> {
	use a::play_toClient_packet_world_particles__data::Ty as Old;
	let (id, args) = match &p.data {
		Old::v36(v) => (36, v.clone()),
		Old::v37(v) => (37, v.clone()),
		Old::v38(v) => (38, v.clone()),
		Old::v46(v) => (46, v.clone()),
		Old::default(t) => (*t, vec![]),
	};
	let (particleId, mut data) = particle_up(id, &args).ok_or(Error::Dropped)?;
	let mut out = b::play_toClient_packet_world_particles::Ty {
		particleId,
		longDistance: p.longDistance,
		x: p.x,
		y: p.y,
		z: p.z,
		offsetX: p.offsetX,
		offsetY: p.offsetY,
		offsetZ: p.offsetZ,
		particleData: p.particleData,
		particles: p.particles,
		data: Data13::default(particleId),
	};
	if matches!(data, Data13::v11(_)) && p.particles == 0 {
		data = dust(
			dust_color(p.particleData, p.offsetX, true),
			dust_color(p.particleData, p.offsetY, false),
			dust_color(p.particleData, p.offsetZ, false),
		);
		out.particles = 1;
		out.particleData = 0.0;
		out.offsetX = 0.0;
		out.offsetY = 0.0;
		out.offsetZ = 0.0;
	}
	out.data = data;
	Ok(out)
}

pub fn flattening_world_particles_1_12_2(
	p: b::play_toClient_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<a::play_toClient_packet_world_particles::Ty> {
	use a::play_toClient_packet_world_particles__data::Ty as Old;
	let mut out = a::play_toClient_packet_world_particles::Ty {
		longDistance: p.longDistance,
		x: p.x,
		y: p.y,
		z: p.z,
		offsetX: p.offsetX,
		offsetY: p.offsetY,
		offsetZ: p.offsetZ,
		particleData: p.particleData,
		particles: p.particles,
		data: Old::default(0),
	};
	if let Data13::v11(color) = &p.data {
		out.offsetX = if color.red == 0.0 {
			f32::MIN_POSITIVE
		} else {
			color.red
		};
		out.offsetY = color.green;
		out.offsetZ = color.blue;
		out.particleData = color.scale;
		out.particles = 0;
	}
	let block = matches!(p.data, Data13::v3(_) | Data13::v20(_));
	let (id, args) = particle_data_down(p.data, p.particleId).ok_or(Error::Dropped)?;
	if block && args.first() == Some(&0) {
		return Err(Error::Dropped);
	}
	out.data = match id {
		36 => Old::v36(args),
		37 => Old::v37(args),
		38 => Old::v38(args),
		46 => Old::v46(args),
		_ => Old::default(id),
	};
	Ok(out)
}

fn particle_data(
	data: Data13,
	state: fn(u32) -> u32,
	slot: fn(b::slot::Ty) -> b::slot::Ty,
) -> Data13 {
	match data {
		Data13::v3(s) => Data13::v3(state(s.max(0) as u32) as i32),
		Data13::v20(s) => Data13::v20(state(s.max(0) as u32) as i32),
		Data13::v27(item) => Data13::v27(slot(item)),
		d => d,
	}
}

pub(super) fn particle_data_later(data: Data13) -> Data13 {
	particle_data(data, state_later, slot_later)
}

pub(super) fn particle_data_earlier(data: Data13) -> Data13 {
	particle_data(data, state_earlier, slot_earlier)
}

pub fn flattening_particle_data_1_13_1(data: Data13, _cx: &mut Session) -> Result<Data13> {
	Ok(particle_data_later(data))
}

pub fn flattening_particle_data_1_13(data: Data13, _cx: &mut Session) -> Result<Data13> {
	Ok(particle_data_earlier(data))
}

type Event = b::play_toClient_packet_world_event::Ty;

pub fn flattening_world_event_data_1_13(
	src: &a::play_toClient_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(match src.effectId {
		1010 => item_of_legacy(src.data).first().copied().unwrap_or(0),
		2001 => state_of_packed(src.data) as i32,
		_ => src.data,
	})
}

pub fn flattening_world_event_data_1_12_2(src: &Event, _cx: &mut Session) -> Result<i32> {
	Ok(match src.effectId {
		1010 => legacy_of_item(src.data.max(0) as u32).map_or(0, |(id, _)| id as i32),
		2001 => packed_of_state(src.data.max(0) as u32),
		_ => src.data,
	})
}

const SMOKE_UP: [i32; 9] = [4, 2, 5, 4, 0, 5, 4, 3, 5];

pub fn flattening_event_data_1_13_1(src: &Event, _cx: &mut Session) -> Result<i32> {
	Ok(match src.effectId {
		1010 => item_later(src.data).unwrap_or(0),
		2000 => SMOKE_UP.get(src.data.max(0) as usize).copied().unwrap_or(0),
		2001 => state_later(src.data.max(0) as u32) as i32,
		_ => src.data,
	})
}

pub fn flattening_event_data_1_13(src: &Event, _cx: &mut Session) -> Result<i32> {
	Ok(match src.effectId {
		1010 => item_earlier(src.data).unwrap_or(0),
		2000 => match src.data {
			2 => 1,
			3 => 7,
			4 => 3,
			5 => 5,
			_ => 4,
		},
		2001 => state_earlier(src.data.max(0) as u32) as i32,
		_ => src.data,
	})
}

pub fn flattening_event_location_1_13(src: &Event, _cx: &mut Session) -> Result<b::position::Ty> {
	let mut at = src.location.clone();
	if src.effectId == 2000 {
		match src.data {
			0 => at.y -= 1,
			1 => at.y += 1,
			_ => {}
		}
	}
	Ok(at)
}

fn sound_renames() -> impl Iterator<Item = &'static (&'static str, &'static str, &'static str)> {
	renames::table("sounds")
		.iter()
		.filter(|(v, _, _)| *v == FLAT.name())
}

fn bare(name: &str) -> &str {
	name.strip_prefix("minecraft:").unwrap_or(name)
}

fn sound_up(name: &str) -> String {
	let name = bare(name);
	sound_renames()
		.find(|(_, old, _)| *old == name)
		.map_or_else(
			|| name.to_ascii_lowercase(),
			|(_, _, new)| (*new).to_owned(),
		)
}

fn sound_down(name: &str) -> String {
	let name = bare(name);
	sound_renames()
		.find(|(_, _, new)| *new == name)
		.map(|(_, old, _)| *old)
		.or_else(|| super::fallback::sound(name))
		.unwrap_or(name)
		.to_owned()
}

pub(super) fn sound_name_down(name: &str) -> String {
	sound_down(name)
}

pub fn flattening_sound_name_1_13(
	src: &a::play_toClient_packet_named_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(sound_up(&src.soundName))
}

pub fn flattening_sound_name_1_12_2(
	src: &b::play_toClient_packet_named_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(sound_down(&src.soundName))
}

fn sound_id(
	id: i32,
	from: crate::gen::convert::Version,
	to: crate::gen::convert::Version,
	rename: fn(&str) -> String,
) -> Result<i32> {
	let from = registry::names(from, "sounds").and_then(|n| n.name(id.max(0) as u32));
	let to = registry::names(to, "sounds");
	from.and_then(|n| to?.id(&rename(n)))
		.map(|i| i as i32)
		.ok_or(Error::Dropped)
}

pub fn flattening_sound_id_1_13(
	src: &a::play_toClient_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound_id(src.soundId, OLD, FLAT, sound_up)
}

pub fn flattening_sound_id_1_12_2(
	src: &b::play_toClient_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound_id(src.soundId, FLAT, OLD, sound_down)
}

pub fn flattening_effect_flags_1_13(
	src: &a::play_toClient_packet_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i8> {
	Ok(src.hideParticles | 4)
}

pub fn flattening_effect_flags_1_12_2(
	src: &b::play_toClient_packet_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i8> {
	Ok(src.hideParticles & 3)
}

fn enchantment(
	id: i16,
	from: crate::gen::convert::Version,
	to: crate::gen::convert::Version,
) -> i16 {
	if id < 0 {
		return id;
	}
	registry::remap_name("enchantments", from, to, id as u32).map_or(-1, |n| n as i16)
}

pub fn flattening_window_property_1_13(
	src: &a::play_toClient_packet_craft_progress_bar::Ty,
	_cx: &mut Session,
) -> Result<i16> {
	Ok(match src.property {
		4..=6 => enchantment(src.value, OLD, FLAT),
		_ => src.value,
	})
}

pub fn flattening_window_property_1_12_2(
	src: &b::play_toClient_packet_craft_progress_bar::Ty,
	_cx: &mut Session,
) -> Result<i16> {
	Ok(match src.property {
		4..=6 => enchantment(src.value, FLAT, OLD),
		_ => src.value,
	})
}

const PAINTINGS: [&str; 26] = [
	"Kebab",
	"Aztec",
	"Alban",
	"Aztec2",
	"Bomb",
	"Plant",
	"Wasteland",
	"Pool",
	"Courbet",
	"Sea",
	"Sunset",
	"Creebet",
	"Wanderer",
	"Graham",
	"Match",
	"Bust",
	"Stage",
	"Void",
	"SkullAndRoses",
	"Wither",
	"Fighters",
	"Pointer",
	"Pigscene",
	"BurningSkull",
	"Skeleton",
	"DonkeyKong",
];

pub fn flattening_painting_1_13(
	src: &a::play_toClient_packet_spawn_entity_painting::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	let title = src
		.title
		.strip_prefix("minecraft:")
		.unwrap_or(&src.title)
		.replace('_', "");
	Ok(PAINTINGS
		.iter()
		.position(|p| p.eq_ignore_ascii_case(&title))
		.unwrap_or(0) as i32)
}

pub fn flattening_painting_1_12_2(
	src: &b::play_toClient_packet_spawn_entity_painting::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(PAINTINGS
		.get(src.title.max(0) as usize)
		.copied()
		.unwrap_or("Kebab")
		.to_owned())
}

pub fn flattening_map_icon_1_13(
	v: a::play_toClient_packet_map__icons__items::Ty,
	_cx: &mut Session,
) -> Result<b::play_toClient_packet_map__icons__items::Ty> {
	Ok(b::play_toClient_packet_map__icons__items::Ty {
		r#type: (v.directionAndType as u8 >> 4) as i32,
		x: v.x,
		z: v.z,
		direction: v.directionAndType as u8 & 15,
		displayName: None,
	})
}

pub fn flattening_map_icon_1_12_2(
	v: b::play_toClient_packet_map__icons__items::Ty,
	_cx: &mut Session,
) -> Result<a::play_toClient_packet_map__icons__items::Ty> {
	if !(0..=9).contains(&v.r#type) {
		return Err(Error::Dropped);
	}
	Ok(a::play_toClient_packet_map__icons__items::Ty {
		directionAndType: ((v.r#type as u8) << 4 | (v.direction & 15)) as i8,
		x: v.x,
		z: v.z,
	})
}

pub(super) fn item_of_legacy(id: i32) -> Vec<i32> {
	let names = new_items();
	if id == 383 {
		let mut eggs: Vec<i32> = names
			.names
			.iter()
			.filter(|(_, n)| n.ends_with("_spawn_egg"))
			.map(|(i, _)| *i as i32)
			.collect();
		eggs.sort();
		return eggs;
	}
	let mut out: Vec<i32> = Vec::new();
	for meta in 0..16 {
		if let Some(n) = legacy_item_name(id as u16, meta).and_then(|n| names.id(n)) {
			if !out.contains(&(n as i32)) {
				out.push(n as i32);
			}
		}
	}
	out
}
