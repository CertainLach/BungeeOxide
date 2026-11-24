use super::registries;
use super::remap::{self, remaps};
use super::rules::{blocks, entities, hidden, items, statistics};
use super::shared::{self, meta};
use super::slots::{self, exact_matchers, untrusted_step};
use super::tags::Derived;
use super::util::{encode, lp_vec3, read_lp_vec3, read_varint};
use super::world;
use crate::Session;
use crate::generated::convert::v1_21_5::configuration_to_client_packet_tags_items_tags_items as tag;
use crate::generated::convert::{Root, Version, v26_3 as a, v26_4_snapshot_10 as b};
use crate::pds::convert::{Error, Result};
use crate::pds_native::{pds_entityDelta, pds_vecDelta};
use bungeeoxide_nbt::{Payload, Tag};
use std::collections::{HashMap, HashSet};

const OLD: Version = Version::V26_3;
const NEW: Version = Version::V26_4_snapshot_10;

remaps!(Version::V26_3 => Version::V26_4_snapshot_10 { late_sound_up_26_4: a::item_sound_holder::Ty = remap::sound; });
remaps!(Version::V26_4_snapshot_10 => Version::V26_3 { late_sound_down_26_3: a::item_sound_holder::Ty = remap::sound; });

items!(a => b, Version::V26_3 => Version::V26_4_snapshot_10 {
	slot: late_slot_item_up_26_4, hashed: late_hashed_item_up_26_4, trade1: late_trade1_up_26_4, trade2: late_trade2_up_26_4,
	display: late_display_item_up_26_4, recipes: late_recipe_items_up_26_4, stonecutter: late_stonecutter_up_26_4,
	requirements: late_requirements_up_26_4, repairable: late_repairable_up_26_4,
});
items!(b => a, Version::V26_4_snapshot_10 => Version::V26_3 {
	slot: late_slot_item_down_26_3, hashed: late_hashed_item_down_26_3, trade1: late_trade1_down_26_3, trade2: late_trade2_down_26_3,
	display: late_display_item_down_26_3, recipes: late_recipe_items_down_26_3, stonecutter: late_stonecutter_down_26_3,
	requirements: late_requirements_down_26_3, repairable: late_repairable_down_26_3,
});

blocks!(a => b, Version::V26_3 => Version::V26_4_snapshot_10 {
	block_change: late_block_change_up_26_4, multi_block: late_multi_block_up_26_4, block_action: late_block_action_up_26_4,
	world_event: late_world_event_up_26_4, chunk: late_chunk_up_26_4, tool: late_tool_up_26_4, predicate: late_predicate_up_26_4,
	particle: [late_particle_block_up_26_4, late_particle_marker_up_26_4, late_particle_dust_up_26_4, late_particle_pillar_up_26_4, late_particle_crumble_up_26_4],
	meta: [late_meta_state_up_26_4, late_meta_optional_state_up_26_4],
});
blocks!(b => a, Version::V26_4_snapshot_10 => Version::V26_3 {
	block_change: late_block_change_down_26_3, multi_block: late_multi_block_down_26_3, block_action: late_block_action_down_26_3,
	world_event: late_world_event_down_26_3, chunk: late_chunk_down_26_3, tool: late_tool_down_26_3, predicate: late_predicate_down_26_3,
	particle: [late_particle_block_down_26_3, late_particle_marker_down_26_3, late_particle_dust_down_26_3, late_particle_pillar_down_26_3, late_particle_crumble_down_26_3],
	meta: [late_meta_state_down_26_3, late_meta_optional_state_down_26_3],
});

entities!(a => b, Version::V26_3 => Version::V26_4_snapshot_10 { spawn: late_spawn_type_up_26_4, object: late_object_data_up_26_4, equippable: late_equip_entities_up_26_4, });
entities!(b => a, Version::V26_4_snapshot_10 => Version::V26_3 { spawn: late_spawn_type_down_26_3, object: late_object_data_down_26_3, equippable: late_equip_entities_down_26_3, });

statistics!(a => b, Version::V26_3 => Version::V26_4_snapshot_10 { late_statistics_up_26_4 });
statistics!(b => a, Version::V26_4_snapshot_10 => Version::V26_3 { late_statistics_down_26_3 });

hidden!(a, Version::V26_3 => Version::V26_4_snapshot_10, late_hidden_up_26_4);
hidden!(b, Version::V26_4_snapshot_10 => Version::V26_3, late_hidden_down_26_3);

untrusted_step!(late_untrusted_up_26_4: a => b, Version::V26_3 => Version::V26_4_snapshot_10, slots::late_to_slot_26_3, slots::late_to_untrusted_26_4);
untrusted_step!(late_untrusted_down_26_3: b => a, Version::V26_4_snapshot_10 => Version::V26_3, slots::late_to_slot_26_4, slots::late_to_untrusted_26_3);

exact_matchers!(exact_up: a => b, Version::V26_3 => Version::V26_4_snapshot_10);
exact_matchers!(exact_down: b => a, Version::V26_4_snapshot_10 => Version::V26_3);

pub fn late_matchers_up_26_4(
	v: &a::item_block_predicate::Ty,
	cx: &mut Session,
) -> Result<b::data_component_matchers::Ty> {
	Ok(b::data_component_matchers::Ty {
		exact_matchers: exact_up(&v.components.exact_matchers, cx)?,
		partial_matchers: v
			.components
			.partial_matchers
			.iter()
			.map(|p| b::data_component_matchers_partial_matchers_items::Ty {
				is_predicate_type: p.is_predicate_type,
				r#type: p.r#type,
				predicate: p.predicate.clone(),
			})
			.collect(),
	})
}

pub fn late_matchers_down_26_3(
	v: &b::item_block_predicate::Ty,
	cx: &mut Session,
) -> Result<a::data_component_matchers::Ty> {
	Ok(a::data_component_matchers::Ty {
		exact_matchers: exact_down(&v.components.exact_matchers, cx)?,
		partial_matchers: v
			.components
			.partial_matchers
			.iter()
			.map(|p| a::data_component_matchers_partial_matchers_items::Ty {
				is_predicate_type: p.is_predicate_type,
				r#type: p.r#type,
				predicate: p.predicate.clone(),
			})
			.collect(),
	})
}

pub fn late_template_item_up_26_4(
	v: &a::item_stack_template::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap::item(OLD, NEW, v.item_id)
}

pub fn late_template_item_down_26_3(
	v: &b::item_stack_template::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap::item(NEW, OLD, v.item_id)
}

fn surviving<T>(
	items: &[T],
	ser: impl Fn(&T, &mut crate::pds::Output) -> crate::pds::Result<()>,
) -> Result<i32> {
	let mut n = 0;
	for c in items {
		let bytes = encode(c, &ser)?;
		let mut at = 0;
		n += super::components::remap(NEW, OLD, read_varint(&bytes, &mut at)?).is_some() as i32;
	}
	Ok(n)
}

macro_rules! counts {
	($($name:ident: $ty:ty, $field:ident, $ser:path;)*) => {
		$(
			pub fn $name(v: &$ty, _cx: &mut Session) -> Result<i32> {
				surviving(&v.$field, $ser)
			}
		)*
	};
}

counts! {
	late_added_count_down_26_3: b::slot_default::Ty, components, b::slot_component::serialize;
	late_removed_count_down_26_3: b::slot_default::Ty, remove_components, b::slot_component_type::serialize;
	late_template_added_down_26_3: b::item_stack_template::Ty, components, b::slot_component::serialize;
	late_template_removed_down_26_3: b::item_stack_template::Ty, remove_components, b::slot_component_type::serialize;
}

const POP_CHANCE: f32 = 0.5;
const CURRENT_RADIUS: f32 = 0.5;

pub fn late_particle_freezing_down_26_3(_v: (), _cx: &mut Session) -> Result<a::particle::Ty> {
	Ok(a::particle::Ty::Snowflake)
}

pub fn late_particle_snowball_up_26_4(_v: (), _cx: &mut Session) -> Result<b::particle::Ty> {
	let snowball = crate::registry::names(NEW, "items")
		.and_then(|n| n.id("snowball"))
		.ok_or(Error::Dropped)?;
	Ok(b::particle::Ty::Item(b::item_stack_template::Ty {
		item_id: snowball as i32,
		item_count: 1,
		added_component_count: 0,
		removed_component_count: 0,
		components: Vec::new(),
		remove_components: Vec::new(),
	}))
}

pub fn late_particle_bubble_column_up_26_4(_v: (), _cx: &mut Session) -> Result<b::particle::Ty> {
	Ok(b::particle::Ty::BubbleColumnUp(POP_CHANCE))
}

pub fn late_particle_current_down_up_26_4(_v: (), _cx: &mut Session) -> Result<b::particle::Ty> {
	Ok(b::particle::Ty::CurrentDown(CURRENT_RADIUS))
}

pub fn late_particle_bubble_column_down_26_3(
	_v: f32,
	_cx: &mut Session,
) -> Result<a::particle::Ty> {
	Ok(a::particle::Ty::BubbleColumnUp)
}

pub fn late_particle_current_down_down_26_3(_v: f32, _cx: &mut Session) -> Result<a::particle::Ty> {
	Ok(a::particle::Ty::CurrentDown)
}

const SILENT: &str = "intentionally_empty";

fn silent() -> a::item_sound_holder::Ty {
	a::item_sound_holder::Ty::Id(remap::sound_id(OLD, SILENT).unwrap_or(0))
}

fn sound_up(v: &a::item_sound_holder::Ty) -> Option<b::item_sound_holder::Ty> {
	if let a::item_sound_holder::Ty::Id(id) = v {
		if remap::sound_name(OLD, *id) == Some(SILENT) {
			return None;
		}
	}
	remap::sound(OLD, NEW, v.clone()).ok()
}

fn effect_down(id: i32) -> Result<i32> {
	super::util::remap("effects", NEW, OLD, id).ok_or(Error::Dropped)
}

const OLD_POTIONS: i32 = 46;

pub fn late_effect_down_26_3(v: &b::item_potion_effect::Ty, _cx: &mut Session) -> Result<i32> {
	effect_down(v.id)
}

pub fn late_stew_effect_down_26_3(
	v: &b::slot_component_suspicious_stew_effects_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	effect_down(v.effect)
}

pub fn late_entity_effect_down_26_3(
	v: &b::play_to_client_packet_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	effect_down(v.effect_id)
}

pub fn late_remove_effect_down_26_3(
	v: &b::play_to_client_packet_remove_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	effect_down(v.effect_id)
}

pub fn late_potion_down_26_3(
	v: &b::slot_component_potion_contents::Ty,
	_cx: &mut Session,
) -> Result<Option<i32>> {
	Ok(v.potion_id.filter(|p| *p < OLD_POTIONS))
}

pub fn late_remove_effects_down_26_3(
	v: b::id_set::Ty,
	_cx: &mut Session,
) -> Result<a::item_consume_effect::Ty> {
	Ok(a::item_consume_effect::Ty::RemoveEffects(match v {
		b::id_set::Ty::Ids(ids) => b::id_set::Ty::Ids(
			ids.into_iter()
				.filter_map(|id| effect_down(id).ok())
				.collect(),
		),
		named => named,
	}))
}

fn sound_down(v: &Option<b::item_sound_holder::Ty>) -> a::item_sound_holder::Ty {
	v.clone()
		.and_then(|s| remap::sound(NEW, OLD, s).ok())
		.unwrap_or_else(silent)
}

pub fn late_consume_sound_up_26_4(
	v: &a::slot_component_consumable::Ty,
	_cx: &mut Session,
) -> Result<Option<b::item_sound_holder::Ty>> {
	Ok(sound_up(&v.sound))
}

pub fn late_consume_sound_down_26_3(
	v: &b::slot_component_consumable::Ty,
	_cx: &mut Session,
) -> Result<a::item_sound_holder::Ty> {
	Ok(sound_down(&v.sound))
}

pub fn late_equip_sound_up_26_4(
	v: &a::slot_component_equippable::Ty,
	_cx: &mut Session,
) -> Result<Option<b::item_sound_holder::Ty>> {
	Ok(sound_up(&v.sound))
}

pub fn late_equip_sound_down_26_3(
	v: &b::slot_component_equippable::Ty,
	_cx: &mut Session,
) -> Result<a::item_sound_holder::Ty> {
	Ok(sound_down(&v.sound))
}

pub fn late_shearing_sound_up_26_4(
	v: &a::slot_component_equippable::Ty,
	_cx: &mut Session,
) -> Result<Option<b::item_sound_holder::Ty>> {
	Ok(sound_up(&v.shearing_sound))
}

pub fn late_shearing_sound_down_26_3(
	v: &b::slot_component_equippable::Ty,
	_cx: &mut Session,
) -> Result<a::item_sound_holder::Ty> {
	Ok(sound_down(&v.shearing_sound))
}

pub fn late_meta_entity_up_26_4(
	p: &a::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, OLD, p.entity_id);
	Ok(p.entity_id)
}

pub fn late_meta_entity_down_26_3(
	p: &b::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, NEW, p.entity_id);
	Ok(p.entity_id)
}

fn meta_key(cx: &mut Session, from: Version, to: Version, key: u8) -> Result<u8> {
	if let Some(mapped) = shared::fallback_key(cx, from, to, key) {
		return mapped;
	}
	meta(cx).key = key;
	let Some(name) = meta(cx).entity.clone() else {
		return Ok(key);
	};
	let keys = |v: Version| {
		crate::registry::names(v, "entities")
			.and_then(|n| n.id(&name))
			.and_then(|id| crate::registry::entity_metadata_keys(v, id))
	};
	let (Some(source), Some(target)) = (keys(from), keys(to)) else {
		return Ok(key);
	};
	let Some(field) = source.get(key as usize) else {
		return Ok(key);
	};
	target
		.iter()
		.position(|k| k == field)
		.map(|i| i as u8)
		.ok_or(Error::Dropped)
}

pub fn late_meta_key_up_26_4(e: &a::entity_metadata_entry::Ty, cx: &mut Session) -> Result<u8> {
	meta_key(cx, OLD, NEW, e.key)
}

pub fn late_meta_key_down_26_3(e: &b::entity_metadata_entry::Ty, cx: &mut Session) -> Result<u8> {
	meta_key(cx, NEW, OLD, e.key)
}

pub fn late_login_entity_up_26_4(
	p: &a::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<i32> {
	world::world(cx).player = Some(p.entity_id);
	Ok(p.entity_id)
}

pub fn late_login_entity_down_26_3(
	p: &b::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<i32> {
	world::world(cx).player = Some(p.entity_id);
	Ok(p.entity_id)
}

const JUKEBOX_MENU: i32 = 8;

pub fn late_menu_up_26_4(
	v: &a::play_to_client_packet_open_window::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(if v.inventory_type >= JUKEBOX_MENU {
		v.inventory_type + 1
	} else {
		v.inventory_type
	})
}

pub fn late_menu_down_26_3(
	v: &b::play_to_client_packet_open_window::Ty,
	cx: &mut Session,
) -> Result<i32> {
	match v.inventory_type {
		JUKEBOX_MENU => {
			let close = b::play_to_server_packet::Ty::CloseWindow(v.window_id);
			cx.send(
				Root::PlayToServer,
				NEW,
				encode(&close, b::play_to_server_packet::serialize)?,
			);
			Err(Error::Dropped)
		}
		t if t > JUKEBOX_MENU => Ok(t - 1),
		t => Ok(t),
	}
}

pub fn late_transfer_properties_up_26_4(
	_v: &a::packet_common_transfer::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::packet_common_transfer_properties_items::Ty>> {
	Ok(Vec::new())
}

#[derive(Default)]
struct Login {
	threshold: Option<i32>,
}

pub fn late_compress_up_26_4(
	threshold: i32,
	cx: &mut Session,
) -> Result<Vec<b::login_to_client_packet::Ty>> {
	cx.state::<Login>().threshold = Some(threshold);
	Ok(vec![b::login_to_client_packet::Ty::SetupCompression(
		threshold,
	)])
}

pub fn late_success_options_up_26_4(
	_v: &a::login_to_client_packet_success::Ty,
	cx: &mut Session,
) -> Result<b::login_to_client_connection_options::Ty> {
	let threshold = cx.state::<Login>().threshold.take();
	Ok(b::login_to_client_connection_options::Ty {
		encryption: false,
		compression: threshold.is_some_and(|t| t >= 0),
	})
}

pub fn late_setup_compression_down_26_3(
	threshold: i32,
	cx: &mut Session,
) -> Result<Vec<a::login_to_client_packet::Ty>> {
	cx.state::<Login>().threshold = Some(threshold);
	Ok(Vec::new())
}

pub fn late_success_down_26_3(
	v: b::login_to_client_packet_success::Ty,
	cx: &mut Session,
) -> Result<Vec<a::login_to_client_packet::Ty>> {
	let threshold = cx.state::<Login>().threshold.take();
	let mut out = vec![a::login_to_client_packet::Ty::Success(
		a::login_to_client_packet_success::Ty {
			uuid: v.uuid,
			username: v.username,
			properties: v
				.properties
				.into_iter()
				.map(|p| a::login_to_client_packet_success_properties_items::Ty {
					name: p.name,
					value: p.value,
					signature: p.signature,
				})
				.collect(),
			session_id: v.session_id,
		},
	)];
	if v.options.compression {
		out.push(a::login_to_client_packet::Ty::Compress(
			threshold.unwrap_or(-1),
		));
	}
	Ok(out)
}

const BRAND: &str = "minecraft:brand";

#[derive(Default)]
struct Brand {
	sent: bool,
}

pub fn late_config_payload_up_26_4(
	v: a::configuration_to_server_packet_custom_payload::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_server_packet::Ty>> {
	if v.channel == BRAND && std::mem::replace(&mut cx.state::<Brand>().sent, true) {
		return Ok(Vec::new());
	}
	Ok(vec![b::configuration_to_server_packet::Ty::CustomPayload(
		v,
	)])
}

pub fn late_play_payload_up_26_4(
	v: a::play_to_server_packet_custom_payload::Ty,
	cx: &mut Session,
) -> Result<Vec<b::play_to_server_packet::Ty>> {
	if v.channel == BRAND && std::mem::replace(&mut cx.state::<Brand>().sent, true) {
		return Ok(Vec::new());
	}
	Ok(vec![b::play_to_server_packet::Ty::CustomPayload(v)])
}

#[derive(Default)]
struct Knockback {
	pending: Option<(f64, f64, f64)>,
}

pub fn late_entity_motion_down_26_3(
	v: b::play_to_client_packet_add_entity_motion::Ty,
	cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	if world::world(cx).player == Some(v.entity_id) {
		cx.state::<Knockback>().pending = Some(read_lp_vec3(&v.impulse)?);
	}
	Ok(Vec::new())
}

fn particle_vessel_up(p: &a::particle::Ty, cx: &mut Session) -> Result<b::particle::Ty> {
	let packet = a::play_to_client_packet::Ty::WorldParticles(
		a::play_to_client_packet_world_particles::Ty {
			particle: p.clone(),
			long_distance: false,
			always_show: false,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			offset_x: 0.0,
			offset_y: 0.0,
			offset_z: 0.0,
			x_max_speed: 0.0,
			y_max_speed: 0.0,
			z_max_speed: 0.0,
			amount: 0,
			randomization_type:
				a::play_to_client_packet_world_particles_randomization_type::Ty::default,
		},
	);
	match shared::via_step(
		OLD,
		NEW,
		Root::PlayToClient,
		&packet,
		a::play_to_client_packet::serialize,
		b::play_to_client_packet::deserialize,
		cx,
	)? {
		b::play_to_client_packet::Ty::WorldParticles(w) => Ok(w.particle),
		_ => Err(Error::Dropped),
	}
}

fn particle_vessel_down(p: &b::particle::Ty, cx: &mut Session) -> Result<a::particle::Ty> {
	let packet = b::play_to_client_packet::Ty::WorldParticles(
		b::play_to_client_packet_world_particles::Ty {
			particle: p.clone(),
			long_distance: false,
			always_show: false,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			offset_x: 0.0,
			offset_y: 0.0,
			offset_z: 0.0,
			x_max_speed: 0.0,
			y_max_speed: 0.0,
			z_max_speed: 0.0,
			amount: 0,
			randomization_type:
				b::play_to_client_packet_world_particles_randomization_type::Ty::default,
		},
	);
	match shared::via_step(
		NEW,
		OLD,
		Root::PlayToClient,
		&packet,
		b::play_to_client_packet::serialize,
		a::play_to_client_packet::deserialize,
		cx,
	)? {
		a::play_to_client_packet::Ty::WorldParticles(w) => Ok(w.particle),
		_ => Err(Error::Dropped),
	}
}

pub fn late_explosion_up_26_4(
	v: a::play_to_client_packet_explosion::Ty,
	cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	let mut out = Vec::new();
	if let (Some(k), Some(entity_id)) = (&v.player_knockback, world::world(cx).player) {
		out.push(b::play_to_client_packet::Ty::AddEntityMotion(
			b::play_to_client_packet_add_entity_motion::Ty {
				entity_id,
				impulse: lp_vec3(k.x, k.y, k.z),
			},
		));
	}
	let mut block_particles = Vec::new();
	for e in &v.block_particles {
		let Ok(particle) = particle_vessel_up(&e.data.particle, cx) else {
			continue;
		};
		block_particles.push(b::play_to_client_explosion_particle_entry::Ty {
			data: b::play_to_client_explosion_particle_info::Ty {
				particle,
				scaling: e.data.scaling,
				speed: e.data.speed,
			},
			weight: e.weight,
		});
	}
	out.push(b::play_to_client_packet::Ty::Explosion(
		b::play_to_client_packet_explosion::Ty {
			center: v.center,
			radius: v.radius,
			block_count: v.block_count,
			explosion_particle: particle_vessel_up(&v.explosion_particle, cx).ok(),
			sound: if v.play_sound {
				sound_up(&v.sound)
			} else {
				None
			},
			block_particles,
		},
	));
	Ok(out)
}

pub fn late_explosion_down_26_3(
	v: b::play_to_client_packet_explosion::Ty,
	cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	let knockback = cx.state::<Knockback>().pending.take();
	let explosion_particle = match &v.explosion_particle {
		Some(p) => particle_vessel_down(p, cx).unwrap_or(a::particle::Ty::Explosion),
		None => a::particle::Ty::Explosion,
	};
	let mut block_particles = Vec::new();
	for e in &v.block_particles {
		let Ok(particle) = particle_vessel_down(&e.data.particle, cx) else {
			continue;
		};
		block_particles.push(a::play_to_client_explosion_particle_entry::Ty {
			data: a::play_to_client_explosion_particle_info::Ty {
				particle,
				scaling: e.data.scaling,
				speed: e.data.speed,
			},
			weight: e.weight,
		});
	}
	Ok(vec![a::play_to_client_packet::Ty::Explosion(
		a::play_to_client_packet_explosion::Ty {
			center: v.center,
			radius: v.radius,
			block_count: v.block_count,
			player_knockback: knockback.map(|(x, y, z)| a::vec3f64::Ty { x, y, z }),
			explosion_particle,
			play_sound: v.sound.is_some(),
			sound: sound_down(&v.sound),
			block_particles,
		},
	)])
}

fn delta_up(d: pds_entityDelta::Ty) -> pds_vecDelta::Ty {
	pds_vecDelta::Ty {
		onGround: d.onGround,
		delta: match d.delta {
			pds_entityDelta::Delta::Linear { dX, dY, dZ } => {
				pds_vecDelta::Delta::Linear { dX, dY, dZ }
			}
			pds_entityDelta::Delta::Stepped(steps) => pds_vecDelta::Delta::Stepped {
				startImmediately: false,
				steps,
			},
		},
	}
}

fn delta_down(d: pds_vecDelta::Ty) -> pds_entityDelta::Ty {
	pds_entityDelta::Ty {
		onGround: d.onGround,
		delta: match d.delta {
			pds_vecDelta::Delta::Linear { dX, dY, dZ } => {
				pds_entityDelta::Delta::Linear { dX, dY, dZ }
			}
			pds_vecDelta::Delta::Stepped { steps, .. } => pds_entityDelta::Delta::Stepped(steps),
		},
	}
}

pub fn late_rel_move_up_26_4(
	v: a::play_to_client_packet_rel_entity_move::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	Ok(vec![b::play_to_client_packet::Ty::RelEntityMove(
		b::play_to_client_packet_rel_entity_move::Ty {
			entity_id: v.entity_id,
			r#move: delta_up(v.r#move),
		},
	)])
}

pub fn late_rel_move_down_26_3(
	v: b::play_to_client_packet_rel_entity_move::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	Ok(vec![a::play_to_client_packet::Ty::RelEntityMove(
		a::play_to_client_packet_rel_entity_move::Ty {
			entity_id: v.entity_id,
			r#move: delta_down(v.r#move),
		},
	)])
}

pub fn late_move_look_up_26_4(
	v: a::play_to_client_packet_entity_move_look::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	Ok(vec![b::play_to_client_packet::Ty::EntityMoveLook(
		b::play_to_client_packet_entity_move_look::Ty {
			entity_id: v.entity_id,
			r#move: delta_up(v.r#move),
			yaw: v.yaw,
			pitch: v.pitch,
		},
	)])
}

pub fn late_move_look_down_26_3(
	v: b::play_to_client_packet_entity_move_look::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	Ok(vec![a::play_to_client_packet::Ty::EntityMoveLook(
		a::play_to_client_packet_entity_move_look::Ty {
			entity_id: v.entity_id,
			r#move: delta_down(v.r#move),
			yaw: v.yaw,
			pitch: v.pitch,
		},
	)])
}

#[derive(Default)]
struct Heads {
	yaw: HashMap<i32, i8>,
}

fn unpack_degrees(v: i8) -> f32 {
	v as f32 * 360.0 / 256.0
}

fn pack_degrees(v: f32) -> i8 {
	(v * 256.0 / 360.0).floor() as i32 as i8
}

pub fn late_spawn_head_up_26_4(
	v: &a::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<i8> {
	cx.state::<Heads>().yaw.insert(v.entity_id, v.head_pitch);
	Ok(v.head_pitch)
}

pub fn late_head_yaw_up_26_4(
	v: &a::play_to_client_packet_entity_head_rotation::Ty,
	cx: &mut Session,
) -> Result<i8> {
	cx.state::<Heads>().yaw.insert(v.entity_id, v.head_yaw);
	Ok(v.head_yaw)
}

pub fn late_sync_position_up_26_4(
	v: a::play_to_client_packet_sync_entity_position::Ty,
	cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	use a::play_to_client_packet_sync_entity_position_anon::Ty as From;
	use b::play_to_client_packet_sync_entity_position_anon::Ty as To;
	let anon = match v.anon {
		From::Linear(p) => To::Linear(
			b::play_to_client_packet_sync_entity_position_anon_linear::Ty {
				x: p.x,
				y: p.y,
				z: p.z,
			},
		),
		From::Stepped(steps) => To::Stepped(
			b::play_to_client_packet_sync_entity_position_anon_stepped::Ty {
				steps: steps
					.into_iter()
					.map(|s| {
						b::play_to_client_packet_sync_entity_position_anon_stepped_steps_items::Ty {
							x: s.x,
							y: s.y,
							z: s.z,
							tick_offset: s.tick_offset,
						}
					})
					.collect(),
				start_immediately: false,
			},
		),
		From::Default(t) => To::Default(t),
	};
	let head_yaw = cx
		.state::<Heads>()
		.yaw
		.get(&v.entity_id)
		.map_or(v.yaw, |h| unpack_degrees(*h));
	Ok(vec![b::play_to_client_packet::Ty::SyncEntityPosition(
		b::play_to_client_packet_sync_entity_position::Ty {
			entity_id: v.entity_id,
			anon,
			yaw: v.yaw,
			pitch: v.pitch,
			head_yaw,
			on_ground: v.on_ground,
		},
	)])
}

pub fn late_sync_position_down_26_3(
	v: b::play_to_client_packet_sync_entity_position::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	use a::play_to_client_packet_sync_entity_position_anon::Ty as To;
	use b::play_to_client_packet_sync_entity_position_anon::Ty as From;
	let anon = match v.anon {
		From::Linear(p) => To::Linear(
			a::play_to_client_packet_sync_entity_position_anon_linear::Ty {
				x: p.x,
				y: p.y,
				z: p.z,
			},
		),
		From::Stepped(s) => To::Stepped(
			s.steps
				.into_iter()
				.map(
					|s| a::play_to_client_packet_sync_entity_position_anon_stepped_items::Ty {
						x: s.x,
						y: s.y,
						z: s.z,
						tick_offset: s.tick_offset,
					},
				)
				.collect(),
		),
		From::Default(t) => To::Default(t),
	};
	Ok(vec![
		a::play_to_client_packet::Ty::SyncEntityPosition(
			a::play_to_client_packet_sync_entity_position::Ty {
				entity_id: v.entity_id,
				anon,
				yaw: v.yaw,
				pitch: v.pitch,
				on_ground: v.on_ground,
			},
		),
		a::play_to_client_packet::Ty::EntityHeadRotation(
			a::play_to_client_packet_entity_head_rotation::Ty {
				entity_id: v.entity_id,
				head_yaw: pack_degrees(v.head_yaw),
			},
		),
	])
}

const WATER: [i32; 2] = [2, 1];
const LAVA: [i32; 2] = [4, 3];

fn fluid(registry: &str) -> bool {
	registry.strip_prefix("minecraft:").unwrap_or(registry) == "fluid"
}

fn fluids_up(tags: Vec<tag::Ty>) -> Vec<tag::Ty> {
	tags.into_iter()
		.map(|t| {
			let mut entries = Vec::new();
			for id in t.entries {
				let id = match id {
					1 | 2 => 1,
					3 | 4 => 2,
					other => other,
				};
				if !entries.contains(&id) {
					entries.push(id);
				}
			}
			tag::Ty {
				tag_name: t.tag_name,
				entries,
			}
		})
		.collect()
}

fn fluids_down(tags: Vec<tag::Ty>) -> Vec<tag::Ty> {
	tags.into_iter()
		.map(|t| tag::Ty {
			tag_name: t.tag_name,
			entries: t
				.entries
				.into_iter()
				.flat_map(|id| match id {
					1 => WATER.to_vec(),
					2 => LAVA.to_vec(),
					other => vec![other],
				})
				.collect(),
		})
		.collect()
}

const TAG_RENAMES_UP: &[(&str, &str, &str)] = &[(
	"entity_type",
	"freeze_immune_entity_types",
	"immune_to_freezing",
)];
const TAG_RENAMES_DOWN: &[(&str, &str, &str)] = &[(
	"entity_type",
	"immune_to_freezing",
	"freeze_immune_entity_types",
)];

const JUKEBOX_CONTAINERS: &[&str] = &[
	"chest",
	"trapped_chest",
	"copper_chest",
	"exposed_copper_chest",
	"weathered_copper_chest",
	"oxidized_copper_chest",
	"waxed_copper_chest",
	"waxed_exposed_copper_chest",
	"waxed_weathered_copper_chest",
	"waxed_oxidized_copper_chest",
];

const DERIVED_UP: &[Derived] = &[
	(
		"entity_type",
		"immune_to_poison",
		&[
			"#minecraft:ignores_poison_and_regen",
			"nautilus",
			"zombie_nautilus",
			"spider",
			"cave_spider",
		],
	),
	(
		"entity_type",
		"immune_to_regeneration",
		&["#minecraft:ignores_poison_and_regen"],
	),
	(
		"entity_type",
		"immune_to_wither",
		&["wither", "wither_skeleton"],
	),
	("entity_type", "immune_to_weakness", &["parched"]),
	(
		"entity_type",
		"does_not_break_brittle_ice",
		&["frostbite", "stray"],
	),
	("fluid", "swimmable", &["#minecraft:water"]),
	("item", "sulfur_cube_archetype/cold", &["blue_ice"]),
	("item", "sheep_wool_dyes", &["#minecraft:dyes"]),
	(
		"item",
		"knocks_back_players_even_with_zero_damage",
		&["snowball"],
	),
	("item", "jukebox_containers", JUKEBOX_CONTAINERS),
	("block", "jukebox_containers", JUKEBOX_CONTAINERS),
	(
		"block",
		"face_occluding_ice_blocks",
		&["ice", "brittle_ice", "frosted_ice"],
	),
	(
		"block",
		"triggers_ambient_ice_crystal_sounds",
		&["packed_ice"],
	),
	("block", "triggers_ambient_ice_block_sounds", &["ice"]),
	("block", "melts_icicle_above", &["magma_block"]),
];

const DERIVED_DOWN: &[Derived] = &[(
	"entity_type",
	"ignores_poison_and_regen",
	&["#minecraft:immune_to_regeneration"],
)];

fn derive_play(
	to: Version,
	v: Vec<a::play_to_client_packet_tags_items::Ty>,
	derived: &[Derived],
) -> Vec<a::play_to_client_packet_tags_items::Ty> {
	let mut config = v
		.into_iter()
		.map(|r| a::configuration_to_client_packet_tags_items::Ty {
			tag_type: r.tag_type,
			tags: r.tags,
		})
		.collect();
	super::tags::derive(to, &mut config, derived);
	config
		.into_iter()
		.map(|r| a::play_to_client_packet_tags_items::Ty {
			tag_type: r.tag_type,
			tags: r.tags,
		})
		.collect()
}

pub fn late_play_tags_up_26_4(
	v: Vec<a::play_to_client_packet_tags_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	let v = super::tags::play(OLD, NEW, TAG_RENAMES_UP, v)
		.into_iter()
		.map(|mut r| {
			if fluid(&r.tag_type) {
				r.tags = fluids_up(r.tags);
			}
			r
		})
		.collect();
	Ok(vec![b::play_to_client_packet::Ty::Tags(derive_play(
		NEW, v, DERIVED_UP,
	))])
}

pub fn late_play_tags_down_26_3(
	v: Vec<b::play_to_client_packet_tags_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	let v = super::tags::play(NEW, OLD, TAG_RENAMES_DOWN, v)
		.into_iter()
		.map(|mut r| {
			if fluid(&r.tag_type) {
				r.tags = fluids_down(r.tags);
			}
			r
		})
		.collect();
	Ok(vec![a::play_to_client_packet::Ty::Tags(derive_play(
		OLD,
		v,
		DERIVED_DOWN,
	))])
}

const REQUIRED: &[(&str, &[&str])] = &[
	("jukebox_song", &["flurry"]),
	("block_transformer", &["honeycomb"]),
	("sulfur_cube_archetype", &["cold"]),
];
const ADDED: &[&str] = &["block_sound_set", "dispense_behavior"];
const VANILLA: &[&str] = &["block_transformer", "enchantment", "chicken_sound_variant"];

#[derive(Default)]
struct Config {
	registries: HashSet<String>,
	tags: bool,
}

fn short(s: &str) -> &str {
	s.strip_prefix("minecraft:").unwrap_or(s)
}

fn full(key: &str) -> String {
	if key.contains(':') {
		key.to_owned()
	} else {
		format!("minecraft:{key}")
	}
}

fn substitute(version: Version, id: &str, entries: &mut [(String, Option<Tag>)]) -> Result<()> {
	if !VANILLA.contains(&id) {
		return Ok(());
	}
	let vanilla = registries::vanilla(version, &format!("minecraft:{id}"))?;
	for (key, value) in entries.iter_mut() {
		if value.is_none() {
			continue;
		}
		let key = full(key);
		if let Some((_, data)) = vanilla.iter().find(|(k, _)| *k == key) {
			*value = data.clone();
		}
	}
	Ok(())
}

fn add_required(id: &str, entries: &mut Vec<(String, Option<Tag>)>) -> Result<()> {
	let Some((_, keys)) = REQUIRED.iter().find(|(r, _)| *r == id) else {
		return Ok(());
	};
	let vanilla = registries::vanilla(NEW, &format!("minecraft:{id}"))?;
	for key in *keys {
		let wanted = full(key);
		if entries.iter().any(|(k, _)| full(k) == wanted) {
			continue;
		}
		if let Some(entry) = vanilla.iter().find(|(k, _)| *k == wanted) {
			entries.push(entry.clone());
		}
	}
	Ok(())
}

pub fn late_registry_data_up_26_4(
	p: a::configuration_to_client_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_client_packet::Ty>> {
	shared::count_biomes(&p, cx);
	let id = short(&p.id).to_owned();
	cx.state::<Config>().registries.insert(id.clone());
	let mut entries: Vec<(String, Option<Tag>)> =
		p.entries.into_iter().map(|e| (e.key, e.value)).collect();
	substitute(NEW, &id, &mut entries)?;
	add_required(&id, &mut entries)?;
	Ok(vec![b::configuration_to_client_packet::Ty::RegistryData(
		shared::registry(&id, entries),
	)])
}

const NEW_ATTRIBUTES: &[&str] = &["minecraft:visual/has_sky_occluder"];
const STAND_INS: &[(&str, &str, &str)] = &[("jukebox_song", "flurry", "13")];

fn strip_attributes(tag: &mut Payload) {
	match tag {
		Payload::Compound(entries) => {
			entries.retain(|e| !NEW_ATTRIBUTES.contains(&e.name.as_str()));
			for e in entries {
				strip_attributes(&mut e.value);
			}
		}
		Payload::List(_) => {
			for c in tag.compounds_mut().into_iter().flatten() {
				let mut item = Payload::Compound(std::mem::take(c));
				strip_attributes(&mut item);
				if let Payload::Compound(back) = item {
					*c = back;
				}
			}
		}
		_ => {}
	}
}

fn stand_ins(id: &str, entries: &mut [(String, Option<Tag>)]) -> Result<()> {
	for (registry, key, stand_in) in STAND_INS {
		if *registry != id {
			continue;
		}
		let Some((_, value)) = entries.iter_mut().find(|(k, _)| full(k) == full(key)) else {
			continue;
		};
		let vanilla = registries::vanilla(OLD, &format!("minecraft:{id}"))?;
		if let Some((_, data)) = vanilla.into_iter().find(|(k, _)| *k == full(stand_in)) {
			*value = data;
		}
	}
	Ok(())
}

pub fn late_registry_data_down_26_3(
	p: b::configuration_to_client_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<a::configuration_to_client_packet::Ty>> {
	shared::count_biomes(&p, cx);
	let id = short(&p.id).to_owned();
	if ADDED.contains(&id.as_str()) {
		return Ok(Vec::new());
	}
	let mut entries: Vec<(String, Option<Tag>)> =
		p.entries.into_iter().map(|e| (e.key, e.value)).collect();
	for (_, value) in entries.iter_mut() {
		if let Some(tag) = value {
			strip_attributes(&mut tag.value_mut());
		}
	}
	substitute(OLD, &id, &mut entries)?;
	stand_ins(&id, &mut entries)?;
	Ok(vec![a::configuration_to_client_packet::Ty::RegistryData(
		shared::registry(&id, entries),
	)])
}

pub fn late_config_tags_up_26_4(
	v: Vec<a::configuration_to_client_packet_tags_items::Ty>,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_client_packet::Ty>> {
	cx.state::<Config>().tags = true;
	let mut v: Vec<_> = super::tags::config(OLD, NEW, TAG_RENAMES_UP, v)
		.into_iter()
		.map(|mut r| {
			if fluid(&r.tag_type) {
				r.tags = fluids_up(r.tags);
			}
			r
		})
		.collect();
	super::tags::derive(NEW, &mut v, DERIVED_UP);
	Ok(vec![b::configuration_to_client_packet::Ty::Tags(v)])
}

pub fn late_config_tags_down_26_3(
	v: Vec<b::configuration_to_client_packet_tags_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<a::configuration_to_client_packet::Ty>> {
	let mut v: Vec<_> = super::tags::config(NEW, OLD, TAG_RENAMES_DOWN, v)
		.into_iter()
		.map(|mut r| {
			if fluid(&r.tag_type) {
				r.tags = fluids_down(r.tags);
			}
			r
		})
		.collect();
	super::tags::derive(OLD, &mut v, DERIVED_DOWN);
	Ok(vec![a::configuration_to_client_packet::Ty::Tags(v)])
}

pub fn late_finish_configuration_up_26_4(
	p: a::configuration_to_client_packet_finish_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_client_packet::Ty>> {
	let config = std::mem::take(cx.state::<Config>());
	let mut out = Vec::new();
	for id in ADDED.iter().chain(REQUIRED.iter().map(|(r, _)| r)) {
		if config.registries.contains(*id) {
			continue;
		}
		let entries = registries::vanilla(NEW, &format!("minecraft:{id}"))?;
		out.push(b::configuration_to_client_packet::Ty::RegistryData(
			shared::registry(id, entries),
		));
	}
	if !config.tags {
		let mut tags = Vec::new();
		super::tags::derive(NEW, &mut tags, DERIVED_UP);
		out.push(b::configuration_to_client_packet::Ty::Tags(tags));
	}
	out.push(b::configuration_to_client_packet::Ty::FinishConfiguration(
		p,
	));
	Ok(out)
}
