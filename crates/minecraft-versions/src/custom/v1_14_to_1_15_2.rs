use std::collections::{HashMap, HashSet};

use super::v1_13_2_to_1_14::{
	entity_name, entry_14, is_living, item, read_states, remap, state, statistic, world_event,
	write_states, Bytes,
};
use crate::gen::convert::{v1_14, v1_14_1, v1_14_3, v1_14_4, v1_15, Root, Version};
use crate::pds::convert::{unconvertible, Error, Result};
use crate::pds::{Input, Output};
use crate::registry;
use crate::session::Session;

type Client14 = v1_14::play_to_client_packet::Ty;
type Client141 = v1_14_1::play_to_client_packet::Ty;
type Client143 = v1_14_3::play_to_client_packet::Ty;
type Client144 = v1_14_4::play_to_client_packet::Ty;
type Client15 = v1_15::play_to_client_packet::Ty;
type Slot = v1_14::slot::Ty;
type M14 = v1_14::entity_metadata_item::Ty;
type E14 = v1_14::entity_metadata_items::Ty;
type P14 = v1_14::particle_data::Ty;
type Recipe = v1_14::play_to_client_packet_declare_recipes_items::Ty;

const OLD: Version = Version::V1_14_4;
const NEW: Version = Version::V1_15;
const CUSTOM_STATISTICS: i32 = 69;

#[derive(Default)]
struct Traders(HashSet<i32>);

fn is_trader(ty: i32) -> bool {
	matches!(
		entity_name(Version::V1_14, ty),
		"villager" | "wandering_trader"
	)
}

fn head_shake(items: Vec<E14>, up: bool) -> Vec<E14> {
	items
		.into_iter()
		.filter_map(|mut e| {
			match (up, e.key) {
				(true, k) if k >= 15 => e.key = k + 1,
				(false, 15) => return None,
				(false, k) if k > 15 => e.key = k - 1,
				_ => {}
			}
			Some(e)
		})
		.collect()
}

fn trader_spawn(
	p: v1_14::play_to_client_packet_spawn_entity_living::Ty,
	up: bool,
	cx: &mut Session,
) -> v1_14::play_to_client_packet_spawn_entity_living::Ty {
	let traders = &mut cx.state::<Traders>().0;
	match is_trader(p.r#type) {
		true => {
			traders.insert(p.entity_id);
			v1_14::play_to_client_packet_spawn_entity_living::Ty {
				metadata: head_shake(p.metadata, up),
				..p
			}
		}
		false => {
			traders.remove(&p.entity_id);
			p
		}
	}
}

fn trader_metadata(
	p: v1_14::play_to_client_packet_entity_metadata::Ty,
	up: bool,
	cx: &mut Session,
) -> v1_14::play_to_client_packet_entity_metadata::Ty {
	match cx.state::<Traders>().0.contains(&p.entity_id) {
		true => v1_14::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
			metadata: head_shake(p.metadata, up),
		},
		false => p,
	}
}

fn forget_traders(ids: &[i32], cx: &mut Session) {
	let traders = &mut cx.state::<Traders>().0;
	for id in ids {
		traders.remove(id);
	}
}

pub fn village_trader_spawn_1_14_1(
	p: v1_14::play_to_client_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client141>> {
	Ok(vec![Client141::SpawnEntityLiving(trader_spawn(
		p, true, cx,
	))])
}

pub fn village_trader_spawn_1_14(
	p: v1_14_1::play_to_client_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	Ok(vec![Client14::SpawnEntityLiving(trader_spawn(
		p, false, cx,
	))])
}

pub fn village_trader_metadata_1_14_1(
	p: v1_14::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client141>> {
	Ok(vec![Client141::EntityMetadata(trader_metadata(
		p, true, cx,
	))])
}

pub fn village_trader_metadata_1_14(
	p: v1_14_1::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client14>> {
	Ok(vec![Client14::EntityMetadata(trader_metadata(
		p, false, cx,
	))])
}

pub fn village_trader_destroy_1_14_1(ids: Vec<i32>, cx: &mut Session) -> Result<Vec<Client141>> {
	forget_traders(&ids, cx);
	Ok(vec![Client141::EntityDestroy(ids)])
}

pub fn village_trader_destroy_1_14(ids: Vec<i32>, cx: &mut Session) -> Result<Vec<Client14>> {
	forget_traders(&ids, cx);
	Ok(vec![Client14::EntityDestroy(ids)])
}

pub fn village_recipe_1_14_1(r: Recipe, _cx: &mut Session) -> Result<Recipe> {
	use v1_14::play_to_client_packet_declare_recipes_items_data::Ty as Data;
	match r.data {
		Data::MinecraftCraftingSpecialRepairitem => Err(Error::Dropped),
		_ => Ok(r),
	}
}

pub fn village_digging_ack_1_14_3(
	p: v1_14_4::play_to_client_packet_acknowledge_player_digging::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client143>> {
	if p.successful && p.status == 0 {
		return Ok(vec![]);
	}
	Ok(vec![Client143::BlockChange(
		v1_14_3::play_to_client_packet_block_change::Ty {
			location: p.location,
			r#type: p.block,
		},
	)])
}

#[derive(Default)]
struct State15 {
	self_id: i32,
	types: HashMap<i32, &'static str>,
	immediate_respawn: bool,
}

fn key_1_15(name: &str, k: u8) -> Option<u8> {
	match (name, k) {
		("wolf", 17) => None,
		("wolf", 18..) => Some(k),
		(n, 12..) if is_living(n) => Some(k + 1),
		_ => Some(k),
	}
}

fn key_1_14_4(name: &str, k: u8) -> Option<u8> {
	match (name, k) {
		("bee", 15..) | ("enderman", 17) | ("trident", 11) => None,
		_ => (0..=k).find(|old| key_1_15(name, *old) == Some(k)),
	}
}

pub fn village_slot_1_15(s: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(item(OLD, NEW, s))
}

pub fn village_slot_1_14_4(s: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(item(NEW, OLD, s))
}

fn particle_data(id: i32, data: P14, from: Version, to: Version) -> Result<(i32, P14)> {
	let id = remap("particles", from, to, id).ok_or(Error::Dropped)?;
	Ok((
		id,
		match data {
			P14::V3(s) => P14::V3(state(from, to, s)),
			P14::V23(s) => P14::V23(state(from, to, s)),
			P14::V32(s) => P14::V32(item(from, to, s)),
			d => d,
		},
	))
}

fn value(v: M14, from: Version, to: Version) -> Option<M14> {
	Some(match v {
		M14::V6(s) => M14::V6(item(from, to, s)),
		M14::V13(s) => M14::V13(state(from, to, s)),
		M14::V15(p) => {
			let (particle_id, data) = particle_data(p.particle_id, p.data, from, to).ok()?;
			M14::V15(v1_14::particle::Ty { particle_id, data })
		}
		v => v,
	})
}

fn metadata(name: &str, items: Vec<E14>, up: bool) -> Vec<E14> {
	let (from, to) = if up { (OLD, NEW) } else { (NEW, OLD) };
	let mut out = Vec::with_capacity(items.len() + 1);
	for e in items {
		let Some(key) = (if up {
			key_1_15(name, e.key)
		} else {
			key_1_14_4(name, e.key)
		}) else {
			continue;
		};
		let v = match (e.value, name.ends_with("minecart") && e.key == 10) {
			(M14::V1(s), true) => M14::V1(state(from, to, s)),
			(v, _) => match value(v, from, to) {
				Some(v) => v,
				None => continue,
			},
		};
		if let (false, "wolf", 8, M14::V2(health)) = (up, name, key, &v) {
			out.push(entry_14(17, M14::V2(*health)));
		}
		out.push(entry_14(key, v));
	}
	out
}

fn custom_name(text: &str) -> E14 {
	entry_14(
		2,
		M14::V5(Some(serde_json::json!({ "text": text }).to_string())),
	)
}

fn named(cx: &mut Session, id: i32) -> &'static str {
	cx.state::<State15>().types.get(&id).copied().unwrap_or("")
}

fn track(cx: &mut Session, id: i32, name: &'static str) {
	cx.state::<State15>().types.insert(id, name);
}

fn metadata_packet_15(entity_id: i32, metadata: Vec<E14>) -> Option<Client15> {
	(!metadata.is_empty()).then(|| {
		Client15::EntityMetadata(v1_15::play_to_client_packet_entity_metadata::Ty {
			entity_id,
			metadata,
		})
	})
}

pub fn village_spawn_entity_living_1_15(
	p: v1_14_4::play_to_client_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client15>> {
	let Some(ty) = remap("entities", OLD, NEW, p.r#type) else {
		return Ok(vec![]);
	};
	let name = entity_name(NEW, ty);
	track(cx, p.entity_id, name);
	let mut out = vec![Client15::SpawnEntityLiving(
		v1_15::play_to_client_packet_spawn_entity_living::Ty {
			entity_id: p.entity_id,
			entity_uuid: p.entity_uuid,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			head_pitch: p.head_pitch,
			velocity: p.velocity,
		},
	)];
	out.extend(metadata_packet_15(
		p.entity_id,
		metadata(name, p.metadata, true),
	));
	Ok(out)
}

pub fn village_named_entity_spawn_1_15(
	p: v1_14_4::play_to_client_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client15>> {
	track(cx, p.entity_id, "player");
	let mut out = vec![Client15::NamedEntitySpawn(
		v1_15::play_to_client_packet_named_entity_spawn::Ty {
			entity_id: p.entity_id,
			player_uuid: p.player_uuid,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
		},
	)];
	out.extend(metadata_packet_15(
		p.entity_id,
		metadata("player", p.metadata, true),
	));
	Ok(out)
}

pub fn village_spawn_entity_living_1_14_4(
	p: v1_15::play_to_client_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client144>> {
	let Some(ty) = remap("entities", NEW, OLD, p.r#type) else {
		return Ok(vec![]);
	};
	let name = entity_name(NEW, p.r#type);
	track(cx, p.entity_id, name);
	let metadata = match name {
		"bee" => vec![
			entry_14(14, M14::V7(false)),
			entry_14(15, M14::V1(2)),
			custom_name("Bee"),
		],
		_ => Vec::new(),
	};
	Ok(vec![Client144::SpawnEntityLiving(
		v1_14_4::play_to_client_packet_spawn_entity_living::Ty {
			entity_id: p.entity_id,
			entity_uuid: p.entity_uuid,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			head_pitch: p.head_pitch,
			velocity: p.velocity,
			metadata,
		},
	)])
}

pub fn village_named_entity_spawn_1_14_4(
	p: v1_15::play_to_client_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client144>> {
	track(cx, p.entity_id, "player");
	Ok(vec![Client144::NamedEntitySpawn(
		v1_14_4::play_to_client_packet_named_entity_spawn::Ty {
			entity_id: p.entity_id,
			player_uuid: p.player_uuid,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			metadata: Vec::new(),
		},
	)])
}

pub fn village_spawn_entity_1_15(
	p: v1_14_4::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client15>> {
	let Some(ty) = remap("entities", OLD, NEW, p.r#type) else {
		return Ok(vec![]);
	};
	let name = entity_name(NEW, ty);
	track(cx, p.entity_id, name);
	let object_data = match name {
		"falling_block" => state(OLD, NEW, p.object_data),
		_ => p.object_data,
	};
	Ok(vec![Client15::SpawnEntity(
		v1_15::play_to_client_packet_spawn_entity::Ty {
			r#type: ty,
			object_data,
			..p
		},
	)])
}

pub fn village_spawn_entity_1_14_4(
	p: v1_15::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client144>> {
	let Some(ty) = remap("entities", NEW, OLD, p.r#type) else {
		return Ok(vec![]);
	};
	let name = entity_name(NEW, p.r#type);
	track(cx, p.entity_id, name);
	let object_data = match name {
		"falling_block" => state(NEW, OLD, p.object_data),
		_ => p.object_data,
	};
	Ok(vec![Client144::SpawnEntity(
		v1_14_4::play_to_client_packet_spawn_entity::Ty {
			r#type: ty,
			object_data,
			..p
		},
	)])
}

pub fn village_entity_metadata_1_15(
	p: v1_14_4::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client15>> {
	let name = named(cx, p.entity_id);
	Ok(vec![Client15::EntityMetadata(
		v1_15::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
			metadata: metadata(name, p.metadata, true),
		},
	)])
}

pub fn village_entity_metadata_1_14_4(
	p: v1_15::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client144>> {
	let name = named(cx, p.entity_id);
	Ok(vec![Client144::EntityMetadata(
		v1_14_4::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
			metadata: metadata(name, p.metadata, false),
		},
	)])
}

fn forget(ids: &[i32], cx: &mut Session) {
	let types = &mut cx.state::<State15>().types;
	for id in ids {
		types.remove(id);
	}
}

pub fn village_entity_destroy_1_15(ids: Vec<i32>, cx: &mut Session) -> Result<Vec<Client15>> {
	forget(&ids, cx);
	Ok(vec![Client15::EntityDestroy(ids)])
}

pub fn village_entity_destroy_1_14_4(ids: Vec<i32>, cx: &mut Session) -> Result<Vec<Client144>> {
	forget(&ids, cx);
	Ok(vec![Client144::EntityDestroy(ids)])
}

fn reset(cx: &mut Session, self_id: Option<i32>) {
	let st = cx.state::<State15>();
	st.self_id = self_id.unwrap_or(st.self_id);
	st.types.clear();
	st.types.insert(st.self_id, "player");
}

pub fn village_login_1_15(
	p: v1_14_4::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client15>> {
	reset(cx, Some(p.entity_id));
	Ok(vec![Client15::Login(
		v1_15::play_to_client_packet_login::Ty {
			entity_id: p.entity_id,
			game_mode: p.game_mode,
			dimension: p.dimension,
			hashed_seed: 0,
			max_players: p.max_players,
			level_type: p.level_type,
			view_distance: p.view_distance,
			reduced_debug_info: p.reduced_debug_info,
			enable_respawn_screen: true,
		},
	)])
}

pub fn village_game_state_change_1_14_4(
	p: v1_15::play_to_client_packet_game_state_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client144>> {
	let reason = v1_15::play_to_client_packet_game_state_change_reason::virtual_tag(&p.reason);
	if reason == 11 {
		cx.state::<State15>().immediate_respawn = p.game_mode == 1.0;
		return Ok(vec![]);
	}
	let reason =
		v1_14_4::play_to_client_packet_game_state_change_reason::deserialize(&mut Input::new(&[
			reason,
		]))
		.map_err(|e| Error::Unconvertible(format!("game state {reason}: {e:?}")))?;
	Ok(vec![Client144::GameStateChange(
		v1_14_4::play_to_client_packet_game_state_change::Ty {
			reason,
			game_mode: p.game_mode,
		},
	)])
}

pub fn village_update_health_1_14_4(
	p: v1_15::play_to_client_packet_update_health::Ty,
	cx: &mut Session,
) -> Result<Vec<Client144>> {
	if p.health <= 0.0 && cx.state::<State15>().immediate_respawn {
		let mut o = Output::new();
		let respawn = v1_15::play_to_server_packet::Ty::ClientCommand(
			v1_15::play_to_server_packet_client_command::Ty::PerformRespawn,
		);
		v1_15::play_to_server_packet::serialize(&respawn, &mut o)
			.map_err(|e| Error::Encoding(format!("{e:?}")))?;
		cx.send(Root::PlayToServer, NEW as usize, o.into_inner());
	}
	Ok(vec![Client144::UpdateHealth(p)])
}

pub fn village_login_1_14_4(
	p: v1_15::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client144>> {
	reset(cx, Some(p.entity_id));
	cx.state::<State15>().immediate_respawn = !p.enable_respawn_screen;
	Ok(vec![Client144::Login(
		v1_14_4::play_to_client_packet_login::Ty {
			entity_id: p.entity_id,
			game_mode: p.game_mode,
			dimension: p.dimension,
			max_players: p.max_players,
			level_type: p.level_type,
			view_distance: p.view_distance,
			reduced_debug_info: p.reduced_debug_info,
		},
	)])
}

pub fn village_respawn_1_15(
	p: v1_14_4::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client15>> {
	reset(cx, None);
	Ok(vec![Client15::Respawn(
		v1_15::play_to_client_packet_respawn::Ty {
			dimension: p.dimension,
			hashed_seed: 0,
			gamemode: p.gamemode,
			level_type: p.level_type,
		},
	)])
}

pub fn village_respawn_1_14_4(
	p: v1_15::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client144>> {
	reset(cx, None);
	Ok(vec![Client144::Respawn(
		v1_14_4::play_to_client_packet_respawn::Ty {
			dimension: p.dimension,
			gamemode: p.gamemode,
			level_type: p.level_type,
		},
	)])
}

fn sections(data: &[u8], bitmap: i32, from: Version, to: Version) -> Result<(Vec<u8>, Vec<u8>)> {
	let table = registry::state_table(from, to)
		.ok_or_else(|| Error::Unconvertible(format!("no {from:?} -> {to:?} block states")))?;
	let mut r = Bytes::new(data);
	let mut out = Vec::with_capacity(data.len());
	for _ in (0..16).filter(|y| bitmap & 1 << y != 0) {
		out.extend(r.take(2)?);
		let mut states = read_states(&mut r)?;
		for s in &mut states {
			*s = table.get(*s as usize).copied().unwrap_or(0);
		}
		write_states(&mut out, &states);
	}
	Ok((out, r.rest().to_vec()))
}

pub fn village_biomes_1_15(old: &[i32]) -> Vec<i32> {
	(0..1024)
		.map(|i| old[((i >> 2 & 3) * 4 + 2) * 16 + (i & 3) * 4 + 2])
		.collect()
}

pub fn village_biomes_1_14_4(new: &[i32]) -> Vec<i32> {
	(0..256).map(|i| new[(i >> 6) * 4 + (i & 15) / 4]).collect()
}

pub fn village_map_chunk_1_15(
	p: v1_14_4::play_to_client_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client15>> {
	use v1_15::play_to_client_packet_map_chunk_biomes::Ty as Biomes;
	let (chunk_data, rest) = sections(&p.chunk_data, p.bit_map, OLD, NEW)?;
	let biomes = match (p.ground_up, rest.len()) {
		(true, 1024) => {
			let old: Vec<i32> = rest
				.chunks_exact(4)
				.map(|c| i32::from_be_bytes(c.try_into().expect("4 bytes")))
				.collect();
			Biomes::True(village_biomes_1_15(&old))
		}
		(false, 0) => Biomes::False,
		(_, n) => return unconvertible(format!("{n} bytes follow the chunk sections")),
	};
	Ok(vec![Client15::MapChunk(
		v1_15::play_to_client_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			bit_map: p.bit_map,
			heightmaps: p.heightmaps,
			biomes,
			chunk_data,
			block_entities: p.block_entities,
		},
	)])
}

pub fn village_map_chunk_1_14_4(
	p: v1_15::play_to_client_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client144>> {
	use v1_15::play_to_client_packet_map_chunk_biomes::Ty as Biomes;
	let (mut chunk_data, rest) = sections(&p.chunk_data, p.bit_map, NEW, OLD)?;
	if !rest.is_empty() {
		return unconvertible(format!("{} bytes follow the chunk sections", rest.len()));
	}
	let ground_up = match p.biomes {
		Biomes::True(b) if b.len() == 1024 => {
			for v in village_biomes_1_14_4(&b) {
				chunk_data.extend(v.to_be_bytes());
			}
			true
		}
		Biomes::True(b) => return unconvertible(format!("{} biomes instead of 1024", b.len())),
		Biomes::False => false,
	};
	Ok(vec![Client144::MapChunk(
		v1_14_4::play_to_client_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			ground_up,
			bit_map: p.bit_map,
			heightmaps: p.heightmaps,
			chunk_data,
			block_entities: p.block_entities,
		},
	)])
}

pub fn village_world_particles_1_15(
	p: v1_14_4::play_to_client_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<v1_15::play_to_client_packet_world_particles::Ty> {
	let (particle_id, data) = particle_data(p.particle_id, p.data, OLD, NEW)?;
	Ok(v1_15::play_to_client_packet_world_particles::Ty {
		particle_id,
		long_distance: p.long_distance,
		x: p.x as f64,
		y: p.y as f64,
		z: p.z as f64,
		offset_x: p.offset_x,
		offset_y: p.offset_y,
		offset_z: p.offset_z,
		particle_data: p.particle_data,
		particles: p.particles,
		data,
	})
}

pub fn village_world_particles_1_14_4(
	p: v1_15::play_to_client_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<v1_14_4::play_to_client_packet_world_particles::Ty> {
	let (particle_id, data) = particle_data(p.particle_id, p.data, NEW, OLD)?;
	Ok(v1_14_4::play_to_client_packet_world_particles::Ty {
		particle_id,
		long_distance: p.long_distance,
		x: p.x as f32,
		y: p.y as f32,
		z: p.z as f32,
		offset_x: p.offset_x,
		offset_y: p.offset_y,
		offset_z: p.offset_z,
		particle_data: p.particle_data,
		particles: p.particles,
		data,
	})
}

fn retag(
	tags: &mut [v1_14::play_to_client_packet_tags_block_tags_items::Ty],
	f: impl Fn(i32) -> Option<i32>,
) {
	for t in tags {
		t.entries = t.entries.iter().filter_map(|id| f(*id)).collect();
	}
}

fn tags(
	mut p: v1_14::play_to_client_packet_tags::Ty,
	from: Version,
	to: Version,
) -> v1_14::play_to_client_packet_tags::Ty {
	retag(&mut p.block_tags, |id| remap("blocks", from, to, id));
	retag(&mut p.item_tags, |id| remap("items", from, to, id));
	retag(&mut p.entity_tags, |id| remap("entities", from, to, id));
	p
}

pub fn village_tags_1_15(
	p: v1_14_4::play_to_client_packet_tags::Ty,
	_cx: &mut Session,
) -> Result<v1_15::play_to_client_packet_tags::Ty> {
	Ok(tags(p, OLD, NEW))
}

pub fn village_tags_1_14_4(
	p: v1_15::play_to_client_packet_tags::Ty,
	_cx: &mut Session,
) -> Result<v1_14_4::play_to_client_packet_tags::Ty> {
	Ok(tags(p, NEW, OLD))
}

type Statistic = v1_14::play_to_client_packet_statistics_items::Ty;

pub fn village_statistic_1_15(s: Statistic, _cx: &mut Session) -> Result<Statistic> {
	statistic(s, OLD, NEW, CUSTOM_STATISTICS)
}

pub fn village_statistic_1_14_4(s: Statistic, _cx: &mut Session) -> Result<Statistic> {
	statistic(s, NEW, OLD, CUSTOM_STATISTICS)
}

pub fn village_block_change_1_15(
	p: &v1_14_4::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(OLD, NEW, p.r#type))
}

pub fn village_block_change_1_14_4(
	p: &v1_15::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(NEW, OLD, p.r#type))
}

pub fn village_multi_block_change_1_15(
	r: &v1_14_4::play_to_client_packet_multi_block_change_records_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(OLD, NEW, r.block_id))
}

pub fn village_multi_block_change_1_14_4(
	r: &v1_15::play_to_client_packet_multi_block_change_records_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(NEW, OLD, r.block_id))
}

pub fn village_digging_ack_1_15(
	p: &v1_14_4::play_to_client_packet_acknowledge_player_digging::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(OLD, NEW, p.block))
}

pub fn village_digging_ack_1_14_4(
	p: &v1_15::play_to_client_packet_acknowledge_player_digging::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(NEW, OLD, p.block))
}

pub fn village_block_action_1_15(
	p: &v1_14_4::play_to_client_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("blocks", OLD, NEW, p.block_id).ok_or(Error::Dropped)
}

pub fn village_block_action_1_14_4(
	p: &v1_15::play_to_client_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("blocks", NEW, OLD, p.block_id).ok_or(Error::Dropped)
}

pub fn village_world_event_1_15(
	p: &v1_14_4::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	world_event(p.effect_id, p.data, OLD, NEW)
}

pub fn village_world_event_1_14_4(
	p: &v1_15::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	world_event(p.effect_id, p.data, NEW, OLD)
}

pub fn village_sound_effect_1_15(
	p: &v1_14_4::play_to_client_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("sounds", OLD, NEW, p.sound_id).ok_or(Error::Dropped)
}

pub fn village_sound_effect_1_14_4(
	p: &v1_15::play_to_client_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("sounds", NEW, OLD, p.sound_id).ok_or(Error::Dropped)
}

pub fn village_entity_sound_effect_1_15(
	p: &v1_14_4::play_to_client_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("sounds", OLD, NEW, p.sound_id).ok_or(Error::Dropped)
}

pub fn village_entity_sound_effect_1_14_4(
	p: &v1_15::play_to_client_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("sounds", NEW, OLD, p.sound_id).ok_or(Error::Dropped)
}

pub fn village_set_cooldown_1_15(
	p: &v1_14_4::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("items", OLD, NEW, p.item_id).ok_or(Error::Dropped)
}

pub fn village_set_cooldown_1_14_4(
	p: &v1_15::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap("items", NEW, OLD, p.item_id).ok_or(Error::Dropped)
}
