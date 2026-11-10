use super::chunk;
use super::meta::{self, Kind, Meta, MINECARTS};
use super::*;
use crate::gen::convert::{v1_18_2, v1_19, v1_19_2};

type Client18 = v1_18_2::play_toClient_packet::Ty;
type Client19 = v1_19::play_toClient_packet::Ty;
type Client192 = v1_19_2::play_toClient_packet::Ty;
type Server18 = v1_18_2::play_toServer_packet::Ty;
type Server19 = v1_19::play_toServer_packet::Ty;
type Login18 = v1_18_2::login_toServer_packet::Ty;
type Login19 = v1_19::login_toServer_packet::Ty;

const OLD: V = V::V1_18_2;
const NEW: V = V::V1_19;
const NEWER: V = V::V1_19_2;

pub fn caves_absent_1_19<S, T>(_src: &S, _cx: &mut Session) -> Result<Option<T>> {
	Ok(None)
}

pub fn caves_absent_1_19_2<S, T>(_src: &S, _cx: &mut Session) -> Result<Option<T>> {
	Ok(None)
}

pub fn caves_seed_1_19<S>(_src: &S, cx: &mut Session) -> Result<i64> {
	Ok(world(cx).random())
}

pub fn caves_slot_1_19(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot(OLD, NEW, v))
}

pub fn caves_slot_down_1_18_2(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot(NEW, OLD, v))
}

pub fn caves_particle_1_19(
	v: v1_18_2::particle::Ty,
	_cx: &mut Session,
) -> Result<v1_19::particle::Ty> {
	particle(
		OLD,
		NEW,
		&v,
		v1_18_2::particle::serialize,
		v1_19::particle::deserialize,
	)
}

pub fn caves_particle_down_1_18_2(
	v: v1_19::particle::Ty,
	_cx: &mut Session,
) -> Result<v1_18_2::particle::Ty> {
	particle(
		NEW,
		OLD,
		&v,
		v1_19::particle::serialize,
		v1_18_2::particle::deserialize,
	)
}

pub(super) const PARSERS_1_19: [&str; 48] = [
	"brigadier:bool",
	"brigadier:float",
	"brigadier:double",
	"brigadier:integer",
	"brigadier:long",
	"brigadier:string",
	"minecraft:entity",
	"minecraft:game_profile",
	"minecraft:block_pos",
	"minecraft:column_pos",
	"minecraft:vec3",
	"minecraft:vec2",
	"minecraft:block_state",
	"minecraft:block_predicate",
	"minecraft:item_stack",
	"minecraft:item_predicate",
	"minecraft:color",
	"minecraft:component",
	"minecraft:message",
	"minecraft:nbt_compound_tag",
	"minecraft:nbt_tag",
	"minecraft:nbt_path",
	"minecraft:objective",
	"minecraft:objective_criteria",
	"minecraft:operation",
	"minecraft:particle",
	"minecraft:angle",
	"minecraft:rotation",
	"minecraft:scoreboard_slot",
	"minecraft:score_holder",
	"minecraft:swizzle",
	"minecraft:team",
	"minecraft:item_slot",
	"minecraft:resource_location",
	"minecraft:mob_effect",
	"minecraft:function",
	"minecraft:entity_anchor",
	"minecraft:int_range",
	"minecraft:float_range",
	"minecraft:item_enchantment",
	"minecraft:entity_summon",
	"minecraft:dimension",
	"minecraft:time",
	"minecraft:resource_or_tag",
	"minecraft:resource",
	"minecraft:template_mirror",
	"minecraft:template_rotation",
	"minecraft:uuid",
];

pub(super) fn greedy_string() -> (&'static str, Vec<u8>) {
	("brigadier:string", vec![2])
}

fn parser_down(src: &v1_19::command_node__extraNodeData__v2::Ty) -> Result<(String, Vec<u8>)> {
	let bytes = encode(
		&src.properties,
		v1_19::command_node__extraNodeData__v2__properties::serialize,
	)?;
	let mut at = 0;
	let id = read_varint(&bytes, &mut at)?;
	let (name, payload) = match PARSERS_1_19.get(id as usize) {
		Some(name) if !name.starts_with("minecraft:template_") => (*name, bytes[at..].to_vec()),
		_ => greedy_string(),
	};
	Ok((name.to_owned(), payload))
}

pub fn caves_parser_down_1_18_2(
	src: &v1_19::command_node__extraNodeData__v2::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(parser_down(src)?.0)
}

pub fn caves_parser_properties_down_1_18_2(
	src: &v1_19::command_node__extraNodeData__v2::Ty,
	_cx: &mut Session,
) -> Result<v1_18_2::command_node__extraNodeData__v2__properties::Ty> {
	let (name, payload) = parser_down(src)?;
	let args = v1_18_2::command_node__extraNodeData__v2__properties::Args { parser: &name };
	v1_18_2::command_node__extraNodeData__v2__properties::deserialize(
		&mut Input::new(&payload),
		args,
	)
	.map_err(encoding)
}

pub fn caves_parser_properties_1_19(
	src: &v1_18_2::command_node__extraNodeData__v2::Ty,
	_cx: &mut Session,
) -> Result<v1_19::command_node__extraNodeData__v2__properties::Ty> {
	let payload = encode(
		&src.properties,
		v1_18_2::command_node__extraNodeData__v2__properties::serialize,
	)?;
	let (name, payload) = match PARSERS_1_19.contains(&src.parser.as_str()) {
		true => (src.parser.as_str(), payload),
		false => greedy_string(),
	};
	let mut out = Vec::new();
	write_varint(
		&mut out,
		PARSERS_1_19.iter().position(|p| *p == name).unwrap_or(5) as i32,
	);
	out.extend(payload);
	decode(
		&out,
		v1_19::command_node__extraNodeData__v2__properties::deserialize,
	)
}

pub fn caves_login_start_1_19(username: String, _cx: &mut Session) -> Result<Vec<Login19>> {
	Ok(vec![Login19::login_start(
		v1_19::login_toServer_packet_login_start::Ty {
			username,
			signature: None,
		},
	)])
}

pub fn caves_login_start_down_1_18_2(
	p: v1_19::login_toServer_packet_login_start::Ty,
	_cx: &mut Session,
) -> Result<Vec<Login18>> {
	Ok(vec![Login18::login_start(p.username)])
}

pub fn caves_encryption_crypto_1_19(
	src: &v1_18_2::login_toServer_packet_encryption_begin::Ty,
	_cx: &mut Session,
) -> Result<v1_19::login_toServer_packet_encryption_begin__crypto::Ty> {
	Ok(v1_19::login_toServer_packet_encryption_begin__crypto::Ty::r#true(src.verifyToken.clone()))
}

pub fn caves_encryption_verify_token_down_1_18_2(
	src: &v1_19::login_toServer_packet_encryption_begin::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	match &src.crypto {
		v1_19::login_toServer_packet_encryption_begin__crypto::Ty::r#true(token) => {
			Ok(token.clone())
		}
		_ => unconvertible(
			"a signed nonce can't become the verify token without the server's private key",
		),
	}
}

pub fn caves_chat_1_19(
	p: v1_18_2::play_toClient_packet_chat::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client19>> {
	Ok(vec![Client19::system_chat(
		v1_19::play_toClient_packet_system_chat::Ty {
			content: p.message,
			r#type: if p.position == 2 { 2 } else { 1 },
		},
	)])
}

fn overlay_1_19(cx: &mut Session, ty: i32) -> bool {
	chat_types::chat_type(cx, NEW, ty).is_some_and(|t| t.overlay)
}

pub fn caves_system_chat_down_1_18_2(
	p: v1_19::play_toClient_packet_system_chat::Ty,
	cx: &mut Session,
) -> Result<Vec<Client18>> {
	Ok(vec![Client18::chat(
		v1_18_2::play_toClient_packet_chat::Ty {
			message: p.content,
			position: if overlay_1_19(cx, p.r#type) { 2 } else { 1 },
			sender: 0,
		},
	)])
}

pub fn caves_player_chat_down_1_18_2(
	p: v1_19::play_toClient_packet_player_chat::Ty,
	cx: &mut Session,
) -> Result<Vec<Client18>> {
	let content = p
		.unsignedChatContent
		.as_deref()
		.unwrap_or(&p.signedChatContent);
	let (message, overlay) = chat_types::decorate(
		cx,
		(NEW, OLD),
		p.r#type,
		&p.senderName,
		p.senderTeam.as_deref(),
		content,
	)?;
	Ok(vec![Client18::chat(
		v1_18_2::play_toClient_packet_chat::Ty {
			message,
			position: if overlay { 2 } else { 0 },
			sender: p.senderUuid,
		},
	)])
}

pub fn caves_spawn_entity_living_1_19(
	p: v1_18_2::play_toClient_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	world(cx).spawned(p.entityId, OLD, p.r#type);
	let ty = entity_type(OLD, NEW, p.r#type).ok_or(Error::Dropped)?;
	Ok(vec![Client19::spawn_entity(
		v1_19::play_toClient_packet_spawn_entity::Ty {
			entityId: p.entityId,
			objectUUID: p.entityUUID,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			pitch: p.pitch,
			yaw: p.yaw,
			headPitch: p.headPitch,
			objectData: 0,
			velocity: p.velocity,
		},
	)])
}

fn painting_variant_1_19(motive: i32) -> i32 {
	match motive {
		25 => 29,
		m => m,
	}
}

fn painting_motive_1_18_2(variant: i32) -> i32 {
	match variant {
		29 => 25,
		25 => 19,
		26 => 16,
		27 => 18,
		28 => 14,
		v => v,
	}
}

pub fn caves_spawn_entity_painting_1_19(
	p: v1_18_2::play_toClient_packet_spawn_entity_painting::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	let ty = registry::names(NEW, "entities")
		.and_then(|n| n.id("painting"))
		.ok_or(Error::Dropped)? as i32;
	world(cx).entities.insert(p.entityId, "painting".into());
	let data = match p.direction {
		2 => 2,
		0 => 3,
		1 => 4,
		3 => 5,
		_ => 1,
	};
	let l = &p.location;
	let variant = meta::write(
		NEW,
		&[Meta::varint(
			8,
			Kind::Painting,
			painting_variant_1_19(p.title),
		)],
	);
	Ok(vec![
		Client19::spawn_entity(v1_19::play_toClient_packet_spawn_entity::Ty {
			entityId: p.entityId,
			objectUUID: p.entityUUID,
			r#type: ty,
			x: l.x as f64 + 0.5,
			y: l.y as f64 + 0.5,
			z: l.z as f64 + 0.5,
			pitch: 0,
			yaw: 0,
			headPitch: 0,
			objectData: data,
			velocity: v1_19::vec3i16::Ty { x: 0, y: 0, z: 0 },
		}),
		Client19::entity_metadata(v1_19::play_toClient_packet_entity_metadata::Ty {
			entityId: p.entityId,
			metadata: decode(&variant, v1_19::entityMetadata::deserialize)?,
		}),
	])
}

pub fn caves_spawn_entity_1_19(
	p: v1_18_2::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	let w = world(cx);
	w.spawned(p.entityId, OLD, p.r#type);
	let falling = w.is(p.entityId, &["falling_block"]);
	let ty = entity_type(OLD, NEW, p.r#type).ok_or(Error::Dropped)?;
	Ok(vec![Client19::spawn_entity(
		v1_19::play_toClient_packet_spawn_entity::Ty {
			entityId: p.entityId,
			objectUUID: p.objectUUID,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			pitch: p.pitch,
			yaw: p.yaw,
			headPitch: p.yaw,
			objectData: if falling {
				state(OLD, NEW, p.objectData)
			} else {
				p.objectData
			},
			velocity: p.velocity,
		},
	)])
}

pub fn caves_spawn_entity_down_1_18_2(
	p: v1_19::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client18>> {
	world(cx).spawned(p.entityId, NEW, p.r#type);
	let names = registry::names(NEW, "entities").ok_or(Error::Dropped)?;
	let name = names.name(p.r#type as u32).ok_or(Error::Dropped)?;
	let category = names
		.extra
		.get(&(p.r#type as u32))
		.and_then(|e| e.first())
		.map_or("other", String::as_str);
	let ty = entity_type(NEW, OLD, p.r#type).ok_or(Error::Dropped)?;
	if name == "painting" {
		world(cx).paintings.insert(
			p.entityId,
			Painting {
				uuid: p.objectUUID,
				position: (p.x, p.y, p.z),
				direction: p.objectData,
			},
		);
		return Ok(vec![]);
	}
	if !matches!(category, "other" | "projectile" | "player") {
		return Ok(vec![Client18::spawn_entity_living(
			v1_18_2::play_toClient_packet_spawn_entity_living::Ty {
				entityId: p.entityId,
				entityUUID: p.objectUUID,
				r#type: ty,
				x: p.x,
				y: p.y,
				z: p.z,
				yaw: p.yaw,
				pitch: p.pitch,
				headPitch: p.headPitch,
				velocity: p.velocity,
			},
		)]);
	}
	Ok(vec![Client18::spawn_entity(
		v1_18_2::play_toClient_packet_spawn_entity::Ty {
			entityId: p.entityId,
			objectUUID: p.objectUUID,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			pitch: p.pitch,
			yaw: p.yaw,
			objectData: if name == "falling_block" {
				state(NEW, OLD, p.objectData)
			} else {
				p.objectData
			},
			velocity: p.velocity,
		},
	)])
}

fn requested(cx: &mut Session, sequence: i32) {
	let w = world(cx);
	w.ack = Some((w.ack.map_or(sequence, |(s, _)| s.max(sequence)), false));
}

fn acknowledged(cx: &mut Session, sequence: i32) -> Result<()> {
	if world(cx).ack.is_some() {
		requested(cx, sequence);
		return Ok(());
	}
	let ack = encode(
		&Client19::acknowledge_player_digging(sequence),
		v1_19::play_toClient_packet::serialize,
	)?;
	cx.send(crate::gen::convert::Root::PlayToClient, NEW.index(), ack);
	Ok(())
}

fn acknowledge(cx: &mut Session, mut out: Vec<Client19>, now: bool) -> Vec<Client19> {
	let w = world(cx);
	match w.ack {
		Some((sequence, aged)) if now || aged => {
			out.push(Client19::acknowledge_player_digging(sequence));
			w.ack = None;
		}
		Some((sequence, _)) => w.ack = Some((sequence, true)),
		None => {}
	}
	out
}

pub fn caves_digging_ack_1_19(
	_p: v1_18_2::play_toClient_packet_acknowledge_player_digging::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	Ok(acknowledge(cx, vec![], true))
}

pub fn caves_block_change_1_19(
	p: v1_18_2::play_toClient_packet_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	let packet = Client19::block_change(v1_19::play_toClient_packet_block_change::Ty {
		location: p.location,
		r#type: state(OLD, NEW, p.r#type),
	});
	Ok(acknowledge(cx, vec![packet], true))
}

pub fn caves_block_change_down_1_18_2(
	p: v1_19::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client18>> {
	Ok(vec![Client18::block_change(
		v1_18_2::play_toClient_packet_block_change::Ty {
			location: p.location,
			r#type: state(NEW, OLD, p.r#type),
		},
	)])
}

pub(super) fn section_records(from: V, to: V, records: &[i64]) -> Vec<i64> {
	let table = registry::state_table(from, to).unwrap_or_default();
	records
		.iter()
		.map(|r| ((table.get((r >> 12) as usize).copied().unwrap_or(0) as i64) << 12) | (r & 0xfff))
		.collect()
}

pub fn caves_multi_block_change_1_19(
	p: v1_18_2::play_toClient_packet_multi_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	let c = &p.chunkCoordinates;
	let packet = Client19::multi_block_change(v1_19::play_toClient_packet_multi_block_change::Ty {
		chunkCoordinates: v1_19::play_toClient_packet_multi_block_change__chunkCoordinates::Ty {
			x: c.x,
			z: c.z,
			y: c.y,
		},
		notTrustEdges: p.notTrustEdges,
		records: section_records(OLD, NEW, &p.records),
	});
	Ok(acknowledge(cx, vec![packet], true))
}

pub fn caves_multi_block_change_down_1_18_2(
	p: v1_19::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client18>> {
	let c = &p.chunkCoordinates;
	Ok(vec![Client18::multi_block_change(
		v1_18_2::play_toClient_packet_multi_block_change::Ty {
			chunkCoordinates:
				v1_18_2::play_toClient_packet_multi_block_change__chunkCoordinates::Ty {
					x: c.x,
					z: c.z,
					y: c.y,
				},
			notTrustEdges: p.notTrustEdges,
			records: section_records(NEW, OLD, &p.records),
		},
	)])
}

pub fn caves_update_time_1_19(
	p: v1_18_2::play_toClient_packet_update_time::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	Ok(acknowledge(cx, vec![Client19::update_time(p)], false))
}

pub fn caves_keep_alive_1_19(p: i64, cx: &mut Session) -> Result<Vec<Client19>> {
	Ok(acknowledge(cx, vec![Client19::keep_alive(p)], false))
}

pub fn caves_entity_destroy_1_19(ids: Vec<i32>, cx: &mut Session) -> Result<Vec<Client19>> {
	let w = world(cx);
	for id in &ids {
		w.entities.remove(id);
	}
	Ok(vec![Client19::entity_destroy(ids)])
}

const TAG_RENAMES_1_19: &[(&str, &str)] = &[
	("minecraft:carpets", "minecraft:wool_carpets"),
	(
		"minecraft:polar_bears_spawnable_on_in_frozen_ocean",
		"minecraft:polar_bears_spawnable_on_alternate",
	),
];

pub fn caves_tags_1_19(p: Tags, _cx: &mut Session) -> Result<Vec<Client19>> {
	Ok(vec![Client19::tags(tags(
		OLD,
		NEW,
		p,
		TAG_RENAMES_1_19,
		super::new_tags::NEW_1_19,
	))])
}

pub fn caves_tags_down_1_18_2(p: Tags, _cx: &mut Session) -> Result<Vec<Client18>> {
	let back: Vec<(&str, &str)> = TAG_RENAMES_1_19.iter().map(|(a, b)| (*b, *a)).collect();
	Ok(vec![Client18::tags(tags(NEW, OLD, p, &back, &[]))])
}

pub fn caves_statistics_1_19(p: Stats, _cx: &mut Session) -> Result<Vec<Client19>> {
	Ok(vec![Client19::statistics(statistics(OLD, NEW, p))])
}

pub fn caves_statistics_down_1_18_2(p: Stats, _cx: &mut Session) -> Result<Vec<Client18>> {
	Ok(vec![Client18::statistics(statistics(NEW, OLD, p))])
}

fn monster_spawning(c: &mut Vec<Entry>) {
	if c.field("monster_spawn_light_level").is_none() {
		c.put("monster_spawn_light_level", Payload::int(11));
	}
	if c.field("monster_spawn_block_light_limit").is_none() {
		c.put("monster_spawn_block_light_limit", Payload::int(0));
	}
}

pub fn caves_codec_1_19(
	src: &v1_18_2::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	let w = world(cx);
	w.track_codec(&src.dimensionCodec);
	w.track_dimension(&src.dimension);
	w.entities.clear();
	w.ack = None;
	let key = w.dimension_key(&src.dimension);
	let mut codec = src.dimensionCodec.clone();
	for d in registry_entries(&mut codec, "minecraft:dimension_type")
		.into_iter()
		.flatten()
	{
		if let (true, Payload::compound(own)) = (
			d.field("name").and_then(Payload::as_str) == Some(key.as_str()),
			&src.dimension.value,
		) {
			d.put("element", own.clone());
		}
		if let Some(element) = d.compound_mut("element") {
			monster_spawning(element);
		}
	}
	put_registry(&mut codec, NEW, "minecraft:chat_type")?;
	Ok(codec)
}

pub fn caves_codec_down_1_18_2(
	src: &v1_19::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	chat_types::track(cx, NEW, &src.dimensionCodec);
	let w = world(cx);
	w.track_codec(&src.dimensionCodec);
	w.entities.clear();
	w.paintings.clear();
	let mut codec = src.dimensionCodec.clone();
	if let Payload::compound(root) = &mut codec.value {
		root.retain(|e| e.name != "minecraft:chat_type");
	}
	Ok(codec)
}

pub fn caves_world_type_1_19(
	src: &v1_18_2::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<String> {
	let w = world(cx);
	w.track_codec(&src.dimensionCodec);
	Ok(w.dimension_key(&src.dimension))
}

fn known_dimension(cx: &mut Session, key: &str) -> Result<Entry> {
	let w = world(cx);
	let Some(d) = w.dimension(key) else {
		return unconvertible(format!("dimension type {key} is not in the registry codec"));
	};
	w.track_dimension(&d);
	Ok(d)
}

pub fn caves_login_dimension_down_1_18_2(
	src: &v1_19::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	world(cx).track_codec(&src.dimensionCodec);
	known_dimension(cx, &src.worldType)
}

pub fn caves_respawn_dimension_1_19(
	src: &v1_18_2::play_toClient_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<String> {
	let w = world(cx);
	w.track_dimension(&src.dimension);
	Ok(w.dimension_key(&src.dimension))
}

pub fn caves_respawn_dimension_down_1_18_2(
	src: &v1_19::play_toClient_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	known_dimension(cx, &src.dimension)
}

pub fn caves_map_chunk_1_19(
	mut p: v1_18_2::play_toClient_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<v1_19::play_toClient_packet_map_chunk::Ty> {
	p.chunkData = chunk::remap_states(&p.chunkData, world(cx).known_sections(), OLD, NEW)?;
	Ok(p)
}

pub fn caves_map_chunk_down_1_18_2(
	mut p: v1_19::play_toClient_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<v1_18_2::play_toClient_packet_map_chunk::Ty> {
	p.chunkData = chunk::remap_states(&p.chunkData, world(cx).known_sections(), NEW, OLD)?;
	p.blockEntities = caves_chunk_entities_1_18_2(p.blockEntities);
	Ok(p)
}

pub fn caves_chunk_entities_1_18_2(
	mut list: Vec<v1_19::chunkBlockEntity::Ty>,
) -> Vec<v1_19::chunkBlockEntity::Ty> {
	list.retain(|b| block_entity_type(NEW, OLD, b.r#type).is_some());
	list
}

pub(super) fn minecart_block(
	from: V,
	to: V,
	minecart: bool,
) -> impl FnMut(Meta) -> Result<Option<Meta>> {
	move |m| {
		Ok(Some(match (minecart, m.key, m.kind) {
			(true, 11, Kind::Int) => {
				Meta::varint(11, Kind::Int, state(from, to, m.int().unwrap_or(0)))
			}
			_ => m,
		}))
	}
}

pub fn caves_metadata_1_19(
	src: &v1_18_2::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<v1_19::entityMetadata::Ty> {
	let w = world(cx);
	let (minecart, cat) = (w.is(src.entityId, MINECARTS), w.is(src.entityId, &["cat"]));
	let mut minecart = minecart_block(OLD, NEW, minecart);
	let bytes = meta::convert(
		OLD,
		NEW,
		&encode(&src.metadata, v1_18_2::entityMetadata::serialize)?,
		|m| match (cat, m.key, m.kind) {
			(true, 19, Kind::Int) => Ok(Some(Meta {
				kind: Kind::Cat,
				..m
			})),
			_ => minecart(m),
		},
	)?;
	decode(&bytes, v1_19::entityMetadata::deserialize)
}

pub(super) fn dropped_keys(name: Option<&str>, keys: &[(&str, &'static [u8])]) -> &'static [u8] {
	let name = name.unwrap_or_default();
	keys.iter()
		.find(|(n, _)| *n == name)
		.map_or(&[], |(_, k)| k)
}

const NEW_KEYS_1_19: &[(&str, &[u8])] = &[
	("frog", &[16, 17, 18]),
	("warden", &[16]),
	("goat", &[18, 19]),
];

pub fn caves_entity_metadata_down_1_18_2(
	p: v1_19::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client18>> {
	let w = world(cx);
	let (minecart, cat) = (w.is(p.entityId, MINECARTS), w.is(p.entityId, &["cat"]));
	let dropped = dropped_keys(w.name(p.entityId), NEW_KEYS_1_19);
	let mut painting = None;
	let mut minecart = minecart_block(NEW, OLD, minecart);
	let bytes = meta::convert(
		NEW,
		OLD,
		&encode(&p.metadata, v1_19::entityMetadata::serialize)?,
		|m| match (m.key, m.kind) {
			(_, Kind::Painting) => {
				painting = m.int();
				Ok(None)
			}
			(19, Kind::Cat) if cat => Ok(Some(Meta {
				kind: Kind::Int,
				..m
			})),
			(k, _) if dropped.contains(&k) => Ok(None),
			(6, Kind::Pose) if m.int().is_some_and(|p| p >= 8) => {
				Ok(Some(Meta::varint(6, Kind::Pose, 0)))
			}
			_ => minecart(m),
		},
	)?;
	let mut out = Vec::new();
	if let Some(variant) = painting {
		if let Some(stored) = world(cx).paintings.remove(&p.entityId) {
			let (x, y, z) = stored.position;
			out.push(Client18::spawn_entity_painting(
				v1_18_2::play_toClient_packet_spawn_entity_painting::Ty {
					entityId: p.entityId,
					entityUUID: stored.uuid,
					title: painting_motive_1_18_2(variant),
					location: v1_18_2::position::Ty {
						x: x.floor() as i32,
						y: y.floor() as i16,
						z: z.floor() as i32,
					},
					direction: match stored.direction {
						2 => 2,
						3 => 0,
						4 => 1,
						5 => 3,
						_ => 0,
					},
				},
			));
		}
	}
	let metadata = decode(&bytes, v1_18_2::entityMetadata::deserialize)?;
	if !metadata.is_empty() || out.is_empty() {
		out.push(Client18::entity_metadata(
			v1_18_2::play_toClient_packet_entity_metadata::Ty {
				entityId: p.entityId,
				metadata,
			},
		));
	}
	Ok(out)
}

pub fn caves_sound_1_19(
	src: &v1_18_2::play_toClient_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound(OLD, NEW, src.soundId)
}

pub fn caves_sound_down_1_18_2(
	src: &v1_19::play_toClient_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound(NEW, OLD, src.soundId)
}

pub fn caves_entity_sound_1_19(
	src: &v1_18_2::play_toClient_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound(OLD, NEW, src.soundId)
}

pub fn caves_entity_sound_down_1_18_2(
	src: &v1_19::play_toClient_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound(NEW, OLD, src.soundId)
}

macro_rules! player_info {
	($name:ident, $from:ident, $to:ident $(, $field:ident: $value:expr)*) => {
		pub fn $name(
			v: $from::play_toClient_packet_player_info::Ty,
			_cx: &mut Session,
		) -> Result<$to::play_toClient_packet_player_info::Ty> {
			use $from::play_toClient_packet_player_info__action::Ty as A;
			use $from::play_toClient_packet_player_info__data__items__ANON::Ty as D;
			use $to::play_toClient_packet_player_info__action::Ty as B;
			use $to::play_toClient_packet_player_info__data__items__ANON::Ty as E;
			let action = |a: &A| match a {
				A::add_player => B::add_player,
				A::update_game_mode => B::update_game_mode,
				A::update_latency => B::update_latency,
				A::update_display_name => B::update_display_name,
				A::remove_player => B::remove_player,
				A::default(t) => B::default(*t),
			};
			Ok($to::play_toClient_packet_player_info::Ty {
				action: action(&v.action),
				data: v
					.data
					.into_iter()
					.map(|d| $to::play_toClient_packet_player_info__data__items::Ty {
						uuid: d.uuid,
						ANON: match d.ANON {
							D::add_player(p) => E::add_player($to::play_toClient_packet_player_info__data__items__ANON__add_player::Ty {
								name: p.name,
								properties: p
									.properties
									.into_iter()
									.map(|x| $to::play_toClient_packet_player_info__data__items__ANON__add_player__properties__items::Ty {
										name: x.name,
										value: x.value,
										signature: x.signature,
									})
									.collect(),
								gamemode: p.gamemode,
								ping: p.ping,
								displayName: p.displayName,
								$($field: $value,)*
							}),
							D::remove_player => E::remove_player,
							D::update_display_name(n) => E::update_display_name(n),
							D::update_game_mode(g) => E::update_game_mode(g),
							D::update_latency(l) => E::update_latency(l),
							D::default(t) => E::default(action(&t)),
						},
					})
					.collect(),
			})
		}
	};
}

player_info!(caves_player_info_1_19, v1_18_2, v1_19, crypto: None);
player_info!(caves_player_info_down_1_18_2, v1_19, v1_18_2);

pub fn caves_trade_second_input_1_19(
	src: &v1_18_2::play_toClient_packet_trade_list__trades__items::Ty,
	_cx: &mut Session,
) -> Result<Slot> {
	Ok(src
		.inputItem2
		.clone()
		.map_or(Slot::r#false, |s| slot(OLD, NEW, s)))
}

pub fn caves_trade_second_input_down_1_18_2(
	src: &v1_19::play_toClient_packet_trade_list__trades__items::Ty,
	_cx: &mut Session,
) -> Result<Option<Slot>> {
	Ok(match slot(NEW, OLD, src.inputItem2.clone()) {
		Slot::r#false => None,
		s => Some(s),
	})
}

macro_rules! world_particles {
	($name:ident, $from:ident, $to:ident, $a:expr, $b:expr) => {
		pub fn $name(
			v: $from::play_toClient_packet_world_particles::Ty,
			_cx: &mut Session,
		) -> Result<$to::play_toClient_packet_world_particles::Ty> {
			let data = encode(&v.data, $from::particleData::serialize)?;
			let Some((id, data)) = particle_data($a, $b, v.particleId as i32, &data)? else {
				return Err(Error::Dropped);
			};
			let args = $to::particleData::Args {
				compareTo: &(id as _),
			};
			let data =
				$to::particleData::deserialize(&mut Input::new(&data), args).map_err(encoding)?;
			Ok($to::play_toClient_packet_world_particles::Ty {
				particleId: id as _,
				longDistance: v.longDistance,
				x: v.x,
				y: v.y,
				z: v.z,
				offsetX: v.offsetX,
				offsetY: v.offsetY,
				offsetZ: v.offsetZ,
				particleData: v.particleData,
				particles: v.particles,
				data,
			})
		}
	};
}

pub(super) use world_particles;

world_particles!(caves_world_particles_1_19, v1_18_2, v1_19, OLD, NEW);
world_particles!(caves_world_particles_down_1_18_2, v1_19, v1_18_2, NEW, OLD);

pub(super) fn world_event(from: V, to: V, effect: i32, data: i32) -> i32 {
	match effect {
		2001 => state(from, to, data),
		1010 => item(from, to, data).unwrap_or(0),
		_ => data,
	}
}

pub fn caves_world_event_data_1_19(
	src: &v1_18_2::play_toClient_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(world_event(OLD, NEW, src.effectId, src.data))
}

pub fn caves_world_event_data_down_1_18_2(
	src: &v1_19::play_toClient_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(world_event(NEW, OLD, src.effectId, src.data))
}

pub fn caves_block_action_id_1_19(
	src: &v1_18_2::play_toClient_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block(OLD, NEW, src.blockId).ok_or(Error::Dropped)
}

pub fn caves_block_action_id_down_1_18_2(
	src: &v1_19::play_toClient_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block(NEW, OLD, src.blockId).ok_or(Error::Dropped)
}

pub fn caves_cooldown_item_1_19(
	src: &v1_18_2::play_toClient_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	item(OLD, NEW, src.itemID).ok_or(Error::Dropped)
}

pub fn caves_cooldown_item_down_1_18_2(
	src: &v1_19::play_toClient_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	item(NEW, OLD, src.itemID).ok_or(Error::Dropped)
}

pub(super) fn now_millis() -> i64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map_or(0, |d| d.as_millis() as i64)
}

pub fn caves_chat_input_1_19(message: String, _cx: &mut Session) -> Result<Vec<Server19>> {
	Ok(vec![match message.strip_prefix('/') {
		Some(command) => Server19::chat_command(v1_19::play_toServer_packet_chat_command::Ty {
			command: command.to_owned(),
			timestamp: now_millis(),
			salt: 0,
			argumentSignatures: vec![],
			signedPreview: false,
		}),
		None => Server19::chat_message(v1_19::play_toServer_packet_chat_message::Ty {
			message,
			timestamp: now_millis(),
			salt: 0,
			signature: vec![],
			signedPreview: false,
		}),
	}])
}

pub fn caves_chat_command_down_1_18_2(
	p: v1_19::play_toServer_packet_chat_command::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server18>> {
	Ok(vec![Server18::chat(format!("/{}", p.command))])
}

pub fn caves_chat_message_down_1_18_2(
	p: v1_19::play_toServer_packet_chat_message::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server18>> {
	Ok(vec![Server18::chat(p.message)])
}

pub fn caves_sequence_1_19<S>(_src: &S, cx: &mut Session) -> Result<i32> {
	let w = world(cx);
	w.sequence = w.sequence.wrapping_add(1) & i32::MAX;
	Ok(w.sequence)
}

pub fn caves_use_item_1_19(hand: i32, cx: &mut Session) -> Result<Vec<Server19>> {
	let sequence = caves_sequence_1_19(&(), cx)?;
	Ok(vec![Server19::use_item(
		v1_19::play_toServer_packet_use_item::Ty { hand, sequence },
	)])
}

pub fn caves_use_item_down_1_18_2(
	p: v1_19::play_toServer_packet_use_item::Ty,
	cx: &mut Session,
) -> Result<Vec<Server18>> {
	acknowledged(cx, p.sequence)?;
	Ok(vec![Server18::use_item(p.hand)])
}

pub fn caves_block_dig_down_1_18_2(
	p: v1_19::play_toServer_packet_block_dig::Ty,
	cx: &mut Session,
) -> Result<Vec<Server18>> {
	match p.status {
		0..=2 => requested(cx, p.sequence),
		_ => acknowledged(cx, p.sequence)?,
	}
	Ok(vec![Server18::block_dig(
		v1_18_2::play_toServer_packet_block_dig::Ty {
			status: p.status,
			location: p.location,
			face: p.face,
		},
	)])
}

pub fn caves_block_place_down_1_18_2(
	p: v1_19::play_toServer_packet_block_place::Ty,
	cx: &mut Session,
) -> Result<Vec<Server18>> {
	requested(cx, p.sequence);
	Ok(vec![Server18::block_place(
		v1_18_2::play_toServer_packet_block_place::Ty {
			hand: p.hand,
			location: p.location,
			direction: p.direction,
			cursorX: p.cursorX,
			cursorY: p.cursorY,
			cursorZ: p.cursorZ,
			insideBlock: p.insideBlock,
		},
	)])
}

pub fn caves_beacon_1_19(
	v: v1_18_2::play_toServer_packet_set_beacon_effect::Ty,
	_cx: &mut Session,
) -> Result<v1_19::play_toServer_packet_set_beacon_effect::Ty> {
	Ok(v1_19::play_toServer_packet_set_beacon_effect::Ty {
		primary_effect: (v.primary_effect >= 0).then_some(v.primary_effect),
		secondary_effect: (v.secondary_effect >= 0).then_some(v.secondary_effect),
	})
}

pub fn caves_beacon_down_1_18_2(
	v: v1_19::play_toServer_packet_set_beacon_effect::Ty,
	_cx: &mut Session,
) -> Result<v1_18_2::play_toServer_packet_set_beacon_effect::Ty> {
	Ok(v1_18_2::play_toServer_packet_set_beacon_effect::Ty {
		primary_effect: v.primary_effect.unwrap_or(-1),
		secondary_effect: v.secondary_effect.unwrap_or(-1),
	})
}

pub fn caves_player_chat_1_19_2(
	p: v1_19::play_toClient_packet_player_chat::Ty,
	cx: &mut Session,
) -> Result<Vec<Client192>> {
	let content = p
		.unsignedChatContent
		.as_deref()
		.unwrap_or(&p.signedChatContent);
	let (content, isActionBar) = chat_types::decorate(
		cx,
		(NEW, NEWER),
		p.r#type,
		&p.senderName,
		p.senderTeam.as_deref(),
		content,
	)?;
	Ok(vec![Client192::system_chat(
		v1_19_2::play_toClient_packet_system_chat::Ty {
			content,
			isActionBar,
		},
	)])
}

pub fn caves_player_chat_down_1_19(
	p: v1_19_2::play_toClient_packet_player_chat::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	let content = p
		.unsignedChatContent
		.or(p.formattedMessage)
		.unwrap_or_else(|| chat_text(&p.plainMessage));
	let (content, overlay) = chat_types::decorate(
		cx,
		(NEWER, NEW),
		p.r#type,
		&p.networkName,
		p.networkTargetName.as_deref(),
		&content,
	)?;
	Ok(vec![Client19::system_chat(
		v1_19::play_toClient_packet_system_chat::Ty {
			content,
			r#type: if overlay { 2 } else { 1 },
		},
	)])
}

pub fn caves_action_bar_1_19_2(
	src: &v1_19::play_toClient_packet_system_chat::Ty,
	cx: &mut Session,
) -> Result<bool> {
	Ok(overlay_1_19(cx, src.r#type))
}

pub fn caves_system_chat_type_down_1_19(
	src: &v1_19_2::play_toClient_packet_system_chat::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(if src.isActionBar { 2 } else { 1 })
}

pub fn caves_codec_1_19_2(
	src: &v1_19::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	chat_types::track(cx, NEW, &src.dimensionCodec);
	let mut codec = src.dimensionCodec.clone();
	put_registry(&mut codec, NEWER, "minecraft:chat_type")?;
	Ok(codec)
}

pub fn caves_codec_down_1_19(
	src: &v1_19_2::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	chat_types::track(cx, NEWER, &src.dimensionCodec);
	let mut codec = src.dimensionCodec.clone();
	put_registry(&mut codec, NEW, "minecraft:chat_type")?;
	Ok(codec)
}

pub fn caves_metadata_down_1_19(
	src: &v1_19_2::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<v1_19::entityMetadata::Ty> {
	let dropped = dropped_keys(world(cx).name(src.entityId), &[("allay", &[16, 17])]);
	let bytes = meta::convert(
		NEWER,
		NEW,
		&encode(&src.metadata, v1_19_2::entityMetadata::serialize)?,
		|m| Ok((!dropped.contains(&m.key)).then_some(m)),
	)?;
	decode(&bytes, v1_19::entityMetadata::deserialize)
}
