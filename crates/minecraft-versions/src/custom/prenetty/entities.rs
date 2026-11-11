use super::c::meta_1_3_2;
use super::d::kind;
use super::{old_meta_item, state, text, Kind};
use crate::gen::convert::Version;
use crate::gen::convert::{v1_0_1, v1_1, v1_2_3, v1_3_2, v1_4_2, v1_5_2, v1_6_4, v1_7};
use crate::pds::convert::Result;
use crate::session::Session;

const ARTS: [&str; 25] = [
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
	"Fighters",
	"Pointer",
	"Pigscene",
	"BurningSkull",
	"Skeleton",
	"DonkeyKong",
];

pub fn prenetty_painting_title_1_3_2(
	p: &v1_4_2::play_to_client_packet_spawn_entity_painting::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(match p.title.as_str() {
		t if ARTS.contains(&t) => t,
		"Wither" => "SkullAndRoses",
		_ => "Kebab",
	}
	.to_owned())
}

fn effect_exists(id: i8, to: Version) -> bool {
	let since = match id {
		1..=19 => Version::V1_0_1,
		20 => Version::V1_4_2,
		21..=23 => Version::V1_6_1,
		_ => return false,
	};
	since <= to
}

macro_rules! effects {
	($to:expr, $m:ident, $effect:ident, $remove:ident) => {
		pub fn $effect(
			p: $m::play_to_client_packet_entity_effect::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_client_packet::Ty>> {
			if !effect_exists(p.effect_id, $to) {
				return Ok(vec![]);
			}
			Ok(vec![$m::play_to_client_packet::Ty::EntityEffect(p)])
		}

		pub fn $remove(
			p: $m::play_to_client_packet_remove_entity_effect::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_client_packet::Ty>> {
			if !effect_exists(p.effect_id, $to) {
				return Ok(vec![]);
			}
			Ok(vec![$m::play_to_client_packet::Ty::RemoveEntityEffect(p)])
		}
	};
}

effects!(
	Version::V1_5_2,
	v1_5_2,
	prenetty_entity_effect_1_5_2,
	prenetty_remove_entity_effect_1_5_2
);
effects!(
	Version::V1_3_2,
	v1_3_2,
	prenetty_entity_effect_1_3_2,
	prenetty_remove_entity_effect_1_3_2
);

pub(super) fn track(cx: &mut Session, id: i32, kind: Kind) {
	let st = state(cx);
	if st.kinds.insert(id, kind) != Some(kind) {
		st.stand_ins.remove(&id);
	}
}

pub(super) fn stand_in(cx: &mut Session, id: i32, kind: Kind) {
	let st = state(cx);
	st.kinds.insert(id, kind);
	st.stand_ins.insert(id);
}

pub(super) fn joined(cx: &mut Session, player: i32) {
	let st = state(cx);
	st.player = player;
	st.joined = true;
	st.kinds.clear();
	st.stand_ins.clear();
}

fn player(cx: &mut Session, id: i32) -> bool {
	kind(cx, id) == Some(Kind::Player)
}

fn object(cx: &mut Session, id: i32) -> bool {
	matches!(kind(cx, id), Some(Kind::Object(..)))
}

pub(super) fn mob_stand_in(ty: u8, to: Version) -> Option<u8> {
	let (since, with) = match ty {
		100 => (Version::V1_6_1, 90),
		64 => (Version::V1_4_2, 56),
		65 => (Version::V1_4_2, 93),
		66 => (Version::V1_4_2, 61),
		98 => (Version::V1_2_3, 95),
		99 => (Version::V1_2_3, 97),
		_ => return None,
	};
	(to < since).then_some(with)
}

pub(super) fn object_stand_in(ty: i8, to: Version) -> Option<i8> {
	let (since, with) = match ty {
		66 => (Version::V1_4_2, 64),
		75 => (Version::V1_2_3, 61),
		_ => return None,
	};
	(to < since).then_some(with)
}

fn base_entry(living: bool, key: u8, ty: u8, to: Version) -> bool {
	match (key, ty) {
		(0, 0) | (1, 1) => true,
		_ if !living => false,
		(8, 2) => true,
		(9, 0) => to >= Version::V1_4_2,
		(10, 0) => to >= Version::V1_4_5,
		(5, 4) | (6, 0) => to >= Version::V1_5_1,
		_ => false,
	}
}

pub(super) fn keeps(cx: &mut Session, id: i32, key: u8, ty: u8, to: Version) -> bool {
	let st = state(cx);
	if !st.stand_ins.contains(&id) {
		return true;
	}
	let living = matches!(st.kinds.get(&id), Some(Kind::Mob(_)));
	base_entry(living, key, ty, to)
}

pub fn prenetty_collect_1_6_4(
	p: v1_7::play_to_client_packet_collect::Ty,
	cx: &mut Session,
) -> Result<Vec<v1_6_4::play_to_client_packet::Ty>> {
	if object(cx, p.collector_entity_id) {
		return Ok(vec![]);
	}
	Ok(vec![v1_6_4::play_to_client_packet::Ty::Collect(
		v1_6_4::play_to_client_packet_collect::Ty {
			collected_entity_id: p.collected_entity_id,
			collector_entity_id: p.collector_entity_id,
		},
	)])
}

pub(super) fn animation_reaches_1_6_4(cx: &mut Session, id: i32, animation: u8) -> bool {
	match animation {
		1 => !object(cx, id),
		3 => player(cx, id),
		_ => true,
	}
}

pub(super) fn bed_reaches_1_6_4(cx: &mut Session, id: i32) -> bool {
	player(cx, id)
}

type Client132 = v1_3_2::play_to_client_packet::Ty;

pub fn prenetty_animation_1_3_2(
	p: v1_4_2::play_to_client_packet_animation::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	if matches!(p.animation, 1 | 3 | 4) && !player(cx, p.entity_id) {
		return Ok(vec![]);
	}
	Ok(vec![Client132::Animation(p)])
}

pub fn prenetty_bed_1_3_2(
	p: v1_4_2::play_to_client_packet_bed::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	if p.action == 0 && !player(cx, p.entity_id) {
		return Ok(vec![]);
	}
	Ok(vec![Client132::Bed(p)])
}

pub fn prenetty_collect_1_3_2(
	p: v1_4_2::play_to_client_packet_collect::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	if object(cx, p.collector_entity_id) {
		return Ok(vec![]);
	}
	Ok(vec![Client132::Collect(p)])
}

pub fn prenetty_entity_metadata_1_3_2(
	p: v1_4_2::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	let metadata: Vec<_> = meta_1_3_2(p.metadata)
		.into_iter()
		.filter(|e| keeps(cx, p.entity_id, e.anon.key, e.anon.r#type, Version::V1_3_2))
		.collect();
	if metadata.is_empty() {
		return Ok(vec![]);
	}
	Ok(vec![Client132::EntityMetadata(
		v1_3_2::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
			metadata,
		},
	)])
}

type Client11 = v1_1::play_to_client_packet::Ty;

pub(super) fn meta_1_1(
	cx: &mut Session,
	id: i32,
	metadata: v1_2_3::entity_metadata::Ty,
) -> v1_1::entity_metadata::Ty {
	metadata
		.into_iter()
		.filter(|e| keeps(cx, id, e.anon.key, e.anon.r#type, Version::V1_1))
		.map(|mut e| {
			e.value = old_meta_item(e.value, Version::V1_1);
			e
		})
		.collect()
}

pub fn prenetty_entity_metadata_1_1(
	p: v1_2_3::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let metadata = meta_1_1(cx, p.entity_id, p.metadata);
	if metadata.is_empty() {
		return Ok(vec![]);
	}
	Ok(vec![Client11::EntityMetadata(
		v1_1::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
			metadata,
		},
	)])
}

pub fn prenetty_open_window_1_1(
	mut p: v1_2_3::play_to_client_packet_open_window::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client11>> {
	p.window_title = text::truncate(&p.window_title, 16);
	Ok(vec![Client11::OpenWindow(p)])
}

pub fn prenetty_kick_1_0_1(
	reason: String,
	_cx: &mut Session,
) -> Result<Vec<v1_0_1::play_to_client_packet::Ty>> {
	Ok(vec![v1_0_1::play_to_client_packet::Ty::KickDisconnect(
		text::truncate(&reason, 100),
	)])
}
