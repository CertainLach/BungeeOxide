use ::nbt::{CompoundExt, CompoundVecExt, Entry, Payload as P};

use super::chunk::{self, F1_16_2, F1_17};
use super::meta::{self, Meta, Pos, Slot};
use super::step_1_16_2::section_y;
use super::tag_data::NEW_TAGS_1_17;
use super::tall::{self, Painting as TrackedPainting, Tall};
use super::*;
use crate::custom::chunks::{BlockEntity, Column};
use crate::custom::window::{Emit, Teleport as Move};
use crate::gen::convert::{v1_16_2 as old, v1_17 as new};
use crate::gen::convert::{Carrier, Root};
use crate::pds::convert::{Error, Result};
use crate::pds::Output;
use crate::session::Session;

const FROM: Version = Version::V1_16_2;
const TO: Version = Version::V1_17;

type OldRoot = old::play_toClient_packet::Ty;
type NewRoot = new::play_toClient_packet::Ty;

fn slot_up(v: Slot) -> Slot {
	meta::slot(v, FROM, TO, |_, _| {})
}

fn slot_down(v: Slot) -> Slot {
	meta::slot(v, TO, FROM, |_, _| {})
}

fn view_base(cx: &mut Session) -> i32 {
	cx.state::<Tall>().view()
}

fn input_base(cx: &mut Session) -> i32 {
	cx.state::<Tall>().input()
}

fn shifted(mut p: Pos, dy: i32) -> Pos {
	p.y = (p.y as i32 + dy) as i16;
	p
}

fn tall_state(cx: &mut Session) -> &mut Tall {
	cx.state::<Tall>()
}

pub fn nether_slot_1_17(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot_up(v))
}

pub fn nether_slot_1_16_2(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot_down(v))
}

pub fn nether_position_1_17(v: Pos, cx: &mut Session) -> Result<Pos> {
	Ok(shifted(v, input_base(cx)))
}

pub fn nether_position_1_16_2(v: Pos, cx: &mut Session) -> Result<Pos> {
	Ok(shifted(v, -view_base(cx)))
}

fn dimension_types_mut(codec: &mut Entry) -> &mut [Vec<Entry>] {
	match codec
		.value
		.as_compound_mut()
		.and_then(|c| c.compound_mut("minecraft:dimension_type"))
		.and_then(|c| c.compounds_mut("value"))
	{
		Some(v) => v,
		None => &mut [],
	}
}

fn add_height(e: &mut [Entry]) -> Vec<Entry> {
	let mut c = e.to_vec();
	if c.field("min_y").is_none() {
		c.put("min_y", 0);
	}
	if c.field("height").is_none() {
		c.put("height", 256);
	}
	c
}

fn strip_height(c: &mut Vec<Entry>) -> (i32, i32) {
	let min_y = c.take("min_y").and_then(|p| p.as_i64()).unwrap_or(0) as i32;
	let height = c.take("height").and_then(|p| p.as_i64()).unwrap_or(256) as i32;
	if let Some(mut h) = c.field_mut("logical_height") {
		if let P::int(h) = &mut *h {
			*h = (*h).min(256);
		}
	}
	(min_y, height)
}

fn codec_up(mut codec: Entry) -> Entry {
	for t in dimension_types_mut(&mut codec) {
		if let Some(e) = t.compound_mut("element") {
			*e = add_height(e);
		}
	}
	codec
}

fn codec_down(mut codec: Entry) -> Entry {
	for t in dimension_types_mut(&mut codec) {
		if let Some(e) = t.compound_mut("element") {
			strip_height(e);
		}
	}
	codec
}

fn dimension_up(mut d: Entry) -> Entry {
	if let Some(c) = d.value.as_compound_mut() {
		*c = add_height(c);
	}
	d
}

fn dimension_down(mut d: Entry, cx: &mut Session) -> Entry {
	let (min_y, height) = d.value.as_compound_mut().map_or((0, 256), strip_height);
	let window = tall::window_for(cx);
	tall_state(cx).reset(min_y, height, window);
	d
}

pub fn nether_login_1_17(
	v: old::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<new::play_toClient_packet_login::Ty> {
	player(cx, TO, v.entityId);
	Ok(new::play_toClient_packet_login::Ty {
		entityId: v.entityId,
		isHardcore: v.isHardcore,
		gameMode: v.gameMode,
		previousGameMode: v.previousGameMode as i8,
		worldNames: v.worldNames,
		dimensionCodec: codec_up(v.dimensionCodec),
		dimension: dimension_up(v.dimension),
		worldName: v.worldName,
		hashedSeed: v.hashedSeed,
		maxPlayers: v.maxPlayers,
		viewDistance: v.viewDistance,
		reducedDebugInfo: v.reducedDebugInfo,
		enableRespawnScreen: v.enableRespawnScreen,
		isDebug: v.isDebug,
		isFlat: v.isFlat,
	})
}

pub fn nether_login_1_16_2(
	v: new::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<old::play_toClient_packet_login::Ty> {
	player(cx, TO, v.entityId);
	Ok(old::play_toClient_packet_login::Ty {
		entityId: v.entityId,
		isHardcore: v.isHardcore,
		gameMode: v.gameMode,
		previousGameMode: v.previousGameMode as u8,
		worldNames: v.worldNames,
		dimensionCodec: codec_down(v.dimensionCodec),
		dimension: dimension_down(v.dimension, cx),
		worldName: v.worldName,
		hashedSeed: v.hashedSeed,
		maxPlayers: v.maxPlayers,
		viewDistance: v.viewDistance,
		reducedDebugInfo: v.reducedDebugInfo,
		enableRespawnScreen: v.enableRespawnScreen,
		isDebug: v.isDebug,
		isFlat: v.isFlat,
	})
}

type Respawn = old::play_toClient_packet_respawn::Ty;

pub fn nether_respawn_1_17(mut v: Respawn, _cx: &mut Session) -> Result<Vec<NewRoot>> {
	v.dimension = dimension_up(v.dimension);
	Ok(vec![NewRoot::respawn(v)])
}

pub fn nether_respawn_1_16_2(mut v: Respawn, cx: &mut Session) -> Result<Vec<OldRoot>> {
	let mut out: Vec<OldRoot> = tall_state(cx)
		.forget_columns()
		.into_iter()
		.map(|(chunkX, chunkZ)| {
			OldRoot::unload_chunk(old::play_toClient_packet_unload_chunk::Ty { chunkX, chunkZ })
		})
		.collect();
	v.dimension = dimension_down(v.dimension, cx);
	out.push(OldRoot::respawn(v));
	Ok(out)
}

fn mask_longs(mask: i32) -> Vec<i64> {
	if mask == 0 {
		vec![]
	} else {
		vec![mask as u32 as i64]
	}
}

pub fn nether_section_change_1_17(x: i32, z: i32, y: u32, states: &[u32]) -> NewRoot {
	let records = states
		.iter()
		.enumerate()
		.map(|(i, state)| {
			super::step_1_16_2::record(
				*state as i32,
				i as i32 & 15,
				i as i32 >> 8,
				(i as i32 >> 4) & 15,
			)
		})
		.collect();
	NewRoot::multi_block_change(new::play_toClient_packet_multi_block_change::Ty {
		chunkCoordinates: new::play_toClient_packet_multi_block_change__chunkCoordinates::Ty {
			x,
			z,
			y,
		},
		notTrustEdges: true,
		records,
	})
}

pub fn nether_map_chunk_1_17(
	p: old::play_toClient_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<NewRoot>> {
	let present = (p.bitMap as u32).count_ones() as usize;
	let old::play_toClient_packet_map_chunk__biomes::Ty::r#true(biomes) = p.biomes else {
		let (sections, _) = chunk::read_sections(&p.chunkData, present)?;
		let ys = (0..16).filter(|i| p.bitMap & (1 << i) != 0);
		return Ok(ys
			.zip(sections)
			.map(|(y, s)| {
				nether_section_change_1_17(
					p.x,
					p.z,
					y as u32,
					&s.convert(F1_16_2, F1_17).states(false),
				)
			})
			.collect());
	};
	Ok(vec![NewRoot::map_chunk(
		new::play_toClient_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			bitMap: mask_longs(p.bitMap),
			heightmaps: p.heightmaps,
			biomes,
			chunkData: chunk::convert_sections(&p.chunkData, present, F1_16_2, F1_17)?,
			blockEntities: p.blockEntities,
		},
	)])
}

pub fn nether_map_chunk_1_16_2(
	p: new::play_toClient_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<OldRoot>> {
	tall::wire_chunk(p, tall_state(cx))
}

pub fn nether_update_light_1_17(
	v: old::play_toClient_packet_update_light::Ty,
	_cx: &mut Session,
) -> Result<new::play_toClient_packet_update_light::Ty> {
	let mut r = chunk::Reader::new(&v.data);
	let sky = chunk::read_light(&mut r, v.skyLightMask as u32 as u64)?;
	let block = chunk::read_light(&mut r, v.blockLightMask as u32 as u64)?;
	Ok(new::play_toClient_packet_update_light::Ty {
		chunkX: v.chunkX,
		chunkZ: v.chunkZ,
		trustEdges: v.trustEdges,
		skyLightMask: mask_longs(v.skyLightMask),
		blockLightMask: mask_longs(v.blockLightMask),
		emptySkyLightMask: mask_longs(v.emptySkyLightMask),
		emptyBlockLightMask: mask_longs(v.emptyBlockLightMask),
		skyLight: sky.into_iter().map(|(_, a)| a).collect(),
		blockLight: block.into_iter().map(|(_, a)| a).collect(),
	})
}

pub fn nether_update_light_1_16_2(
	v: new::play_toClient_packet_update_light::Ty,
	cx: &mut Session,
) -> Result<old::play_toClient_packet_update_light::Ty> {
	tall::wire_light(v, tall_state(cx))
}

pub fn nether_unload_chunk_1_17(
	p: old::play_toClient_packet_unload_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<NewRoot>> {
	Ok(vec![NewRoot::unload_chunk(p)])
}

pub fn nether_unload_chunk_1_16_2(
	p: old::play_toClient_packet_unload_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<OldRoot>> {
	tall_state(cx).unload(p.chunkX, p.chunkZ);
	Ok(vec![OldRoot::unload_chunk(p)])
}

pub fn nether_player_position_1_17(
	p: old::play_toClient_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<NewRoot>> {
	let dy = if p.flags & 2 == 0 {
		input_base(cx) as f64
	} else {
		0.0
	};
	Ok(vec![NewRoot::position(
		new::play_toClient_packet_position::Ty {
			x: p.x,
			y: p.y + dy,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			flags: p.flags,
			teleportId: p.teleportId,
			dismountVehicle: false,
		},
	)])
}

pub fn nether_player_position_1_16_2(
	p: new::play_toClient_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<OldRoot>> {
	let t = tall_state(cx);
	let emits = t.server_teleport(teleport(&p));
	tall::typed_emits(emits, t.view())
}

fn teleport(p: &new::play_toClient_packet_position::Ty) -> Move {
	Move {
		x: p.x,
		y: p.y,
		z: p.z,
		yaw: p.yaw,
		pitch: p.pitch,
		flags: p.flags,
		id: p.teleportId,
	}
}

pub fn nether_column_1_16_2(col: Column, cx: &mut Session) -> Result<Vec<Carrier>> {
	tall::native(col, tall_state(cx))
}

fn position_id() -> i32 {
	static ID: std::sync::OnceLock<i32> = std::sync::OnceLock::new();
	*ID.get_or_init(|| {
		let p = new::play_toClient_packet_position::Ty {
			x: 0.0,
			y: 0.0,
			z: 0.0,
			yaw: 0.0,
			pitch: 0.0,
			flags: 0,
			teleportId: 0,
			dismountVehicle: false,
		};
		let mut o = Output::new();
		match new::play_toClient_packet::serialize(&NewRoot::position(p), &mut o) {
			Ok(()) => chunk::Reader::new(&o.into_inner()).varint().unwrap_or(-1),
			Err(_) => -1,
		}
	})
}

pub fn nether_wire_1_16_2(data: &[u8], cx: &mut Session) -> Result<Option<Vec<Carrier>>> {
	if !tall_state(cx).tall() || chunk::Reader::new(data).varint().ok() != Some(position_id()) {
		return Ok(None);
	}
	let Ok(NewRoot::position(p)) =
		new::play_toClient_packet::deserialize(&mut crate::pds::Input::new(data))
	else {
		return Ok(None);
	};
	let t = tall_state(cx);
	let emits = t.server_teleport(teleport(&p));
	Ok(Some(tall::carriers(emits, t.view())?))
}

type ServerPosition = old::play_toServer_packet_position::Ty;
type ServerPositionLook = old::play_toServer_packet_position_look::Ty;
type ServerVehicle = old::play_toServer_packet_vehicle_move::Ty;

fn emit(cx: &mut Session, emits: Vec<Emit<TrackedPainting>>) -> Result<()> {
	let view = view_base(cx);
	tall::send(cx, emits, view)
}

const BACKLOG: usize = 2;

fn drain(cx: &mut Session) -> Result<()> {
	let out = tall_state(cx).backlog(BACKLOG);
	emit(cx, out)
}

fn moved(x: f64, y: f64, z: f64, vehicle: Option<(f32, f32)>, cx: &mut Session) -> Result<f64> {
	let (y, out) = tall_state(cx).client_moved(x, y, z, vehicle);
	let switched = !out.is_empty();
	emit(cx, out)?;
	if !switched {
		drain(cx)?;
	}
	Ok(y)
}

pub fn nether_server_position_1_17(
	mut v: ServerPosition,
	cx: &mut Session,
) -> Result<ServerPosition> {
	v.y = moved(v.x, v.y, v.z, None, cx)?;
	Ok(v)
}

pub fn nether_position_look_1_17(
	mut v: ServerPositionLook,
	cx: &mut Session,
) -> Result<ServerPositionLook> {
	v.y = moved(v.x, v.y, v.z, None, cx)?;
	Ok(v)
}

pub fn nether_vehicle_move_1_17(mut v: ServerVehicle, cx: &mut Session) -> Result<ServerVehicle> {
	v.y = moved(v.x, v.y, v.z, Some((v.yaw, v.pitch)), cx)?;
	Ok(v)
}

pub fn nether_teleport_confirm_1_17(
	id: i32,
	cx: &mut Session,
) -> Result<Vec<new::play_toServer_packet::Ty>> {
	if tall_state(cx).confirmed(id) {
		return Ok(vec![]);
	}
	Ok(vec![new::play_toServer_packet::Ty::teleport_confirm(id)])
}

pub fn nether_teleport_confirm_1_16_2(
	id: i32,
	_cx: &mut Session,
) -> Result<Vec<old::play_toServer_packet::Ty>> {
	Ok(vec![old::play_toServer_packet::Ty::teleport_confirm(id)])
}

pub fn nether_spawn_entity_1_17(p: SpawnObject, cx: &mut Session) -> Result<Vec<NewRoot>> {
	let Ok(p) = spawn_object(p, FROM, TO, cx) else {
		return Ok(vec![]);
	};
	let frame = entity_name(TO, p.r#type) == Some("item_frame");
	let look = new::play_toClient_packet_entity_look::Ty {
		entityId: p.entityId,
		yaw: p.yaw,
		pitch: p.pitch,
		onGround: false,
	};
	let mut out = vec![NewRoot::spawn_entity(p)];
	if frame {
		out.push(NewRoot::entity_look(look));
	}
	Ok(out)
}

pub fn nether_spawn_entity_1_16_2(p: SpawnObject, cx: &mut Session) -> Result<Vec<OldRoot>> {
	let Ok(mut p) = spawn_object(p, TO, FROM, cx) else {
		return Ok(vec![]);
	};
	let t = tall_state(cx);
	t.moved(p.entityId, p.x, p.y, p.z, Some((p.yaw, p.pitch)), None);
	p.y -= t.view() as f64;
	Ok(vec![OldRoot::spawn_entity(p)])
}

pub fn nether_spawn_entity_living_1_17(p: SpawnMob, cx: &mut Session) -> Result<SpawnMob> {
	spawn_mob(p, FROM, TO, cx)
}

pub fn nether_spawn_entity_living_1_16_2(p: SpawnMob, cx: &mut Session) -> Result<SpawnMob> {
	let mut p = spawn_mob(p, TO, FROM, cx)?;
	let t = tall_state(cx);
	t.moved(p.entityId, p.x, p.y, p.z, Some((p.yaw, p.pitch)), None);
	p.y -= t.view() as f64;
	Ok(p)
}

type Orb = old::play_toClient_packet_spawn_entity_experience_orb::Ty;

pub fn nether_spawn_entity_experience_orb_1_16_2(mut p: Orb, cx: &mut Session) -> Result<Orb> {
	let t = tall_state(cx);
	t.moved(p.entityId, p.x, p.y, p.z, None, None);
	p.y -= t.view() as f64;
	Ok(p)
}

type Painting = old::play_toClient_packet_spawn_entity_painting::Ty;

pub fn nether_spawn_entity_painting_1_16_2(p: Painting, cx: &mut Session) -> Result<Painting> {
	let t = tall_state(cx);
	t.painting(p.entityId, p.clone());
	Ok(tall::shifted_painting(p, t.view()))
}

type PlayerSpawn = old::play_toClient_packet_named_entity_spawn::Ty;

pub fn nether_named_entity_spawn_1_16_2(
	mut p: PlayerSpawn,
	cx: &mut Session,
) -> Result<PlayerSpawn> {
	player(cx, TO, p.entityId);
	let t = tall_state(cx);
	t.moved(p.entityId, p.x, p.y, p.z, Some((p.yaw, p.pitch)), None);
	p.y -= t.view() as f64;
	Ok(p)
}

type Teleport = old::play_toClient_packet_entity_teleport::Ty;

pub fn nether_entity_teleport_1_16_2(mut p: Teleport, cx: &mut Session) -> Result<Teleport> {
	let t = tall_state(cx);
	t.moved(
		p.entityId,
		p.x,
		p.y,
		p.z,
		Some((p.yaw, p.pitch)),
		Some(p.onGround),
	);
	p.y -= t.view() as f64;
	drain(cx)?;
	Ok(p)
}

type RelMove = old::play_toClient_packet_rel_entity_move::Ty;
type MoveLook = old::play_toClient_packet_entity_move_look::Ty;

fn delta(x: i16, y: i16, z: i16) -> (f64, f64, f64) {
	(x as f64 / 4096.0, y as f64 / 4096.0, z as f64 / 4096.0)
}

pub fn nether_rel_entity_move_1_16_2(p: RelMove, cx: &mut Session) -> Result<RelMove> {
	tall_state(cx).moved_by(p.entityId, delta(p.dX, p.dY, p.dZ), None, p.onGround);
	drain(cx)?;
	Ok(p)
}

pub fn nether_entity_move_look_1_16_2(p: MoveLook, cx: &mut Session) -> Result<MoveLook> {
	tall_state(cx).moved_by(
		p.entityId,
		delta(p.dX, p.dY, p.dZ),
		Some((p.yaw, p.pitch)),
		p.onGround,
	);
	drain(cx)?;
	Ok(p)
}

type Vehicle = old::play_toClient_packet_vehicle_move::Ty;

pub fn nether_vehicle_move_1_16_2(mut p: Vehicle, cx: &mut Session) -> Result<Vehicle> {
	p.y = tall_state(cx).vehicle_moved(p.x, p.y, p.z);
	Ok(p)
}

type Look = old::play_toClient_packet_entity_look::Ty;

pub fn nether_entity_look_1_16_2(p: Look, cx: &mut Session) -> Result<Look> {
	tall_state(cx).looked(p.entityId, (p.yaw, p.pitch), p.onGround);
	drain(cx)?;
	Ok(p)
}

pub fn nether_entity_destroy_1_17(p: Vec<i32>, cx: &mut Session) -> Result<Vec<NewRoot>> {
	untrack(cx, &p);
	let t = tall_state(cx);
	for id in &p {
		t.forget(*id);
	}
	Ok(p.into_iter().map(NewRoot::destroy_entity).collect())
}

pub fn nether_destroy_entity_1_16_2(id: i32, cx: &mut Session) -> Result<Vec<OldRoot>> {
	untrack(cx, &[id]);
	tall_state(cx).forget(id);
	Ok(vec![OldRoot::entity_destroy(vec![id])])
}

pub fn nether_world_particles_1_17(
	v: old::play_toClient_packet_world_particles::Ty,
	cx: &mut Session,
) -> Result<new::play_toClient_packet_world_particles::Ty> {
	let p = kept(meta::particle(
		meta::particle_16(v.particleId, v.data),
		FROM,
		TO,
		slot_up,
	))?;
	Ok(new::play_toClient_packet_world_particles::Ty {
		particleId: p.id,
		data: kept(meta::data_17(p))?,
		longDistance: v.longDistance,
		x: v.x,
		y: v.y + input_base(cx) as f64,
		z: v.z,
		offsetX: v.offsetX,
		offsetY: v.offsetY,
		offsetZ: v.offsetZ,
		particleData: v.particleData,
		particles: v.particles,
	})
}

pub fn nether_world_particles_1_16_2(
	v: new::play_toClient_packet_world_particles::Ty,
	cx: &mut Session,
) -> Result<old::play_toClient_packet_world_particles::Ty> {
	let p = kept(meta::particle(
		meta::particle_17(v.particleId, v.data),
		TO,
		FROM,
		slot_down,
	))?;
	Ok(old::play_toClient_packet_world_particles::Ty {
		particleId: p.id,
		data: kept(meta::data_16(p))?,
		longDistance: v.longDistance,
		x: v.x,
		y: v.y - view_base(cx) as f64,
		z: v.z,
		offsetX: v.offsetX,
		offsetY: v.offsetY,
		offsetZ: v.offsetZ,
		particleData: v.particleData,
		particles: v.particles,
	})
}

type NamedSound = old::play_toClient_packet_named_sound_effect::Ty;

pub fn nether_named_sound_effect_1_16_2(mut v: NamedSound, cx: &mut Session) -> Result<NamedSound> {
	v.y -= view_base(cx) * 8;
	Ok(v)
}

pub fn nether_sound_effect_1_17(v: Sound, cx: &mut Session) -> Result<Sound> {
	let mut v = sound_effect(v, FROM, TO)?;
	v.y += input_base(cx) * 8;
	Ok(v)
}

pub fn nether_sound_effect_1_16_2(v: Sound, cx: &mut Session) -> Result<Sound> {
	let mut v = sound_effect(v, TO, FROM)?;
	v.y -= view_base(cx) * 8;
	Ok(v)
}

pub fn nether_entity_sound_effect_1_17(v: EntitySound, _cx: &mut Session) -> Result<EntitySound> {
	entity_sound(v, FROM, TO)
}

pub fn nether_entity_sound_effect_1_16_2(v: EntitySound, _cx: &mut Session) -> Result<EntitySound> {
	entity_sound(v, TO, FROM)
}

pub fn nether_explosion_1_17(
	v: old::play_toClient_packet_explosion::Ty,
	cx: &mut Session,
) -> Result<new::play_toClient_packet_explosion::Ty> {
	Ok(new::play_toClient_packet_explosion::Ty {
		x: v.x,
		y: v.y + input_base(cx) as f32,
		z: v.z,
		radius: v.radius,
		affectedBlockOffsets: v
			.affectedBlockOffsets
			.into_iter()
			.map(
				|o| new::play_toClient_packet_explosion__affectedBlockOffsets__items::Ty {
					x: o.x,
					y: o.y,
					z: o.z,
				},
			)
			.collect(),
		playerMotionX: v.playerMotionX,
		playerMotionY: v.playerMotionY,
		playerMotionZ: v.playerMotionZ,
	})
}

pub fn nether_explosion_1_16_2(
	v: new::play_toClient_packet_explosion::Ty,
	cx: &mut Session,
) -> Result<old::play_toClient_packet_explosion::Ty> {
	Ok(old::play_toClient_packet_explosion::Ty {
		x: v.x,
		y: v.y - view_base(cx) as f32,
		z: v.z,
		radius: v.radius,
		affectedBlockOffsets: v
			.affectedBlockOffsets
			.into_iter()
			.map(
				|o| old::play_toClient_packet_explosion__affectedBlockOffsets__items::Ty {
					x: o.x,
					y: o.y,
					z: o.z,
				},
			)
			.collect(),
		playerMotionX: v.playerMotionX,
		playerMotionY: v.playerMotionY,
		playerMotionZ: v.playerMotionZ,
	})
}

type Face = old::play_toClient_packet_face_player::Ty;

pub fn nether_face_player_1_16_2(mut v: Face, cx: &mut Session) -> Result<Face> {
	v.y -= view_base(cx) as f64;
	Ok(v)
}

type TileEntity = old::play_toClient_packet_tile_entity_data::Ty;

pub fn nether_tile_entity_data_1_16_2(mut v: TileEntity, cx: &mut Session) -> Result<TileEntity> {
	let (x, y, z) = (v.location.x, v.location.y as i32, v.location.z);
	let entity = v.nbtData.map(|e| BlockEntity {
		x,
		y,
		z,
		kind: None,
		name: e.name,
		nbt: Some(::nbt::Tag::new(e.value)),
	});
	let Some((y, entity)) = tall_state(cx).block_entity(x, y, z, entity) else {
		return dropped();
	};
	v.location.y = y as i16;
	v.nbtData = entity.and_then(|e| e.nbt.map(|t| t.named(e.name)));
	Ok(v)
}

pub fn nether_block_change_1_17(v: BlockChange, cx: &mut Session) -> Result<BlockChange> {
	let mut v = block_change(v, FROM, TO);
	v.location = shifted(v.location, input_base(cx));
	Ok(v)
}

pub fn nether_block_change_1_16_2(mut v: BlockChange, cx: &mut Session) -> Result<BlockChange> {
	let (x, y, z) = (v.location.x, v.location.y as i32, v.location.z);
	let Some((y, shown)) = tall_state(cx).block_changed(x, y, z, state(TO, FROM, v.r#type) as u32)
	else {
		return dropped();
	};
	v.location.y = y as i16;
	v.r#type = shown as i32;
	Ok(v)
}

type MultiBlock = old::play_toClient_packet_multi_block_change::Ty;

fn remap_records(records: Vec<i64>, from: Version, to: Version) -> Vec<i64> {
	records
		.into_iter()
		.map(|r| ((state(from, to, (r >> 12) as i32) as i64) << 12) | (r & 0xfff))
		.collect()
}

pub fn nether_multi_block_change_1_17(mut v: MultiBlock, cx: &mut Session) -> Result<MultiBlock> {
	v.records = remap_records(v.records, FROM, TO);
	v.chunkCoordinates.y =
		(section_y(v.chunkCoordinates.y) + (input_base(cx) >> 4)) as u32 & 0xfffff;
	Ok(v)
}

pub fn nether_multi_block_change_1_16_2(mut v: MultiBlock, cx: &mut Session) -> Result<MultiBlock> {
	let (x, y, z) = (
		v.chunkCoordinates.x,
		section_y(v.chunkCoordinates.y),
		v.chunkCoordinates.z,
	);
	let changes: Vec<(i32, i32, i32, u32)> = v
		.records
		.iter()
		.map(|r| {
			(
				x * 16 + ((r >> 8) & 15) as i32,
				y * 16 + (r & 15) as i32,
				z * 16 + ((r >> 4) & 15) as i32,
				state(TO, FROM, (r >> 12) as i32) as u32,
			)
		})
		.collect();
	let shown = tall_state(cx).blocks_changed(&changes);
	let Some(section) = shown.first().map(|c| c.1 >> 4) else {
		return dropped();
	};
	let mut records: Vec<i64> = shown
		.iter()
		.filter(|c| c.1 >> 4 == section)
		.map(|(x, y, z, s)| {
			((*s as i64) << 12) | (((x & 15) << 8) | ((z & 15) << 4) | (y & 15)) as i64
		})
		.collect();
	records.sort_unstable_by_key(|r| r & 0xfff);
	v.chunkCoordinates.y = section as u32;
	v.records = records;
	Ok(v)
}

pub fn nether_acknowledge_player_digging_1_17(v: Digging, cx: &mut Session) -> Result<Digging> {
	let mut v = digging(v, FROM, TO);
	v.location = shifted(v.location, input_base(cx));
	Ok(v)
}

pub fn nether_acknowledge_player_digging_1_16_2(v: Digging, cx: &mut Session) -> Result<Digging> {
	let mut v = digging(v, TO, FROM);
	v.location = shifted(v.location, -view_base(cx));
	Ok(v)
}

pub fn nether_block_action_1_17(v: BlockAction, cx: &mut Session) -> Result<BlockAction> {
	let mut v = block_action(v, FROM, TO)?;
	v.location = shifted(v.location, input_base(cx));
	Ok(v)
}

pub fn nether_block_action_1_16_2(v: BlockAction, cx: &mut Session) -> Result<BlockAction> {
	if !tall_state(cx).inside(v.location.y as i32) {
		return dropped();
	}
	let mut v = block_action(v, TO, FROM)?;
	v.location = shifted(v.location, -view_base(cx));
	Ok(v)
}

pub fn nether_world_event_1_17(v: WorldEvent, cx: &mut Session) -> Result<WorldEvent> {
	let mut v = world_event(v, FROM, TO)?;
	v.location = shifted(v.location, input_base(cx));
	Ok(v)
}

pub fn nether_world_event_1_16_2(v: WorldEvent, cx: &mut Session) -> Result<WorldEvent> {
	let mut v = world_event(v, TO, FROM)?;
	v.location = shifted(v.location, -view_base(cx));
	Ok(v)
}

pub fn nether_set_cooldown_1_17(v: Cooldown, _cx: &mut Session) -> Result<Cooldown> {
	cooldown(v, FROM, TO)
}

pub fn nether_set_cooldown_1_16_2(v: Cooldown, _cx: &mut Session) -> Result<Cooldown> {
	cooldown(v, TO, FROM)
}

pub fn nether_statistics_1_17(p: Vec<Stat>, _cx: &mut Session) -> Result<Vec<NewRoot>> {
	Ok(vec![NewRoot::statistics(statistics(p, FROM, TO))])
}

pub fn nether_statistics_1_16_2(p: Vec<Stat>, _cx: &mut Session) -> Result<Vec<OldRoot>> {
	Ok(vec![OldRoot::statistics(statistics(p, TO, FROM))])
}

type TagGroup = new::play_toClient_packet__tags__items::Ty;

const TAG_TYPES: [(&str, &str); 5] = [
	("minecraft:block", "blocks"),
	("minecraft:item", "items"),
	("minecraft:fluid", "fluids"),
	("minecraft:entity_type", "entities"),
	("minecraft:game_event", "game_events"),
];

pub fn nether_tags_1_17(v: Tags, _cx: &mut Session) -> Result<Vec<TagGroup>> {
	let mut lists = [
		v.blockTags,
		v.itemTags,
		v.fluidTags,
		v.entityTags,
		Vec::new(),
	]
	.into_iter();
	Ok(TAG_TYPES
		.iter()
		.map(|(ty, kind)| TagGroup {
			tagType: (*ty).to_owned(),
			tags: tags(
				lists.next().unwrap_or_default(),
				kind,
				FROM,
				TO,
				NEW_TAGS_1_17,
				&[],
			),
		})
		.collect())
}

pub fn nether_tags_1_16_2(v: Vec<TagGroup>, _cx: &mut Session) -> Result<Tags> {
	let mut lists: Vec<Vec<Tag>> = TAG_TYPES
		.iter()
		.take(4)
		.map(|(ty, kind)| {
			let list = v
				.iter()
				.find(|g| strip(&g.tagType) == strip(ty))
				.map(|g| g.tags.clone())
				.unwrap_or_default();
			tags(list, kind, TO, FROM, &[], &[])
		})
		.collect();
	let entityTags = lists.pop().unwrap_or_default();
	let fluidTags = lists.pop().unwrap_or_default();
	let itemTags = lists.pop().unwrap_or_default();
	let blockTags = lists.pop().unwrap_or_default();
	Ok(Tags {
		blockTags,
		itemTags,
		fluidTags,
		entityTags,
	})
}

fn metadata_kind(cx: &mut Session, entity: i32) -> &'static str {
	kind_of(cx, TO, entity).name
}

fn shift_positions(items: &mut [(u8, Meta)], dy: i32) {
	for (_, m) in items.iter_mut() {
		match m {
			Meta::Pos(p) => *p = shifted(p.clone(), dy),
			Meta::OptPos(Some(p)) => *p = shifted(p.clone(), dy),
			_ => {}
		}
	}
}

pub fn nether_entity_metadata_1_17(
	v: old::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<new::play_toClient_packet_entity_metadata::Ty> {
	let kind = metadata_kind(cx, v.entityId);
	let mut items = meta::remap_values(meta::decode_16(v.metadata), FROM, TO, &mut slot_up);
	for (k, m) in items.iter_mut() {
		if *k >= 7 {
			*k += 1;
		}
		if let Meta::Pose(p) = m {
			if *p > 5 {
				*p += 1;
			}
		}
	}
	if kind == "shulker" {
		items.retain(|(k, _)| *k != 17);
		for (k, _) in items.iter_mut().filter(|(k, _)| *k > 17) {
			*k -= 1;
		}
	}
	meta::minecart_block(&mut items, kind, 11, FROM, TO);
	shift_positions(&mut items, input_base(cx));
	Ok(new::play_toClient_packet_entity_metadata::Ty {
		entityId: v.entityId,
		metadata: meta::encode_17(items),
	})
}

pub fn nether_entity_metadata_1_16_2(
	v: new::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<old::play_toClient_packet_entity_metadata::Ty> {
	drain(cx)?;
	let kind = metadata_kind(cx, v.entityId);
	let mut items = meta::remap_values(meta::decode_17(v.metadata), TO, FROM, &mut slot_down);
	meta::minecart_block(&mut items, kind, 11, TO, FROM);
	items.retain(|(k, _)| *k != 7);
	for (k, m) in items.iter_mut() {
		if *k > 7 {
			*k -= 1;
		}
		if let Meta::Pose(p) = m {
			*p = match *p {
				6 => 0,
				p if p > 6 => p - 1,
				p => p,
			};
		}
	}
	if kind == "shulker" {
		for (k, _) in items.iter_mut().filter(|(k, _)| *k >= 16) {
			*k += 1;
		}
	}
	meta::substituted(&mut items, kind_of(cx, TO, v.entityId));
	shift_positions(&mut items, -view_base(cx));
	Ok(old::play_toClient_packet_entity_metadata::Ty {
		entityId: v.entityId,
		metadata: meta::encode_16(items),
	})
}

pub fn nether_spawn_position_1_17(
	v: Pos,
	cx: &mut Session,
) -> Result<new::play_toClient_packet_spawn_position::Ty> {
	Ok(new::play_toClient_packet_spawn_position::Ty {
		location: shifted(v, input_base(cx)),
		angle: 0.0,
	})
}

pub fn nether_spawn_position_1_16_2(
	v: new::play_toClient_packet_spawn_position::Ty,
	cx: &mut Session,
) -> Result<Pos> {
	let p = v.location;
	let y = tall_state(cx).spawn(p.x, p.y as i32, p.z);
	Ok(Pos { y: y as i16, ..p })
}

pub fn nether_map_1_17(
	v: old::play_toClient_packet_map::Ty,
	_cx: &mut Session,
) -> Result<new::play_toClient_packet_map::Ty> {
	use new::play_toClient_packet_map__columns::Ty as C;
	Ok(new::play_toClient_packet_map::Ty {
		itemDamage: v.itemDamage,
		scale: v.scale,
		locked: v.locked,
		icons: (!v.icons.is_empty()).then(|| {
			v.icons
				.into_iter()
				.map(|i| new::play_toClient_packet_map__icons__items__items::Ty {
					r#type: i.r#type,
					x: i.x,
					z: i.z,
					direction: i.direction,
					displayName: i.displayName,
				})
				.collect()
		}),
		columns: match v.columns {
			old::play_toClient_packet_map__columns::Ty::default(tag, c) => C::default(
				tag as u8,
				new::play_toClient_packet_map__columns__default::Ty {
					rows: c.rows as u8,
					x: c.x as u8,
					y: c.y as u8,
					data: c.data,
				},
			),
			old::play_toClient_packet_map__columns::Ty::v0 => C::v0,
		},
	})
}

const MAP_COLORS_1_16_2: [u8; 12] = [85, 27, 45, 84, 144, 145, 146, 147, 127, 226, 124, 227];

pub fn nether_map_1_16_2(
	v: new::play_toClient_packet_map::Ty,
	_cx: &mut Session,
) -> Result<old::play_toClient_packet_map::Ty> {
	use old::play_toClient_packet_map__columns::Ty as C;
	Ok(old::play_toClient_packet_map::Ty {
		itemDamage: v.itemDamage,
		scale: v.scale,
		trackingPosition: true,
		locked: v.locked,
		icons: v
			.icons
			.unwrap_or_default()
			.into_iter()
			.map(|i| old::play_toClient_packet_map__icons__items::Ty {
				r#type: i.r#type,
				x: i.x,
				z: i.z,
				direction: i.direction,
				displayName: i.displayName,
			})
			.collect(),
		columns: match v.columns {
			new::play_toClient_packet_map__columns::Ty::default(tag, c) => {
				let data = c
					.data
					.into_iter()
					.map(|b| {
						(b as usize)
							.checked_sub(236)
							.and_then(|i| MAP_COLORS_1_16_2.get(i))
							.copied()
							.unwrap_or(b)
					})
					.collect();
				C::default(
					tag as i8,
					old::play_toClient_packet_map__columns__default::Ty {
						rows: c.rows as i8,
						x: c.x as i8,
						y: c.y as i8,
						data,
					},
				)
			}
			new::play_toClient_packet_map__columns::Ty::v0 => C::v0,
		},
	})
}

pub fn nether_resource_pack_prompt_1_17(
	_src: &old::play_toClient_packet_resource_pack_send::Ty,
	_cx: &mut Session,
) -> Result<Option<String>> {
	Ok(None)
}

pub fn nether_title_1_17(
	p: old::play_toClient_packet_title::Ty,
	_cx: &mut Session,
) -> Result<Vec<NewRoot>> {
	use old::play_toClient_packet_title::Ty as T;
	Ok(vec![match p {
		T::v0(text) => NewRoot::set_title_text(text),
		T::v1(text) => NewRoot::set_title_subtitle(text),
		T::v2(text) => NewRoot::action_bar(text),
		T::v3(t) => NewRoot::set_title_time(new::play_toClient_packet_set_title_time::Ty {
			fadeIn: t.fadeIn,
			stay: t.stay,
			fadeOut: t.fadeOut,
		}),
		T::default(4) => NewRoot::clear_titles(false),
		T::default(5) => NewRoot::clear_titles(true),
		T::default(_) => return Ok(vec![]),
	}])
}

pub fn nether_set_title_text_1_16_2(text: String, _cx: &mut Session) -> Result<Vec<OldRoot>> {
	Ok(vec![OldRoot::title(
		old::play_toClient_packet_title::Ty::v0(text),
	)])
}

pub fn nether_set_title_subtitle_1_16_2(text: String, _cx: &mut Session) -> Result<Vec<OldRoot>> {
	Ok(vec![OldRoot::title(
		old::play_toClient_packet_title::Ty::v1(text),
	)])
}

pub fn nether_action_bar_1_16_2(text: String, _cx: &mut Session) -> Result<Vec<OldRoot>> {
	Ok(vec![OldRoot::title(
		old::play_toClient_packet_title::Ty::v2(text),
	)])
}

pub fn nether_set_title_time_1_16_2(
	t: new::play_toClient_packet_set_title_time::Ty,
	_cx: &mut Session,
) -> Result<Vec<OldRoot>> {
	Ok(vec![OldRoot::title(
		old::play_toClient_packet_title::Ty::v3(old::play_toClient_packet_title__v3::Ty {
			fadeIn: t.fadeIn,
			stay: t.stay,
			fadeOut: t.fadeOut,
		}),
	)])
}

pub fn nether_clear_titles_1_16_2(reset: bool, _cx: &mut Session) -> Result<Vec<OldRoot>> {
	Ok(vec![OldRoot::title(
		old::play_toClient_packet_title::Ty::default(if reset { 5 } else { 4 }),
	)])
}

pub fn nether_combat_event_1_17(
	p: old::play_toClient_packet_combat_event::Ty,
	_cx: &mut Session,
) -> Result<Vec<NewRoot>> {
	use old::play_toClient_packet_combat_event::Ty as C;
	Ok(vec![match p {
		C::v1(e) => NewRoot::end_combat_event(new::play_toClient_packet_end_combat_event::Ty {
			duration: e.duration,
			entityId: e.entityId,
		}),
		C::v2(e) => NewRoot::death_combat_event(new::play_toClient_packet_death_combat_event::Ty {
			playerId: e.playerId,
			entityId: e.entityId,
			message: e.message,
		}),
		C::default(0) => {
			NewRoot::enter_combat_event(new::play_toClient_packet_enter_combat_event::Ty {})
		}
		C::default(_) => return Ok(vec![]),
	}])
}

pub fn nether_enter_combat_event_1_16_2(
	_p: new::play_toClient_packet_enter_combat_event::Ty,
	_cx: &mut Session,
) -> Result<Vec<OldRoot>> {
	Ok(vec![OldRoot::combat_event(
		old::play_toClient_packet_combat_event::Ty::default(0),
	)])
}

pub fn nether_end_combat_event_1_16_2(
	e: new::play_toClient_packet_end_combat_event::Ty,
	_cx: &mut Session,
) -> Result<Vec<OldRoot>> {
	Ok(vec![OldRoot::combat_event(
		old::play_toClient_packet_combat_event::Ty::v1(
			old::play_toClient_packet_combat_event__v1::Ty {
				duration: e.duration,
				entityId: e.entityId,
			},
		),
	)])
}

pub fn nether_death_combat_event_1_16_2(
	e: new::play_toClient_packet_death_combat_event::Ty,
	_cx: &mut Session,
) -> Result<Vec<OldRoot>> {
	Ok(vec![OldRoot::combat_event(
		old::play_toClient_packet_combat_event::Ty::v2(
			old::play_toClient_packet_combat_event__v2::Ty {
				playerId: e.playerId,
				entityId: e.entityId,
				message: e.message,
			},
		),
	)])
}

pub fn nether_world_border_1_17(
	p: old::play_toClient_packet_world_border::Ty,
	_cx: &mut Session,
) -> Result<Vec<NewRoot>> {
	use old::play_toClient_packet_world_border::Ty as W;
	Ok(vec![match p {
		W::v0(size) => NewRoot::world_border_size(size),
		W::v1(l) => {
			NewRoot::world_border_lerp_size(new::play_toClient_packet_world_border_lerp_size::Ty {
				oldDiameter: l.old_radius,
				newDiameter: l.new_radius,
				speed: l.speed,
			})
		}
		W::v2(c) => {
			NewRoot::world_border_center(new::play_toClient_packet_world_border_center::Ty {
				x: c.x,
				z: c.z,
			})
		}
		W::v3(i) => NewRoot::initialize_world_border(
			new::play_toClient_packet_initialize_world_border::Ty {
				x: i.x,
				z: i.z,
				oldDiameter: i.old_radius,
				newDiameter: i.new_radius,
				speed: i.speed,
				portalTeleportBoundary: i.portalBoundary,
				warningBlocks: i.warning_blocks,
				warningTime: i.warning_time,
			},
		),
		W::v4(t) => NewRoot::world_border_warning_delay(t),
		W::v5(b) => NewRoot::world_border_warning_reach(b),
		W::default(_) => return Ok(vec![]),
	}])
}

fn border(w: old::play_toClient_packet_world_border::Ty) -> Result<Vec<OldRoot>> {
	Ok(vec![OldRoot::world_border(w)])
}

pub fn nether_world_border_size_1_16_2(size: f64, _cx: &mut Session) -> Result<Vec<OldRoot>> {
	border(old::play_toClient_packet_world_border::Ty::v0(size))
}

pub fn nether_world_border_lerp_size_1_16_2(
	l: new::play_toClient_packet_world_border_lerp_size::Ty,
	_cx: &mut Session,
) -> Result<Vec<OldRoot>> {
	border(old::play_toClient_packet_world_border::Ty::v1(
		old::play_toClient_packet_world_border__v1::Ty {
			old_radius: l.oldDiameter,
			new_radius: l.newDiameter,
			speed: l.speed,
		},
	))
}

pub fn nether_world_border_center_1_16_2(
	c: new::play_toClient_packet_world_border_center::Ty,
	_cx: &mut Session,
) -> Result<Vec<OldRoot>> {
	border(old::play_toClient_packet_world_border::Ty::v2(
		old::play_toClient_packet_world_border__v2::Ty { x: c.x, z: c.z },
	))
}

pub fn nether_initialize_world_border_1_16_2(
	i: new::play_toClient_packet_initialize_world_border::Ty,
	_cx: &mut Session,
) -> Result<Vec<OldRoot>> {
	border(old::play_toClient_packet_world_border::Ty::v3(
		old::play_toClient_packet_world_border__v3::Ty {
			x: i.x,
			z: i.z,
			old_radius: i.oldDiameter,
			new_radius: i.newDiameter,
			speed: i.speed,
			portalBoundary: i.portalTeleportBoundary,
			warning_time: i.warningTime,
			warning_blocks: i.warningBlocks,
		},
	))
}

pub fn nether_world_border_warning_delay_1_16_2(t: i32, _cx: &mut Session) -> Result<Vec<OldRoot>> {
	border(old::play_toClient_packet_world_border::Ty::v4(t))
}

pub fn nether_world_border_warning_reach_1_16_2(b: i32, _cx: &mut Session) -> Result<Vec<OldRoot>> {
	border(old::play_toClient_packet_world_border::Ty::v5(b))
}

const PING_ACK: i32 = 1 << 30;

pub fn nether_transaction_1_17(
	p: old::play_toClient_packet_transaction::Ty,
	_cx: &mut Session,
) -> Result<Vec<NewRoot>> {
	if p.accepted {
		return Ok(vec![]);
	}
	Ok(vec![NewRoot::ping(
		PING_ACK | ((p.windowId as u8 as i32) << 16) | (p.action as u16 as i32),
	)])
}

pub fn nether_pong_1_16_2(
	id: i32,
	_cx: &mut Session,
) -> Result<Vec<old::play_toServer_packet::Ty>> {
	if id & PING_ACK == 0 {
		return Ok(vec![]);
	}
	Ok(vec![old::play_toServer_packet::Ty::transaction(
		old::play_toServer_packet_transaction::Ty {
			windowId: (id >> 16) as u8 as i8,
			action: id as i16,
			accepted: true,
		},
	)])
}

pub fn nether_ping_1_16_2(id: i32, _cx: &mut Session) -> Result<Vec<OldRoot>> {
	if id != id as i16 as i32 {
		return Ok(vec![]);
	}
	Ok(vec![OldRoot::transaction(
		old::play_toClient_packet_transaction::Ty {
			windowId: 0,
			action: id as i16,
			accepted: false,
		},
	)])
}

pub fn nether_server_transaction_1_17(
	p: old::play_toServer_packet_transaction::Ty,
	_cx: &mut Session,
) -> Result<Vec<new::play_toServer_packet::Ty>> {
	if p.windowId != 0 || !p.accepted {
		return Ok(vec![]);
	}
	Ok(vec![new::play_toServer_packet::Ty::pong(p.action as i32)])
}

pub fn nether_window_click_1_17(
	v: old::play_toServer_packet_window_click::Ty,
	_cx: &mut Session,
) -> Result<new::play_toServer_packet_window_click::Ty> {
	Ok(new::play_toServer_packet_window_click::Ty {
		windowId: v.windowId,
		slot: v.slot,
		mouseButton: v.mouseButton,
		mode: v.mode,
		changedSlots: vec![],
		cursorItem: slot_up(v.item),
	})
}

pub fn nether_window_click_1_16_2(
	v: new::play_toServer_packet_window_click::Ty,
	_cx: &mut Session,
) -> Result<old::play_toServer_packet_window_click::Ty> {
	Ok(old::play_toServer_packet_window_click::Ty {
		windowId: v.windowId,
		slot: v.slot,
		mouseButton: v.mouseButton,
		action: 0,
		mode: v.mode,
		item: if matches!(v.mode, 1 | 5) {
			Slot::r#false
		} else {
			slot_down(v.cursorItem)
		},
	})
}
