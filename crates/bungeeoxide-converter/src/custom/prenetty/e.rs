use super::c::{remap_bulk, remap_single};
use super::ids;
use super::tables::{SOUNDS_1_4_2, SOUNDS_1_6_1, SOUNDS_1_7, STATS_1_7};
use super::text;
use super::{falling_block, item_id, state, world_event_data};
use crate::gen::convert::{v1_6_4, v1_7, Root, Version};
use crate::pds::convert::Result;
use crate::pds::Output;
use crate::session::Session;

type Client164 = v1_6_4::play_to_client_packet::Ty;
type Client17 = v1_7::play_to_client_packet::Ty;
type Server164 = v1_6_4::play_to_server_packet::Ty;
type Server17 = v1_7::play_to_server_packet::Ty;

const TO: Version = Version::V1_6_4;
const EYES: f64 = 1.6200000047683716;

pub fn prenetty_map_chunk_1_6_4_down(
	mut p: v1_7::play_to_client_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client164>> {
	p.compressed_chunk_data = remap_single(
		&p.compressed_chunk_data,
		p.bit_map,
		p.add_bit_map,
		p.ground_up,
		TO,
	)?;
	Ok(vec![Client164::MapChunk(p)])
}

pub fn prenetty_world_event_1_6_4_down(
	mut p: v1_7::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client164>> {
	p.data = world_event_data(p.effect_id, p.data, TO);
	Ok(vec![Client164::WorldEvent(p)])
}

fn sound(table: &[(&str, &str)], name: &str, up: bool) -> Option<String> {
	if name.is_empty() {
		return Some(String::new());
	}
	table
		.iter()
		.find(|(old, new)| if up { *old == name } else { *new == name })
		.map(|(old, new)| if up { new } else { old }.to_string())
}

pub fn prenetty_chat_server_1_7(message: String, _cx: &mut Session) -> Result<Vec<Server17>> {
	Ok(vec![Server17::Chat(message)])
}

pub fn prenetty_chat_server_1_6_4(message: String, _cx: &mut Session) -> Result<Vec<Server164>> {
	Ok(vec![Server164::Chat(text::truncate(&message, 100))])
}

pub fn prenetty_tab_complete_server_1_7(text: String, _cx: &mut Session) -> Result<Vec<Server17>> {
	Ok(vec![Server17::TabComplete(text)])
}

pub fn prenetty_tab_complete_server_1_6_4(
	input: String,
	_cx: &mut Session,
) -> Result<Vec<Server164>> {
	Ok(vec![Server164::TabComplete(text::truncate(&input, 100))])
}

macro_rules! sounds {
	($up:ident, $down:ident, $old:ident, $new:ident, $table:expr) => {
		pub fn $up(
			mut p: $old::play_to_client_packet_named_sound_effect::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$new::play_to_client_packet::Ty>> {
			match sound(&$table, &p.sound_name, true) {
				Some(name) if !name.is_empty() => {
					p.sound_name = name;
					Ok(vec![$new::play_to_client_packet::Ty::NamedSoundEffect(p)])
				}
				_ => Ok(vec![]),
			}
		}

		pub fn $down(
			mut p: $new::play_to_client_packet_named_sound_effect::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$old::play_to_client_packet::Ty>> {
			match sound(&$table, &p.sound_name, false) {
				Some(name) if !name.is_empty() => {
					p.sound_name = name;
					Ok(vec![$old::play_to_client_packet::Ty::NamedSoundEffect(p)])
				}
				_ => Ok(vec![]),
			}
		}
	};
}

use crate::gen::convert::{v1_3_2, v1_4_2, v1_5_2, v1_6_1};

sounds!(
	prenetty_sound_1_4_2,
	prenetty_sound_1_3_2,
	v1_3_2,
	v1_4_2,
	SOUNDS_1_4_2
);
sounds!(
	prenetty_sound_1_6_1,
	prenetty_sound_1_5_2,
	v1_5_2,
	v1_6_1,
	SOUNDS_1_6_1
);

pub fn prenetty_sound_1_7(
	p: v1_6_4::play_to_client_packet_named_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client17>> {
	match sound(&SOUNDS_1_7, &p.sound_name, true) {
		Some(name) if !name.is_empty() => Ok(vec![Client17::NamedSoundEffect(
			v1_7::play_to_client_packet_named_sound_effect::Ty {
				sound_name: name,
				x: p.x,
				y: p.y,
				z: p.z,
				volume: p.volume,
				pitch: p.pitch,
			},
		)]),
		_ => Ok(vec![]),
	}
}

pub fn prenetty_sound_1_6_4(
	p: v1_7::play_to_client_packet_named_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client164>> {
	match sound(&SOUNDS_1_7, &p.sound_name, false) {
		Some(name) if !name.is_empty() => Ok(vec![Client164::NamedSoundEffect(
			v1_6_4::play_to_client_packet_named_sound_effect::Ty {
				sound_name: name,
				x: p.x,
				y: p.y,
				z: p.z,
				volume: p.volume,
				pitch: p.pitch,
			},
		)]),
		_ => Ok(vec![]),
	}
}

pub fn prenetty_login_1_7(
	p: v1_6_4::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client17>> {
	let st = state(cx);
	st.player = p.entity_id;
	st.stats.clear();
	Ok(vec![Client17::Login(
		v1_7::play_to_client_packet_login::Ty {
			entity_id: p.entity_id,
			game_mode: p.game_mode,
			dimension: p.dimension,
			difficulty: p.difficulty,
			max_players: p.max_players,
			level_type: p.level_type,
		},
	)])
}

pub fn prenetty_login_1_6_4(
	p: v1_7::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client164>> {
	super::entities::joined(cx, p.entity_id);
	state(cx).stat_values.clear();
	Ok(vec![Client164::Login(
		v1_6_4::play_to_client_packet_login::Ty {
			entity_id: p.entity_id,
			level_type: p.level_type,
			game_mode: p.game_mode,
			dimension: p.dimension,
			difficulty: p.difficulty,
			world_height: 0,
			max_players: p.max_players,
		},
	)])
}

pub fn prenetty_position_1_7(
	p: v1_6_4::play_to_client_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<Client17>> {
	let st = state(cx);
	(st.x, st.y, st.z, st.yaw, st.pitch) = (p.x, p.stance, p.z, p.yaw, p.pitch);
	Ok(vec![Client17::Position(
		v1_7::play_to_client_packet_position::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			on_ground: p.on_ground,
		},
	)])
}

pub fn prenetty_position_1_6_4(
	p: v1_7::play_to_client_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<Client164>> {
	let st = state(cx);
	(st.x, st.y, st.z, st.yaw, st.pitch) = (p.x, p.y - EYES, p.z, p.yaw, p.pitch);
	Ok(vec![Client164::Position(
		v1_6_4::play_to_client_packet_position::Ty {
			x: p.x,
			y: p.y,
			stance: p.y - EYES,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			on_ground: p.on_ground,
		},
	)])
}

fn position_from_state(cx: &mut Session, on_ground: bool) -> Client17 {
	let st = state(cx);
	Client17::Position(v1_7::play_to_client_packet_position::Ty {
		x: st.x,
		y: st.y + EYES,
		z: st.z,
		yaw: st.yaw,
		pitch: st.pitch,
		on_ground: on_ground,
	})
}

pub fn prenetty_position_only_1_7(
	p: v1_6_4::play_to_client_packet_position_only::Ty,
	cx: &mut Session,
) -> Result<Vec<Client17>> {
	let st = state(cx);
	(st.x, st.y, st.z) = (p.x, p.stance, p.z);
	Ok(vec![position_from_state(cx, p.on_ground)])
}

pub fn prenetty_look_1_7(
	p: v1_6_4::play_to_client_packet_look::Ty,
	cx: &mut Session,
) -> Result<Vec<Client17>> {
	let st = state(cx);
	(st.yaw, st.pitch) = (p.yaw, p.pitch);
	Ok(vec![position_from_state(cx, p.on_ground)])
}

macro_rules! track_moves {
	($position:ident, $look:ident, $position_look:ident, $from:ident, $to:ident) => {
		pub fn $position(
			p: $from::play_to_server_packet_position::Ty,
			cx: &mut Session,
		) -> Result<Vec<$to::play_to_server_packet::Ty>> {
			let st = state(cx);
			(st.x, st.y, st.z) = (p.x, p.y, p.z);
			Ok(vec![$to::play_to_server_packet::Ty::Position(p)])
		}

		pub fn $look(
			p: $from::play_to_server_packet_look::Ty,
			cx: &mut Session,
		) -> Result<Vec<$to::play_to_server_packet::Ty>> {
			let st = state(cx);
			(st.yaw, st.pitch) = (p.yaw, p.pitch);
			Ok(vec![$to::play_to_server_packet::Ty::Look(p)])
		}

		pub fn $position_look(
			p: $from::play_to_server_packet_position_look::Ty,
			cx: &mut Session,
		) -> Result<Vec<$to::play_to_server_packet::Ty>> {
			let st = state(cx);
			(st.x, st.y, st.z, st.yaw, st.pitch) = (p.x, p.y, p.z, p.yaw, p.pitch);
			Ok(vec![$to::play_to_server_packet::Ty::PositionLook(p)])
		}
	};
}

track_moves!(
	prenetty_move_1_7,
	prenetty_look_server_1_7,
	prenetty_move_look_1_7,
	v1_6_4,
	v1_7
);
track_moves!(
	prenetty_move_1_6_4,
	prenetty_look_server_1_6_4,
	prenetty_move_look_1_6_4,
	v1_7,
	v1_6_4
);

pub fn prenetty_kick_1_7(reason: String, _cx: &mut Session) -> Result<Vec<Client17>> {
	Ok(vec![Client17::KickDisconnect(text::legacy_to_json(
		&reason,
	))])
}

pub fn prenetty_kick_1_6_4(reason: String, _cx: &mut Session) -> Result<Vec<Client164>> {
	Ok(vec![Client164::KickDisconnect(text::truncate(
		&text::json_to_legacy(&reason),
		256,
	))])
}

pub fn prenetty_chat_1_7(message: String, _cx: &mut Session) -> Result<Vec<Client17>> {
	Ok(vec![Client17::Chat(text::rename_key(
		&message, "using", "with",
	))])
}

pub fn prenetty_chat_1_6_4(message: String, _cx: &mut Session) -> Result<Vec<Client164>> {
	Ok(vec![Client164::Chat(text::truncate(
		&text::rename_key(&message, "with", "using"),
		i16::MAX as usize,
	))])
}

pub fn prenetty_tab_complete_1_7(matches: String, _cx: &mut Session) -> Result<Vec<Client17>> {
	let list = if matches.is_empty() {
		vec![]
	} else {
		matches.split('\0').map(str::to_owned).collect()
	};
	Ok(vec![Client17::TabComplete(list)])
}

pub fn prenetty_tab_complete_1_6_4(
	matches: Vec<String>,
	_cx: &mut Session,
) -> Result<Vec<Client164>> {
	Ok(vec![Client164::TabComplete(matches.join("\0"))])
}

fn stat_name(id: i32) -> Option<String> {
	if let Some((_, name)) = STATS_1_7.iter().find(|(i, _)| *i == id) {
		return Some(name.to_string());
	}
	let (prefix, base) = match id {
		16777216..=16842751 => ("stat.mineBlock", 16777216),
		16842752..=16908287 => ("stat.craftItem", 16842752),
		16908288..=16973823 => ("stat.useItem", 16908288),
		16973824..=17039359 => ("stat.breakItem", 16973824),
		_ => return None,
	};
	Some(format!("{prefix}.{}", id - base))
}

fn stat_id(name: &str) -> Option<i32> {
	if let Some((id, _)) = STATS_1_7.iter().find(|(_, n)| *n == name) {
		return Some(*id);
	}
	let (prefix, number) = name.rsplit_once('.')?;
	let base = match prefix {
		"stat.mineBlock" => 16777216,
		"stat.craftItem" => 16842752,
		"stat.useItem" => 16908288,
		"stat.breakItem" => 16973824,
		_ => return None,
	};
	Some(base + number.parse::<i32>().ok()?)
}

pub fn prenetty_statistics_1_7(
	p: v1_6_4::play_to_client_packet_statistics::Ty,
	cx: &mut Session,
) -> Result<Vec<Client17>> {
	let total = {
		let v = state(cx).stats.entry(p.statistic_id).or_default();
		*v = v.saturating_add(p.amount);
		*v
	};
	let Some(name) = stat_name(p.statistic_id) else {
		return Ok(vec![]);
	};
	Ok(vec![Client17::Statistics(vec![
		v1_7::play_to_client_packet_statistics_items::Ty { name, value: total },
	])])
}

pub fn prenetty_statistics_1_6_4(
	p: Vec<v1_7::play_to_client_packet_statistics_items::Ty>,
	cx: &mut Session,
) -> Result<Vec<Client164>> {
	let st = state(cx);
	let mut out = Vec::new();
	for s in p {
		let Some(id) = stat_id(&s.name) else { continue };
		let before = st.stat_values.insert(s.name, s.value).unwrap_or(0);
		let mut delta = s.value as i64 - before as i64;
		while delta > 0 {
			let amount = delta.min(i32::MAX as i64) as i32;
			out.push(Client164::Statistics(
				v1_6_4::play_to_client_packet_statistics::Ty {
					statistic_id: id,
					amount,
				},
			));
			delta -= amount as i64;
		}
	}
	Ok(out)
}

fn animation_up(a: u8) -> Option<u8> {
	match a {
		0 | 4 => None,
		1..=3 => Some(a - 1),
		a => Some(a.wrapping_sub(2)),
	}
}

fn animation_down(a: u8) -> u8 {
	match a {
		0..=2 => a + 1,
		a => a.wrapping_add(2),
	}
}

pub fn prenetty_animation_1_7(
	p: v1_6_4::play_to_client_packet_animation::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client17>> {
	let Some(animation) = animation_up(p.animation) else {
		return Ok(vec![]);
	};
	Ok(vec![Client17::Animation(
		v1_7::play_to_client_packet_animation::Ty {
			entity_id: p.entity_id,
			animation,
		},
	)])
}

pub fn prenetty_animation_1_6_4(
	p: v1_7::play_to_client_packet_animation::Ty,
	cx: &mut Session,
) -> Result<Vec<Client164>> {
	let animation = animation_down(p.animation);
	if !super::entities::animation_reaches_1_6_4(cx, p.entity_id, animation) {
		return Ok(vec![]);
	}
	Ok(vec![Client164::Animation(
		v1_6_4::play_to_client_packet_animation::Ty {
			entity_id: p.entity_id,
			animation,
		},
	)])
}

pub fn prenetty_bed_1_7(
	p: v1_6_4::play_to_client_packet_bed::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client17>> {
	if p.action != 0 {
		return Ok(vec![]);
	}
	Ok(vec![Client17::Bed(v1_7::play_to_client_packet_bed::Ty {
		entity_id: p.entity_id,
		location: p.location,
	})])
}

pub fn prenetty_bed_1_6_4(
	p: v1_7::play_to_client_packet_bed::Ty,
	cx: &mut Session,
) -> Result<Vec<Client164>> {
	if !super::entities::bed_reaches_1_6_4(cx, p.entity_id) {
		return Ok(vec![]);
	}
	Ok(vec![Client164::Bed(
		v1_6_4::play_to_client_packet_bed::Ty {
			entity_id: p.entity_id,
			action: 0,
			location: p.location,
		},
	)])
}

pub fn prenetty_open_sign_entity_1_7(
	p: v1_6_4::play_to_client_packet_open_sign_entity::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client17>> {
	Ok(vec![Client17::OpenSignEntity(p.location)])
}

pub fn prenetty_open_sign_entity_1_6_4(
	p: v1_7::position_iii::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client164>> {
	Ok(vec![Client164::OpenSignEntity(
		v1_6_4::play_to_client_packet_open_sign_entity::Ty {
			tile_entity_id: 0,
			location: p,
		},
	)])
}

fn offline_uuid(name: &str) -> String {
	let mut h = [0u64; 2];
	for (i, seed) in [0xcbf2_9ce4_8422_2325u64, 0x6c62_272e_07bb_0142]
		.into_iter()
		.enumerate()
	{
		h[i] = format!("OfflinePlayer:{name}").bytes().fold(seed, |h, b| {
			(h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
		});
	}
	let u = (h[0] as u128) << 64 | h[1] as u128;
	let u = u & !(0xf << 76) & !(0x3 << 62) | 0x3 << 76 | 0x2 << 62;
	let s = format!("{u:032x}");
	format!(
		"{}-{}-{}-{}-{}",
		&s[..8],
		&s[8..12],
		&s[12..16],
		&s[16..20],
		&s[20..]
	)
}

pub fn prenetty_named_entity_spawn_1_7(
	p: v1_6_4::play_to_client_packet_named_entity_spawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client17>> {
	Ok(vec![Client17::NamedEntitySpawn(
		v1_7::play_to_client_packet_named_entity_spawn::Ty {
			entity_id: p.entity_id,
			player_uuid: offline_uuid(&p.player_name),
			player_name: p.player_name,
			data: vec![],
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			current_item: p.current_item,
			metadata: metadata_up(p.metadata),
		},
	)])
}

pub fn prenetty_named_entity_spawn_1_6_4(
	p: v1_7::play_to_client_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client164>> {
	super::entities::track(cx, p.entity_id, super::Kind::Player);
	Ok(vec![Client164::NamedEntitySpawn(
		v1_6_4::play_to_client_packet_named_entity_spawn::Ty {
			entity_id: p.entity_id,
			player_name: text::truncate(&p.player_name, 16),
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			current_item: item_id(p.current_item, TO),
			metadata: metadata_down(p.metadata),
		},
	)])
}

type Meta164 = v1_6_4::entity_metadata::Ty;
type Meta17 = v1_7::entity_metadata::Ty;
type Item164 = v1_6_4::entity_metadata_item::Ty;
type Item17 = v1_7::entity_metadata_item::Ty;

fn metadata_up(m: Meta164) -> Meta17 {
	m.into_iter()
		.map(|e| v1_7::entity_metadata_items::Ty {
			anon: v1_7::entity_metadata_items_anon::Ty {
				r#type: e.anon.r#type,
				key: e.anon.key,
			},
			value: match e.value {
				Item164::V0(x) => Item17::V0(x),
				Item164::V1(x) => Item17::V1(x),
				Item164::V2(x) => Item17::V2(x),
				Item164::V3(x) => Item17::V3(x),
				Item164::V4(x) => Item17::V4(x),
				Item164::V5(x) => Item17::V5(x),
				Item164::V6(p) => Item17::V6(v1_7::entity_metadata_item_v6::Ty {
					x: p.x,
					y: p.y,
					z: p.z,
				}),
				Item164::Default(t) => Item17::Default(t),
			},
		})
		.collect()
}

fn metadata_down(m: Meta17) -> Meta164 {
	m.into_iter()
		.filter_map(|e| {
			let value = match e.value {
				Item17::V0(x) => Item164::V0(x),
				Item17::V1(x) => Item164::V1(x),
				Item17::V2(x) => Item164::V2(x),
				Item17::V3(x) => Item164::V3(x),
				Item17::V4(x) => Item164::V4(text::truncate(&x, i16::MAX as usize)),
				Item17::V5(x) => Item164::V5(super::slot(x, TO)),
				Item17::V6(p) => Item164::V6(v1_6_4::entity_metadata_item_v6::Ty {
					x: p.x,
					y: p.y,
					z: p.z,
				}),
				Item17::V7(_) => return None,
				Item17::Default(t) => Item164::Default(t),
			};
			Some(v1_6_4::entity_metadata_items::Ty {
				anon: v1_6_4::entity_metadata_items_anon::Ty {
					r#type: e.anon.r#type,
					key: e.anon.key,
				},
				value,
			})
		})
		.collect()
}

pub fn prenetty_metadata_1_7(m: Meta164, _cx: &mut Session) -> Result<Meta17> {
	Ok(metadata_up(m))
}

pub fn prenetty_metadata_1_6_4(m: Meta17, _cx: &mut Session) -> Result<Meta164> {
	Ok(metadata_down(m))
}

pub fn prenetty_multi_block_change_1_6_4(
	mut p: v1_7::play_to_client_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client164>> {
	for r in &mut p.records {
		r.block_id = super::remap_record(r.block_id, TO);
	}
	Ok(vec![Client164::MultiBlockChange(p)])
}

pub fn prenetty_block_change_1_7(
	p: v1_6_4::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client17>> {
	Ok(vec![Client17::BlockChange(
		v1_7::play_to_client_packet_block_change::Ty {
			location: p.location,
			r#type: p.r#type as u16 as i32,
			metadata: p.metadata,
		},
	)])
}

pub fn prenetty_block_change_1_6_4(
	p: v1_7::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client164>> {
	let (id, meta) = ids::block(p.r#type as u16, p.metadata, TO);
	Ok(vec![Client164::BlockChange(
		v1_6_4::play_to_client_packet_block_change::Ty {
			location: p.location,
			r#type: id as i16,
			metadata: meta,
		},
	)])
}

pub fn prenetty_map_chunk_bulk_1_6_4_down(
	mut p: v1_7::play_to_client_packet_map_chunk_bulk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client164>> {
	let columns: Vec<_> = p
		.meta
		.iter()
		.map(|m| (m.x, m.z, m.bit_map, m.add_bit_map))
		.collect();
	p.compressed_chunk_data = remap_bulk(&p.compressed_chunk_data, &columns, p.sky_light_sent, TO)?;
	Ok(vec![Client164::MapChunkBulk(p)])
}

pub fn prenetty_spawn_entity_1_6_4(
	mut p: v1_7::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client164>> {
	type Data = v1_7::play_to_client_packet_spawn_entity_object_data::Ty;
	type Old = v1_6_4::play_to_client_packet_spawn_entity_object_data::Ty;
	let data = match &p.object_data {
		Data::Default(d, _) => *d,
		Data::V0 => 0,
	};
	super::entities::track(cx, p.entity_id, super::Kind::Object(p.r#type, data));
	if p.r#type == 70 {
		if let Data::Default(data, v) = p.object_data {
			p.object_data = Data::Default(falling_block(data, TO, true), v);
		}
	}
	let object_data = match p.object_data {
		Data::V0 => Old::V0,
		Data::Default(t, v) => Old::Default(
			t,
			v1_6_4::play_to_client_packet_spawn_entity_object_data_default::Ty {
				velocity_x: v.velocity_x,
				velocity_y: v.velocity_y,
				velocity_z: v.velocity_z,
			},
		),
	};
	Ok(vec![Client164::SpawnEntity(
		v1_6_4::play_to_client_packet_spawn_entity::Ty {
			entity_id: p.entity_id,
			r#type: p.r#type,
			x: p.x,
			y: p.y,
			z: p.z,
			pitch: p.pitch,
			yaw: p.yaw,
			object_data: object_data,
		},
	)])
}

fn particle(name: &str, up: bool) -> String {
	let mut parts: Vec<String> = name.splitn(3, '_').map(str::to_owned).collect();
	if up && parts[0] == "tilecrack" {
		parts[0] = "blockcrack".to_owned();
	}
	if !up && parts.len() == 3 && (parts[0] == "blockcrack" || parts[0] == "blockdust") {
		parts[0] = "tilecrack".to_owned();
		if let (Ok(id), Ok(meta)) = (parts[1].parse::<u16>(), parts[2].parse::<u8>()) {
			let (id, meta) = ids::block(id, meta, TO);
			parts[1] = id.to_string();
			parts[2] = meta.to_string();
		}
	}
	parts.join("_")
}

pub fn prenetty_particle_1_7(
	p: &v1_6_4::play_to_client_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(particle(&p.particle_name, true))
}

pub fn prenetty_particle_1_6_4(
	p: &v1_7::play_to_client_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(particle(&p.particle_name, false))
}

pub fn prenetty_client_command_1_7(
	p: v1_6_4::play_to_server_packet_client_command::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server17>> {
	use v1_6_4::play_to_server_packet_client_command::Ty as C;
	match p {
		C::PerformRespawn => Ok(vec![Server17::ClientCommand(
			v1_7::play_to_server_packet_client_command::Ty::PerformRespawn,
		)]),
		_ => Ok(vec![]),
	}
}

pub fn prenetty_client_command_1_6_4(
	p: v1_7::play_to_server_packet_client_command::Ty,
	cx: &mut Session,
) -> Result<Vec<Server164>> {
	use v1_7::play_to_server_packet_client_command::Ty as C;
	match p {
		C::PerformRespawn => Ok(vec![Server164::ClientCommand(
			v1_6_4::play_to_server_packet_client_command::Ty::PerformRespawn,
		)]),
		C::RequestStats => {
			let stats: Vec<_> = state(cx)
				.stats
				.iter()
				.filter_map(|(id, v)| {
					stat_name(*id).map(|name| v1_7::play_to_client_packet_statistics_items::Ty {
						name,
						value: *v,
					})
				})
				.collect();
			let mut out = Output::new();
			if crate::gen::v1_7::play::to_client::packet::serialize(
				&Client17::Statistics(stats),
				&mut out,
			)
			.is_ok()
			{
				cx.send(Root::PlayToClient, Version::V1_7, out.into_inner());
			}
			Ok(vec![])
		}
		_ => Ok(vec![]),
	}
}

pub fn prenetty_entity_action_1_7(
	p: v1_6_4::play_to_server_packet_entity_action::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server17>> {
	use v1_7::play_to_server_packet_entity_action_action_id::Ty as A;
	let action = match p.action_id {
		1 => A::StartSneaking,
		2 => A::StopSneaking,
		3 => A::StopSleeping,
		4 => A::StartSprinting,
		5 => A::StopSprinting,
		6 => A::StartRidingJump,
		7 => A::OpenInventory,
		n => A::Default(n),
	};
	Ok(vec![Server17::EntityAction(
		v1_7::play_to_server_packet_entity_action::Ty {
			entity_id: p.entity_id,
			action_id: action,
			jump_boost: p.jump_boost,
		},
	)])
}

pub fn prenetty_entity_action_1_6_4(
	p: v1_7::play_to_server_packet_entity_action::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server164>> {
	use v1_7::play_to_server_packet_entity_action_action_id::Ty as A;
	let action = match p.action_id {
		A::StartSneaking => 1,
		A::StopSneaking => 2,
		A::StopSleeping => 3,
		A::StartSprinting => 4,
		A::StopSprinting => 5,
		A::StartRidingJump => 6,
		A::OpenInventory => 7,
		A::Default(n) => n,
	};
	Ok(vec![Server164::EntityAction(
		v1_6_4::play_to_server_packet_entity_action::Ty {
			entity_id: p.entity_id,
			action_id: action,
			jump_boost: p.jump_boost,
		},
	)])
}

pub fn prenetty_settings_1_7(
	p: v1_6_4::play_to_server_packet_settings::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server17>> {
	let view = match p.view_distance {
		0 => 16,
		1 => 8,
		2 => 4,
		_ => 2,
	};
	Ok(vec![Server17::Settings(
		v1_7::play_to_server_packet_settings::Ty {
			locale: p.locale,
			view_distance: view,
			chat_flags: p.chat_flags & 7,
			chat_colors: p.chat_flags & 8 != 0,
			difficulty: p.difficulty,
			show_cape: p.show_cape,
		},
	)])
}

pub fn prenetty_settings_1_6_4(
	p: v1_7::play_to_server_packet_settings::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server164>> {
	let view = match p.view_distance {
		..=2 => 3,
		3..=4 => 2,
		5..=8 => 1,
		_ => 0,
	};
	Ok(vec![Server164::Settings(
		v1_6_4::play_to_server_packet_settings::Ty {
			locale: text::truncate(&p.locale, 7),
			view_distance: view,
			chat_flags: p.chat_flags & 7 | (p.chat_colors as i8) << 3,
			difficulty: p.difficulty,
			show_cape: p.show_cape,
		},
	)])
}

pub fn prenetty_use_entity_1_7(
	p: v1_6_4::play_to_server_packet_use_entity::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server17>> {
	Ok(vec![Server17::UseEntity(
		v1_7::play_to_server_packet_use_entity::Ty {
			target: p.target,
			mouse: v1_7::play_to_server_packet_use_entity_mouse::Ty::Default(p.mouse),
		},
	)])
}

pub fn prenetty_use_entity_1_6_4(
	p: v1_7::play_to_server_packet_use_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Server164>> {
	let mouse = match p.mouse {
		v1_7::play_to_server_packet_use_entity_mouse::Ty::Default(m) => m,
		_ => return Ok(vec![]),
	};
	Ok(vec![Server164::UseEntity(
		v1_6_4::play_to_server_packet_use_entity::Ty {
			user: state(cx).player,
			target: p.target,
			mouse,
		},
	)])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn records_drop_blocks_1_6_4_lacks() {
		let mut cx = Session::new();
		let p = v1_7::play_to_client_packet_multi_block_change::Ty {
			chunk_x: 0,
			chunk_z: 0,
			data_length: 8,
			records: vec![
				v1_7::play_to_client_packet_multi_block_change_records_items::Ty {
					horizontal_pos: 0x5a,
					y: 70,
					block_id: 95 << 4 | 3,
				},
				v1_7::play_to_client_packet_multi_block_change_records_items::Ty {
					horizontal_pos: 0,
					y: 1,
					block_id: 0x234,
				},
			],
		};
		let Client164::MultiBlockChange(old) =
			super::super::one(prenetty_multi_block_change_1_6_4(p, &mut cx).unwrap())
		else {
			panic!()
		};
		let r = &old.records;
		assert_eq!(
			(r[0].horizontal_pos, r[0].y, r[0].block_id),
			(0x5a, 70, 20 << 4)
		);
		assert_eq!(r[1].block_id, 0x234);
	}

	#[test]
	fn statistics_turn_increments_into_totals_and_back() {
		let mut cx = Session::new();
		let up = |cx: &mut Session, amount| {
			prenetty_statistics_1_7(
				v1_6_4::play_to_client_packet_statistics::Ty {
					statistic_id: 2022,
					amount,
				},
				cx,
			)
			.unwrap()
		};
		up(&mut cx, 2);
		let Client17::Statistics(s) = super::super::one(up(&mut cx, 3)) else {
			panic!()
		};
		assert_eq!((s[0].name.as_str(), s[0].value), ("stat.deaths", 5));
		let down = prenetty_statistics_1_6_4(s, &mut cx).unwrap();
		assert!(
			matches!(down.as_slice(), [Client164::Statistics(p)] if (p.statistic_id, p.amount) == (2022, 5))
		);
		assert_eq!(stat_id("stat.mineBlock.1"), Some(16777217));
	}

	#[test]
	fn animations_and_particles_are_renumbered() {
		for a in [1, 2, 3, 5, 6, 7, 104, 105] {
			assert_eq!(animation_down(animation_up(a).unwrap()), a);
		}
		assert_eq!(particle("tilecrack_1_0", true), "blockcrack_1_0");
		assert_eq!(particle("blockcrack_95_3", false), "tilecrack_20_0");
		assert_eq!(
			sound(&SOUNDS_1_7, "damage.hit", true).as_deref(),
			Some("game.neutral.hurt")
		);
	}
}
