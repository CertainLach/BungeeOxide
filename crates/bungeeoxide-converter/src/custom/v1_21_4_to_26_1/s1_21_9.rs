use super::remap::{self, remaps};
use super::rules::{blocks, entities, items, statistics, tags};
use super::shared::{self, meta, text_down};
use super::slots::{self, exact_matchers, untrusted_step};
use super::text;
use super::util::{lp_vec3, read_lp_vec3};
use super::world;
use crate::Session;
use crate::generated::convert::{Version, v1_21_8 as a, v1_21_9 as b};
use crate::pds::convert::{Error, Result};
use bungeeoxide_nbt::{Entry, Payload, Tag};
use std::collections::HashMap;

remaps!(Version::V1_21_8 => Version::V1_21_9 { late_sound_up_1_21_9: a::item_sound_holder::Ty = remap::sound; });
remaps!(Version::V1_21_9 => Version::V1_21_8 { late_sound_down_1_21_8: a::item_sound_holder::Ty = remap::sound; });

items!(a => b, Version::V1_21_8 => Version::V1_21_9 {
	slot: late_slot_item_up_1_21_9, hashed: late_hashed_item_up_1_21_9, trade1: late_trade1_up_1_21_9, trade2: late_trade2_up_1_21_9,
	display: late_display_item_up_1_21_9, recipes: late_recipe_items_up_1_21_9, stonecutter: late_stonecutter_up_1_21_9,
	requirements: late_requirements_up_1_21_9, repairable: late_repairable_up_1_21_9, pots: late_pots_up_1_21_9,
});
items!(b => a, Version::V1_21_9 => Version::V1_21_8 {
	slot: late_slot_item_down_1_21_8, hashed: late_hashed_item_down_1_21_8, trade1: late_trade1_down_1_21_8, trade2: late_trade2_down_1_21_8,
	display: late_display_item_down_1_21_8, recipes: late_recipe_items_down_1_21_8, stonecutter: late_stonecutter_down_1_21_8,
	requirements: late_requirements_down_1_21_8, repairable: late_repairable_down_1_21_8, pots: late_pots_down_1_21_8,
});

blocks!(a => b, Version::V1_21_8 => Version::V1_21_9 {
	block_change: late_block_change_up_1_21_9, multi_block: late_multi_block_up_1_21_9, block_action: late_block_action_up_1_21_9,
	world_event: late_world_event_up_1_21_9, chunk: late_chunk_up_1_21_9, tool: late_tool_up_1_21_9, predicate: late_predicate_up_1_21_9,
	particle: [late_particle_block_up_1_21_9, late_particle_marker_up_1_21_9, late_particle_dust_up_1_21_9, late_particle_pillar_up_1_21_9, late_particle_crumble_up_1_21_9],
	meta: [late_meta_state_up_1_21_9, late_meta_optional_state_up_1_21_9],
});
blocks!(b => a, Version::V1_21_9 => Version::V1_21_8 {
	block_change: late_block_change_down_1_21_8, multi_block: late_multi_block_down_1_21_8, block_action: late_block_action_down_1_21_8,
	world_event: late_world_event_down_1_21_8, chunk: late_chunk_down_1_21_8, tool: late_tool_down_1_21_8, predicate: late_predicate_down_1_21_8,
	particle: [late_particle_block_down_1_21_8, late_particle_marker_down_1_21_8, late_particle_dust_down_1_21_8, late_particle_pillar_down_1_21_8, late_particle_crumble_down_1_21_8],
	meta: [late_meta_state_down_1_21_8, late_meta_optional_state_down_1_21_8],
});

entities!(a => b, Version::V1_21_8 => Version::V1_21_9 { spawn: late_spawn_type_up_1_21_9, object: late_object_data_up_1_21_9, equippable: late_equip_entities_up_1_21_9, });
entities!(b => a, Version::V1_21_9 => Version::V1_21_8 { spawn: late_spawn_type_down_1_21_8, object: late_object_data_down_1_21_8, equippable: late_equip_entities_down_1_21_8, });

statistics!(a => b, Version::V1_21_8 => Version::V1_21_9 { late_statistics_up_1_21_9 });
statistics!(b => a, Version::V1_21_9 => Version::V1_21_8 { late_statistics_down_1_21_8 });

tags!(a => b, Version::V1_21_8 => Version::V1_21_9, &[], { config: late_config_tags_up_1_21_9, play: late_play_tags_up_1_21_9 });
tags!(b => a, Version::V1_21_9 => Version::V1_21_8, &[], { config: late_config_tags_down_1_21_8, play: late_play_tags_down_1_21_8 });

untrusted_step!(late_untrusted_up_1_21_9: a => b, Version::V1_21_8 => Version::V1_21_9, slots::late_to_slot_1_21_8, slots::late_to_untrusted_1_21_9);
untrusted_step!(late_untrusted_down_1_21_8: b => a, Version::V1_21_9 => Version::V1_21_8, slots::late_to_slot_1_21_9, slots::late_to_untrusted_1_21_8);

exact_matchers!(exact_up: a => b, Version::V1_21_8 => Version::V1_21_9);
exact_matchers!(exact_down: b => a, Version::V1_21_9 => Version::V1_21_8);

pub fn late_matchers_up_1_21_9(
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
				r#type: p.r#type,
				predicate: p.predicate.clone(),
			})
			.collect(),
	})
}

pub fn late_matchers_down_1_21_8(
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
				r#type: p.r#type,
				predicate: p.predicate.clone(),
			})
			.collect(),
	})
}

const BLOCK_ENTITIES: &[&str] = &[
	"furnace",
	"chest",
	"trapped_chest",
	"ender_chest",
	"jukebox",
	"dispenser",
	"dropper",
	"sign",
	"hanging_sign",
	"mob_spawner",
	"creaking_heart",
	"piston",
	"brewing_stand",
	"enchanting_table",
	"end_portal",
	"beacon",
	"skull",
	"daylight_detector",
	"hopper",
	"comparator",
	"banner",
	"structure_block",
	"end_gateway",
	"command_block",
	"shulker_box",
	"bed",
	"conduit",
	"barrel",
	"smoker",
	"blast_furnace",
	"lectern",
	"bell",
	"jigsaw",
	"campfire",
	"beehive",
	"sculk_sensor",
	"calibrated_sculk_sensor",
	"sculk_catalyst",
	"sculk_shrieker",
	"chiseled_bookshelf",
	"shelf",
	"brushable_block",
	"decorated_pot",
	"crafter",
	"trial_spawner",
	"vault",
	"test_block",
	"test_instance_block",
	"copper_golem_statue",
];

fn block_entity_up(v: i32) -> i32 {
	if v >= 40 { v + 1 } else { v }
}

fn block_entity_down(v: i32) -> Result<i32> {
	match v {
		40 | 48.. => Err(Error::Dropped),
		v if v > 40 => Ok(v - 1),
		v => Ok(v),
	}
}

pub fn late_chunk_block_entity_up_1_21_9(
	v: &a::chunk_block_entity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(block_entity_up(v.r#type))
}

pub fn late_chunk_block_entity_down_1_21_8(
	v: &b::chunk_block_entity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block_entity_down(v.r#type)
}

pub fn late_tile_entity_up_1_21_9(
	v: &a::play_to_client_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(block_entity_up(v.action))
}

pub fn late_tile_entity_down_1_21_8(
	v: &b::play_to_client_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block_entity_down(v.action)
}

fn tag_id(tag: &bungeeoxide_nbt::Tag) -> Option<&str> {
	let id = tag.value.field("id").and_then(Payload::as_str)?;
	Some(id.strip_prefix("minecraft:").unwrap_or(id))
}

fn with_id(mut tag: bungeeoxide_nbt::Tag, name: &str) -> bungeeoxide_nbt::Tag {
	if !matches!(tag.value, Payload::Compound(_)) {
		tag = Tag::new(Payload::Compound(vec![]));
	}
	tag.value
		.put("id", Payload::String(format!("minecraft:{name}")));
	tag
}

pub fn late_entity_data_up_1_21_9(
	v: bungeeoxide_nbt::Tag,
	_cx: &mut Session,
) -> Result<b::slot_component::Ty> {
	let name = tag_id(&v).unwrap_or("pig");
	let r#type = crate::registry::names(Version::V1_21_9, "entities")
		.and_then(|n| n.id(name))
		.unwrap_or(0) as i32;
	Ok(b::slot_component::Ty::EntityData(
		b::slot_component_entity_data::Ty { r#type, data: v },
	))
}

pub fn late_entity_data_down_1_21_8(
	v: b::slot_component_entity_data::Ty,
	_cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	let name = crate::registry::names(Version::V1_21_9, "entities")
		.and_then(|n| n.name(v.r#type.max(0) as u32));
	Ok(a::slot_component::Ty::EntityData(match name {
		Some(name) => with_id(v.data, name),
		None => v.data,
	}))
}

pub fn late_block_entity_data_up_1_21_9(
	v: bungeeoxide_nbt::Tag,
	_cx: &mut Session,
) -> Result<b::slot_component::Ty> {
	let name = tag_id(&v).unwrap_or("furnace");
	let r#type = BLOCK_ENTITIES.iter().position(|n| *n == name).unwrap_or(0) as i32;
	Ok(b::slot_component::Ty::BlockEntityData(
		b::slot_component_block_entity_data::Ty { r#type, data: v },
	))
}

pub fn late_block_entity_data_down_1_21_8(
	v: b::slot_component_block_entity_data::Ty,
	_cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	Ok(a::slot_component::Ty::BlockEntityData(
		match BLOCK_ENTITIES.get(v.r#type.max(0) as usize) {
			Some(name) => with_id(v.data, name),
			None => v.data,
		},
	))
}

pub fn late_profile_up_1_21_9(
	v: a::slot_component_profile::Ty,
	_cx: &mut Session,
) -> Result<b::slot_component::Ty> {
	let properties = v
		.properties
		.into_iter()
		.map(|p| b::game_profile_property::Ty {
			name: p.name,
			value: p.value,
			signature: p.signature,
		})
		.collect();
	Ok(b::slot_component::Ty::Profile(b::resolvable_profile::Ty {
		anon: b::resolvable_profile_anon::Ty::Partial(b::partial_resolvable_profile::Ty {
			name: v.name,
			uuid: v.uuid,
			properties,
		}),
		skin_patch: b::player_skin_patch::Ty {
			body: None,
			cape: None,
			elytra: None,
			model: None,
		},
	}))
}

pub fn late_profile_down_1_21_8(
	v: b::resolvable_profile::Ty,
	_cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	let (name, uuid, properties) = match v.anon {
		b::resolvable_profile_anon::Ty::Partial(p) => (p.name, p.uuid, p.properties),
		b::resolvable_profile_anon::Ty::Complete(p) => (Some(p.name), Some(p.uuid), p.properties),
		b::resolvable_profile_anon::Ty::Default(_) => (None, None, Vec::new()),
	};
	let properties = properties
		.into_iter()
		.map(|p| a::slot_component_profile_properties_items::Ty {
			name: p.name,
			value: p.value,
			signature: p.signature,
		})
		.collect();
	Ok(a::slot_component::Ty::Profile(
		a::slot_component_profile::Ty {
			name,
			uuid,
			properties,
		},
	))
}

pub fn late_particle_effect_up_1_21_9(_v: (), _cx: &mut Session) -> Result<b::particle::Ty> {
	Ok(b::particle::Ty::Effect(b::particle_effect::Ty {
		color: -1,
		power: 1.0,
	}))
}

pub fn late_particle_instant_effect_up_1_21_9(
	_v: (),
	_cx: &mut Session,
) -> Result<b::particle::Ty> {
	Ok(b::particle::Ty::InstantEffect(
		b::particle_instant_effect::Ty {
			color: -1,
			power: 1.0,
		},
	))
}

pub fn late_particle_flash_up_1_21_9(_v: (), _cx: &mut Session) -> Result<b::particle::Ty> {
	Ok(b::particle::Ty::Flash(-1))
}

pub fn late_particle_dragon_breath_up_1_21_9(_v: (), _cx: &mut Session) -> Result<b::particle::Ty> {
	Ok(b::particle::Ty::DragonBreath(1.0))
}

pub fn late_particle_effect_down_1_21_8(
	_v: b::particle_effect::Ty,
	_cx: &mut Session,
) -> Result<a::particle::Ty> {
	Ok(a::particle::Ty::Effect)
}

pub fn late_particle_instant_effect_down_1_21_8(
	_v: b::particle_instant_effect::Ty,
	_cx: &mut Session,
) -> Result<a::particle::Ty> {
	Ok(a::particle::Ty::InstantEffect)
}

pub fn late_particle_flash_down_1_21_8(_v: i32, _cx: &mut Session) -> Result<a::particle::Ty> {
	Ok(a::particle::Ty::Flash)
}

pub fn late_particle_dragon_breath_down_1_21_8(
	_v: f32,
	_cx: &mut Session,
) -> Result<a::particle::Ty> {
	Ok(a::particle::Ty::DragonBreath)
}

fn velocity_up(v: &a::vec3i16::Ty) -> Vec<u8> {
	lp_vec3(
		v.x as f64 / 8000.0,
		v.y as f64 / 8000.0,
		v.z as f64 / 8000.0,
	)
}

fn velocity_down(v: &[u8]) -> Result<a::vec3i16::Ty> {
	let (x, y, z) = read_lp_vec3(v)?;
	let s = |v: f64| (v * 8000.0).round().clamp(i16::MIN as f64, i16::MAX as f64) as i16;
	Ok(a::vec3i16::Ty {
		x: s(x),
		y: s(y),
		z: s(z),
	})
}

pub fn late_spawn_velocity_up_1_21_9(
	v: &a::play_to_client_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	Ok(velocity_up(&v.velocity))
}

pub fn late_spawn_velocity_down_1_21_8(
	v: &b::play_to_client_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<a::vec3i16::Ty> {
	velocity_down(&v.velocity)
}

pub fn late_velocity_up_1_21_9(
	v: &a::play_to_client_packet_entity_velocity::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	Ok(velocity_up(&v.velocity))
}

pub fn late_velocity_down_1_21_8(
	v: &b::play_to_client_packet_entity_velocity::Ty,
	_cx: &mut Session,
) -> Result<a::vec3i16::Ty> {
	velocity_down(&v.velocity)
}

#[derive(Default)]
struct Dimensions {
	scales: Vec<f64>,
	current: usize,
}

pub fn late_registry_data_up_1_21_9(
	mut p: a::configuration_to_client_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_client_packet::Ty>> {
	shared::count_biomes(&p, cx);
	if p.id.strip_prefix("minecraft:").unwrap_or(&p.id) == "dimension_type" {
		cx.state::<Dimensions>().scales = scales(&p);
	}
	shared::registry_entries(&mut p, "dimension_type", |key, e| {
		if key.strip_prefix("minecraft:").unwrap_or(key) == "the_end" {
			e.put("ambient_light", Payload::Float(0.25));
		}
	});
	Ok(vec![b::configuration_to_client_packet::Ty::RegistryData(p)])
}

pub fn late_spawn_info_up_1_21_9(
	v: &a::play_to_client_spawn_info::Ty,
	cx: &mut Session,
) -> Result<String> {
	world::world(cx).dimension = v.name.clone();
	cx.state::<Dimensions>().current = v.dimension.max(0) as usize;
	Ok(v.name.clone())
}

fn scale(cx: &mut Session) -> f64 {
	let d = cx.state::<Dimensions>();
	d.scales
		.get(d.current)
		.copied()
		.filter(|s| *s > 0.0)
		.unwrap_or(1.0)
}

pub fn late_border_x_up_1_21_9(
	v: &a::play_to_client_packet_initialize_world_border::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.x / scale(cx))
}

pub fn late_border_z_up_1_21_9(
	v: &a::play_to_client_packet_initialize_world_border::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.z / scale(cx))
}

pub fn late_border_x_down_1_21_8(
	v: &b::play_to_client_packet_initialize_world_border::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.x * scale(cx))
}

pub fn late_border_z_down_1_21_8(
	v: &b::play_to_client_packet_initialize_world_border::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.z * scale(cx))
}

pub fn late_border_center_x_up_1_21_9(
	v: &a::play_to_client_packet_world_border_center::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.x / scale(cx))
}

pub fn late_border_center_z_up_1_21_9(
	v: &a::play_to_client_packet_world_border_center::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.z / scale(cx))
}

pub fn late_border_center_x_down_1_21_8(
	v: &b::play_to_client_packet_world_border_center::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.x * scale(cx))
}

pub fn late_border_center_z_down_1_21_8(
	v: &b::play_to_client_packet_world_border_center::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.z * scale(cx))
}

pub fn late_spawn_position_up_1_21_9(
	p: a::play_to_client_packet_spawn_position::Ty,
	cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	let dimension = world::world(cx).dimension.clone();
	let dimension_name = if dimension.is_empty() {
		"minecraft:overworld".to_owned()
	} else {
		dimension
	};
	Ok(vec![b::play_to_client_packet::Ty::SpawnPosition(
		b::respawn_data::Ty {
			global_pos: b::global_pos::Ty {
				dimension_name,
				location: p.location,
			},
			yaw: p.angle,
			pitch: 0.0,
		},
	)])
}

pub fn late_spawn_position_down_1_21_8(
	p: b::respawn_data::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	Ok(vec![a::play_to_client_packet::Ty::SpawnPosition(
		a::play_to_client_packet_spawn_position::Ty {
			location: p.global_pos.location,
			angle: p.yaw,
		},
	)])
}

pub fn late_meta_entity_up_1_21_9(
	p: &a::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, Version::V1_21_8, p.entity_id);
	Ok(p.entity_id)
}

pub fn late_meta_entity_down_1_21_8(
	p: &b::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, Version::V1_21_9, p.entity_id);
	mannequin_metadata(p, cx)?;
	Ok(p.entity_id)
}

pub fn late_meta_key_up_1_21_9(e: &a::entity_metadata_entry::Ty, cx: &mut Session) -> Result<u8> {
	meta(cx).key = e.key;
	Ok(match (shared::is(cx, &["player"]), e.key) {
		(true, 15) => 17,
		(true, 16) => 18,
		(true, 17) => 16,
		(true, 18) => 15,
		(_, k) => k,
	})
}

pub fn late_meta_key_down_1_21_8(e: &b::entity_metadata_entry::Ty, cx: &mut Session) -> Result<u8> {
	if let Some(key) = shared::fallback_key(cx, Version::V1_21_9, Version::V1_21_8, e.key) {
		return key;
	}
	meta(cx).key = e.key;
	Ok(match (shared::is(cx, &["player", "mannequin"]), e.key) {
		(true, 15) => 18,
		(true, 16) => 17,
		(true, 17) => 15,
		(true, 18) => 16,
		(_, k) => k,
	})
}

fn is_shoulder(cx: &mut Session) -> bool {
	shared::is(cx, &["player"]) && matches!(meta(cx).key, 19 | 20)
}

pub fn late_meta_compound_up_1_21_9(
	v: bungeeoxide_nbt::Tag,
	cx: &mut Session,
) -> Result<b::entity_metadata_entry_value::Ty> {
	if !is_shoulder(cx) {
		return Err(Error::Dropped);
	}
	let variant = v
		.value
		.field("Variant")
		.and_then(Payload::as_i64)
		.filter(|_| v.value.field("id").and_then(Payload::as_str).is_some());
	Ok(b::entity_metadata_entry_value::Ty::OptionalUnsignedInt(
		variant.map_or(0, |v| v as i32 + 1),
	))
}

pub fn late_meta_unsigned_down_1_21_8(
	v: i32,
	cx: &mut Session,
) -> Result<a::entity_metadata_entry_value::Ty> {
	if !is_shoulder(cx) {
		return Ok(a::entity_metadata_entry_value::Ty::OptionalUnsignedInt(v));
	}
	let tag = match v {
		0 => Payload::Compound(vec![]),
		v => Payload::Compound(vec![
			Entry::new("id", Payload::from("minecraft:parrot")),
			Entry::new("Variant", Payload::Int(v - 1)),
		]),
	};
	Ok(a::entity_metadata_entry_value::Ty::CompoundTag(Tag::new(
		tag,
	)))
}

pub fn late_bee_type_up_1_21_9(
	_v: &a::slot_component_bees_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(crate::registry::names(Version::V1_21_9, "entities")
		.and_then(|n| n.id("bee"))
		.unwrap_or(0) as i32)
}

fn scales(p: &a::configuration_to_client_packet_registry_data::Ty) -> Vec<f64> {
	p.entries
		.iter()
		.map(|e| {
			match e
				.value
				.as_ref()
				.and_then(|t| t.value.field("coordinate_scale"))
			{
				Some(Payload::Double(d)) => *d,
				Some(Payload::Float(f)) => *f as f64,
				_ if e.key.strip_prefix("minecraft:").unwrap_or(&e.key) == "the_nether" => 8.0,
				_ => 1.0,
			}
		})
		.collect()
}

pub fn late_registry_data_down_1_21_8(
	p: b::configuration_to_client_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<a::configuration_to_client_packet::Ty>> {
	shared::count_biomes(&p, cx);
	if p.id.strip_prefix("minecraft:").unwrap_or(&p.id) == "dimension_type" {
		cx.state::<Dimensions>().scales = scales(&p);
	}
	Ok(vec![a::configuration_to_client_packet::Ty::RegistryData(p)])
}

pub fn late_spawn_info_down_1_21_8(
	v: &b::play_to_client_spawn_info::Ty,
	cx: &mut Session,
) -> Result<String> {
	world::world(cx).dimension = v.name.clone();
	cx.state::<Dimensions>().current = v.dimension.max(0) as usize;
	Ok(v.name.clone())
}

pub fn late_code_of_conduct_down_1_21_8(
	_v: String,
	cx: &mut Session,
) -> Result<Vec<a::configuration_to_client_packet::Ty>> {
	let accept = b::configuration_to_server_packet::Ty::AcceptCodeOfConduct(
		b::configuration_to_server_packet_accept_code_of_conduct::Ty {},
	);
	let data = super::util::encode(&accept, b::configuration_to_server_packet::serialize)?;
	cx.send(
		crate::generated::convert::Root::ConfigurationToServer,
		Version::V1_21_9,
		data,
	);
	Ok(Vec::new())
}

#[derive(Default)]
struct Rotation {
	yaw: f32,
	pitch: f32,
}

pub fn late_look_yaw_up_1_21_9(
	v: &a::play_to_server_packet_look::Ty,
	cx: &mut Session,
) -> Result<f32> {
	*cx.state::<Rotation>() = Rotation {
		yaw: v.yaw,
		pitch: v.pitch,
	};
	Ok(v.yaw)
}

pub fn late_position_look_yaw_up_1_21_9(
	v: &a::play_to_server_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<f32> {
	*cx.state::<Rotation>() = Rotation {
		yaw: v.yaw,
		pitch: v.pitch,
	};
	Ok(v.yaw)
}

pub fn late_rotation_up_1_21_9(
	p: a::play_to_client_packet_player_rotation::Ty,
	_cx: &mut Session,
) -> Result<b::play_to_client_packet_player_rotation::Ty> {
	Ok(b::play_to_client_packet_player_rotation::Ty {
		yaw: p.yaw,
		relative_yaw: false,
		pitch: p.pitch,
		relative_pitch: false,
	})
}

pub fn late_text_keep_up_1_21_9(
	v: bungeeoxide_nbt::Tag,
	_cx: &mut Session,
) -> Result<bungeeoxide_nbt::Tag> {
	Ok(v)
}

pub fn late_rotation_down_1_21_8(
	p: b::play_to_client_packet_player_rotation::Ty,
	cx: &mut Session,
) -> Result<a::play_to_client_packet_player_rotation::Ty> {
	let r = cx.state::<Rotation>();
	let yaw = if p.relative_yaw { r.yaw + p.yaw } else { p.yaw };
	let pitch = if p.relative_pitch {
		r.pitch + p.pitch
	} else {
		p.pitch
	};
	*r = Rotation { yaw, pitch };
	Ok(a::play_to_client_packet_player_rotation::Ty { yaw, pitch })
}

struct Mannequin {
	uuid: u128,
	name: String,
	x: f64,
	y: f64,
	z: f64,
	yaw: i8,
	pitch: i8,
	head_yaw: i8,
	properties: Vec<a::game_profile_properties_items::Ty>,
	display: Option<Tag>,
	metadata: Vec<b::entity_metadata_entry::Ty>,
	passengers: Vec<i32>,
	equipment: Vec<b::play_to_client_packet_entity_equipment_equipments_items::Ty>,
}

#[derive(Default)]
struct Mannequins {
	by_id: HashMap<i32, Mannequin>,
	bundling: bool,
}

fn mannequins(cx: &mut Session) -> &mut Mannequins {
	cx.state::<Mannequins>()
}

fn hidden_name() -> String {
	use std::hash::BuildHasher;
	let mut bits = std::collections::hash_map::RandomState::new().hash_one(4u8);
	let mut name = String::new();
	for _ in 0..7 {
		name.push('\u{a7}');
		name.push(char::from_digit((bits & 0xf) as u32, 16).unwrap_or('f'));
		bits >>= 4;
	}
	name.push_str("\u{a7}r");
	name
}

fn empty_text() -> Tag {
	Tag::new(Payload::String(String::new()))
}

fn label(m: &Mannequin) -> Tag {
	m.display
		.clone()
		.unwrap_or_else(|| Tag::new(Payload::String("Mannequin".to_owned())))
}

fn player_info_add(m: &Mannequin) -> a::play_to_client_packet::Ty {
	use a::play_to_client_packet_player_info_data_items_chat_session::Ty as Session;
	use a::play_to_client_packet_player_info_data_items_display_name::Ty as Name;
	use a::play_to_client_packet_player_info_data_items_gamemode::Ty as Gamemode;
	use a::play_to_client_packet_player_info_data_items_latency::Ty as Latency;
	use a::play_to_client_packet_player_info_data_items_list_priority::Ty as Priority;
	use a::play_to_client_packet_player_info_data_items_listed::Ty as Listed;
	use a::play_to_client_packet_player_info_data_items_player::Ty as Player;
	use a::play_to_client_packet_player_info_data_items_show_hat::Ty as Hat;
	a::play_to_client_packet::Ty::PlayerInfo(a::play_to_client_packet_player_info::Ty {
		action: a::play_to_client_packet_player_info_action::Ty {
			add_player: true,
			initialize_chat: true,
			update_game_mode: true,
			update_listed: true,
			update_latency: true,
			update_display_name: true,
			update_list_order: true,
			update_hat: true,
		},
		data: vec![a::play_to_client_packet_player_info_data_items::Ty {
			uuid: m.uuid,
			player: Player::True(a::game_profile::Ty {
				name: m.name.clone(),
				properties: m.properties.clone(),
			}),
			chat_session: Session::True(None),
			gamemode: Gamemode::True(0),
			listed: Listed::True(0),
			latency: Latency::True(0),
			display_name: Name::True(m.display.clone()),
			list_priority: Priority::True(1000),
			show_hat: Hat::True(true),
		}],
	})
}

fn team_add(m: &Mannequin) -> a::play_to_client_packet::Ty {
	a::play_to_client_packet::Ty::Teams(a::play_to_client_packet_teams::Ty {
		team: m.name.clone(),
		mode: a::play_to_client_packet_teams_mode::Ty::Add(
			a::play_to_client_packet_teams_mode_add::Ty {
				name: label(m),
				flags: a::play_to_client_packet_teams_mode_add_flags::Ty {
					friendly_fire: false,
					see_friendly_invisible: false,
				},
				name_tag_visibility:
					a::play_to_client_packet_teams_mode_add_name_tag_visibility::Ty::Always,
				collision_rule: a::play_to_client_packet_teams_mode_add_collision_rule::Ty::Always,
				formatting: 21,
				prefix: label(m),
				suffix: empty_text(),
				players: vec![m.name.clone()],
			},
		),
	})
}

fn team_change(m: &Mannequin) -> a::play_to_client_packet::Ty {
	a::play_to_client_packet::Ty::Teams(a::play_to_client_packet_teams::Ty {
		team: m.name.clone(),
		mode: a::play_to_client_packet_teams_mode::Ty::Change(
			a::play_to_client_packet_teams_mode_change::Ty {
				name: label(m),
				flags: a::play_to_client_packet_teams_mode_change_flags::Ty {
					friendly_fire: false,
					see_friendly_invisible: false,
				},
				name_tag_visibility:
					a::play_to_client_packet_teams_mode_change_name_tag_visibility::Ty::Always,
				collision_rule:
					a::play_to_client_packet_teams_mode_change_collision_rule::Ty::Always,
				formatting: 21,
				prefix: label(m),
				suffix: empty_text(),
			},
		),
	})
}

fn player_info_display(m: &Mannequin) -> a::play_to_client_packet::Ty {
	use a::play_to_client_packet_player_info_data_items_chat_session::Ty as Session;
	use a::play_to_client_packet_player_info_data_items_display_name::Ty as Name;
	use a::play_to_client_packet_player_info_data_items_gamemode::Ty as Gamemode;
	use a::play_to_client_packet_player_info_data_items_latency::Ty as Latency;
	use a::play_to_client_packet_player_info_data_items_list_priority::Ty as Priority;
	use a::play_to_client_packet_player_info_data_items_listed::Ty as Listed;
	use a::play_to_client_packet_player_info_data_items_player::Ty as Player;
	use a::play_to_client_packet_player_info_data_items_show_hat::Ty as Hat;
	a::play_to_client_packet::Ty::PlayerInfo(a::play_to_client_packet_player_info::Ty {
		action: a::play_to_client_packet_player_info_action::Ty {
			add_player: false,
			initialize_chat: false,
			update_game_mode: false,
			update_listed: false,
			update_latency: false,
			update_display_name: true,
			update_list_order: false,
			update_hat: false,
		},
		data: vec![a::play_to_client_packet_player_info_data_items::Ty {
			uuid: m.uuid,
			player: Player::Default(false),
			chat_session: Session::Default(false),
			gamemode: Gamemode::Default(false),
			listed: Listed::Default(false),
			latency: Latency::Default(false),
			display_name: Name::True(m.display.clone()),
			list_priority: Priority::Default(false),
			show_hat: Hat::Default(false),
		}],
	})
}

fn forget(m: &Mannequin) -> [a::play_to_client_packet::Ty; 2] {
	[
		a::play_to_client_packet::Ty::PlayerRemove(vec![m.uuid]),
		a::play_to_client_packet::Ty::Teams(a::play_to_client_packet_teams::Ty {
			team: m.name.clone(),
			mode: a::play_to_client_packet_teams_mode::Ty::Remove,
		}),
	]
}

fn player_spawn(id: i32, m: &Mannequin) -> Result<a::play_to_client_packet::Ty> {
	let player = crate::registry::names(Version::V1_21_8, "entities")
		.and_then(|n| n.id("player"))
		.ok_or(Error::Dropped)?;
	Ok(a::play_to_client_packet::Ty::SpawnEntity(
		a::play_to_client_packet_spawn_entity::Ty {
			entity_id: id,
			object_uuid: m.uuid,
			r#type: player as i32,
			x: m.x,
			y: m.y,
			z: m.z,
			pitch: m.pitch,
			yaw: m.yaw,
			head_pitch: m.head_yaw,
			object_data: 0,
			velocity: a::vec3i16::Ty { x: 0, y: 0, z: 0 },
		},
	))
}

fn queue_old(cx: &mut Session, p: &a::play_to_client_packet::Ty) -> Result<()> {
	let data = super::util::encode(p, a::play_to_client_packet::serialize)?;
	cx.send(
		crate::generated::convert::Root::PlayToClient,
		Version::V1_21_8,
		data,
	);
	Ok(())
}

fn queue_new(cx: &mut Session, p: &b::play_to_client_packet::Ty) -> Result<()> {
	let data = super::util::encode(p, b::play_to_client_packet::serialize)?;
	cx.send(
		crate::generated::convert::Root::PlayToClient,
		Version::V1_21_9,
		data,
	);
	Ok(())
}

fn profile_properties(p: &b::resolvable_profile::Ty) -> Vec<a::game_profile_properties_items::Ty> {
	let properties = match &p.anon {
		b::resolvable_profile_anon::Ty::Partial(p) => p.properties.as_slice(),
		b::resolvable_profile_anon::Ty::Complete(p) => p.properties.as_slice(),
		b::resolvable_profile_anon::Ty::Default(_) => &[],
	};
	properties
		.iter()
		.map(|p| a::game_profile_properties_items::Ty {
			name: p.name.clone(),
			value: p.value.clone(),
			signature: p.signature.clone(),
		})
		.collect()
}

fn mannequin_metadata(
	p: &b::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<()> {
	let Some(m) = mannequins(cx).by_id.get_mut(&p.entity_id) else {
		return Ok(());
	};
	let mut profile = None;
	let mut renamed = false;
	for e in &p.metadata {
		match &e.value {
			b::entity_metadata_entry_value::Ty::ResolvableProfile(v) => profile = Some(v),
			value => {
				if let b::entity_metadata_entry_value::Ty::OptionalComponent(name) = value {
					let display = name.clone().map(text::unobject);
					if is_custom_name(e.key) && m.display != display {
						m.display = display;
						renamed = true;
					}
				}
				m.metadata.retain(|old| old.key != e.key);
				m.metadata.push(e.clone());
			}
		}
	}
	let mut updates = Vec::new();
	if let Some(profile) = profile {
		m.properties = profile_properties(profile);
	} else if renamed {
		updates.push(player_info_display(m));
	}
	if renamed {
		updates.push(team_change(m));
	}
	if profile.is_some() {
		respawn_mannequin(cx, p.entity_id)?;
	}
	for u in &updates {
		queue_old(cx, u)?;
	}
	Ok(())
}

fn is_custom_name(key: u8) -> bool {
	crate::registry::names(Version::V1_21_9, "entities")
		.and_then(|n| n.id("mannequin"))
		.and_then(|id| crate::registry::entity_metadata_keys(Version::V1_21_9, id))
		.and_then(|keys| keys.get(key as usize))
		.is_some_and(|k| k == "custom_name")
}

fn respawn_mannequin(cx: &mut Session, id: i32) -> Result<()> {
	let bundling = mannequins(cx).bundling;
	let Some(m) = mannequins(cx).by_id.remove(&id) else {
		return Ok(());
	};
	let mut old = Vec::new();
	if !bundling {
		old.push(a::play_to_client_packet::Ty::BundleDelimiter);
	}
	old.push(a::play_to_client_packet::Ty::EntityDestroy(vec![id]));
	old.push(a::play_to_client_packet::Ty::PlayerRemove(vec![m.uuid]));
	old.push(player_info_add(&m));
	old.push(player_spawn(id, &m)?);
	for p in &old {
		queue_old(cx, p)?;
	}
	if !m.metadata.is_empty() {
		queue_new(
			cx,
			&b::play_to_client_packet::Ty::EntityMetadata(
				b::play_to_client_packet_entity_metadata::Ty {
					entity_id: id,
					metadata: m.metadata.clone(),
				},
			),
		)?;
	}
	if !m.passengers.is_empty() {
		queue_old(
			cx,
			&a::play_to_client_packet::Ty::SetPassengers(
				a::play_to_client_packet_set_passengers::Ty {
					entity_id: id,
					passengers: m.passengers.clone(),
				},
			),
		)?;
	}
	if !m.equipment.is_empty() {
		queue_new(
			cx,
			&b::play_to_client_packet::Ty::EntityEquipment(
				b::play_to_client_packet_entity_equipment::Ty {
					entity_id: id,
					equipments: m.equipment.clone(),
				},
			),
		)?;
	}
	if !bundling {
		queue_old(cx, &a::play_to_client_packet::Ty::BundleDelimiter)?;
	}
	mannequins(cx).by_id.insert(id, m);
	Ok(())
}

pub fn late_spawn_entity_down_1_21_8(
	p: b::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	let spawn = a::play_to_client_packet_spawn_entity::Ty {
		entity_id: p.entity_id,
		object_uuid: p.object_uuid,
		r#type: late_spawn_type_down_1_21_8(&p, cx)?,
		x: p.x,
		y: p.y,
		z: p.z,
		pitch: p.pitch,
		yaw: p.yaw,
		head_pitch: p.head_pitch,
		object_data: late_object_data_down_1_21_8(&p, cx)?,
		velocity: late_spawn_velocity_down_1_21_8(&p, cx)?,
	};
	let mut out = Vec::new();
	if let Some(stale) = mannequins(cx).by_id.remove(&p.entity_id) {
		out.extend(forget(&stale));
	}
	let kind = crate::registry::names(Version::V1_21_9, "entities")
		.and_then(|n| n.name(p.r#type.max(0) as u32));
	if kind == Some("mannequin") {
		let m = Mannequin {
			uuid: p.object_uuid,
			name: hidden_name(),
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			head_yaw: p.head_pitch,
			properties: Vec::new(),
			display: None,
			metadata: Vec::new(),
			passengers: Vec::new(),
			equipment: Vec::new(),
		};
		out.push(player_info_add(&m));
		out.push(team_add(&m));
		mannequins(cx).by_id.insert(p.entity_id, m);
	}
	out.push(a::play_to_client_packet::Ty::SpawnEntity(spawn));
	Ok(out)
}

pub fn late_entity_destroy_down_1_21_8(
	ids: Vec<i32>,
	cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	let gone: Vec<Mannequin> = ids
		.iter()
		.filter_map(|id| mannequins(cx).by_id.remove(id))
		.collect();
	let mut out = vec![a::play_to_client_packet::Ty::EntityDestroy(ids)];
	for m in &gone {
		out.extend(forget(m));
	}
	Ok(out)
}

pub fn late_bundle_down_1_21_8(
	_v: (),
	cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	let m = mannequins(cx);
	m.bundling = !m.bundling;
	Ok(vec![a::play_to_client_packet::Ty::BundleDelimiter])
}

pub fn late_login_entity_down_1_21_8(
	p: &b::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<i32> {
	*mannequins(cx) = Mannequins::default();
	Ok(p.entity_id)
}

pub fn late_respawn_down_1_21_8(
	p: &b::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<u8> {
	let gone = std::mem::take(&mut mannequins(cx).by_id);
	for m in gone.values() {
		for packet in forget(m) {
			queue_old(cx, &packet)?;
		}
	}
	Ok(p.copy_metadata)
}

fn track(cx: &mut Session, id: i32, f: impl FnOnce(&mut Mannequin)) -> i32 {
	if let Some(m) = mannequins(cx).by_id.get_mut(&id) {
		f(m);
	}
	id
}

fn angle(degrees: f32) -> i8 {
	(degrees * 256.0 / 360.0).floor() as i32 as i8
}

pub fn late_teleport_entity_down_1_21_8(
	p: &b::play_to_client_packet_entity_teleport::Ty,
	cx: &mut Session,
) -> Result<i32> {
	Ok(track(cx, p.entity_id, |m| {
		let r = &p.flags;
		m.x = if r.x { m.x + p.x } else { p.x };
		m.y = if r.y { m.y + p.y } else { p.y };
		m.z = if r.z { m.z + p.z } else { p.z };
		m.yaw = if r.yaw {
			m.yaw.wrapping_add(angle(p.yaw))
		} else {
			angle(p.yaw)
		};
		m.pitch = if r.pitch {
			m.pitch.wrapping_add(angle(p.pitch))
		} else {
			angle(p.pitch)
		};
	}))
}

pub fn late_sync_position_entity_down_1_21_8(
	p: &b::play_to_client_packet_sync_entity_position::Ty,
	cx: &mut Session,
) -> Result<i32> {
	Ok(track(cx, p.entity_id, |m| {
		(m.x, m.y, m.z) = (p.x, p.y, p.z);
		(m.yaw, m.pitch) = (angle(p.yaw), angle(p.pitch));
	}))
}

fn moved(m: &mut Mannequin, dx: i16, dy: i16, dz: i16) {
	m.x += dx as f64 / 4096.0;
	m.y += dy as f64 / 4096.0;
	m.z += dz as f64 / 4096.0;
}

pub fn late_rel_move_entity_down_1_21_8(
	p: &b::play_to_client_packet_rel_entity_move::Ty,
	cx: &mut Session,
) -> Result<i32> {
	Ok(track(cx, p.entity_id, |m| moved(m, p.d_x, p.d_y, p.d_z)))
}

pub fn late_move_look_entity_down_1_21_8(
	p: &b::play_to_client_packet_entity_move_look::Ty,
	cx: &mut Session,
) -> Result<i32> {
	Ok(track(cx, p.entity_id, |m| {
		moved(m, p.d_x, p.d_y, p.d_z);
		(m.yaw, m.pitch) = (p.yaw, p.pitch);
	}))
}

pub fn late_look_entity_down_1_21_8(
	p: &b::play_to_client_packet_entity_look::Ty,
	cx: &mut Session,
) -> Result<i32> {
	Ok(track(cx, p.entity_id, |m| {
		(m.yaw, m.pitch) = (p.yaw, p.pitch)
	}))
}

pub fn late_head_rotation_entity_down_1_21_8(
	p: &b::play_to_client_packet_entity_head_rotation::Ty,
	cx: &mut Session,
) -> Result<i32> {
	Ok(track(cx, p.entity_id, |m| m.head_yaw = p.head_yaw))
}

pub fn late_passengers_entity_down_1_21_8(
	p: &b::play_to_client_packet_set_passengers::Ty,
	cx: &mut Session,
) -> Result<i32> {
	Ok(track(cx, p.entity_id, |m| {
		m.passengers = p.passengers.clone()
	}))
}

pub fn late_equipment_entity_down_1_21_8(
	p: &b::play_to_client_packet_entity_equipment::Ty,
	cx: &mut Session,
) -> Result<i32> {
	Ok(track(cx, p.entity_id, |m| {
		for e in &p.equipments {
			m.equipment.retain(|old| old.slot != e.slot);
			m.equipment.push(e.clone());
		}
	}))
}

text_down!(b => a, text::unobject, {
	tag: late_unobject_down_1_21_8,
	links: [late_config_links_down_1_21_8, late_links_down_1_21_8],
	window: late_open_window_down_1_21_8,
	chat: late_player_chat_down_1_21_8,
	packets: [
		play_to_client_packet_system_chat: system_chat => late_system_chat_down_1_21_8,
		play_to_client_packet_playerlist_header: header => late_header_down_1_21_8,
		play_to_client_packet_death_combat_event: death => late_death_down_1_21_8,
		play_to_client_packet_profileless_chat: profileless => late_profileless_down_1_21_8,
		play_to_client_packet_scoreboard_objective: objective => late_objective_down_1_21_8,
		play_to_client_packet_scoreboard_score: score => late_score_down_1_21_8,
		play_to_client_packet_boss_bar: boss_bar => late_boss_bar_down_1_21_8,
		play_to_client_packet_map: map => late_map_down_1_21_8,
		play_to_client_packet_tab_complete: tab_complete => late_tab_complete_down_1_21_8,
		play_to_client_packet_server_data: server_data => late_server_data_down_1_21_8,
		packet_common_add_resource_pack: resource_pack => late_resource_pack_down_1_21_8,
	],
	arms: [
		play_to_client_packet: ActionBar => late_action_bar_down_1_21_8,
		play_to_client_packet: SetTitleText => late_title_down_1_21_8,
		play_to_client_packet: SetTitleSubtitle => late_subtitle_down_1_21_8,
		play_to_client_packet: KickDisconnect => late_kick_down_1_21_8,
		configuration_to_client_packet: Disconnect => late_config_disconnect_down_1_21_8,
	],
});
