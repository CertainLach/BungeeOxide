use super::remap::{self, remaps};
use super::rules::{blocks, entities, items, statistics, tags};
use super::shared::{self, meta, text_down};
use super::slots::{self, untrusted_step};
use super::text;
use super::util::{lp_vec3, read_lp_vec3};
use super::world;
use crate::gen::convert::{v1_21_8 as a, v1_21_9 as b, Version};
use crate::pds::convert::{Error, Result};
use crate::Session;
use nbt::{Entry, Payload, Tag};

remaps!(Version::V1_21_8 => Version::V1_21_9 { late_sound_up_1_21_9: a::ItemSoundHolder::Ty = remap::sound; });
remaps!(Version::V1_21_9 => Version::V1_21_8 { late_sound_down_1_21_8: a::ItemSoundHolder::Ty = remap::sound; });

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

pub fn late_matchers_up_1_21_9(
	v: &a::ItemBlockPredicate::Ty,
	_cx: &mut Session,
) -> Result<b::DataComponentMatchers::Ty> {
	Ok(v.components.clone())
}

pub fn late_matchers_down_1_21_8(
	v: &b::ItemBlockPredicate::Ty,
	_cx: &mut Session,
) -> Result<a::DataComponentMatchers::Ty> {
	Ok(v.components.clone())
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
	if v >= 40 {
		v + 1
	} else {
		v
	}
}

fn block_entity_down(v: i32) -> Result<i32> {
	match v {
		40 | 48.. => Err(Error::Dropped),
		v if v > 40 => Ok(v - 1),
		v => Ok(v),
	}
}

pub fn late_chunk_block_entity_up_1_21_9(
	v: &a::chunkBlockEntity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(block_entity_up(v.r#type))
}

pub fn late_chunk_block_entity_down_1_21_8(
	v: &b::chunkBlockEntity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block_entity_down(v.r#type)
}

pub fn late_tile_entity_up_1_21_9(
	v: &a::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(block_entity_up(v.action))
}

pub fn late_tile_entity_down_1_21_8(
	v: &b::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block_entity_down(v.action)
}

fn tag_id(tag: &nbt::Tag) -> Option<&str> {
	let id = tag.value.field("id").and_then(Payload::as_str)?;
	Some(id.strip_prefix("minecraft:").unwrap_or(id))
}

fn with_id(mut tag: nbt::Tag, name: &str) -> nbt::Tag {
	if !matches!(tag.value, Payload::compound(_)) {
		tag = Tag::new(Payload::compound(vec![]));
	}
	tag.value
		.put("id", Payload::string(format!("minecraft:{name}")));
	tag
}

pub fn late_entity_data_up_1_21_9(v: nbt::Tag, _cx: &mut Session) -> Result<b::SlotComponent::Ty> {
	let name = tag_id(&v).unwrap_or("pig");
	let r#type = crate::registry::names(Version::V1_21_9, "entities")
		.and_then(|n| n.id(name))
		.unwrap_or(0) as i32;
	Ok(b::SlotComponent::Ty::entity_data(
		b::SlotComponent__entity_data::Ty { r#type, data: v },
	))
}

pub fn late_entity_data_down_1_21_8(
	v: b::SlotComponent__entity_data::Ty,
	_cx: &mut Session,
) -> Result<a::SlotComponent::Ty> {
	let name = crate::registry::names(Version::V1_21_9, "entities")
		.and_then(|n| n.name(v.r#type.max(0) as u32));
	Ok(a::SlotComponent::Ty::entity_data(match name {
		Some(name) => with_id(v.data, name),
		None => v.data,
	}))
}

pub fn late_block_entity_data_up_1_21_9(
	v: nbt::Tag,
	_cx: &mut Session,
) -> Result<b::SlotComponent::Ty> {
	let name = tag_id(&v).unwrap_or("furnace");
	let r#type = BLOCK_ENTITIES.iter().position(|n| *n == name).unwrap_or(0) as i32;
	Ok(b::SlotComponent::Ty::block_entity_data(
		b::SlotComponent__block_entity_data::Ty { r#type, data: v },
	))
}

pub fn late_block_entity_data_down_1_21_8(
	v: b::SlotComponent__block_entity_data::Ty,
	_cx: &mut Session,
) -> Result<a::SlotComponent::Ty> {
	Ok(a::SlotComponent::Ty::block_entity_data(
		match BLOCK_ENTITIES.get(v.r#type.max(0) as usize) {
			Some(name) => with_id(v.data, name),
			None => v.data,
		},
	))
}

pub fn late_profile_up_1_21_9(
	v: a::SlotComponent__profile::Ty,
	_cx: &mut Session,
) -> Result<b::SlotComponent::Ty> {
	let properties = v
		.properties
		.into_iter()
		.map(|p| b::GameProfileProperty::Ty {
			name: p.name,
			value: p.value,
			signature: p.signature,
		})
		.collect();
	Ok(b::SlotComponent::Ty::profile(b::ResolvableProfile::Ty {
		ANON: b::ResolvableProfile__ANON::Ty::partial(b::PartialResolvableProfile::Ty {
			name: v.name,
			uuid: v.uuid,
			properties,
		}),
		skinPatch: b::PlayerSkinPatch::Ty {
			body: None,
			cape: None,
			elytra: None,
			model: None,
		},
	}))
}

pub fn late_profile_down_1_21_8(
	v: b::ResolvableProfile::Ty,
	_cx: &mut Session,
) -> Result<a::SlotComponent::Ty> {
	let (name, uuid, properties) = match v.ANON {
		b::ResolvableProfile__ANON::Ty::partial(p) => (p.name, p.uuid, p.properties),
		b::ResolvableProfile__ANON::Ty::complete(p) => (Some(p.name), Some(p.uuid), p.properties),
		b::ResolvableProfile__ANON::Ty::default(_) => (None, None, Vec::new()),
	};
	let properties = properties
		.into_iter()
		.map(|p| a::SlotComponent__profile__properties__items::Ty {
			name: p.name,
			value: p.value,
			signature: p.signature,
		})
		.collect();
	Ok(a::SlotComponent::Ty::profile(
		a::SlotComponent__profile::Ty {
			name,
			uuid,
			properties,
		},
	))
}

pub fn late_particle_effect_up_1_21_9(_v: (), _cx: &mut Session) -> Result<b::Particle::Ty> {
	Ok(b::Particle::Ty::effect(b::Particle__effect::Ty {
		color: -1,
		power: 1.0,
	}))
}

pub fn late_particle_instant_effect_up_1_21_9(
	_v: (),
	_cx: &mut Session,
) -> Result<b::Particle::Ty> {
	Ok(b::Particle::Ty::instant_effect(
		b::Particle__instant_effect::Ty {
			color: -1,
			power: 1.0,
		},
	))
}

pub fn late_particle_flash_up_1_21_9(_v: (), _cx: &mut Session) -> Result<b::Particle::Ty> {
	Ok(b::Particle::Ty::flash(-1))
}

pub fn late_particle_dragon_breath_up_1_21_9(_v: (), _cx: &mut Session) -> Result<b::Particle::Ty> {
	Ok(b::Particle::Ty::dragon_breath(1.0))
}

pub fn late_particle_effect_down_1_21_8(
	_v: b::Particle__effect::Ty,
	_cx: &mut Session,
) -> Result<a::Particle::Ty> {
	Ok(a::Particle::Ty::effect)
}

pub fn late_particle_instant_effect_down_1_21_8(
	_v: b::Particle__instant_effect::Ty,
	_cx: &mut Session,
) -> Result<a::Particle::Ty> {
	Ok(a::Particle::Ty::instant_effect)
}

pub fn late_particle_flash_down_1_21_8(_v: i32, _cx: &mut Session) -> Result<a::Particle::Ty> {
	Ok(a::Particle::Ty::flash)
}

pub fn late_particle_dragon_breath_down_1_21_8(
	_v: f32,
	_cx: &mut Session,
) -> Result<a::Particle::Ty> {
	Ok(a::Particle::Ty::dragon_breath)
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
	v: &a::play_toClient_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	Ok(velocity_up(&v.velocity))
}

pub fn late_spawn_velocity_down_1_21_8(
	v: &b::play_toClient_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<a::vec3i16::Ty> {
	velocity_down(&v.velocity)
}

pub fn late_velocity_up_1_21_9(
	v: &a::play_toClient_packet_entity_velocity::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	Ok(velocity_up(&v.velocity))
}

pub fn late_velocity_down_1_21_8(
	v: &b::play_toClient_packet_entity_velocity::Ty,
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
	mut p: a::configuration_toClient_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_toClient_packet::Ty>> {
	shared::count_biomes(&p, cx);
	if p.id.strip_prefix("minecraft:").unwrap_or(&p.id) == "dimension_type" {
		cx.state::<Dimensions>().scales = scales(&p);
	}
	shared::registry_entries(&mut p, "dimension_type", |key, e| {
		if key.strip_prefix("minecraft:").unwrap_or(key) == "the_end" {
			e.put("ambient_light", Payload::float(0.25));
		}
	});
	Ok(vec![b::configuration_toClient_packet::Ty::registry_data(p)])
}

pub fn late_spawn_info_up_1_21_9(
	v: &a::play_toClient_SpawnInfo::Ty,
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
	v: &a::play_toClient_packet_initialize_world_border::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.x / scale(cx))
}

pub fn late_border_z_up_1_21_9(
	v: &a::play_toClient_packet_initialize_world_border::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.z / scale(cx))
}

pub fn late_border_x_down_1_21_8(
	v: &b::play_toClient_packet_initialize_world_border::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.x * scale(cx))
}

pub fn late_border_z_down_1_21_8(
	v: &b::play_toClient_packet_initialize_world_border::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.z * scale(cx))
}

pub fn late_border_center_x_up_1_21_9(
	v: &a::play_toClient_packet_world_border_center::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.x / scale(cx))
}

pub fn late_border_center_z_up_1_21_9(
	v: &a::play_toClient_packet_world_border_center::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.z / scale(cx))
}

pub fn late_border_center_x_down_1_21_8(
	v: &b::play_toClient_packet_world_border_center::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.x * scale(cx))
}

pub fn late_border_center_z_down_1_21_8(
	v: &b::play_toClient_packet_world_border_center::Ty,
	cx: &mut Session,
) -> Result<f64> {
	Ok(v.z * scale(cx))
}

pub fn late_spawn_position_up_1_21_9(
	p: a::play_toClient_packet_spawn_position::Ty,
	cx: &mut Session,
) -> Result<Vec<b::play_toClient_packet::Ty>> {
	let dimension = world::world(cx).dimension.clone();
	let dimensionName = if dimension.is_empty() {
		"minecraft:overworld".to_owned()
	} else {
		dimension
	};
	Ok(vec![b::play_toClient_packet::Ty::spawn_position(
		b::RespawnData::Ty {
			globalPos: b::GlobalPos::Ty {
				dimensionName,
				location: p.location,
			},
			yaw: p.angle,
			pitch: 0.0,
		},
	)])
}

pub fn late_spawn_position_down_1_21_8(
	p: b::RespawnData::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_toClient_packet::Ty>> {
	Ok(vec![a::play_toClient_packet::Ty::spawn_position(
		a::play_toClient_packet_spawn_position::Ty {
			location: p.globalPos.location,
			angle: p.yaw,
		},
	)])
}

pub fn late_meta_entity_up_1_21_9(
	p: &a::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, Version::V1_21_8, p.entityId);
	Ok(p.entityId)
}

pub fn late_meta_entity_down_1_21_8(
	p: &b::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, Version::V1_21_9, p.entityId);
	Ok(p.entityId)
}

pub fn late_meta_key_up_1_21_9(e: &a::entityMetadataEntry::Ty, cx: &mut Session) -> Result<u8> {
	meta(cx).key = e.key;
	Ok(match (shared::is(cx, &["player"]), e.key) {
		(true, 15) => 17,
		(true, 16) => 18,
		(true, 17) => 16,
		(true, 18) => 15,
		(_, k) => k,
	})
}

pub fn late_meta_key_down_1_21_8(e: &b::entityMetadataEntry::Ty, cx: &mut Session) -> Result<u8> {
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
	v: nbt::Tag,
	cx: &mut Session,
) -> Result<b::entityMetadataEntry__value::Ty> {
	if !is_shoulder(cx) {
		return Err(Error::Dropped);
	}
	let variant = v
		.value
		.field("Variant")
		.and_then(Payload::as_i64)
		.filter(|_| v.value.field("id").and_then(Payload::as_str).is_some());
	Ok(b::entityMetadataEntry__value::Ty::optional_unsigned_int(
		variant.map_or(0, |v| v as i32 + 1),
	))
}

pub fn late_meta_unsigned_down_1_21_8(
	v: i32,
	cx: &mut Session,
) -> Result<a::entityMetadataEntry__value::Ty> {
	if !is_shoulder(cx) {
		return Ok(a::entityMetadataEntry__value::Ty::optional_unsigned_int(v));
	}
	let tag = match v {
		0 => Payload::compound(vec![]),
		v => Payload::compound(vec![
			Entry::new("id", Payload::from("minecraft:parrot")),
			Entry::new("Variant", Payload::int(v - 1)),
		]),
	};
	Ok(a::entityMetadataEntry__value::Ty::compound_tag(Tag::new(
		tag,
	)))
}

pub fn late_bee_type_up_1_21_9(
	_v: &a::SlotComponent__bees__items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(crate::registry::names(Version::V1_21_9, "entities")
		.and_then(|n| n.id("bee"))
		.unwrap_or(0) as i32)
}

fn scales(p: &a::configuration_toClient_packet_registry_data::Ty) -> Vec<f64> {
	p.entries
		.iter()
		.map(|e| {
			match e
				.value
				.as_ref()
				.and_then(|t| t.value.field("coordinate_scale"))
			{
				Some(Payload::double(d)) => *d,
				Some(Payload::float(f)) => *f as f64,
				_ if e.key.strip_prefix("minecraft:").unwrap_or(&e.key) == "the_nether" => 8.0,
				_ => 1.0,
			}
		})
		.collect()
}

pub fn late_registry_data_down_1_21_8(
	p: b::configuration_toClient_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<a::configuration_toClient_packet::Ty>> {
	shared::count_biomes(&p, cx);
	if p.id.strip_prefix("minecraft:").unwrap_or(&p.id) == "dimension_type" {
		cx.state::<Dimensions>().scales = scales(&p);
	}
	Ok(vec![a::configuration_toClient_packet::Ty::registry_data(p)])
}

pub fn late_spawn_info_down_1_21_8(
	v: &b::play_toClient_SpawnInfo::Ty,
	cx: &mut Session,
) -> Result<String> {
	world::world(cx).dimension = v.name.clone();
	cx.state::<Dimensions>().current = v.dimension.max(0) as usize;
	Ok(v.name.clone())
}

pub fn late_code_of_conduct_down_1_21_8(
	_v: String,
	cx: &mut Session,
) -> Result<Vec<a::configuration_toClient_packet::Ty>> {
	let accept = b::configuration_toServer_packet::Ty::accept_code_of_conduct(
		b::configuration_toServer_packet_accept_code_of_conduct::Ty {},
	);
	let data = super::util::encode(&accept, b::configuration_toServer_packet::serialize)?;
	cx.send(
		crate::gen::convert::Root::ConfigurationToServer,
		Version::V1_21_9.index(),
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
	v: &a::play_toServer_packet_look::Ty,
	cx: &mut Session,
) -> Result<f32> {
	*cx.state::<Rotation>() = Rotation {
		yaw: v.yaw,
		pitch: v.pitch,
	};
	Ok(v.yaw)
}

pub fn late_position_look_yaw_up_1_21_9(
	v: &a::play_toServer_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<f32> {
	*cx.state::<Rotation>() = Rotation {
		yaw: v.yaw,
		pitch: v.pitch,
	};
	Ok(v.yaw)
}

pub fn late_rotation_up_1_21_9(
	p: a::play_toClient_packet_player_rotation::Ty,
	_cx: &mut Session,
) -> Result<b::play_toClient_packet_player_rotation::Ty> {
	Ok(b::play_toClient_packet_player_rotation::Ty {
		yaw: p.yaw,
		relativeYaw: false,
		pitch: p.pitch,
		relativePitch: false,
	})
}

pub fn late_text_keep_up_1_21_9(v: nbt::Tag, _cx: &mut Session) -> Result<nbt::Tag> {
	Ok(v)
}

pub fn late_rotation_down_1_21_8(
	p: b::play_toClient_packet_player_rotation::Ty,
	cx: &mut Session,
) -> Result<a::play_toClient_packet_player_rotation::Ty> {
	let r = cx.state::<Rotation>();
	let yaw = if p.relativeYaw { r.yaw + p.yaw } else { p.yaw };
	let pitch = if p.relativePitch {
		r.pitch + p.pitch
	} else {
		p.pitch
	};
	*r = Rotation { yaw, pitch };
	Ok(a::play_toClient_packet_player_rotation::Ty { yaw, pitch })
}

text_down!(b => a, text::unobject, {
	tag: late_unobject_down_1_21_8,
	links: [late_config_links_down_1_21_8, late_links_down_1_21_8],
	window: late_open_window_down_1_21_8,
	chat: late_player_chat_down_1_21_8,
	packets: [
		play_toClient_packet_system_chat: system_chat => late_system_chat_down_1_21_8,
		play_toClient_packet_playerlist_header: header => late_header_down_1_21_8,
		play_toClient_packet_death_combat_event: death => late_death_down_1_21_8,
		play_toClient_packet_profileless_chat: profileless => late_profileless_down_1_21_8,
		play_toClient_packet_scoreboard_objective: objective => late_objective_down_1_21_8,
		play_toClient_packet_scoreboard_score: score => late_score_down_1_21_8,
		play_toClient_packet_boss_bar: boss_bar => late_boss_bar_down_1_21_8,
		play_toClient_packet_map: map => late_map_down_1_21_8,
		play_toClient_packet_tab_complete: tab_complete => late_tab_complete_down_1_21_8,
		play_toClient_packet_server_data: server_data => late_server_data_down_1_21_8,
		packet_common_add_resource_pack: resource_pack => late_resource_pack_down_1_21_8,
	],
	arms: [
		play_toClient_packet: action_bar => late_action_bar_down_1_21_8,
		play_toClient_packet: set_title_text => late_title_down_1_21_8,
		play_toClient_packet: set_title_subtitle => late_subtitle_down_1_21_8,
		play_toClient_packet: kick_disconnect => late_kick_down_1_21_8,
		configuration_toClient_packet: disconnect => late_config_disconnect_down_1_21_8,
	],
});
