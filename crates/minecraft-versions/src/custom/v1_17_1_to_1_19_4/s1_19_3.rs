use super::chunk;
use super::meta::{self, Kind, Meta, MINECARTS};
use super::s1_19::{dropped_keys, minecart_block, section_records, world_event, world_particles};
use super::*;
use crate::gen::convert::{v1_19_2, v1_19_3};

type Client192 = v1_19_2::play_to_client_packet::Ty;
type Client193 = v1_19_3::play_to_client_packet::Ty;
type Properties192 = v1_19_2::command_node_extra_node_data_v2_properties::Ty;
type Properties193 = v1_19_3::command_node_extra_node_data_v2_properties::Ty;
type Info192 = v1_19_2::play_to_client_packet_player_info::Ty;
type Info193 = v1_19_3::play_to_client_packet_player_info::Ty;

const OLD: V = V::V1_19_2;
const NEW: V = V::V1_19_3;

pub fn caves_absent_1_19_3<S, T>(_src: &S, _cx: &mut Session) -> Result<Option<T>> {
	Ok(None)
}

pub fn caves_absent_down_1_19_2<S, T>(_src: &S, _cx: &mut Session) -> Result<Option<T>> {
	Ok(None)
}

pub fn caves_unsigned_down_1_19_2<S>(_src: &S, _cx: &mut Session) -> Result<Vec<u8>> {
	Ok(vec![])
}

pub fn caves_acknowledged_1_19_3<S>(_src: &S, _cx: &mut Session) -> Result<Vec<u8>> {
	Ok(vec![0; 3])
}

pub fn caves_slot_1_19_3(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot(OLD, NEW, v))
}

pub fn caves_slot_down_1_19_2(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot(NEW, OLD, v))
}

pub fn caves_particle_1_19_3(
	v: v1_19_2::particle::Ty,
	_cx: &mut Session,
) -> Result<v1_19_3::particle::Ty> {
	particle(
		OLD,
		NEW,
		&v,
		v1_19_2::particle::serialize,
		v1_19_3::particle::deserialize,
	)
}

pub fn caves_particle_down_1_19_2(
	v: v1_19_3::particle::Ty,
	_cx: &mut Session,
) -> Result<v1_19_2::particle::Ty> {
	particle(
		NEW,
		OLD,
		&v,
		v1_19_3::particle::serialize,
		v1_19_2::particle::deserialize,
	)
}

world_particles!(caves_world_particles_1_19_3, v1_19_2, v1_19_3, OLD, NEW);
world_particles!(
	caves_world_particles_down_1_19_2,
	v1_19_3,
	v1_19_2,
	NEW,
	OLD
);

pub fn caves_entity_summon_parser_1_19_3(_p: (), _cx: &mut Session) -> Result<Properties193> {
	Ok(Properties193::MinecraftResource(
		"minecraft:entity_type".into(),
	))
}

pub fn caves_enchantment_parser_1_19_3(_p: (), _cx: &mut Session) -> Result<Properties193> {
	Ok(Properties193::MinecraftResource(
		"minecraft:enchantment".into(),
	))
}

pub fn caves_mob_effect_parser_1_19_3(_p: (), _cx: &mut Session) -> Result<Properties193> {
	Ok(Properties193::MinecraftResource(
		"minecraft:mob_effect".into(),
	))
}

pub fn caves_gamemode_parser_down_1_19_2(_p: (), _cx: &mut Session) -> Result<Properties192> {
	Ok(Properties192::BrigadierString(
		v1_19_2::command_node_extra_node_data_v2_properties_brigadier_string::Ty::SingleWord,
	))
}

pub fn caves_resource_key_parser_down_1_19_2(
	registry: String,
	_cx: &mut Session,
) -> Result<Properties192> {
	Ok(Properties192::MinecraftResource(registry))
}

pub fn caves_resource_or_tag_key_parser_down_1_19_2(
	registry: String,
	_cx: &mut Session,
) -> Result<Properties192> {
	Ok(Properties192::MinecraftResourceOrTag(registry))
}

pub fn caves_encryption_verify_token_1_19_3(
	src: &v1_19_2::login_to_server_packet_encryption_begin::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	match &src.crypto {
		v1_19_2::login_to_server_packet_encryption_begin_crypto::Ty::True(token) => {
			Ok(token.clone())
		}
		_ => unconvertible("a 1.19.3 server only accepts the verify token, not a signed nonce"),
	}
}

pub fn caves_encryption_crypto_down_1_19_2(
	src: &v1_19_3::login_to_server_packet_encryption_begin::Ty,
	_cx: &mut Session,
) -> Result<v1_19_2::login_to_server_packet_encryption_begin_crypto::Ty> {
	Ok(v1_19_2::login_to_server_packet_encryption_begin_crypto::Ty::True(src.verify_token.clone()))
}

fn sound_name(version: V, id: i32) -> Result<String> {
	let name = registry::names(version, "sounds")
		.and_then(|n| n.name(id as u32))
		.ok_or(Error::Dropped)?;
	Ok(format!("minecraft:{name}"))
}

fn sound_id(version: V, name: &str) -> Option<i32> {
	registry::names(version, "sounds")
		.and_then(|n| n.id(name))
		.map(|i| i as i32)
}

enum Sound {
	Id(i32),
	Named(String),
}

fn sound_up(id: i32) -> Result<Sound> {
	Ok(match sound(OLD, NEW, id) {
		Ok(id) => Sound::Id(id),
		Err(_) => Sound::Named(sound_name(OLD, id)?),
	})
}

fn sound_down(id: Option<i32>, name: Option<&str>) -> Result<Sound> {
	let id = id.or_else(|| name.and_then(|n| sound_id(NEW, n)));
	if let Some(old) = id.and_then(|id| sound(NEW, OLD, id).ok()) {
		return Ok(Sound::Id(old));
	}
	match (name, id) {
		(Some(n), _) => Ok(sound_id(OLD, n).map_or_else(|| Sound::Named(n.to_owned()), Sound::Id)),
		(None, Some(id)) => Ok(Sound::Named(sound_name(NEW, id)?)),
		(None, None) => Err(Error::Dropped),
	}
}

fn holder(sound: Sound) -> v1_19_3::item_sound_holder::Ty {
	match sound {
		Sound::Id(id) => v1_19_3::item_sound_holder::Ty::Id(id),
		Sound::Named(sound_name) => {
			v1_19_3::item_sound_holder::Ty::Inline(v1_19_3::item_sound_event::Ty {
				sound_name,
				fixed_range: None,
			})
		}
	}
}

fn source(category: i32) -> Result<v1_19_3::sound_source::Ty> {
	let mut b = Vec::new();
	write_varint(&mut b, category);
	decode(&b, v1_19_3::sound_source::deserialize)
}

fn category(source: &v1_19_3::sound_source::Ty) -> Result<i32> {
	let b = encode(source, v1_19_3::sound_source::serialize)?;
	read_varint(&b, &mut 0)
}

pub fn caves_sound_holder_1_19_3(
	src: &v1_19_2::play_to_client_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<v1_19_3::item_sound_holder::Ty> {
	Ok(holder(sound_up(src.sound_id)?))
}

pub fn caves_sound_effect_down_1_19_2(
	p: v1_19_3::play_to_client_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client192>> {
	let sound = match &p.sound {
		v1_19_3::item_sound_holder::Ty::Id(id) => sound_down(Some(*id), None)?,
		v1_19_3::item_sound_holder::Ty::Inline(e) => sound_down(None, Some(&e.sound_name))?,
	};
	let sound_category = category(&p.sound_category)?;
	Ok(vec![match sound {
		Sound::Id(sound_id) => {
			Client192::SoundEffect(v1_19_2::play_to_client_packet_sound_effect::Ty {
				sound_id,
				sound_category,
				x: p.x,
				y: p.y,
				z: p.z,
				volume: p.volume,
				pitch: p.pitch,
				seed: p.seed,
			})
		}
		Sound::Named(sound_name) => {
			Client192::NamedSoundEffect(v1_19_2::play_to_client_packet_named_sound_effect::Ty {
				sound_name,
				sound_category,
				x: p.x,
				y: p.y,
				z: p.z,
				volume: p.volume,
				pitch: p.pitch,
				seed: p.seed,
			})
		}
	}])
}

pub fn caves_sound_source_1_19_3(
	src: &v1_19_2::play_to_client_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<v1_19_3::sound_source::Ty> {
	source(src.sound_category)
}

pub fn caves_entity_sound_event_1_19_3(
	src: &v1_19_2::play_to_client_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<v1_19_3::play_to_client_packet_entity_sound_effect_sound_event::Ty> {
	use v1_19_3::play_to_client_packet_entity_sound_effect_sound_event::Ty as Event;
	Ok(match sound_up(src.sound_id)? {
		Sound::Id(id) => Event::Default(id + 1),
		Sound::Named(resource) => Event::V0(
			v1_19_3::play_to_client_packet_entity_sound_effect_sound_event_v0::Ty {
				resource,
				range: None,
			},
		),
	})
}

pub fn caves_entity_sound_id_down_1_19_2(
	src: &v1_19_3::play_to_client_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	use v1_19_3::play_to_client_packet_entity_sound_effect_sound_event::Ty as Event;
	let sound = match &src.sound_event {
		Event::V0(e) => sound_down(None, Some(&e.resource))?,
		Event::Default(t) => sound_down(Some(t - 1), None)?,
	};
	match sound {
		Sound::Id(id) => Ok(id),
		Sound::Named(_) => Err(Error::Dropped),
	}
}

pub fn caves_entity_sound_source_1_19_3(
	src: &v1_19_2::play_to_client_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<v1_19_3::sound_source::Ty> {
	source(src.sound_category)
}

pub fn caves_entity_sound_source_down_1_19_2(
	src: &v1_19_3::play_to_client_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	category(&src.sound_category)
}

pub fn caves_named_sound_1_19_3(
	p: v1_19_2::play_to_client_packet_named_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client193>> {
	let sound = sound_id(NEW, &p.sound_name).map_or(Sound::Named(p.sound_name), Sound::Id);
	Ok(vec![Client193::SoundEffect(
		v1_19_3::play_to_client_packet_sound_effect::Ty {
			sound: holder(sound),
			sound_category: source(p.sound_category)?,
			x: p.x,
			y: p.y,
			z: p.z,
			volume: p.volume,
			pitch: p.pitch,
			seed: p.seed,
		},
	)])
}

pub fn caves_hide_message_1_19_3(signature: Vec<u8>, _cx: &mut Session) -> Result<Vec<Client193>> {
	match signature.len() {
		256 => Ok(vec![Client193::HideMessage(
			v1_19_3::play_to_client_packet_hide_message::Ty::V0(signature),
		)]),
		_ => Ok(vec![]),
	}
}

pub fn caves_hide_message_down_1_19_2(
	p: v1_19_3::play_to_client_packet_hide_message::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client192>> {
	Ok(match p {
		v1_19_3::play_to_client_packet_hide_message::Ty::V0(signature) => {
			vec![Client192::HideMessage(signature)]
		}
		_ => vec![],
	})
}

fn decorate_1_19_2(
	cx: &mut Session,
	ty: i32,
	sender: &str,
	target: Option<&str>,
	content: &str,
) -> Result<Vec<Client192>> {
	let (content, is_action_bar) =
		chat_types::decorate(cx, (NEW, OLD), ty, sender, target, content)?;
	Ok(vec![Client192::SystemChat(
		v1_19_2::play_to_client_packet_system_chat::Ty {
			content,
			is_action_bar,
		},
	)])
}

pub fn caves_player_chat_1_19_3(
	p: v1_19_2::play_to_client_packet_player_chat::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client193>> {
	let message = p
		.unsigned_chat_content
		.or(p.formatted_message)
		.unwrap_or_else(|| chat_text(&p.plain_message));
	Ok(vec![Client193::ProfilelessChat(
		v1_19_3::play_to_client_packet_profileless_chat::Ty {
			message,
			r#type: p.r#type,
			name: p.network_name,
			target: p.network_target_name,
		},
	)])
}

pub fn caves_player_chat_down_1_19_2(
	p: v1_19_3::play_to_client_packet_player_chat::Ty,
	cx: &mut Session,
) -> Result<Vec<Client192>> {
	let content = p
		.unsigned_chat_content
		.unwrap_or_else(|| chat_text(&p.plain_message));
	decorate_1_19_2(
		cx,
		p.r#type,
		&p.network_name,
		p.network_target_name.as_deref(),
		&content,
	)
}

pub fn caves_profileless_chat_down_1_19_2(
	p: v1_19_3::play_to_client_packet_profileless_chat::Ty,
	cx: &mut Session,
) -> Result<Vec<Client192>> {
	decorate_1_19_2(cx, p.r#type, &p.name, p.target.as_deref(), &p.message)
}

pub fn caves_login_1_19_3(
	p: v1_19_2::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client193>> {
	world(cx).enter(&p.dimension_codec, &p.world_type);
	Ok(vec![
		Client193::Login(p),
		Client193::FeatureFlags(vec!["minecraft:vanilla".into()]),
	])
}

pub fn caves_login_down_1_19_2(
	p: v1_19_3::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client192>> {
	chat_types::track(cx, NEW, &p.dimension_codec);
	world(cx).enter(&p.dimension_codec, &p.world_type);
	Ok(vec![Client192::Login(p)])
}

pub fn caves_player_info_1_19_3(p: Info192, _cx: &mut Session) -> Result<Vec<Client193>> {
	use v1_19_2::play_to_client_packet_player_info_action::Ty as A;
	use v1_19_2::play_to_client_packet_player_info_data_items_anon::Ty as D;
	use v1_19_3::play_to_client_packet_player_info_data_items_chat_session::Ty as Chat;
	use v1_19_3::play_to_client_packet_player_info_data_items_display_name::Ty as Name;
	use v1_19_3::play_to_client_packet_player_info_data_items_gamemode::Ty as Mode;
	use v1_19_3::play_to_client_packet_player_info_data_items_latency::Ty as Latency;
	use v1_19_3::play_to_client_packet_player_info_data_items_listed::Ty as Listed;
	use v1_19_3::play_to_client_packet_player_info_data_items_player::Ty as Player;
	if let A::RemovePlayer = p.action {
		return Ok(vec![Client193::PlayerRemove(
			p.data.into_iter().map(|d| d.uuid).collect(),
		)]);
	}
	let add = matches!(p.action, A::AddPlayer);
	let action = v1_19_3::play_to_client_packet_player_info_action::Ty {
		add_player: add,
		initialize_chat: add,
		update_game_mode: add || matches!(p.action, A::UpdateGameMode),
		update_listed: add,
		update_latency: add || matches!(p.action, A::UpdateLatency),
		update_display_name: add || matches!(p.action, A::UpdateDisplayName),
	};
	let data = p
		.data
		.into_iter()
		.map(|d| {
			let mut e = v1_19_3::play_to_client_packet_player_info_data_items::Ty {
				uuid: d.uuid,
				player: Player::Default(false),
				chat_session: Chat::Default(false),
				gamemode: Mode::Default(false),
				listed: Listed::Default(false),
				latency: Latency::Default(false),
				display_name: Name::Default(false),
			};
			match d.anon {
				D::AddPlayer(a) => {
					e.player = Player::True(v1_19_3::game_profile::Ty {
						name: a.name,
						properties: a
							.properties
							.into_iter()
							.map(|x| v1_19_3::game_profile_properties_items::Ty {
								name: x.name,
								value: x.value,
								signature: x.signature,
							})
							.collect(),
					});
					e.chat_session = Chat::True(None);
					e.gamemode = Mode::True(a.gamemode);
					e.listed = Listed::True(1);
					e.latency = Latency::True(a.ping);
					e.display_name = Name::True(a.display_name);
				}
				D::UpdateGameMode(g) => e.gamemode = Mode::True(g),
				D::UpdateLatency(l) => e.latency = Latency::True(l),
				D::UpdateDisplayName(n) => e.display_name = Name::True(n),
				_ => {}
			}
			e
		})
		.collect();
	Ok(vec![Client193::PlayerInfo(Info193 { action, data })])
}

pub fn caves_player_info_down_1_19_2(p: Info193, _cx: &mut Session) -> Result<Vec<Client192>> {
	use v1_19_2::play_to_client_packet_player_info_action::Ty as A;
	use v1_19_2::play_to_client_packet_player_info_data_items_anon::Ty as D;
	use v1_19_3::play_to_client_packet_player_info_data_items_display_name::Ty as Name;
	use v1_19_3::play_to_client_packet_player_info_data_items_gamemode::Ty as Mode;
	use v1_19_3::play_to_client_packet_player_info_data_items_latency::Ty as Latency;
	use v1_19_3::play_to_client_packet_player_info_data_items_player::Ty as Player;
	let item =
		|uuid, anon| v1_19_2::play_to_client_packet_player_info_data_items::Ty { uuid, anon: anon };
	let packet = |action, data| Client192::PlayerInfo(Info192 { action, data });
	let mode = |m: &Mode| {
		if let Mode::True(g) = m {
			Some(*g)
		} else {
			None
		}
	};
	let latency = |l: &Latency| {
		if let Latency::True(v) = l {
			Some(*v)
		} else {
			None
		}
	};
	let name = |n: &Name| {
		if let Name::True(v) = n {
			Some(v.clone())
		} else {
			None
		}
	};
	if p.action.add_player {
		let data = p
			.data
			.iter()
			.filter_map(|d| {
				let Player::True(profile) = &d.player else { return None };
				Some(item(
					d.uuid,
					D::AddPlayer(v1_19_2::play_to_client_packet_player_info_data_items_anon_add_player::Ty {
						name: profile.name.clone(),
						properties: profile
							.properties
							.iter()
							.map(|x| v1_19_2::play_to_client_packet_player_info_data_items_anon_add_player_properties_items::Ty {
								name: x.name.clone(),
								value: x.value.clone(),
								signature: x.signature.clone(),
							})
							.collect(),
						gamemode: mode(&d.gamemode).unwrap_or(0),
						ping: latency(&d.latency).unwrap_or(0),
						display_name: name(&d.display_name).flatten(),
						crypto: None,
					}),
				))
			})
			.collect();
		return Ok(vec![packet(A::AddPlayer, data)]);
	}
	let mut out = Vec::new();
	if p.action.update_game_mode {
		out.push(packet(
			A::UpdateGameMode,
			p.data
				.iter()
				.filter_map(|d| Some(item(d.uuid, D::UpdateGameMode(mode(&d.gamemode)?))))
				.collect(),
		));
	}
	if p.action.update_latency {
		out.push(packet(
			A::UpdateLatency,
			p.data
				.iter()
				.filter_map(|d| Some(item(d.uuid, D::UpdateLatency(latency(&d.latency)?))))
				.collect(),
		));
	}
	if p.action.update_display_name {
		let data = p
			.data
			.iter()
			.filter_map(|d| Some(item(d.uuid, D::UpdateDisplayName(name(&d.display_name)?))))
			.collect();
		out.push(packet(A::UpdateDisplayName, data));
	}
	Ok(out)
}

pub fn caves_player_remove_down_1_19_2(
	uuids: Vec<u128>,
	_cx: &mut Session,
) -> Result<Vec<Client192>> {
	use v1_19_2::play_to_client_packet_player_info_data_items_anon::Ty as D;
	Ok(vec![Client192::PlayerInfo(Info192 {
		action: v1_19_2::play_to_client_packet_player_info_action::Ty::RemovePlayer,
		data: uuids
			.into_iter()
			.map(
				|uuid| v1_19_2::play_to_client_packet_player_info_data_items::Ty {
					uuid,
					anon: D::RemovePlayer,
				},
			)
			.collect(),
	})])
}

pub fn caves_block_change_1_19_3(
	p: v1_19_2::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client193>> {
	Ok(vec![Client193::BlockChange(
		v1_19_3::play_to_client_packet_block_change::Ty {
			location: p.location,
			r#type: state(OLD, NEW, p.r#type),
		},
	)])
}

pub fn caves_block_change_down_1_19_2(
	p: v1_19_3::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client192>> {
	Ok(vec![Client192::BlockChange(
		v1_19_2::play_to_client_packet_block_change::Ty {
			location: p.location,
			r#type: state(NEW, OLD, p.r#type),
		},
	)])
}

pub fn caves_section_records_1_19_3(
	src: &v1_19_2::play_to_client_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<i64>> {
	Ok(section_records(OLD, NEW, &src.records))
}

pub fn caves_section_records_down_1_19_2(
	src: &v1_19_3::play_to_client_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<i64>> {
	Ok(section_records(NEW, OLD, &src.records))
}

pub(super) fn block_entities<T>(
	from: V,
	to: V,
	list: Vec<T>,
	ty: impl Fn(&mut T) -> &mut i32,
) -> Vec<T> {
	list.into_iter()
		.filter_map(|mut b| {
			let t = ty(&mut b);
			*t = block_entity_type(from, to, *t)?;
			Some(b)
		})
		.collect()
}

pub fn caves_chunk_entities(
	from: V,
	to: V,
	list: Vec<v1_19_3::chunk_block_entity::Ty>,
) -> Vec<v1_19_3::chunk_block_entity::Ty> {
	block_entities(from, to, list, |b| &mut b.r#type)
}

pub fn caves_map_chunk_1_19_3(
	mut p: v1_19_2::play_to_client_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<v1_19_3::play_to_client_packet_map_chunk::Ty> {
	p.chunk_data = chunk::remap_states(&p.chunk_data, world(cx).known_sections(), OLD, NEW)?;
	p.block_entities = block_entities(OLD, NEW, p.block_entities, |b| &mut b.r#type);
	Ok(p)
}

pub fn caves_map_chunk_down_1_19_2(
	mut p: v1_19_3::play_to_client_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<v1_19_2::play_to_client_packet_map_chunk::Ty> {
	p.chunk_data = chunk::remap_states(&p.chunk_data, world(cx).known_sections(), NEW, OLD)?;
	p.block_entities = block_entities(NEW, OLD, p.block_entities, |b| &mut b.r#type);
	Ok(p)
}

pub fn caves_tile_entity_action_1_19_3(
	src: &v1_19_2::play_to_client_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block_entity_type(OLD, NEW, src.action).ok_or(Error::Dropped)
}

pub fn caves_tile_entity_action_down_1_19_2(
	src: &v1_19_3::play_to_client_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block_entity_type(NEW, OLD, src.action).ok_or(Error::Dropped)
}

pub(super) fn spawn(
	from: V,
	to: V,
	mut p: v1_19_3::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<v1_19_3::play_to_client_packet_spawn_entity::Ty> {
	let w = world(cx);
	w.spawned(p.entity_id, from, p.r#type);
	if w.is(p.entity_id, &["falling_block"]) {
		p.object_data = state(from, to, p.object_data);
	}
	p.r#type = entity_type(from, to, p.r#type).ok_or(Error::Dropped)?;
	Ok(p)
}

pub fn caves_spawn_entity_1_19_3(
	p: v1_19_2::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<v1_19_3::play_to_client_packet_spawn_entity::Ty> {
	spawn(OLD, NEW, p, cx)
}

pub fn caves_spawn_entity_down_1_19_2(
	p: v1_19_3::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<v1_19_2::play_to_client_packet_spawn_entity::Ty> {
	spawn(NEW, OLD, p, cx)
}

fn pose(up: bool, m: Meta) -> Meta {
	match (m.key, m.kind, m.int()) {
		(6, Kind::Pose, Some(p)) if p >= 10 => Meta::varint(
			6,
			Kind::Pose,
			if up {
				p + 1
			} else if p == 10 {
				0
			} else {
				p - 1
			},
		),
		_ => m,
	}
}

pub fn caves_metadata_1_19_3(
	src: &v1_19_2::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<v1_19_3::entity_metadata::Ty> {
	let mut minecart = minecart_block(OLD, NEW, world(cx).is(src.entity_id, MINECARTS));
	let bytes = meta::convert(
		OLD,
		NEW,
		&encode(&src.metadata, v1_19_2::entity_metadata::serialize)?,
		|m| minecart(pose(true, m)),
	)?;
	decode(&bytes, v1_19_3::entity_metadata::deserialize)
}

pub fn caves_metadata_down_1_19_2(
	src: &v1_19_3::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<v1_19_2::entity_metadata::Ty> {
	let w = world(cx);
	let dropped = dropped_keys(w.name(src.entity_id), &[("camel", &[19, 20])]);
	let mut minecart = minecart_block(NEW, OLD, w.is(src.entity_id, MINECARTS));
	let bytes = meta::convert(
		NEW,
		OLD,
		&encode(&src.metadata, v1_19_3::entity_metadata::serialize)?,
		|m| match m.kind {
			Kind::Long => Ok(None),
			_ if dropped.contains(&m.key) => Ok(None),
			_ => minecart(pose(false, m)),
		},
	)?;
	decode(&bytes, v1_19_2::entity_metadata::deserialize)
}

fn removed_event_sound(effect: i32) -> Option<&'static str> {
	Some(match effect {
		1005 => "minecraft:block.iron_door.open",
		1006 => "minecraft:block.wooden_door.open",
		1007 => "minecraft:block.wooden_trapdoor.open",
		1008 => "minecraft:block.fence_gate.open",
		1011 => "minecraft:block.iron_door.close",
		1012 => "minecraft:block.wooden_door.close",
		1013 => "minecraft:block.wooden_trapdoor.close",
		1014 => "minecraft:block.fence_gate.close",
		1036 => "minecraft:block.iron_trapdoor.close",
		1037 => "minecraft:block.iron_trapdoor.open",
		_ => return None,
	})
}

pub fn caves_world_event_1_19_3(
	p: v1_19_2::play_to_client_packet_world_event::Ty,
	cx: &mut Session,
) -> Result<Vec<Client193>> {
	let Some(sound) = removed_event_sound(p.effect_id) else {
		return Ok(vec![Client193::WorldEvent(
			v1_19_3::play_to_client_packet_world_event::Ty {
				data: world_event(OLD, NEW, p.effect_id, p.data),
				..p
			},
		)]);
	};
	let seed = world(cx).random();
	let l = &p.location;
	Ok(vec![Client193::SoundEffect(
		v1_19_3::play_to_client_packet_sound_effect::Ty {
			sound: holder(
				sound_id(NEW, sound).map_or_else(|| Sound::Named(sound.into()), Sound::Id),
			),
			sound_category: v1_19_3::sound_source::Ty::Block,
			x: l.x * 8 + 4,
			y: l.y as i32 * 8 + 4,
			z: l.z * 8 + 4,
			volume: 1.0,
			pitch: 0.9 + (seed as u64 % 1000) as f32 / 10000.0,
			seed,
		},
	)])
}

pub fn caves_world_event_data_down_1_19_2(
	src: &v1_19_3::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(world_event(NEW, OLD, src.effect_id, src.data))
}

pub fn caves_block_action_id_1_19_3(
	src: &v1_19_2::play_to_client_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block(OLD, NEW, src.block_id).ok_or(Error::Dropped)
}

pub fn caves_block_action_id_down_1_19_2(
	src: &v1_19_3::play_to_client_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block(NEW, OLD, src.block_id).ok_or(Error::Dropped)
}

pub fn caves_cooldown_item_1_19_3(
	src: &v1_19_2::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	item(OLD, NEW, src.item_id).ok_or(Error::Dropped)
}

pub fn caves_cooldown_item_down_1_19_2(
	src: &v1_19_3::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	item(NEW, OLD, src.item_id).ok_or(Error::Dropped)
}

pub fn caves_tags_1_19_3(p: Tags, _cx: &mut Session) -> Result<Vec<Client193>> {
	Ok(vec![Client193::Tags(tags(
		OLD,
		NEW,
		p,
		&[],
		super::new_tags::NEW_1_19_3,
	))])
}

pub fn caves_tags_down_1_19_2(p: Tags, _cx: &mut Session) -> Result<Vec<Client192>> {
	Ok(vec![Client192::Tags(tags(NEW, OLD, p, &[], &[]))])
}

pub fn caves_statistics_1_19_3(p: Stats, _cx: &mut Session) -> Result<Vec<Client193>> {
	Ok(vec![Client193::Statistics(statistics(OLD, NEW, p))])
}

pub fn caves_statistics_down_1_19_2(p: Stats, _cx: &mut Session) -> Result<Vec<Client192>> {
	Ok(vec![Client192::Statistics(statistics(NEW, OLD, p))])
}
