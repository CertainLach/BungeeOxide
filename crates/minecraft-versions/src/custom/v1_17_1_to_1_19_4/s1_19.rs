use super::chunk;
use super::meta::{self, Kind, Meta, MINECARTS};
use super::*;
use crate::gen::convert::{v1_18_2, v1_19, v1_19_2};

type Client18 = v1_18_2::play_to_client_packet::Ty;
type Client19 = v1_19::play_to_client_packet::Ty;
type Client192 = v1_19_2::play_to_client_packet::Ty;
type Server18 = v1_18_2::play_to_server_packet::Ty;
type Server19 = v1_19::play_to_server_packet::Ty;
type Login18 = v1_18_2::login_to_server_packet::Ty;
type Login19 = v1_19::login_to_server_packet::Ty;

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

fn parser_down(src: &v1_19::command_node_extra_node_data_v2::Ty) -> Result<(String, Vec<u8>)> {
	let bytes = encode(
		&src.properties,
		v1_19::command_node_extra_node_data_v2_properties::serialize,
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
	src: &v1_19::command_node_extra_node_data_v2::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(parser_down(src)?.0)
}

pub fn caves_parser_properties_down_1_18_2(
	src: &v1_19::command_node_extra_node_data_v2::Ty,
	_cx: &mut Session,
) -> Result<v1_18_2::command_node_extra_node_data_v2_properties::Ty> {
	let (name, payload) = parser_down(src)?;
	let args = v1_18_2::command_node_extra_node_data_v2_properties::Args { parser: &name };
	v1_18_2::command_node_extra_node_data_v2_properties::deserialize(
		&mut Input::new(&payload),
		args,
	)
	.map_err(encoding)
}

pub fn caves_parser_properties_1_19(
	src: &v1_18_2::command_node_extra_node_data_v2::Ty,
	_cx: &mut Session,
) -> Result<v1_19::command_node_extra_node_data_v2_properties::Ty> {
	let payload = encode(
		&src.properties,
		v1_18_2::command_node_extra_node_data_v2_properties::serialize,
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
		v1_19::command_node_extra_node_data_v2_properties::deserialize,
	)
}

pub fn caves_login_start_1_19(username: String, _cx: &mut Session) -> Result<Vec<Login19>> {
	Ok(vec![Login19::LoginStart(
		v1_19::login_to_server_packet_login_start::Ty {
			username,
			signature: None,
		},
	)])
}

pub fn caves_login_start_down_1_18_2(
	p: v1_19::login_to_server_packet_login_start::Ty,
	_cx: &mut Session,
) -> Result<Vec<Login18>> {
	Ok(vec![Login18::LoginStart(p.username)])
}

pub fn caves_encryption_crypto_1_19(
	src: &v1_18_2::login_to_server_packet_encryption_begin::Ty,
	_cx: &mut Session,
) -> Result<v1_19::login_to_server_packet_encryption_begin_crypto::Ty> {
	Ok(v1_19::login_to_server_packet_encryption_begin_crypto::Ty::True(src.verify_token.clone()))
}

pub fn caves_encryption_verify_token_down_1_18_2(
	src: &v1_19::login_to_server_packet_encryption_begin::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	match &src.crypto {
		v1_19::login_to_server_packet_encryption_begin_crypto::Ty::True(token) => Ok(token.clone()),
		_ => unconvertible(
			"a signed nonce can't become the verify token without the server's private key",
		),
	}
}

pub fn caves_chat_1_19(
	p: v1_18_2::play_to_client_packet_chat::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client19>> {
	Ok(vec![Client19::SystemChat(
		v1_19::play_to_client_packet_system_chat::Ty {
			content: p.message,
			r#type: if p.position == 2 { 2 } else { 1 },
		},
	)])
}

fn overlay_1_19(cx: &mut Session, ty: i32) -> bool {
	chat_types::chat_type(cx, NEW, ty).is_some_and(|t| t.overlay)
}

pub fn caves_system_chat_down_1_18_2(
	p: v1_19::play_to_client_packet_system_chat::Ty,
	cx: &mut Session,
) -> Result<Vec<Client18>> {
	Ok(vec![Client18::Chat(
		v1_18_2::play_to_client_packet_chat::Ty {
			message: p.content,
			position: if overlay_1_19(cx, p.r#type) { 2 } else { 1 },
			sender: 0,
		},
	)])
}

pub fn caves_player_chat_down_1_18_2(
	p: v1_19::play_to_client_packet_player_chat::Ty,
	cx: &mut Session,
) -> Result<Vec<Client18>> {
	let content = p
		.unsigned_chat_content
		.as_deref()
		.unwrap_or(&p.signed_chat_content);
	let (message, overlay) = chat_types::decorate(
		cx,
		(NEW, OLD),
		p.r#type,
		&p.sender_name,
		p.sender_team.as_deref(),
		content,
	)?;
	Ok(vec![Client18::Chat(
		v1_18_2::play_to_client_packet_chat::Ty {
			message,
			position: if overlay { 2 } else { 0 },
			sender: p.sender_uuid,
		},
	)])
}

pub fn caves_spawn_entity_living_1_19(
	p: v1_18_2::play_to_client_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	world(cx).spawned(p.entity_id, OLD, p.r#type);
	let ty = entity_type(OLD, NEW, p.r#type).ok_or(Error::Dropped)?;
	Ok(vec![Client19::SpawnEntity(
		v1_19::play_to_client_packet_spawn_entity::Ty {
			entity_id: p.entity_id,
			object_uuid: p.entity_uuid,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			pitch: p.pitch,
			yaw: p.yaw,
			head_pitch: p.head_pitch,
			object_data: 0,
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
	p: v1_18_2::play_to_client_packet_spawn_entity_painting::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	let ty = registry::names(NEW, "entities")
		.and_then(|n| n.id("painting"))
		.ok_or(Error::Dropped)? as i32;
	world(cx).entities.insert(p.entity_id, "painting".into());
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
		Client19::SpawnEntity(v1_19::play_to_client_packet_spawn_entity::Ty {
			entity_id: p.entity_id,
			object_uuid: p.entity_uuid,
			r#type: ty,
			x: l.x as f64 + 0.5,
			y: l.y as f64 + 0.5,
			z: l.z as f64 + 0.5,
			pitch: 0,
			yaw: 0,
			head_pitch: 0,
			object_data: data,
			velocity: v1_19::vec3i16::Ty { x: 0, y: 0, z: 0 },
		}),
		Client19::EntityMetadata(v1_19::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
			metadata: decode(&variant, v1_19::entity_metadata::deserialize)?,
		}),
	])
}

pub fn caves_spawn_entity_1_19(
	p: v1_18_2::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	let w = world(cx);
	w.spawned(p.entity_id, OLD, p.r#type);
	let falling = w.is(p.entity_id, &["falling_block"]);
	let ty = entity_type(OLD, NEW, p.r#type).ok_or(Error::Dropped)?;
	Ok(vec![Client19::SpawnEntity(
		v1_19::play_to_client_packet_spawn_entity::Ty {
			entity_id: p.entity_id,
			object_uuid: p.object_uuid,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			pitch: p.pitch,
			yaw: p.yaw,
			head_pitch: p.yaw,
			object_data: if falling {
				state(OLD, NEW, p.object_data)
			} else {
				p.object_data
			},
			velocity: p.velocity,
		},
	)])
}

pub fn caves_spawn_entity_down_1_18_2(
	p: v1_19::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client18>> {
	world(cx).spawned(p.entity_id, NEW, p.r#type);
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
			p.entity_id,
			Painting {
				uuid: p.object_uuid,
				position: (p.x, p.y, p.z),
				direction: p.object_data,
			},
		);
		return Ok(vec![]);
	}
	if !matches!(category, "other" | "projectile" | "player") {
		return Ok(vec![Client18::SpawnEntityLiving(
			v1_18_2::play_to_client_packet_spawn_entity_living::Ty {
				entity_id: p.entity_id,
				entity_uuid: p.object_uuid,
				r#type: ty,
				x: p.x,
				y: p.y,
				z: p.z,
				yaw: p.yaw,
				pitch: p.pitch,
				head_pitch: p.head_pitch,
				velocity: p.velocity,
			},
		)]);
	}
	Ok(vec![Client18::SpawnEntity(
		v1_18_2::play_to_client_packet_spawn_entity::Ty {
			entity_id: p.entity_id,
			object_uuid: p.object_uuid,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			pitch: p.pitch,
			yaw: p.yaw,
			object_data: if name == "falling_block" {
				state(NEW, OLD, p.object_data)
			} else {
				p.object_data
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
		&Client19::AcknowledgePlayerDigging(sequence),
		v1_19::play_to_client_packet::serialize,
	)?;
	cx.send(crate::gen::convert::Root::PlayToClient, NEW.index(), ack);
	Ok(())
}

fn acknowledge(cx: &mut Session, mut out: Vec<Client19>, now: bool) -> Vec<Client19> {
	let w = world(cx);
	match w.ack {
		Some((sequence, aged)) if now || aged => {
			out.push(Client19::AcknowledgePlayerDigging(sequence));
			w.ack = None;
		}
		Some((sequence, _)) => w.ack = Some((sequence, true)),
		None => {}
	}
	out
}

pub fn caves_digging_ack_1_19(
	_p: v1_18_2::play_to_client_packet_acknowledge_player_digging::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	Ok(acknowledge(cx, vec![], true))
}

pub fn caves_block_change_1_19(
	p: v1_18_2::play_to_client_packet_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	let packet = Client19::BlockChange(v1_19::play_to_client_packet_block_change::Ty {
		location: p.location,
		r#type: state(OLD, NEW, p.r#type),
	});
	Ok(acknowledge(cx, vec![packet], true))
}

pub fn caves_block_change_down_1_18_2(
	p: v1_19::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client18>> {
	Ok(vec![Client18::BlockChange(
		v1_18_2::play_to_client_packet_block_change::Ty {
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
	p: v1_18_2::play_to_client_packet_multi_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	let c = &p.chunk_coordinates;
	let packet = Client19::MultiBlockChange(v1_19::play_to_client_packet_multi_block_change::Ty {
		chunk_coordinates: v1_19::play_to_client_packet_multi_block_change_chunk_coordinates::Ty {
			x: c.x,
			z: c.z,
			y: c.y,
		},
		not_trust_edges: p.not_trust_edges,
		records: section_records(OLD, NEW, &p.records),
	});
	Ok(acknowledge(cx, vec![packet], true))
}

pub fn caves_multi_block_change_down_1_18_2(
	p: v1_19::play_to_client_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client18>> {
	let c = &p.chunk_coordinates;
	Ok(vec![Client18::MultiBlockChange(
		v1_18_2::play_to_client_packet_multi_block_change::Ty {
			chunk_coordinates:
				v1_18_2::play_to_client_packet_multi_block_change_chunk_coordinates::Ty {
					x: c.x,
					z: c.z,
					y: c.y,
				},
			not_trust_edges: p.not_trust_edges,
			records: section_records(NEW, OLD, &p.records),
		},
	)])
}

pub fn caves_update_time_1_19(
	p: v1_18_2::play_to_client_packet_update_time::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	Ok(acknowledge(cx, vec![Client19::UpdateTime(p)], false))
}

pub fn caves_keep_alive_1_19(p: i64, cx: &mut Session) -> Result<Vec<Client19>> {
	Ok(acknowledge(cx, vec![Client19::KeepAlive(p)], false))
}

pub fn caves_entity_destroy_1_19(ids: Vec<i32>, cx: &mut Session) -> Result<Vec<Client19>> {
	let w = world(cx);
	for id in &ids {
		w.entities.remove(id);
	}
	Ok(vec![Client19::EntityDestroy(ids)])
}

const TAG_RENAMES_1_19: &[(&str, &str)] = &[
	("minecraft:carpets", "minecraft:wool_carpets"),
	(
		"minecraft:polar_bears_spawnable_on_in_frozen_ocean",
		"minecraft:polar_bears_spawnable_on_alternate",
	),
];

pub fn caves_tags_1_19(p: Tags, _cx: &mut Session) -> Result<Vec<Client19>> {
	Ok(vec![Client19::Tags(tags(
		OLD,
		NEW,
		p,
		TAG_RENAMES_1_19,
		super::new_tags::NEW_1_19,
	))])
}

pub fn caves_tags_down_1_18_2(p: Tags, _cx: &mut Session) -> Result<Vec<Client18>> {
	let back: Vec<(&str, &str)> = TAG_RENAMES_1_19.iter().map(|(a, b)| (*b, *a)).collect();
	Ok(vec![Client18::Tags(tags(NEW, OLD, p, &back, &[]))])
}

pub fn caves_statistics_1_19(p: Stats, _cx: &mut Session) -> Result<Vec<Client19>> {
	Ok(vec![Client19::Statistics(statistics(OLD, NEW, p))])
}

pub fn caves_statistics_down_1_18_2(p: Stats, _cx: &mut Session) -> Result<Vec<Client18>> {
	Ok(vec![Client18::Statistics(statistics(NEW, OLD, p))])
}

fn monster_spawning(c: &mut Vec<Entry>) {
	if c.field("monster_spawn_light_level").is_none() {
		c.put("monster_spawn_light_level", Payload::Int(11));
	}
	if c.field("monster_spawn_block_light_limit").is_none() {
		c.put("monster_spawn_block_light_limit", Payload::Int(0));
	}
}

pub fn caves_codec_1_19(
	src: &v1_18_2::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	let w = world(cx);
	w.track_codec(&src.dimension_codec);
	w.track_dimension(&src.dimension);
	w.entities.clear();
	w.ack = None;
	let key = w.dimension_key(&src.dimension);
	let mut codec = src.dimension_codec.clone();
	for d in registry_entries(&mut codec, "minecraft:dimension_type")
		.into_iter()
		.flatten()
	{
		if let (true, Payload::Compound(own)) = (
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
	src: &v1_19::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	chat_types::track(cx, NEW, &src.dimension_codec);
	let w = world(cx);
	w.track_codec(&src.dimension_codec);
	w.entities.clear();
	w.paintings.clear();
	let mut codec = src.dimension_codec.clone();
	if let Payload::Compound(root) = &mut codec.value {
		root.retain(|e| e.name != "minecraft:chat_type");
	}
	Ok(codec)
}

pub fn caves_world_type_1_19(
	src: &v1_18_2::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<String> {
	let w = world(cx);
	w.track_codec(&src.dimension_codec);
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
	src: &v1_19::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	world(cx).track_codec(&src.dimension_codec);
	known_dimension(cx, &src.world_type)
}

pub fn caves_respawn_dimension_1_19(
	src: &v1_18_2::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<String> {
	let w = world(cx);
	w.track_dimension(&src.dimension);
	Ok(w.dimension_key(&src.dimension))
}

pub fn caves_respawn_dimension_down_1_18_2(
	src: &v1_19::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	known_dimension(cx, &src.dimension)
}

pub fn caves_map_chunk_1_19(
	mut p: v1_18_2::play_to_client_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<v1_19::play_to_client_packet_map_chunk::Ty> {
	p.chunk_data = chunk::remap_states(&p.chunk_data, world(cx).known_sections(), OLD, NEW)?;
	Ok(p)
}

pub fn caves_map_chunk_down_1_18_2(
	mut p: v1_19::play_to_client_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<v1_18_2::play_to_client_packet_map_chunk::Ty> {
	p.chunk_data = chunk::remap_states(&p.chunk_data, world(cx).known_sections(), NEW, OLD)?;
	p.block_entities = caves_chunk_entities_1_18_2(p.block_entities);
	Ok(p)
}

pub fn caves_chunk_entities_1_18_2(
	mut list: Vec<v1_19::chunk_block_entity::Ty>,
) -> Vec<v1_19::chunk_block_entity::Ty> {
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
	src: &v1_18_2::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<v1_19::entity_metadata::Ty> {
	let w = world(cx);
	let (minecart, cat) = (
		w.is(src.entity_id, MINECARTS),
		w.is(src.entity_id, &["cat"]),
	);
	let mut minecart = minecart_block(OLD, NEW, minecart);
	let bytes = meta::convert(
		OLD,
		NEW,
		&encode(&src.metadata, v1_18_2::entity_metadata::serialize)?,
		|m| match (cat, m.key, m.kind) {
			(true, 19, Kind::Int) => Ok(Some(Meta {
				kind: Kind::Cat,
				..m
			})),
			_ => minecart(m),
		},
	)?;
	decode(&bytes, v1_19::entity_metadata::deserialize)
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
	p: v1_19::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client18>> {
	let w = world(cx);
	let (minecart, cat) = (w.is(p.entity_id, MINECARTS), w.is(p.entity_id, &["cat"]));
	let dropped = dropped_keys(w.name(p.entity_id), NEW_KEYS_1_19);
	let mut painting = None;
	let mut minecart = minecart_block(NEW, OLD, minecart);
	let bytes = meta::convert(
		NEW,
		OLD,
		&encode(&p.metadata, v1_19::entity_metadata::serialize)?,
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
		if let Some(stored) = world(cx).paintings.remove(&p.entity_id) {
			let (x, y, z) = stored.position;
			out.push(Client18::SpawnEntityPainting(
				v1_18_2::play_to_client_packet_spawn_entity_painting::Ty {
					entity_id: p.entity_id,
					entity_uuid: stored.uuid,
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
	let metadata = decode(&bytes, v1_18_2::entity_metadata::deserialize)?;
	if !metadata.is_empty() || out.is_empty() {
		out.push(Client18::EntityMetadata(
			v1_18_2::play_to_client_packet_entity_metadata::Ty {
				entity_id: p.entity_id,
				metadata,
			},
		));
	}
	Ok(out)
}

pub fn caves_sound_1_19(
	src: &v1_18_2::play_to_client_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound(OLD, NEW, src.sound_id)
}

pub fn caves_sound_down_1_18_2(
	src: &v1_19::play_to_client_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound(NEW, OLD, src.sound_id)
}

pub fn caves_entity_sound_1_19(
	src: &v1_18_2::play_to_client_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound(OLD, NEW, src.sound_id)
}

pub fn caves_entity_sound_down_1_18_2(
	src: &v1_19::play_to_client_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound(NEW, OLD, src.sound_id)
}

macro_rules! player_info {
	($name:ident, $from:ident, $to:ident $(, $field:ident: $value:expr)*) => {
		pub fn $name(
			v: $from::play_to_client_packet_player_info::Ty,
			_cx: &mut Session,
		) -> Result<$to::play_to_client_packet_player_info::Ty> {
			use $from::play_to_client_packet_player_info_action::Ty as A;
			use $from::play_to_client_packet_player_info_data_items_anon::Ty as D;
			use $to::play_to_client_packet_player_info_action::Ty as B;
			use $to::play_to_client_packet_player_info_data_items_anon::Ty as E;
			let action = |a: &A| match a {
				A::AddPlayer => B::AddPlayer,
				A::UpdateGameMode => B::UpdateGameMode,
				A::UpdateLatency => B::UpdateLatency,
				A::UpdateDisplayName => B::UpdateDisplayName,
				A::RemovePlayer => B::RemovePlayer,
				A::Default(t) => B::Default(*t),
			};
			Ok($to::play_to_client_packet_player_info::Ty {
				action: action(&v.action),
				data: v
					.data
					.into_iter()
					.map(|d| $to::play_to_client_packet_player_info_data_items::Ty {
						uuid: d.uuid,
						anon: match d.anon {
							D::AddPlayer(p) => E::AddPlayer($to::play_to_client_packet_player_info_data_items_anon_add_player::Ty {
								name: p.name,
								properties: p
									.properties
									.into_iter()
									.map(|x| $to::play_to_client_packet_player_info_data_items_anon_add_player_properties_items::Ty {
										name: x.name,
										value: x.value,
										signature: x.signature,
									})
									.collect(),
								gamemode: p.gamemode,
								ping: p.ping,
								display_name: p.display_name,
								$($field: $value,)*
							}),
							D::RemovePlayer => E::RemovePlayer,
							D::UpdateDisplayName(n) => E::UpdateDisplayName(n),
							D::UpdateGameMode(g) => E::UpdateGameMode(g),
							D::UpdateLatency(l) => E::UpdateLatency(l),
							D::Default(t) => E::Default(action(&t)),
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
	src: &v1_18_2::play_to_client_packet_trade_list_trades_items::Ty,
	_cx: &mut Session,
) -> Result<Slot> {
	Ok(src
		.input_item2
		.clone()
		.map_or(Slot::False, |s| slot(OLD, NEW, s)))
}

pub fn caves_trade_second_input_down_1_18_2(
	src: &v1_19::play_to_client_packet_trade_list_trades_items::Ty,
	_cx: &mut Session,
) -> Result<Option<Slot>> {
	Ok(match slot(NEW, OLD, src.input_item2.clone()) {
		Slot::False => None,
		s => Some(s),
	})
}

macro_rules! world_particles {
	($name:ident, $from:ident, $to:ident, $a:expr, $b:expr) => {
		pub fn $name(
			v: $from::play_to_client_packet_world_particles::Ty,
			_cx: &mut Session,
		) -> Result<$to::play_to_client_packet_world_particles::Ty> {
			let data = encode(&v.data, $from::particle_data::serialize)?;
			let Some((id, data)) = particle_data($a, $b, v.particle_id as i32, &data)? else {
				return Err(Error::Dropped);
			};
			let args = $to::particle_data::Args {
				compare_to: &(id as _),
			};
			let data =
				$to::particle_data::deserialize(&mut Input::new(&data), args).map_err(encoding)?;
			Ok($to::play_to_client_packet_world_particles::Ty {
				particle_id: id as _,
				long_distance: v.long_distance,
				x: v.x,
				y: v.y,
				z: v.z,
				offset_x: v.offset_x,
				offset_y: v.offset_y,
				offset_z: v.offset_z,
				particle_data: v.particle_data,
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
	src: &v1_18_2::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(world_event(OLD, NEW, src.effect_id, src.data))
}

pub fn caves_world_event_data_down_1_18_2(
	src: &v1_19::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(world_event(NEW, OLD, src.effect_id, src.data))
}

pub fn caves_block_action_id_1_19(
	src: &v1_18_2::play_to_client_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block(OLD, NEW, src.block_id).ok_or(Error::Dropped)
}

pub fn caves_block_action_id_down_1_18_2(
	src: &v1_19::play_to_client_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block(NEW, OLD, src.block_id).ok_or(Error::Dropped)
}

pub fn caves_cooldown_item_1_19(
	src: &v1_18_2::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	item(OLD, NEW, src.item_id).ok_or(Error::Dropped)
}

pub fn caves_cooldown_item_down_1_18_2(
	src: &v1_19::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	item(NEW, OLD, src.item_id).ok_or(Error::Dropped)
}

pub(super) fn now_millis() -> i64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map_or(0, |d| d.as_millis() as i64)
}

pub fn caves_chat_input_1_19(message: String, _cx: &mut Session) -> Result<Vec<Server19>> {
	Ok(vec![match message.strip_prefix('/') {
		Some(command) => Server19::ChatCommand(v1_19::play_to_server_packet_chat_command::Ty {
			command: command.to_owned(),
			timestamp: now_millis(),
			salt: 0,
			argument_signatures: vec![],
			signed_preview: false,
		}),
		None => Server19::ChatMessage(v1_19::play_to_server_packet_chat_message::Ty {
			message,
			timestamp: now_millis(),
			salt: 0,
			signature: vec![],
			signed_preview: false,
		}),
	}])
}

pub fn caves_chat_command_down_1_18_2(
	p: v1_19::play_to_server_packet_chat_command::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server18>> {
	Ok(vec![Server18::Chat(format!("/{}", p.command))])
}

pub fn caves_chat_message_down_1_18_2(
	p: v1_19::play_to_server_packet_chat_message::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server18>> {
	Ok(vec![Server18::Chat(p.message)])
}

pub fn caves_sequence_1_19<S>(_src: &S, cx: &mut Session) -> Result<i32> {
	let w = world(cx);
	w.sequence = w.sequence.wrapping_add(1) & i32::MAX;
	Ok(w.sequence)
}

pub fn caves_use_item_1_19(hand: i32, cx: &mut Session) -> Result<Vec<Server19>> {
	let sequence = caves_sequence_1_19(&(), cx)?;
	Ok(vec![Server19::UseItem(
		v1_19::play_to_server_packet_use_item::Ty { hand, sequence },
	)])
}

pub fn caves_use_item_down_1_18_2(
	p: v1_19::play_to_server_packet_use_item::Ty,
	cx: &mut Session,
) -> Result<Vec<Server18>> {
	acknowledged(cx, p.sequence)?;
	Ok(vec![Server18::UseItem(p.hand)])
}

pub fn caves_block_dig_down_1_18_2(
	p: v1_19::play_to_server_packet_block_dig::Ty,
	cx: &mut Session,
) -> Result<Vec<Server18>> {
	match p.status {
		0..=2 => requested(cx, p.sequence),
		_ => acknowledged(cx, p.sequence)?,
	}
	Ok(vec![Server18::BlockDig(
		v1_18_2::play_to_server_packet_block_dig::Ty {
			status: p.status,
			location: p.location,
			face: p.face,
		},
	)])
}

pub fn caves_block_place_down_1_18_2(
	p: v1_19::play_to_server_packet_block_place::Ty,
	cx: &mut Session,
) -> Result<Vec<Server18>> {
	requested(cx, p.sequence);
	Ok(vec![Server18::BlockPlace(
		v1_18_2::play_to_server_packet_block_place::Ty {
			hand: p.hand,
			location: p.location,
			direction: p.direction,
			cursor_x: p.cursor_x,
			cursor_y: p.cursor_y,
			cursor_z: p.cursor_z,
			inside_block: p.inside_block,
		},
	)])
}

pub fn caves_beacon_1_19(
	v: v1_18_2::play_to_server_packet_set_beacon_effect::Ty,
	_cx: &mut Session,
) -> Result<v1_19::play_to_server_packet_set_beacon_effect::Ty> {
	Ok(v1_19::play_to_server_packet_set_beacon_effect::Ty {
		primary_effect: (v.primary_effect >= 0).then_some(v.primary_effect),
		secondary_effect: (v.secondary_effect >= 0).then_some(v.secondary_effect),
	})
}

pub fn caves_beacon_down_1_18_2(
	v: v1_19::play_to_server_packet_set_beacon_effect::Ty,
	_cx: &mut Session,
) -> Result<v1_18_2::play_to_server_packet_set_beacon_effect::Ty> {
	Ok(v1_18_2::play_to_server_packet_set_beacon_effect::Ty {
		primary_effect: v.primary_effect.unwrap_or(-1),
		secondary_effect: v.secondary_effect.unwrap_or(-1),
	})
}

pub fn caves_player_chat_1_19_2(
	p: v1_19::play_to_client_packet_player_chat::Ty,
	cx: &mut Session,
) -> Result<Vec<Client192>> {
	let content = p
		.unsigned_chat_content
		.as_deref()
		.unwrap_or(&p.signed_chat_content);
	let (content, is_action_bar) = chat_types::decorate(
		cx,
		(NEW, NEWER),
		p.r#type,
		&p.sender_name,
		p.sender_team.as_deref(),
		content,
	)?;
	Ok(vec![Client192::SystemChat(
		v1_19_2::play_to_client_packet_system_chat::Ty {
			content,
			is_action_bar,
		},
	)])
}

pub fn caves_player_chat_down_1_19(
	p: v1_19_2::play_to_client_packet_player_chat::Ty,
	cx: &mut Session,
) -> Result<Vec<Client19>> {
	let content = p
		.unsigned_chat_content
		.or(p.formatted_message)
		.unwrap_or_else(|| chat_text(&p.plain_message));
	let (content, overlay) = chat_types::decorate(
		cx,
		(NEWER, NEW),
		p.r#type,
		&p.network_name,
		p.network_target_name.as_deref(),
		&content,
	)?;
	Ok(vec![Client19::SystemChat(
		v1_19::play_to_client_packet_system_chat::Ty {
			content,
			r#type: if overlay { 2 } else { 1 },
		},
	)])
}

pub fn caves_action_bar_1_19_2(
	src: &v1_19::play_to_client_packet_system_chat::Ty,
	cx: &mut Session,
) -> Result<bool> {
	Ok(overlay_1_19(cx, src.r#type))
}

pub fn caves_system_chat_type_down_1_19(
	src: &v1_19_2::play_to_client_packet_system_chat::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(if src.is_action_bar { 2 } else { 1 })
}

pub fn caves_codec_1_19_2(
	src: &v1_19::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	chat_types::track(cx, NEW, &src.dimension_codec);
	let mut codec = src.dimension_codec.clone();
	put_registry(&mut codec, NEWER, "minecraft:chat_type")?;
	Ok(codec)
}

pub fn caves_codec_down_1_19(
	src: &v1_19_2::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	chat_types::track(cx, NEWER, &src.dimension_codec);
	let mut codec = src.dimension_codec.clone();
	put_registry(&mut codec, NEW, "minecraft:chat_type")?;
	Ok(codec)
}

pub fn caves_metadata_down_1_19(
	src: &v1_19_2::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<v1_19::entity_metadata::Ty> {
	let dropped = dropped_keys(world(cx).name(src.entity_id), &[("allay", &[16, 17])]);
	let bytes = meta::convert(
		NEWER,
		NEW,
		&encode(&src.metadata, v1_19_2::entity_metadata::serialize)?,
		|m| Ok((!dropped.contains(&m.key)).then_some(m)),
	)?;
	decode(&bytes, v1_19::entity_metadata::deserialize)
}
