use super::step_1_21_3::{block_set, chunk, records};
use super::*;
use crate::gen::convert::{v1_21_3 as b, v1_21_4 as c};

const FROM: Version = Version::V1_21_3;
const TO: Version = Version::V1_21_4;

type ConfigB = b::configuration_to_client_packet::Ty;
type ConfigC = c::configuration_to_client_packet::Ty;
type ClientB = b::play_to_client_packet::Ty;
type ClientC = c::play_to_client_packet::Ty;
type ServerC = c::play_to_server_packet::Ty;
type ComponentB = b::slot_component::Ty;
type ComponentC = c::slot_component::Ty;
type ParticleB = b::particle::Ty;
type ParticleC = c::particle::Ty;
type MetaB = b::entity_metadata_entry_value::Ty;
type MetaC = c::entity_metadata_entry_value::Ty;
type DisplayB = b::play_to_client_slot_display::Ty;
type DisplayC = c::play_to_client_slot_display::Ty;
type BlockSet = b::item_block_predicate_block_set_items::Ty;

const TRAIL_TICKS: i32 = 30;
const SWORDS: &[&str] = &[
	"wooden_sword",
	"stone_sword",
	"iron_sword",
	"golden_sword",
	"diamond_sword",
	"netherite_sword",
];

fn equipment_up(slots: Vec<b::slot::Ty>, cx: &mut Session) -> Result<Vec<c::slot::Ty>> {
	if slots.is_empty() {
		return Ok(vec![]);
	}
	let packet = ClientB::EntityEquipment(b::play_to_client_packet_entity_equipment::Ty {
		entity_id: 0,
		equipments: slots
			.into_iter()
			.map(
				|item| b::play_to_client_packet_entity_equipment_equipments_items::Ty {
					slot: 0,
					item,
				},
			)
			.collect(),
	});
	let mut slots: Vec<c::slot::Ty> = carry(
		Root::PlayToClient,
		(FROM, TO),
		&packet,
		b::play_to_client_packet::serialize,
		c::play_to_client_packet::deserialize,
		|p| match p {
			ClientC::EntityEquipment(e) => Some(e.equipments.into_iter().map(|e| e.item).collect()),
			_ => None,
		},
		cx,
	)?;
	if legacy_combat(cx) {
		slots.iter_mut().for_each(blocking);
	}
	Ok(slots)
}

fn equipment_down(slots: Vec<c::slot::Ty>, cx: &mut Session) -> Result<Vec<b::slot::Ty>> {
	if slots.is_empty() {
		return Ok(vec![]);
	}
	let packet = ClientC::EntityEquipment(c::play_to_client_packet_entity_equipment::Ty {
		entity_id: 0,
		equipments: slots
			.into_iter()
			.map(
				|item| c::play_to_client_packet_entity_equipment_equipments_items::Ty {
					slot: 0,
					item,
				},
			)
			.collect(),
	});
	carry(
		Root::PlayToClient,
		(TO, FROM),
		&packet,
		c::play_to_client_packet::serialize,
		b::play_to_client_packet::deserialize,
		|p| match p {
			ClientB::EntityEquipment(e) => Some(e.equipments.into_iter().map(|e| e.item).collect()),
			_ => None,
		},
		cx,
	)
}

fn slot_up(slot: b::slot::Ty, cx: &mut Session) -> Result<c::slot::Ty> {
	equipment_up(vec![slot], cx)?.pop().ok_or(Error::Dropped)
}

fn slot_down(slot: c::slot::Ty, cx: &mut Session) -> Result<b::slot::Ty> {
	equipment_down(vec![slot], cx)?.pop().ok_or(Error::Dropped)
}

fn blocking(slot: &mut c::slot::Ty) {
	let c::slot::Ty::Default(_, s) = slot else {
		return;
	};
	let sword = registry::names(TO, "items")
		.and_then(|n| n.name(u32::try_from(s.item_id).ok()?))
		.is_some_and(|n| SWORDS.contains(&n));
	if !sword
		|| s.components
			.iter()
			.any(|c| matches!(c, ComponentC::Consumable(_)))
	{
		return;
	}
	s.components
		.push(ComponentC::Consumable(c::slot_component_consumable::Ty {
			consume_seconds: 3600.0,
			animation: c::slot_component_consumable_animation::Ty::Block,
			sound: SoundHolder::Inline(v1_21_1::item_sound_event::Ty {
				sound_name: "minecraft:intentionally_empty".to_owned(),
				fixed_range: None,
			}),
			makes_particles: false,
			effects: vec![],
		}));
	s.added_component_count = s.components.len() as i32;
}

fn item_up(id: i32) -> i32 {
	item(FROM, TO, id)
}

fn item_down(id: i32) -> i32 {
	item(TO, FROM, id)
}

fn state_up(id: i32) -> i32 {
	state(FROM, TO, id)
}

fn state_down(id: i32) -> i32 {
	state(TO, FROM, id)
}

fn items_up(set: IdSet) -> IdSet {
	id_set(set, |id| remap("items", FROM, TO, id))
}

fn items_down(set: IdSet) -> IdSet {
	id_set(set, |id| remap("items", TO, FROM, id))
}

fn trim_model_index(key: &str) -> f32 {
	match strip(key) {
		"copper" => 0.5,
		"diamond" => 0.8,
		"emerald" => 0.7,
		"gold" | "resin" => 0.6,
		"iron" => 0.2,
		"lapis" => 0.9,
		"netherite" => 0.3,
		"quartz" => 0.1,
		"redstone" => 0.4,
		_ => 1.0,
	}
}

fn registry_fix_down(registry: &str, tag: &mut Tag) {
	unwrap_music(registry, tag);
	if strip(registry) != "trim_material" {
		return;
	}
	let Some(fields) = tag.value.as_compound_mut() else {
		return;
	};
	let asset = fields
		.field("asset_name")
		.and_then(Payload::as_str)
		.map(str::to_owned);
	if asset.as_deref() == Some("resin") {
		fields.put("asset_name", "gold");
	}
	let older = fields
		.field("ingredient")
		.and_then(Payload::as_str)
		.and_then(|name| item_id(TO, name))
		.map(item_down)
		.and_then(|i| item_name(FROM, i));
	if let Some(older) = older {
		fields.put("ingredient", older);
	}
	if fields.field("item_model_index").is_none() {
		let index = trim_model_index(asset.as_deref().unwrap_or_default());
		fields.push(Entry::new("item_model_index", Payload::Float(index)));
	}
}

pub fn tricky_registry_data_up_1_21_4(p: RegistryData, cx: &mut Session) -> Result<Vec<ConfigC>> {
	Ok(vec![ConfigC::RegistryData(registry_up(
		cx,
		(FROM, TO),
		p,
		wrap_music,
	))])
}

pub fn tricky_registry_data_down_1_21_3(p: RegistryData, cx: &mut Session) -> Result<Vec<ConfigB>> {
	Ok(vec![ConfigB::RegistryData(registry_down(
		cx,
		(TO, FROM),
		p,
		registry_fix_down,
	)?)])
}

pub fn tricky_finish_configuration_up_1_21_4(
	p: b::configuration_to_client_packet_finish_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<ConfigC>> {
	let mut out: Vec<ConfigC> = missing_registries(cx, TO, wrap_music)
		.into_iter()
		.map(ConfigC::RegistryData)
		.collect();
	out.push(ConfigC::FinishConfiguration(p));
	Ok(out)
}

pub fn tricky_feature_flags_up_1_21_4(p: Vec<String>, _cx: &mut Session) -> Result<Vec<ConfigC>> {
	Ok(vec![ConfigC::FeatureFlags(features(
		p,
		&[],
		&["winter_drop"],
	))])
}

pub fn tricky_feature_flags_down_1_21_3(p: Vec<String>, _cx: &mut Session) -> Result<Vec<ConfigB>> {
	Ok(vec![ConfigB::FeatureFlags(features(
		p,
		&["winter_drop"],
		&[],
	))])
}

fn tags_down(registry: &str, tags: &mut Vec<TagList>) {
	remap_tags(TO, FROM, registry, tags);
	match strip(registry) {
		"item" => ensure_tags(tags, &[("tall_flowers", vec![]), ("flowers", vec![])]),
		"block" => ensure_tags(tags, &[("tall_flowers", vec![])]),
		_ => {}
	}
}

pub fn tricky_config_tags_up_1_21_4(
	mut p: Vec<b::configuration_to_client_packet_tags_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<ConfigC>> {
	for g in &mut p {
		remap_tags(FROM, TO, &g.tag_type, &mut g.tags);
	}
	Ok(vec![ConfigC::Tags(p)])
}

pub fn tricky_config_tags_down_1_21_3(
	mut p: Vec<b::configuration_to_client_packet_tags_items::Ty>,
	cx: &mut Session,
) -> Result<Vec<ConfigB>> {
	for g in &mut p {
		store_item_tags(cx, TO, &g.tag_type, &g.tags);
		tags_down(&g.tag_type, &mut g.tags);
	}
	Ok(vec![ConfigB::Tags(p)])
}

pub fn tricky_play_tags_up_1_21_4(
	mut p: Vec<b::play_to_client_packet_tags_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<ClientC>> {
	for g in &mut p {
		remap_tags(FROM, TO, &g.tag_type, &mut g.tags);
	}
	Ok(vec![ClientC::Tags(p)])
}

pub fn tricky_play_tags_down_1_21_3(
	mut p: Vec<b::play_to_client_packet_tags_items::Ty>,
	cx: &mut Session,
) -> Result<Vec<ClientB>> {
	for g in &mut p {
		store_item_tags(cx, TO, &g.tag_type, &g.tags);
		tags_down(&g.tag_type, &mut g.tags);
	}
	Ok(vec![ClientB::Tags(p)])
}

pub fn tricky_item_id_up_1_21_4(src: &b::slot_default::Ty, _cx: &mut Session) -> Result<i32> {
	Ok(item_up(src.item_id))
}

pub fn tricky_item_id_down_1_21_3(src: &c::slot_default::Ty, _cx: &mut Session) -> Result<i32> {
	Ok(item_down(src.item_id))
}

fn kept_down(c: &ComponentC) -> bool {
	!matches!(c, ComponentC::CustomModelData(d) if d.floats.is_empty())
}

pub fn tricky_added_components_down_1_21_3(
	src: &c::slot_default::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(src.components.iter().filter(|c| kept_down(c)).count() as i32)
}

pub fn tricky_trade_components_down_1_21_3(
	src: &c::play_to_client_packet_trade_list_trades_items_input_item1::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(src.components.iter().filter(|c| kept_down(c)).count() as i32)
}

pub fn tricky_trade_components2_down_1_21_3(
	src: &c::play_to_client_packet_trade_list_trades_items_input_item2_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(src.components.iter().filter(|c| kept_down(c)).count() as i32)
}

pub fn tricky_custom_model_data_up_1_21_4(v: i32, _cx: &mut Session) -> Result<ComponentC> {
	Ok(ComponentC::CustomModelData(
		c::slot_component_custom_model_data::Ty {
			floats: vec![v as f32],
			flags: vec![],
			strings: vec![],
			colors: vec![],
		},
	))
}

pub fn tricky_custom_model_data_down_1_21_3(
	v: c::slot_component_custom_model_data::Ty,
	_cx: &mut Session,
) -> Result<ComponentB> {
	v.floats
		.first()
		.map(|f| ComponentB::CustomModelData(*f as i32))
		.ok_or(Error::Dropped)
}

pub fn tricky_trade_item_up_1_21_4(
	src: &b::play_to_client_packet_trade_list_trades_items_input_item1::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_up(src.item_id))
}

pub fn tricky_trade_item_down_1_21_3(
	src: &c::play_to_client_packet_trade_list_trades_items_input_item1::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_down(src.item_id))
}

pub fn tricky_trade_item2_up_1_21_4(
	src: &b::play_to_client_packet_trade_list_trades_items_input_item2_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_up(src.item_id))
}

pub fn tricky_trade_item2_down_1_21_3(
	src: &c::play_to_client_packet_trade_list_trades_items_input_item2_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_down(src.item_id))
}

pub fn tricky_pot_decorations_up_1_21_4(p: Vec<i32>, _cx: &mut Session) -> Result<ComponentC> {
	Ok(ComponentC::PotDecorations(
		p.into_iter().map(item_up).collect(),
	))
}

pub fn tricky_pot_decorations_down_1_21_3(p: Vec<i32>, _cx: &mut Session) -> Result<ComponentB> {
	Ok(ComponentB::PotDecorations(
		p.into_iter().map(item_down).collect(),
	))
}

pub fn tricky_repairable_up_1_21_4(set: IdSet, _cx: &mut Session) -> Result<ComponentC> {
	Ok(ComponentC::Repairable(items_up(set)))
}

pub fn tricky_repairable_down_1_21_3(set: IdSet, _cx: &mut Session) -> Result<ComponentB> {
	Ok(ComponentB::Repairable(items_down(set)))
}

pub fn tricky_equippable_entities_up_1_21_4(
	src: &b::slot_component_equippable::Ty,
	_cx: &mut Session,
) -> Result<Option<IdSet>> {
	Ok(src
		.allowed_entities
		.clone()
		.map(|s| id_set(s, |id| remap("entities", FROM, TO, id))))
}

pub fn tricky_equippable_entities_down_1_21_3(
	src: &c::slot_component_equippable::Ty,
	_cx: &mut Session,
) -> Result<Option<IdSet>> {
	Ok(src
		.allowed_entities
		.clone()
		.map(|s| id_set(s, |id| remap("entities", TO, FROM, id))))
}

pub fn tricky_tool_blocks_up_1_21_4(
	src: &b::slot_component_tool_rules_items::Ty,
	_cx: &mut Session,
) -> Result<IdSet> {
	Ok(id_set(src.blocks.clone(), |id| block(FROM, TO, id)))
}

pub fn tricky_tool_blocks_down_1_21_3(
	src: &c::slot_component_tool_rules_items::Ty,
	_cx: &mut Session,
) -> Result<IdSet> {
	Ok(id_set(src.blocks.clone(), |id| block(TO, FROM, id)))
}

pub fn tricky_predicate_blocks_up_1_21_4(
	src: &b::item_block_predicate::Ty,
	_cx: &mut Session,
) -> Result<Option<BlockSet>> {
	Ok(block_set(src.block_set.clone(), |id| block(FROM, TO, id)))
}

pub fn tricky_predicate_blocks_down_1_21_3(
	src: &c::item_block_predicate::Ty,
	_cx: &mut Session,
) -> Result<Option<BlockSet>> {
	Ok(block_set(src.block_set.clone(), |id| block(TO, FROM, id)))
}

macro_rules! block_particles {
	($($arm:ident: $up:ident, $down:ident;)*) => {$(
		pub fn $up(state: i32, _cx: &mut Session) -> Result<ParticleC> {
			Ok(ParticleC::$arm(state_up(state)))
		}

		pub fn $down(state: i32, _cx: &mut Session) -> Result<ParticleB> {
			Ok(ParticleB::$arm(state_down(state)))
		}
	)*};
}

block_particles! {
	Block: tricky_particle_block_up_1_21_4, tricky_particle_block_down_1_21_3;
	BlockMarker: tricky_particle_block_marker_up_1_21_4, tricky_particle_block_marker_down_1_21_3;
	FallingDust: tricky_particle_falling_dust_up_1_21_4, tricky_particle_falling_dust_down_1_21_3;
	DustPillar: tricky_particle_dust_pillar_up_1_21_4, tricky_particle_dust_pillar_down_1_21_3;
	BlockCrumble: tricky_particle_block_crumble_up_1_21_4, tricky_particle_block_crumble_down_1_21_3;
}

pub fn tricky_trail_duration_up_1_21_4(
	_src: &b::particle_trail::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(TRAIL_TICKS)
}

fn slot_display_up(d: DisplayB, cx: &mut Session) -> Result<DisplayC> {
	Ok(match d {
		DisplayB::Empty => DisplayC::Empty,
		DisplayB::AnyFuel => DisplayC::AnyFuel,
		DisplayB::Item(id) => DisplayC::Item(item_up(id)),
		DisplayB::ItemStack(s) => DisplayC::ItemStack(slot_up(s, cx)?),
		DisplayB::Tag(t) => DisplayC::Tag(t),
		DisplayB::SmithingTrim(t) => {
			DisplayC::SmithingTrim(Box::new(c::play_to_client_slot_display_smithing_trim::Ty {
				base: slot_display_up(t.base, cx)?,
				material: slot_display_up(t.material, cx)?,
				pattern: slot_display_up(t.pattern, cx)?,
			}))
		}
		DisplayB::WithRemainder(r) => DisplayC::WithRemainder(Box::new(
			c::play_to_client_slot_display_with_remainder::Ty {
				input: slot_display_up(r.input, cx)?,
				remainder: slot_display_up(r.remainder, cx)?,
			},
		)),
		DisplayB::Composite(list) => DisplayC::Composite(
			list.into_iter()
				.map(|d| slot_display_up(d, cx))
				.collect::<Result<_>>()?,
		),
		DisplayB::Default(t) => DisplayC::Default(t),
	})
}

fn slot_display_down(d: DisplayC, cx: &mut Session) -> Result<DisplayB> {
	Ok(match d {
		DisplayC::Empty => DisplayB::Empty,
		DisplayC::AnyFuel => DisplayB::AnyFuel,
		DisplayC::Item(id) => DisplayB::Item(item_down(id)),
		DisplayC::ItemStack(s) => DisplayB::ItemStack(slot_down(s, cx)?),
		DisplayC::Tag(t) => DisplayB::Tag(t),
		DisplayC::SmithingTrim(t) => {
			DisplayB::SmithingTrim(Box::new(b::play_to_client_slot_display_smithing_trim::Ty {
				base: slot_display_down(t.base, cx)?,
				material: slot_display_down(t.material, cx)?,
				pattern: slot_display_down(t.pattern, cx)?,
			}))
		}
		DisplayC::WithRemainder(r) => DisplayB::WithRemainder(Box::new(
			b::play_to_client_slot_display_with_remainder::Ty {
				input: slot_display_down(r.input, cx)?,
				remainder: slot_display_down(r.remainder, cx)?,
			},
		)),
		DisplayC::Composite(list) => DisplayB::Composite(
			list.into_iter()
				.map(|d| slot_display_down(d, cx))
				.collect::<Result<_>>()?,
		),
		DisplayC::Default(t) => DisplayB::Default(t),
	})
}

pub fn tricky_slot_display_up_1_21_4(d: DisplayB, cx: &mut Session) -> Result<DisplayC> {
	slot_display_up(d, cx)
}

pub fn tricky_slot_display_down_1_21_3(d: DisplayC, cx: &mut Session) -> Result<DisplayB> {
	slot_display_down(d, cx)
}

pub fn tricky_property_set_up_1_21_4(
	src: &b::play_to_client_packet_declare_recipes_recipes_items::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(src
		.items
		.iter()
		.filter_map(|i| remap("items", FROM, TO, *i))
		.collect())
}

pub fn tricky_property_set_down_1_21_3(
	src: &c::play_to_client_packet_declare_recipes_recipes_items::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(src
		.items
		.iter()
		.filter_map(|i| remap("items", TO, FROM, *i))
		.collect())
}

pub fn tricky_stonecutter_input_up_1_21_4(
	src: &b::play_to_client_packet_declare_recipes_stone_cutter_recipes_items::Ty,
	_cx: &mut Session,
) -> Result<IdSet> {
	Ok(items_up(src.input.clone()))
}

pub fn tricky_stonecutter_input_down_1_21_3(
	src: &c::play_to_client_packet_declare_recipes_stone_cutter_recipes_items::Ty,
	_cx: &mut Session,
) -> Result<IdSet> {
	Ok(items_down(src.input.clone()))
}

pub fn tricky_requirements_up_1_21_4(
	src: &b::play_to_client_packet_recipe_book_add_entries_items_recipe::Ty,
	_cx: &mut Session,
) -> Result<Option<Vec<IdSet>>> {
	Ok(src
		.crafting_requirements
		.clone()
		.map(|r| r.into_iter().map(items_up).collect()))
}

pub fn tricky_requirements_down_1_21_3(
	src: &c::play_to_client_packet_recipe_book_add_entries_items_recipe::Ty,
	_cx: &mut Session,
) -> Result<Option<Vec<IdSet>>> {
	Ok(src
		.crafting_requirements
		.clone()
		.map(|r| r.into_iter().map(items_down).collect()))
}

pub fn tricky_meta_entity_up_1_21_4(
	src: &b::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	metadata_of(cx, FROM, src.entity_id);
	Ok(src.entity_id)
}

pub fn tricky_meta_entity_down_1_21_3(
	src: &c::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	metadata_of(cx, TO, src.entity_id);
	Ok(src.entity_id)
}

pub fn tricky_meta_key_up_1_21_4(
	src: &b::entity_metadata_entry::Ty,
	cx: &mut Session,
) -> Result<u8> {
	let key = src.key;
	Ok(match metadata_key(cx, FROM, key) {
		Some("creaking" | "creaking_transient") if key >= 18 => key + 1,
		_ => key,
	})
}

pub fn tricky_meta_key_down_1_21_3(
	src: &c::entity_metadata_entry::Ty,
	cx: &mut Session,
) -> Result<u8> {
	let key = src.key;
	match metadata_key(cx, TO, key) {
		Some("creaking") if key >= 18 => Err(Error::Dropped),
		_ => Ok(key),
	}
}

const SALMON_SIZES: &[&str] = &["small", "medium", "large"];

pub fn tricky_meta_string_up_1_21_4(v: String, cx: &mut Session) -> Result<MetaC> {
	Ok(match current_entry(cx, FROM) {
		(Some("salmon"), 17) => {
			MetaC::Int(SALMON_SIZES.iter().position(|s| *s == v).unwrap_or(1) as i32)
		}
		_ => MetaC::String(v),
	})
}

pub fn tricky_meta_int_up_1_21_4(v: i32, cx: &mut Session) -> Result<MetaC> {
	Ok(MetaC::Int(match current_entry(cx, FROM) {
		(ty, 11) if is_minecart(ty) => state_up(v),
		_ => v,
	}))
}

pub fn tricky_meta_int_down_1_21_3(v: i32, cx: &mut Session) -> Result<MetaB> {
	Ok(match current_entry(cx, TO) {
		(Some("salmon"), 17) => MetaB::String(
			SALMON_SIZES
				.get(v as usize)
				.unwrap_or(&"medium")
				.to_string(),
		),
		(ty, 11) if is_minecart(ty) => MetaB::Int(state_down(v)),
		_ => MetaB::Int(v),
	})
}

pub fn tricky_meta_block_up_1_21_4(state: i32, _cx: &mut Session) -> Result<MetaC> {
	Ok(MetaC::BlockState(state_up(state)))
}

pub fn tricky_meta_block_down_1_21_3(state: i32, _cx: &mut Session) -> Result<MetaB> {
	Ok(MetaB::BlockState(state_down(state)))
}

pub fn tricky_meta_optional_block_up_1_21_4(state: i32, _cx: &mut Session) -> Result<MetaC> {
	Ok(MetaC::OptionalBlockState(state_up(state)))
}

pub fn tricky_meta_optional_block_down_1_21_3(state: i32, _cx: &mut Session) -> Result<MetaB> {
	Ok(MetaB::OptionalBlockState(state_down(state)))
}

pub fn tricky_entity_type_up_1_21_4(
	src: &b::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<i32> {
	let id = remap("entities", FROM, TO, src.r#type).ok_or(Error::Dropped)?;
	spawned(cx, FROM, src.entity_id, entity_name(FROM, src.r#type));
	spawned(cx, TO, src.entity_id, entity_name(TO, id));
	Ok(id)
}

pub fn tricky_entity_type_down_1_21_3(
	src: &c::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<i32> {
	let id = remap("entities", TO, FROM, src.r#type).ok_or(Error::Dropped)?;
	spawned(cx, TO, src.entity_id, entity_name(TO, src.r#type));
	spawned(cx, FROM, src.entity_id, entity_name(FROM, id));
	Ok(id)
}

pub fn tricky_object_data_up_1_21_4(
	src: &b::play_to_client_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(match entity_name(FROM, src.r#type) {
		Some("falling_block") => state_up(src.object_data),
		_ => src.object_data,
	})
}

pub fn tricky_object_data_down_1_21_3(
	src: &c::play_to_client_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(match entity_name(TO, src.r#type) {
		Some("falling_block") => state_down(src.object_data),
		_ => src.object_data,
	})
}

pub fn tricky_block_change_up_1_21_4(
	src: &b::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state_up(src.r#type))
}

pub fn tricky_block_change_down_1_21_3(
	src: &c::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state_down(src.r#type))
}

pub fn tricky_multi_block_up_1_21_4(
	src: &b::play_to_client_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(records(&src.records, state_up))
}

pub fn tricky_multi_block_down_1_21_3(
	src: &c::play_to_client_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(records(&src.records, state_down))
}

pub fn tricky_chunk_up_1_21_4(
	src: &b::play_to_client_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	chunk(FROM, TO, &src.chunk_data)
}

pub fn tricky_chunk_down_1_21_3(
	src: &c::play_to_client_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	chunk(TO, FROM, &src.chunk_data)
}

pub fn tricky_world_event_up_1_21_4(
	src: &b::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(if src.effect_id == 2001 {
		state_up(src.data)
	} else {
		src.data
	})
}

pub fn tricky_world_event_down_1_21_3(
	src: &c::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(if src.effect_id == 2001 {
		state_down(src.data)
	} else {
		src.data
	})
}

pub fn tricky_sound_up_1_21_4(holder: SoundHolder, _cx: &mut Session) -> Result<SoundHolder> {
	Ok(sound(FROM, TO, holder))
}

pub fn tricky_sound_down_1_21_3(holder: SoundHolder, _cx: &mut Session) -> Result<SoundHolder> {
	Ok(sound(TO, FROM, holder))
}

pub fn tricky_show_hat_up_1_21_4(
	_src: &b::play_to_client_packet_player_info_data_items::Ty,
	_cx: &mut Session,
) -> Result<c::play_to_client_packet_player_info_data_items_show_hat::Ty> {
	Ok(c::play_to_client_packet_player_info_data_items_show_hat::Ty::Default(false))
}

pub fn tricky_window_items_up_1_21_4(
	p: b::play_to_client_packet_window_items::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientC>> {
	let mut slots = p.items;
	slots.push(p.carried_item);
	let mut slots = equipment_up(slots, cx)?;
	let carried = slots.pop().ok_or(Error::Dropped)?;
	Ok(vec![ClientC::WindowItems(
		c::play_to_client_packet_window_items::Ty {
			window_id: p.window_id,
			state_id: p.state_id,
			items: slots,
			carried_item: carried,
		},
	)])
}

pub fn tricky_set_slot_up_1_21_4(
	p: b::play_to_client_packet_set_slot::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientC>> {
	Ok(vec![ClientC::SetSlot(
		c::play_to_client_packet_set_slot::Ty {
			window_id: p.window_id,
			state_id: p.state_id,
			slot: p.slot,
			item: slot_up(p.item, cx)?,
		},
	)])
}

pub fn tricky_set_cursor_item_up_1_21_4(
	item: b::slot::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientC>> {
	Ok(vec![ClientC::SetCursorItem(slot_up(item, cx)?)])
}

pub fn tricky_set_player_inventory_up_1_21_4(
	p: b::play_to_client_packet_set_player_inventory::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientC>> {
	Ok(vec![ClientC::SetPlayerInventory(
		c::play_to_client_packet_set_player_inventory::Ty {
			slot_id: p.slot_id,
			contents: slot_up(p.contents, cx)?,
		},
	)])
}

fn player_loaded(cx: &mut Session) -> Result<()> {
	send(
		cx,
		Root::PlayToServer,
		TO,
		&ServerC::PlayerLoaded(c::play_to_server_packet_player_loaded::Ty {}),
		c::play_to_server_packet::serialize,
	)
}

pub fn tricky_login_down_1_21_3(
	p: c::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<b::play_to_client_packet_login::Ty> {
	player_loaded(cx)?;
	Ok(p)
}

pub fn tricky_respawn_down_1_21_3(
	p: c::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<b::play_to_client_packet_respawn::Ty> {
	player_loaded(cx)?;
	Ok(p)
}
