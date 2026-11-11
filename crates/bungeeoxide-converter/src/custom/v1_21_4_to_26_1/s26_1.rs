use super::components;
use super::registries;
use super::remap::{self, remaps};
use super::rules::{blocks, hidden, items, statistics};
use super::shared::{self, meta};
use super::slots::{self, untrusted_step};
use super::util::{encode, read_lp_vec3, read_varint, recode};
use crate::gen::convert::{v1_21_11 as a, v26_1 as b, Root, Version};
use crate::pds::convert::{Error, Result};
use crate::Session;
use bungeeoxide_nbt::{Entry, Payload, Tag};
use std::collections::{HashMap, HashSet};

remaps!(Version::V1_21_11 => Version::V26_1 { late_sound_up_26_1: a::item_sound_holder::Ty = remap::sound; });
remaps!(Version::V26_1 => Version::V1_21_11 { late_sound_down_1_21_11: a::item_sound_holder::Ty = remap::sound; });

items!(a => b, Version::V1_21_11 => Version::V26_1 {
	slot: late_slot_item_up_26_1, hashed: late_hashed_item_up_26_1, trade1: late_trade1_up_26_1, trade2: late_trade2_up_26_1,
	display: late_display_item_up_26_1, recipes: late_recipe_items_up_26_1, stonecutter: late_stonecutter_up_26_1,
	requirements: late_requirements_up_26_1, repairable: late_repairable_up_26_1, pots: late_pots_up_26_1,
});
items!(b => a, Version::V26_1 => Version::V1_21_11 {
	slot: late_slot_item_down_1_21_11, hashed: late_hashed_item_down_1_21_11, trade1: late_trade1_down_1_21_11, trade2: late_trade2_down_1_21_11,
	display: late_display_item_down_1_21_11, recipes: late_recipe_items_down_1_21_11, stonecutter: late_stonecutter_down_1_21_11,
	requirements: late_requirements_down_1_21_11, repairable: late_repairable_down_1_21_11, pots: late_pots_down_1_21_11,
});

blocks!(a => b, Version::V1_21_11 => Version::V26_1 {
	block_change: late_block_change_up_26_1, multi_block: late_multi_block_up_26_1, block_action: late_block_action_up_26_1,
	world_event: late_world_event_up_26_1, chunk: late_chunk_up_26_1, tool: late_tool_up_26_1, predicate: late_predicate_up_26_1,
	particle: [late_particle_block_up_26_1, late_particle_marker_up_26_1, late_particle_dust_up_26_1, late_particle_pillar_up_26_1, late_particle_crumble_up_26_1],
	meta: [late_meta_state_up_26_1, late_meta_optional_state_up_26_1],
});
blocks!(b => a, Version::V26_1 => Version::V1_21_11 {
	block_change: late_block_change_down_1_21_11, multi_block: late_multi_block_down_1_21_11, block_action: late_block_action_down_1_21_11,
	world_event: late_world_event_down_1_21_11, chunk: late_chunk_down_1_21_11, tool: late_tool_down_1_21_11, predicate: late_predicate_down_1_21_11,
	particle: [late_particle_block_down_1_21_11, late_particle_marker_down_1_21_11, late_particle_dust_down_1_21_11, late_particle_pillar_down_1_21_11, late_particle_crumble_down_1_21_11],
	meta: [late_meta_state_down_1_21_11, late_meta_optional_state_down_1_21_11],
});

statistics!(a => b, Version::V1_21_11 => Version::V26_1 { late_statistics_up_26_1 });
statistics!(b => a, Version::V26_1 => Version::V1_21_11 { late_statistics_down_1_21_11 });

untrusted_step!(late_untrusted_up_26_1: a => b, Version::V1_21_11 => Version::V26_1, slots::late_to_slot_1_21_11, slots::late_to_untrusted_26_1);
untrusted_step!(late_untrusted_down_1_21_11: b => a, Version::V26_1 => Version::V1_21_11, slots::late_to_slot_26_1, slots::late_to_untrusted_1_21_11);

pub fn late_matchers_up_26_1(
	v: &a::item_block_predicate::Ty,
	_cx: &mut Session,
) -> Result<b::data_component_matchers::Ty> {
	Ok(v.components.clone())
}

hidden!(a, Version::V1_21_11 => Version::V26_1, late_hidden_up_26_1);
hidden!(b, Version::V26_1 => Version::V1_21_11, late_hidden_down_1_21_11);

pub fn late_matchers_down_1_21_11(
	v: &b::item_block_predicate::Ty,
	_cx: &mut Session,
) -> Result<a::data_component_matchers::Ty> {
	Ok(v.components.clone())
}

fn survives_down(c: &b::slot_component::Ty) -> Result<bool> {
	use b::slot_component::Ty as C;
	if matches!(
		c,
		C::ProvidesBannerPatterns(b::id_set::Ty::Ids(_))
			| C::DamageResistant(b::id_set::Ty::Ids(_))
	) {
		return Ok(false);
	}
	let bytes = encode(c, b::slot_component::serialize)?;
	let mut at = 0;
	Ok(components::remap(
		Version::V26_1,
		Version::V1_21_11,
		read_varint(&bytes, &mut at)?,
	)
	.is_some())
}

pub fn late_added_count_down_1_21_11(v: &b::slot_default::Ty, _cx: &mut Session) -> Result<i32> {
	let mut n = 0;
	for c in &v.components {
		n += survives_down(c)? as i32;
	}
	Ok(n)
}

pub fn late_removed_count_down_1_21_11(v: &b::slot_default::Ty, _cx: &mut Session) -> Result<i32> {
	let mut n = 0;
	for c in &v.remove_components {
		let bytes = encode(c, b::slot_component_type::serialize)?;
		let mut at = 0;
		n += components::remap(
			Version::V26_1,
			Version::V1_21_11,
			read_varint(&bytes, &mut at)?,
		)
		.is_some() as i32;
	}
	Ok(n)
}

fn key_index(cx: &mut Session, registry: &str, key: &str) -> Option<i32> {
	let key = if key.contains(':') {
		key.to_owned()
	} else {
		format!("minecraft:{key}")
	};
	let keys = cx
		.state::<Config>()
		.registries
		.get(&format!("minecraft:{registry}"))?;
	keys.iter().position(|k| *k == key).map(|i| i as i32)
}

fn survives_up(c: &a::slot_component::Ty, cx: &mut Session) -> Result<bool> {
	use a::slot_component::Ty as C;
	let known =
		|cx: &mut Session, registry: &str, key: &str| key_index(cx, registry, key).is_some();
	Ok(match c {
		C::UseRemainder(a::slot::Ty::V0) => false,
		C::DamageType(a::slot_component_damage_type::Ty::False(k)) => known(cx, "damage_type", k),
		C::ChickenVariant(a::slot_component_chicken_variant::Ty::False(k)) => {
			known(cx, "chicken_variant", k)
		}
		C::ZombieNautilusVariant(a::slot_component_zombie_nautilus_variant::Ty::False(k)) => {
			known(cx, "zombie_nautilus_variant", k)
		}
		C::JukeboxPlayable(a::slot_component_jukebox_playable::Ty::False(k)) => {
			known(cx, "jukebox_song", k)
		}
		C::Instrument(a::slot_component_instrument::Ty::False(k)) => known(cx, "instrument", k),
		C::ProvidesTrimMaterial(a::slot_component_provides_trim_material::Ty::False(k)) => {
			known(cx, "trim_material", k)
		}
		_ => true,
	})
}

pub fn late_added_count_up_26_1(v: &a::slot_default::Ty, cx: &mut Session) -> Result<i32> {
	let mut n = 0;
	for c in &v.components {
		n += survives_up(c, cx)? as i32;
	}
	Ok(n)
}

fn slot_up(s: a::slot::Ty, cx: &mut Session) -> Result<b::slot::Ty> {
	let packet = a::play_to_client_packet::Ty::SetCursorItem(s);
	match shared::via_step(
		Version::V1_21_11,
		Version::V26_1,
		Root::PlayToClient,
		&packet,
		a::play_to_client_packet::serialize,
		b::play_to_client_packet::deserialize,
		cx,
	)? {
		b::play_to_client_packet::Ty::SetCursorItem(s) => Ok(s),
		_ => Err(Error::Dropped),
	}
}

fn slot_down(s: b::slot::Ty, cx: &mut Session) -> Result<a::slot::Ty> {
	let packet = b::play_to_client_packet::Ty::SetCursorItem(s);
	match shared::via_step(
		Version::V26_1,
		Version::V1_21_11,
		Root::PlayToClient,
		&packet,
		b::play_to_client_packet::serialize,
		a::play_to_client_packet::deserialize,
		cx,
	)? {
		a::play_to_client_packet::Ty::SetCursorItem(s) => Ok(s),
		_ => Err(Error::Dropped),
	}
}

fn template(s: b::slot::Ty) -> Option<b::item_stack_template::Ty> {
	match s {
		b::slot::Ty::V0 => None,
		b::slot::Ty::Default(count, d) => Some(b::item_stack_template::Ty {
			item_id: d.item_id,
			item_count: count,
			added_component_count: d.components.len() as i32,
			removed_component_count: d.remove_components.len() as i32,
			components: d.components,
			remove_components: d.remove_components,
		}),
	}
}

fn untemplate(t: b::item_stack_template::Ty) -> b::slot::Ty {
	b::slot::Ty::Default(
		t.item_count,
		b::slot_default::Ty {
			item_id: t.item_id,
			added_component_count: t.components.len() as i32,
			removed_component_count: t.remove_components.len() as i32,
			components: t.components,
			remove_components: t.remove_components,
		},
	)
}

fn template_up(s: a::slot::Ty, cx: &mut Session) -> Result<Option<b::item_stack_template::Ty>> {
	Ok(template(slot_up(s, cx)?))
}

fn template_down(t: b::item_stack_template::Ty, cx: &mut Session) -> Result<a::slot::Ty> {
	slot_down(untemplate(t), cx)
}

fn templates_up(v: Vec<a::slot::Ty>, cx: &mut Session) -> Result<Vec<b::item_stack_template::Ty>> {
	let mut out = Vec::new();
	for s in v {
		out.extend(template_up(s, cx)?);
	}
	Ok(out)
}

fn templates_down(
	v: Vec<b::item_stack_template::Ty>,
	cx: &mut Session,
) -> Result<Vec<a::slot::Ty>> {
	v.into_iter().map(|t| template_down(t, cx)).collect()
}

pub fn late_use_remainder_up_26_1(
	v: a::slot::Ty,
	cx: &mut Session,
) -> Result<b::slot_component::Ty> {
	template_up(v, cx)?
		.map(b::slot_component::Ty::UseRemainder)
		.ok_or(Error::Dropped)
}

pub fn late_use_remainder_down_1_21_11(
	v: b::item_stack_template::Ty,
	cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	Ok(a::slot_component::Ty::UseRemainder(template_down(v, cx)?))
}

pub fn late_charged_up_26_1(
	v: Vec<a::slot::Ty>,
	cx: &mut Session,
) -> Result<b::slot_component::Ty> {
	Ok(b::slot_component::Ty::ChargedProjectiles(templates_up(
		v, cx,
	)?))
}

pub fn late_charged_down_1_21_11(
	v: Vec<b::item_stack_template::Ty>,
	cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	Ok(a::slot_component::Ty::ChargedProjectiles(templates_down(
		v, cx,
	)?))
}

pub fn late_bundle_up_26_1(v: Vec<a::slot::Ty>, cx: &mut Session) -> Result<b::slot_component::Ty> {
	Ok(b::slot_component::Ty::BundleContents(templates_up(v, cx)?))
}

pub fn late_bundle_down_1_21_11(
	v: Vec<b::item_stack_template::Ty>,
	cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	Ok(a::slot_component::Ty::BundleContents(templates_down(
		v, cx,
	)?))
}

pub fn late_container_up_26_1(
	v: Vec<a::slot::Ty>,
	cx: &mut Session,
) -> Result<b::slot_component::Ty> {
	Ok(b::slot_component::Ty::Container(
		v.into_iter()
			.map(|s| template_up(s, cx))
			.collect::<Result<_>>()?,
	))
}

pub fn late_container_down_1_21_11(
	v: Vec<Option<b::item_stack_template::Ty>>,
	cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	Ok(a::slot_component::Ty::Container(
		v.into_iter()
			.map(|t| t.map_or(Ok(a::slot::Ty::V0), |t| template_down(t, cx)))
			.collect::<Result<_>>()?,
	))
}

pub fn late_particle_item_up_26_1(v: a::slot::Ty, cx: &mut Session) -> Result<b::particle::Ty> {
	template_up(v, cx)?
		.map(b::particle::Ty::Item)
		.ok_or(Error::Dropped)
}

pub fn late_particle_item_down_1_21_11(
	v: b::item_stack_template::Ty,
	cx: &mut Session,
) -> Result<a::particle::Ty> {
	Ok(a::particle::Ty::Item(template_down(v, cx)?))
}

pub fn late_display_stack_up_26_1(
	v: a::slot::Ty,
	cx: &mut Session,
) -> Result<b::play_to_client_slot_display::Ty> {
	Ok(match template_up(v, cx)? {
		Some(t) => b::play_to_client_slot_display::Ty::ItemStack(t),
		None => b::play_to_client_slot_display::Ty::Empty,
	})
}

pub fn late_display_stack_down_1_21_11(
	v: b::item_stack_template::Ty,
	cx: &mut Session,
) -> Result<a::play_to_client_slot_display::Ty> {
	Ok(a::play_to_client_slot_display::Ty::ItemStack(
		template_down(v, cx)?,
	))
}

pub fn late_icon_up_26_1(
	v: &a::play_to_client_packet_advancements_advancement_mapping_items_value_display_data_items::Ty,
	cx: &mut Session,
) -> Result<b::item_stack_template::Ty> {
	match template_up(v.icon.clone(), cx)? {
		Some(t) => Ok(t),
		None => Ok(b::item_stack_template::Ty {
			item_id: crate::registry::names(Version::V26_1, "items")
				.and_then(|n| n.id("stone"))
				.unwrap_or(1) as i32,
			item_count: 1,
			added_component_count: 0,
			removed_component_count: 0,
			components: Vec::new(),
			remove_components: Vec::new(),
		}),
	}
}

pub fn late_icon_down_1_21_11(
	v: &b::play_to_client_packet_advancements_advancement_mapping_items_value_display_data_items::Ty,
	cx: &mut Session,
) -> Result<a::slot::Ty> {
	template_down(v.icon.clone(), cx)
}

macro_rules! dyes {
	($($arm:ident: $up:ident, $down:ident;)*) => {
		$(
			pub fn $up(v: i32, _cx: &mut Session) -> Result<b::slot_component::Ty> {
				Ok(b::slot_component::Ty::$arm(recode(&v, crate::pds_native::pds_varint::serialize, b::dye_color::deserialize)?))
			}

			pub fn $down(v: b::dye_color::Ty, _cx: &mut Session) -> Result<a::slot_component::Ty> {
				Ok(a::slot_component::Ty::$arm(recode(&v, b::dye_color::serialize, crate::pds_native::pds_varint::deserialize)?))
			}
		)*
	};
}

dyes! {
	CatCollar: late_cat_collar_up_26_1, late_cat_collar_down_1_21_11;
	SheepColor: late_sheep_color_up_26_1, late_sheep_color_down_1_21_11;
	ShulkerColor: late_shulker_color_up_26_1, late_shulker_color_down_1_21_11;
	BaseColor: late_base_color_up_26_1, late_base_color_down_1_21_11;
	WolfCollar: late_wolf_collar_up_26_1, late_wolf_collar_down_1_21_11;
	TropicalFishBaseColor: late_fish_base_up_26_1, late_fish_base_down_1_21_11;
	TropicalFishPatternColor: late_fish_pattern_up_26_1, late_fish_pattern_down_1_21_11;
}

#[derive(Default)]
struct Config {
	registries: HashMap<String, Vec<String>>,
	tags: bool,
}

fn keyed(cx: &mut Session, registry: &str, key: &str) -> Result<i32> {
	key_index(cx, registry, key).ok_or(Error::Dropped)
}

pub fn late_chicken_variant_up_26_1(
	v: a::slot_component_chicken_variant::Ty,
	cx: &mut Session,
) -> Result<b::slot_component::Ty> {
	Ok(b::slot_component::Ty::ChickenVariant(match v {
		a::slot_component_chicken_variant::Ty::True(id) => id,
		a::slot_component_chicken_variant::Ty::False(key) => keyed(cx, "chicken_variant", &key)?,
	}))
}

pub fn late_chicken_variant_down_1_21_11(
	v: i32,
	_cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	Ok(a::slot_component::Ty::ChickenVariant(
		a::slot_component_chicken_variant::Ty::True(v),
	))
}

pub fn late_nautilus_variant_up_26_1(
	v: a::slot_component_zombie_nautilus_variant::Ty,
	cx: &mut Session,
) -> Result<b::slot_component::Ty> {
	Ok(b::slot_component::Ty::ZombieNautilusVariant(match v {
		a::slot_component_zombie_nautilus_variant::Ty::True(id) => id,
		a::slot_component_zombie_nautilus_variant::Ty::False(key) => {
			keyed(cx, "zombie_nautilus_variant", &key)?
		}
	}))
}

pub fn late_nautilus_variant_down_1_21_11(
	v: i32,
	_cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	Ok(a::slot_component::Ty::ZombieNautilusVariant(
		a::slot_component_zombie_nautilus_variant::Ty::True(v),
	))
}

pub fn late_damage_type_up_26_1(
	v: a::slot_component_damage_type::Ty,
	cx: &mut Session,
) -> Result<b::slot_component::Ty> {
	Ok(b::slot_component::Ty::DamageType(match v {
		a::slot_component_damage_type::Ty::True(id) => id,
		a::slot_component_damage_type::Ty::False(key) => keyed(cx, "damage_type", &key)?,
	}))
}

pub fn late_damage_type_down_1_21_11(v: i32, _cx: &mut Session) -> Result<a::slot_component::Ty> {
	Ok(a::slot_component::Ty::DamageType(
		a::slot_component_damage_type::Ty::True(v),
	))
}

macro_rules! either_holders {
	($($arm:ident, $registry:literal, $either:ident, $holder:ident: $up:ident, $down:ident;)*) => {
		$(
			pub fn $up(v: a::$either::Ty, cx: &mut Session) -> Result<b::slot_component::Ty> {
				let holder = match v {
					a::$either::Ty::True(h) => recode(&h, a::$holder::serialize, b::$either::deserialize)?,
					a::$either::Ty::False(key) => {
						let id = keyed(cx, $registry, &key)?;
						recode(&(id + 1), crate::pds_native::pds_varint::serialize, b::$either::deserialize)?
					}
				};
				Ok(b::slot_component::Ty::$arm(holder))
			}

			pub fn $down(v: b::$either::Ty, _cx: &mut Session) -> Result<a::slot_component::Ty> {
				Ok(a::slot_component::Ty::$arm(a::$either::Ty::True(recode(&v, b::$either::serialize, a::$holder::deserialize)?)))
			}
		)*
	};
}

either_holders! {
	JukeboxPlayable, "jukebox_song", slot_component_jukebox_playable, slot_component_jukebox_playable_true: late_jukebox_up_26_1, late_jukebox_down_1_21_11;
	Instrument, "instrument", slot_component_instrument, slot_component_instrument_true: late_instrument_up_26_1, late_instrument_down_1_21_11;
	ProvidesTrimMaterial, "trim_material", slot_component_provides_trim_material, slot_component_provides_trim_material_true: late_trim_material_up_26_1, late_trim_material_down_1_21_11;
}

fn tag_set(key: String) -> b::id_set::Ty {
	b::id_set::Ty::Named(key.trim_start_matches('#').to_owned())
}

fn set_tag(v: b::id_set::Ty) -> Result<String> {
	match v {
		b::id_set::Ty::Named(n) => Ok(n),
		b::id_set::Ty::Ids(_) => Err(Error::Dropped),
	}
}

pub fn late_banner_patterns_up_26_1(v: String, _cx: &mut Session) -> Result<b::slot_component::Ty> {
	Ok(b::slot_component::Ty::ProvidesBannerPatterns(tag_set(v)))
}

pub fn late_banner_patterns_down_1_21_11(
	v: b::id_set::Ty,
	_cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	Ok(a::slot_component::Ty::ProvidesBannerPatterns(set_tag(v)?))
}

pub fn late_damage_resistant_up_26_1(
	v: String,
	_cx: &mut Session,
) -> Result<b::slot_component::Ty> {
	Ok(b::slot_component::Ty::DamageResistant(tag_set(v)))
}

pub fn late_damage_resistant_down_1_21_11(
	v: b::id_set::Ty,
	_cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	Ok(a::slot_component::Ty::DamageResistant(set_tag(v)?))
}

pub fn late_bypassed_up_26_1(
	v: &a::slot_component_blocks_attacks::Ty,
	_cx: &mut Session,
) -> Result<Option<b::id_set::Ty>> {
	Ok(v.bypassed_by.clone().map(tag_set))
}

pub fn late_bypassed_down_1_21_11(
	v: &b::slot_component_blocks_attacks::Ty,
	_cx: &mut Session,
) -> Result<Option<String>> {
	Ok(v.bypassed_by.clone().and_then(|s| set_tag(s).ok()))
}

pub fn late_update_time_up_26_1(
	v: &a::play_to_client_packet_update_time::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet_update_time_clock_updates_items::Ty>> {
	let rate = if v.tick_day_time { 1.0 } else { 0.0 };
	Ok((0..2)
		.map(
			|id| b::play_to_client_packet_update_time_clock_updates_items::Ty {
				id,
				total_ticks: v.time,
				partial_tick: 0.0,
				rate,
			},
		)
		.collect())
}

pub fn late_day_time_down_1_21_11(
	v: &b::play_to_client_packet_update_time::Ty,
	_cx: &mut Session,
) -> Result<i64> {
	Ok(v.clock_updates
		.iter()
		.find(|c| c.id == 0)
		.map_or(0, |c| c.total_ticks))
}

pub fn late_tick_day_time_down_1_21_11(
	v: &b::play_to_client_packet_update_time::Ty,
	_cx: &mut Session,
) -> Result<bool> {
	Ok(v.clock_updates
		.iter()
		.find(|c| c.id == 0)
		.is_some_and(|c| c.rate != 0.0))
}

pub fn late_open_book_up_26_1(
	v: i32,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	Ok(vec![b::play_to_client_packet::Ty::OpenBook(recode(
		&v,
		crate::pds_native::pds_varint::serialize,
		b::play_to_client_packet_open_book::deserialize,
	)?)])
}

pub fn late_open_book_down_1_21_11(
	v: b::play_to_client_packet_open_book::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	Ok(vec![a::play_to_client_packet::Ty::OpenBook(recode(
		&v,
		b::play_to_client_packet_open_book::serialize,
		crate::pds_native::pds_varint::deserialize,
	)?)])
}

const AGEABLE: &[&str] = &[
	"villager",
	"wandering_trader",
	"axolotl",
	"chicken",
	"panda",
	"pig",
	"polar_bear",
	"rabbit",
	"sheep",
	"bee",
	"turtle",
	"fox",
	"frog",
	"goat",
	"hoglin",
	"strider",
	"sniffer",
	"armadillo",
	"happy_ghast",
	"ocelot",
	"cow",
	"mooshroom",
	"cat",
	"wolf",
	"parrot",
	"nautilus",
	"zombie_nautilus",
	"horse",
	"skeleton_horse",
	"zombie_horse",
	"camel",
	"camel_husk",
	"donkey",
	"mule",
	"llama",
	"trader_llama",
	"dolphin",
	"squid",
	"glow_squid",
];

pub fn late_meta_entity_up_26_1(
	p: &a::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, Version::V1_21_11, p.entity_id);
	Ok(p.entity_id)
}

pub fn late_meta_entity_down_1_21_11(
	p: &b::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, Version::V26_1, p.entity_id);
	Ok(p.entity_id)
}

pub fn late_meta_key_up_26_1(e: &a::entity_metadata_entry::Ty, cx: &mut Session) -> Result<u8> {
	meta(cx).key = e.key;
	Ok(if e.key >= 17 && shared::is(cx, AGEABLE) {
		e.key + 1
	} else {
		e.key
	})
}

pub fn late_meta_key_down_1_21_11(
	e: &b::entity_metadata_entry::Ty,
	cx: &mut Session,
) -> Result<u8> {
	if let Some(key) = shared::fallback_key(cx, Version::V26_1, Version::V1_21_11, e.key) {
		return key;
	}
	meta(cx).key = e.key;
	if e.key < 17 || !shared::is(cx, AGEABLE) {
		return Ok(e.key);
	}
	match e.key {
		17 => Err(Error::Dropped),
		k => Ok(k - 1),
	}
}

pub fn late_attack_down_1_21_11(
	v: i32,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_server_packet::Ty>> {
	Ok(vec![a::play_to_server_packet::Ty::UseEntity(
		a::play_to_server_packet_use_entity::Ty {
			target: v,
			mouse: a::play_to_server_packet_use_entity_mouse::Ty::Default(1),
			sneaking: false,
		},
	)])
}

pub fn late_use_entity_down_1_21_11(
	v: b::play_to_server_packet_use_entity::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_server_packet::Ty>> {
	let hand = recode(
		&v.hand,
		b::play_to_server_packet_use_entity_hand::serialize,
		crate::pds_native::pds_varint::deserialize,
	)?;
	let (x, y, z) = read_lp_vec3(&v.location)?;
	let packet = |mouse| {
		a::play_to_server_packet::Ty::UseEntity(a::play_to_server_packet_use_entity::Ty {
			target: v.target,
			mouse,
			sneaking: v.sneaking,
		})
	};
	Ok(vec![
		packet(a::play_to_server_packet_use_entity_mouse::Ty::V2(
			a::play_to_server_packet_use_entity_mouse_v2::Ty {
				x: x as f32,
				y: y as f32,
				z: z as f32,
				hand,
			},
		)),
		packet(a::play_to_server_packet_use_entity_mouse::Ty::V0(hand)),
	])
}

pub fn late_use_entity_up_26_1(
	v: a::play_to_server_packet_use_entity::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_server_packet::Ty>> {
	use a::play_to_server_packet_use_entity_mouse::Ty as Mouse;
	Ok(match v.mouse {
		Mouse::V2(at) => vec![b::play_to_server_packet::Ty::UseEntity(
			b::play_to_server_packet_use_entity::Ty {
				target: v.target,
				hand: recode(
					&at.hand,
					crate::pds_native::pds_varint::serialize,
					b::play_to_server_packet_use_entity_hand::deserialize,
				)?,
				location: super::util::lp_vec3(at.x as f64, at.y as f64, at.z as f64),
				sneaking: v.sneaking,
			},
		)],
		Mouse::Default(1) => vec![b::play_to_server_packet::Ty::Attack(v.target)],
		_ => Vec::new(),
	})
}

pub fn late_client_command_down_1_21_11(
	v: b::play_to_server_packet_client_command::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_server_packet::Ty>> {
	if matches!(
		v,
		b::play_to_server_packet_client_command::Ty::RequestGameruleValues
	) {
		return Ok(vec![]);
	}
	Ok(vec![a::play_to_server_packet::Ty::ClientCommand(recode(
		&v,
		b::play_to_server_packet_client_command::serialize,
		a::play_to_server_packet_client_command::deserialize,
	)?)])
}

pub fn late_arm_animation_down_1_21_11(
	v: b::play_to_server_packet_arm_animation::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_server_packet::Ty>> {
	Ok(vec![a::play_to_server_packet::Ty::ArmAnimation(recode(
		&v,
		b::play_to_server_packet_arm_animation::serialize,
		crate::pds_native::pds_varint::deserialize,
	)?)])
}

pub fn late_block_place_hand_down_1_21_11(
	v: &b::play_to_server_packet_block_place::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	recode(
		&v.hand,
		b::play_to_server_packet_block_place_hand::serialize,
		crate::pds_native::pds_varint::deserialize,
	)
}

pub fn late_use_item_hand_down_1_21_11(
	v: &b::play_to_server_packet_use_item::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	recode(
		&v.hand,
		b::play_to_server_packet_use_item_hand::serialize,
		crate::pds_native::pds_varint::deserialize,
	)
}

pub fn late_block_place_hand_up_26_1(
	v: &a::play_to_server_packet_block_place::Ty,
	_cx: &mut Session,
) -> Result<b::play_to_server_packet_block_place_hand::Ty> {
	recode(
		&v.hand,
		crate::pds_native::pds_varint::serialize,
		b::play_to_server_packet_block_place_hand::deserialize,
	)
}

pub fn late_use_item_hand_up_26_1(
	v: &a::play_to_server_packet_use_item::Ty,
	_cx: &mut Session,
) -> Result<b::play_to_server_packet_use_item_hand::Ty> {
	recode(
		&v.hand,
		crate::pds_native::pds_varint::serialize,
		b::play_to_server_packet_use_item_hand::deserialize,
	)
}

pub fn late_arm_animation_up_26_1(
	v: i32,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_server_packet::Ty>> {
	Ok(vec![b::play_to_server_packet::Ty::ArmAnimation(recode(
		&v,
		crate::pds_native::pds_varint::serialize,
		b::play_to_server_packet_arm_animation::deserialize,
	)?)])
}

fn asset_affix(tag: &mut Payload, name: &str) {
	let Some(asset) = tag
		.field("asset_id")
		.and_then(Payload::as_str)
		.map(str::to_owned)
	else {
		return;
	};
	let suffix = format!("_{name}");
	if let Some(stripped) = asset.strip_suffix(&suffix) {
		let renamed = stripped.replace(&format!("{name}/"), &format!("{name}/{name}_"));
		tag.put("asset_id", Payload::String(renamed.to_owned()));
	}
}

fn asset_prefix(tag: &mut Payload, name: &str) {
	let Some(asset) = tag
		.field("asset_id")
		.and_then(Payload::as_str)
		.map(str::to_owned)
	else {
		return;
	};
	if !asset.contains(&format!("{name}/{name}_")) {
		tag.put(
			"asset_id",
			Payload::String(asset.replace(&format!("{name}/"), &format!("{name}/{name}_"))),
		);
	}
}

fn baby_asset(tag: &mut Payload) {
	if let Some(asset) = tag
		.field("asset_id")
		.and_then(Payload::as_str)
		.map(str::to_owned)
	{
		tag.put("baby_asset_id", Payload::String(format!("{asset}_baby")));
	}
}

fn ambient_light_color(key: &str, tag: &Payload) -> i32 {
	match key {
		"the_end" => -12630209,
		"the_nether" => -13621215,
		"overworld" => -16119286,
		_ => {
			let light = match tag.field("ambient_light") {
				Some(Payload::Float(f)) => *f,
				_ => 0.0,
			};
			let v = (10.0 + light * 245.0).round().clamp(10.0, 255.0) as i32;
			(0xFF00_0000u32 as i32) | v << 16 | v << 8 | v
		}
	}
}

fn convert_entry(registry: &str, key: &str, tag: &mut Payload) {
	let key = key.strip_prefix("minecraft:").unwrap_or(key);
	match registry {
		"timeline" => tag.put("clock", Payload::from("overworld")),
		"wolf_sound_variant" => {
			let mut sounds = std::mem::replace(tag, Payload::Compound(vec![]));
			sounds.put("step_sound", Payload::from("entity.wolf.step"));
			tag.put("baby_sounds", sounds.clone());
			tag.put("adult_sounds", sounds);
		}
		"wolf_variant" => {
			if let Some(Payload::Compound(assets)) = tag.field("assets") {
				let baby = assets
					.iter()
					.filter_map(|e| match &e.value {
						Payload::String(s) => {
							Some(Entry::new(&e.name, Payload::String(format!("{s}_baby"))))
						}
						_ => None,
					})
					.collect();
				tag.put("baby_assets", Payload::Compound(baby));
			}
		}
		"frog_variant" => asset_affix(tag, "frog"),
		"chicken_variant" | "cow_variant" | "pig_variant" => {
			asset_affix(tag, registry.trim_end_matches("_variant"));
			baby_asset(tag);
		}
		"cat_variant" => {
			asset_prefix(tag, "cat");
			baby_asset(tag);
		}
		"dimension_type" => {
			tag.put(
				"has_ender_dragon_fight",
				Payload::Byte((key == "the_end") as i8),
			);
			match key {
				"overworld" | "overworld_caves" => {
					tag.put("default_clock", Payload::from("minecraft:overworld"))
				}
				"the_end" => tag.put("default_clock", Payload::from("minecraft:the_end")),
				_ => {}
			}
			let color = ambient_light_color(key, tag);
			let mut attributes = tag
				.field("attributes")
				.cloned()
				.unwrap_or_else(|| Payload::Compound(vec![]));
			attributes.put("visual/ambient_light_color", Payload::Int(color));
			tag.put("attributes", attributes);
		}
		_ => {}
	}
}

const REQUIRED: &[(&str, &[&str])] = &[
	("damage_type", &["spear"]),
	("chicken_variant", &["temperate", "cold", "warm"]),
	("instrument", &["ponder_goat_horn"]),
	(
		"jukebox_song",
		&[
			"11",
			"13",
			"5",
			"blocks",
			"cat",
			"chirp",
			"far",
			"mall",
			"mellohi",
			"otherside",
			"pigstep",
			"relic",
			"stal",
			"strad",
			"wait",
			"ward",
			"creator",
			"creator_music_box",
			"lava_chicken",
			"tears",
			"precipice",
		],
	),
	(
		"trim_material",
		&[
			"quartz",
			"iron",
			"netherite",
			"redstone",
			"copper",
			"gold",
			"emerald",
			"diamond",
			"lapis",
			"amethyst",
			"resin",
		],
	),
];

const ADDED: &[&str] = &[
	"world_clock",
	"cat_sound_variant",
	"cow_sound_variant",
	"pig_sound_variant",
	"chicken_sound_variant",
];

fn add_required(
	registry: &str,
	entries: &mut Vec<(String, Option<bungeeoxide_nbt::Tag>)>,
) -> Result<()> {
	let Some((_, keys)) = REQUIRED.iter().find(|(r, _)| *r == registry) else {
		return Ok(());
	};
	let vanilla = registries::vanilla(Version::V26_1, &format!("minecraft:{registry}"))?;
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

pub fn late_registry_data_up_26_1(
	p: a::configuration_to_client_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_client_packet::Ty>> {
	shared::count_biomes(&p, cx);
	let id = p.id.strip_prefix("minecraft:").unwrap_or(&p.id).to_owned();
	let mut entries: Vec<(String, Option<bungeeoxide_nbt::Tag>)> = p
		.entries
		.into_iter()
		.map(|e| {
			let value = e.value.map(|mut t| {
				convert_entry(&id, &e.key, &mut t.value);
				Tag::new(t.value)
			});
			(e.key, value)
		})
		.collect();
	add_required(&id, &mut entries)?;
	let keys = entries
		.iter()
		.map(|(k, _)| {
			if k.contains(':') {
				k.clone()
			} else {
				format!("minecraft:{k}")
			}
		})
		.collect();
	cx.state::<Config>()
		.registries
		.insert(format!("minecraft:{id}"), keys);
	Ok(vec![b::configuration_to_client_packet::Ty::RegistryData(
		shared::registry(&id, entries),
	)])
}

const DAMAGE_TAGS: &[&str] = &[
	"damages_helmet",
	"bypasses_armor",
	"bypasses_shield",
	"bypasses_invulnerability",
	"bypasses_cooldown",
	"bypasses_effects",
	"bypasses_resistance",
	"bypasses_enchantments",
	"is_fire",
	"is_projectile",
	"witch_resistant_to",
	"is_explosion",
	"is_fall",
	"is_drowning",
	"is_freezing",
	"is_lightning",
	"no_anger",
	"no_impact",
	"always_most_significant_fall",
	"wither_immune_to",
	"ignites_armor_stands",
	"burns_armor_stands",
	"avoids_guardian_thorns",
	"always_triggers_silverfish",
	"always_hurts_ender_dragons",
	"no_knockback",
	"always_kills_armor_stands",
	"can_break_armor_stand",
	"bypasses_wolf_armor",
	"is_player_attack",
	"burn_from_stepping",
	"panic_causes",
	"panic_environmental_causes",
	"mace_smash",
];

const BANNER_TAGS: &[&str] = &[
	"no_item_required",
	"pattern_item/flower",
	"pattern_item/creeper",
	"pattern_item/skull",
	"pattern_item/mojang",
	"pattern_item/globe",
	"pattern_item/piglin",
	"pattern_item/flow",
	"pattern_item/guster",
	"pattern_item/field_masoned",
	"pattern_item/bordure_indented",
];

const TAG_RENAMES: &[(&str, &str, &str)] = &[
	(
		"block",
		"dry_vegetation_may_place_on",
		"supports_dry_vegetation",
	),
	("block", "bamboo_plantable_on", "supports_bamboo"),
	(
		"block",
		"small_dripleaf_placeable",
		"supports_small_dripleaf",
	),
	("block", "big_dripleaf_placeable", "supports_big_dripleaf"),
	(
		"block",
		"mushroom_grow_block",
		"overrides_mushroom_light_requirement",
	),
	(
		"block",
		"snow_layer_can_survive_on",
		"support_override_snow_layer",
	),
	(
		"block",
		"snow_layer_cannot_survive_on",
		"cannot_support_snow_layer",
	),
];

const TAG_RENAMES_DOWN: &[(&str, &str, &str)] = &[
	(
		"block",
		"supports_dry_vegetation",
		"dry_vegetation_may_place_on",
	),
	("block", "supports_bamboo", "bamboo_plantable_on"),
	(
		"block",
		"supports_small_dripleaf",
		"small_dripleaf_placeable",
	),
	("block", "supports_big_dripleaf", "big_dripleaf_placeable"),
	(
		"block",
		"overrides_mushroom_light_requirement",
		"mushroom_grow_block",
	),
	(
		"block",
		"support_override_snow_layer",
		"snow_layer_can_survive_on",
	),
	(
		"block",
		"cannot_support_snow_layer",
		"snow_layer_cannot_survive_on",
	),
];

fn required_tags(v: &mut Vec<b::configuration_to_client_packet_tags_items::Ty>) {
	for (registry, names) in [
		("minecraft:damage_type", DAMAGE_TAGS),
		("minecraft:banner_pattern", BANNER_TAGS),
	] {
		let at = match v.iter().position(|r| {
			r.tag_type == registry || r.tag_type == registry.trim_start_matches("minecraft:")
		}) {
			Some(at) => at,
			None => {
				v.push(b::configuration_to_client_packet_tags_items::Ty {
					tag_type: registry.to_owned(),
					tags: Vec::new(),
				});
				v.len() - 1
			}
		};
		for name in names {
			let full = format!("minecraft:{name}");
			if !v[at]
				.tags
				.iter()
				.any(|t| t.tag_name == full || t.tag_name == *name)
			{
				v[at].tags.push(
					b::configuration_to_client_packet_tags_items_tags_items::Ty {
						tag_name: full,
						entries: Vec::new(),
					},
				);
			}
		}
	}
}

pub fn late_config_tags_up_26_1(
	v: Vec<a::configuration_to_client_packet_tags_items::Ty>,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_client_packet::Ty>> {
	cx.state::<Config>().tags = true;
	let mut v = super::tags::config(Version::V1_21_11, Version::V26_1, TAG_RENAMES, v);
	required_tags(&mut v);
	Ok(vec![b::configuration_to_client_packet::Ty::Tags(v)])
}

pub fn late_config_tags_down_1_21_11(
	v: Vec<b::configuration_to_client_packet_tags_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<a::configuration_to_client_packet::Ty>> {
	Ok(vec![a::configuration_to_client_packet::Ty::Tags(
		super::tags::config(Version::V26_1, Version::V1_21_11, TAG_RENAMES_DOWN, v),
	)])
}

pub fn late_play_tags_up_26_1(
	v: Vec<a::play_to_client_packet_tags_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<b::play_to_client_packet::Ty>> {
	Ok(vec![b::play_to_client_packet::Ty::Tags(super::tags::play(
		Version::V1_21_11,
		Version::V26_1,
		TAG_RENAMES,
		v,
	))])
}

pub fn late_play_tags_down_1_21_11(
	v: Vec<b::play_to_client_packet_tags_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<a::play_to_client_packet::Ty>> {
	Ok(vec![a::play_to_client_packet::Ty::Tags(super::tags::play(
		Version::V26_1,
		Version::V1_21_11,
		TAG_RENAMES_DOWN,
		v,
	))])
}

pub fn late_finish_configuration_up_26_1(
	p: a::configuration_to_client_packet_finish_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_to_client_packet::Ty>> {
	let config = std::mem::take(cx.state::<Config>());
	let seen: HashSet<&str> = config.registries.keys().map(String::as_str).collect();
	let mut out = Vec::new();
	for id in ADDED.iter().chain(REQUIRED.iter().map(|(r, _)| r)) {
		if seen.contains(format!("minecraft:{id}").as_str()) {
			continue;
		}
		let entries = registries::vanilla(Version::V26_1, &format!("minecraft:{id}"))?;
		out.push(b::configuration_to_client_packet::Ty::RegistryData(
			shared::registry(id, entries),
		));
	}
	if !config.tags {
		let mut tags = Vec::new();
		required_tags(&mut tags);
		out.push(b::configuration_to_client_packet::Ty::Tags(tags));
	}
	out.push(b::configuration_to_client_packet::Ty::FinishConfiguration(
		p,
	));
	Ok(out)
}

const REMOVED: &[&str] = &[
	"world_clock",
	"cat_sound_variant",
	"cow_sound_variant",
	"pig_sound_variant",
	"chicken_sound_variant",
];
const NEW_ATTRIBUTES: &[&str] = &[
	"visual/block_light_tint",
	"visual/night_vision_color",
	"visual/ambient_light_color",
];

fn strip_attributes(tag: &mut Payload, field: &str) {
	if let Some(Payload::Compound(mut entries)) = tag.take(field) {
		entries.retain(|e| {
			!NEW_ATTRIBUTES.contains(&e.name.strip_prefix("minecraft:").unwrap_or(&e.name))
		});
		tag.put(field, Payload::Compound(entries));
	}
}

fn asset_unaffix(tag: &mut Payload, name: &str) {
	let Some(asset) = tag
		.field("asset_id")
		.and_then(Payload::as_str)
		.map(str::to_owned)
	else {
		return;
	};
	let prefix = format!("{name}_");
	if asset.contains(&prefix) {
		tag.put(
			"asset_id",
			Payload::String(format!("{}_{name}", asset.replace(&prefix, ""))),
		);
	}
}

fn convert_entry_down(registry: &str, _key: &str, tag: &mut Payload) {
	match registry {
		"dimension_type" => {
			strip_attributes(tag, "attributes");
			tag.take("has_ender_dragon_fight");
			tag.take("default_clock");
		}
		"timeline" => {
			strip_attributes(tag, "tracks");
			tag.take("clock");
		}
		"wolf_sound_variant" => {
			tag.take("baby_sounds");
			if let Some(Payload::Compound(sounds)) = tag.take("adult_sounds") {
				for e in sounds {
					tag.put(&e.name, e.value);
				}
			}
		}
		"wolf_variant" => {
			tag.take("baby_assets");
		}
		"frog_variant" => asset_unaffix(tag, "frog"),
		"chicken_variant" | "cow_variant" | "pig_variant" => {
			asset_unaffix(tag, registry.trim_end_matches("_variant"));
			tag.take("baby_asset_id");
		}
		"cat_variant" => {
			if let Some(asset) = tag
				.field("asset_id")
				.and_then(Payload::as_str)
				.map(str::to_owned)
			{
				tag.put("asset_id", Payload::String(asset.replace("cat_", "")));
			}
			tag.take("baby_asset_id");
		}
		_ => {}
	}
}

pub fn late_registry_data_down_1_21_11(
	mut p: b::configuration_to_client_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<a::configuration_to_client_packet::Ty>> {
	shared::count_biomes(&p, cx);
	let id = p.id.strip_prefix("minecraft:").unwrap_or(&p.id).to_owned();
	if REMOVED.contains(&id.as_str()) {
		return Ok(Vec::new());
	}
	shared::registry_entries(&mut p, &id, |key, tag| convert_entry_down(&id, key, tag));
	Ok(vec![a::configuration_to_client_packet::Ty::RegistryData(p)])
}
