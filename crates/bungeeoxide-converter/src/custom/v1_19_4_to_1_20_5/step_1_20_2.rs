use super::common::{self, block_state, decode, encode, meta_key, ParticleMap};
use super::step_1_20::{
	entity_sound, optional_state, record, sound_holder, spawn_object_data, world_event_data,
};
use crate::gen::convert::{v1_20 as old, v1_20_2 as new};
use crate::gen::convert::{Root, Version};
use crate::pds::convert::{unconvertible, Error, Result};
use crate::Session;
use bungeeoxide_nbt::{CompoundExt, CompoundVecExt, Entry, List, Payload, Tag};

const OLD: Version = Version::V1_20;
const NEW: Version = Version::V1_20_2;

type Client = new::play_to_client_packet::Ty;
type ClientOld = old::play_to_client_packet::Ty;

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
	let Some(Payload::List(l)) = c.take(from) else {
		return;
	};
	let converted = l
		.values()
		.into_iter()
		.map(|e| {
			let Payload::Compound(mut e) = e else {
				return e;
			};
			if up {
				if let Some(name) = e
					.take("Id")
					.as_ref()
					.and_then(Payload::to_i32)
					.and_then(|id| common::effect_name(OLD, id))
				{
					e.push(Entry::new("id", Payload::String(name)));
				}
			} else if let Some(id) = e
				.take("id")
				.as_ref()
				.and_then(Payload::as_str)
				.and_then(|k| common::effect_by_key(OLD, k))
			{
				e.push(Entry::new("Id", Payload::Int(id)));
			}
			for (a, b) in EFFECT_RENAMES {
				if up {
					e.rename(a, b)
				} else {
					e.rename(b, a)
				}
			}
			Payload::Compound(e)
		})
		.collect();
	c.push(Entry::new(to, List::of(converted)));
}

fn item_nbt(t: &mut Tag, up: bool) {
	if let Payload::Compound(c) = &mut t.value {
		effects(c, up);
	}
}

pub fn trails_slot_1_20_to_1_20_2(v: old::slot::Ty, _cx: &mut Session) -> Result<new::slot::Ty> {
	Ok(match v {
		old::slot::Ty::False => new::slot::Ty::False,
		old::slot::Ty::True(s) => new::slot::Ty::True(new::slot_true::Ty {
			item_id: s.item_id,
			item_count: s.item_count,
			nbt_data: s.nbt_data.map(|e| {
				let mut t = Tag::from(e);
				item_nbt(&mut t, true);
				t
			}),
		}),
	})
}

pub fn trails_slot_1_20_2_to_1_20(v: new::slot::Ty, _cx: &mut Session) -> Result<old::slot::Ty> {
	Ok(match v {
		new::slot::Ty::False => old::slot::Ty::False,
		new::slot::Ty::True(s) => old::slot::Ty::True(old::slot_true::Ty {
			item_id: s.item_id,
			item_count: s.item_count,
			nbt_data: s.nbt_data.map(|mut t| {
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
			v: $src::entity_metadata::Ty,
			cx: &mut Session,
		) -> Result<$dst::entity_metadata::Ty> {
			use $dst::entity_metadata_item::Ty as N;
			use $src::entity_metadata_item::Ty as V;
			let mut out = Vec::with_capacity(v.len());
			for e in v {
				let Some(key) = meta_key(cx, $from, $to, entity, e.key, META_RENAMES) else {
					continue;
				};
				let ty = decode(
					&encode(&e.r#type, $src::entity_metadata_items_type::serialize)?,
					|i| $dst::entity_metadata_items_type::deserialize(i),
				)?;
				let value = match e.value {
					V::Particle(p) => match $particle(p) {
						Ok(p) => N::Particle(p),
						Err(Error::Dropped) => continue,
						Err(e) => return Err(e),
					},
					V::BlockState(s) => N::BlockState(block_state($from, $to, s)),
					V::OptionalBlockState(s) => {
						N::OptionalBlockState(optional_state($from, $to, s))
					}
					V::ItemStack(s) => N::ItemStack($slot(s, cx)?),
					V::CompoundTag(t) => N::CompoundTag($nbt(t)),
					v => decode(&encode(&v, $src::entity_metadata_item::serialize)?, |i| {
						$dst::entity_metadata_item::deserialize(
							i,
							$dst::entity_metadata_item::Args { compare_to: &ty },
						)
					})?,
				};
				out.push($dst::entity_metadata_items::Ty {
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
	EntityMetadata,
	old::play_to_client_packet_entity_metadata::Ty,
	|p, cx| {
		new::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
			metadata: metadata_up(p.entity_id, p.metadata, cx)?,
		}
	}
);

arm!(
	trails_entity_metadata_1_20_2_to_1_20,
	ClientOld,
	EntityMetadata,
	new::play_to_client_packet_entity_metadata::Ty,
	|p, cx| {
		old::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
			metadata: metadata_down(p.entity_id, p.metadata, cx)?,
		}
	}
);

arm!(
	trails_login_1_20_to_1_20_2,
	Client,
	Login,
	old::play_to_client_packet_login::Ty,
	|p, cx| {
		let s = common::state(cx);
		s.player = Some(p.entity_id);
		s.codec = Some(Tag::from(p.dimension_codec));
		new::play_to_client_packet_login::Ty {
			entity_id: p.entity_id,
			is_hardcore: p.is_hardcore,
			world_names: p.world_names,
			max_players: p.max_players,
			view_distance: p.view_distance,
			simulation_distance: p.simulation_distance,
			reduced_debug_info: p.reduced_debug_info,
			enable_respawn_screen: p.enable_respawn_screen,
			do_limited_crafting: false,
			world_type: p.world_type,
			world_name: p.world_name,
			hashed_seed: p.hashed_seed,
			game_mode: p.game_mode,
			previous_game_mode: p.previous_game_mode,
			is_debug: p.is_debug,
			is_flat: p.is_flat,
			death: p
				.death
				.map(|d| new::play_to_client_packet_login_death_items::Ty {
					dimension_name: d.dimension_name,
					location: d.location,
				}),
			portal_cooldown: p.portal_cooldown,
		}
	}
);

pub fn trails_login_1_20_2_to_1_20(
	p: new::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let s = common::state(cx);
	s.player = Some(p.entity_id);
	let codec = s.codec.clone();
	let features = s.features.clone();
	let tags = s.tags.clone();
	let payloads = std::mem::take(&mut s.payloads);
	let pack = s.resource_pack.take();
	let codec = match codec {
		Some(c) => c,
		None => super::codec::vanilla(OLD)?,
	};
	let mut out = vec![ClientOld::Login(old::play_to_client_packet_login::Ty {
		entity_id: p.entity_id,
		is_hardcore: p.is_hardcore,
		game_mode: p.game_mode,
		previous_game_mode: p.previous_game_mode,
		world_names: p.world_names,
		dimension_codec: codec.named(""),
		world_type: p.world_type,
		world_name: p.world_name,
		hashed_seed: p.hashed_seed,
		max_players: p.max_players,
		view_distance: p.view_distance,
		simulation_distance: p.simulation_distance,
		reduced_debug_info: p.reduced_debug_info,
		enable_respawn_screen: p.enable_respawn_screen,
		is_debug: p.is_debug,
		is_flat: p.is_flat,
		death: p
			.death
			.map(|d| old::play_to_client_packet_login_death_items::Ty {
				dimension_name: d.dimension_name,
				location: d.location,
			}),
		portal_cooldown: p.portal_cooldown,
	})];
	out.push(ClientOld::FeatureFlags(
		features.unwrap_or_else(|| vec!["minecraft:vanilla".to_owned()]),
	));
	if let Some(tags) = tags {
		let tags = tags
			.into_iter()
			.map(|t| old::play_to_client_packet_tags_items::Ty {
				tag_type: t.tag_type,
				tags: t.tags,
			})
			.collect();
		out.push(ClientOld::Tags(super::step_1_20::remap_tags(
			NEW, OLD, tags,
		)));
	}
	for (channel, data) in payloads {
		out.push(ClientOld::CustomPayload(
			old::play_to_client_packet_custom_payload::Ty { channel, data },
		));
	}
	if let Some((url, hash, forced, prompt_message)) = pack {
		out.push(ClientOld::ResourcePackSend(
			old::play_to_client_packet_resource_pack_send::Ty {
				url,
				hash,
				forced,
				prompt_message,
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
		&new::play_to_server_packet::Ty::ChunkBatchReceived(64.0),
		new::play_to_server_packet::serialize,
	)?;
	Ok(vec![])
}

pub fn trails_start_configuration_1_20_2_to_1_20(
	_p: new::play_to_client_packet_start_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let ack = new::play_to_server_packet::Ty::ConfigurationAcknowledged(
		new::play_to_server_packet_configuration_acknowledged::Ty {},
	);
	common::reply(
		cx,
		Root::PlayToServer,
		NEW,
		&ack,
		new::play_to_server_packet::serialize,
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
	p: old::play_to_client_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	common::state(cx)
		.entities
		.insert(p.entity_id, "player".to_owned());
	Ok(vec![Client::SpawnEntity(
		new::play_to_client_packet_spawn_entity::Ty {
			entity_id: p.entity_id,
			object_uuid: p.player_uuid,
			r#type: player_id(NEW)?,
			x: p.x,
			y: p.y,
			z: p.z,
			pitch: p.pitch,
			yaw: p.yaw,
			head_pitch: p.yaw,
			object_data: 0,
			velocity: new::vec3i16::Ty { x: 0, y: 0, z: 0 },
		},
	)])
}

pub fn trails_spawn_entity_1_20_to_1_20_2(
	mut p: old::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	common::spawned(cx, OLD, p.entity_id, p.r#type);
	let kind = crate::registry::names(OLD, "entities")
		.and_then(|n| n.name(p.r#type as u32))
		.unwrap_or_default();
	p.object_data = spawn_object_data(OLD, NEW, kind, p.object_data);
	Ok(vec![Client::SpawnEntity(p)])
}

pub fn trails_spawn_entity_1_20_2_to_1_20(
	mut p: new::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	common::spawned(cx, NEW, p.entity_id, p.r#type);
	if p.r#type == player_id(NEW)? {
		return Ok(vec![ClientOld::NamedEntitySpawn(
			old::play_to_client_packet_named_entity_spawn::Ty {
				entity_id: p.entity_id,
				player_uuid: p.object_uuid,
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
	p.object_data = spawn_object_data(NEW, OLD, kind, p.object_data);
	Ok(vec![ClientOld::SpawnEntity(p)])
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
		from,
		None,
	)
}

arm!(
	trails_map_chunk_1_20_to_1_20_2,
	Client,
	MapChunk,
	old::play_to_client_packet_map_chunk::Ty,
	|p, _cx| {
		new::play_to_client_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			heightmaps: Tag::from(p.heightmaps),
			chunk_data: chunk_data(OLD, NEW, &p.chunk_data)?,
			block_entities: p
				.block_entities
				.into_iter()
				.map(|b| new::chunk_block_entity::Ty {
					anon: new::chunk_block_entity_anon::Ty {
						x: b.anon.x,
						z: b.anon.z,
					},
					y: b.y,
					r#type: b.r#type,
					nbt_data: b.nbt_data.map(Tag::from),
				})
				.collect(),
			sky_light_mask: p.sky_light_mask,
			block_light_mask: p.block_light_mask,
			empty_sky_light_mask: p.empty_sky_light_mask,
			empty_block_light_mask: p.empty_block_light_mask,
			sky_light: p.sky_light,
			block_light: p.block_light,
		}
	}
);

arm!(
	trails_map_chunk_1_20_2_to_1_20,
	ClientOld,
	MapChunk,
	new::play_to_client_packet_map_chunk::Ty,
	|p, _cx| {
		old::play_to_client_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			heightmaps: p.heightmaps.named(""),
			chunk_data: chunk_data(NEW, OLD, &p.chunk_data)?,
			block_entities: p
				.block_entities
				.into_iter()
				.map(|b| old::chunk_block_entity::Ty {
					anon: old::chunk_block_entity_anon::Ty {
						x: b.anon.x,
						z: b.anon.z,
					},
					y: b.y,
					r#type: b.r#type,
					nbt_data: b.nbt_data.map(Entry::from),
				})
				.collect(),
			sky_light_mask: p.sky_light_mask,
			block_light_mask: p.block_light_mask,
			empty_sky_light_mask: p.empty_sky_light_mask,
			empty_block_light_mask: p.empty_block_light_mask,
			sky_light: p.sky_light,
			block_light: p.block_light,
		}
	}
);

arm!(
	trails_multi_block_change_1_20_to_1_20_2,
	Client,
	MultiBlockChange,
	old::play_to_client_packet_multi_block_change::Ty,
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
	MultiBlockChange,
	new::play_to_client_packet_multi_block_change::Ty,
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
	BlockChange,
	old::play_to_client_packet_block_change::Ty,
	|mut p, _cx| {
		p.r#type = block_state(OLD, NEW, p.r#type);
		p
	}
);

arm!(
	trails_block_change_1_20_2_to_1_20,
	ClientOld,
	BlockChange,
	new::play_to_client_packet_block_change::Ty,
	|mut p, _cx| {
		p.r#type = block_state(NEW, OLD, p.r#type);
		p
	}
);

arm!(
	trails_world_event_1_20_to_1_20_2,
	Client,
	WorldEvent,
	old::play_to_client_packet_world_event::Ty,
	|mut p, _cx| {
		p.data = world_event_data(OLD, NEW, p.effect_id, p.data);
		p
	}
);

arm!(
	trails_world_event_1_20_2_to_1_20,
	ClientOld,
	WorldEvent,
	new::play_to_client_packet_world_event::Ty,
	|mut p, _cx| {
		p.data = world_event_data(NEW, OLD, p.effect_id, p.data);
		p
	}
);

macro_rules! world_particles {
	($name:ident, $root:ty, $src:ident, $dst:ident, $from:expr, $to:expr, $item:expr) => {
		arm!(
			$name,
			$root,
			WorldParticles,
			$src::play_to_client_packet_world_particles::Ty,
			|p, _cx| {
				let payload = encode(&p.data, $src::particle_data::serialize)?;
				let (id, payload) = common::particle(
					&ParticleMap {
						from: $from,
						to: $to,
						item: &$item,
					},
					p.particle_id,
					&payload,
				)?;
				$dst::play_to_client_packet_world_particles::Ty {
					particle_id: id,
					long_distance: p.long_distance,
					x: p.x,
					y: p.y,
					z: p.z,
					offset_x: p.offset_x,
					offset_y: p.offset_y,
					offset_z: p.offset_z,
					particle_data: p.particle_data,
					particles: p.particles,
					data: decode(&payload, |i| {
						$dst::particle_data::deserialize(
							i,
							$dst::particle_data::Args { compare_to: &id },
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
	SoundEffect,
	old::play_to_client_packet_sound_effect::Ty,
	|mut p, _cx| {
		p.sound = sound_holder(OLD, NEW, p.sound)?;
		p
	}
);

arm!(
	trails_sound_effect_1_20_2_to_1_20,
	ClientOld,
	SoundEffect,
	new::play_to_client_packet_sound_effect::Ty,
	|mut p, _cx| {
		p.sound = sound_holder(NEW, OLD, p.sound)?;
		p
	}
);

arm!(
	trails_entity_sound_effect_1_20_to_1_20_2,
	Client,
	EntitySoundEffect,
	old::play_to_client_packet_entity_sound_effect::Ty,
	|mut p, _cx| {
		p.sound_event = entity_sound(OLD, NEW, p.sound_event)?;
		p
	}
);

arm!(
	trails_entity_sound_effect_1_20_2_to_1_20,
	ClientOld,
	EntitySoundEffect,
	new::play_to_client_packet_entity_sound_effect::Ty,
	|mut p, _cx| {
		p.sound_event = entity_sound(NEW, OLD, p.sound_event)?;
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
	src: &old::play_to_client_packet_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	effect_1_20_to_1_20_2(src.effect_id)
}

pub fn trails_entity_effect_id_1_20_2_to_1_20(
	src: &new::play_to_client_packet_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	effect_1_20_2_to_1_20(src.effect_id)
}

pub fn trails_remove_effect_id_1_20_to_1_20_2(
	src: &old::play_to_client_packet_remove_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	effect_1_20_to_1_20_2(src.effect_id)
}

pub fn trails_remove_effect_id_1_20_2_to_1_20(
	src: &new::play_to_client_packet_remove_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	effect_1_20_2_to_1_20(src.effect_id)
}

pub fn trails_beacon_primary_1_20_to_1_20_2(
	src: &old::play_to_server_packet_set_beacon_effect::Ty,
	_cx: &mut Session,
) -> Result<Option<i32>> {
	src.primary_effect.map(effect_1_20_to_1_20_2).transpose()
}

pub fn trails_beacon_primary_1_20_2_to_1_20(
	src: &new::play_to_server_packet_set_beacon_effect::Ty,
	_cx: &mut Session,
) -> Result<Option<i32>> {
	src.primary_effect.map(effect_1_20_2_to_1_20).transpose()
}

pub fn trails_beacon_secondary_1_20_to_1_20_2(
	src: &old::play_to_server_packet_set_beacon_effect::Ty,
	_cx: &mut Session,
) -> Result<Option<i32>> {
	src.secondary_effect.map(effect_1_20_to_1_20_2).transpose()
}

pub fn trails_beacon_secondary_1_20_2_to_1_20(
	src: &new::play_to_server_packet_set_beacon_effect::Ty,
	_cx: &mut Session,
) -> Result<Option<i32>> {
	src.secondary_effect.map(effect_1_20_2_to_1_20).transpose()
}

pub fn trails_criteria_1_20_2_to_1_20(
	src: &new::play_to_client_packet_advancements_advancement_mapping_items_value::Ty,
	_cx: &mut Session,
) -> Result<
	Vec<old::play_to_client_packet_advancements_advancement_mapping_items_value_criteria_items::Ty>,
> {
	let mut keys: Vec<String> = src.requirements.iter().flatten().cloned().collect();
	keys.sort();
	keys.dedup();
	Ok(keys
		.into_iter()
		.map(|key| old::play_to_client_packet_advancements_advancement_mapping_items_value_criteria_items::Ty { key, value: () })
		.collect())
}

pub fn trails_login_uuid_1_20_to_1_20_2(
	src: &old::login_to_server_packet_login_start::Ty,
	_cx: &mut Session,
) -> Result<u128> {
	Ok(src
		.player_uuid
		.unwrap_or_else(|| common::offline_uuid(&src.username)))
}

pub fn trails_login_uuid_1_20_2_to_1_20(
	src: &new::login_to_server_packet_login_start::Ty,
	_cx: &mut Session,
) -> Result<Option<u128>> {
	Ok(Some(src.player_uuid))
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
) -> new::configuration_to_client_packet_tags_items_tags_items::Ty {
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
	new::configuration_to_client_packet_tags_items_tags_items::Ty {
		tag_name: format!("minecraft:{name}"),
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

fn physics_tags() -> Vec<new::configuration_to_client_packet_tags_items::Ty> {
	use new::configuration_to_client_packet_tags_items::Ty as Registry;
	use new::configuration_to_client_packet_tags_items_tags_items::Ty as T;
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
			tag_type: "minecraft:fluid".to_owned(),
			tags: vec![
				T {
					tag_name: "minecraft:water".to_owned(),
					entries: vec![2, 1],
				},
				T {
					tag_name: "minecraft:lava".to_owned(),
					entries: vec![4, 3],
				},
			],
		},
		Registry {
			tag_type: "minecraft:block".to_owned(),
			tags: blocks,
		},
		Registry {
			tag_type: "minecraft:worldgen/biome".to_owned(),
			tags: ["is_badlands", "is_jungle", "is_savanna"]
				.iter()
				.map(|n| T {
					tag_name: format!("minecraft:{n}"),
					entries: vec![],
				})
				.collect(),
		},
	]
}

pub fn configuration_capture_1_20_2(cx: &mut Session, packet: &[u8]) -> Result<Vec<Vec<u8>>> {
	use new::configuration_to_client_packet::Ty as P;
	use new::configuration_to_server_packet::Ty as R;
	let s = common::state(cx);
	let replies = match decode(packet, |i| {
		new::configuration_to_client_packet::deserialize(i)
	})? {
		P::RegistryData(codec) => {
			s.codec = Some(codec);
			vec![]
		}
		P::FeatureFlags(f) => {
			s.features = Some(f);
			vec![]
		}
		P::Tags(t) => {
			s.tags = Some(t);
			vec![]
		}
		P::CustomPayload(p) => {
			s.payloads.push((p.channel, p.data));
			vec![]
		}
		P::ResourcePackSend(p) => {
			s.resource_pack = Some((p.url, p.hash, p.forced, p.prompt_message));
			vec![R::ResourcePackReceive(3), R::ResourcePackReceive(0)]
		}
		P::KeepAlive(id) => vec![R::KeepAlive(id)],
		P::Ping(id) => vec![R::Pong(id)],
		P::FinishConfiguration(_) => vec![R::FinishConfiguration(
			new::configuration_to_server_packet_finish_configuration::Ty {},
		)],
		P::Disconnect(_) | P::Default(_) => vec![],
	};
	replies
		.iter()
		.map(|r| encode(r, new::configuration_to_server_packet::serialize))
		.collect()
}

pub fn configuration_1_20_2(cx: &mut Session) -> Result<Vec<Vec<u8>>> {
	use new::configuration_to_client_packet::Ty as P;
	let codec = match common::state(cx).codec.clone() {
		Some(c) => c,
		None => super::codec::vanilla(NEW)?,
	};
	[
		P::RegistryData(codec),
		P::FeatureFlags(vec!["minecraft:vanilla".to_owned()]),
		P::Tags(physics_tags()),
		P::FinishConfiguration(new::configuration_to_client_packet_finish_configuration::Ty {}),
	]
	.iter()
	.map(|p| encode(p, new::configuration_to_client_packet::serialize))
	.collect()
}
