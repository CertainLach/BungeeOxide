use super::components;
use super::registries;
use super::remap::{self, remaps};
use super::rules::{blocks, entities, items, statistics, tags};
use super::shared;
use super::slots::{self, exact_matchers, untrusted_step};
use super::tags::Derived;
use super::util::{decode, encode, read_varint, recode};
use crate::custom::chunks::bits::{bitset_bytes, bitset_longs};
use crate::gen::convert::{v26_2 as a, v26_3 as b, Version};
use crate::pds::convert::{Error, Result};
use crate::pds_native::pds_entityDelta::{Delta, Step, Ty as EntityDelta};
use crate::Session;
use bungeeoxide_nbt::{Payload, Tag};
use std::collections::HashSet;

remaps!(Version::V26_2 => Version::V26_3 { late_sound_up_26_3: a::item_sound_holder::Ty = remap::sound; });
remaps!(Version::V26_3 => Version::V26_2 { late_sound_down_26_2: a::item_sound_holder::Ty = remap::sound; });

items!(a => b, Version::V26_2 => Version::V26_3 {
	slot: late_slot_item_up_26_3, hashed: late_hashed_item_up_26_3, trade1: late_trade1_up_26_3, trade2: late_trade2_up_26_3,
	display: late_display_item_up_26_3, recipes: late_recipe_items_up_26_3, stonecutter: late_stonecutter_up_26_3,
	requirements: late_requirements_up_26_3, repairable: late_repairable_up_26_3,
});
items!(b => a, Version::V26_3 => Version::V26_2 {
	slot: late_slot_item_down_26_2, hashed: late_hashed_item_down_26_2, trade1: late_trade1_down_26_2, trade2: late_trade2_down_26_2,
	display: late_display_item_down_26_2, recipes: late_recipe_items_down_26_2, stonecutter: late_stonecutter_down_26_2,
	requirements: late_requirements_down_26_2, repairable: late_repairable_down_26_2,
});

blocks!(a => b, Version::V26_2 => Version::V26_3 {
	block_change: late_block_change_up_26_3, multi_block: late_multi_block_up_26_3, block_action: late_block_action_up_26_3,
	world_event: late_world_event_data_up_26_3, chunk: late_chunk_up_26_3, tool: late_tool_up_26_3, predicate: late_predicate_up_26_3,
	particle: [late_particle_block_up_26_3, late_particle_marker_up_26_3, late_particle_dust_up_26_3, late_particle_pillar_up_26_3, late_particle_crumble_up_26_3],
	meta: [late_meta_state_up_26_3, late_meta_optional_state_up_26_3],
});
blocks!(b => a, Version::V26_3 => Version::V26_2 {
	block_change: late_block_change_down_26_2, multi_block: late_multi_block_down_26_2, block_action: late_block_action_down_26_2,
	world_event: late_world_event_data_down_26_2, chunk: late_chunk_down_26_2, tool: late_tool_down_26_2, predicate: late_predicate_down_26_2,
	particle: [late_particle_block_down_26_2, late_particle_marker_down_26_2, late_particle_dust_down_26_2, late_particle_pillar_down_26_2, late_particle_crumble_down_26_2],
	meta: [late_meta_state_down_26_2, late_meta_optional_state_down_26_2],
});

entities!(a => b, Version::V26_2 => Version::V26_3 { spawn: late_spawn_type_up_26_3, object: late_object_data_up_26_3, equippable: late_equip_entities_up_26_3, });
entities!(b => a, Version::V26_3 => Version::V26_2 { spawn: late_spawn_type_down_26_2, object: late_object_data_down_26_2, equippable: late_equip_entities_down_26_2, });

statistics!(a => b, Version::V26_2 => Version::V26_3 { late_statistics_up_26_3 });
statistics!(b => a, Version::V26_3 => Version::V26_2 { late_statistics_down_26_2 });

const TAG_RENAMES_UP: &[(&str, &str, &str)] =
	&[("block", "convertable_to_mud", "convertible_to_mud")];
const TAG_RENAMES_DOWN: &[(&str, &str, &str)] =
	&[("block", "convertible_to_mud", "convertable_to_mud")];

tags!(a => b, Version::V26_2 => Version::V26_3, TAG_RENAMES_UP, { config: config_tags_up_26_3, play: late_play_tags_up_26_3 });
tags!(b => a, Version::V26_3 => Version::V26_2, TAG_RENAMES_DOWN, { config: late_config_tags_down_26_2, play: late_play_tags_down_26_2 });

untrusted_step!(late_untrusted_up_26_3: a => b, Version::V26_2 => Version::V26_3, slots::late_to_slot_26_2, slots::late_to_untrusted_26_3);
untrusted_step!(late_untrusted_down_26_2: b => a, Version::V26_3 => Version::V26_2, slots::late_to_slot_26_3, slots::late_to_untrusted_26_2);

exact_matchers!(exact_up: a => b, Version::V26_2 => Version::V26_3);
exact_matchers!(exact_down: b => a, Version::V26_3 => Version::V26_2);

pub fn late_matchers_up_26_3(
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

pub fn late_matchers_down_26_2(
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

fn component(from: Version, to: Version, id: i32) -> Option<i32> {
	let name = match components::name(from, id)? {
		"swing_animation" if to == Version::V26_3 => "attack_animation",
		"attack_animation" if to == Version::V26_2 => "swing_animation",
		name => name,
	};
	components::id(to, name)
}

fn surviving<T>(
	from: Version,
	to: Version,
	items: &[T],
	ser: impl Fn(&T, &mut crate::pds::Output) -> crate::pds::Result<()>,
) -> Result<i32> {
	let mut n = 0;
	for c in items {
		let bytes = encode(c, &ser)?;
		let mut at = 0;
		n += component(from, to, read_varint(&bytes, &mut at)?).is_some() as i32;
	}
	Ok(n)
}

macro_rules! counts {
	($from:path => $to:path { $($name:ident: $ty:ty, $field:ident, $ser:path;)* }) => {
		$(
			pub fn $name(v: &$ty, _cx: &mut Session) -> Result<i32> {
				surviving($from, $to, &v.$field, $ser)
			}
		)*
	};
}

counts!(Version::V26_2 => Version::V26_3 {
	late_added_count_up_26_3: a::slot_default::Ty, components, a::slot_component::serialize;
	late_removed_count_up_26_3: a::slot_default::Ty, remove_components, a::slot_component_type::serialize;
	late_template_added_up_26_3: a::item_stack_template::Ty, components, a::slot_component::serialize;
	late_template_removed_up_26_3: a::item_stack_template::Ty, remove_components, a::slot_component_type::serialize;
});
counts!(Version::V26_3 => Version::V26_2 {
	late_added_count_down_26_2: b::slot_default::Ty, components, b::slot_component::serialize;
	late_removed_count_down_26_2: b::slot_default::Ty, remove_components, b::slot_component_type::serialize;
	late_template_added_down_26_2: b::item_stack_template::Ty, components, b::slot_component::serialize;
	late_template_removed_down_26_2: b::item_stack_template::Ty, remove_components, b::slot_component_type::serialize;
});

pub fn late_hidden_up_26_3(
	v: &a::slot_component_tooltip_display::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(v.hidden_components
		.iter()
		.filter_map(|id| component(Version::V26_2, Version::V26_3, *id))
		.collect())
}

pub fn late_hidden_down_26_2(
	v: &b::slot_component_tooltip_display::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(v.hidden_components
		.iter()
		.filter_map(|id| component(Version::V26_3, Version::V26_2, *id))
		.collect())
}

pub fn late_template_item_up_26_3(
	v: &a::item_stack_template::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap::item(Version::V26_2, Version::V26_3, v.item_id)
}

pub fn late_template_item_down_26_2(
	v: &b::item_stack_template::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	remap::item(Version::V26_3, Version::V26_2, v.item_id)
}

pub fn late_swing_animation_up_26_3(
	v: a::slot_component_swing_animation::Ty,
	_cx: &mut Session,
) -> Result<b::slot_component::Ty> {
	Ok(b::slot_component::Ty::AttackAnimation(recode(
		&v,
		a::slot_component_swing_animation::serialize,
		b::slot_component_attack_animation::deserialize,
	)?))
}

pub fn late_attack_animation_down_26_2(
	v: b::slot_component_attack_animation::Ty,
	_cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	Ok(a::slot_component::Ty::SwingAnimation(recode(
		&v,
		b::slot_component_attack_animation::serialize,
		a::slot_component_swing_animation::deserialize,
	)?))
}

pub fn late_swing_animation_type_up_26_3(
	_v: (),
	_cx: &mut Session,
) -> Result<b::slot_component_type::Ty> {
	Ok(b::slot_component_type::Ty::AttackAnimation)
}

pub fn late_attack_animation_type_down_26_2(
	_v: (),
	_cx: &mut Session,
) -> Result<a::slot_component_type::Ty> {
	Ok(a::slot_component_type::Ty::SwingAnimation)
}

fn pot(id: i32) -> Option<b::item_stack_template::Ty> {
	let item = remap::item(Version::V26_2, Version::V26_3, id).ok()?;
	Some(b::item_stack_template::Ty {
		item_id: item,
		item_count: 1,
		added_component_count: 0,
		removed_component_count: 0,
		components: Vec::new(),
		remove_components: Vec::new(),
	})
}

pub fn late_pots_up_26_3(v: Vec<i32>, _cx: &mut Session) -> Result<b::slot_component::Ty> {
	let side = |i: usize| v.get(i).copied().and_then(pot);
	Ok(b::slot_component::Ty::PotDecorations(
		b::slot_component_pot_decorations::Ty {
			back: side(0),
			left: side(1),
			right: side(2),
			front: side(3),
		},
	))
}

pub fn late_pots_down_26_2(
	v: b::slot_component_pot_decorations::Ty,
	_cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	let brick = crate::registry::names(Version::V26_2, "items")
		.and_then(|n| n.id("brick"))
		.unwrap_or(0) as i32;
	let side = |t: Option<b::item_stack_template::Ty>| -> Result<i32> {
		t.map_or(Ok(brick), |t| {
			remap::item(Version::V26_3, Version::V26_2, t.item_id)
		})
	};
	Ok(a::slot_component::Ty::PotDecorations(vec![
		side(v.back)?,
		side(v.left)?,
		side(v.right)?,
		side(v.front)?,
	]))
}

fn tinted(color: u32) -> Result<a::particle::Ty> {
	Ok(a::particle::Ty::TintedLeaves(color as i32))
}

pub fn late_particle_red_poplar_down_26_2(_v: (), _cx: &mut Session) -> Result<a::particle::Ty> {
	tinted(0xffb8_3a26)
}

pub fn late_particle_orange_poplar_down_26_2(_v: (), _cx: &mut Session) -> Result<a::particle::Ty> {
	tinted(0xffd9_7a1e)
}

pub fn late_particle_yellow_poplar_down_26_2(_v: (), _cx: &mut Session) -> Result<a::particle::Ty> {
	tinted(0xffe8_c21f)
}

pub fn late_trim_palette_up_26_3(
	v: &a::armor_trim_material::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(match v.asset_base.split_once(':') {
		Some((ns, path)) => format!("{ns}:trim/{path}"),
		None => format!("minecraft:trim/{}", v.asset_base),
	})
}

pub fn late_trim_asset_down_26_2(
	v: &b::armor_trim_material::Ty,
	_cx: &mut Session,
) -> Result<String> {
	let path = v
		.palette_id
		.split_once(':')
		.map_or(v.palette_id.as_str(), |(_, p)| p);
	Ok(path.strip_prefix("trim/").unwrap_or(path).to_owned())
}

pub fn late_trim_overrides_down_26_2(
	_v: &b::armor_trim_material::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::armor_trim_material_override_armor_assets_items::Ty>> {
	Ok(Vec::new())
}

pub fn late_teleport_effect_up_26_3(
	diameter: f32,
	_cx: &mut Session,
) -> Result<b::item_consume_effect::Ty> {
	Ok(b::item_consume_effect::Ty::TeleportRandomly(
		b::item_consume_effect_teleport_randomly::Ty {
			diameter,
			directional_particles: false,
		},
	))
}

pub fn late_teleport_effect_down_26_2(
	v: b::item_consume_effect_teleport_randomly::Ty,
	_cx: &mut Session,
) -> Result<a::item_consume_effect::Ty> {
	Ok(a::item_consume_effect::Ty::TeleportRandomly(v.diameter))
}

pub fn late_display_tag_up_26_3(
	tag: String,
	_cx: &mut Session,
) -> Result<b::play_to_client_slot_display::Ty> {
	Ok(b::play_to_client_slot_display::Ty::Tag(
		b::id_set::Ty::Named(tag.trim_start_matches('#').to_owned()),
	))
}

pub fn late_display_tag_down_26_2(
	v: b::id_set::Ty,
	_cx: &mut Session,
) -> Result<a::play_to_client_slot_display::Ty> {
	Ok(match v {
		b::id_set::Ty::Named(name) => a::play_to_client_slot_display::Ty::Tag(name),
		b::id_set::Ty::Ids(ids) => a::play_to_client_slot_display::Ty::Composite(
			remap::items(Version::V26_3, Version::V26_2, ids)?
				.into_iter()
				.map(a::play_to_client_slot_display::Ty::Item)
				.collect(),
		),
	})
}

pub fn late_previous_gamemode_up_26_3(
	v: &a::play_to_client_spawn_info::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(match v.previous_gamemode {
		mode @ 0..=3 => mode as i32 + 1,
		_ => 0,
	})
}

pub fn late_previous_gamemode_down_26_2(
	v: &b::play_to_client_spawn_info::Ty,
	_cx: &mut Session,
) -> Result<u8> {
	Ok(match v.previous_gamemode {
		mode @ 1..=4 => (mode - 1) as u8,
		_ => 255,
	})
}

#[derive(Default)]
struct Placement {
	x: f32,
	y: f32,
}

pub fn late_advancement_x_up_26_3(
	v: &a::play_to_client_packet_advancements_advancement_mapping_items::Ty,
	_cx: &mut Session,
) -> Result<f32> {
	Ok(v.value.display_data.as_ref().map_or(0.0, |d| d.x_cord))
}

pub fn late_advancement_y_up_26_3(
	v: &a::play_to_client_packet_advancements_advancement_mapping_items::Ty,
	_cx: &mut Session,
) -> Result<f32> {
	Ok(v.value.display_data.as_ref().map_or(0.0, |d| d.y_cord))
}

pub fn late_advancement_key_down_26_2(
	v: &b::play_to_client_packet_advancements_advancement_mapping_items::Ty,
	cx: &mut Session,
) -> Result<String> {
	*cx.state::<Placement>() = Placement { x: v.x, y: v.y };
	Ok(v.key.clone())
}

pub fn late_advancement_x_down_26_2(
	_v: &b::play_to_client_packet_advancements_advancement_mapping_items_value_display_data_items::Ty,
	cx: &mut Session,
) -> Result<f32> {
	Ok(cx.state::<Placement>().x)
}

pub fn late_advancement_y_down_26_2(
	_v: &b::play_to_client_packet_advancements_advancement_mapping_items_value_display_data_items::Ty,
	cx: &mut Session,
) -> Result<f32> {
	Ok(cx.state::<Placement>().y)
}

pub fn late_mask_up_26_3(v: Vec<i64>, _cx: &mut Session) -> Result<Vec<u8>> {
	Ok(bitset_bytes(&v))
}

pub fn late_mask_down_26_2(v: Vec<u8>, _cx: &mut Session) -> Result<Vec<i64>> {
	Ok(bitset_longs(&v))
}

pub fn late_randomization_up_26_3(
	_v: &a::play_to_client_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<b::play_to_client_packet_world_particles_randomization_type::Ty> {
	Ok(b::play_to_client_packet_world_particles_randomization_type::Ty::default)
}

const BREWING: &[(&str, &[&str])] = &[
	(
		"minecraft:brewing_input",
		&[
			"potion",
			"splash_potion",
			"lingering_potion",
			"glass_bottle",
		],
	),
	(
		"minecraft:brewing_reagent",
		&[
			"blaze_powder",
			"breeze_rod",
			"cobweb",
			"dragon_breath",
			"fermented_spider_eye",
			"ghast_tear",
			"glistering_melon_slice",
			"glowstone_dust",
			"golden_carrot",
			"gunpowder",
			"magma_cream",
			"nether_wart",
			"phantom_membrane",
			"pufferfish",
			"rabbit_foot",
			"redstone",
			"slime_block",
			"spider_eye",
			"stone",
			"sugar",
			"turtle_helmet",
		],
	),
];

pub fn late_recipe_sets_up_26_3(
	v: &a::play_to_client_packet_declare_recipes::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet_declare_recipes_recipes_items::Ty>> {
	let mut out: Vec<_> = v
		.recipes
		.iter()
		.map(|r| {
			Ok(b::play_to_client_packet_declare_recipes_recipes_items::Ty {
				name: r.name.clone(),
				items: remap::items(Version::V26_2, Version::V26_3, r.items.clone())?,
			})
		})
		.collect::<Result<_>>()?;
	let items = crate::registry::names(Version::V26_3, "items");
	for (name, list) in BREWING {
		if out.iter().any(|r| r.name == *name) {
			continue;
		}
		out.push(b::play_to_client_packet_declare_recipes_recipes_items::Ty {
			name: (*name).to_owned(),
			items: list
				.iter()
				.filter_map(|i| items?.id(i).map(|id| id as i32))
				.collect(),
		});
	}
	Ok(out)
}

pub fn late_recipe_sets_down_26_2(
	v: &b::play_to_client_packet_declare_recipes::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet_declare_recipes_recipes_items::Ty>> {
	v.recipes
		.iter()
		.filter(|r| !BREWING.iter().any(|(n, _)| *n == r.name))
		.map(|r| {
			Ok(a::play_to_client_packet_declare_recipes_recipes_items::Ty {
				name: r.name.clone(),
				items: remap::items(Version::V26_3, Version::V26_2, r.items.clone())?,
			})
		})
		.collect()
}

const BREWING_STAND_MENU: i32 = 11;
const TOTAL_BREW_TIME: i16 = 2;
const TOTAL_FUEL: i16 = 3;

#[derive(Default)]
struct BrewingStand {
	window: Option<i32>,
}

pub fn late_open_window_up_26_3(
	v: a::play_to_client_packet_open_window::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	let total = |property, value| {
		b::play_to_client_packet::Ty::CraftProgressBar(b::play_to_client_packet_craft_progress_bar::Ty {
			window_id: v.window_id,
			property,
			value,
		})
	};
	let mut out = Vec::new();
	if v.inventory_type == BREWING_STAND_MENU {
		out.push(total(TOTAL_BREW_TIME, 400));
		out.push(total(TOTAL_FUEL, 20));
	}
	out.insert(0, b::play_to_client_packet::Ty::OpenWindow(v));
	Ok(out)
}

pub fn late_open_window_down_26_2(
	v: b::play_to_client_packet_open_window::Ty,
	cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	cx.state::<BrewingStand>().window =
		(v.inventory_type == BREWING_STAND_MENU).then_some(v.window_id);
	Ok(vec![a::play_to_client_packet::Ty::OpenWindow(v)])
}

pub fn late_progress_bar_down_26_2(
	v: b::play_to_client_packet_craft_progress_bar::Ty,
	cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	if v.property >= TOTAL_BREW_TIME && cx.state::<BrewingStand>().window == Some(v.window_id) {
		return Ok(Vec::new());
	}
	Ok(vec![a::play_to_client_packet::Ty::CraftProgressBar(v)])
}

const MAIN_HAND: i32 = 0;
const WHACK: i32 = 1;
const SWING_TICKS: i32 = 6;

pub fn late_animation_up_26_3(
	v: a::play_to_client_packet_animation::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	let animate = |animation| {
		vec![b::play_to_client_packet::Ty::Animation(
			b::play_to_client_packet_animation::Ty {
				entity_id: v.entity_id,
				animation,
			},
		)]
	};
	Ok(match v.animation {
		hand @ (0 | 3) => vec![b::play_to_client_packet::Ty::SwingAnimation(
			b::play_to_client_packet_swing_animation::Ty {
				entity_id: v.entity_id,
				hand: if hand == 0 { MAIN_HAND } else { 1 },
				animation_type: WHACK,
				duration: SWING_TICKS,
			},
		)],
		2 => animate(0),
		4 => animate(1),
		5 => animate(2),
		_ => Vec::new(),
	})
}

pub fn late_animation_down_26_2(
	v: b::play_to_client_packet_animation::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	Ok(vec![a::play_to_client_packet::Ty::Animation(
		a::play_to_client_packet_animation::Ty {
			entity_id: v.entity_id,
			animation: match v.animation {
				0 => 2,
				1 => 4,
				2 => 5,
				other => other,
			},
		},
	)])
}

pub fn late_swing_down_26_2(
	v: b::play_to_client_packet_swing_animation::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	if v.animation_type == 0 {
		return Ok(Vec::new());
	}
	Ok(vec![a::play_to_client_packet::Ty::Animation(
		a::play_to_client_packet_animation::Ty {
			entity_id: v.entity_id,
			animation: if v.hand == MAIN_HAND { 0 } else { 3 },
		},
	)])
}

const STEP_TICKS: i32 = 3;

fn delta_up(dx: i16, dy: i16, dz: i16, on_ground: bool) -> EntityDelta {
	EntityDelta {
		onGround: on_ground,
		delta: Delta::Stepped(vec![Step {
			ticks: STEP_TICKS,
			dX: dx,
			dY: dy,
			dZ: dz,
		}]),
	}
}

fn deltas_down(delta: &EntityDelta) -> Vec<(i16, i16, i16)> {
	let steps: Vec<(i16, i16, i16)> = match &delta.delta {
		Delta::Linear { dX, dY, dZ } => vec![(*dX, *dY, *dZ)],
		Delta::Stepped(steps) => steps.iter().map(|s| (s.dX, s.dY, s.dZ)).collect(),
	};
	let fits = |v: i32| (i16::MIN as i32..=i16::MAX as i32).contains(&v);
	let mut out = Vec::new();
	let mut sum = (0i32, 0i32, 0i32);
	for (x, y, z) in steps {
		let next = (sum.0 + x as i32, sum.1 + y as i32, sum.2 + z as i32);
		if fits(next.0) && fits(next.1) && fits(next.2) {
			sum = next;
		} else {
			out.push((sum.0 as i16, sum.1 as i16, sum.2 as i16));
			sum = (x as i32, y as i32, z as i32);
		}
	}
	out.push((sum.0 as i16, sum.1 as i16, sum.2 as i16));
	out
}

fn rel_moves_down(
	entity_id: i32,
	deltas: &[(i16, i16, i16)],
	on_ground: bool,
) -> Vec<a::play_to_client_packet::Ty> {
	deltas
		.iter()
		.map(|(x, y, z)| {
			a::play_to_client_packet::Ty::RelEntityMove(
				a::play_to_client_packet_rel_entity_move::Ty {
					entity_id,
					d_x: *x,
					d_y: *y,
					d_z: *z,
					on_ground,
				},
			)
		})
		.collect()
}

pub fn late_rel_move_up_26_3(
	v: a::play_to_client_packet_rel_entity_move::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	Ok(vec![b::play_to_client_packet::Ty::RelEntityMove(
		b::play_to_client_packet_rel_entity_move::Ty {
			entity_id: v.entity_id,
			r#move: delta_up(v.d_x, v.d_y, v.d_z, v.on_ground),
		},
	)])
}

pub fn late_rel_move_down_26_2(
	v: b::play_to_client_packet_rel_entity_move::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	Ok(rel_moves_down(
		v.entity_id,
		&deltas_down(&v.r#move),
		v.r#move.onGround,
	))
}

pub fn late_move_look_up_26_3(
	v: a::play_to_client_packet_entity_move_look::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	Ok(vec![b::play_to_client_packet::Ty::EntityMoveLook(
		b::play_to_client_packet_entity_move_look::Ty {
			entity_id: v.entity_id,
			r#move: delta_up(v.d_x, v.d_y, v.d_z, v.on_ground),
			yaw: v.yaw,
			pitch: v.pitch,
		},
	)])
}

pub fn late_move_look_down_26_2(
	v: b::play_to_client_packet_entity_move_look::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	let deltas = deltas_down(&v.r#move);
	let on_ground = v.r#move.onGround;
	let (x, y, z) = deltas[0];
	let mut out = vec![a::play_to_client_packet::Ty::EntityMoveLook(
		a::play_to_client_packet_entity_move_look::Ty {
			entity_id: v.entity_id,
			d_x: x,
			d_y: y,
			d_z: z,
			yaw: v.yaw,
			pitch: v.pitch,
			on_ground,
		},
	)];
	out.extend(rel_moves_down(v.entity_id, &deltas[1..], on_ground));
	Ok(out)
}

pub fn late_sync_position_up_26_3(
	v: a::play_to_client_packet_sync_entity_position::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	Ok(vec![b::play_to_client_packet::Ty::SyncEntityPosition(
		b::play_to_client_packet_sync_entity_position::Ty {
			entity_id: v.entity_id,
			anon: b::play_to_client_packet_sync_entity_position_anon::Ty::Stepped(vec![
				b::play_to_client_packet_sync_entity_position_anon_stepped_items::Ty {
					x: v.x,
					y: v.y,
					z: v.z,
					tick_offset: STEP_TICKS,
				},
			]),
			yaw: v.yaw,
			pitch: v.pitch,
			on_ground: v.on_ground,
		},
	)])
}

pub fn late_sync_position_down_26_2(
	v: b::play_to_client_packet_sync_entity_position::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	use b::play_to_client_packet_sync_entity_position_anon::Ty as Path;
	let ((x, y, z), (dx, dy, dz)) = match &v.anon {
		Path::Linear(p) => ((p.x, p.y, p.z), (0.0, 0.0, 0.0)),
		Path::Stepped(steps) => match steps.as_slice() {
			[] => return Ok(Vec::new()),
			[.., prev, last] if last.tick_offset > 0 => {
				let t = last.tick_offset as f64;
				(
					(last.x, last.y, last.z),
					(
						(last.x - prev.x) / t,
						(last.y - prev.y) / t,
						(last.z - prev.z) / t,
					),
				)
			}
			[.., last] => ((last.x, last.y, last.z), (0.0, 0.0, 0.0)),
		},
		Path::Default(_) => return Ok(Vec::new()),
	};
	Ok(vec![a::play_to_client_packet::Ty::SyncEntityPosition(
		a::play_to_client_packet_sync_entity_position::Ty {
			entity_id: v.entity_id,
			x,
			y,
			z,
			dx,
			dy,
			dz,
			yaw: v.yaw,
			pitch: v.pitch,
			on_ground: v.on_ground,
		},
	)])
}

const SPLASH_SOUNDS: &[(i32, i32)] = &[(2002, 1053), (2007, 1054)];

pub fn late_world_event_up_26_3(
	mut v: a::play_to_client_packet_world_event::Ty,
	cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	v.data = late_world_event_data_up_26_3(&v, cx)?;
	let sound = SPLASH_SOUNDS
		.iter()
		.find(|(e, _)| *e == v.effect_id)
		.map(|(_, s)| {
			b::play_to_client_packet::Ty::WorldEvent(b::play_to_client_packet_world_event::Ty {
				effect_id: *s,
				location: v.location.clone(),
				data: 0,
				global: false,
			})
		});
	let mut out = vec![b::play_to_client_packet::Ty::WorldEvent(v)];
	out.extend(sound);
	Ok(out)
}

pub fn late_world_event_down_26_2(
	mut v: b::play_to_client_packet_world_event::Ty,
	cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	if SPLASH_SOUNDS.iter().any(|(_, s)| *s == v.effect_id) {
		return Ok(Vec::new());
	}
	v.data = late_world_event_data_down_26_2(&v, cx)?;
	Ok(vec![a::play_to_client_packet::Ty::WorldEvent(v)])
}

pub fn late_dig_status_up_26_3(
	v: &a::play_to_server_packet_block_dig::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(if v.status >= 1 {
		v.status + 1
	} else {
		v.status
	})
}

pub fn late_dig_status_down_26_2(
	v: &b::play_to_server_packet_block_dig::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	match v.status {
		1 => Err(Error::Dropped),
		s if s > 1 => Ok(s - 1),
		s => Ok(s),
	}
}

pub fn late_arm_animation_up_26_3(
	v: a::play_to_server_packet_arm_animation::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_server_packet::Ty>> {
	Ok(match v {
		a::play_to_server_packet_arm_animation::Ty::MainHand => {
			vec![b::play_to_server_packet::Ty::Punch]
		}
		_ => Vec::new(),
	})
}

pub fn late_punch_down_26_2(
	_v: (),
	_cx: &mut Session,
) -> Result<Vec<a::play_to_server_packet::Ty>> {
	Ok(vec![a::play_to_server_packet::Ty::ArmAnimation(
		a::play_to_server_packet_arm_animation::Ty::MainHand,
	)])
}

#[derive(Default)]
struct Teleport {
	pending: Option<i32>,
	moved: bool,
}

fn ticked(
	cx: &mut Session,
	packet: b::play_to_server_packet::Ty,
) -> Vec<b::play_to_server_packet::Ty> {
	let state = cx.state::<Teleport>();
	let extra = std::mem::replace(&mut state.moved, true);
	let mut out = Vec::new();
	if extra {
		out.push(b::play_to_server_packet::Ty::TickEnd(
			b::play_to_server_packet_tick_end::Ty {},
		));
	}
	out.push(packet);
	out
}

pub fn late_tick_end_up_26_3(
	v: a::play_to_server_packet_tick_end::Ty,
	cx: &mut Session,
) -> Result<Vec<b::play_to_server_packet::Ty>> {
	cx.state::<Teleport>().moved = false;
	Ok(vec![b::play_to_server_packet::Ty::TickEnd(v)])
}

pub fn late_position_up_26_3(
	v: a::play_to_server_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<b::play_to_server_packet::Ty>> {
	Ok(ticked(cx, b::play_to_server_packet::Ty::Position(v)))
}

pub fn late_teleport_confirm_up_26_3(
	id: i32,
	cx: &mut Session,
) -> Result<Vec<b::play_to_server_packet::Ty>> {
	cx.state::<Teleport>().pending = Some(id);
	Ok(Vec::new())
}

pub fn late_position_look_up_26_3(
	v: a::play_to_server_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<Vec<b::play_to_server_packet::Ty>> {
	match cx.state::<Teleport>().pending.take() {
		Some(teleport_id) => Ok(vec![b::play_to_server_packet::Ty::TeleportConfirm(
			b::play_to_server_packet_teleport_confirm::Ty {
				teleport_id,
				x: v.x,
				y: v.y,
				z: v.z,
				y_rot: v.yaw,
				x_rot: v.pitch,
			},
		)]),
		None => Ok(ticked(cx, b::play_to_server_packet::Ty::PositionLook(v))),
	}
}

pub fn late_teleport_confirm_down_26_2(
	v: b::play_to_server_packet_teleport_confirm::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_server_packet::Ty>> {
	Ok(vec![
		a::play_to_server_packet::Ty::TeleportConfirm(v.teleport_id),
		a::play_to_server_packet::Ty::PositionLook(a::play_to_server_packet_position_look::Ty {
			x: v.x,
			y: v.y,
			z: v.z,
			yaw: v.y_rot,
			pitch: v.x_rot,
			flags: decode(&[0], a::play_to_server_movement_flags::deserialize)?,
		}),
	])
}

const ADDED: &[&str] = &[
	"decorated_pot_pattern",
	"block_transformer",
	"worldgen/block_state_provider",
];

const CONDITIONS: &[&str] = &[
	"all_of",
	"any_of",
	"block_state_property",
	"damage_source_properties",
	"enchantment_active_check",
	"entity_properties",
	"entity_scores",
	"inverted",
	"killed_by_player",
	"location_check",
	"match_block",
	"match_tool",
	"int_value_check",
	"random_chance",
	"random_chance_with_enchanted_bonus",
	"reference",
	"survives_explosion",
	"table_bonus",
	"time_check",
	"value_check",
	"weather_check",
];

fn short(s: &str) -> &str {
	s.strip_prefix("minecraft:").unwrap_or(s)
}

fn rename(tag: &mut Payload, from: &str, to: &str) {
	if let Some(v) = tag.take(from) {
		tag.put(to, v);
	}
}

fn term_up(tag: &mut Payload) {
	let Some(condition) = tag.take("condition") else {
		return;
	};
	let kind = condition
		.as_str()
		.map(|c| short(c).to_owned())
		.unwrap_or_default();
	match kind.as_str() {
		"value_check" => {
			tag.put("type", Payload::from("minecraft:int_value_check"));
			rename(tag, "range", "test");
		}
		"block_state_property" => {
			tag.put("type", Payload::from("minecraft:match_block"));
			rename(tag, "block", "blocks");
			rename(tag, "properties", "state");
		}
		_ => tag.put("type", condition),
	}
	if kind == "damage_source_properties" {
		if let Some(tags) = tag.field_mut("predicate").and_then(|p| p.field_mut("tags")) {
			for t in tags.compounds_mut().into_iter().flatten() {
				for e in t.iter_mut().filter(|e| e.name == "id") {
					if let Payload::String(s) = &mut e.value {
						if !s.starts_with('#') {
							s.insert(0, '#');
						}
					}
				}
			}
		}
	}
}

fn term_down(tag: &mut Payload) {
	let kind = match tag.field("type").and_then(Payload::as_str) {
		Some(t) if CONDITIONS.contains(&short(t)) => short(t).to_owned(),
		_ => return,
	};
	let Some(condition) = tag.take("type") else {
		return;
	};
	match kind.as_str() {
		"int_value_check" => {
			tag.put("condition", Payload::from("minecraft:value_check"));
			rename(tag, "test", "range");
		}
		"match_block" => {
			tag.put("condition", Payload::from("minecraft:block_state_property"));
			rename(tag, "blocks", "block");
			rename(tag, "state", "properties");
		}
		_ => tag.put("condition", condition),
	}
	if kind == "damage_source_properties" {
		if let Some(tags) = tag.field_mut("predicate").and_then(|p| p.field_mut("tags")) {
			for t in tags.compounds_mut().into_iter().flatten() {
				for e in t.iter_mut().filter(|e| e.name == "id") {
					if let Payload::String(s) = &mut e.value {
						*s = s.trim_start_matches('#').to_owned();
					}
				}
			}
		}
	}
}

fn each_compound(tag: &mut Payload, f: fn(&mut Payload)) {
	match tag {
		Payload::Compound(_) => {
			f(tag);
			if let Payload::Compound(entries) = tag {
				for e in entries {
					each_compound(&mut e.value, f);
				}
			}
		}
		Payload::List(_) => {
			for c in tag.compounds_mut().into_iter().flatten() {
				let mut item = Payload::Compound(std::mem::take(c));
				each_compound(&mut item, f);
				if let Payload::Compound(back) = item {
					*c = back;
				}
			}
		}
		_ => {}
	}
}

fn enchantments(
	version: Version,
	entries: &mut [(String, Option<Tag>)],
	convert: fn(&mut Payload),
) -> Result<()> {
	let vanilla = registries::vanilla(version, "minecraft:enchantment")?;
	for (key, value) in entries.iter_mut() {
		let full = if key.contains(':') {
			key.clone()
		} else {
			format!("minecraft:{key}")
		};
		match vanilla.iter().find(|(k, _)| *k == full) {
			Some((_, data)) if value.is_some() => *value = data.clone(),
			_ => {
				if let Some(tag) = value {
					each_compound(&mut tag.value_mut(), convert);
				}
			}
		}
	}
	Ok(())
}

const LIGHTNING_RODS: &[&str] = &[
	"minecraft:lightning_rod",
	"minecraft:exposed_lightning_rod",
	"minecraft:weathered_lightning_rod",
	"minecraft:oxidized_lightning_rod",
	"minecraft:waxed_lightning_rod",
	"minecraft:waxed_exposed_lightning_rod",
	"minecraft:waxed_weathered_lightning_rod",
	"minecraft:waxed_oxidized_lightning_rod",
];

const DERIVED: &[Derived] = &[
	(
		"item",
		"enchantable/melee_weapon",
		&["#minecraft:swords", "#minecraft:spears"],
	),
	("item", "enchantable/sweeping", &["#minecraft:swords"]),
	("block", "lightning_rods", LIGHTNING_RODS),
	("item", "lightning_rods", LIGHTNING_RODS),
];

#[derive(Default)]
struct Config {
	registries: HashSet<String>,
	tags: bool,
}

pub fn late_config_tags_up_26_3(
	v: Vec<a::configuration_to_client_packet_tags_items::Ty>,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_client_packet::Ty>> {
	cx.state::<Config>().tags = true;
	let mut v = super::tags::config(Version::V26_2, Version::V26_3, TAG_RENAMES_UP, v);
	super::tags::derive(Version::V26_3, &mut v, DERIVED);
	Ok(vec![b::configuration_to_client_packet::Ty::Tags(v)])
}

pub fn late_registry_data_up_26_3(
	mut p: a::configuration_to_client_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_client_packet::Ty>> {
	shared::count_biomes(&p, cx);
	let id = short(&p.id).to_owned();
	cx.state::<Config>().registries.insert(id.clone());
	shared::registry_entries(&mut p, "trim_material", |_, tag| {
		if let Some(Payload::String(asset)) = tag.take("asset_name") {
			let palette = match asset.split_once(':') {
				Some((ns, path)) => format!("{ns}:trim/{path}"),
				None => format!("minecraft:trim/{asset}"),
			};
			tag.put("palette_id", Payload::String(palette));
			tag.take("override_armor_assets");
		}
	});
	let mut entries: Vec<(String, Option<Tag>)> =
		p.entries.into_iter().map(|e| (e.key, e.value)).collect();
	if id == "enchantment" {
		enchantments(Version::V26_3, &mut entries, term_up)?;
	}
	Ok(vec![b::configuration_to_client_packet::Ty::RegistryData(
		shared::registry(&id, entries),
	)])
}

pub fn late_finish_configuration_up_26_3(
	p: a::configuration_to_client_packet_finish_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_client_packet::Ty>> {
	let config = std::mem::take(cx.state::<Config>());
	let mut out = Vec::new();
	for id in ADDED {
		if config.registries.contains(*id) {
			continue;
		}
		let entries = registries::vanilla(Version::V26_3, &format!("minecraft:{id}"))?;
		out.push(b::configuration_to_client_packet::Ty::RegistryData(
			shared::registry(id, entries),
		));
	}
	if !config.tags {
		let mut tags = Vec::new();
		super::tags::derive(Version::V26_3, &mut tags, DERIVED);
		out.push(b::configuration_to_client_packet::Ty::Tags(tags));
	}
	out.push(b::configuration_to_client_packet::Ty::FinishConfiguration(
		p,
	));
	Ok(out)
}

pub fn late_registry_data_down_26_2(
	mut p: b::configuration_to_client_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<a::configuration_to_client_packet::Ty>> {
	shared::count_biomes(&p, cx);
	let id = short(&p.id).to_owned();
	if ADDED.contains(&id.as_str()) {
		return Ok(Vec::new());
	}
	shared::registry_entries(&mut p, "trim_material", |_, tag| {
		if let Some(Payload::String(palette)) = tag.take("palette_id") {
			let path = palette.split_once(':').map_or(palette.as_str(), |(_, p)| p);
			tag.put(
				"asset_name",
				Payload::String(path.strip_prefix("trim/").unwrap_or(path).to_owned()),
			);
		}
	});
	let mut entries: Vec<(String, Option<Tag>)> =
		p.entries.into_iter().map(|e| (e.key, e.value)).collect();
	if id == "enchantment" {
		enchantments(Version::V26_2, &mut entries, term_down)?;
	}
	Ok(vec![a::configuration_to_client_packet::Ty::RegistryData(
		shared::registry(&id, entries),
	)])
}
