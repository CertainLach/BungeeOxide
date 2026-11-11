use super::step_1_21_1::{attribute, ATTRIBUTES_1_21_1};
use super::*;
use crate::gen::convert::{v1_21_1 as a, v1_21_3 as b};

const FROM: Version = Version::V1_21_1;
const TO: Version = Version::V1_21_3;

type ConfigA = a::configuration_to_client_packet::Ty;
type ConfigB = b::configuration_to_client_packet::Ty;
type ClientA = a::play_to_client_packet::Ty;
type ClientB = b::play_to_client_packet::Ty;
type ServerA = a::play_to_server_packet::Ty;
type ServerB = b::play_to_server_packet::Ty;
type ComponentA = a::slot_component::Ty;
type ComponentB = b::slot_component::Ty;
type ParticleA = a::particle::Ty;
type ParticleB = b::particle::Ty;
type MetaA = a::entity_metadata_entry_value::Ty;
type MetaB = b::entity_metadata_entry_value::Ty;
type Flags = b::play_to_server_movement_flags::Ty;
type RecipeA = a::play_to_client_packet_declare_recipes_items::Ty;
type RecipeDataA = a::play_to_client_packet_declare_recipes_items_data::Ty;
type Display = b::play_to_client_slot_display::Ty;

pub(super) const ATTRIBUTES_1_21_3: &[&str] = &[
	"armor",
	"armor_toughness",
	"attack_damage",
	"attack_knockback",
	"attack_speed",
	"block_break_speed",
	"block_interaction_range",
	"burning_time",
	"explosion_knockback_resistance",
	"entity_interaction_range",
	"fall_damage_multiplier",
	"flying_speed",
	"follow_range",
	"gravity",
	"jump_strength",
	"knockback_resistance",
	"luck",
	"max_absorption",
	"max_health",
	"mining_efficiency",
	"movement_efficiency",
	"movement_speed",
	"oxygen_bonus",
	"safe_fall_distance",
	"scale",
	"sneaking_speed",
	"spawn_reinforcements",
	"step_height",
	"submerged_mining_speed",
	"sweeping_damage_ratio",
	"tempt_range",
	"water_movement_efficiency",
];

const CREAKING_HEART: i32 = 10;
const BOAT_WOODS: &[&str] = &[
	"oak", "spruce", "birch", "jungle", "acacia", "cherry", "dark_oak", "mangrove", "bamboo",
];
const GOAT_HORNS: &[&str] = &[
	"ponder_goat_horn",
	"sing_goat_horn",
	"seek_goat_horn",
	"feel_goat_horn",
	"admire_goat_horn",
	"call_goat_horn",
	"yearn_goat_horn",
	"dream_goat_horn",
];

fn equipment_up(slots: Vec<a::slot::Ty>, cx: &mut Session) -> Result<Vec<b::slot::Ty>> {
	if slots.is_empty() {
		return Ok(vec![]);
	}
	let packet = ClientA::EntityEquipment(a::play_to_client_packet_entity_equipment::Ty {
		entity_id: 0,
		equipments: slots
			.into_iter()
			.map(
				|item| a::play_to_client_packet_entity_equipment_equipments_items::Ty {
					slot: 0,
					item,
				},
			)
			.collect(),
	});
	carry(
		Root::PlayToClient,
		(FROM, TO),
		&packet,
		a::play_to_client_packet::serialize,
		b::play_to_client_packet::deserialize,
		|p| match p {
			ClientB::EntityEquipment(e) => Some(e.equipments.into_iter().map(|e| e.item).collect()),
			_ => None,
		},
		cx,
	)
}

fn equipment_down(slots: Vec<b::slot::Ty>, cx: &mut Session) -> Result<Vec<a::slot::Ty>> {
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
	carry(
		Root::PlayToClient,
		(TO, FROM),
		&packet,
		b::play_to_client_packet::serialize,
		a::play_to_client_packet::deserialize,
		|p| match p {
			ClientA::EntityEquipment(e) => Some(e.equipments.into_iter().map(|e| e.item).collect()),
			_ => None,
		},
		cx,
	)
}

fn slot_up(slot: a::slot::Ty, cx: &mut Session) -> Result<b::slot::Ty> {
	let source = slot.clone();
	let mut out = equipment_up(vec![slot], cx)?.pop().ok_or(Error::Dropped)?;
	split_food(&source, &mut out, cx)?;
	Ok(out)
}

fn slot_down(slot: b::slot::Ty, cx: &mut Session) -> Result<a::slot::Ty> {
	equipment_down(vec![slot], cx)?.pop().ok_or(Error::Dropped)
}

fn particle_up(particle: ParticleA, cx: &mut Session) -> Result<ParticleB> {
	let packet = ClientA::WorldParticles(a::play_to_client_packet_world_particles::Ty {
		long_distance: false,
		x: 0.0,
		y: 0.0,
		z: 0.0,
		offset_x: 0.0,
		offset_y: 0.0,
		offset_z: 0.0,
		velocity_offset: 0.0,
		amount: 0,
		particle,
	});
	carry(
		Root::PlayToClient,
		(FROM, TO),
		&packet,
		a::play_to_client_packet::serialize,
		b::play_to_client_packet::deserialize,
		|p| match p {
			ClientB::WorldParticles(w) => Some(w.particle),
			_ => None,
		},
		cx,
	)
}

fn particle_down(particle: ParticleB, cx: &mut Session) -> Result<ParticleA> {
	let packet = ClientB::WorldParticles(b::play_to_client_packet_world_particles::Ty {
		long_distance: false,
		x: 0.0,
		y: 0.0,
		z: 0.0,
		offset_x: 0.0,
		offset_y: 0.0,
		offset_z: 0.0,
		velocity_offset: 0.0,
		amount: 0,
		particle,
	});
	carry(
		Root::PlayToClient,
		(TO, FROM),
		&packet,
		b::play_to_client_packet::serialize,
		a::play_to_client_packet::deserialize,
		|p| match p {
			ClientA::WorldParticles(w) => Some(w.particle),
			_ => None,
		},
		cx,
	)
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

fn split_food(source: &a::slot::Ty, target: &mut b::slot::Ty, cx: &mut Session) -> Result<()> {
	let (a::slot::Ty::Default(_, s), b::slot::Ty::Default(_, t)) = (source, target) else {
		return Ok(());
	};
	let Some(food) = s.components.iter().find_map(|c| match c {
		ComponentA::Food(f) => Some(f),
		_ => None,
	}) else {
		return Ok(());
	};
	if t.components
		.iter()
		.any(|c| matches!(c, ComponentB::Consumable(_)))
	{
		return Ok(());
	}
	t.components
		.push(ComponentB::Consumable(b::slot_component_consumable::Ty {
			consume_seconds: food.seconds_to_eat,
			animation: b::slot_component_consumable_animation::Ty::Eat,
			sound: SoundHolder::Inline(v1_21_1::item_sound_event::Ty {
				sound_name: "minecraft:entity.generic.eat".to_owned(),
				fixed_range: None,
			}),
			makes_particles: true,
			effects: food
				.effects
				.iter()
				.map(|e| {
					b::item_consume_effect::Ty::ApplyEffects(
						b::item_consume_effect_apply_effects::Ty {
							effects: vec![e.effect.clone()],
							probability: e.probability,
						},
					)
				})
				.collect(),
		}));
	if !matches!(food.using_converts_to, a::slot::Ty::V0) {
		let remainder = slot_up(food.using_converts_to.clone(), cx)?;
		t.components.push(ComponentB::UseRemainder(remainder));
	}
	t.added_component_count = t.components.len() as i32;
	Ok(())
}

#[derive(Default)]
struct FoodStash(
	Vec<(
		f32,
		Option<b::slot::Ty>,
		Vec<a::slot_component_food_effects_items::Ty>,
	)>,
);

fn stash_food(cx: &mut Session, components: &[ComponentB]) {
	if !components.iter().any(|c| matches!(c, ComponentB::Food(_))) {
		return;
	}
	let mut seconds = 1.6;
	let mut remainder = None;
	let mut effects = Vec::new();
	for c in components {
		match c {
			ComponentB::Consumable(c) => {
				seconds = c.consume_seconds;
				for e in &c.effects {
					if let b::item_consume_effect::Ty::ApplyEffects(a) = e {
						effects.extend(a.effects.iter().map(|effect| {
							a::slot_component_food_effects_items::Ty {
								effect: effect.clone(),
								probability: a.probability,
							}
						}));
					}
				}
			}
			ComponentB::UseRemainder(s) => remainder = Some(s.clone()),
			_ => {}
		}
	}
	cx.state::<FoodStash>()
		.0
		.push((seconds, remainder, effects));
}

pub fn tricky_registry_data_1_21_3(p: RegistryData, cx: &mut Session) -> Result<Vec<ConfigB>> {
	Ok(vec![ConfigB::RegistryData(registry_up(
		cx,
		(FROM, TO),
		p,
		registry_fix_up,
	))])
}

pub fn tricky_registry_data_down_1_21_1(p: RegistryData, cx: &mut Session) -> Result<Vec<ConfigA>> {
	Ok(vec![ConfigA::RegistryData(registry_down(
		cx,
		(TO, FROM),
		p,
		registry_fix_down,
	)?)])
}

fn dust_colors(registry: &str, tag: &mut Tag, to_int: bool) {
	let Some(effects) = effects(registry, tag) else {
		return;
	};
	let Some(options) = effects
		.compound_mut("particle")
		.and_then(|p| p.compound_mut("options"))
	else {
		return;
	};
	let kind = options
		.field("type")
		.and_then(Payload::as_str)
		.map(|s| strip(s).to_owned());
	let names: &[&str] = match kind.as_deref() {
		Some("dust") => &["color"],
		Some("dust_color_transition") => &["from_color", "to_color"],
		_ => return,
	};
	for name in names {
		let Some(mut v) = options.field_mut(name) else {
			continue;
		};
		match (&*v, to_int) {
			(
				Payload::List(List {
					items: ListItems::Float(c),
					..
				}),
				true,
			) if c.len() == 3 => {
				let rgb = argb(c[0], c[1], c[2]) & 0xffffff;
				*v = Payload::Int(rgb);
			}
			(Payload::Int(c), false) => {
				let (r, g, b) = rgb(*c);
				*v = ListItems::Float(vec![r, g, b]).into();
			}
			_ => {}
		}
	}
}

fn registry_fix_up(registry: &str, tag: &mut Tag) {
	dust_colors(registry, tag, true);
	if strip(registry) == "enchantment" {
		rename_strings(&mut tag.value, "damage_item", "change_item_damage");
	}
}

fn registry_fix_down(registry: &str, tag: &mut Tag) {
	dust_colors(registry, tag, false);
	if strip(registry) == "enchantment" {
		rename_strings(&mut tag.value, "change_item_damage", "damage_item");
	}
}

pub fn tricky_finish_configuration_1_21_3(
	p: a::configuration_to_client_packet_finish_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<ConfigB>> {
	let mut out: Vec<ConfigB> = missing_registries(cx, TO, registry_fix_up)
		.into_iter()
		.map(ConfigB::RegistryData)
		.collect();
	out.push(ConfigB::FinishConfiguration(p));
	Ok(out)
}

pub fn tricky_feature_flags_1_21_3(p: Vec<String>, _cx: &mut Session) -> Result<Vec<ConfigB>> {
	Ok(vec![ConfigB::FeatureFlags(features(p, &[], &["bundle"]))])
}

pub fn tricky_feature_flags_down_1_21_1(p: Vec<String>, _cx: &mut Session) -> Result<Vec<ConfigA>> {
	Ok(vec![ConfigA::FeatureFlags(features(p, &["bundle"], &[]))])
}

pub fn tricky_config_tags_1_21_3(
	mut p: Vec<a::configuration_to_client_packet_tags_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<ConfigB>> {
	for g in &mut p {
		remap_tags(FROM, TO, &g.tag_type, &mut g.tags);
	}
	Ok(vec![ConfigB::Tags(p)])
}

pub fn tricky_config_tags_down_1_21_1(
	mut p: Vec<a::configuration_to_client_packet_tags_items::Ty>,
	cx: &mut Session,
) -> Result<Vec<ConfigA>> {
	for g in &mut p {
		store_item_tags(cx, TO, &g.tag_type, &g.tags);
		remap_tags(TO, FROM, &g.tag_type, &mut g.tags);
	}
	Ok(vec![ConfigA::Tags(p)])
}

pub fn tricky_play_tags_1_21_3(
	mut p: Vec<a::play_to_client_packet_tags_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<ClientB>> {
	for g in &mut p {
		remap_tags(FROM, TO, &g.tag_type, &mut g.tags);
	}
	Ok(vec![ClientB::Tags(p)])
}

pub fn tricky_play_tags_down_1_21_1(
	mut p: Vec<a::play_to_client_packet_tags_items::Ty>,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	for g in &mut p {
		store_item_tags(cx, TO, &g.tag_type, &g.tags);
		remap_tags(TO, FROM, &g.tag_type, &mut g.tags);
	}
	Ok(vec![ClientA::Tags(p)])
}

pub fn tricky_strict_errors_down_1_21_1(
	_src: &b::login_to_client_packet_success::Ty,
	_cx: &mut Session,
) -> Result<bool> {
	Ok(true)
}

pub fn tricky_item_id_1_21_3(src: &a::slot_default::Ty, _cx: &mut Session) -> Result<i32> {
	Ok(item_up(src.item_id))
}

pub fn tricky_item_id_down_1_21_1(src: &b::slot_default::Ty, _cx: &mut Session) -> Result<i32> {
	Ok(item_down(src.item_id))
}

fn kept_down(c: &ComponentB) -> bool {
	use ComponentB as C;
	match c {
		C::DamageResistant(t) => strip(t) == "is_fire",
		C::Consumable(_)
		| C::UseRemainder(_)
		| C::UseCooldown(_)
		| C::Enchantable(_)
		| C::Equippable(_)
		| C::Repairable(_)
		| C::Glider
		| C::TooltipStyle(_)
		| C::DeathProtection(_)
		| C::ItemModel(_) => false,
		_ => true,
	}
}

fn kept_type_down(c: &b::slot_component_type::Ty) -> bool {
	use b::slot_component_type::Ty as C;
	!matches!(
		c,
		C::Consumable
			| C::UseRemainder
			| C::UseCooldown
			| C::Enchantable
			| C::Equippable
			| C::Repairable
			| C::Glider
			| C::TooltipStyle
			| C::DeathProtection
			| C::ItemModel
	)
}

pub fn tricky_added_components_down_1_21_1(
	src: &b::slot_default::Ty,
	cx: &mut Session,
) -> Result<i32> {
	stash_food(cx, &src.components);
	Ok(src.components.iter().filter(|c| kept_down(c)).count() as i32)
}

pub fn tricky_removed_components_down_1_21_1(
	src: &b::slot_default::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(src
		.remove_components
		.iter()
		.filter(|c| kept_type_down(c))
		.count() as i32)
}

pub fn tricky_trade_components_down_1_21_1(
	src: &b::play_to_client_packet_trade_list_trades_items_input_item1::Ty,
	cx: &mut Session,
) -> Result<i32> {
	stash_food(cx, &src.components);
	Ok(src.components.iter().filter(|c| kept_down(c)).count() as i32)
}

pub fn tricky_trade_components2_down_1_21_1(
	src: &b::play_to_client_packet_trade_list_trades_items_input_item2_items::Ty,
	cx: &mut Session,
) -> Result<i32> {
	stash_food(cx, &src.components);
	Ok(src.components.iter().filter(|c| kept_down(c)).count() as i32)
}

pub fn tricky_trade_item_1_21_3(
	src: &a::play_to_client_packet_trade_list_trades_items_input_item1::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_up(src.item_id))
}

pub fn tricky_trade_item_down_1_21_1(
	src: &b::play_to_client_packet_trade_list_trades_items_input_item1::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_down(src.item_id))
}

pub fn tricky_trade_item2_1_21_3(
	src: &a::play_to_client_packet_trade_list_trades_items_input_item2_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_up(src.item_id))
}

pub fn tricky_trade_item2_down_1_21_1(
	src: &b::play_to_client_packet_trade_list_trades_items_input_item2_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_down(src.item_id))
}

pub fn tricky_fire_resistant_1_21_3(_: (), _cx: &mut Session) -> Result<ComponentB> {
	Ok(ComponentB::DamageResistant("minecraft:is_fire".to_owned()))
}

pub fn tricky_damage_resistant_down_1_21_1(types: String, _cx: &mut Session) -> Result<ComponentA> {
	match strip(&types) {
		"is_fire" => Ok(ComponentA::FireResistant),
		_ => Err(Error::Dropped),
	}
}

pub fn tricky_enchantments_1_21_3(
	v: a::slot_component_enchantments::Ty,
	_cx: &mut Session,
) -> Result<ComponentB> {
	let had = !v.enchantments.is_empty();
	let kept: Vec<_> = v
		.enchantments
		.into_iter()
		.filter(|e| e.level > 0)
		.map(|e| b::slot_component_enchantments_enchantments_items::Ty {
			id: e.id,
			level: e.level,
		})
		.collect();
	if had && kept.is_empty() {
		return Ok(ComponentB::EnchantmentGlintOverride(true));
	}
	Ok(ComponentB::Enchantments(
		b::slot_component_enchantments::Ty {
			enchantments: kept,
			show_tooltip: v.show_tooltip,
		},
	))
}

fn instrument_data_up(d: a::instrument_data::Ty) -> b::instrument_data::Ty {
	b::instrument_data::Ty {
		sound_event: d.sound_event,
		use_duration: d.use_duration as f32 / 20.0,
		range: d.range,
		description: Tag::new(""),
	}
}

fn instrument_data_down(d: b::instrument_data::Ty) -> a::instrument_data::Ty {
	a::instrument_data::Ty {
		sound_event: d.sound_event,
		use_duration: (d.use_duration * 20.0) as i32,
		range: d.range,
	}
}

pub fn tricky_instrument_1_21_3(
	p: a::slot_component_instrument::Ty,
	cx: &mut Session,
) -> Result<ComponentB> {
	Ok(ComponentB::Instrument(match p {
		a::slot_component_instrument::Ty::Id(id) => {
			let name = usize::try_from(id).ok().and_then(|i| GOAT_HORNS.get(i));
			b::slot_component_instrument::Ty::Id(
				name.and_then(|n| registry_index(cx, TO, "instrument", n))
					.unwrap_or(id),
			)
		}
		a::slot_component_instrument::Ty::Inline(data) => {
			b::slot_component_instrument::Ty::Inline(instrument_data_up(data))
		}
	}))
}

pub fn tricky_instrument_down_1_21_1(
	p: b::slot_component_instrument::Ty,
	cx: &mut Session,
) -> Result<ComponentA> {
	Ok(ComponentA::Instrument(match p {
		b::slot_component_instrument::Ty::Id(id) => {
			let name = registry_key(cx, TO, "instrument", id);
			a::slot_component_instrument::Ty::Id(
				GOAT_HORNS
					.iter()
					.position(|n| Some(*n) == name.as_deref())
					.map_or(id, |i| i as i32),
			)
		}
		b::slot_component_instrument::Ty::Inline(data) => {
			a::slot_component_instrument::Ty::Inline(instrument_data_down(data))
		}
	}))
}

pub fn tricky_pot_decorations_1_21_3(p: Vec<i32>, _cx: &mut Session) -> Result<ComponentB> {
	Ok(ComponentB::PotDecorations(
		p.into_iter().map(item_up).collect(),
	))
}

pub fn tricky_pot_decorations_down_1_21_1(p: Vec<i32>, _cx: &mut Session) -> Result<ComponentA> {
	Ok(ComponentA::PotDecorations(
		p.into_iter().map(item_down).collect(),
	))
}

pub fn tricky_potion_name_1_21_3(
	_src: &a::slot_component_potion_contents::Ty,
	_cx: &mut Session,
) -> Result<Option<String>> {
	Ok(None)
}

pub fn tricky_painting_title_1_21_3(
	_src: &a::entity_metadata_painting_variant::Ty,
	_cx: &mut Session,
) -> Result<Option<Tag>> {
	Ok(None)
}

pub fn tricky_painting_author_1_21_3(
	_src: &a::entity_metadata_painting_variant::Ty,
	_cx: &mut Session,
) -> Result<Option<Tag>> {
	Ok(None)
}

pub fn tricky_attribute_type_1_21_3(
	src: &a::slot_component_attribute_modifiers_attributes_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	attribute(ATTRIBUTES_1_21_1, ATTRIBUTES_1_21_3, src.type_id)
}

pub fn tricky_attribute_type_down_1_21_1(
	src: &b::slot_component_attribute_modifiers_attributes_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	attribute(ATTRIBUTES_1_21_3, ATTRIBUTES_1_21_1, src.type_id)
}

fn stashed(
	cx: &mut Session,
) -> Option<&mut (
	f32,
	Option<b::slot::Ty>,
	Vec<a::slot_component_food_effects_items::Ty>,
)> {
	cx.state::<FoodStash>().0.last_mut()
}

pub fn tricky_food_seconds_down_1_21_1(
	_src: &b::slot_component_food::Ty,
	cx: &mut Session,
) -> Result<f32> {
	Ok(stashed(cx).map_or(1.6, |s| s.0))
}

pub fn tricky_food_remainder_down_1_21_1(
	_src: &b::slot_component_food::Ty,
	cx: &mut Session,
) -> Result<a::slot::Ty> {
	match stashed(cx).and_then(|s| s.1.take()) {
		Some(slot) => slot_down(slot, cx),
		None => Ok(a::slot::Ty::V0),
	}
}

pub fn tricky_food_effects_down_1_21_1(
	_src: &b::slot_component_food::Ty,
	cx: &mut Session,
) -> Result<a::slot_component_food_effects::Ty> {
	Ok(cx
		.state::<FoodStash>()
		.0
		.pop()
		.map(|s| s.2)
		.unwrap_or_default())
}

pub fn tricky_tool_blocks_1_21_3(
	src: &a::slot_component_tool_rules_items::Ty,
	_cx: &mut Session,
) -> Result<IdSet> {
	Ok(id_set(src.blocks.clone(), |id| block(FROM, TO, id)))
}

pub fn tricky_tool_blocks_down_1_21_1(
	src: &b::slot_component_tool_rules_items::Ty,
	_cx: &mut Session,
) -> Result<IdSet> {
	Ok(id_set(src.blocks.clone(), |id| block(TO, FROM, id)))
}

type BlockSet = a::item_block_predicate_block_set_items::Ty;

pub(super) fn block_set(
	set: Option<BlockSet>,
	map: impl Fn(i32) -> Option<i32>,
) -> Option<BlockSet> {
	set.map(|s| match s {
		BlockSet::Ids(ids) => BlockSet::Ids(ids.into_iter().filter_map(map).collect()),
		named => named,
	})
}

pub fn tricky_predicate_blocks_1_21_3(
	src: &a::item_block_predicate::Ty,
	_cx: &mut Session,
) -> Result<Option<BlockSet>> {
	Ok(block_set(src.block_set.clone(), |id| block(FROM, TO, id)))
}

pub fn tricky_predicate_blocks_down_1_21_1(
	src: &b::item_block_predicate::Ty,
	_cx: &mut Session,
) -> Result<Option<BlockSet>> {
	Ok(block_set(src.block_set.clone(), |id| block(TO, FROM, id)))
}

fn argb(r: f32, g: f32, b: f32) -> i32 {
	let c = |v: f32| ((v * 255.0) as i32).clamp(0, 255);
	(255 << 24) | (c(r) << 16) | (c(g) << 8) | c(b)
}

fn rgb(c: i32) -> (f32, f32, f32) {
	let ch = |s: u32| ((c >> s) & 0xff) as f32 / 255.0;
	(ch(16), ch(8), ch(0))
}

pub fn tricky_dust_1_21_3(p: a::particle_dust::Ty, _cx: &mut Session) -> Result<ParticleB> {
	Ok(ParticleB::Dust(b::particle_dust::Ty {
		color: argb(p.red, p.green, p.blue),
		scale: p.scale,
	}))
}

pub fn tricky_dust_down_1_21_1(p: b::particle_dust::Ty, _cx: &mut Session) -> Result<ParticleA> {
	let (red, green, blue) = rgb(p.color);
	Ok(ParticleA::Dust(a::particle_dust::Ty {
		red,
		green,
		blue,
		scale: p.scale,
	}))
}

pub fn tricky_dust_transition_1_21_3(
	p: a::particle_dust_color_transition::Ty,
	_cx: &mut Session,
) -> Result<ParticleB> {
	Ok(ParticleB::DustColorTransition(
		b::particle_dust_color_transition::Ty {
			from_color: argb(p.from_red, p.from_green, p.from_blue),
			to_color: argb(p.to_red, p.to_green, p.to_blue),
			scale: p.scale,
		},
	))
}

pub fn tricky_dust_transition_down_1_21_1(
	p: b::particle_dust_color_transition::Ty,
	_cx: &mut Session,
) -> Result<ParticleA> {
	let (from_red, from_green, from_blue) = rgb(p.from_color);
	let (to_red, to_green, to_blue) = rgb(p.to_color);
	Ok(ParticleA::DustColorTransition(
		a::particle_dust_color_transition::Ty {
			from_red,
			from_green,
			from_blue,
			scale: p.scale,
			to_red,
			to_green,
			to_blue,
		},
	))
}

pub fn tricky_trail_down_1_21_1(p: b::particle_trail::Ty, _cx: &mut Session) -> Result<ParticleA> {
	Ok(ParticleA::EntityEffect(p.color))
}

pub fn tricky_block_crumble_down_1_21_1(state: i32, _cx: &mut Session) -> Result<ParticleA> {
	Ok(ParticleA::Block(state_down(state)))
}

macro_rules! block_particles {
	($($arm:ident: $up:ident, $down:ident;)*) => {$(
		pub fn $up(state: i32, _cx: &mut Session) -> Result<ParticleB> {
			Ok(ParticleB::$arm(state_up(state)))
		}

		pub fn $down(state: i32, _cx: &mut Session) -> Result<ParticleA> {
			Ok(ParticleA::$arm(state_down(state)))
		}
	)*};
}

block_particles! {
	Block: tricky_particle_block_1_21_3, tricky_particle_block_down_1_21_1;
	BlockMarker: tricky_particle_block_marker_1_21_3, tricky_particle_block_marker_down_1_21_1;
	FallingDust: tricky_particle_falling_dust_1_21_3, tricky_particle_falling_dust_down_1_21_1;
	DustPillar: tricky_particle_dust_pillar_1_21_3, tricky_particle_dust_pillar_down_1_21_1;
}

pub fn tricky_meta_entity_1_21_3(
	src: &a::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	metadata_of(cx, FROM, src.entity_id);
	Ok(src.entity_id)
}

pub fn tricky_meta_entity_down_1_21_1(
	src: &b::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	metadata_of(cx, TO, src.entity_id);
	Ok(src.entity_id)
}

fn boat(ty: &str) -> bool {
	ty.ends_with("boat") || ty.ends_with("raft")
}

fn ageable_water(ty: &str) -> bool {
	matches!(ty, "squid" | "glow_squid" | "dolphin")
}

fn arrow(ty: &str) -> bool {
	matches!(ty, "arrow" | "spectral_arrow" | "trident")
}

pub fn tricky_meta_key_1_21_3(src: &a::entity_metadata_entry::Ty, cx: &mut Session) -> Result<u8> {
	let key = src.key;
	match metadata_key(cx, FROM, key) {
		Some(t) if boat(t) && key == 11 => Err(Error::Dropped),
		Some(t) if boat(t) && key > 11 => Ok(key - 1),
		Some(t) if ageable_water(t) && key >= 16 => Ok(key + 1),
		Some(t) if arrow(t) && key >= 10 => Ok(key + 1),
		_ => Ok(key),
	}
}

pub fn tricky_meta_key_down_1_21_1(
	src: &b::entity_metadata_entry::Ty,
	cx: &mut Session,
) -> Result<u8> {
	let key = src.key;
	match metadata_key(cx, TO, key) {
		Some(t) if boat(t) && key >= 11 => Ok(key + 1),
		Some(t) if ageable_water(t) && key == 16 => Err(Error::Dropped),
		Some(t) if ageable_water(t) && key > 16 => Ok(key - 1),
		Some(t) if arrow(t) && key == 10 => Err(Error::Dropped),
		Some(t) if arrow(t) && key > 10 => Ok(key - 1),
		Some("creaking" | "creaking_transient") if key >= 16 => Err(Error::Dropped),
		Some("salmon") if key == 17 => Err(Error::Dropped),
		_ => Ok(key),
	}
}

pub fn tricky_meta_block_1_21_3(state: i32, _cx: &mut Session) -> Result<MetaB> {
	Ok(MetaB::BlockState(state_up(state)))
}

pub fn tricky_meta_block_down_1_21_1(state: i32, _cx: &mut Session) -> Result<MetaA> {
	Ok(MetaA::BlockState(state_down(state)))
}

pub fn tricky_meta_optional_block_1_21_3(state: i32, _cx: &mut Session) -> Result<MetaB> {
	Ok(MetaB::OptionalBlockState(state_up(state)))
}

pub fn tricky_meta_optional_block_down_1_21_1(state: i32, _cx: &mut Session) -> Result<MetaA> {
	Ok(MetaA::OptionalBlockState(state_down(state)))
}

pub fn tricky_meta_int_1_21_3(v: i32, cx: &mut Session) -> Result<MetaB> {
	Ok(MetaB::Int(match current_entry(cx, FROM) {
		(ty, 11) if is_minecart(ty) => state_up(v),
		_ => v,
	}))
}

pub fn tricky_meta_int_down_1_21_1(v: i32, cx: &mut Session) -> Result<MetaA> {
	Ok(MetaA::Int(match current_entry(cx, TO) {
		(ty, 11) if is_minecart(ty) => state_down(v),
		_ => v,
	}))
}

pub fn tricky_entity_type_1_21_3(
	src: &a::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<i32> {
	let id = remap("entities", FROM, TO, src.r#type).ok_or(Error::Dropped)?;
	spawned(cx, FROM, src.entity_id, entity_name(FROM, src.r#type));
	spawned(cx, TO, src.entity_id, entity_name(TO, id));
	Ok(id)
}

pub fn tricky_object_data_1_21_3(
	src: &a::play_to_client_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(match entity_name(FROM, src.r#type) {
		Some("falling_block") => state_up(src.object_data),
		_ => src.object_data,
	})
}

pub fn tricky_spawn_entity_down_1_21_1(
	mut p: b::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let name = entity_name(TO, p.r#type);
	let id = remap("entities", TO, FROM, p.r#type).ok_or(Error::Dropped)?;
	spawned(cx, TO, p.entity_id, name);
	spawned(cx, FROM, p.entity_id, entity_name(FROM, id));
	if name == Some("falling_block") {
		p.object_data = state_down(p.object_data);
	}
	p.r#type = id;
	let entity = p.entity_id;
	let mut out = vec![ClientA::SpawnEntity(p)];
	if let Some(name) = name.filter(|n| boat(n)) {
		let wood = BOAT_WOODS
			.iter()
			.position(|w| name.starts_with(&format!("{w}_")))
			.unwrap_or(0);
		out.push(ClientA::EntityMetadata(
			a::play_to_client_packet_entity_metadata::Ty {
				entity_id: entity,
				metadata: vec![a::entity_metadata_entry::Ty {
					key: 11,
					value: MetaA::Int(wood as i32),
				}],
			},
		));
	}
	Ok(out)
}

pub fn tricky_block_change_1_21_3(
	src: &a::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state_up(src.r#type))
}

pub fn tricky_block_change_down_1_21_1(
	src: &b::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state_down(src.r#type))
}

pub(super) fn records(records: &[i32], map: impl Fn(i32) -> i32) -> Vec<i32> {
	records
		.iter()
		.map(|r| (map((*r as u32 >> 12) as i32) << 12) | (r & 0xfff))
		.collect()
}

pub fn tricky_multi_block_1_21_3(
	src: &a::play_to_client_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(records(&src.records, state_up))
}

pub fn tricky_multi_block_down_1_21_1(
	src: &b::play_to_client_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(records(&src.records, state_down))
}

pub(super) fn chunk(from: Version, to: Version, data: &[u8]) -> Result<Vec<u8>> {
	let states = registry::blocks(to).map_or(1, |b| b.states());
	let table = registry::state_table(from, to).unwrap_or_default();
	chunk::remap_sections(data, states, &|s| {
		usize::try_from(s)
			.ok()
			.and_then(|s| table.get(s))
			.map_or(0, |v| *v as i32)
	})
	.ok_or_else(|| {
		Error::Encoding(format!(
			"chunk sections of {} bytes do not parse",
			data.len()
		))
	})
}

#[derive(Default)]
struct Chunks {
	loaded: HashSet<(i32, i32)>,
	bundling: bool,
}

pub fn tricky_resend_1_21_3(cx: &mut Session, x: i32, z: i32) -> bool {
	let chunks = cx.state::<Chunks>();
	!(chunks.loaded.insert((x, z)) || chunks.bundling)
}

pub fn tricky_map_chunk_1_21_3(
	mut p: a::play_to_client_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientB>> {
	p.chunk_data = chunk(FROM, TO, &p.chunk_data)?;
	for e in &mut p.block_entities {
		e.r#type = block_entity_up(e.r#type);
	}
	let (x, z) = (p.x, p.z);
	if !tricky_resend_1_21_3(cx, x, z) {
		return Ok(vec![ClientB::MapChunk(p)]);
	}
	Ok(vec![
		ClientB::BundleDelimiter,
		ClientB::UnloadChunk(b::play_to_client_packet_unload_chunk::Ty {
			chunk_x: x,
			chunk_z: z,
		}),
		ClientB::MapChunk(p),
		ClientB::BundleDelimiter,
	])
}

pub fn tricky_chunk_down_1_21_1(
	src: &b::play_to_client_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	chunk(TO, FROM, &src.chunk_data)
}

pub fn tricky_unload_chunk_1_21_3(
	p: a::play_to_client_packet_unload_chunk::Ty,
	cx: &mut Session,
) -> Result<b::play_to_client_packet_unload_chunk::Ty> {
	cx.state::<Chunks>().loaded.remove(&(p.chunk_x, p.chunk_z));
	Ok(p)
}

pub fn tricky_bundle_1_21_3(_: (), cx: &mut Session) -> Result<Vec<ClientB>> {
	let chunks = cx.state::<Chunks>();
	chunks.bundling = !chunks.bundling;
	Ok(vec![ClientB::BundleDelimiter])
}

pub fn tricky_respawn_1_21_3(
	src: &a::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<u8> {
	cx.state::<Chunks>().loaded.clear();
	Ok(src.copy_metadata)
}

fn block_entity_up(id: i32) -> i32 {
	if id >= CREAKING_HEART {
		id + 1
	} else {
		id
	}
}

fn block_entity_down(id: i32) -> Result<i32> {
	match id {
		CREAKING_HEART => Err(Error::Dropped),
		id if id > CREAKING_HEART => Ok(id - 1),
		id => Ok(id),
	}
}

pub fn tricky_block_entity_1_21_3(
	src: &a::chunk_block_entity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(block_entity_up(src.r#type))
}

pub fn tricky_block_entity_down_1_21_1(
	src: &b::chunk_block_entity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block_entity_down(src.r#type)
}

pub fn tricky_tile_entity_1_21_3(
	src: &a::play_to_client_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(block_entity_up(src.action))
}

pub fn tricky_tile_entity_down_1_21_1(
	src: &b::play_to_client_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block_entity_down(src.action)
}

pub fn tricky_world_event_1_21_3(
	src: &a::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(if src.effect_id == 2001 {
		state_up(src.data)
	} else {
		src.data
	})
}

pub fn tricky_world_event_down_1_21_1(
	src: &b::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(if src.effect_id == 2001 {
		state_down(src.data)
	} else {
		src.data
	})
}

pub fn tricky_sound_1_21_3(holder: SoundHolder, _cx: &mut Session) -> Result<SoundHolder> {
	Ok(sound(FROM, TO, holder))
}

pub fn tricky_sound_down_1_21_1(holder: SoundHolder, _cx: &mut Session) -> Result<SoundHolder> {
	Ok(sound(TO, FROM, holder))
}

pub fn tricky_cooldown_group_1_21_3(
	src: &a::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<String> {
	item_name(TO, item_up(src.item_id)).ok_or(Error::Dropped)
}

pub fn tricky_cooldown_item_down_1_21_1(
	src: &b::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	item_id(TO, &src.cooldown_group)
		.map(item_down)
		.ok_or(Error::Dropped)
}

pub fn tricky_tick_day_time_1_21_3(
	src: &a::play_to_client_packet_update_time::Ty,
	_cx: &mut Session,
) -> Result<bool> {
	Ok(src.time >= 0)
}

pub fn tricky_day_time_1_21_3(
	src: &a::play_to_client_packet_update_time::Ty,
	_cx: &mut Session,
) -> Result<i64> {
	Ok(src.time.wrapping_abs())
}

pub fn tricky_day_time_down_1_21_1(
	src: &b::play_to_client_packet_update_time::Ty,
	_cx: &mut Session,
) -> Result<i64> {
	Ok(match (src.tick_day_time, src.time) {
		(true, t) => t,
		(false, 0) => -1,
		(false, t) => -t,
	})
}

pub fn tricky_list_priority_1_21_3(
	_src: &a::play_to_client_packet_player_info_data_items::Ty,
	_cx: &mut Session,
) -> Result<b::play_to_client_packet_player_info_data_items_list_priority::Ty> {
	Ok(b::play_to_client_packet_player_info_data_items_list_priority::Ty::Default(false))
}

#[derive(Default)]
struct Inventory {
	state_id: i32,
}

pub fn tricky_window_items_1_21_3(
	p: a::play_to_client_packet_window_items::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientB>> {
	let mut sources = p.items.clone();
	sources.push(p.carried_item.clone());
	let mut slots = p.items;
	slots.push(p.carried_item);
	let mut slots = equipment_up(slots, cx)?;
	for (s, t) in sources.iter().zip(slots.iter_mut()) {
		split_food(s, t, cx)?;
	}
	let carried = slots.pop().ok_or(Error::Dropped)?;
	Ok(vec![ClientB::WindowItems(
		b::play_to_client_packet_window_items::Ty {
			window_id: p.window_id as i32,
			state_id: p.state_id,
			items: slots,
			carried_item: carried,
		},
	)])
}

pub fn tricky_window_items_down_1_21_1(
	p: b::play_to_client_packet_window_items::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	cx.state::<Inventory>().state_id = p.state_id;
	let mut slots = p.items;
	slots.push(p.carried_item);
	let mut slots = equipment_down(slots, cx)?;
	let carried = slots.pop().ok_or(Error::Dropped)?;
	Ok(vec![ClientA::WindowItems(
		a::play_to_client_packet_window_items::Ty {
			window_id: p.window_id as u8,
			state_id: p.state_id,
			items: slots,
			carried_item: carried,
		},
	)])
}

pub fn tricky_set_slot_1_21_3(
	p: a::play_to_client_packet_set_slot::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientB>> {
	let item = slot_up(p.item, cx)?;
	Ok(vec![match p.window_id as i8 {
		-1 => ClientB::SetCursorItem(item),
		-2 => ClientB::SetPlayerInventory(b::play_to_client_packet_set_player_inventory::Ty {
			slot_id: p.slot as i32,
			contents: item,
		}),
		id => ClientB::SetSlot(b::play_to_client_packet_set_slot::Ty {
			window_id: id as i32,
			state_id: p.state_id,
			slot: p.slot,
			item,
		}),
	}])
}

pub fn tricky_set_slot_down_1_21_1(
	p: b::play_to_client_packet_set_slot::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	cx.state::<Inventory>().state_id = p.state_id;
	let item = slot_down(p.item, cx)?;
	let mut out = vec![ClientA::SetSlot(a::play_to_client_packet_set_slot::Ty {
		window_id: p.window_id as u8,
		state_id: p.state_id,
		slot: p.slot,
		item: item.clone(),
	})];
	if p.window_id == 0 && (5..=8).contains(&p.slot) {
		out.push(ClientA::EntityEquipment(
			a::play_to_client_packet_entity_equipment::Ty {
				entity_id: cx.state::<Movement>().player,
				equipments: vec![
					a::play_to_client_packet_entity_equipment_equipments_items::Ty {
						slot: (10 - p.slot) as i8,
						item,
					},
				],
			},
		));
	}
	Ok(out)
}

pub fn tricky_set_cursor_item_down_1_21_1(
	item: b::slot::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let state_id = cx.state::<Inventory>().state_id;
	Ok(vec![ClientA::SetSlot(
		a::play_to_client_packet_set_slot::Ty {
			window_id: 255,
			state_id: state_id,
			slot: -1,
			item: slot_down(item, cx)?,
		},
	)])
}

pub fn tricky_set_player_inventory_down_1_21_1(
	p: b::play_to_client_packet_set_player_inventory::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	Ok(vec![ClientA::SetSlot(
		a::play_to_client_packet_set_slot::Ty {
			window_id: 254,
			state_id: 0,
			slot: p.slot_id as i16,
			item: slot_down(p.contents, cx)?,
		},
	)])
}

pub fn tricky_teleport_1_21_3(
	p: a::play_to_client_packet_entity_teleport::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientB>> {
	Ok(vec![ClientB::SyncEntityPosition(
		b::play_to_client_packet_sync_entity_position::Ty {
			entity_id: p.entity_id,
			x: p.x,
			y: p.y,
			z: p.z,
			dx: 0.0,
			dy: 0.0,
			dz: 0.0,
			yaw: angle(p.yaw),
			pitch: angle(p.pitch),
			on_ground: p.on_ground,
		},
	)])
}

pub fn tricky_sync_position_down_1_21_1(
	p: b::play_to_client_packet_sync_entity_position::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientA>> {
	Ok(vec![ClientA::EntityTeleport(
		a::play_to_client_packet_entity_teleport::Ty {
			entity_id: p.entity_id,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: to_angle(p.yaw),
			pitch: to_angle(p.pitch),
			on_ground: p.on_ground,
		},
	)])
}

fn velocity_packet(entity: i32, (x, y, z): (f64, f64, f64)) -> ClientA {
	ClientA::EntityVelocity(a::play_to_client_packet_entity_velocity::Ty {
		entity_id: entity,
		velocity: a::vec3i16::Ty {
			x: velocity(x),
			y: velocity(y),
			z: velocity(z),
		},
	})
}

fn knockback_packet((x, y, z): (f64, f64, f64)) -> ClientA {
	ClientA::Explosion(a::play_to_client_packet_explosion::Ty {
		x: 0.0,
		y: 20000.0,
		z: 0.0,
		radius: 0.0,
		affected_block_offsets: vec![],
		player_motion_x: x as f32,
		player_motion_y: y as f32,
		player_motion_z: z as f32,
		block_interaction_type: 0,
		small_explosion_particle: ParticleA::AngryVillager,
		large_explosion_particle: ParticleA::AngryVillager,
		sound: SoundHolder::Inline(v1_21_1::item_sound_event::Ty {
			sound_name: "minecraft:intentionally_empty".to_owned(),
			fixed_range: None,
		}),
	})
}

fn motion(
	entity: i32,
	flags: &b::play_to_client_position_update_relatives::Ty,
	delta: (f64, f64, f64),
	player: bool,
) -> Option<ClientA> {
	let moving = delta != (0.0, 0.0, 0.0);
	match (flags.dx, flags.dy, flags.dz) {
		(true, true, true) if player && moving => Some(knockback_packet(delta)),
		(false, false, false) => Some(velocity_packet(entity, delta)),
		_ => None,
	}
}

pub fn tricky_position_down_1_21_1(
	p: b::play_to_client_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let f = &p.flags;
	let m = cx.state::<Movement>();
	let base = m.position.unwrap_or((0.0, 0.0, 0.0));
	let x = if f.x { base.0 + p.x } else { p.x };
	let y = if f.y { base.1 + p.y } else { p.y };
	let z = if f.z { base.2 + p.z } else { p.z };
	let yaw = if f.yaw { m.rotation.0 + p.yaw } else { p.yaw };
	let pitch = if f.pitch {
		m.rotation.1 + p.pitch
	} else {
		p.pitch
	};
	m.position = Some((x, y, z));
	m.rotation = (yaw, pitch);
	let player = m.player;
	let mut out = vec![ClientA::Position(a::play_to_client_packet_position::Ty {
		x: p.x,
		y: p.y,
		z: p.z,
		yaw: p.yaw,
		pitch: p.pitch,
		flags: a::play_to_client_position_update_relatives::Ty {
			x: f.x,
			y: f.y,
			z: f.z,
			yaw: f.yaw,
			pitch: f.pitch,
		},
		teleport_id: p.teleport_id,
	})];
	out.extend(motion(player, f, (p.dx, p.dy, p.dz), true));
	Ok(out)
}

pub fn tricky_teleport_down_1_21_1(
	p: b::play_to_client_packet_entity_teleport::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let f = &p.flags;
	let m = cx.state::<Movement>();
	let player = p.entity_id == m.player;
	let relative = f.x || f.y || f.z || f.yaw || f.pitch;
	let (x, y, z, yaw, pitch) = match (relative, player, m.position) {
		(false, _, _) => (p.x, p.y, p.z, p.yaw, p.pitch),
		(true, true, Some(base)) => (
			if f.x { base.0 + p.x } else { p.x },
			if f.y { base.1 + p.y } else { p.y },
			if f.z { base.2 + p.z } else { p.z },
			if f.yaw { m.rotation.0 + p.yaw } else { p.yaw },
			if f.pitch {
				m.rotation.1 + p.pitch
			} else {
				p.pitch
			},
		),
		_ => return Ok(vec![]),
	};
	if player {
		m.position = Some((x, y, z));
		m.rotation = (yaw, pitch);
	}
	let mut out = vec![ClientA::EntityTeleport(
		a::play_to_client_packet_entity_teleport::Ty {
			entity_id: p.entity_id,
			x,
			y,
			z,
			yaw: to_angle(yaw),
			pitch: to_angle(pitch),
			on_ground: p.on_ground,
		},
	)];
	out.extend(motion(p.entity_id, f, (p.dx, p.dy, p.dz), player));
	Ok(out)
}

pub fn tricky_move_minecart_down_1_21_1(
	p: b::play_to_client_packet_move_minecart::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let Some(step) = p.steps.last() else {
		return Ok(vec![]);
	};
	Ok(vec![
		ClientA::EntityTeleport(a::play_to_client_packet_entity_teleport::Ty {
			entity_id: p.entity_id,
			x: step.position.x,
			y: step.position.y,
			z: step.position.z,
			yaw: (128 - i32::from(step.yaw)) as i8,
			pitch: step.pitch,
			on_ground: false,
		}),
		velocity_packet(
			p.entity_id,
			(step.movement.x, step.movement.y, step.movement.z),
		),
	])
}

pub fn tricky_player_rotation_down_1_21_1(
	p: b::play_to_client_packet_player_rotation::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let m = cx.state::<Movement>();
	let (x, y, z) = m.position.unwrap_or((0.0, 0.0, 0.0));
	m.rotation = (p.yaw, p.pitch);
	let on_ground = m.on_ground;
	let (yaw, pitch) = (p.yaw.to_radians() as f64, p.pitch.to_radians() as f64);
	let factor = -(-pitch).cos();
	let look = ClientA::FacePlayer(a::play_to_client_packet_face_player::Ty {
		feet_eyes: 0,
		x: x + (-yaw - std::f64::consts::PI).sin() * factor,
		y: y + (-pitch).sin(),
		z: z + (-yaw - std::f64::consts::PI).cos() * factor,
		is_entity: a::play_to_client_packet_face_player_is_entity::Ty::Default(false),
	});
	let rotation = ServerB::Look(b::play_to_server_packet_look::Ty {
		yaw: p.yaw,
		pitch: p.pitch,
		flags: ground_flags(on_ground),
	});
	send(
		cx,
		Root::PlayToServer,
		TO,
		&rotation,
		b::play_to_server_packet::serialize,
	)?;
	Ok(vec![look])
}

pub fn tricky_explosion_1_21_3(
	p: a::play_to_client_packet_explosion::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientB>> {
	let knockback = (p.player_motion_x != 0.0
		|| p.player_motion_y != 0.0
		|| p.player_motion_z != 0.0)
		.then(|| b::vec3f64::Ty {
			x: p.player_motion_x as f64,
			y: p.player_motion_y as f64,
			z: p.player_motion_z as f64,
		});
	let large = p.radius >= 2.0 && p.block_interaction_type != 0;
	let particle = particle_up(
		if large {
			p.large_explosion_particle
		} else {
			p.small_explosion_particle
		},
		cx,
	)?;
	let (x, y, z) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
	let mut out = vec![ClientB::Explosion(b::play_to_client_packet_explosion::Ty {
		x: p.x,
		y: p.y,
		z: p.z,
		player_knockback: knockback,
		explosion_particle: particle,
		sound: sound(FROM, TO, p.sound),
	})];
	if matches!(p.block_interaction_type, 1 | 2) {
		out.extend(p.affected_block_offsets.iter().map(|o| {
			ClientB::BlockChange(b::play_to_client_packet_block_change::Ty {
				location: b::position::Ty {
					x: x + o.x as i32,
					y: (y + o.y as i32) as i16,
					z: z + o.z as i32,
				},
				r#type: 0,
			})
		}));
	}
	Ok(out)
}

pub fn tricky_explosion_down_1_21_1(
	p: b::play_to_client_packet_explosion::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let knockback = p.player_knockback.unwrap_or(b::vec3f64::Ty {
		x: 0.0,
		y: 0.0,
		z: 0.0,
	});
	let particle = particle_down(p.explosion_particle, cx)?;
	Ok(vec![ClientA::Explosion(
		a::play_to_client_packet_explosion::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
			radius: 0.0,
			affected_block_offsets: vec![],
			player_motion_x: knockback.x as f32,
			player_motion_y: knockback.y as f32,
			player_motion_z: knockback.z as f32,
			block_interaction_type: 0,
			small_explosion_particle: particle.clone(),
			large_explosion_particle: particle,
			sound: sound(TO, FROM, p.sound),
		},
	)])
}

fn ingredient_items(ingredient: &[a::slot::Ty]) -> Vec<i32> {
	ingredient
		.iter()
		.filter_map(|s| match s {
			a::slot::Ty::Default(_, d) => Some(item_up(d.item_id)),
			_ => None,
		})
		.collect()
}

pub fn tricky_declare_recipes_1_21_3(p: Vec<RecipeA>, cx: &mut Session) -> Result<Vec<ClientB>> {
	use RecipeDataA as R;
	let names = [
		"minecraft:furnace_input",
		"minecraft:blast_furnace_input",
		"minecraft:smoker_input",
		"minecraft:campfire_input",
		"minecraft:smithing_template",
		"minecraft:smithing_base",
		"minecraft:smithing_addition",
	];
	let mut sets: Vec<Vec<i32>> = vec![vec![]; names.len()];
	let mut stonecutter = Vec::new();
	for recipe in p {
		let found: Vec<(usize, Vec<i32>)> = match recipe.data {
			R::MinecraftSmelting(r) => vec![(0, ingredient_items(&r.ingredient))],
			R::MinecraftBlasting(r) => vec![(1, ingredient_items(&r.ingredient))],
			R::MinecraftSmoking(r) => vec![(2, ingredient_items(&r.ingredient))],
			R::MinecraftCampfireCooking(r) => vec![(3, ingredient_items(&r.ingredient))],
			R::MinecraftSmithingTransform(r) => vec![
				(4, ingredient_items(&r.template)),
				(5, ingredient_items(&r.base)),
				(6, ingredient_items(&r.addition)),
			],
			R::MinecraftSmithingTrim(r) => vec![
				(4, ingredient_items(&r.template)),
				(5, ingredient_items(&r.base)),
				(6, ingredient_items(&r.addition)),
			],
			R::MinecraftStonecutting(r) => {
				stonecutter.push((ingredient_items(&r.ingredient), r.result));
				vec![]
			}
			_ => vec![],
		};
		for (set, items) in found {
			for i in items {
				if !sets[set].contains(&i) {
					sets[set].push(i);
				}
			}
		}
	}
	let mut cutting = Vec::new();
	for (input, result) in stonecutter {
		cutting.push(
			b::play_to_client_packet_declare_recipes_stone_cutter_recipes_items::Ty {
				input: IdSet::Ids(input),
				slot_display: Display::ItemStack(slot_up(result, cx)?),
			},
		);
	}
	Ok(vec![ClientB::DeclareRecipes(
		b::play_to_client_packet_declare_recipes::Ty {
			recipes: names
				.iter()
				.zip(sets)
				.map(
					|(name, items)| b::play_to_client_packet_declare_recipes_recipes_items::Ty {
						name: (*name).to_owned(),
						items,
					},
				)
				.collect(),
			stone_cutter_recipes: cutting,
		},
	)])
}

#[derive(Default)]
struct Recipes {
	stonecutter: Vec<RecipeA>,
	book: Vec<(i32, RecipeA)>,
	settings: [bool; 8],
}

fn plain(item: i32, count: i32) -> a::slot::Ty {
	if item <= 0 || count <= 0 {
		return a::slot::Ty::V0;
	}
	a::slot::Ty::Default(
		count,
		a::slot_default::Ty {
			item_id: item,
			added_component_count: 0,
			removed_component_count: 0,
			components: vec![],
			remove_components: vec![],
		},
	)
}

fn display_items(d: &Display, cx: &mut Session) -> Result<Vec<a::slot::Ty>> {
	Ok(match d {
		Display::Item(id) => vec![plain(item_down(*id), 1)],
		Display::ItemStack(s) => vec![slot_down(s.clone(), cx)?],
		Display::Tag(t) => item_tag(cx, TO, t)
			.into_iter()
			.map(|i| plain(item_down(i), 1))
			.collect(),
		Display::WithRemainder(r) => display_items(&r.input, cx)?,
		Display::Composite(list) => {
			let mut out = Vec::new();
			for d in list {
				out.extend(display_items(d, cx)?);
			}
			out
		}
		_ => vec![],
	})
}

fn display_result(d: &Display, cx: &mut Session) -> Result<a::slot::Ty> {
	Ok(display_items(d, cx)?
		.into_iter()
		.next()
		.unwrap_or(a::slot::Ty::V0))
}

fn station(d: &Display) -> Option<&'static str> {
	let id = match d {
		Display::Item(id) => *id,
		Display::ItemStack(b::slot::Ty::Default(_, s)) => s.item_id,
		_ => return None,
	};
	registry::names(TO, "items")?.name(u32::try_from(id).ok()?)
}

fn book_category(
	c: &b::play_to_client_packet_recipe_book_add_entries_items_recipe_category::Ty,
) -> i32 {
	use b::play_to_client_packet_recipe_book_add_entries_items_recipe_category::Ty as C;
	match c {
		C::CraftingBuildingBlocks | C::FurnaceFood | C::SmokerFood | C::Campfire => 0,
		C::CraftingRedstone | C::FurnaceBlocks | C::BlastFurnaceBlocks => 1,
		C::CraftingEquipment | C::FurnaceMisc | C::BlastFurnaceMisc => 2,
		_ => 3,
	}
}

fn recipe_down(
	id: i32,
	display: &b::play_to_client_recipe_display::Ty,
	category: i32,
	cx: &mut Session,
) -> Result<Option<RecipeA>> {
	use b::play_to_client_recipe_display::Ty as D;
	let data = match display {
		D::CraftingShaped(r) => {
			let mut cells = Vec::new();
			for d in &r.ingredients {
				cells.push(display_items(d, cx)?);
			}
			let height = r.height.max(1) as usize;
			RecipeDataA::MinecraftCraftingShaped(
				a::play_to_client_packet_declare_recipes_items_data_minecraft_crafting_shaped::Ty {
					group: String::new(),
					category,
					grid_width: r.width,
					grid_height: r.height,
					ingredients: cells.chunks(height).map(|c| c.to_vec()).collect(),
					result: display_result(&r.result, cx)?,
					show_notification: true,
				},
			)
		}
		D::CraftingShapeless(r) => {
			let mut ingredients = Vec::new();
			for d in &r.ingredients {
				ingredients.push(display_items(d, cx)?);
			}
			RecipeDataA::MinecraftCraftingShapeless(a::play_to_client_packet_declare_recipes_items_data_minecraft_crafting_shapeless::Ty {
				group: String::new(),
				category,
				ingredients,
				result: display_result(&r.result, cx)?,
			})
		}
		D::Furnace(r) => {
			let format = a::minecraft_smelting_format::Ty {
				group: String::new(),
				category,
				ingredient: display_items(&r.ingredient, cx)?,
				result: display_result(&r.result, cx)?,
				experience: r.experience,
				cook_time: r.duration,
			};
			match station(&r.crafting_station) {
				Some("blast_furnace") => RecipeDataA::MinecraftBlasting(format),
				Some("smoker") => RecipeDataA::MinecraftSmoking(format),
				Some("campfire" | "soul_campfire") => RecipeDataA::MinecraftCampfireCooking(format),
				_ => RecipeDataA::MinecraftSmelting(format),
			}
		}
		D::Stonecutter(r) => RecipeDataA::MinecraftStonecutting(
			a::play_to_client_packet_declare_recipes_items_data_minecraft_stonecutting::Ty {
				group: String::new(),
				ingredient: display_items(&r.ingredient, cx)?,
				result: display_result(&r.result, cx)?,
			},
		),
		D::Smithing(r) => RecipeDataA::MinecraftSmithingTransform(
			a::play_to_client_packet_declare_recipes_items_data_minecraft_smithing_transform::Ty {
				template: display_items(&r.template, cx)?,
				base: display_items(&r.base, cx)?,
				addition: display_items(&r.addition, cx)?,
				result: display_result(&r.result, cx)?,
			},
		),
		_ => return Ok(None),
	};
	Ok(Some(RecipeA {
		name: format!("minecraft:{id}"),
		data,
	}))
}

fn all_recipes(cx: &mut Session) -> ClientA {
	let r = cx.state::<Recipes>();
	ClientA::DeclareRecipes(
		r.stonecutter
			.iter()
			.cloned()
			.chain(r.book.iter().map(|(_, r)| r.clone()))
			.collect(),
	)
}

fn unlock(
	cx: &mut Session,
	recipes: Vec<String>,
	action: a::play_to_client_packet_unlock_recipes_recipes2::Ty,
) -> ClientA {
	let s = cx.state::<Recipes>().settings;
	ClientA::UnlockRecipes(a::play_to_client_packet_unlock_recipes::Ty {
		crafting_book_open: s[0],
		filtering_craftable: s[1],
		smelting_book_open: s[2],
		filtering_smeltable: s[3],
		blast_furnace_open: s[4],
		filtering_blast_furnace: s[5],
		smoker_book_open: s[6],
		filtering_smoker: s[7],
		recipes1: recipes,
		recipes2: action,
	})
}

pub fn tricky_declare_recipes_down_1_21_1(
	p: b::play_to_client_packet_declare_recipes::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let mut stonecutter = Vec::new();
	for (i, r) in p.stone_cutter_recipes.into_iter().enumerate() {
		let input: Vec<i32> = match r.input {
			IdSet::Ids(ids) => ids,
			IdSet::Named(tag) => item_tag(cx, TO, &tag),
		};
		let result = display_result(&r.slot_display, cx)?;
		stonecutter.push(RecipeA {
			name: format!("minecraft:stonecutter_{i}"),
			data: RecipeDataA::MinecraftStonecutting(
				a::play_to_client_packet_declare_recipes_items_data_minecraft_stonecutting::Ty {
					group: String::new(),
					ingredient: input.into_iter().map(|i| plain(item_down(i), 1)).collect(),
					result,
				},
			),
		});
	}
	cx.state::<Recipes>().stonecutter = stonecutter;
	Ok(vec![all_recipes(cx)])
}

pub fn tricky_recipe_book_add_down_1_21_1(
	p: b::play_to_client_packet_recipe_book_add::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	if p.replace {
		cx.state::<Recipes>().book.clear();
	}
	let mut added = Vec::new();
	let mut highlighted = Vec::new();
	for e in p.entries {
		let id = e.recipe.display_id;
		let category = book_category(&e.recipe.category);
		let Some(recipe) = recipe_down(id, &e.recipe.display, category, cx)? else {
			continue;
		};
		added.push(recipe.name.clone());
		if e.flags.highlight {
			highlighted.push(recipe.name.clone());
		}
		let book = &mut cx.state::<Recipes>().book;
		book.retain(|(i, _)| *i != id);
		book.push((id, recipe));
	}
	let action = match p.replace {
		true => {
			added = cx
				.state::<Recipes>()
				.book
				.iter()
				.map(|(_, r)| r.name.clone())
				.collect();
			a::play_to_client_packet_unlock_recipes_recipes2::Ty::V0(highlighted)
		}
		false => a::play_to_client_packet_unlock_recipes_recipes2::Ty::Default(1),
	};
	Ok(vec![all_recipes(cx), unlock(cx, added, action)])
}

pub fn tricky_recipe_book_remove_down_1_21_1(
	p: Vec<i32>,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let names = p.iter().map(|i| format!("minecraft:{i}")).collect();
	Ok(vec![unlock(
		cx,
		names,
		a::play_to_client_packet_unlock_recipes_recipes2::Ty::Default(2),
	)])
}

pub fn tricky_recipe_book_settings_down_1_21_1(
	p: b::play_to_client_packet_recipe_book_settings::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	cx.state::<Recipes>().settings = [
		p.crafting_gui_open,
		p.crafting_filtering_craftable,
		p.smelting_gui_open,
		p.smelting_filtering_craftable,
		p.blast_gui_open,
		p.blast_filtering_craftable,
		p.smoker_gui_open,
		p.smoker_filtering_craftable,
	];
	Ok(vec![])
}

fn recipe_id(name: &str) -> Option<i32> {
	strip(name).parse().ok()
}

pub fn tricky_craft_recipe_request_1_21_3(
	p: a::play_to_server_packet_craft_recipe_request::Ty,
	_cx: &mut Session,
) -> Result<Vec<ServerB>> {
	let Some(id) = recipe_id(&p.recipe) else {
		return Ok(vec![]);
	};
	Ok(vec![ServerB::CraftRecipeRequest(
		b::play_to_server_packet_craft_recipe_request::Ty {
			window_id: p.window_id as i32,
			recipe_id: id,
			make_all: p.make_all,
		},
	)])
}

pub fn tricky_displayed_recipe_1_21_3(p: String, _cx: &mut Session) -> Result<Vec<ServerB>> {
	Ok(recipe_id(&p)
		.map(ServerB::DisplayedRecipe)
		.into_iter()
		.collect())
}

#[derive(Default)]
struct Movement {
	player: i32,
	vehicle: Option<i32>,
	position: Option<(f64, f64, f64)>,
	rotation: (f32, f32),
	on_ground: bool,
	moved: bool,
	input: (f32, f32, u8),
	sneaking: bool,
}

pub fn tricky_login_entity_1_21_3(
	src: &a::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<i32> {
	cx.state::<Chunks>().loaded.clear();
	*cx.state::<Movement>() = Movement {
		player: src.entity_id,
		..Default::default()
	};
	Ok(src.entity_id)
}

pub fn tricky_login_entity_down_1_21_1(
	src: &b::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<i32> {
	*cx.state::<Movement>() = Movement {
		player: src.entity_id,
		..Default::default()
	};
	Ok(src.entity_id)
}

fn passengers(cx: &mut Session, vehicle: i32, riders: &[i32]) {
	let m = cx.state::<Movement>();
	if riders.contains(&m.player) {
		m.vehicle = Some(vehicle);
	} else if m.vehicle == Some(vehicle) {
		m.vehicle = None;
	}
}

pub fn tricky_passengers_1_21_3(
	p: a::play_to_client_packet_set_passengers::Ty,
	cx: &mut Session,
) -> Result<b::play_to_client_packet_set_passengers::Ty> {
	passengers(cx, p.entity_id, &p.passengers);
	Ok(p)
}

pub fn tricky_passengers_down_1_21_1(
	p: b::play_to_client_packet_set_passengers::Ty,
	cx: &mut Session,
) -> Result<a::play_to_client_packet_set_passengers::Ty> {
	passengers(cx, p.entity_id, &p.passengers);
	Ok(p)
}

fn destroyed(cx: &mut Session, ids: &[i32]) {
	let m = cx.state::<Movement>();
	if m.vehicle.is_some_and(|v| ids.contains(&v)) {
		m.vehicle = None;
	}
}

pub fn tricky_destroy_1_21_3(p: Vec<i32>, cx: &mut Session) -> Result<Vec<ClientB>> {
	destroyed(cx, &p);
	Ok(vec![ClientB::EntityDestroy(p)])
}

pub fn tricky_destroy_down_1_21_1(p: Vec<i32>, cx: &mut Session) -> Result<Vec<ClientA>> {
	destroyed(cx, &p);
	Ok(vec![ClientA::EntityDestroy(p)])
}

fn moved(
	cx: &mut Session,
	position: Option<(f64, f64, f64)>,
	rotation: Option<(f32, f32)>,
	on_ground: bool,
) {
	let m = cx.state::<Movement>();
	m.moved = true;
	m.on_ground = on_ground;
	if position.is_some() {
		m.position = position;
	}
	if let Some(r) = rotation {
		m.rotation = r;
	}
}

pub fn tricky_look_ground_down_1_21_1(
	src: &b::play_to_server_packet_look::Ty,
	cx: &mut Session,
) -> Result<bool> {
	moved(cx, None, Some((src.yaw, src.pitch)), src.flags.on_ground);
	Ok(src.flags.on_ground)
}

pub fn tricky_position_ground_down_1_21_1(
	src: &b::play_to_server_packet_position::Ty,
	cx: &mut Session,
) -> Result<bool> {
	moved(cx, Some((src.x, src.y, src.z)), None, src.flags.on_ground);
	Ok(src.flags.on_ground)
}

pub fn tricky_position_look_ground_down_1_21_1(
	src: &b::play_to_server_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<bool> {
	moved(
		cx,
		Some((src.x, src.y, src.z)),
		Some((src.yaw, src.pitch)),
		src.flags.on_ground,
	);
	Ok(src.flags.on_ground)
}

pub fn tricky_flying_down_1_21_1(flags: Flags, cx: &mut Session) -> Result<Vec<ServerA>> {
	moved(cx, None, None, flags.on_ground);
	Ok(vec![ServerA::Flying(flags.on_ground)])
}

fn ground_flags(on_ground: bool) -> Flags {
	Flags {
		on_ground: on_ground,
		has_horizontal_collision: false,
	}
}

fn tick_end() -> ServerB {
	ServerB::TickEnd(b::play_to_server_packet_tick_end::Ty {})
}

pub fn tricky_look_1_21_3(
	p: a::play_to_server_packet_look::Ty,
	cx: &mut Session,
) -> Result<Vec<ServerB>> {
	moved(cx, None, Some((p.yaw, p.pitch)), p.on_ground);
	Ok(vec![
		ServerB::Look(b::play_to_server_packet_look::Ty {
			yaw: p.yaw,
			pitch: p.pitch,
			flags: ground_flags(p.on_ground),
		}),
		tick_end(),
	])
}

pub fn tricky_position_1_21_3(
	p: a::play_to_server_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<ServerB>> {
	moved(cx, Some((p.x, p.y, p.z)), None, p.on_ground);
	Ok(vec![
		ServerB::Position(b::play_to_server_packet_position::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
			flags: ground_flags(p.on_ground),
		}),
		tick_end(),
	])
}

pub fn tricky_position_look_1_21_3(
	p: a::play_to_server_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<Vec<ServerB>> {
	moved(
		cx,
		Some((p.x, p.y, p.z)),
		Some((p.yaw, p.pitch)),
		p.on_ground,
	);
	Ok(vec![
		ServerB::PositionLook(b::play_to_server_packet_position_look::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			flags: ground_flags(p.on_ground),
		}),
		tick_end(),
	])
}

pub fn tricky_flying_1_21_3(on_ground: bool, cx: &mut Session) -> Result<Vec<ServerB>> {
	moved(cx, None, None, on_ground);
	Ok(vec![ServerB::Flying(ground_flags(on_ground)), tick_end()])
}

const IMPULSE: f32 = 0.98;

fn steer(m: &Movement) -> ServerA {
	ServerA::SteerVehicle(a::play_to_server_packet_steer_vehicle::Ty {
		sideways: m.input.0,
		forward: m.input.1,
		jump: m.input.2,
	})
}

pub fn tricky_player_input_down_1_21_1(
	p: b::play_to_server_packet_player_input::Ty,
	cx: &mut Session,
) -> Result<Vec<ServerA>> {
	let axis = |pos: bool, neg: bool| match (pos, neg) {
		(true, false) => IMPULSE,
		(false, true) => -IMPULSE,
		_ => 0.0,
	};
	let m = cx.state::<Movement>();
	m.input = (
		axis(p.left, p.right),
		axis(p.forward, p.backward),
		p.jump as u8 | (p.shift as u8) << 1,
	);
	Ok(m.vehicle.map(|_| steer(m)).into_iter().collect())
}

fn sneak_action(entity: i32, sneaking: bool) -> ServerB {
	use b::play_to_server_packet_entity_action_action_id::Ty as A;
	ServerB::EntityAction(b::play_to_server_packet_entity_action::Ty {
		entity_id: entity,
		action_id: if sneaking {
			A::StartSneaking
		} else {
			A::StopSneaking
		},
		jump_boost: 0,
	})
}

pub fn tricky_steer_vehicle_1_21_3(
	p: a::play_to_server_packet_steer_vehicle::Ty,
	cx: &mut Session,
) -> Result<Vec<ServerB>> {
	let shift = p.jump & 2 != 0;
	let m = cx.state::<Movement>();
	let mut out = vec![ServerB::PlayerInput(
		b::play_to_server_packet_player_input::Ty {
			forward: p.forward > 0.0,
			backward: p.forward < 0.0,
			left: p.sideways > 0.0,
			right: p.sideways < 0.0,
			jump: p.jump & 1 != 0,
			shift,
			sprint: false,
		},
	)];
	if m.sneaking != shift {
		m.sneaking = shift;
		out.push(sneak_action(m.player, shift));
	}
	Ok(out)
}

pub fn tricky_entity_action_1_21_3(
	p: a::play_to_server_packet_entity_action::Ty,
	cx: &mut Session,
) -> Result<b::play_to_server_packet_entity_action::Ty> {
	use a::play_to_server_packet_entity_action_action_id::Ty as A;
	match p.action_id {
		A::StartSneaking => cx.state::<Movement>().sneaking = true,
		A::StopSneaking => cx.state::<Movement>().sneaking = false,
		_ => {}
	}
	Ok(p)
}

pub fn tricky_tick_end_down_1_21_1(
	_: b::play_to_server_packet_tick_end::Ty,
	cx: &mut Session,
) -> Result<Vec<ServerA>> {
	let m = cx.state::<Movement>();
	let idle = !std::mem::replace(&mut m.moved, false);
	let mut out = Vec::new();
	if m.vehicle.is_some() {
		if idle {
			let (yaw, pitch) = m.rotation;
			out.push(ServerA::Look(a::play_to_server_packet_look::Ty {
				yaw,
				pitch,
				on_ground: m.on_ground,
			}));
		}
		out.push(steer(m));
	} else if idle {
		out.push(ServerA::Flying(m.on_ground));
	}
	Ok(out)
}
