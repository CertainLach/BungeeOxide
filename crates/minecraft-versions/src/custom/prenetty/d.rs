use super::c::{blocks, remap_bulk};
use super::ids::{self, V};
use super::text;
use super::{falling_block, item_id, state, Kind};
use crate::gen::convert::{index_of, v1_5_2, v1_6_1, v1_6_2, Root};
use crate::pds::convert::Result;
use crate::pds::Output;
use crate::session::Session;

type Client152 = v1_5_2::play_toClient_packet::Ty;
type Client161 = v1_6_1::play_toClient_packet::Ty;
type Server152 = v1_5_2::play_toServer_packet::Ty;
type Entry = v1_6_1::entityMetadata__items::Ty;
type Value = v1_6_1::entityMetadataItem::Ty;

const TO: V = V::R1_5;

blocks!(
	V::R1_5,
	v1_5_2,
	prenetty_block_change_1_5_2_down,
	prenetty_multi_block_change_1_5_2_down,
	prenetty_map_chunk_1_5_2_down,
	prenetty_world_event_1_5_2_down
);

pub fn prenetty_map_chunk_bulk_1_5_2_down(
	mut p: v1_6_1::play_toClient_packet_map_chunk_bulk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client152>> {
	let columns: Vec<_> = p
		.meta
		.iter()
		.map(|m| (m.x, m.z, m.bitMap, m.addBitMap))
		.collect();
	p.compressedChunkData = remap_bulk(&p.compressedChunkData, &columns, p.skyLightSent, TO)?;
	Ok(vec![Client152::map_chunk_bulk(p)])
}

pub fn prenetty_chat_1_6_1(message: String, _cx: &mut Session) -> Result<Vec<Client161>> {
	Ok(vec![Client161::chat(text::legacy_to_json(&message))])
}

pub fn prenetty_chat_1_5_2(message: String, _cx: &mut Session) -> Result<Vec<Client152>> {
	Ok(vec![Client152::chat(text::truncate(
		&text::json_to_legacy(&message),
		119,
	))])
}

#[derive(Clone, Copy, PartialEq)]
enum Slot {
	Byte,
	Int,
	Float,
	String,
}

fn kind_of(v: &Value) -> Option<Slot> {
	Some(match v {
		Value::v0(_) => Slot::Byte,
		Value::v2(_) => Slot::Int,
		Value::v3(_) => Slot::Float,
		Value::v4(_) => Slot::String,
		_ => return None,
	})
}

fn number(v: &Value) -> f64 {
	match v {
		Value::v0(x) => *x as f64,
		Value::v1(x) => *x as f64,
		Value::v2(x) => *x as f64,
		Value::v3(x) => *x as f64,
		_ => 0.0,
	}
}

fn cast(v: Value, to: Slot) -> (u8, Value) {
	match to {
		Slot::Byte => (0, Value::v0(number(&v) as i8)),
		Slot::Int => (2, Value::v2(number(&v) as i32)),
		Slot::Float => (3, Value::v3(number(&v) as f32)),
		Slot::String => (4, v),
	}
}

fn living(kind: Option<Kind>) -> bool {
	matches!(kind, Some(Kind::Player | Kind::Mob(_)))
}

fn rule(kind: Option<Kind>, key: u8, up: bool) -> Option<Option<(Slot, u8, Slot)>> {
	let special = match (kind, up, key) {
		(Some(Kind::Mob(63 | 64)), true, 16) => Some((Slot::Int, 6, Slot::Float)),
		(Some(Kind::Mob(63 | 64)), false, 6) => Some((Slot::Float, 16, Slot::Int)),
		(Some(Kind::Mob(95)), true, 18) => Some((Slot::Int, 18, Slot::Float)),
		(Some(Kind::Mob(95)), false, 18) => Some((Slot::Float, 18, Slot::Int)),
		(Some(Kind::Player), true, 17) => Some((Slot::Byte, 17, Slot::Float)),
		(Some(Kind::Player), false, 17) => Some((Slot::Float, 17, Slot::Byte)),
		(Some(Kind::Object(1 | 10..=12, _)), true, 19) => Some((Slot::Int, 19, Slot::Float)),
		(Some(Kind::Object(1 | 10..=12, _)), false, 19) => Some((Slot::Float, 19, Slot::Int)),
		_ => None,
	};
	if special.is_some() {
		return Some(special);
	}
	if !living(kind) {
		return None;
	}
	let table: [(u8, Slot, u8); 5] = [
		(8, Slot::Int, 7),
		(9, Slot::Byte, 8),
		(10, Slot::Byte, 9),
		(5, Slot::String, 10),
		(6, Slot::Byte, 11),
	];
	for (old, ty, new) in table {
		if up && key == old {
			return Some(Some((ty, new, ty)));
		}
		if !up && key == new {
			return Some(Some((ty, old, ty)));
		}
	}
	if !up && key == 6 {
		return Some(None);
	}
	None
}

fn remap_metadata(kind: Option<Kind>, entries: Vec<Entry>, up: bool) -> Vec<Entry> {
	let mut out = Vec::with_capacity(entries.len());
	for mut e in entries {
		match rule(kind, e.ANON.key, up) {
			None => out.push(e),
			Some(None) => {}
			Some(Some((from, key, to))) => {
				if kind_of(&e.value) != Some(from) {
					continue;
				}
				let (ty, value) = cast(e.value, to);
				e.ANON.r#type = ty;
				e.ANON.key = key;
				e.value = value;
				out.push(e);
			}
		}
	}
	out
}

pub(super) fn kind(cx: &mut Session, id: i32) -> Option<Kind> {
	let st = state(cx);
	if st.joined && id == st.player {
		return Some(Kind::Player);
	}
	st.kinds.get(&id).copied()
}

pub fn prenetty_entity_metadata_1_6_1(
	mut p: v1_5_2::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client161>> {
	let k = kind(cx, p.entityId);
	p.metadata = remap_metadata(k, p.metadata, true);
	Ok(vec![Client161::entity_metadata(p)])
}

pub fn prenetty_entity_metadata_1_5_2(
	mut p: v1_6_1::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client152>> {
	let k = kind(cx, p.entityId);
	p.metadata = remap_metadata(k, p.metadata, false)
		.into_iter()
		.filter(|e| super::entities::keeps(cx, p.entityId, e.ANON.key, e.ANON.r#type, TO))
		.collect();
	if p.metadata.is_empty() {
		return Ok(vec![]);
	}
	Ok(vec![Client152::entity_metadata(p)])
}

pub fn prenetty_spawn_entity_living_1_6_1(
	mut p: v1_5_2::play_toClient_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client161>> {
	state(cx).kinds.insert(p.entityId, Kind::Mob(p.r#type));
	p.metadata = remap_metadata(Some(Kind::Mob(p.r#type)), p.metadata, true);
	Ok(vec![Client161::spawn_entity_living(p)])
}

pub fn prenetty_spawn_entity_living_1_5_2(
	mut p: v1_6_1::play_toClient_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client152>> {
	match super::entities::mob_stand_in(p.r#type, TO) {
		Some(with) => {
			super::entities::stand_in(cx, p.entityId, Kind::Mob(with));
			p.r#type = with;
		}
		None => {
			super::entities::track(cx, p.entityId, Kind::Mob(p.r#type));
			if !ids::mob_exists(p.r#type, TO) {
				return Ok(vec![]);
			}
		}
	}
	p.metadata = remap_metadata(Some(Kind::Mob(p.r#type)), p.metadata, false)
		.into_iter()
		.filter(|e| super::entities::keeps(cx, p.entityId, e.ANON.key, e.ANON.r#type, TO))
		.collect();
	Ok(vec![Client152::spawn_entity_living(p)])
}

pub fn prenetty_named_entity_spawn_1_6_1(
	mut p: v1_5_2::play_toClient_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client161>> {
	state(cx).kinds.insert(p.entityId, Kind::Player);
	p.metadata = remap_metadata(Some(Kind::Player), p.metadata, true);
	Ok(vec![Client161::named_entity_spawn(p)])
}

pub fn prenetty_named_entity_spawn_1_5_2(
	mut p: v1_6_1::play_toClient_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client152>> {
	super::entities::track(cx, p.entityId, Kind::Player);
	p.metadata = remap_metadata(Some(Kind::Player), p.metadata, false);
	p.currentItem = item_id(p.currentItem, TO);
	Ok(vec![Client152::named_entity_spawn(p)])
}

fn object_data(p: &v1_6_1::play_toClient_packet_spawn_entity::Ty) -> i32 {
	match &p.objectData {
		v1_6_1::play_toClient_packet_spawn_entity__objectData::Ty::default(data, _) => *data,
		_ => 0,
	}
}

pub fn prenetty_spawn_entity_1_6_1(
	p: v1_5_2::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client161>> {
	let data = object_data(&p);
	state(cx)
		.kinds
		.insert(p.entityId, Kind::Object(p.r#type, data));
	Ok(vec![Client161::spawn_entity(p)])
}

pub fn prenetty_spawn_entity_1_5_2(
	mut p: v1_6_1::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client152>> {
	let data = object_data(&p);
	super::entities::track(cx, p.entityId, Kind::Object(p.r#type, data));
	if !ids::object_exists(p.r#type, TO) {
		return Ok(vec![]);
	}
	if p.r#type == 70 {
		if let v1_6_1::play_toClient_packet_spawn_entity__objectData::Ty::default(data, v) =
			p.objectData
		{
			p.objectData = v1_6_1::play_toClient_packet_spawn_entity__objectData::Ty::default(
				falling_block(data, TO, true),
				v,
			);
		}
	}
	Ok(vec![Client152::spawn_entity(p)])
}

pub fn prenetty_login_1_6_1(
	p: v1_5_2::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client161>> {
	let st = state(cx);
	st.player = p.entityId;
	st.joined = true;
	st.kinds.clear();
	Ok(vec![Client161::login(p)])
}

pub fn prenetty_login_1_5_2(
	p: v1_6_1::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client152>> {
	super::entities::joined(cx, p.entityId);
	Ok(vec![Client152::login(p)])
}

pub fn prenetty_attach_entity_1_6_1(
	p: v1_5_2::play_toClient_packet_attach_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client161>> {
	let st = state(cx);
	if p.entityId == st.player {
		st.vehicle = (p.vehicleId >= 0).then_some(p.vehicleId);
	}
	Ok(vec![Client161::attach_entity(
		v1_6_1::play_toClient_packet_attach_entity::Ty {
			entityId: p.entityId,
			vehicleId: p.vehicleId,
			leash: false,
		},
	)])
}

pub fn prenetty_attach_entity_1_5_2(
	p: v1_6_1::play_toClient_packet_attach_entity::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client152>> {
	if p.leash {
		return Ok(vec![]);
	}
	Ok(vec![Client152::attach_entity(
		v1_5_2::play_toClient_packet_attach_entity::Ty {
			entityId: p.entityId,
			vehicleId: p.vehicleId,
		},
	)])
}

pub fn prenetty_custom_payload_1_6_1(
	mut p: v1_5_2::play_toClient_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client161>> {
	if p.channel == "MC|TPack" {
		let data = String::from_utf8_lossy(&p.data).into_owned();
		let mut parts = data.split('\0');
		let url = parts.next().unwrap_or_default().to_owned();
		if parts.next().is_some_and(|size| size != "16") {
			return Ok(vec![]);
		}
		p.channel = "MC|RPack".to_owned();
		p.data = url.into_bytes();
	}
	Ok(vec![Client161::custom_payload(p)])
}

pub fn prenetty_custom_payload_1_5_2(
	mut p: v1_6_1::play_toClient_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client152>> {
	if p.channel == "MC|RPack" {
		p.channel = "MC|TPack".to_owned();
		p.data.extend_from_slice(b"\x0016");
	}
	Ok(vec![Client152::custom_payload(p)])
}

pub fn prenetty_steer_vehicle_1_5_2(
	p: v1_6_1::play_toServer_packet_steer_vehicle::Ty,
	cx: &mut Session,
) -> Result<Vec<Server152>> {
	let st = state(cx);
	let pressed = p.unmount && !st.sneaking;
	st.sneaking = p.unmount;
	match (pressed, st.vehicle) {
		(true, Some(vehicle)) => Ok(vec![Server152::use_entity(
			v1_5_2::play_toServer_packet_use_entity::Ty {
				user: st.player,
				target: vehicle,
				mouse: 0,
			},
		)]),
		_ => Ok(vec![]),
	}
}

pub fn prenetty_entity_action_1_5_2(
	p: v1_6_1::play_toServer_packet_entity_action::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server152>> {
	if p.actionId > 5 {
		return Ok(vec![]);
	}
	Ok(vec![Server152::entity_action(
		v1_5_2::play_toServer_packet_entity_action::Ty {
			entityId: p.entityId,
			actionId: p.actionId,
		},
	)])
}

pub fn prenetty_block_place_1_6_1(
	p: v1_6_2::play_toServer_packet_block_place::Ty,
	cx: &mut Session,
) -> Result<Vec<v1_6_1::play_toServer_packet::Ty>> {
	let is_sign = matches!(&p.heldItem, v1_6_2::slot::Ty::default(323, _));
	if is_sign && p.direction != -1 && p.direction != 0 {
		let (dx, dy, dz) = match p.direction {
			1 => (0, 1, 0),
			2 => (0, 0, -1),
			3 => (0, 0, 1),
			4 => (-1, 0, 0),
			_ => (1, 0, 0),
		};
		let packet = v1_6_2::play_toClient_packet::Ty::open_sign_entity(
			v1_6_2::play_toClient_packet_open_sign_entity::Ty {
				tileEntityId: 0,
				location: v1_6_2::position_iii::Ty {
					x: p.location.x + dx,
					y: p.location.y as i32 + dy,
					z: p.location.z + dz,
				},
			},
		);
		let mut out = Output::new();
		if crate::gen::pds_v1_6_2_play_toClient_packet::serialize(&packet, &mut out).is_ok() {
			if let Some(version) = index_of("1.6.2") {
				cx.send(Root::PlayToClient, version, out.into_inner());
			}
		}
	}
	Ok(vec![v1_6_1::play_toServer_packet::Ty::block_place(p)])
}

pub fn prenetty_health_1_5_2(
	src: &v1_6_1::play_toClient_packet_update_health::Ty,
	_cx: &mut Session,
) -> Result<i16> {
	let health = src.health;
	Ok(if health.is_nan() || health <= 0.0 {
		0
	} else {
		(health.round() as i16).max(1)
	})
}
