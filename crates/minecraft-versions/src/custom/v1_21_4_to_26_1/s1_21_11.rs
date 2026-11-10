use super::components;
use super::registries;
use super::remap::{self, remaps};
use super::rules::{entities, hidden, items, statistics, tags};
use super::shared::{self, meta};
use super::slots::{self, untrusted_step};
use super::util::{encode, read_varint};
use crate::gen::convert::{v1_21_11 as b, v1_21_5, v1_21_9 as a, Version};
use crate::pds::convert::{Error, Result};
use crate::Session;
use nbt::{CompoundExt, Entry, List, ListItems, Payload};

remaps!(Version::V1_21_9 => Version::V1_21_11 { late_sound_up_1_21_11: a::ItemSoundHolder::Ty = remap::sound; });
remaps!(Version::V1_21_11 => Version::V1_21_9 { late_sound_down_1_21_9: a::ItemSoundHolder::Ty = remap::sound; });

items!(a => b, Version::V1_21_9 => Version::V1_21_11 {
	slot: late_slot_item_up_1_21_11, hashed: late_hashed_item_up_1_21_11, trade1: late_trade1_up_1_21_11, trade2: late_trade2_up_1_21_11,
	display: late_display_item_up_1_21_11, recipes: late_recipe_items_up_1_21_11, stonecutter: late_stonecutter_up_1_21_11,
	requirements: late_requirements_up_1_21_11, repairable: late_repairable_up_1_21_11, pots: late_pots_up_1_21_11,
});
items!(b => a, Version::V1_21_11 => Version::V1_21_9 {
	slot: late_slot_item_down_1_21_9, hashed: late_hashed_item_down_1_21_9, trade1: late_trade1_down_1_21_9, trade2: late_trade2_down_1_21_9,
	display: late_display_item_down_1_21_9, recipes: late_recipe_items_down_1_21_9, stonecutter: late_stonecutter_down_1_21_9,
	requirements: late_requirements_down_1_21_9, repairable: late_repairable_down_1_21_9, pots: late_pots_down_1_21_9,
});

entities!(a => b, Version::V1_21_9 => Version::V1_21_11 { spawn: late_spawn_type_up_1_21_11, object: late_object_data_up_1_21_11, equippable: late_equip_entities_up_1_21_11, });
entities!(b => a, Version::V1_21_11 => Version::V1_21_9 { spawn: late_spawn_type_down_1_21_9, object: late_object_data_down_1_21_9, equippable: late_equip_entities_down_1_21_9, });

statistics!(a => b, Version::V1_21_9 => Version::V1_21_11 { late_statistics_up_1_21_11 });
statistics!(b => a, Version::V1_21_11 => Version::V1_21_9 { late_statistics_down_1_21_9 });

tags!(a => b, Version::V1_21_9 => Version::V1_21_11, &[], { config: late_config_tags_up_1_21_11, play: late_play_tags_up_1_21_11 });
tags!(b => a, Version::V1_21_11 => Version::V1_21_9, &[], { config: late_config_tags_down_1_21_9, play: late_play_tags_down_1_21_9 });

untrusted_step!(late_untrusted_up_1_21_11: a => b, Version::V1_21_9 => Version::V1_21_11, slots::late_to_slot_1_21_9, slots::late_to_untrusted_1_21_11);
untrusted_step!(late_untrusted_down_1_21_9: b => a, Version::V1_21_11 => Version::V1_21_9, slots::late_to_slot_1_21_11, slots::late_to_untrusted_1_21_9);

pub fn late_matchers_up_1_21_11(
	v: &a::ItemBlockPredicate::Ty,
	_cx: &mut Session,
) -> Result<b::DataComponentMatchers::Ty> {
	let mut exactMatchers = Vec::new();
	for c in &v.components.exactMatchers {
		let map = |t| components::remap(Version::V1_21_9, Version::V1_21_11, t);
		exactMatchers.extend(components::retag(
			map,
			c,
			v1_21_5::SlotComponent::serialize,
			b::SlotComponent::deserialize,
		)?);
	}
	let partialMatchers = v
		.components
		.partialMatchers
		.iter()
		.map(|p| b::DataComponentMatchers__partialMatchers__items::Ty {
			isPredicateType: true,
			r#type: p.r#type,
			predicate: p.predicate.clone(),
		})
		.collect();
	Ok(b::DataComponentMatchers::Ty {
		exactMatchers,
		partialMatchers,
	})
}

hidden!(a, Version::V1_21_9 => Version::V1_21_11, late_hidden_up_1_21_11);
hidden!(b, Version::V1_21_11 => Version::V1_21_9, late_hidden_down_1_21_9);

pub fn late_matchers_down_1_21_9(
	v: &b::ItemBlockPredicate::Ty,
	_cx: &mut Session,
) -> Result<a::DataComponentMatchers::Ty> {
	let mut exactMatchers = Vec::new();
	for c in &v.components.exactMatchers {
		let map = |t| components::remap(Version::V1_21_11, Version::V1_21_9, t);
		exactMatchers.extend(components::retag(
			map,
			c,
			b::SlotComponent::serialize,
			v1_21_5::SlotComponent::deserialize,
		)?);
	}
	let partialMatchers = v
		.components
		.partialMatchers
		.iter()
		.filter(|p| p.isPredicateType)
		.map(|p| a::DataComponentMatchers__partialMatchers__items::Ty {
			r#type: p.r#type,
			predicate: p.predicate.clone(),
		})
		.collect();
	Ok(a::DataComponentMatchers::Ty {
		exactMatchers,
		partialMatchers,
	})
}

fn late_known_in_1_21_9<T>(
	v: &T,
	ser: impl Fn(&T, &mut crate::pds::Output) -> crate::pds::Result<()>,
) -> Result<bool> {
	let bytes = encode(v, ser)?;
	let mut at = 0;
	Ok(components::remap(
		Version::V1_21_11,
		Version::V1_21_9,
		read_varint(&bytes, &mut at)?,
	)
	.is_some())
}

pub fn late_added_count_down_1_21_9(v: &b::Slot__default::Ty, _cx: &mut Session) -> Result<i32> {
	let mut n = 0;
	for c in &v.components {
		n += late_known_in_1_21_9(c, b::SlotComponent::serialize)? as i32;
	}
	Ok(n)
}

pub fn late_removed_count_down_1_21_9(v: &b::Slot__default::Ty, _cx: &mut Session) -> Result<i32> {
	let mut n = 0;
	for c in &v.removeComponents {
		n += late_known_in_1_21_9(c, b::SlotComponentType::serialize)? as i32;
	}
	Ok(n)
}

pub fn late_border_speed_up_1_21_11(
	v: &a::play_toClient_packet_initialize_world_border::Ty,
	_cx: &mut Session,
) -> Result<i64> {
	Ok(v.speed / 50)
}

pub fn late_border_speed_down_1_21_9(
	v: &b::play_toClient_packet_initialize_world_border::Ty,
	_cx: &mut Session,
) -> Result<i64> {
	Ok(v.speed.saturating_mul(50))
}

pub fn late_lerp_speed_up_1_21_11(
	v: &a::play_toClient_packet_world_border_lerp_size::Ty,
	_cx: &mut Session,
) -> Result<i64> {
	Ok(v.speed / 50)
}

pub fn late_lerp_speed_down_1_21_9(
	v: &b::play_toClient_packet_world_border_lerp_size::Ty,
	_cx: &mut Session,
) -> Result<i64> {
	Ok(v.speed.saturating_mul(50))
}

#[derive(Default)]
struct GameTime(i64);

pub fn late_game_time_up_1_21_11(
	v: &a::play_toClient_packet_update_time::Ty,
	cx: &mut Session,
) -> Result<i64> {
	cx.state::<GameTime>().0 = v.age;
	Ok(v.age)
}

pub fn late_tick_end_down_1_21_9(
	v: b::play_toServer_packet_tick_end::Ty,
	cx: &mut Session,
) -> Result<Vec<a::play_toServer_packet::Ty>> {
	cx.state::<GameTime>().0 += 1;
	Ok(vec![a::play_toServer_packet::Ty::tick_end(v)])
}

pub fn late_dig_status_down_1_21_9(
	v: &b::play_toServer_packet_block_dig::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	match v.status {
		7 => Err(Error::Dropped),
		s => Ok(s),
	}
}

pub fn late_meta_entity_up_1_21_11(
	p: &a::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, Version::V1_21_9, p.entityId);
	Ok(p.entityId)
}

pub fn late_meta_entity_down_1_21_9(
	p: &b::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, Version::V1_21_11, p.entityId);
	Ok(p.entityId)
}

pub fn late_meta_key_up_1_21_11(e: &a::entityMetadataEntry::Ty, cx: &mut Session) -> Result<u8> {
	meta(cx).key = e.key;
	Ok(e.key)
}

pub fn late_meta_key_down_1_21_9(e: &b::entityMetadataEntry::Ty, cx: &mut Session) -> Result<u8> {
	if let Some(key) = shared::fallback_key(cx, Version::V1_21_11, Version::V1_21_9, e.key) {
		return key;
	}
	meta(cx).key = e.key;
	Ok(e.key)
}

fn anger(cx: &mut Session) -> bool {
	let key = meta(cx).key;
	(key == 21 && shared::is(cx, &["wolf"])) || (key == 18 && shared::is(cx, &["bee"]))
}

fn main_hand(cx: &mut Session) -> bool {
	meta(cx).key == 15 && shared::is(cx, &["player", "mannequin"])
}

pub fn late_meta_int_up_1_21_11(
	v: i32,
	cx: &mut Session,
) -> Result<b::entityMetadataEntry__value::Ty> {
	if !anger(cx) {
		return Ok(b::entityMetadataEntry__value::Ty::int(v));
	}
	let end = if v > 0 {
		cx.state::<GameTime>().0 + v as i64
	} else {
		0
	};
	Ok(b::entityMetadataEntry__value::Ty::long(end))
}

pub fn late_meta_long_down_1_21_9(
	v: i64,
	cx: &mut Session,
) -> Result<a::entityMetadataEntry__value::Ty> {
	if !anger(cx) {
		return Ok(a::entityMetadataEntry__value::Ty::long(v));
	}
	let left = (v - cx.state::<GameTime>().0).clamp(0, i32::MAX as i64);
	Ok(a::entityMetadataEntry__value::Ty::int(left as i32))
}

pub fn late_meta_byte_up_1_21_11(
	v: i8,
	cx: &mut Session,
) -> Result<b::entityMetadataEntry__value::Ty> {
	use b::entityMetadataEntry__value__humanoid_arm::Ty as Arm;
	if !main_hand(cx) {
		return Ok(b::entityMetadataEntry__value::Ty::byte(v));
	}
	Ok(b::entityMetadataEntry__value::Ty::humanoid_arm(if v == 0 {
		Arm::left
	} else {
		Arm::right
	}))
}

pub fn late_meta_arm_down_1_21_9(
	v: b::entityMetadataEntry__value__humanoid_arm::Ty,
	_cx: &mut Session,
) -> Result<a::entityMetadataEntry__value::Ty> {
	use b::entityMetadataEntry__value__humanoid_arm::Ty as Arm;
	Ok(a::entityMetadataEntry__value::Ty::byte(match v {
		Arm::left => 0,
		Arm::right => 1,
		Arm::default(t) => t as i8,
	}))
}

fn attribute(attributes: &mut Payload, key: &str, value: Payload) {
	attributes.put(key, value);
}

fn copy(
	from: &Payload,
	field: &str,
	attributes: &mut Payload,
	key: &str,
	default: Option<Payload>,
) {
	match (from.field(field), default) {
		(Some(v), _) => attribute(attributes, key, v.clone()),
		(None, Some(d)) => attribute(attributes, key, d),
		(None, None) => {}
	}
}

fn strings(values: &[&str]) -> Payload {
	ListItems::string(values.iter().map(|s| (*s).to_owned()).collect()).into()
}

fn mood() -> Payload {
	Payload::compound(vec![Entry::new(
		"mood",
		Payload::compound(vec![
			Entry::new("tick_delay", Payload::int(6000)),
			Entry::new("offset", Payload::float(2.0)),
			Entry::new("sound", Payload::from("ambient.cave")),
			Entry::new("block_search_extent", Payload::int(8)),
		]),
	)])
}

fn music(sound: &str, replace: bool, min: i32) -> Payload {
	let mut m = Payload::compound(vec![
		Entry::new("max_delay", Payload::int(24000)),
		Entry::new("sound", Payload::from(sound)),
		Entry::new("min_delay", Payload::int(min)),
	]);
	if replace {
		m.put("replace_current_music", Payload::byte(1));
	}
	m
}

fn truthy(p: &Payload, field: &str) -> bool {
	p.field(field)
		.and_then(Payload::as_i64)
		.is_some_and(|v| v != 0)
}

fn dimension_type(key: &str, tag: &mut Payload) {
	let mut attributes = Payload::compound(vec![]);
	match key.strip_prefix("minecraft:").unwrap_or(key) {
		"the_nether" => {
			tag.put("timelines", strings(&["villager_schedule"]));
			tag.put("skybox", Payload::from("none"));
			tag.put("cardinal_light", Payload::from("nether"));
			attribute(
				&mut attributes,
				"visual/sky_light_color",
				Payload::from("#7a7aff"),
			);
			attribute(
				&mut attributes,
				"visual/fog_start_distance",
				Payload::float(10.0),
			);
			attribute(
				&mut attributes,
				"visual/fog_end_distance",
				Payload::float(96.0),
			);
			attribute(
				&mut attributes,
				"gameplay/sky_light_level",
				Payload::float(4.0),
			);
		}
		"the_end" => {
			tag.put("timelines", strings(&["villager_schedule"]));
			tag.put("skybox", Payload::from("end"));
			attribute(
				&mut attributes,
				"visual/fog_color",
				Payload::from("#181318"),
			);
			attribute(
				&mut attributes,
				"visual/sky_color",
				Payload::from("#000000"),
			);
			attribute(
				&mut attributes,
				"visual/sky_light_color",
				Payload::from("#ac60cd"),
			);
			attribute(&mut attributes, "audio/ambient_sounds", mood());
			attribute(
				&mut attributes,
				"audio/background_music",
				Payload::compound(vec![Entry::new("default", music("music.end", true, 6000))]),
			);
		}
		_ => {
			tag.put("timelines", strings(&["day", "moon", "early_game"]));
			attribute(
				&mut attributes,
				"visual/fog_color",
				Payload::from("#c0d8ff"),
			);
			attribute(
				&mut attributes,
				"visual/sky_color",
				Payload::from("#78a7ff"),
			);
			attribute(&mut attributes, "audio/ambient_sounds", mood());
			attribute(
				&mut attributes,
				"audio/background_music",
				Payload::compound(vec![
					Entry::new("default", music("music.game", false, 12000)),
					Entry::new("creative", music("music.creative", false, 12000)),
				]),
			);
		}
	}
	if tag.take("fixed_time").is_some() {
		tag.put("has_fixed_time", Payload::byte(1));
	}
	if !truthy(tag, "natural") {
		attribute(
			&mut attributes,
			"visual/sky_light_factor",
			Payload::float(0.0),
		);
	}
	let ultrawarm = truthy(tag, "ultrawarm");
	if ultrawarm {
		attribute(
			&mut attributes,
			"visual/default_dripstone_particle",
			Payload::compound(vec![Entry::new(
				"type",
				Payload::from("dripping_dripstone_lava"),
			)]),
		);
	}
	if let Some(height) = tag.field("cloud_height").and_then(Payload::as_i64) {
		attribute(
			&mut attributes,
			"visual/cloud_color",
			Payload::from("#ccffffff"),
		);
		attribute(
			&mut attributes,
			"visual/cloud_height",
			Payload::float(height as f32),
		);
	}
	copy(
		tag,
		"has_raids",
		&mut attributes,
		"gameplay/can_start_raid",
		Some(Payload::byte(1)),
	);
	let zombify = tag
		.field("piglin_safe")
		.and_then(Payload::as_i64)
		.is_some_and(|v| v == 0);
	attribute(
		&mut attributes,
		"gameplay/piglins_zombify",
		Payload::byte(zombify as i8),
	);
	copy(
		tag,
		"respawn_anchor_works",
		&mut attributes,
		"gameplay/respawn_anchor_works",
		Some(Payload::byte(1)),
	);
	attribute(
		&mut attributes,
		"gameplay/fast_lava",
		Payload::byte(ultrawarm as i8),
	);
	attribute(
		&mut attributes,
		"gameplay/water_evaporates",
		Payload::byte(ultrawarm as i8),
	);
	tag.put("attributes", attributes);
}

const NO_SKY_COLOR: &[&str] = &[
	"warped_forest",
	"basalt_deltas",
	"nether_wastes",
	"soul_sand_valley",
	"crimson_forest",
];

fn biome(key: &str, tag: &mut Payload) {
	let key = key.strip_prefix("minecraft:").unwrap_or(key);
	let effects = tag
		.field("effects")
		.cloned()
		.unwrap_or_else(|| Payload::compound(vec![]));
	let mut attributes = Payload::compound(vec![]);
	if !NO_SKY_COLOR.contains(&key) {
		copy(
			&effects,
			"sky_color",
			&mut attributes,
			"visual/sky_color",
			Some(Payload::int(0)),
		);
	}
	if key != "the_end" {
		copy(
			&effects,
			"water_fog_color",
			&mut attributes,
			"visual/water_fog_color",
			Some(Payload::int(-16448205)),
		);
		copy(
			&effects,
			"fog_color",
			&mut attributes,
			"visual/fog_color",
			Some(Payload::int(0)),
		);
	}
	if let Some(Payload::list(nbt::List {
		items: nbt::ListItems::compound(entries),
		..
	})) = effects.field("music")
	{
		if let Some(data) = entries.first().and_then(|e| e.field("data")) {
			let mut music = Payload::compound(vec![]);
			for field in ["max_delay", "min_delay", "sound"] {
				copy(data, field, &mut music, field, None);
			}
			attribute(
				&mut attributes,
				"audio/background_music",
				Payload::compound(vec![Entry::new("default", music)]),
			);
		}
	}
	let mut ambient = Payload::compound(vec![]);
	copy(&effects, "mood_sound", &mut ambient, "mood", None);
	copy(&effects, "additions_sound", &mut ambient, "additions", None);
	copy(&effects, "ambient_sound", &mut ambient, "loop", None);
	if !matches!(&ambient, Payload::compound(e) if e.is_empty()) {
		attribute(&mut attributes, "audio/ambient_sounds", ambient);
	}
	if let Some(particle) = effects.field("particle") {
		let mut entry = Payload::compound(vec![]);
		copy(particle, "probability", &mut entry, "probability", None);
		copy(particle, "options", &mut entry, "particle", None);
		let Payload::compound(entry) = entry else {
			unreachable!("compound")
		};
		attribute(
			&mut attributes,
			"visual/ambient_particles",
			Payload::list(List::new(nbt::ListItems::compound(vec![entry]))),
		);
	}
	tag.put("attributes", attributes);
}

pub fn late_registry_data_up_1_21_11(
	mut p: a::configuration_toClient_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_toClient_packet::Ty>> {
	shared::count_biomes(&p, cx);
	shared::registry_entries(&mut p, "dimension_type", dimension_type);
	shared::registry_entries(&mut p, "worldgen/biome", biome);
	Ok(vec![b::configuration_toClient_packet::Ty::registry_data(p)])
}

pub fn late_finish_configuration_up_1_21_11(
	p: a::configuration_toClient_packet_finish_configuration::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::configuration_toClient_packet::Ty>> {
	let mut out = Vec::new();
	for id in ["zombie_nautilus_variant", "timeline"] {
		let entries = registries::vanilla(Version::V1_21_11, &format!("minecraft:{id}"))?;
		out.push(b::configuration_toClient_packet::Ty::registry_data(
			shared::registry(id, entries),
		));
	}
	out.push(b::configuration_toClient_packet::Ty::finish_configuration(
		p,
	));
	Ok(out)
}

pub fn late_game_time_down_1_21_9(
	v: &b::play_toClient_packet_update_time::Ty,
	cx: &mut Session,
) -> Result<i64> {
	cx.state::<GameTime>().0 = v.age;
	Ok(v.age)
}

fn color(p: &Payload) -> Payload {
	match p {
		Payload::string(s) => {
			Payload::int(u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0) as i32)
		}
		Payload::list(l) => {
			let channels: Vec<f32> = l
				.values()
				.iter()
				.map(|c| match c {
					Payload::float(f) => *f,
					Payload::double(d) => *d as f32,
					_ => 0.0,
				})
				.collect();
			let at =
				|i: usize| (channels.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 255.0) as i32;
			Payload::int((0xFF00_0000u32 as i32) | at(0) << 16 | at(1) << 8 | at(2))
		}
		other => other.clone(),
	}
}

fn attribute_of<'p>(attributes: Option<&'p Payload>, key: &str) -> Option<&'p Payload> {
	let attributes = attributes?;
	attributes
		.field(key)
		.or_else(|| attributes.field(&format!("minecraft:{key}")))
}

fn move_attribute(
	tag: &mut Payload,
	attributes: Option<&Payload>,
	key: &str,
	field: &str,
	map: impl Fn(&Payload) -> Payload,
	default: Option<Payload>,
) {
	match (attribute_of(attributes, key), default) {
		(Some(v), _) => tag.put(field, map(v)),
		(None, Some(d)) => tag.put(field, d),
		(None, None) => {}
	}
}

fn dimension_type_down(key: &str, tag: &mut Payload) {
	let key = key.strip_prefix("minecraft:").unwrap_or(key);
	let (effects, natural) = match key {
		"the_nether" => ("minecraft:the_nether", false),
		"the_end" => ("minecraft:the_end", false),
		_ => ("minecraft:overworld", true),
	};
	if tag.field("effects").is_none() {
		tag.put("effects", Payload::from(effects));
	}
	if tag.field("natural").is_none() {
		tag.put("natural", Payload::byte(natural as i8));
	}
	let attributes = tag.take("attributes");
	let attributes = attributes.as_ref();
	let same = |p: &Payload| p.clone();
	let cloud = |p: &Payload| match p {
		Payload::float(f) => Payload::int(*f as i32),
		Payload::double(d) => Payload::int(*d as i32),
		other => other.clone(),
	};
	let not = |p: &Payload| Payload::byte(matches!(p, Payload::byte(0)) as i8);
	move_attribute(
		tag,
		attributes,
		"visual/cloud_height",
		"cloud_height",
		cloud,
		None,
	);
	move_attribute(
		tag,
		attributes,
		"gameplay/can_start_raid",
		"has_raids",
		same,
		Some(Payload::byte(1)),
	);
	move_attribute(
		tag,
		attributes,
		"gameplay/piglins_zombify",
		"piglin_safe",
		not,
		Some(Payload::byte(0)),
	);
	move_attribute(
		tag,
		attributes,
		"gameplay/respawn_anchor_works",
		"respawn_anchor_works",
		same,
		Some(Payload::byte(0)),
	);
	move_attribute(
		tag,
		attributes,
		"gameplay/fast_lava",
		"ultrawarm",
		same,
		Some(Payload::byte(0)),
	);
	if tag.field("bed_works").is_none() {
		let works = attribute_of(attributes, "gameplay/bed_rule")
			.is_none_or(|r| truthy(r, "can_sleep") || truthy(r, "can_set_spawn"));
		tag.put("bed_works", Payload::byte(works as i8));
	}
	if tag
		.take("has_fixed_time")
		.is_some_and(|v| !matches!(v, Payload::byte(0)))
		&& tag.field("fixed_time").is_none()
	{
		tag.put(
			"fixed_time",
			Payload::long(if key == "the_nether" { 18000 } else { 6000 }),
		);
	}
	for field in ["timelines", "skybox", "cardinal_light"] {
		tag.take(field);
	}
}

fn biome_down(key: &str, tag: &mut Payload) {
	let key = key.strip_prefix("minecraft:").unwrap_or(key);
	let mut effects = tag
		.take("effects")
		.unwrap_or_else(|| Payload::compound(vec![]));
	for field in [
		"water_color",
		"foliage_color",
		"dry_foliage_color",
		"grass_color",
	] {
		if let Some(v) = effects.field(field).map(color) {
			effects.put(field, v);
		}
	}
	if effects.field("water_color").is_none() {
		effects.put("water_color", Payload::int(4159204));
	}
	let attributes = tag.take("attributes");
	let attributes = attributes.as_ref();
	let fog = if key == "the_end" { 10518688 } else { 12638463 };
	move_attribute(
		&mut effects,
		attributes,
		"visual/sky_color",
		"sky_color",
		color,
		Some(Payload::int(0)),
	);
	move_attribute(
		&mut effects,
		attributes,
		"visual/water_fog_color",
		"water_fog_color",
		color,
		Some(Payload::int(-16448205)),
	);
	move_attribute(
		&mut effects,
		attributes,
		"visual/fog_color",
		"fog_color",
		color,
		Some(Payload::int(fog)),
	);
	if let Some(default) =
		attribute_of(attributes, "audio/background_music").and_then(|m| m.field("default"))
	{
		let mut data =
			Payload::compound(vec![Entry::new("replace_current_music", Payload::byte(0))]);
		for field in ["max_delay", "min_delay", "sound", "replace_current_music"] {
			if let Some(v) = default.field(field) {
				data.put(field, v.clone());
			}
		}
		let entry = vec![
			Entry::new("data", data),
			Entry::new("weight", Payload::int(1)),
		];
		effects.put("music", List::new(nbt::ListItems::compound(vec![entry])));
	}
	if let Some(ambient) = attribute_of(attributes, "audio/ambient_sounds") {
		for (from, to) in [
			("loop", "ambient_sound"),
			("mood", "mood_sound"),
			("additions", "additions_sound"),
		] {
			if let Some(v) = ambient.field(from) {
				effects.put(to, v.clone());
			}
		}
	}
	if let Some(Payload::list(l)) = attribute_of(attributes, "visual/ambient_particles") {
		if let Some(first) = l.values().into_iter().next() {
			let mut particle = Payload::compound(vec![]);
			if let Some(p) = first.field("probability") {
				particle.put("probability", p.clone());
			}
			if let Some(p) = first.field("particle") {
				particle.put("options", p.clone());
			}
			effects.put("particle", particle);
		}
	}
	tag.put("effects", effects);
}

const REMOVED_EFFECTS: &[&str] = &["apply_exhaustion", "apply_impulse"];

fn strip_effects(p: &mut Payload) {
	match p {
		Payload::compound(entries) => {
			let removed = entries.iter().any(|e| {
				e.name == "type" && matches!(&e.value, Payload::string(t) if REMOVED_EFFECTS.contains(&t.strip_prefix("minecraft:").unwrap_or(t)))
			});
			if removed {
				*p = Payload::compound(vec![
					Entry::new("type", Payload::from("minecraft:all_of")),
					Entry::new("effects", List::new(nbt::ListItems::compound(Vec::new()))),
				]);
				return;
			}
			entries.retain(|e| {
				e.name.strip_prefix("minecraft:").unwrap_or(&e.name) != "post_piercing_attack"
			});
			for e in entries.iter_mut() {
				strip_effects(&mut e.value_mut());
			}
		}
		Payload::list(l) => {
			let mut items = l.values();
			items.iter_mut().for_each(strip_effects);
			if let nbt::ListItems::compound(_) = l.items {
				*p = Payload::list(List::new(nbt::ListItems::compound(
					items
						.into_iter()
						.map(|i| match i {
							Payload::compound(c) => c,
							_ => Vec::new(),
						})
						.collect(),
				)));
			}
		}
		_ => {}
	}
}

pub fn late_registry_data_down_1_21_9(
	mut p: b::configuration_toClient_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<a::configuration_toClient_packet::Ty>> {
	shared::count_biomes(&p, cx);
	if matches!(
		p.id.strip_prefix("minecraft:").unwrap_or(&p.id),
		"zombie_nautilus_variant" | "timeline"
	) {
		return Ok(Vec::new());
	}
	shared::registry_entries(&mut p, "dimension_type", dimension_type_down);
	shared::registry_entries(&mut p, "worldgen/biome", biome_down);
	shared::registry_entries(&mut p, "enchantment", |_, e| {
		if let Some(mut effects) = e.take("effects") {
			strip_effects(&mut effects);
			e.put("effects", effects);
		}
	});
	Ok(vec![a::configuration_toClient_packet::Ty::registry_data(p)])
}
