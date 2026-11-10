mod chunk;
mod items;
mod meta;
mod sounds;

pub use chunk::*;
pub use sounds::*;

use std::collections::{HashMap, VecDeque};

use crate::gen::convert::{
	v1_10, v1_11, v1_12, v1_12_1, v1_12_2, v1_8, v1_9, v1_9_2, v1_9_4, Version,
};
use crate::pds::convert::Result;
use crate::session::{Kind, Session};

type C8 = v1_8::play_toClient_packet::Ty;
type C92 = v1_9_2::play_toClient_packet::Ty;
type C94 = v1_9_4::play_toClient_packet::Ty;
type C10 = v1_10::play_toClient_packet::Ty;
type C11 = v1_11::play_toClient_packet::Ty;
type C121 = v1_12_1::play_toClient_packet::Ty;
type S9 = v1_9::play_toServer_packet::Ty;
type S122 = v1_12_2::play_toServer_packet::Ty;
type Slot = items::Slot;

#[derive(Default)]
struct Player {
	x: f64,
	y: f64,
	z: f64,
	yaw: f32,
	pitch: f32,
}

#[derive(Clone, Copy)]
struct Teleport {
	id: i32,
	at: (f64, f64, f64),
	relative: bool,
}

const RELATIVE_SLACK: f64 = 16.0;
const UNANSWERED: u32 = 60;

impl Teleport {
	fn answered_by(&self, (x, y, z): (f64, f64, f64), look: bool) -> bool {
		let (tx, ty, tz) = self.at;
		if self.relative {
			return look
				&& [tx - x, ty - y, tz - z]
					.iter()
					.all(|d| d.abs() <= RELATIVE_SLACK);
		}
		tx == x && (ty - y).abs() <= 1e-7 && tz == z
	}
}

#[derive(Default)]
struct Rewind {
	kinds: HashMap<(Version, i32), Kind>,
	moves: HashMap<i32, [i32; 3]>,
	status: HashMap<i32, meta::Status>,
	horses: HashMap<i32, meta::Horse>,
	passengers: HashMap<i32, Vec<i32>>,
	windows: HashMap<u8, String>,
	enchantments: [i16; 3],
	teleports: VecDeque<Teleport>,
	unanswered: u32,
	player: Player,
	swing: bool,
	keep_alive: VecDeque<(i32, i64)>,
}

fn state(cx: &mut Session) -> &mut Rewind {
	cx.state::<Rewind>()
}

fn kind(cx: &mut Session, step: Version, id: i32) -> Kind {
	state(cx)
		.kinds
		.get(&(step, id))
		.copied()
		.unwrap_or(Kind::Player)
}

fn fixed(v: f64) -> i32 {
	(v * 32.0).floor() as i32
}

fn remainder(v: f64) -> i32 {
	((v * 4096.0).round() as i64 - (v * 32.0).floor() as i64 * 128) as i32
}

fn tracked(cx: &mut Session, id: i32, x: f64, y: f64, z: f64) {
	state(cx)
		.moves
		.insert(id, [remainder(x), remainder(y), remainder(z)]);
}

fn split_move(cx: &mut Session, id: i32, delta: [i16; 3]) -> Vec<[i8; 3]> {
	let r = state(cx).moves.entry(id).or_default();
	let mut total = [0i32; 3];
	for i in 0..3 {
		let t = r[i] + delta[i] as i32;
		total[i] = t.div_euclid(128);
		r[i] = t.rem_euclid(128);
	}
	if total.iter().all(|t| (-128..=127).contains(t)) {
		return vec![total.map(|t| t as i8)];
	}
	let half = total.map(|t| t / 2);
	vec![
		half.map(|t| t as i8),
		[0, 1, 2].map(|i| (total[i] - half[i]) as i8),
	]
}

fn boat(cx: &mut Session, id: i32) -> bool {
	kind(cx, Version::V1_9, id) == Kind::Object(1)
}

pub fn legacy_down_slot_1_8(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(items::down(Version::V1_8, Version::V1_9, v, meta::mob_1_8))
}

pub fn legacy_up_slot_1_9(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(items::up(Version::V1_9, Version::V1_8, v))
}

pub fn legacy_down_slot_1_9_4(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(items::down(Version::V1_9_4, Version::V1_10, v, |m| m))
}

pub fn legacy_up_slot_1_10(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(items::up(Version::V1_10, Version::V1_9_4, v))
}

pub fn legacy_down_slot_1_10(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(items::down(Version::V1_10, Version::V1_11, v, |m| {
		meta::mob_1_10(m).0
	}))
}

pub fn legacy_up_slot_1_11(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(items::up(Version::V1_11, Version::V1_10, v))
}

pub fn legacy_down_slot_1_11(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(items::down(Version::V1_11, Version::V1_12, v, |m| {
		meta::mob_1_11(m).0
	}))
}

pub fn legacy_up_slot_1_12(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(items::up(Version::V1_12, Version::V1_11, v))
}

fn slots_down(target: Version, newer: Version, metadata: &mut [meta::E9]) {
	for e in metadata.iter_mut() {
		if let meta::M9::v5(s) = &e.value {
			let mob = |m: u32| match target {
				Version::V1_8 => meta::mob_1_8(m),
				Version::V1_10 => meta::mob_1_10(m).0,
				Version::V1_11 => meta::mob_1_11(m).0,
				_ => m,
			};
			e.value = meta::M9::v5(items::down(target, newer, s.clone(), mob));
		}
	}
}

pub fn legacy_down_spawn_entity_1_8(
	p: v1_9::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	use v1_8::play_toClient_packet_spawn_entity__velocity::Ty as V8;
	let ty = match p.r#type as u8 {
		3 | 91 | 93 => return Ok(vec![]),
		67 => 66,
		t => t,
	};
	state(cx)
		.kinds
		.insert((Version::V1_9, p.entityId), Kind::Object(ty));
	tracked(cx, p.entityId, p.x, p.y, p.z);
	let (mut y, mut yaw, mut data) = (fixed(p.y), p.yaw, p.objectData);
	match ty {
		1 => {
			yaw = yaw.wrapping_sub(64);
			y += 6;
		}
		60 if data > 0 => data -= 1,
		70 => data = items::state_back(Version::V1_8, data),
		_ => {}
	}
	let mut out = Vec::with_capacity(2);
	let velocity = if data > 0 {
		V8::default(data, p.velocity.clone())
	} else {
		if (p.velocity.x, p.velocity.y, p.velocity.z) != (0, 0, 0) {
			out.push(C8::entity_velocity(
				v1_8::play_toClient_packet_entity_velocity::Ty {
					entityId: p.entityId,
					velocity: p.velocity.clone(),
				},
			));
		}
		V8::v0
	};
	out.insert(
		0,
		C8::spawn_entity(v1_8::play_toClient_packet_spawn_entity::Ty {
			entityId: p.entityId,
			r#type: ty as i8,
			x: fixed(p.x),
			y,
			z: fixed(p.z),
			pitch: p.pitch,
			yaw,
			velocity,
		}),
	);
	Ok(out)
}

pub fn legacy_down_spawn_entity_living_1_8(
	p: v1_9::play_toClient_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	let kind = Kind::Mob(p.r#type);
	state(cx).kinds.insert((Version::V1_9, p.entityId), kind);
	tracked(cx, p.entityId, p.x, p.y, p.z);
	let status = state(cx).status.entry(p.entityId).or_default();
	let metadata = meta::down_1_8(kind, p.metadata, status);
	Ok(vec![C8::spawn_entity_living(
		v1_8::play_toClient_packet_spawn_entity_living::Ty {
			entityId: p.entityId,
			r#type: meta::mob_1_8(p.r#type as u32) as u8,
			x: fixed(p.x),
			y: fixed(p.y),
			z: fixed(p.z),
			yaw: p.yaw,
			pitch: p.pitch,
			headPitch: p.headPitch,
			velocity: p.velocity,
			metadata,
		},
	)])
}

pub fn legacy_down_named_entity_spawn_1_8(
	p: v1_9::play_toClient_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	state(cx)
		.kinds
		.insert((Version::V1_9, p.entityId), Kind::Player);
	tracked(cx, p.entityId, p.x, p.y, p.z);
	let status = state(cx).status.entry(p.entityId).or_default();
	let metadata = meta::down_1_8(Kind::Player, p.metadata, status);
	Ok(vec![C8::named_entity_spawn(
		v1_8::play_toClient_packet_named_entity_spawn::Ty {
			entityId: p.entityId,
			playerUUID: p.playerUUID,
			x: fixed(p.x),
			y: fixed(p.y),
			z: fixed(p.z),
			yaw: p.yaw,
			pitch: p.pitch,
			currentItem: 0,
			metadata,
		},
	)])
}

pub fn legacy_down_entity_metadata_1_8(
	p: v1_9::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	let kind = kind(cx, Version::V1_9, p.entityId);
	let status = state(cx).status.entry(p.entityId).or_default();
	let metadata = meta::down_1_8(kind, p.metadata, status);
	if metadata.is_empty() {
		return Ok(vec![]);
	}
	Ok(vec![C8::entity_metadata(
		v1_8::play_toClient_packet_entity_metadata::Ty {
			entityId: p.entityId,
			metadata,
		},
	)])
}

pub fn legacy_down_rel_entity_move_1_8(
	p: v1_9::play_toClient_packet_rel_entity_move::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	Ok(split_move(cx, p.entityId, [p.dX, p.dY, p.dZ])
		.into_iter()
		.map(|[dX, dY, dZ]| {
			C8::rel_entity_move(v1_8::play_toClient_packet_rel_entity_move::Ty {
				entityId: p.entityId,
				dX,
				dY,
				dZ,
				onGround: p.onGround,
			})
		})
		.collect())
}

pub fn legacy_down_entity_move_look_1_8(
	p: v1_9::play_toClient_packet_entity_move_look::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	let yaw = if boat(cx, p.entityId) {
		p.yaw.wrapping_sub(64)
	} else {
		p.yaw
	};
	Ok(split_move(cx, p.entityId, [p.dX, p.dY, p.dZ])
		.into_iter()
		.map(|[dX, dY, dZ]| {
			C8::entity_move_look(v1_8::play_toClient_packet_entity_move_look::Ty {
				entityId: p.entityId,
				dX,
				dY,
				dZ,
				yaw,
				pitch: p.pitch,
				onGround: p.onGround,
			})
		})
		.collect())
}

pub fn legacy_down_entity_look_1_8(
	mut p: v1_9::play_toClient_packet_entity_look::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	if boat(cx, p.entityId) {
		p.yaw = p.yaw.wrapping_sub(64);
	}
	Ok(vec![C8::entity_look(p)])
}

pub fn legacy_down_entity_teleport_1_8(
	p: v1_9::play_toClient_packet_entity_teleport::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	tracked(cx, p.entityId, p.x, p.y, p.z);
	let (mut y, mut yaw) = (fixed(p.y), p.yaw);
	if boat(cx, p.entityId) {
		y += 6;
		yaw = yaw.wrapping_sub(64);
	}
	Ok(vec![C8::entity_teleport(
		v1_8::play_toClient_packet_entity_teleport::Ty {
			entityId: p.entityId,
			x: fixed(p.x),
			y,
			z: fixed(p.z),
			yaw,
			pitch: p.pitch,
			onGround: p.onGround,
		},
	)])
}

pub fn legacy_down_entity_destroy_1_8(ids: Vec<i32>, cx: &mut Session) -> Result<Vec<C8>> {
	let st = state(cx);
	for id in &ids {
		for step in [
			Version::V1_9,
			Version::V1_10,
			Version::V1_11,
			Version::V1_12,
		] {
			st.kinds.remove(&(step, *id));
		}
		st.moves.remove(id);
		st.status.remove(id);
		st.horses.remove(id);
		st.passengers.remove(id);
	}
	Ok(vec![C8::entity_destroy(ids)])
}

fn link(entity: i32, vehicle: i32) -> C8 {
	C8::attach_entity(v1_8::play_toClient_packet_attach_entity::Ty {
		entityId: entity,
		vehicleId: vehicle,
		leash: false,
	})
}

pub fn legacy_down_set_passengers_1_8(
	p: v1_9::play_toClient_packet_set_passengers::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	let old = state(cx)
		.passengers
		.insert(p.entityId, p.passengers.clone())
		.unwrap_or_default();
	let mut out: Vec<C8> = old
		.iter()
		.filter(|o| !p.passengers.contains(o))
		.map(|o| link(*o, -1))
		.collect();
	let mut holder = p.entityId;
	for passenger in &p.passengers {
		out.push(link(*passenger, holder));
		holder = *passenger;
	}
	Ok(out)
}

pub fn legacy_down_entity_effect_1_8(
	p: v1_9::play_toClient_packet_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<Vec<C8>> {
	if p.effectId > 23 || p.effectId < 1 {
		return Ok(vec![]);
	}
	Ok(vec![C8::entity_effect(
		v1_8::play_toClient_packet_entity_effect::Ty {
			entityId: p.entityId,
			effectId: p.effectId,
			amplifier: p.amplifier,
			duration: p.duration,
			hideParticles: p.hideParticles & 2 == 0,
		},
	)])
}

pub fn legacy_down_remove_entity_effect_1_8(
	p: v1_9::play_toClient_packet_remove_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<Vec<C8>> {
	if p.effectId > 23 || p.effectId < 1 {
		return Ok(vec![]);
	}
	Ok(vec![C8::remove_entity_effect(p)])
}

pub fn legacy_down_position_1_8(
	p: v1_9::play_toClient_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	let st = state(cx);
	let me = &mut st.player;
	let rel = |bit: i8, v: f64, base: f64| if p.flags & bit != 0 { v + base } else { v };
	(me.x, me.y, me.z) = (rel(1, p.x, me.x), rel(2, p.y, me.y), rel(4, p.z, me.z));
	let relative = p.flags & 7 == 7;
	let pending = Teleport {
		id: p.teleportId,
		at: (me.x, me.y, me.z),
		relative,
	};
	let (x, y, z) = if relative {
		(p.x, p.y, p.z)
	} else {
		(me.x, me.y, me.z)
	};
	if st.teleports.len() >= 16 {
		st.teleports.pop_front();
	}
	st.teleports.push_back(pending);
	Ok(vec![C8::position(
		v1_8::play_toClient_packet_position::Ty {
			x,
			y,
			z,
			yaw: p.yaw,
			pitch: p.pitch,
			flags: p.flags & if relative { 0x1f } else { 0x18 },
		},
	)])
}

fn movement(
	cx: &mut Session,
	at: Option<(f64, f64, f64)>,
	look: Option<(f32, f32)>,
	packet: S9,
) -> Vec<S9> {
	let st = state(cx);
	let mut out = Vec::with_capacity(3);
	if std::mem::take(&mut st.swing) {
		out.push(S9::arm_animation(0));
	}
	if let Some((x, y, z)) = at {
		match st
			.teleports
			.iter()
			.rposition(|t| t.answered_by((x, y, z), look.is_some()))
		{
			Some(i) => {
				let id = st.teleports[i].id;
				st.teleports.drain(..=i);
				st.unanswered = 0;
				if st.teleports.is_empty() {
					(st.player.x, st.player.y, st.player.z) = (x, y, z);
				}
				out.push(S9::teleport_confirm(id));
			}
			None if st.teleports.is_empty() => (st.player.x, st.player.y, st.player.z) = (x, y, z),
			None => {
				st.unanswered += 1;
				if st.unanswered > UNANSWERED {
					st.teleports.clear();
					st.unanswered = 0;
					(st.player.x, st.player.y, st.player.z) = (x, y, z);
				}
			}
		}
	}
	if let Some((yaw, pitch)) = look {
		(st.player.yaw, st.player.pitch) = (yaw, pitch);
	}
	out.push(packet);
	out
}

pub fn legacy_up_flying_1_9(on_ground: bool, cx: &mut Session) -> Result<Vec<S9>> {
	Ok(movement(cx, None, None, S9::flying(on_ground)))
}

pub fn legacy_up_position_1_9(
	p: v1_8::play_toServer_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<S9>> {
	let at = Some((p.x, p.y, p.z));
	Ok(movement(cx, at, None, S9::position(p)))
}

pub fn legacy_up_look_1_9(
	p: v1_8::play_toServer_packet_look::Ty,
	cx: &mut Session,
) -> Result<Vec<S9>> {
	Ok(movement(cx, None, Some((p.yaw, p.pitch)), S9::look(p)))
}

pub fn legacy_up_position_look_1_9(
	p: v1_8::play_toServer_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<Vec<S9>> {
	let (at, look) = (Some((p.x, p.y, p.z)), Some((p.yaw, p.pitch)));
	Ok(movement(cx, at, look, S9::position_look(p)))
}

pub fn legacy_up_arm_animation_1_9(
	_p: v1_8::play_toServer_packet_arm_animation::Ty,
	cx: &mut Session,
) -> Result<Vec<S9>> {
	state(cx).swing = true;
	Ok(vec![])
}

pub fn legacy_up_block_place_1_9(
	p: v1_8::play_toServer_packet_block_place::Ty,
	_cx: &mut Session,
) -> Result<Vec<S9>> {
	if p.direction == -1 {
		return Ok(vec![S9::use_item(0)]);
	}
	Ok(vec![S9::block_place(
		v1_9::play_toServer_packet_block_place::Ty {
			location: p.location,
			direction: p.direction as i32,
			hand: 0,
			cursorX: p.cursorX,
			cursorY: p.cursorY,
			cursorZ: p.cursorZ,
		},
	)])
}

pub fn legacy_up_update_sign_1_9(
	p: v1_8::play_toServer_packet_update_sign::Ty,
	_cx: &mut Session,
) -> Result<Vec<S9>> {
	use super::v1_7_to_1_8::text::to_legacy;
	Ok(vec![S9::update_sign(
		v1_9::play_toServer_packet_update_sign::Ty {
			location: p.location,
			text1: to_legacy(&p.text1),
			text2: to_legacy(&p.text2),
			text3: to_legacy(&p.text3),
			text4: to_legacy(&p.text4),
		},
	)])
}

fn brewing(cx: &mut Session, window: u8) -> bool {
	state(cx)
		.windows
		.get(&window)
		.is_some_and(|w| w == "minecraft:brewing_stand")
}

pub fn legacy_up_window_click_1_9(
	mut p: v1_8::play_toServer_packet_window_click::Ty,
	cx: &mut Session,
) -> Result<Vec<S9>> {
	if brewing(cx, p.windowId) && p.slot > 3 {
		p.slot += 1;
	}
	p.item = items::up(Version::V1_9, Version::V1_8, p.item);
	Ok(vec![S9::window_click(p)])
}

pub fn legacy_up_close_window_1_9(window: u8, cx: &mut Session) -> Result<Vec<S9>> {
	state(cx).windows.remove(&window);
	Ok(vec![S9::close_window(window)])
}

pub fn legacy_down_open_window_1_8(
	p: v1_9::play_toClient_packet_open_window::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	state(cx)
		.windows
		.insert(p.windowId, p.inventoryType.clone());
	Ok(vec![C8::open_window(p)])
}

pub fn legacy_down_close_window_1_8(window: u8, cx: &mut Session) -> Result<Vec<C8>> {
	state(cx).windows.remove(&window);
	Ok(vec![C8::close_window(window)])
}

fn slot_down_1_8(s: Slot) -> Slot {
	items::down(Version::V1_8, Version::V1_9, s, meta::mob_1_8)
}

pub fn legacy_down_window_items_1_8(
	p: v1_9::play_toClient_packet_window_items::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	let mut items = p.items;
	if p.windowId == 0 && items.len() == 46 {
		items.truncate(45);
	} else if brewing(cx, p.windowId) && items.len() > 4 {
		items.remove(4);
	}
	Ok(vec![C8::window_items(
		v1_8::play_toClient_packet_window_items::Ty {
			windowId: p.windowId,
			items: items.into_iter().map(slot_down_1_8).collect(),
		},
	)])
}

pub fn legacy_down_set_slot_1_8(
	mut p: v1_9::play_toClient_packet_set_slot::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	if p.windowId == 0 && p.slot == 45 {
		return Ok(vec![]);
	}
	if brewing(cx, p.windowId as u8) {
		match p.slot {
			4 => return Ok(vec![]),
			s if s > 4 => p.slot -= 1,
			_ => {}
		}
	}
	p.item = slot_down_1_8(p.item);
	Ok(vec![C8::set_slot(p)])
}

pub fn legacy_down_craft_progress_bar_1_8(
	mut p: v1_9::play_toClient_packet_craft_progress_bar::Ty,
	cx: &mut Session,
) -> Result<Vec<C8>> {
	let st = state(cx);
	if st
		.windows
		.get(&p.windowId)
		.is_some_and(|w| w == "minecraft:enchanting_table")
	{
		match p.property {
			4..=6 => {
				st.enchantments[(p.property - 4) as usize] = p.value;
				return Ok(vec![]);
			}
			7..=9 => {
				p.property -= 3;
				p.value = st.enchantments[(p.property - 4) as usize] & 0xff | p.value << 8;
			}
			_ => {}
		}
	}
	Ok(vec![C8::craft_progress_bar(p)])
}

fn particles_down(
	target: Version,
	mut p: v1_8::play_toClient_packet_world_particles::Ty,
) -> Option<v1_8::play_toClient_packet_world_particles::Ty> {
	use v1_8::play_toClient_packet_world_particles__data::Ty as D;
	let limit = match target {
		Version::V1_8 => 41,
		Version::V1_9 | Version::V1_9_2 | Version::V1_9_4 => 45,
		Version::V1_10 => 46,
		_ => 48,
	};
	p.data = match p.data {
		D::v36(d) => D::v36(items::item_pair_back(target, &d)),
		D::v37(d) => D::v37(
			d.into_iter()
				.map(|s| items::state_back(target, s))
				.collect(),
		),
		D::v38(d) => D::v38(
			d.into_iter()
				.map(|s| items::state_back(target, s))
				.collect(),
		),
		D::default(id) if id > limit => match (target, id) {
			(Version::V1_8, 42) => D::default(24),
			(Version::V1_8, 43) => D::default(3),
			(Version::V1_8, 44) => D::default(34),
			(Version::V1_8, 45) => D::default(1),
			_ => return None,
		},
		d => d,
	};
	Some(p)
}

pub fn legacy_down_world_particles_1_8(
	p: v1_9::play_toClient_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<Vec<C8>> {
	Ok(particles_down(Version::V1_8, p)
		.map(C8::world_particles)
		.into_iter()
		.collect())
}

fn particles_before_1_10(
	p: v1_10::play_toClient_packet_world_particles::Ty,
) -> Option<v1_8::play_toClient_packet_world_particles::Ty> {
	use v1_10::play_toClient_packet_world_particles__data::Ty as D10;
	use v1_8::play_toClient_packet_world_particles__data::Ty as D8;
	let data = match p.data {
		D10::v36(d) => D8::v36(d),
		D10::v37(d) => D8::v37(d),
		D10::v38(d) => D8::v38(d),
		D10::v46(_) => return None,
		D10::default(id) => D8::default(id),
	};
	Some(v1_8::play_toClient_packet_world_particles::Ty {
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
	})
}

pub fn legacy_down_world_particles_1_9_4(
	p: v1_10::play_toClient_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<Vec<C94>> {
	Ok(particles_before_1_10(p)
		.and_then(|p| particles_down(Version::V1_9_4, p))
		.map(C94::world_particles)
		.into_iter()
		.collect())
}

pub fn legacy_down_world_particles_1_10(
	mut p: v1_11::play_toClient_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<Vec<C10>> {
	use v1_10::play_toClient_packet_world_particles__data::Ty as D;
	p.data = match p.data {
		D::v36(d) => D::v36(items::item_pair_back(Version::V1_10, &d)),
		D::v37(d) => D::v37(
			d.into_iter()
				.map(|s| items::state_back(Version::V1_10, s))
				.collect(),
		),
		D::v38(d) => D::v38(
			d.into_iter()
				.map(|s| items::state_back(Version::V1_10, s))
				.collect(),
		),
		D::v46(d) => D::v46(
			d.into_iter()
				.map(|s| items::state_back(Version::V1_10, s))
				.collect(),
		),
		D::default(id) if id > 46 => return Ok(vec![]),
		d => d,
	};
	Ok(vec![C10::world_particles(p)])
}

const EFFECTS: [(i32, i32); 37] = [
	(1003, 1002),
	(1005, 1003),
	(1006, 1003),
	(1007, 1003),
	(1008, 1003),
	(1009, 1004),
	(1010, 1005),
	(1011, 1006),
	(1012, 1006),
	(1013, 1006),
	(1014, 1006),
	(1015, 1007),
	(1016, 1008),
	(1017, 1008),
	(1018, 1009),
	(1019, 1010),
	(1020, 1011),
	(1021, 1012),
	(1022, 1012),
	(1023, 1013),
	(1024, 1014),
	(1025, 1015),
	(1026, 1016),
	(1027, 1017),
	(1028, 1018),
	(1029, 1020),
	(1030, 1021),
	(1031, 1022),
	(1032, -1),
	(1033, -1),
	(1034, -1),
	(1035, -1),
	(1036, 1003),
	(1037, 1006),
	(3000, -1),
	(3001, -1),
	(2007, 2002),
];

pub fn legacy_down_world_event_1_8(
	mut p: v1_9::play_toClient_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<Vec<C8>> {
	p.effectId = EFFECTS
		.iter()
		.find(|(new, _)| *new == p.effectId)
		.map_or(p.effectId, |(_, old)| *old);
	match p.effectId {
		-1 => return Ok(vec![]),
		2001 => p.data = items::state_back(Version::V1_8, p.data),
		_ => {}
	}
	Ok(vec![C8::world_event(p)])
}

fn block_change(
	target: Version,
	mut p: v1_8::play_toClient_packet_block_change::Ty,
) -> v1_8::play_toClient_packet_block_change::Ty {
	p.r#type = items::packed_back(target, p.r#type);
	p
}

fn multi_block_change(
	target: Version,
	mut p: v1_8::play_toClient_packet_multi_block_change::Ty,
) -> v1_8::play_toClient_packet_multi_block_change::Ty {
	for r in &mut p.records {
		r.blockId = items::packed_back(target, r.blockId);
	}
	p
}

pub fn legacy_down_block_change_1_8(
	p: v1_9::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<C8>> {
	Ok(vec![C8::block_change(block_change(Version::V1_8, p))])
}

pub fn legacy_down_multi_block_change_1_8(
	p: v1_9::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<C8>> {
	Ok(vec![C8::multi_block_change(multi_block_change(
		Version::V1_8,
		p,
	))])
}

pub fn legacy_down_block_change_1_9_4(
	p: v1_10::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<C94>> {
	Ok(vec![C94::block_change(block_change(Version::V1_9_4, p))])
}

pub fn legacy_down_multi_block_change_1_9_4(
	p: v1_10::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<C94>> {
	Ok(vec![C94::multi_block_change(multi_block_change(
		Version::V1_9_4,
		p,
	))])
}

pub fn legacy_down_block_change_1_10(
	p: v1_11::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<C10>> {
	Ok(vec![C10::block_change(block_change(Version::V1_10, p))])
}

pub fn legacy_down_multi_block_change_1_10(
	p: v1_11::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<C10>> {
	Ok(vec![C10::multi_block_change(multi_block_change(
		Version::V1_10,
		p,
	))])
}

pub fn legacy_down_block_change_1_11(
	p: v1_12::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<C11>> {
	Ok(vec![C11::block_change(block_change(Version::V1_11, p))])
}

pub fn legacy_down_multi_block_change_1_11(
	p: v1_12::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<C11>> {
	Ok(vec![C11::multi_block_change(multi_block_change(
		Version::V1_11,
		p,
	))])
}

pub fn legacy_down_block_action_1_10(
	mut p: v1_11::play_toClient_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<Vec<C10>> {
	if (219..=234).contains(&p.blockId) {
		p.blockId = 130;
	}
	Ok(vec![C10::block_action(p)])
}

pub fn legacy_down_open_window_1_10(
	mut p: v1_11::play_toClient_packet_open_window::Ty,
	_cx: &mut Session,
) -> Result<Vec<C10>> {
	use v1_8::play_toClient_packet_open_window__entityId::Ty as E;
	if p.inventoryType == "minecraft:shulker_box" {
		p.inventoryType = "minecraft:chest".to_owned();
		p.entityId = E::default(p.inventoryType.clone());
	}
	Ok(vec![C10::open_window(p)])
}

pub fn legacy_down_tile_entity_data_1_9_2(
	p: v1_9_4::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<Vec<C92>> {
	use nbt::{CompoundExt, Entry, Payload};
	if p.action != 9 {
		return Ok(vec![C92::tile_entity_data(p)]);
	}
	let Some(Entry {
		value: Payload::compound(c),
		..
	}) = &p.nbtData
	else {
		return Ok(vec![]);
	};
	let line = |k: &str| match c.field(k) {
		Some(Payload::string(s)) => s.clone(),
		_ => "\"\"".to_owned(),
	};
	Ok(vec![C92::update_sign(
		v1_9_2::play_toClient_packet_update_sign::Ty {
			location: p.location.clone(),
			text1: line("Text1"),
			text2: line("Text2"),
			text3: line("Text3"),
			text4: line("Text4"),
		},
	)])
}

pub fn legacy_down_spawn_entity_living_1_9_4(
	mut p: v1_10::play_toClient_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<C94>> {
	let kind = Kind::Mob(p.r#type);
	state(cx).kinds.insert((Version::V1_10, p.entityId), kind);
	meta::fix_1_10(kind, &mut p.metadata);
	slots_down(Version::V1_9_4, Version::V1_10, &mut p.metadata);
	p.metadata = crate::custom::metadata_1_9_4(p.metadata, cx)?;
	if p.r#type == 102 {
		p.r#type = 91;
	}
	Ok(vec![C94::spawn_entity_living(p)])
}

pub fn legacy_down_entity_metadata_1_9_4(
	mut p: v1_10::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<C94>> {
	let kind = kind(cx, Version::V1_10, p.entityId);
	meta::fix_1_10(kind, &mut p.metadata);
	slots_down(Version::V1_9_4, Version::V1_10, &mut p.metadata);
	p.metadata = crate::custom::metadata_1_9_4(p.metadata, cx)?;
	Ok(vec![C94::entity_metadata(p)])
}

fn ensure_metadata(metadata: &mut Vec<meta::E9>) {
	if metadata.is_empty() {
		metadata.push(meta::e9(0, meta::M9::v0(0)));
	}
}

pub fn legacy_down_spawn_entity_1_10(
	mut p: v1_11::play_toClient_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<Vec<C10>> {
	match p.r#type {
		79 => return Ok(vec![]),
		68 => p.r#type = 61,
		_ => {}
	}
	Ok(vec![C10::spawn_entity(p)])
}

pub fn legacy_down_spawn_entity_living_1_10(
	p: v1_11::play_toClient_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<C10>> {
	let ty = p.r#type as u32;
	state(cx)
		.kinds
		.insert((Version::V1_11, p.entityId), Kind::Mob(ty as u8));
	let (new, extra) = meta::mob_1_10(ty);
	let horse = state(cx).horses.entry(p.entityId).or_default();
	let mut metadata = meta::fix_1_11(ty, p.metadata, horse);
	if let Some((key, value)) = extra {
		if !(key == 13 && matches!(ty, 23 | 27) && metadata.iter().any(|e| e.key == 13)) {
			metadata.push(meta::e9(key, meta::M9::v1(value)));
		}
	}
	slots_down(Version::V1_10, Version::V1_11, &mut metadata);
	ensure_metadata(&mut metadata);
	Ok(vec![C10::spawn_entity_living(
		v1_10::play_toClient_packet_spawn_entity_living::Ty {
			entityId: p.entityId,
			entityUUID: p.entityUUID,
			r#type: new as u8,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			headPitch: p.headPitch,
			velocity: p.velocity,
			metadata,
		},
	)])
}

pub fn legacy_down_entity_metadata_1_10(
	mut p: v1_11::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<C10>> {
	let ty = match kind(cx, Version::V1_11, p.entityId) {
		Kind::Mob(t) => t as u32,
		_ => 0,
	};
	let horse = state(cx).horses.entry(p.entityId).or_default();
	p.metadata = meta::fix_1_11(ty, p.metadata, horse);
	slots_down(Version::V1_10, Version::V1_11, &mut p.metadata);
	Ok(vec![C10::entity_metadata(p)])
}

pub fn legacy_down_named_entity_spawn_1_10(
	mut p: v1_11::play_toClient_packet_named_entity_spawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<C10>> {
	ensure_metadata(&mut p.metadata);
	Ok(vec![C10::named_entity_spawn(p)])
}

pub fn legacy_down_spawn_entity_living_1_11(
	p: v1_12::play_toClient_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<C11>> {
	let ty = p.r#type as u32;
	state(cx)
		.kinds
		.insert((Version::V1_12, p.entityId), Kind::Mob(ty as u8));
	let (new, extra) = meta::mob_1_11(ty);
	let mut metadata = meta::fix_1_12(Some(ty), p.metadata);
	if let Some((key, value)) = extra {
		metadata.push(meta::e9(key, meta::M9::v0(value as i8)));
	}
	slots_down(Version::V1_11, Version::V1_12, &mut metadata);
	Ok(vec![C11::spawn_entity_living(
		v1_11::play_toClient_packet_spawn_entity_living::Ty {
			entityId: p.entityId,
			entityUUID: p.entityUUID,
			r#type: new as i32,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			headPitch: p.headPitch,
			velocity: p.velocity,
			metadata,
		},
	)])
}

pub fn legacy_down_entity_metadata_1_11(
	p: v1_12::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<C11>> {
	let ty = match kind(cx, Version::V1_12, p.entityId) {
		Kind::Mob(t) => Some(t as u32),
		_ => None,
	};
	let mut metadata = meta::fix_1_12(ty, p.metadata);
	slots_down(Version::V1_11, Version::V1_12, &mut metadata);
	Ok(vec![C11::entity_metadata(
		v1_11::play_toClient_packet_entity_metadata::Ty {
			entityId: p.entityId,
			metadata,
		},
	)])
}

pub fn legacy_down_chat_1_11(
	mut p: v1_12::play_toClient_packet_chat::Ty,
	_cx: &mut Session,
) -> Result<Vec<C11>> {
	p.message = super::v1_7_to_1_8::text::without_keybinds(&p.message);
	Ok(vec![C11::chat(p)])
}

pub fn legacy_down_title_1_11(
	p: v1_12::play_toClient_packet_title::Ty,
	_cx: &mut Session,
) -> Result<Vec<C11>> {
	use super::v1_7_to_1_8::text::without_keybinds;
	use v1_12::play_toClient_packet_title::Ty as T;
	Ok(vec![C11::title(match p {
		T::v0(s) => T::v0(without_keybinds(&s)),
		T::v1(s) => T::v1(without_keybinds(&s)),
		T::v2(s) => T::v2(without_keybinds(&s)),
		t => t,
	})])
}

pub fn legacy_down_keep_alive_1_12_1(id: i64, cx: &mut Session) -> Result<Vec<C121>> {
	let st = state(cx);
	let short = id as i32;
	if st.keep_alive.len() >= 32 {
		st.keep_alive.pop_front();
	}
	st.keep_alive.push_back((short, id));
	Ok(vec![C121::keep_alive(short)])
}

pub fn legacy_up_keep_alive_1_12_2(id: i32, cx: &mut Session) -> Result<Vec<S122>> {
	let st = state(cx);
	let full = match st.keep_alive.iter().position(|(s, _)| *s == id) {
		Some(at) => st.keep_alive.remove(at).map_or(id as i64, |(_, f)| f),
		None => id as i64,
	};
	Ok(vec![S122::keep_alive(full)])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn relative_moves_keep_sub_pixel_precision() {
		let mut cx = Session::new();
		tracked(&mut cx, 1, 0.0, 0.0, 0.0);
		let mut sum = [0i32; 3];
		for _ in 0..1000 {
			for m in split_move(&mut cx, 1, [100, -100, 4000]) {
				for i in 0..3 {
					sum[i] += m[i] as i32;
				}
			}
		}
		assert_eq!(
			sum,
			[
				(100_000f64 / 128.0).floor() as i32,
				(-100_000f64 / 128.0).floor() as i32,
				4_000_000 / 128
			]
		);
		let big = split_move(&mut cx, 2, [32000, 0, -32000]);
		assert_eq!(big.len(), 2);
		assert_eq!(big.iter().map(|m| m[0] as i32).sum::<i32>(), 250);
	}
}
