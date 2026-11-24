use super::components;
use super::registries;
use super::remap::{self, remaps};
use super::rules::{blocks, entities, hidden, items, statistics, tags};
use super::shared::{self, meta};
use super::slots::{self, exact_matchers, untrusted_step};
use super::tags::Derived;
use super::util::{encode, read_varint};
use crate::gen::convert::{v26_1 as a, v26_2 as b, Version};
use crate::pds::convert::{Error, Result};
use crate::Session;
use bungeeoxide_nbt::{Payload, Tag};
use std::collections::HashSet;

remaps!(Version::V26_1 => Version::V26_2 { late_sound_up_26_2: a::item_sound_holder::Ty = remap::sound; });
remaps!(Version::V26_2 => Version::V26_1 { late_sound_down_26_1: a::item_sound_holder::Ty = remap::sound; });

items!(a => b, Version::V26_1 => Version::V26_2 {
	slot: late_slot_item_up_26_2, hashed: late_hashed_item_up_26_2, trade1: late_trade1_up_26_2, trade2: late_trade2_up_26_2,
	display: late_display_item_up_26_2, recipes: late_recipe_items_up_26_2, stonecutter: late_stonecutter_up_26_2,
	requirements: late_requirements_up_26_2, repairable: late_repairable_up_26_2, pots: late_pots_up_26_2,
});
items!(b => a, Version::V26_2 => Version::V26_1 {
	slot: late_slot_item_down_26_1, hashed: late_hashed_item_down_26_1, trade1: late_trade1_down_26_1, trade2: late_trade2_down_26_1,
	display: late_display_item_down_26_1, recipes: late_recipe_items_down_26_1, stonecutter: late_stonecutter_down_26_1,
	requirements: late_requirements_down_26_1, repairable: late_repairable_down_26_1, pots: late_pots_down_26_1,
});

blocks!(a => b, Version::V26_1 => Version::V26_2 {
	block_change: late_block_change_up_26_2, multi_block: late_multi_block_up_26_2, block_action: late_block_action_up_26_2,
	world_event: late_world_event_up_26_2, chunk: late_chunk_up_26_2, tool: late_tool_up_26_2, predicate: late_predicate_up_26_2,
	particle: [late_particle_block_up_26_2, late_particle_marker_up_26_2, late_particle_dust_up_26_2, late_particle_pillar_up_26_2, late_particle_crumble_up_26_2],
	meta: [late_meta_state_up_26_2, late_meta_optional_state_up_26_2],
});
blocks!(b => a, Version::V26_2 => Version::V26_1 {
	block_change: late_block_change_down_26_1, multi_block: late_multi_block_down_26_1, block_action: late_block_action_down_26_1,
	world_event: late_world_event_down_26_1, chunk: late_chunk_down_26_1, tool: late_tool_down_26_1, predicate: late_predicate_down_26_1,
	particle: [late_particle_block_down_26_1, late_particle_marker_down_26_1, late_particle_dust_down_26_1, late_particle_pillar_down_26_1, late_particle_crumble_down_26_1],
	meta: [late_meta_state_down_26_1, late_meta_optional_state_down_26_1],
});

entities!(a => b, Version::V26_1 => Version::V26_2 { spawn: late_spawn_type_up_26_2, object: late_object_data_up_26_2, equippable: late_equip_entities_up_26_2, });
entities!(b => a, Version::V26_2 => Version::V26_1 { spawn: late_spawn_type_down_26_1, object: late_object_data_down_26_1, equippable: late_equip_entities_down_26_1, });

statistics!(a => b, Version::V26_1 => Version::V26_2 { late_statistics_up_26_2 });
statistics!(b => a, Version::V26_2 => Version::V26_1 { late_statistics_down_26_1 });

tags!(a => b, Version::V26_1 => Version::V26_2, &[], { config: config_tags_up_26_2, play: late_play_tags_up_26_2 });
tags!(b => a, Version::V26_2 => Version::V26_1, &[], { config: late_config_tags_down_26_1, play: late_play_tags_down_26_1 });

hidden!(a, Version::V26_1 => Version::V26_2, late_hidden_up_26_2);
hidden!(b, Version::V26_2 => Version::V26_1, late_hidden_down_26_1);

untrusted_step!(late_untrusted_up_26_2: a => b, Version::V26_1 => Version::V26_2, slots::late_to_slot_26_1, slots::late_to_untrusted_26_2);
untrusted_step!(late_untrusted_down_26_1: b => a, Version::V26_2 => Version::V26_1, slots::late_to_slot_26_2, slots::late_to_untrusted_26_1);

exact_matchers!(exact_up: a => b, Version::V26_1 => Version::V26_2);
exact_matchers!(exact_down: b => a, Version::V26_2 => Version::V26_1);

pub fn late_matchers_up_26_2(
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

pub fn late_matchers_down_26_1(
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

pub fn late_template_item_up_26_2(
	v: &a::item_stack_template::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap::item(Version::V26_1, Version::V26_2, v.item_id)
}

pub fn late_template_item_down_26_1(
	v: &b::item_stack_template::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap::item(Version::V26_2, Version::V26_1, v.item_id)
}

fn surviving<T>(
	items: &[T],
	ser: impl Fn(&T, &mut crate::pds::Output) -> crate::pds::Result<()>,
) -> Result<i32> {
	let mut n = 0;
	for c in items {
		let bytes = encode(c, &ser)?;
		let mut at = 0;
		let id = read_varint(&bytes, &mut at)?;
		n += components::remap(Version::V26_2, Version::V26_1, id).is_some() as i32;
	}
	Ok(n)
}

pub fn late_added_count_down_26_1(v: &b::slot_default::Ty, _cx: &mut Session) -> Result<i32> {
	surviving(&v.components, b::slot_component::serialize)
}

pub fn late_removed_count_down_26_1(v: &b::slot_default::Ty, _cx: &mut Session) -> Result<i32> {
	surviving(&v.remove_components, b::slot_component_type::serialize)
}

pub fn late_template_added_down_26_1(
	v: &b::item_stack_template::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	surviving(&v.components, b::slot_component::serialize)
}

pub fn late_template_removed_down_26_1(
	v: &b::item_stack_template::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	surviving(&v.remove_components, b::slot_component_type::serialize)
}

macro_rules! substitutes {
	($($name:ident($($p:ident)?) => $arm:ident;)*) => {
		$(
			pub fn $name($($p: impl Sized,)? _cx: &mut Session) -> Result<a::particle::Ty> {
				Ok(a::particle::Ty::$arm)
			}
		)*
	};
}

substitutes! {
	late_particle_geyser_down_26_1(_p) => Cloud;
	late_particle_geyser_base_down_26_1(_p) => Cloud;
	late_particle_geyser_plume_down_26_1(_p) => Cloud;
	late_particle_geyser_poof_down_26_1(_p) => Cloud;
	late_particle_noxious_gas_down_26_1(_p) => Smoke;
	late_particle_noxious_gas_cloud_down_26_1(_p) => LargeSmoke;
	late_particle_sulfur_bubbles_down_26_1(_p) => Bubble;
	late_particle_sulfur_cube_goo_down_26_1(_p) => ItemSlime;
}

const CUBES: &[&str] = &["slime", "magma_cube"];

pub fn late_meta_entity_up_26_2(
	p: &a::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, Version::V26_1, p.entity_id);
	Ok(p.entity_id)
}

pub fn late_meta_entity_down_26_1(
	p: &b::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, Version::V26_2, p.entity_id);
	Ok(p.entity_id)
}

pub fn late_meta_key_up_26_2(e: &a::entity_metadata_entry::Ty, cx: &mut Session) -> Result<u8> {
	meta(cx).key = e.key;
	Ok(if e.key >= 16 && shared::is(cx, CUBES) {
		e.key + 2
	} else {
		e.key
	})
}

pub fn late_meta_key_down_26_1(e: &b::entity_metadata_entry::Ty, cx: &mut Session) -> Result<u8> {
	if let Some(key) = shared::fallback_key(cx, Version::V26_2, Version::V26_1, e.key) {
		return key;
	}
	meta(cx).key = e.key;
	if e.key < 16 || !shared::is(cx, CUBES) {
		return Ok(e.key);
	}
	match e.key {
		16 | 17 => Err(Error::Dropped),
		k => Ok(k - 2),
	}
}

fn color_up(c: i32) -> Option<i32> {
	(0..16).contains(&c).then_some(c)
}

fn color_down(c: Option<i32>) -> i32 {
	c.unwrap_or(21)
}

pub fn late_team_add_up_26_2(
	v: &a::play_to_client_packet_teams_mode_add::Ty,
	_cx: &mut Session,
) -> Result<Option<i32>> {
	Ok(color_up(v.formatting))
}

pub fn late_team_change_up_26_2(
	v: &a::play_to_client_packet_teams_mode_change::Ty,
	_cx: &mut Session,
) -> Result<Option<i32>> {
	Ok(color_up(v.formatting))
}

pub fn late_team_add_down_26_1(
	v: &b::play_to_client_packet_teams_mode_add::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(color_down(v.formatting))
}

pub fn late_team_change_down_26_1(
	v: &b::play_to_client_packet_teams_mode_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(color_down(v.formatting))
}

#[derive(Default)]
struct Login {
	encrypted: bool,
	session: Option<u128>,
}

fn random_uuid() -> u128 {
	use std::hash::BuildHasher;
	let state = std::collections::hash_map::RandomState::new();
	let bits = (state.hash_one(1u8) as u128) << 64 | state.hash_one(2u8) as u128;
	bits & !(0xf << 76 | 0x3 << 62) | 0x4 << 76 | 0x2 << 62
}

pub fn late_encryption_up_26_2(
	p: a::login_to_client_packet_encryption_begin::Ty,
	cx: &mut Session,
) -> Result<Vec<b::login_to_client_packet::Ty>> {
	cx.state::<Login>().encrypted = true;
	Ok(vec![b::login_to_client_packet::Ty::EncryptionBegin(p)])
}

pub fn late_session_id_up_26_2(
	_v: &a::login_to_client_packet_success::Ty,
	cx: &mut Session,
) -> Result<u128> {
	Ok(*cx.state::<Login>().session.get_or_insert_with(random_uuid))
}

pub fn late_online_mode_up_26_2(
	_v: &a::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<bool> {
	Ok(cx.state::<Login>().encrypted)
}

pub fn late_spectate_up_26_2(
	target: i32,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_server_packet::Ty>> {
	Ok(vec![b::play_to_server_packet::Ty::SpectatorAction(
		target + 1,
	)])
}

pub fn late_spectator_action_down_26_1(
	target: i32,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_server_packet::Ty>> {
	Ok(vec![match target {
		0 => a::play_to_server_packet::Ty::ArmAnimation(
			a::play_to_server_packet_arm_animation::Ty::MainHand,
		),
		t => a::play_to_server_packet::Ty::SpectateEntity(t - 1),
	}])
}

const REQUIRED: &[(&str, &[&str])] = &[
	("jukebox_song", &["bounce"]),
	("damage_type", &["sulfur_cube_hot"]),
];
const ADDED: &[&str] = &["sulfur_cube_archetype"];

const DERIVED: &[Derived] = &[
	("item", "sulfur_cube_archetype/bouncy", &[]),
	("item", "sulfur_cube_archetype/explosive", &[]),
	("item", "sulfur_cube_archetype/fast_flat", &[]),
	("item", "sulfur_cube_archetype/fast_sliding", &[]),
	("item", "sulfur_cube_archetype/high_resistance", &[]),
	("item", "sulfur_cube_archetype/hot", &[]),
	("item", "sulfur_cube_archetype/light", &[]),
	("item", "sulfur_cube_archetype/regular", &[]),
	("item", "sulfur_cube_archetype/slow_bouncy", &[]),
	("item", "sulfur_cube_archetype/slow_flat", &[]),
	("item", "sulfur_cube_archetype/slow_sliding", &[]),
	("item", "sulfur_cube_archetype/sticky", &[]),
];

#[derive(Default)]
struct Config {
	registries: HashSet<String>,
	tags: bool,
}

pub fn late_config_tags_up_26_2(
	v: Vec<a::configuration_to_client_packet_tags_items::Ty>,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_client_packet::Ty>> {
	cx.state::<Config>().tags = true;
	let mut v = super::tags::config(Version::V26_1, Version::V26_2, &[], v);
	super::tags::derive(Version::V26_2, &mut v, DERIVED);
	Ok(vec![b::configuration_to_client_packet::Ty::Tags(v)])
}

fn update_entity_predicate(predicate: &mut Payload) {
	if let Some(Payload::Compound(mut specific)) = predicate.take("type_specific") {
		let kind = specific
			.iter()
			.position(|e| e.name == "type")
			.map(|i| specific.remove(i).value);
		let kind = kind.as_ref().and_then(Payload::as_str).unwrap_or_default();
		match kind.strip_prefix("minecraft:").unwrap_or(kind) {
			k @ ("player" | "lightning" | "fishing_hook" | "raider" | "sheep") => {
				predicate.put(&format!("type_specific/{k}"), Payload::Compound(specific))
			}
			"slime" => predicate.put("type_specific/cube_mob", Payload::Compound(specific)),
			_ => {}
		}
	}
	if let Some(kind) = predicate.take("type") {
		predicate.put("entity_type", kind);
	}
	for nested in ["vehicle", "passenger", "targeted_entity"] {
		if let Some(p) = predicate.field_mut(nested) {
			update_entity_predicate(p);
		}
	}
}

fn downgrade_entity_predicate(predicate: &mut Payload) {
	let Payload::Compound(entries) = predicate else {
		return;
	};
	if let Some(at) = entries
		.iter()
		.position(|e| e.name.starts_with("type_specific/"))
	{
		let entry = entries.remove(at);
		let kind = entry.name.trim_start_matches("type_specific/");
		let kind = if kind == "cube_mob" { "slime" } else { kind };
		let mut specific = entry.value;
		specific.put("type", Payload::String(format!("minecraft:{kind}")));
		predicate.put("type_specific", specific);
	}
	if let Some(kind) = predicate.take("entity_type") {
		predicate.put("type", kind);
	}
	for nested in ["vehicle", "passenger", "targeted_entity"] {
		if let Some(p) = predicate.field_mut(nested) {
			downgrade_entity_predicate(p);
		}
	}
}

fn walk_terms(tag: &mut Payload, f: fn(&mut Payload)) {
	match tag {
		Payload::Compound(entries) => {
			let condition = entries
				.iter()
				.find(|e| e.name == "condition")
				.and_then(|e| e.value.as_str())
				.map(|c| c.strip_prefix("minecraft:").unwrap_or(c).to_owned());
			match condition.as_deref() {
				Some("entity_properties") => {
					if let Some(p) = tag.field_mut("predicate") {
						f(p);
					}
				}
				Some("damage_source_properties") => {
					if let Some(p) = tag.field_mut("predicate") {
						for nested in ["source_entity", "direct_entity"] {
							if let Some(e) = p.field_mut(nested) {
								f(e);
							}
						}
					}
				}
				_ => {}
			}
			if let Payload::Compound(entries) = tag {
				for e in entries {
					walk_terms(&mut e.value, f);
				}
			}
		}
		Payload::List(_) => {
			for c in tag.compounds_mut().into_iter().flatten() {
				let mut item = Payload::Compound(std::mem::take(c));
				walk_terms(&mut item, f);
				if let Payload::Compound(back) = item {
					*c = back;
				}
			}
		}
		_ => {}
	}
}

fn add_required(registry: &str, entries: &mut Vec<(String, Option<Tag>)>) -> Result<()> {
	let Some((_, keys)) = REQUIRED.iter().find(|(r, _)| *r == registry) else {
		return Ok(());
	};
	let vanilla = registries::vanilla(Version::V26_2, &format!("minecraft:{registry}"))?;
	for key in *keys {
		let full = format!("minecraft:{key}");
		if entries.iter().any(|(k, _)| *k == full || k == key) {
			continue;
		}
		if let Some(entry) = vanilla.iter().find(|(k, _)| *k == full) {
			entries.push(entry.clone());
		}
	}
	Ok(())
}

pub fn late_registry_data_up_26_2(
	mut p: a::configuration_to_client_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_client_packet::Ty>> {
	shared::count_biomes(&p, cx);
	let id = p.id.strip_prefix("minecraft:").unwrap_or(&p.id).to_owned();
	cx.state::<Config>().registries.insert(id.clone());
	shared::registry_entries(&mut p, "enchantment", |_, tag| {
		walk_terms(tag, update_entity_predicate)
	});
	let mut entries: Vec<(String, Option<Tag>)> =
		p.entries.into_iter().map(|e| (e.key, e.value)).collect();
	add_required(&id, &mut entries)?;
	Ok(vec![b::configuration_to_client_packet::Ty::RegistryData(
		shared::registry(&id, entries),
	)])
}

pub fn late_finish_configuration_up_26_2(
	p: a::configuration_to_client_packet_finish_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_client_packet::Ty>> {
	let config = std::mem::take(cx.state::<Config>());
	let mut out = Vec::new();
	for id in ADDED.iter().chain(REQUIRED.iter().map(|(r, _)| r)) {
		if config.registries.contains(*id) {
			continue;
		}
		let entries = registries::vanilla(Version::V26_2, &format!("minecraft:{id}"))?;
		out.push(b::configuration_to_client_packet::Ty::RegistryData(
			shared::registry(id, entries),
		));
	}
	if !config.tags {
		let mut tags = Vec::new();
		super::tags::derive(Version::V26_2, &mut tags, DERIVED);
		out.push(b::configuration_to_client_packet::Ty::Tags(tags));
	}
	out.push(b::configuration_to_client_packet::Ty::FinishConfiguration(
		p,
	));
	Ok(out)
}

pub fn late_registry_data_down_26_1(
	mut p: b::configuration_to_client_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<a::configuration_to_client_packet::Ty>> {
	shared::count_biomes(&p, cx);
	let id = p.id.strip_prefix("minecraft:").unwrap_or(&p.id).to_owned();
	if ADDED.contains(&id.as_str()) {
		return Ok(Vec::new());
	}
	shared::registry_entries(&mut p, "enchantment", |_, tag| {
		walk_terms(tag, downgrade_entity_predicate)
	});
	Ok(vec![a::configuration_to_client_packet::Ty::RegistryData(p)])
}
