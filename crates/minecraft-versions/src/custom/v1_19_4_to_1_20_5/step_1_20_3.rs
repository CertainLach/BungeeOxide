use super::common::{
	self, block, block_state, decode, encode, item, name_id, particle_id, sound_named, varint,
	ParticleMap, Sound,
};
use super::step_1_20::{
	entity_sound, optional_state, record, remap_tags, sound_holder, spawn_object_data, tag_entries,
	world_event_data,
};
use super::text;
use crate::gen::convert::Version;
use crate::gen::convert::{v1_20_2 as old, v1_20_3 as new};
use crate::pds::convert::{Error, Result};
use crate::Session;
use nbt::{CompoundExt, Tag};

const OLD: Version = Version::V1_20_2;
const NEW: Version = Version::V1_20_3;

type Client = new::play_toClient_packet::Ty;
type ClientOld = old::play_toClient_packet::Ty;
type Config = new::configuration_toClient_packet::Ty;
type ConfigOld = old::configuration_toClient_packet::Ty;

pub fn trails_text_1_20_2_to_1_20_3(v: String, _cx: &mut Session) -> Result<Tag> {
	Ok(text::to_nbt(&v))
}

pub fn trails_text_1_20_3_to_1_20_2(v: Tag, _cx: &mut Session) -> Result<String> {
	Ok(text::from_nbt(&v))
}

fn normalize_book(s: &mut old::slot__true::Ty) {
	let Some(Tag {
		value: nbt::Payload::compound(c),
		..
	}) = &mut s.nbtData
	else {
		return;
	};
	if crate::registry::names(NEW, "items").and_then(|n| n.name(s.itemId as u32))
		!= Some("written_book")
	{
		return;
	}
	if let Some(nbt::Payload::list(l)) = c.field_mut("pages").as_deref_mut() {
		if let Some(pages) = l.strings_mut() {
			for page in pages.iter_mut() {
				*page = text::from_nbt(&text::to_nbt(page));
			}
		}
	}
}

fn remap_slot(v: old::slot::Ty, from: Version, to: Version) -> old::slot::Ty {
	match v {
		old::slot::Ty::r#true(mut s) => match item(from, to, s.itemId) {
			Some(id) => {
				s.itemId = id;
				if common::later(from, to) {
					normalize_book(&mut s);
				}
				old::slot::Ty::r#true(s)
			}
			None => old::slot::Ty::r#false,
		},
		v => v,
	}
}

pub fn trails_slot_1_20_2_to_1_20_3(v: old::slot::Ty, _cx: &mut Session) -> Result<old::slot::Ty> {
	Ok(remap_slot(v, OLD, NEW))
}

pub fn trails_slot_1_20_3_to_1_20_2(v: old::slot::Ty, _cx: &mut Session) -> Result<old::slot::Ty> {
	Ok(remap_slot(v, NEW, OLD))
}

fn slot_bytes(from: Version, to: Version) -> impl Fn(&[u8]) -> Result<Vec<u8>> {
	move |b: &[u8]| {
		encode(
			&remap_slot(decode(b, |i| old::slot::deserialize(i))?, from, to),
			old::slot::serialize,
		)
	}
}

fn meta_entry(key: u8, ty: i32, value: &[u8]) -> Vec<u8> {
	let mut out = vec![key];
	varint(&mut out, ty);
	out.extend_from_slice(value);
	out
}

fn type_id<T>(
	ty: &T,
	serialize: fn(&T, &mut crate::pds::Output) -> crate::pds::Result<()>,
) -> Result<i32> {
	Ok(common::read_varint(&encode(ty, serialize)?)?.0)
}

fn particle_bytes(from: Version, to: Version, bytes: &[u8]) -> Result<Vec<u8>> {
	let (id, payload) = common::read_varint(bytes)?;
	let item = slot_bytes(from, to);
	let (id, payload) = common::particle(
		&ParticleMap {
			from,
			to,
			item: &item,
		},
		id,
		payload,
	)?;
	let mut out = Vec::new();
	varint(&mut out, id);
	out.extend(payload);
	Ok(out)
}

pub fn trails_metadata_1_20_2_to_1_20_3(
	v: old::entityMetadata::Ty,
	_cx: &mut Session,
) -> Result<new::entityMetadata::Ty> {
	use old::entityMetadataItem::Ty as V;
	let mut out = Vec::with_capacity(v.len());
	for e in v {
		let ty = type_id(&e.r#type, old::entityMetadata__items__type::serialize)?;
		let value = match e.value {
			V::particle(p) => {
				match particle_bytes(OLD, NEW, &encode(&p, old::particle::serialize)?) {
					Ok(b) => b,
					Err(Error::Dropped) => continue,
					Err(e) => return Err(e),
				}
			}
			V::component(s) => encode(&text::to_nbt(&s), nbt::gen::pds_Tag::serialize)?,
			V::optional_component(s) => match s {
				Some(s) => [
					vec![1],
					encode(&text::to_nbt(&s), nbt::gen::pds_Tag::serialize)?,
				]
				.concat(),
				None => vec![0],
			},
			V::block_state(s) => encode(
				&V::block_state(block_state(OLD, NEW, s)),
				old::entityMetadataItem::serialize,
			)?,
			V::optional_block_state(s) => encode(
				&V::optional_block_state(optional_state(OLD, NEW, s)),
				old::entityMetadataItem::serialize,
			)?,
			V::item_stack(s) => encode(
				&V::item_stack(remap_slot(s, OLD, NEW)),
				old::entityMetadataItem::serialize,
			)?,
			v => encode(&v, old::entityMetadataItem::serialize)?,
		};
		out.push(decode(&meta_entry(e.key, ty, &value), |i| {
			new::entityMetadata__items::deserialize(i)
		})?);
	}
	Ok(out)
}

pub fn trails_metadata_1_20_3_to_1_20_2(
	v: new::entityMetadata::Ty,
	_cx: &mut Session,
) -> Result<old::entityMetadata::Ty> {
	use new::entityMetadataItem::Ty as V;
	let mut out = Vec::with_capacity(v.len());
	for e in v {
		let ty = type_id(&e.r#type, new::entityMetadata__items__type::serialize)?;
		let value = match e.value {
			V::component(t) => common::mc_string(&text::from_nbt(&t)),
			V::optional_component(t) => match t {
				Some(t) => [vec![1], common::mc_string(&text::from_nbt(&t))].concat(),
				None => vec![0],
			},
			V::block_state(s) => encode(
				&V::block_state(block_state(NEW, OLD, s)),
				new::entityMetadataItem::serialize,
			)?,
			V::optional_block_state(s) => encode(
				&V::optional_block_state(optional_state(NEW, OLD, s)),
				new::entityMetadataItem::serialize,
			)?,
			V::item_stack(s) => encode(
				&V::item_stack(remap_slot(s, NEW, OLD)),
				new::entityMetadataItem::serialize,
			)?,
			V::particle(p) => {
				match particle_bytes(NEW, OLD, &encode(&p, new::Particle::serialize)?) {
					Ok(b) => b,
					Err(Error::Dropped) => continue,
					Err(e) => return Err(e),
				}
			}
			v => encode(&v, new::entityMetadataItem::serialize)?,
		};
		out.push(decode(&meta_entry(e.key, ty, &value), |i| {
			old::entityMetadata__items::deserialize(i)
		})?);
	}
	Ok(out)
}

macro_rules! world_particles {
	($name:ident, $root:ty, $src:ident, $dst:ident, $from:expr, $to:expr) => {
		arm!(
			$name,
			$root,
			world_particles,
			$src::play_toClient_packet_world_particles::Ty,
			|p, _cx| {
				let payload = encode(&p.data, $src::particleData::serialize)?;
				let item = slot_bytes($from, $to);
				let (id, payload) = common::particle(
					&ParticleMap {
						from: $from,
						to: $to,
						item: &item,
					},
					p.particleId,
					&payload,
				)?;
				$dst::play_toClient_packet_world_particles::Ty {
					particleId: id,
					longDistance: p.longDistance,
					x: p.x,
					y: p.y,
					z: p.z,
					offsetX: p.offsetX,
					offsetY: p.offsetY,
					offsetZ: p.offsetZ,
					particleData: p.particleData,
					particles: p.particles,
					data: decode(&payload, |i| {
						$dst::particleData::deserialize(
							i,
							$dst::particleData::Args { compareTo: &id },
						)
					})?,
				}
			}
		);
	};
}

world_particles!(
	trails_world_particles_1_20_2_to_1_20_3,
	Client,
	old,
	new,
	OLD,
	NEW
);
world_particles!(
	trails_world_particles_1_20_3_to_1_20_2,
	ClientOld,
	new,
	old,
	NEW,
	OLD
);

fn sound_bytes(s: Sound) -> Vec<u8> {
	let mut out = Vec::new();
	match s {
		Sound::Id(id) => varint(&mut out, id + 1),
		Sound::Named(name) => {
			varint(&mut out, 0);
			out.extend(common::mc_string(&name));
			out.push(0);
		}
	}
	out
}

pub fn trails_explosion_1_20_2_to_1_20_3(
	p: old::play_toClient_packet_explosion::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	let mut bytes = encode(&p, old::play_toClient_packet_explosion::serialize)?;
	varint(&mut bytes, 1);
	for name in ["explosion", "explosion_emitter"] {
		varint(&mut bytes, particle_id(NEW, name).ok_or(Error::Dropped)?);
	}
	bytes.extend(sound_bytes(sound_named(NEW, "entity.generic.explode")));
	Ok(vec![Client::explosion(decode(&bytes, |i| {
		new::play_toClient_packet_explosion::deserialize(i)
	})?)])
}

pub fn trails_feet_eyes_1_20_2_to_1_20_3(v: String, _cx: &mut Session) -> Result<i32> {
	Ok(match v.as_str() {
		"eyes" => 1,
		"feet" => 0,
		v => v.parse().unwrap_or(0),
	})
}

pub fn trails_feet_eyes_1_20_3_to_1_20_2(v: i32, _cx: &mut Session) -> Result<String> {
	Ok(if v == 1 { "eyes" } else { "feet" }.to_owned())
}

fn level_chunks_load_start() -> Client {
	Client::game_state_change(new::play_toClient_packet_game_state_change::Ty {
		reason: new::play_toClient_packet_game_state_change__reason::Ty::level_chunks_load_start,
		gameMode: 0.0,
	})
}

pub fn trails_login_1_20_2_to_1_20_3(
	p: old::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	common::state(cx).player = Some(p.entityId);
	Ok(vec![Client::login(p), level_chunks_load_start()])
}

pub fn trails_respawn_1_20_2_to_1_20_3(
	p: old::play_toClient_packet_respawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	Ok(vec![Client::respawn(p), level_chunks_load_start()])
}

pub fn trails_objective_styling_1_20_2_to_1_20_3(
	_src: &old::play_toClient_packet_scoreboard_objective__action__v0::Ty,
	_cx: &mut Session,
) -> Result<new::play_toClient_packet_scoreboard_objective__action__v0__styling::Ty> {
	Ok(new::play_toClient_packet_scoreboard_objective__action__v0__styling::Ty::default(None))
}

pub fn trails_objective_update_styling_1_20_2_to_1_20_3(
	_src: &old::play_toClient_packet_scoreboard_objective__action__v2::Ty,
	_cx: &mut Session,
) -> Result<new::play_toClient_packet_scoreboard_objective__action__v2__styling::Ty> {
	Ok(new::play_toClient_packet_scoreboard_objective__action__v2__styling::Ty::default(None))
}

pub fn trails_scoreboard_score_1_20_2_to_1_20_3(
	p: old::play_toClient_packet_scoreboard_score::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	Ok(vec![match p.value {
		old::play_toClient_packet_scoreboard_score__value::Ty::v1 => {
			Client::reset_score(new::play_toClient_packet_reset_score::Ty {
				entity_name: p.itemName,
				objective_name: Some(p.scoreName).filter(|s| !s.is_empty()),
			})
		}
		old::play_toClient_packet_scoreboard_score__value::Ty::default(_, value) => {
			Client::scoreboard_score(new::play_toClient_packet_scoreboard_score::Ty {
				itemName: p.itemName,
				scoreName: p.scoreName,
				value,
				display_name: None,
				styling: new::play_toClient_packet_scoreboard_score__styling::Ty::default(None),
			})
		}
	}])
}

pub fn trails_scoreboard_score_1_20_3_to_1_20_2(
	p: new::play_toClient_packet_scoreboard_score::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	Ok(vec![ClientOld::scoreboard_score(
		old::play_toClient_packet_scoreboard_score::Ty {
			itemName: p.itemName,
			scoreName: p.scoreName,
			value: old::play_toClient_packet_scoreboard_score__value::Ty::default(0, p.value),
		},
	)])
}

pub fn trails_reset_score_1_20_3_to_1_20_2(
	p: new::play_toClient_packet_reset_score::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	Ok(vec![ClientOld::scoreboard_score(
		old::play_toClient_packet_scoreboard_score::Ty {
			itemName: p.entity_name,
			scoreName: p.objective_name.unwrap_or_default(),
			value: old::play_toClient_packet_scoreboard_score__value::Ty::v1,
		},
	)])
}

pub(super) fn pack_uuid(url: &str) -> u128 {
	let mut h: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
	for b in url.bytes() {
		h ^= b as u128;
		h = h.wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013b);
	}
	(h & !(0xf << 76) | (0x3 << 76)) & !(0x3 << 62) | (0x2 << 62)
}

pub fn trails_resource_pack_send_1_20_2_to_1_20_3(
	p: old::play_toClient_packet_resource_pack_send::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	Ok(vec![
		Client::remove_resource_pack(None),
		Client::add_resource_pack(new::play_toClient_packet_add_resource_pack::Ty {
			uuid: pack_uuid(&p.url),
			url: p.url,
			hash: p.hash,
			forced: p.forced,
			promptMessage: p.promptMessage.map(|m| text::to_nbt(&m)),
		}),
	])
}

pub fn trails_add_resource_pack_1_20_3_to_1_20_2(
	p: new::play_toClient_packet_add_resource_pack::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	common::state(cx).packs.push(p.uuid);
	Ok(vec![ClientOld::resource_pack_send(
		old::play_toClient_packet_resource_pack_send::Ty {
			url: p.url,
			hash: p.hash,
			forced: p.forced,
			promptMessage: p.promptMessage.map(|m| text::from_nbt(&m)),
		},
	)])
}

pub fn trails_config_resource_pack_send_1_20_2_to_1_20_3(
	p: old::configuration_toClient_packet_resource_pack_send::Ty,
	_cx: &mut Session,
) -> Result<Vec<Config>> {
	Ok(vec![
		Config::remove_resource_pack(None),
		Config::add_resource_pack(new::configuration_toClient_packet_add_resource_pack::Ty {
			uuid: pack_uuid(&p.url),
			url: p.url,
			hash: p.hash,
			forced: p.forced,
			promptMessage: p.promptMessage.map(|m| text::to_nbt(&m)),
		}),
	])
}

pub fn trails_config_add_resource_pack_1_20_3_to_1_20_2(
	p: new::configuration_toClient_packet_add_resource_pack::Ty,
	cx: &mut Session,
) -> Result<Vec<ConfigOld>> {
	common::state(cx).packs.push(p.uuid);
	Ok(vec![ConfigOld::resource_pack_send(
		old::configuration_toClient_packet_resource_pack_send::Ty {
			url: p.url,
			hash: p.hash,
			forced: p.forced,
			promptMessage: p.promptMessage.map(|m| text::from_nbt(&m)),
		},
	)])
}

fn pack_result(result: i32) -> Option<i32> {
	match result {
		4 => None,
		r if r > 4 => Some(2),
		r => Some(r),
	}
}

pub fn trails_resource_pack_receive_1_20_3_to_1_20_2(
	p: new::play_toServer_packet_resource_pack_receive::Ty,
	_cx: &mut Session,
) -> Result<Vec<old::play_toServer_packet::Ty>> {
	Ok(pack_result(p.result)
		.map(old::play_toServer_packet::Ty::resource_pack_receive)
		.into_iter()
		.collect())
}

fn pack(cx: &mut Session, result: i32) -> u128 {
	let packs = &mut common::state(cx).packs;
	let uuid = packs.first().copied().unwrap_or(0);
	if result != 3 && !packs.is_empty() {
		packs.remove(0);
	}
	uuid
}

pub fn trails_resource_pack_receive_1_20_2_to_1_20_3(
	result: i32,
	cx: &mut Session,
) -> Result<Vec<new::play_toServer_packet::Ty>> {
	Ok(vec![new::play_toServer_packet::Ty::resource_pack_receive(
		new::play_toServer_packet_resource_pack_receive::Ty {
			uuid: pack(cx, result),
			result,
		},
	)])
}

pub fn trails_config_resource_pack_receive_1_20_3_to_1_20_2(
	p: new::configuration_toServer_packet_resource_pack_receive::Ty,
	_cx: &mut Session,
) -> Result<Vec<old::configuration_toServer_packet::Ty>> {
	Ok(pack_result(p.result)
		.map(old::configuration_toServer_packet::Ty::resource_pack_receive)
		.into_iter()
		.collect())
}

pub fn trails_config_resource_pack_receive_1_20_2_to_1_20_3(
	result: i32,
	cx: &mut Session,
) -> Result<Vec<new::configuration_toServer_packet::Ty>> {
	Ok(vec![
		new::configuration_toServer_packet::Ty::resource_pack_receive(
			new::configuration_toServer_packet_resource_pack_receive::Ty {
				uuid: pack(cx, result),
				result,
			},
		),
	])
}

fn menu(from: Version, to: Version, id: i32) -> Option<i32> {
	match (from, to) {
		(OLD, NEW) => Some(if id >= 7 { id + 1 } else { id }),
		_ => match id {
			7 => None,
			id if id > 7 => Some(id - 1),
			id => Some(id),
		},
	}
}

arm!(
	trails_open_window_1_20_2_to_1_20_3,
	Client,
	open_window,
	old::play_toClient_packet_open_window::Ty,
	|p, _cx| {
		new::play_toClient_packet_open_window::Ty {
			windowId: p.windowId,
			inventoryType: menu(OLD, NEW, p.inventoryType).ok_or(Error::Dropped)?,
			windowTitle: text::to_nbt(&p.windowTitle),
		}
	}
);

arm!(
	trails_open_window_1_20_3_to_1_20_2,
	ClientOld,
	open_window,
	new::play_toClient_packet_open_window::Ty,
	|p, _cx| {
		old::play_toClient_packet_open_window::Ty {
			windowId: p.windowId,
			inventoryType: menu(NEW, OLD, p.inventoryType).ok_or(Error::Dropped)?,
			windowTitle: text::from_nbt(&p.windowTitle),
		}
	}
);

fn chunk_data(from: Version, to: Version, data: &[u8]) -> Result<Vec<u8>> {
	let table = common::state_map(from, to);
	let state = |s: u32| common::mapped(table, s as i32) as u32;
	let biome = |b: u32| b;
	super::chunk::sections(
		data,
		&super::chunk::Remap {
			state: &state,
			states: common::states(to),
			biome: &biome,
			biomes: None,
		},
		from.index(),
		None,
	)
}

arm!(
	trails_map_chunk_1_20_2_to_1_20_3,
	Client,
	map_chunk,
	old::play_toClient_packet_map_chunk::Ty,
	|mut p, _cx| {
		p.chunkData = chunk_data(OLD, NEW, &p.chunkData)?;
		p
	}
);

arm!(
	trails_map_chunk_1_20_3_to_1_20_2,
	ClientOld,
	map_chunk,
	new::play_toClient_packet_map_chunk::Ty,
	|mut p, _cx| {
		p.chunkData = chunk_data(NEW, OLD, &p.chunkData)?;
		p
	}
);

arm!(
	trails_multi_block_change_1_20_2_to_1_20_3,
	Client,
	multi_block_change,
	old::play_toClient_packet_multi_block_change::Ty,
	|mut p, _cx| {
		p.records = p
			.records
			.into_iter()
			.map(|r| record(OLD, NEW, r as i64) as i32)
			.collect();
		p
	}
);

arm!(
	trails_multi_block_change_1_20_3_to_1_20_2,
	ClientOld,
	multi_block_change,
	new::play_toClient_packet_multi_block_change::Ty,
	|mut p, _cx| {
		p.records = p
			.records
			.into_iter()
			.map(|r| record(NEW, OLD, r as i64) as i32)
			.collect();
		p
	}
);

arm!(
	trails_block_change_1_20_2_to_1_20_3,
	Client,
	block_change,
	old::play_toClient_packet_block_change::Ty,
	|mut p, _cx| {
		p.r#type = block_state(OLD, NEW, p.r#type);
		p
	}
);

arm!(
	trails_block_change_1_20_3_to_1_20_2,
	ClientOld,
	block_change,
	new::play_toClient_packet_block_change::Ty,
	|mut p, _cx| {
		p.r#type = block_state(NEW, OLD, p.r#type);
		p
	}
);

arm!(
	trails_block_action_1_20_2_to_1_20_3,
	Client,
	block_action,
	old::play_toClient_packet_block_action::Ty,
	|mut p, _cx| {
		p.blockId = block(OLD, NEW, p.blockId).ok_or(Error::Dropped)?;
		p
	}
);

arm!(
	trails_block_action_1_20_3_to_1_20_2,
	ClientOld,
	block_action,
	new::play_toClient_packet_block_action::Ty,
	|mut p, _cx| {
		p.blockId = block(NEW, OLD, p.blockId).ok_or(Error::Dropped)?;
		p
	}
);

arm!(
	trails_world_event_1_20_2_to_1_20_3,
	Client,
	world_event,
	old::play_toClient_packet_world_event::Ty,
	|mut p, _cx| {
		p.data = world_event_data(OLD, NEW, p.effectId, p.data);
		p
	}
);

arm!(
	trails_world_event_1_20_3_to_1_20_2,
	ClientOld,
	world_event,
	new::play_toClient_packet_world_event::Ty,
	|mut p, _cx| {
		p.data = world_event_data(NEW, OLD, p.effectId, p.data);
		p
	}
);

macro_rules! spawn_entity {
	($name:ident, $root:ty, $src:ident, $from:expr, $to:expr) => {
		arm!(
			$name,
			$root,
			spawn_entity,
			$src::play_toClient_packet_spawn_entity::Ty,
			|mut p, cx| {
				common::spawned(cx, $from, p.entityId, p.r#type);
				let kind = crate::registry::names($from, "entities")
					.and_then(|n| n.name(p.r#type as u32))
					.unwrap_or_default();
				p.objectData = spawn_object_data($from, $to, kind, p.objectData);
				p.r#type = name_id("entities", $from, $to, p.r#type).ok_or(Error::Dropped)?;
				p
			}
		);
	};
}

spawn_entity!(trails_spawn_entity_1_20_2_to_1_20_3, Client, old, OLD, NEW);
spawn_entity!(
	trails_spawn_entity_1_20_3_to_1_20_2,
	ClientOld,
	new,
	NEW,
	OLD
);

arm!(
	trails_set_cooldown_1_20_2_to_1_20_3,
	Client,
	set_cooldown,
	old::play_toClient_packet_set_cooldown::Ty,
	|mut p, _cx| {
		p.itemID = item(OLD, NEW, p.itemID).ok_or(Error::Dropped)?;
		p
	}
);

arm!(
	trails_set_cooldown_1_20_3_to_1_20_2,
	ClientOld,
	set_cooldown,
	new::play_toClient_packet_set_cooldown::Ty,
	|mut p, _cx| {
		p.itemID = item(NEW, OLD, p.itemID).ok_or(Error::Dropped)?;
		p
	}
);

arm!(
	trails_sound_effect_1_20_2_to_1_20_3,
	Client,
	sound_effect,
	old::play_toClient_packet_sound_effect::Ty,
	|mut p, _cx| {
		p.sound = sound_holder(OLD, NEW, p.sound)?;
		p
	}
);

arm!(
	trails_sound_effect_1_20_3_to_1_20_2,
	ClientOld,
	sound_effect,
	new::play_toClient_packet_sound_effect::Ty,
	|mut p, _cx| {
		p.sound = sound_holder(NEW, OLD, p.sound)?;
		p
	}
);

arm!(
	trails_entity_sound_effect_1_20_2_to_1_20_3,
	Client,
	entity_sound_effect,
	old::play_toClient_packet_entity_sound_effect::Ty,
	|mut p, _cx| {
		p.soundEvent = entity_sound(OLD, NEW, p.soundEvent)?;
		p
	}
);

arm!(
	trails_entity_sound_effect_1_20_3_to_1_20_2,
	ClientOld,
	entity_sound_effect,
	new::play_toClient_packet_entity_sound_effect::Ty,
	|mut p, _cx| {
		p.soundEvent = entity_sound(NEW, OLD, p.soundEvent)?;
		p
	}
);

arm!(
	trails_tags_1_20_2_to_1_20_3,
	Client,
	tags,
	Vec<old::play_toClient_packet__tags__items::Ty>,
	|p, _cx| remap_tags(OLD, NEW, p)
);

arm!(
	trails_tags_1_20_3_to_1_20_2,
	ClientOld,
	tags,
	Vec<new::play_toClient_packet__tags__items::Ty>,
	|p, _cx| remap_tags(NEW, OLD, p)
);

fn config_tags(
	from: Version,
	to: Version,
	mut tags: Vec<old::configuration_toClient_packet__tags__items::Ty>,
) -> Vec<old::configuration_toClient_packet__tags__items::Ty> {
	for registry in &mut tags {
		for t in &mut registry.tags {
			t.entries = tag_entries(&registry.tagType, from, to, std::mem::take(&mut t.entries));
		}
	}
	tags
}

arm!(
	trails_config_tags_1_20_2_to_1_20_3,
	Config,
	tags,
	Vec<old::configuration_toClient_packet__tags__items::Ty>,
	|p, _cx| config_tags(OLD, NEW, p)
);

arm!(
	trails_config_tags_1_20_3_to_1_20_2,
	ConfigOld,
	tags,
	Vec<new::configuration_toClient_packet__tags__items::Ty>,
	|p, _cx| config_tags(NEW, OLD, p)
);
