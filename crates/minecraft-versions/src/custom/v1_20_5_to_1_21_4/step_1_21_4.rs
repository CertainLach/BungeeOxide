use super::step_1_21_3::{block_set, chunk, records};
use super::*;
use crate::gen::convert::{v1_21_3 as b, v1_21_4 as c};

const FROM: Version = Version::V1_21_3;
const TO: Version = Version::V1_21_4;

type ConfigB = b::configuration_toClient_packet::Ty;
type ConfigC = c::configuration_toClient_packet::Ty;
type ClientB = b::play_toClient_packet::Ty;
type ClientC = c::play_toClient_packet::Ty;
type ServerC = c::play_toServer_packet::Ty;
type ComponentB = b::SlotComponent::Ty;
type ComponentC = c::SlotComponent::Ty;
type ParticleB = b::Particle::Ty;
type ParticleC = c::Particle::Ty;
type MetaB = b::entityMetadataEntry__value::Ty;
type MetaC = c::entityMetadataEntry__value::Ty;
type DisplayB = b::play_toClient_SlotDisplay::Ty;
type DisplayC = c::play_toClient_SlotDisplay::Ty;
type BlockSet = b::ItemBlockPredicate__blockSet__items::Ty;

const TRAIL_TICKS: i32 = 30;
const SWORDS: &[&str] = &[
	"wooden_sword",
	"stone_sword",
	"iron_sword",
	"golden_sword",
	"diamond_sword",
	"netherite_sword",
];

fn equipment_up(slots: Vec<b::Slot::Ty>, cx: &mut Session) -> Result<Vec<c::Slot::Ty>> {
	if slots.is_empty() {
		return Ok(vec![]);
	}
	let packet = ClientB::entity_equipment(b::play_toClient_packet_entity_equipment::Ty {
		entityId: 0,
		equipments: slots
			.into_iter()
			.map(
				|item| b::play_toClient_packet_entity_equipment__equipments__items::Ty {
					slot: 0,
					item,
				},
			)
			.collect(),
	});
	let mut slots: Vec<c::Slot::Ty> = carry(
		Root::PlayToClient,
		(FROM, TO),
		&packet,
		b::play_toClient_packet::serialize,
		c::play_toClient_packet::deserialize,
		|p| match p {
			ClientC::entity_equipment(e) => {
				Some(e.equipments.into_iter().map(|e| e.item).collect())
			}
			_ => None,
		},
		cx,
	)?;
	if legacy_combat(cx) {
		slots.iter_mut().for_each(blocking);
	}
	Ok(slots)
}

fn equipment_down(slots: Vec<c::Slot::Ty>, cx: &mut Session) -> Result<Vec<b::Slot::Ty>> {
	if slots.is_empty() {
		return Ok(vec![]);
	}
	let packet = ClientC::entity_equipment(c::play_toClient_packet_entity_equipment::Ty {
		entityId: 0,
		equipments: slots
			.into_iter()
			.map(
				|item| c::play_toClient_packet_entity_equipment__equipments__items::Ty {
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
		c::play_toClient_packet::serialize,
		b::play_toClient_packet::deserialize,
		|p| match p {
			ClientB::entity_equipment(e) => {
				Some(e.equipments.into_iter().map(|e| e.item).collect())
			}
			_ => None,
		},
		cx,
	)
}

fn slot_up(slot: b::Slot::Ty, cx: &mut Session) -> Result<c::Slot::Ty> {
	equipment_up(vec![slot], cx)?.pop().ok_or(Error::Dropped)
}

fn slot_down(slot: c::Slot::Ty, cx: &mut Session) -> Result<b::Slot::Ty> {
	equipment_down(vec![slot], cx)?.pop().ok_or(Error::Dropped)
}

fn blocking(slot: &mut c::Slot::Ty) {
	let c::Slot::Ty::default(_, s) = slot else {
		return;
	};
	let sword = registry::names(TO, "items")
		.and_then(|n| n.name(u32::try_from(s.itemId).ok()?))
		.is_some_and(|n| SWORDS.contains(&n));
	if !sword
		|| s.components
			.iter()
			.any(|c| matches!(c, ComponentC::consumable(_)))
	{
		return;
	}
	s.components
		.push(ComponentC::consumable(c::SlotComponent__consumable::Ty {
			consume_seconds: 3600.0,
			animation: c::SlotComponent__consumable__animation::Ty::block,
			sound: SoundHolder::Inline(v1_21_1::ItemSoundEvent::Ty {
				soundName: "minecraft:intentionally_empty".to_owned(),
				fixedRange: None,
			}),
			makes_particles: false,
			effects: vec![],
		}));
	s.addedComponentCount = s.components.len() as i32;
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

fn registry_fix_down(registry: &str, tag: &mut pds_Tag::Ty) {
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
		fields.push(Entry::new("item_model_index", Payload::float(index)));
	}
}

pub fn tricky_registry_data_up_1_21_4(p: RegistryData, cx: &mut Session) -> Result<Vec<ConfigC>> {
	Ok(vec![ConfigC::registry_data(registry_up(
		cx,
		(FROM, TO),
		p,
		wrap_music,
	))])
}

pub fn tricky_registry_data_down_1_21_3(p: RegistryData, cx: &mut Session) -> Result<Vec<ConfigB>> {
	Ok(vec![ConfigB::registry_data(registry_down(
		cx,
		(TO, FROM),
		p,
		registry_fix_down,
	)?)])
}

pub fn tricky_finish_configuration_up_1_21_4(
	p: b::configuration_toClient_packet_finish_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<ConfigC>> {
	let mut out: Vec<ConfigC> = missing_registries(cx, TO, wrap_music)
		.into_iter()
		.map(ConfigC::registry_data)
		.collect();
	out.push(ConfigC::finish_configuration(p));
	Ok(out)
}

pub fn tricky_feature_flags_up_1_21_4(p: Vec<String>, _cx: &mut Session) -> Result<Vec<ConfigC>> {
	Ok(vec![ConfigC::feature_flags(features(
		p,
		&[],
		&["winter_drop"],
	))])
}

pub fn tricky_feature_flags_down_1_21_3(p: Vec<String>, _cx: &mut Session) -> Result<Vec<ConfigB>> {
	Ok(vec![ConfigB::feature_flags(features(
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
	mut p: Vec<b::configuration_toClient_packet__tags__items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<ConfigC>> {
	for g in &mut p {
		remap_tags(FROM, TO, &g.tagType, &mut g.tags);
	}
	Ok(vec![ConfigC::tags(p)])
}

pub fn tricky_config_tags_down_1_21_3(
	mut p: Vec<b::configuration_toClient_packet__tags__items::Ty>,
	cx: &mut Session,
) -> Result<Vec<ConfigB>> {
	for g in &mut p {
		store_item_tags(cx, TO, &g.tagType, &g.tags);
		tags_down(&g.tagType, &mut g.tags);
	}
	Ok(vec![ConfigB::tags(p)])
}

pub fn tricky_play_tags_up_1_21_4(
	mut p: Vec<b::play_toClient_packet__tags__items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<ClientC>> {
	for g in &mut p {
		remap_tags(FROM, TO, &g.tagType, &mut g.tags);
	}
	Ok(vec![ClientC::tags(p)])
}

pub fn tricky_play_tags_down_1_21_3(
	mut p: Vec<b::play_toClient_packet__tags__items::Ty>,
	cx: &mut Session,
) -> Result<Vec<ClientB>> {
	for g in &mut p {
		store_item_tags(cx, TO, &g.tagType, &g.tags);
		tags_down(&g.tagType, &mut g.tags);
	}
	Ok(vec![ClientB::tags(p)])
}

pub fn tricky_item_id_up_1_21_4(src: &b::Slot__default::Ty, _cx: &mut Session) -> Result<i32> {
	Ok(item_up(src.itemId))
}

pub fn tricky_item_id_down_1_21_3(src: &c::Slot__default::Ty, _cx: &mut Session) -> Result<i32> {
	Ok(item_down(src.itemId))
}

fn kept_down(c: &ComponentC) -> bool {
	!matches!(c, ComponentC::custom_model_data(d) if d.floats.is_empty())
}

pub fn tricky_added_components_down_1_21_3(
	src: &c::Slot__default::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(src.components.iter().filter(|c| kept_down(c)).count() as i32)
}

pub fn tricky_trade_components_down_1_21_3(
	src: &c::play_toClient_packet_trade_list__trades__items__inputItem1::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(src.components.iter().filter(|c| kept_down(c)).count() as i32)
}

pub fn tricky_trade_components2_down_1_21_3(
	src: &c::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(src.components.iter().filter(|c| kept_down(c)).count() as i32)
}

pub fn tricky_custom_model_data_up_1_21_4(v: i32, _cx: &mut Session) -> Result<ComponentC> {
	Ok(ComponentC::custom_model_data(
		c::SlotComponent__custom_model_data::Ty {
			floats: vec![v as f32],
			flags: vec![],
			strings: vec![],
			colors: vec![],
		},
	))
}

pub fn tricky_custom_model_data_down_1_21_3(
	v: c::SlotComponent__custom_model_data::Ty,
	_cx: &mut Session,
) -> Result<ComponentB> {
	v.floats
		.first()
		.map(|f| ComponentB::custom_model_data(*f as i32))
		.ok_or(Error::Dropped)
}

pub fn tricky_trade_item_up_1_21_4(
	src: &b::play_toClient_packet_trade_list__trades__items__inputItem1::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_up(src.itemId))
}

pub fn tricky_trade_item_down_1_21_3(
	src: &c::play_toClient_packet_trade_list__trades__items__inputItem1::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_down(src.itemId))
}

pub fn tricky_trade_item2_up_1_21_4(
	src: &b::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_up(src.itemId))
}

pub fn tricky_trade_item2_down_1_21_3(
	src: &c::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_down(src.itemId))
}

pub fn tricky_pot_decorations_up_1_21_4(p: Vec<i32>, _cx: &mut Session) -> Result<ComponentC> {
	Ok(ComponentC::pot_decorations(
		p.into_iter().map(item_up).collect(),
	))
}

pub fn tricky_pot_decorations_down_1_21_3(p: Vec<i32>, _cx: &mut Session) -> Result<ComponentB> {
	Ok(ComponentB::pot_decorations(
		p.into_iter().map(item_down).collect(),
	))
}

pub fn tricky_repairable_up_1_21_4(set: IdSet, _cx: &mut Session) -> Result<ComponentC> {
	Ok(ComponentC::repairable(items_up(set)))
}

pub fn tricky_repairable_down_1_21_3(set: IdSet, _cx: &mut Session) -> Result<ComponentB> {
	Ok(ComponentB::repairable(items_down(set)))
}

pub fn tricky_equippable_entities_up_1_21_4(
	src: &b::SlotComponent__equippable::Ty,
	_cx: &mut Session,
) -> Result<Option<IdSet>> {
	Ok(src
		.allowedEntities
		.clone()
		.map(|s| id_set(s, |id| remap("entities", FROM, TO, id))))
}

pub fn tricky_equippable_entities_down_1_21_3(
	src: &c::SlotComponent__equippable::Ty,
	_cx: &mut Session,
) -> Result<Option<IdSet>> {
	Ok(src
		.allowedEntities
		.clone()
		.map(|s| id_set(s, |id| remap("entities", TO, FROM, id))))
}

pub fn tricky_tool_blocks_up_1_21_4(
	src: &b::SlotComponent__tool__rules__items::Ty,
	_cx: &mut Session,
) -> Result<IdSet> {
	Ok(id_set(src.blocks.clone(), |id| block(FROM, TO, id)))
}

pub fn tricky_tool_blocks_down_1_21_3(
	src: &c::SlotComponent__tool__rules__items::Ty,
	_cx: &mut Session,
) -> Result<IdSet> {
	Ok(id_set(src.blocks.clone(), |id| block(TO, FROM, id)))
}

pub fn tricky_predicate_blocks_up_1_21_4(
	src: &b::ItemBlockPredicate::Ty,
	_cx: &mut Session,
) -> Result<Option<BlockSet>> {
	Ok(block_set(src.blockSet.clone(), |id| block(FROM, TO, id)))
}

pub fn tricky_predicate_blocks_down_1_21_3(
	src: &c::ItemBlockPredicate::Ty,
	_cx: &mut Session,
) -> Result<Option<BlockSet>> {
	Ok(block_set(src.blockSet.clone(), |id| block(TO, FROM, id)))
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
	block: tricky_particle_block_up_1_21_4, tricky_particle_block_down_1_21_3;
	block_marker: tricky_particle_block_marker_up_1_21_4, tricky_particle_block_marker_down_1_21_3;
	falling_dust: tricky_particle_falling_dust_up_1_21_4, tricky_particle_falling_dust_down_1_21_3;
	dust_pillar: tricky_particle_dust_pillar_up_1_21_4, tricky_particle_dust_pillar_down_1_21_3;
	block_crumble: tricky_particle_block_crumble_up_1_21_4, tricky_particle_block_crumble_down_1_21_3;
}

pub fn tricky_trail_duration_up_1_21_4(
	_src: &b::Particle__trail::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(TRAIL_TICKS)
}

fn slot_display_up(d: DisplayB, cx: &mut Session) -> Result<DisplayC> {
	Ok(match d {
		DisplayB::empty => DisplayC::empty,
		DisplayB::any_fuel => DisplayC::any_fuel,
		DisplayB::item(id) => DisplayC::item(item_up(id)),
		DisplayB::item_stack(s) => DisplayC::item_stack(slot_up(s, cx)?),
		DisplayB::tag(t) => DisplayC::tag(t),
		DisplayB::smithing_trim(t) => {
			DisplayC::smithing_trim(Box::new(c::play_toClient_SlotDisplay__smithing_trim::Ty {
				base: slot_display_up(t.base, cx)?,
				material: slot_display_up(t.material, cx)?,
				pattern: slot_display_up(t.pattern, cx)?,
			}))
		}
		DisplayB::with_remainder(r) => {
			DisplayC::with_remainder(Box::new(c::play_toClient_SlotDisplay__with_remainder::Ty {
				input: slot_display_up(r.input, cx)?,
				remainder: slot_display_up(r.remainder, cx)?,
			}))
		}
		DisplayB::composite(list) => DisplayC::composite(
			list.into_iter()
				.map(|d| slot_display_up(d, cx))
				.collect::<Result<_>>()?,
		),
		DisplayB::default(t) => DisplayC::default(t),
	})
}

fn slot_display_down(d: DisplayC, cx: &mut Session) -> Result<DisplayB> {
	Ok(match d {
		DisplayC::empty => DisplayB::empty,
		DisplayC::any_fuel => DisplayB::any_fuel,
		DisplayC::item(id) => DisplayB::item(item_down(id)),
		DisplayC::item_stack(s) => DisplayB::item_stack(slot_down(s, cx)?),
		DisplayC::tag(t) => DisplayB::tag(t),
		DisplayC::smithing_trim(t) => {
			DisplayB::smithing_trim(Box::new(b::play_toClient_SlotDisplay__smithing_trim::Ty {
				base: slot_display_down(t.base, cx)?,
				material: slot_display_down(t.material, cx)?,
				pattern: slot_display_down(t.pattern, cx)?,
			}))
		}
		DisplayC::with_remainder(r) => {
			DisplayB::with_remainder(Box::new(b::play_toClient_SlotDisplay__with_remainder::Ty {
				input: slot_display_down(r.input, cx)?,
				remainder: slot_display_down(r.remainder, cx)?,
			}))
		}
		DisplayC::composite(list) => DisplayB::composite(
			list.into_iter()
				.map(|d| slot_display_down(d, cx))
				.collect::<Result<_>>()?,
		),
		DisplayC::default(t) => DisplayB::default(t),
	})
}

pub fn tricky_slot_display_up_1_21_4(d: DisplayB, cx: &mut Session) -> Result<DisplayC> {
	slot_display_up(d, cx)
}

pub fn tricky_slot_display_down_1_21_3(d: DisplayC, cx: &mut Session) -> Result<DisplayB> {
	slot_display_down(d, cx)
}

pub fn tricky_property_set_up_1_21_4(
	src: &b::play_toClient_packet_declare_recipes__recipes__items::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(src
		.items
		.iter()
		.filter_map(|i| remap("items", FROM, TO, *i))
		.collect())
}

pub fn tricky_property_set_down_1_21_3(
	src: &c::play_toClient_packet_declare_recipes__recipes__items::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(src
		.items
		.iter()
		.filter_map(|i| remap("items", TO, FROM, *i))
		.collect())
}

pub fn tricky_stonecutter_input_up_1_21_4(
	src: &b::play_toClient_packet_declare_recipes__stoneCutterRecipes__items::Ty,
	_cx: &mut Session,
) -> Result<IdSet> {
	Ok(items_up(src.input.clone()))
}

pub fn tricky_stonecutter_input_down_1_21_3(
	src: &c::play_toClient_packet_declare_recipes__stoneCutterRecipes__items::Ty,
	_cx: &mut Session,
) -> Result<IdSet> {
	Ok(items_down(src.input.clone()))
}

pub fn tricky_requirements_up_1_21_4(
	src: &b::play_toClient_packet_recipe_book_add__entries__items__recipe::Ty,
	_cx: &mut Session,
) -> Result<Option<Vec<IdSet>>> {
	Ok(src
		.craftingRequirements
		.clone()
		.map(|r| r.into_iter().map(items_up).collect()))
}

pub fn tricky_requirements_down_1_21_3(
	src: &c::play_toClient_packet_recipe_book_add__entries__items__recipe::Ty,
	_cx: &mut Session,
) -> Result<Option<Vec<IdSet>>> {
	Ok(src
		.craftingRequirements
		.clone()
		.map(|r| r.into_iter().map(items_down).collect()))
}

pub fn tricky_meta_entity_up_1_21_4(
	src: &b::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	metadata_of(cx, FROM, src.entityId);
	Ok(src.entityId)
}

pub fn tricky_meta_entity_down_1_21_3(
	src: &c::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	metadata_of(cx, TO, src.entityId);
	Ok(src.entityId)
}

pub fn tricky_meta_key_up_1_21_4(src: &b::entityMetadataEntry::Ty, cx: &mut Session) -> Result<u8> {
	let key = src.key;
	Ok(match metadata_key(cx, FROM, key) {
		Some("creaking" | "creaking_transient") if key >= 18 => key + 1,
		_ => key,
	})
}

pub fn tricky_meta_key_down_1_21_3(
	src: &c::entityMetadataEntry::Ty,
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
			MetaC::int(SALMON_SIZES.iter().position(|s| *s == v).unwrap_or(1) as i32)
		}
		_ => MetaC::string(v),
	})
}

pub fn tricky_meta_int_up_1_21_4(v: i32, cx: &mut Session) -> Result<MetaC> {
	Ok(MetaC::int(match current_entry(cx, FROM) {
		(ty, 11) if is_minecart(ty) => state_up(v),
		_ => v,
	}))
}

pub fn tricky_meta_int_down_1_21_3(v: i32, cx: &mut Session) -> Result<MetaB> {
	Ok(match current_entry(cx, TO) {
		(Some("salmon"), 17) => MetaB::string(
			SALMON_SIZES
				.get(v as usize)
				.unwrap_or(&"medium")
				.to_string(),
		),
		(ty, 11) if is_minecart(ty) => MetaB::int(state_down(v)),
		_ => MetaB::int(v),
	})
}

pub fn tricky_meta_block_up_1_21_4(state: i32, _cx: &mut Session) -> Result<MetaC> {
	Ok(MetaC::block_state(state_up(state)))
}

pub fn tricky_meta_block_down_1_21_3(state: i32, _cx: &mut Session) -> Result<MetaB> {
	Ok(MetaB::block_state(state_down(state)))
}

pub fn tricky_meta_optional_block_up_1_21_4(state: i32, _cx: &mut Session) -> Result<MetaC> {
	Ok(MetaC::optional_block_state(state_up(state)))
}

pub fn tricky_meta_optional_block_down_1_21_3(state: i32, _cx: &mut Session) -> Result<MetaB> {
	Ok(MetaB::optional_block_state(state_down(state)))
}

pub fn tricky_entity_type_up_1_21_4(
	src: &b::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<i32> {
	let id = remap("entities", FROM, TO, src.r#type).ok_or(Error::Dropped)?;
	spawned(cx, FROM, src.entityId, entity_name(FROM, src.r#type));
	spawned(cx, TO, src.entityId, entity_name(TO, id));
	Ok(id)
}

pub fn tricky_entity_type_down_1_21_3(
	src: &c::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<i32> {
	let id = remap("entities", TO, FROM, src.r#type).ok_or(Error::Dropped)?;
	spawned(cx, TO, src.entityId, entity_name(TO, src.r#type));
	spawned(cx, FROM, src.entityId, entity_name(FROM, id));
	Ok(id)
}

pub fn tricky_object_data_up_1_21_4(
	src: &b::play_toClient_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(match entity_name(FROM, src.r#type) {
		Some("falling_block") => state_up(src.objectData),
		_ => src.objectData,
	})
}

pub fn tricky_object_data_down_1_21_3(
	src: &c::play_toClient_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(match entity_name(TO, src.r#type) {
		Some("falling_block") => state_down(src.objectData),
		_ => src.objectData,
	})
}

pub fn tricky_block_change_up_1_21_4(
	src: &b::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state_up(src.r#type))
}

pub fn tricky_block_change_down_1_21_3(
	src: &c::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state_down(src.r#type))
}

pub fn tricky_multi_block_up_1_21_4(
	src: &b::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(records(&src.records, state_up))
}

pub fn tricky_multi_block_down_1_21_3(
	src: &c::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(records(&src.records, state_down))
}

pub fn tricky_chunk_up_1_21_4(
	src: &b::play_toClient_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	chunk(FROM, TO, &src.chunkData)
}

pub fn tricky_chunk_down_1_21_3(
	src: &c::play_toClient_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	chunk(TO, FROM, &src.chunkData)
}

pub fn tricky_world_event_up_1_21_4(
	src: &b::play_toClient_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(if src.effectId == 2001 {
		state_up(src.data)
	} else {
		src.data
	})
}

pub fn tricky_world_event_down_1_21_3(
	src: &c::play_toClient_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(if src.effectId == 2001 {
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
	_src: &b::play_toClient_packet_player_info__data__items::Ty,
	_cx: &mut Session,
) -> Result<c::play_toClient_packet_player_info__data__items__showHat::Ty> {
	Ok(c::play_toClient_packet_player_info__data__items__showHat::Ty::default(false))
}

pub fn tricky_window_items_up_1_21_4(
	p: b::play_toClient_packet_window_items::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientC>> {
	let mut slots = p.items;
	slots.push(p.carriedItem);
	let mut slots = equipment_up(slots, cx)?;
	let carried = slots.pop().ok_or(Error::Dropped)?;
	Ok(vec![ClientC::window_items(
		c::play_toClient_packet_window_items::Ty {
			windowId: p.windowId,
			stateId: p.stateId,
			items: slots,
			carriedItem: carried,
		},
	)])
}

pub fn tricky_set_slot_up_1_21_4(
	p: b::play_toClient_packet_set_slot::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientC>> {
	Ok(vec![ClientC::set_slot(
		c::play_toClient_packet_set_slot::Ty {
			windowId: p.windowId,
			stateId: p.stateId,
			slot: p.slot,
			item: slot_up(p.item, cx)?,
		},
	)])
}

pub fn tricky_set_cursor_item_up_1_21_4(
	item: b::Slot::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientC>> {
	Ok(vec![ClientC::set_cursor_item(slot_up(item, cx)?)])
}

pub fn tricky_set_player_inventory_up_1_21_4(
	p: b::play_toClient_packet_set_player_inventory::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientC>> {
	Ok(vec![ClientC::set_player_inventory(
		c::play_toClient_packet_set_player_inventory::Ty {
			slotId: p.slotId,
			contents: slot_up(p.contents, cx)?,
		},
	)])
}

fn player_loaded(cx: &mut Session) -> Result<()> {
	send(
		cx,
		Root::PlayToServer,
		TO,
		&ServerC::player_loaded(c::play_toServer_packet_player_loaded::Ty {}),
		c::play_toServer_packet::serialize,
	)
}

pub fn tricky_login_down_1_21_3(
	p: c::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<b::play_toClient_packet_login::Ty> {
	player_loaded(cx)?;
	Ok(p)
}

pub fn tricky_respawn_down_1_21_3(
	p: c::play_toClient_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<b::play_toClient_packet_respawn::Ty> {
	player_loaded(cx)?;
	Ok(p)
}
