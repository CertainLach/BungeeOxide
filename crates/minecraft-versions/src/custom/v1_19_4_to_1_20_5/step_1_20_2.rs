use super::common::{self, block_state, decode, encode, meta_key, ParticleMap};
use super::step_1_20::{
	entity_sound, optional_state, record, sound_holder, spawn_object_data, world_event_data,
};
use crate::gen::convert::{v1_20 as old, v1_20_2 as new};
use crate::gen::convert::{Root, Version};
use crate::pds::convert::{unconvertible, Error, Result};
use crate::Session;
use nbt::{CompoundExt, CompoundVecExt, Entry, List, Payload, Tag};

const OLD: Version = Version::V1_20;
const NEW: Version = Version::V1_20_2;

type Client = new::play_toClient_packet::Ty;
type ClientOld = old::play_toClient_packet::Ty;

const META_RENAMES: &[(&str, &str)] = &[
	(
		"interpolation_start_delta_ticks",
		"transformation_interpolation_start_delta_ticks",
	),
	(
		"interpolation_duration",
		"transformation_interpolation_duration",
	),
];

const EFFECT_RENAMES: &[(&str, &str)] = &[
	("Amplifier", "amplifier"),
	("Duration", "duration"),
	("Ambient", "ambient"),
	("ShowParticles", "show_particles"),
	("ShowIcon", "show_icon"),
	("HiddenEffect", "hidden_effect"),
	("FactorCalculationData", "factor_calculation_data"),
];

pub fn trails_nbt_anonymous_1_20_to_1_20_2(v: Entry, _cx: &mut Session) -> Result<Tag> {
	Ok(Tag::from(v))
}

pub fn trails_nbt_named_1_20_2_to_1_20(v: Tag, _cx: &mut Session) -> Result<Entry> {
	Ok(v.named(""))
}

pub fn trails_optional_nbt_anonymous_1_20_to_1_20_2(
	v: Option<Entry>,
	_cx: &mut Session,
) -> Result<Option<Tag>> {
	Ok(v.map(Tag::from))
}

pub fn trails_optional_nbt_named_1_20_2_to_1_20(
	v: Option<Tag>,
	_cx: &mut Session,
) -> Result<Option<Entry>> {
	Ok(v.map(Entry::from))
}

fn effects(c: &mut Vec<Entry>, up: bool) {
	let (from, to) = if up {
		("CustomPotionEffects", "custom_potion_effects")
	} else {
		("custom_potion_effects", "CustomPotionEffects")
	};
	let Some(Payload::list(l)) = c.take(from) else {
		return;
	};
	let converted = l
		.values()
		.into_iter()
		.map(|e| {
			let Payload::compound(mut e) = e else {
				return e;
			};
			if up {
				if let Some(name) = e
					.take("Id")
					.as_ref()
					.and_then(Payload::to_i32)
					.and_then(|id| common::effect_name(OLD, id))
				{
					e.push(Entry::new("id", Payload::string(name)));
				}
			} else if let Some(id) = e
				.take("id")
				.as_ref()
				.and_then(Payload::as_str)
				.and_then(|k| common::effect_by_key(OLD, k))
			{
				e.push(Entry::new("Id", Payload::int(id)));
			}
			for (a, b) in EFFECT_RENAMES {
				if up {
					e.rename(a, b)
				} else {
					e.rename(b, a)
				}
			}
			Payload::compound(e)
		})
		.collect();
	c.push(Entry::new(to, List::of(converted)));
}

fn item_nbt(t: &mut Tag, up: bool) {
	if let Payload::compound(c) = &mut t.value {
		effects(c, up);
	}
}

pub fn trails_slot_1_20_to_1_20_2(v: old::slot::Ty, _cx: &mut Session) -> Result<new::slot::Ty> {
	Ok(match v {
		old::slot::Ty::r#false => new::slot::Ty::r#false,
		old::slot::Ty::r#true(s) => new::slot::Ty::r#true(new::slot__true::Ty {
			itemId: s.itemId,
			itemCount: s.itemCount,
			nbtData: s.nbtData.map(|e| {
				let mut t = Tag::from(e);
				item_nbt(&mut t, true);
				t
			}),
		}),
	})
}

pub fn trails_slot_1_20_2_to_1_20(v: new::slot::Ty, _cx: &mut Session) -> Result<old::slot::Ty> {
	Ok(match v {
		new::slot::Ty::r#false => old::slot::Ty::r#false,
		new::slot::Ty::r#true(s) => old::slot::Ty::r#true(old::slot__true::Ty {
			itemId: s.itemId,
			itemCount: s.itemCount,
			nbtData: s.nbtData.map(|mut t| {
				item_nbt(&mut t, false);
				t.named("")
			}),
		}),
	})
}

fn slot_bytes_up(b: &[u8]) -> Result<Vec<u8>> {
	let s = decode(b, |i| old::slot::deserialize(i))?;
	encode(
		&trails_slot_1_20_to_1_20_2(s, &mut Session::new())?,
		new::slot::serialize,
	)
}

fn slot_bytes_down(b: &[u8]) -> Result<Vec<u8>> {
	let s = decode(b, |i| new::slot::deserialize(i))?;
	encode(
		&trails_slot_1_20_2_to_1_20(s, &mut Session::new())?,
		old::slot::serialize,
	)
}

macro_rules! particle {
	($name:ident, $src:ident, $dst:ident, $from:expr, $to:expr, $item:expr) => {
		fn $name(v: $src::particle::Ty) -> Result<$dst::particle::Ty> {
			let bytes = encode(&v, $src::particle::serialize)?;
			let (id, payload) = common::read_varint(&bytes)?;
			let (id, payload) = common::particle(
				&ParticleMap {
					from: $from,
					to: $to,
					item: &$item,
				},
				id,
				payload,
			)?;
			let mut out = Vec::new();
			common::varint(&mut out, id);
			out.extend(payload);
			decode(&out, |i| $dst::particle::deserialize(i))
		}
	};
}

particle!(particle_1_20_to_1_20_2, old, new, OLD, NEW, slot_bytes_up);
particle!(particle_1_20_2_to_1_20, new, old, NEW, OLD, slot_bytes_down);

macro_rules! metadata {
	($name:ident, $src:ident, $dst:ident, $from:expr, $to:expr, $particle:ident, $slot:ident, $nbt:expr) => {
		fn $name(
			entity: i32,
			v: $src::entityMetadata::Ty,
			cx: &mut Session,
		) -> Result<$dst::entityMetadata::Ty> {
			use $dst::entityMetadataItem::Ty as N;
			use $src::entityMetadataItem::Ty as V;
			let mut out = Vec::with_capacity(v.len());
			for e in v {
				let Some(key) = meta_key(cx, $from, $to, entity, e.key, META_RENAMES) else {
					continue;
				};
				let ty = decode(
					&encode(&e.r#type, $src::entityMetadata__items__type::serialize)?,
					|i| $dst::entityMetadata__items__type::deserialize(i),
				)?;
				let value = match e.value {
					V::particle(p) => match $particle(p) {
						Ok(p) => N::particle(p),
						Err(Error::Dropped) => continue,
						Err(e) => return Err(e),
					},
					V::block_state(s) => N::block_state(block_state($from, $to, s)),
					V::optional_block_state(s) => {
						N::optional_block_state(optional_state($from, $to, s))
					}
					V::item_stack(s) => N::item_stack($slot(s, cx)?),
					V::compound_tag(t) => N::compound_tag($nbt(t)),
					v => decode(&encode(&v, $src::entityMetadataItem::serialize)?, |i| {
						$dst::entityMetadataItem::deserialize(
							i,
							$dst::entityMetadataItem::Args { compareTo: &ty },
						)
					})?,
				};
				out.push($dst::entityMetadata__items::Ty {
					key,
					r#type: ty,
					value,
				});
			}
			Ok(out)
		}
	};
}

metadata!(
	metadata_up,
	old,
	new,
	OLD,
	NEW,
	particle_1_20_to_1_20_2,
	trails_slot_1_20_to_1_20_2,
	Tag::from
);
metadata!(
	metadata_down,
	new,
	old,
	NEW,
	OLD,
	particle_1_20_2_to_1_20,
	trails_slot_1_20_2_to_1_20,
	Entry::from
);

arm!(
	trails_entity_metadata_1_20_to_1_20_2,
	Client,
	entity_metadata,
	old::play_toClient_packet_entity_metadata::Ty,
	|p, cx| {
		new::play_toClient_packet_entity_metadata::Ty {
			entityId: p.entityId,
			metadata: metadata_up(p.entityId, p.metadata, cx)?,
		}
	}
);

arm!(
	trails_entity_metadata_1_20_2_to_1_20,
	ClientOld,
	entity_metadata,
	new::play_toClient_packet_entity_metadata::Ty,
	|p, cx| {
		old::play_toClient_packet_entity_metadata::Ty {
			entityId: p.entityId,
			metadata: metadata_down(p.entityId, p.metadata, cx)?,
		}
	}
);

arm!(
	trails_login_1_20_to_1_20_2,
	Client,
	login,
	old::play_toClient_packet_login::Ty,
	|p, cx| {
		let s = common::state(cx);
		s.player = Some(p.entityId);
		s.codec = Some(Tag::from(p.dimensionCodec));
		new::play_toClient_packet_login::Ty {
			entityId: p.entityId,
			isHardcore: p.isHardcore,
			worldNames: p.worldNames,
			maxPlayers: p.maxPlayers,
			viewDistance: p.viewDistance,
			simulationDistance: p.simulationDistance,
			reducedDebugInfo: p.reducedDebugInfo,
			enableRespawnScreen: p.enableRespawnScreen,
			doLimitedCrafting: false,
			worldType: p.worldType,
			worldName: p.worldName,
			hashedSeed: p.hashedSeed,
			gameMode: p.gameMode,
			previousGameMode: p.previousGameMode,
			isDebug: p.isDebug,
			isFlat: p.isFlat,
			death: p
				.death
				.map(|d| new::play_toClient_packet_login__death__items::Ty {
					dimensionName: d.dimensionName,
					location: d.location,
				}),
			portalCooldown: p.portalCooldown,
		}
	}
);

pub fn trails_login_1_20_2_to_1_20(
	p: new::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let s = common::state(cx);
	s.player = Some(p.entityId);
	let codec = s.codec.clone();
	let features = s.features.clone();
	let tags = s.tags.clone();
	let payloads = std::mem::take(&mut s.payloads);
	let pack = s.resource_pack.take();
	let codec = match codec {
		Some(c) => c,
		None => super::codec::vanilla(OLD)?,
	};
	let mut out = vec![ClientOld::login(old::play_toClient_packet_login::Ty {
		entityId: p.entityId,
		isHardcore: p.isHardcore,
		gameMode: p.gameMode,
		previousGameMode: p.previousGameMode,
		worldNames: p.worldNames,
		dimensionCodec: codec.named(""),
		worldType: p.worldType,
		worldName: p.worldName,
		hashedSeed: p.hashedSeed,
		maxPlayers: p.maxPlayers,
		viewDistance: p.viewDistance,
		simulationDistance: p.simulationDistance,
		reducedDebugInfo: p.reducedDebugInfo,
		enableRespawnScreen: p.enableRespawnScreen,
		isDebug: p.isDebug,
		isFlat: p.isFlat,
		death: p
			.death
			.map(|d| old::play_toClient_packet_login__death__items::Ty {
				dimensionName: d.dimensionName,
				location: d.location,
			}),
		portalCooldown: p.portalCooldown,
	})];
	out.push(ClientOld::feature_flags(
		features.unwrap_or_else(|| vec!["minecraft:vanilla".to_owned()]),
	));
	if let Some(tags) = tags {
		let tags = tags
			.into_iter()
			.map(|t| old::play_toClient_packet__tags__items::Ty {
				tagType: t.tagType,
				tags: t.tags,
			})
			.collect();
		out.push(ClientOld::tags(super::step_1_20::remap_tags(
			NEW, OLD, tags,
		)));
	}
	for (channel, data) in payloads {
		out.push(ClientOld::custom_payload(
			old::play_toClient_packet_custom_payload::Ty { channel, data },
		));
	}
	if let Some((url, hash, forced, promptMessage)) = pack {
		out.push(ClientOld::resource_pack_send(
			old::play_toClient_packet_resource_pack_send::Ty {
				url,
				hash,
				forced,
				promptMessage,
			},
		));
	}
	Ok(out)
}

pub fn trails_chunk_batch_finished_1_20_2_to_1_20(
	_size: i32,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	common::reply(
		cx,
		Root::PlayToServer,
		NEW,
		&new::play_toServer_packet::Ty::chunk_batch_received(64.0),
		new::play_toServer_packet::serialize,
	)?;
	Ok(vec![])
}

pub fn trails_start_configuration_1_20_2_to_1_20(
	_p: new::play_toClient_packet_start_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let ack = new::play_toServer_packet::Ty::configuration_acknowledged(
		new::play_toServer_packet_configuration_acknowledged::Ty {},
	);
	common::reply(
		cx,
		Root::PlayToServer,
		NEW,
		&ack,
		new::play_toServer_packet::serialize,
	)?;
	Ok(vec![])
}

fn player_id(version: Version) -> Result<i32> {
	match crate::registry::names(version, "entities").and_then(|n| n.id("player")) {
		Some(id) => Ok(id as i32),
		None => unconvertible(format!("no player entity type in {}", version.name())),
	}
}

pub fn trails_named_entity_spawn_1_20_to_1_20_2(
	p: old::play_toClient_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	common::state(cx)
		.entities
		.insert(p.entityId, "player".to_owned());
	Ok(vec![Client::spawn_entity(
		new::play_toClient_packet_spawn_entity::Ty {
			entityId: p.entityId,
			objectUUID: p.playerUUID,
			r#type: player_id(NEW)?,
			x: p.x,
			y: p.y,
			z: p.z,
			pitch: p.pitch,
			yaw: p.yaw,
			headPitch: p.yaw,
			objectData: 0,
			velocity: new::vec3i16::Ty { x: 0, y: 0, z: 0 },
		},
	)])
}

pub fn trails_spawn_entity_1_20_to_1_20_2(
	mut p: old::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	common::spawned(cx, OLD, p.entityId, p.r#type);
	let kind = crate::registry::names(OLD, "entities")
		.and_then(|n| n.name(p.r#type as u32))
		.unwrap_or_default();
	p.objectData = spawn_object_data(OLD, NEW, kind, p.objectData);
	Ok(vec![Client::spawn_entity(p)])
}

pub fn trails_spawn_entity_1_20_2_to_1_20(
	mut p: new::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	common::spawned(cx, NEW, p.entityId, p.r#type);
	if p.r#type == player_id(NEW)? {
		return Ok(vec![ClientOld::named_entity_spawn(
			old::play_toClient_packet_named_entity_spawn::Ty {
				entityId: p.entityId,
				playerUUID: p.objectUUID,
				x: p.x,
				y: p.y,
				z: p.z,
				yaw: p.yaw,
				pitch: p.pitch,
			},
		)]);
	}
	let kind = crate::registry::names(NEW, "entities")
		.and_then(|n| n.name(p.r#type as u32))
		.unwrap_or_default();
	p.objectData = spawn_object_data(NEW, OLD, kind, p.objectData);
	Ok(vec![ClientOld::spawn_entity(p)])
}

fn chunk_data(from: Version, to: Version, data: &[u8]) -> Result<Vec<u8>> {
	let state = |s: u32| block_state(from, to, s as i32) as u32;
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
	trails_map_chunk_1_20_to_1_20_2,
	Client,
	map_chunk,
	old::play_toClient_packet_map_chunk::Ty,
	|p, _cx| {
		new::play_toClient_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			heightmaps: Tag::from(p.heightmaps),
			chunkData: chunk_data(OLD, NEW, &p.chunkData)?,
			blockEntities: p
				.blockEntities
				.into_iter()
				.map(|b| new::chunkBlockEntity::Ty {
					ANON: new::chunkBlockEntity__ANON::Ty {
						x: b.ANON.x,
						z: b.ANON.z,
					},
					y: b.y,
					r#type: b.r#type,
					nbtData: b.nbtData.map(Tag::from),
				})
				.collect(),
			skyLightMask: p.skyLightMask,
			blockLightMask: p.blockLightMask,
			emptySkyLightMask: p.emptySkyLightMask,
			emptyBlockLightMask: p.emptyBlockLightMask,
			skyLight: p.skyLight,
			blockLight: p.blockLight,
		}
	}
);

arm!(
	trails_map_chunk_1_20_2_to_1_20,
	ClientOld,
	map_chunk,
	new::play_toClient_packet_map_chunk::Ty,
	|p, _cx| {
		old::play_toClient_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			heightmaps: p.heightmaps.named(""),
			chunkData: chunk_data(NEW, OLD, &p.chunkData)?,
			blockEntities: p
				.blockEntities
				.into_iter()
				.map(|b| old::chunkBlockEntity::Ty {
					ANON: old::chunkBlockEntity__ANON::Ty {
						x: b.ANON.x,
						z: b.ANON.z,
					},
					y: b.y,
					r#type: b.r#type,
					nbtData: b.nbtData.map(Entry::from),
				})
				.collect(),
			skyLightMask: p.skyLightMask,
			blockLightMask: p.blockLightMask,
			emptySkyLightMask: p.emptySkyLightMask,
			emptyBlockLightMask: p.emptyBlockLightMask,
			skyLight: p.skyLight,
			blockLight: p.blockLight,
		}
	}
);

arm!(
	trails_multi_block_change_1_20_to_1_20_2,
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
	trails_multi_block_change_1_20_2_to_1_20,
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
	trails_block_change_1_20_to_1_20_2,
	Client,
	block_change,
	old::play_toClient_packet_block_change::Ty,
	|mut p, _cx| {
		p.r#type = block_state(OLD, NEW, p.r#type);
		p
	}
);

arm!(
	trails_block_change_1_20_2_to_1_20,
	ClientOld,
	block_change,
	new::play_toClient_packet_block_change::Ty,
	|mut p, _cx| {
		p.r#type = block_state(NEW, OLD, p.r#type);
		p
	}
);

arm!(
	trails_world_event_1_20_to_1_20_2,
	Client,
	world_event,
	old::play_toClient_packet_world_event::Ty,
	|mut p, _cx| {
		p.data = world_event_data(OLD, NEW, p.effectId, p.data);
		p
	}
);

arm!(
	trails_world_event_1_20_2_to_1_20,
	ClientOld,
	world_event,
	new::play_toClient_packet_world_event::Ty,
	|mut p, _cx| {
		p.data = world_event_data(NEW, OLD, p.effectId, p.data);
		p
	}
);

macro_rules! world_particles {
	($name:ident, $root:ty, $src:ident, $dst:ident, $from:expr, $to:expr, $item:expr) => {
		arm!(
			$name,
			$root,
			world_particles,
			$src::play_toClient_packet_world_particles::Ty,
			|p, _cx| {
				let payload = encode(&p.data, $src::particleData::serialize)?;
				let (id, payload) = common::particle(
					&ParticleMap {
						from: $from,
						to: $to,
						item: &$item,
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
	trails_world_particles_1_20_to_1_20_2,
	Client,
	old,
	new,
	OLD,
	NEW,
	slot_bytes_up
);
world_particles!(
	trails_world_particles_1_20_2_to_1_20,
	ClientOld,
	new,
	old,
	NEW,
	OLD,
	slot_bytes_down
);

arm!(
	trails_sound_effect_1_20_to_1_20_2,
	Client,
	sound_effect,
	old::play_toClient_packet_sound_effect::Ty,
	|mut p, _cx| {
		p.sound = sound_holder(OLD, NEW, p.sound)?;
		p
	}
);

arm!(
	trails_sound_effect_1_20_2_to_1_20,
	ClientOld,
	sound_effect,
	new::play_toClient_packet_sound_effect::Ty,
	|mut p, _cx| {
		p.sound = sound_holder(NEW, OLD, p.sound)?;
		p
	}
);

arm!(
	trails_entity_sound_effect_1_20_to_1_20_2,
	Client,
	entity_sound_effect,
	old::play_toClient_packet_entity_sound_effect::Ty,
	|mut p, _cx| {
		p.soundEvent = entity_sound(OLD, NEW, p.soundEvent)?;
		p
	}
);

arm!(
	trails_entity_sound_effect_1_20_2_to_1_20,
	ClientOld,
	entity_sound_effect,
	new::play_toClient_packet_entity_sound_effect::Ty,
	|mut p, _cx| {
		p.soundEvent = entity_sound(NEW, OLD, p.soundEvent)?;
		p
	}
);

pub(super) fn effect_1_20_to_1_20_2(id: i32) -> Result<i32> {
	common::name_id("effects", OLD, NEW, id).ok_or(Error::Dropped)
}

pub(super) fn effect_1_20_2_to_1_20(id: i32) -> Result<i32> {
	common::name_id("effects", NEW, OLD, id).ok_or(Error::Dropped)
}

pub fn trails_entity_effect_id_1_20_to_1_20_2(
	src: &old::play_toClient_packet_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	effect_1_20_to_1_20_2(src.effectId)
}

pub fn trails_entity_effect_id_1_20_2_to_1_20(
	src: &new::play_toClient_packet_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	effect_1_20_2_to_1_20(src.effectId)
}

pub fn trails_remove_effect_id_1_20_to_1_20_2(
	src: &old::play_toClient_packet_remove_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	effect_1_20_to_1_20_2(src.effectId)
}

pub fn trails_remove_effect_id_1_20_2_to_1_20(
	src: &new::play_toClient_packet_remove_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	effect_1_20_2_to_1_20(src.effectId)
}

pub fn trails_beacon_primary_1_20_to_1_20_2(
	src: &old::play_toServer_packet_set_beacon_effect::Ty,
	_cx: &mut Session,
) -> Result<Option<i32>> {
	src.primary_effect.map(effect_1_20_to_1_20_2).transpose()
}

pub fn trails_beacon_primary_1_20_2_to_1_20(
	src: &new::play_toServer_packet_set_beacon_effect::Ty,
	_cx: &mut Session,
) -> Result<Option<i32>> {
	src.primary_effect.map(effect_1_20_2_to_1_20).transpose()
}

pub fn trails_beacon_secondary_1_20_to_1_20_2(
	src: &old::play_toServer_packet_set_beacon_effect::Ty,
	_cx: &mut Session,
) -> Result<Option<i32>> {
	src.secondary_effect.map(effect_1_20_to_1_20_2).transpose()
}

pub fn trails_beacon_secondary_1_20_2_to_1_20(
	src: &new::play_toServer_packet_set_beacon_effect::Ty,
	_cx: &mut Session,
) -> Result<Option<i32>> {
	src.secondary_effect.map(effect_1_20_2_to_1_20).transpose()
}

pub fn trails_criteria_1_20_2_to_1_20(
	src: &new::play_toClient_packet_advancements__advancementMapping__items__value::Ty,
	_cx: &mut Session,
) -> Result<Vec<old::play_toClient_packet_advancements__advancementMapping__items__value__criteria__items::Ty>>{
	let mut keys: Vec<String> = src.requirements.iter().flatten().cloned().collect();
	keys.sort();
	keys.dedup();
	Ok(keys
		.into_iter()
		.map(|key| old::play_toClient_packet_advancements__advancementMapping__items__value__criteria__items::Ty { key, value: () })
		.collect())
}

pub fn trails_login_uuid_1_20_to_1_20_2(
	src: &old::login_toServer_packet_login_start::Ty,
	_cx: &mut Session,
) -> Result<u128> {
	Ok(src
		.playerUUID
		.unwrap_or_else(|| common::offline_uuid(&src.username)))
}

pub fn trails_login_uuid_1_20_2_to_1_20(
	src: &new::login_toServer_packet_login_start::Ty,
	_cx: &mut Session,
) -> Result<Option<u128>> {
	Ok(Some(src.playerUUID))
}

const CLIMBABLE: &[&str] = &[
	"ladder",
	"vine",
	"scaffolding",
	"weeping_vines",
	"weeping_vines_plant",
	"twisting_vines",
	"twisting_vines_plant",
	"cave_vines",
	"cave_vines_plant",
];

fn block_tag(
	name: &str,
	pick: impl Fn(&str) -> bool,
) -> new::configuration_toClient_packet__tags__items__tags__items::Ty {
	let blocks = crate::registry::blocks(NEW);
	let entries = blocks
		.map(|b| {
			b.list
				.iter()
				.filter(|x| pick(&x.name))
				.map(|x| x.id as i32)
				.collect()
		})
		.unwrap_or_default();
	new::configuration_toClient_packet__tags__items__tags__items::Ty {
		tagName: format!("minecraft:{name}"),
		entries,
	}
}

fn data_tags(registry: &str) -> Vec<(String, Vec<String>)> {
	let folder = crate::registry::file("versions.tsv").and_then(|t| {
		t.lines().find_map(|l| {
			let mut p = l.split('\t');
			(p.next() == Some(NEW.name()) && p.next() == Some("tags"))
				.then(|| p.next())
				.flatten()
		})
	});
	let json = folder
		.and_then(|f| crate::registry::file(&format!("{f}/tags.json")))
		.and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok());
	let Some(tags) = json.as_ref().and_then(|j| j[registry].as_object()) else {
		return vec![];
	};
	tags.iter()
		.map(|(name, list)| {
			(
				name.clone(),
				list.as_array()
					.map(|a| {
						a.iter()
							.filter_map(|v| v.as_str().map(str::to_owned))
							.collect()
					})
					.unwrap_or_default(),
			)
		})
		.collect()
}

fn physics_tags() -> Vec<new::configuration_toClient_packet__tags__items::Ty> {
	use new::configuration_toClient_packet__tags__items::Ty as Registry;
	use new::configuration_toClient_packet__tags__items__tags__items::Ty as T;
	let mut blocks = vec![
		block_tag("climbable", |n| CLIMBABLE.contains(&n)),
		block_tag("trapdoors", |n| n.ends_with("_trapdoor")),
		block_tag("fences", |n| n.ends_with("_fence")),
		block_tag("walls", |n| n.ends_with("_wall")),
		block_tag("fence_gates", |n| n.ends_with("_fence_gate")),
		block_tag("infiniburn_overworld", |n| {
			matches!(n, "netherrack" | "magma_block")
		}),
		block_tag("infiniburn_nether", |n| {
			matches!(n, "netherrack" | "magma_block")
		}),
		block_tag("infiniburn_end", |n| {
			matches!(n, "netherrack" | "magma_block" | "bedrock")
		}),
	];
	for (name, members) in data_tags("block") {
		blocks.push(block_tag(&name, |n| members.iter().any(|m| m == n)));
	}
	vec![
		Registry {
			tagType: "minecraft:fluid".to_owned(),
			tags: vec![
				T {
					tagName: "minecraft:water".to_owned(),
					entries: vec![2, 1],
				},
				T {
					tagName: "minecraft:lava".to_owned(),
					entries: vec![4, 3],
				},
			],
		},
		Registry {
			tagType: "minecraft:block".to_owned(),
			tags: blocks,
		},
		Registry {
			tagType: "minecraft:worldgen/biome".to_owned(),
			tags: ["is_badlands", "is_jungle", "is_savanna"]
				.iter()
				.map(|n| T {
					tagName: format!("minecraft:{n}"),
					entries: vec![],
				})
				.collect(),
		},
	]
}

pub fn configuration_capture_1_20_2(cx: &mut Session, packet: &[u8]) -> Result<Vec<Vec<u8>>> {
	use new::configuration_toClient_packet::Ty as P;
	use new::configuration_toServer_packet::Ty as R;
	let s = common::state(cx);
	let replies = match decode(packet, |i| {
		new::configuration_toClient_packet::deserialize(i)
	})? {
		P::registry_data(codec) => {
			s.codec = Some(codec);
			vec![]
		}
		P::feature_flags(f) => {
			s.features = Some(f);
			vec![]
		}
		P::tags(t) => {
			s.tags = Some(t);
			vec![]
		}
		P::custom_payload(p) => {
			s.payloads.push((p.channel, p.data));
			vec![]
		}
		P::resource_pack_send(p) => {
			s.resource_pack = Some((p.url, p.hash, p.forced, p.promptMessage));
			vec![R::resource_pack_receive(3), R::resource_pack_receive(0)]
		}
		P::keep_alive(id) => vec![R::keep_alive(id)],
		P::ping(id) => vec![R::pong(id)],
		P::finish_configuration(_) => vec![R::finish_configuration(
			new::configuration_toServer_packet_finish_configuration::Ty {},
		)],
		P::disconnect(_) | P::default(_) => vec![],
	};
	replies
		.iter()
		.map(|r| encode(r, new::configuration_toServer_packet::serialize))
		.collect()
}

pub fn configuration_1_20_2(cx: &mut Session) -> Result<Vec<Vec<u8>>> {
	use new::configuration_toClient_packet::Ty as P;
	let codec = match common::state(cx).codec.clone() {
		Some(c) => c,
		None => super::codec::vanilla(NEW)?,
	};
	[
		P::registry_data(codec),
		P::feature_flags(vec!["minecraft:vanilla".to_owned()]),
		P::tags(physics_tags()),
		P::finish_configuration(new::configuration_toClient_packet_finish_configuration::Ty {}),
	]
	.iter()
	.map(|p| encode(p, new::configuration_toClient_packet::serialize))
	.collect()
}
