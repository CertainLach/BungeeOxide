use super::step_1_21_1::{attribute, ATTRIBUTES_1_21_1};
use super::*;
use crate::gen::convert::{v1_21_1 as a, v1_21_3 as b};

const FROM: Version = Version::V1_21_1;
const TO: Version = Version::V1_21_3;

type ConfigA = a::configuration_toClient_packet::Ty;
type ConfigB = b::configuration_toClient_packet::Ty;
type ClientA = a::play_toClient_packet::Ty;
type ClientB = b::play_toClient_packet::Ty;
type ServerA = a::play_toServer_packet::Ty;
type ServerB = b::play_toServer_packet::Ty;
type ComponentA = a::SlotComponent::Ty;
type ComponentB = b::SlotComponent::Ty;
type ParticleA = a::Particle::Ty;
type ParticleB = b::Particle::Ty;
type MetaA = a::entityMetadataEntry__value::Ty;
type MetaB = b::entityMetadataEntry__value::Ty;
type Flags = b::play_toServer_MovementFlags::Ty;
type RecipeA = a::play_toClient_packet__declare_recipes__items::Ty;
type RecipeDataA = a::play_toClient_packet__declare_recipes__items__data::Ty;
type Display = b::play_toClient_SlotDisplay::Ty;

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

fn equipment_up(slots: Vec<a::Slot::Ty>, cx: &mut Session) -> Result<Vec<b::Slot::Ty>> {
	if slots.is_empty() {
		return Ok(vec![]);
	}
	let packet = ClientA::entity_equipment(a::play_toClient_packet_entity_equipment::Ty {
		entityId: 0,
		equipments: slots
			.into_iter()
			.map(
				|item| a::play_toClient_packet_entity_equipment__equipments__items::Ty {
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
		a::play_toClient_packet::serialize,
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

fn equipment_down(slots: Vec<b::Slot::Ty>, cx: &mut Session) -> Result<Vec<a::Slot::Ty>> {
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
	carry(
		Root::PlayToClient,
		(TO, FROM),
		&packet,
		b::play_toClient_packet::serialize,
		a::play_toClient_packet::deserialize,
		|p| match p {
			ClientA::entity_equipment(e) => {
				Some(e.equipments.into_iter().map(|e| e.item).collect())
			}
			_ => None,
		},
		cx,
	)
}

fn slot_up(slot: a::Slot::Ty, cx: &mut Session) -> Result<b::Slot::Ty> {
	let source = slot.clone();
	let mut out = equipment_up(vec![slot], cx)?.pop().ok_or(Error::Dropped)?;
	split_food(&source, &mut out, cx)?;
	Ok(out)
}

fn slot_down(slot: b::Slot::Ty, cx: &mut Session) -> Result<a::Slot::Ty> {
	equipment_down(vec![slot], cx)?.pop().ok_or(Error::Dropped)
}

fn particle_up(particle: ParticleA, cx: &mut Session) -> Result<ParticleB> {
	let packet = ClientA::world_particles(a::play_toClient_packet_world_particles::Ty {
		longDistance: false,
		x: 0.0,
		y: 0.0,
		z: 0.0,
		offsetX: 0.0,
		offsetY: 0.0,
		offsetZ: 0.0,
		velocityOffset: 0.0,
		amount: 0,
		particle,
	});
	carry(
		Root::PlayToClient,
		(FROM, TO),
		&packet,
		a::play_toClient_packet::serialize,
		b::play_toClient_packet::deserialize,
		|p| match p {
			ClientB::world_particles(w) => Some(w.particle),
			_ => None,
		},
		cx,
	)
}

fn particle_down(particle: ParticleB, cx: &mut Session) -> Result<ParticleA> {
	let packet = ClientB::world_particles(b::play_toClient_packet_world_particles::Ty {
		longDistance: false,
		x: 0.0,
		y: 0.0,
		z: 0.0,
		offsetX: 0.0,
		offsetY: 0.0,
		offsetZ: 0.0,
		velocityOffset: 0.0,
		amount: 0,
		particle,
	});
	carry(
		Root::PlayToClient,
		(TO, FROM),
		&packet,
		b::play_toClient_packet::serialize,
		a::play_toClient_packet::deserialize,
		|p| match p {
			ClientA::world_particles(w) => Some(w.particle),
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

fn split_food(source: &a::Slot::Ty, target: &mut b::Slot::Ty, cx: &mut Session) -> Result<()> {
	let (a::Slot::Ty::default(_, s), b::Slot::Ty::default(_, t)) = (source, target) else {
		return Ok(());
	};
	let Some(food) = s.components.iter().find_map(|c| match c {
		ComponentA::food(f) => Some(f),
		_ => None,
	}) else {
		return Ok(());
	};
	if t.components
		.iter()
		.any(|c| matches!(c, ComponentB::consumable(_)))
	{
		return Ok(());
	}
	t.components
		.push(ComponentB::consumable(b::SlotComponent__consumable::Ty {
			consume_seconds: food.secondsToEat,
			animation: b::SlotComponent__consumable__animation::Ty::eat,
			sound: SoundHolder::Inline(v1_21_1::ItemSoundEvent::Ty {
				soundName: "minecraft:entity.generic.eat".to_owned(),
				fixedRange: None,
			}),
			makes_particles: true,
			effects: food
				.effects
				.iter()
				.map(|e| {
					b::ItemConsumeEffect::Ty::apply_effects(
						b::ItemConsumeEffect__apply_effects::Ty {
							effects: vec![e.effect.clone()],
							probability: e.probability,
						},
					)
				})
				.collect(),
		}));
	if !matches!(food.usingConvertsTo, a::Slot::Ty::v0) {
		let remainder = slot_up(food.usingConvertsTo.clone(), cx)?;
		t.components.push(ComponentB::use_remainder(remainder));
	}
	t.addedComponentCount = t.components.len() as i32;
	Ok(())
}

#[derive(Default)]
struct FoodStash(
	Vec<(
		f32,
		Option<b::Slot::Ty>,
		Vec<a::SlotComponent__food__effects__items::Ty>,
	)>,
);

fn stash_food(cx: &mut Session, components: &[ComponentB]) {
	if !components.iter().any(|c| matches!(c, ComponentB::food(_))) {
		return;
	}
	let mut seconds = 1.6;
	let mut remainder = None;
	let mut effects = Vec::new();
	for c in components {
		match c {
			ComponentB::consumable(c) => {
				seconds = c.consume_seconds;
				for e in &c.effects {
					if let b::ItemConsumeEffect::Ty::apply_effects(a) = e {
						effects.extend(a.effects.iter().map(|effect| {
							a::SlotComponent__food__effects__items::Ty {
								effect: effect.clone(),
								probability: a.probability,
							}
						}));
					}
				}
			}
			ComponentB::use_remainder(s) => remainder = Some(s.clone()),
			_ => {}
		}
	}
	cx.state::<FoodStash>()
		.0
		.push((seconds, remainder, effects));
}

pub fn tricky_registry_data_1_21_3(p: RegistryData, cx: &mut Session) -> Result<Vec<ConfigB>> {
	Ok(vec![ConfigB::registry_data(registry_up(
		cx,
		(FROM, TO),
		p,
		registry_fix_up,
	))])
}

pub fn tricky_registry_data_down_1_21_1(p: RegistryData, cx: &mut Session) -> Result<Vec<ConfigA>> {
	Ok(vec![ConfigA::registry_data(registry_down(
		cx,
		(TO, FROM),
		p,
		registry_fix_down,
	)?)])
}

fn dust_colors(registry: &str, tag: &mut pds_Tag::Ty, to_int: bool) {
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
				Payload::list(List {
					items: ListItems::float(c),
					..
				}),
				true,
			) if c.len() == 3 => {
				let rgb = argb(c[0], c[1], c[2]) & 0xffffff;
				*v = Payload::int(rgb);
			}
			(Payload::int(c), false) => {
				let (r, g, b) = rgb(*c);
				*v = ListItems::float(vec![r, g, b]).into();
			}
			_ => {}
		}
	}
}

fn registry_fix_up(registry: &str, tag: &mut pds_Tag::Ty) {
	dust_colors(registry, tag, true);
	if strip(registry) == "enchantment" {
		rename_strings(&mut tag.value, "damage_item", "change_item_damage");
	}
}

fn registry_fix_down(registry: &str, tag: &mut pds_Tag::Ty) {
	dust_colors(registry, tag, false);
	if strip(registry) == "enchantment" {
		rename_strings(&mut tag.value, "change_item_damage", "damage_item");
	}
}

pub fn tricky_finish_configuration_1_21_3(
	p: a::configuration_toClient_packet_finish_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<ConfigB>> {
	let mut out: Vec<ConfigB> = missing_registries(cx, TO, registry_fix_up)
		.into_iter()
		.map(ConfigB::registry_data)
		.collect();
	out.push(ConfigB::finish_configuration(p));
	Ok(out)
}

pub fn tricky_feature_flags_1_21_3(p: Vec<String>, _cx: &mut Session) -> Result<Vec<ConfigB>> {
	Ok(vec![ConfigB::feature_flags(features(p, &[], &["bundle"]))])
}

pub fn tricky_feature_flags_down_1_21_1(p: Vec<String>, _cx: &mut Session) -> Result<Vec<ConfigA>> {
	Ok(vec![ConfigA::feature_flags(features(p, &["bundle"], &[]))])
}

pub fn tricky_config_tags_1_21_3(
	mut p: Vec<a::configuration_toClient_packet__tags__items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<ConfigB>> {
	for g in &mut p {
		remap_tags(FROM, TO, &g.tagType, &mut g.tags);
	}
	Ok(vec![ConfigB::tags(p)])
}

pub fn tricky_config_tags_down_1_21_1(
	mut p: Vec<a::configuration_toClient_packet__tags__items::Ty>,
	cx: &mut Session,
) -> Result<Vec<ConfigA>> {
	for g in &mut p {
		store_item_tags(cx, TO, &g.tagType, &g.tags);
		remap_tags(TO, FROM, &g.tagType, &mut g.tags);
	}
	Ok(vec![ConfigA::tags(p)])
}

pub fn tricky_play_tags_1_21_3(
	mut p: Vec<a::play_toClient_packet__tags__items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<ClientB>> {
	for g in &mut p {
		remap_tags(FROM, TO, &g.tagType, &mut g.tags);
	}
	Ok(vec![ClientB::tags(p)])
}

pub fn tricky_play_tags_down_1_21_1(
	mut p: Vec<a::play_toClient_packet__tags__items::Ty>,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	for g in &mut p {
		store_item_tags(cx, TO, &g.tagType, &g.tags);
		remap_tags(TO, FROM, &g.tagType, &mut g.tags);
	}
	Ok(vec![ClientA::tags(p)])
}

pub fn tricky_strict_errors_down_1_21_1(
	_src: &b::login_toClient_packet_success::Ty,
	_cx: &mut Session,
) -> Result<bool> {
	Ok(true)
}

pub fn tricky_item_id_1_21_3(src: &a::Slot__default::Ty, _cx: &mut Session) -> Result<i32> {
	Ok(item_up(src.itemId))
}

pub fn tricky_item_id_down_1_21_1(src: &b::Slot__default::Ty, _cx: &mut Session) -> Result<i32> {
	Ok(item_down(src.itemId))
}

fn kept_down(c: &ComponentB) -> bool {
	use ComponentB as C;
	match c {
		C::damage_resistant(t) => strip(t) == "is_fire",
		C::consumable(_)
		| C::use_remainder(_)
		| C::use_cooldown(_)
		| C::enchantable(_)
		| C::equippable(_)
		| C::repairable(_)
		| C::glider
		| C::tooltip_style(_)
		| C::death_protection(_)
		| C::item_model(_) => false,
		_ => true,
	}
}

fn kept_type_down(c: &b::SlotComponentType::Ty) -> bool {
	use b::SlotComponentType::Ty as C;
	!matches!(
		c,
		C::consumable
			| C::use_remainder
			| C::use_cooldown
			| C::enchantable
			| C::equippable
			| C::repairable
			| C::glider
			| C::tooltip_style
			| C::death_protection
			| C::item_model
	)
}

pub fn tricky_added_components_down_1_21_1(
	src: &b::Slot__default::Ty,
	cx: &mut Session,
) -> Result<i32> {
	stash_food(cx, &src.components);
	Ok(src.components.iter().filter(|c| kept_down(c)).count() as i32)
}

pub fn tricky_removed_components_down_1_21_1(
	src: &b::Slot__default::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(src
		.removeComponents
		.iter()
		.filter(|c| kept_type_down(c))
		.count() as i32)
}

pub fn tricky_trade_components_down_1_21_1(
	src: &b::play_toClient_packet_trade_list__trades__items__inputItem1::Ty,
	cx: &mut Session,
) -> Result<i32> {
	stash_food(cx, &src.components);
	Ok(src.components.iter().filter(|c| kept_down(c)).count() as i32)
}

pub fn tricky_trade_components2_down_1_21_1(
	src: &b::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty,
	cx: &mut Session,
) -> Result<i32> {
	stash_food(cx, &src.components);
	Ok(src.components.iter().filter(|c| kept_down(c)).count() as i32)
}

pub fn tricky_trade_item_1_21_3(
	src: &a::play_toClient_packet_trade_list__trades__items__inputItem1::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_up(src.itemId))
}

pub fn tricky_trade_item_down_1_21_1(
	src: &b::play_toClient_packet_trade_list__trades__items__inputItem1::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_down(src.itemId))
}

pub fn tricky_trade_item2_1_21_3(
	src: &a::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_up(src.itemId))
}

pub fn tricky_trade_item2_down_1_21_1(
	src: &b::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_down(src.itemId))
}

pub fn tricky_fire_resistant_1_21_3(_: (), _cx: &mut Session) -> Result<ComponentB> {
	Ok(ComponentB::damage_resistant("minecraft:is_fire".to_owned()))
}

pub fn tricky_damage_resistant_down_1_21_1(types: String, _cx: &mut Session) -> Result<ComponentA> {
	match strip(&types) {
		"is_fire" => Ok(ComponentA::fire_resistant),
		_ => Err(Error::Dropped),
	}
}

pub fn tricky_enchantments_1_21_3(
	v: a::SlotComponent__enchantments::Ty,
	_cx: &mut Session,
) -> Result<ComponentB> {
	let had = !v.enchantments.is_empty();
	let kept: Vec<_> = v
		.enchantments
		.into_iter()
		.filter(|e| e.level > 0)
		.map(
			|e| b::SlotComponent__enchantments__enchantments__items::Ty {
				id: e.id,
				level: e.level,
			},
		)
		.collect();
	if had && kept.is_empty() {
		return Ok(ComponentB::enchantment_glint_override(true));
	}
	Ok(ComponentB::enchantments(
		b::SlotComponent__enchantments::Ty {
			enchantments: kept,
			showTooltip: v.showTooltip,
		},
	))
}

fn instrument_data_up(d: a::InstrumentData::Ty) -> b::InstrumentData::Ty {
	b::InstrumentData::Ty {
		soundEvent: d.soundEvent,
		useDuration: d.useDuration as f32 / 20.0,
		range: d.range,
		description: Tag::new(""),
	}
}

fn instrument_data_down(d: b::InstrumentData::Ty) -> a::InstrumentData::Ty {
	a::InstrumentData::Ty {
		soundEvent: d.soundEvent,
		useDuration: (d.useDuration * 20.0) as i32,
		range: d.range,
	}
}

pub fn tricky_instrument_1_21_3(
	p: a::SlotComponent__instrument::Ty,
	cx: &mut Session,
) -> Result<ComponentB> {
	Ok(ComponentB::instrument(match p {
		a::SlotComponent__instrument::Ty::Id(id) => {
			let name = usize::try_from(id).ok().and_then(|i| GOAT_HORNS.get(i));
			b::SlotComponent__instrument::Ty::Id(
				name.and_then(|n| registry_index(cx, TO, "instrument", n))
					.unwrap_or(id),
			)
		}
		a::SlotComponent__instrument::Ty::Inline(data) => {
			b::SlotComponent__instrument::Ty::Inline(instrument_data_up(data))
		}
	}))
}

pub fn tricky_instrument_down_1_21_1(
	p: b::SlotComponent__instrument::Ty,
	cx: &mut Session,
) -> Result<ComponentA> {
	Ok(ComponentA::instrument(match p {
		b::SlotComponent__instrument::Ty::Id(id) => {
			let name = registry_key(cx, TO, "instrument", id);
			a::SlotComponent__instrument::Ty::Id(
				GOAT_HORNS
					.iter()
					.position(|n| Some(*n) == name.as_deref())
					.map_or(id, |i| i as i32),
			)
		}
		b::SlotComponent__instrument::Ty::Inline(data) => {
			a::SlotComponent__instrument::Ty::Inline(instrument_data_down(data))
		}
	}))
}

pub fn tricky_pot_decorations_1_21_3(p: Vec<i32>, _cx: &mut Session) -> Result<ComponentB> {
	Ok(ComponentB::pot_decorations(
		p.into_iter().map(item_up).collect(),
	))
}

pub fn tricky_pot_decorations_down_1_21_1(p: Vec<i32>, _cx: &mut Session) -> Result<ComponentA> {
	Ok(ComponentA::pot_decorations(
		p.into_iter().map(item_down).collect(),
	))
}

pub fn tricky_potion_name_1_21_3(
	_src: &a::SlotComponent__potion_contents::Ty,
	_cx: &mut Session,
) -> Result<Option<String>> {
	Ok(None)
}

pub fn tricky_painting_title_1_21_3(
	_src: &a::EntityMetadataPaintingVariant::Ty,
	_cx: &mut Session,
) -> Result<Option<pds_Tag::Ty>> {
	Ok(None)
}

pub fn tricky_painting_author_1_21_3(
	_src: &a::EntityMetadataPaintingVariant::Ty,
	_cx: &mut Session,
) -> Result<Option<pds_Tag::Ty>> {
	Ok(None)
}

pub fn tricky_attribute_type_1_21_3(
	src: &a::SlotComponent__attribute_modifiers__attributes__items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	attribute(ATTRIBUTES_1_21_1, ATTRIBUTES_1_21_3, src.typeId)
}

pub fn tricky_attribute_type_down_1_21_1(
	src: &b::SlotComponent__attribute_modifiers__attributes__items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	attribute(ATTRIBUTES_1_21_3, ATTRIBUTES_1_21_1, src.typeId)
}

fn stashed(
	cx: &mut Session,
) -> Option<&mut (
	f32,
	Option<b::Slot::Ty>,
	Vec<a::SlotComponent__food__effects__items::Ty>,
)> {
	cx.state::<FoodStash>().0.last_mut()
}

pub fn tricky_food_seconds_down_1_21_1(
	_src: &b::SlotComponent__food::Ty,
	cx: &mut Session,
) -> Result<f32> {
	Ok(stashed(cx).map_or(1.6, |s| s.0))
}

pub fn tricky_food_remainder_down_1_21_1(
	_src: &b::SlotComponent__food::Ty,
	cx: &mut Session,
) -> Result<a::Slot::Ty> {
	match stashed(cx).and_then(|s| s.1.take()) {
		Some(slot) => slot_down(slot, cx),
		None => Ok(a::Slot::Ty::v0),
	}
}

pub fn tricky_food_effects_down_1_21_1(
	_src: &b::SlotComponent__food::Ty,
	cx: &mut Session,
) -> Result<a::SlotComponent__food__effects::Ty> {
	Ok(cx
		.state::<FoodStash>()
		.0
		.pop()
		.map(|s| s.2)
		.unwrap_or_default())
}

pub fn tricky_tool_blocks_1_21_3(
	src: &a::SlotComponent__tool__rules__items::Ty,
	_cx: &mut Session,
) -> Result<IdSet> {
	Ok(id_set(src.blocks.clone(), |id| block(FROM, TO, id)))
}

pub fn tricky_tool_blocks_down_1_21_1(
	src: &b::SlotComponent__tool__rules__items::Ty,
	_cx: &mut Session,
) -> Result<IdSet> {
	Ok(id_set(src.blocks.clone(), |id| block(TO, FROM, id)))
}

type BlockSet = a::ItemBlockPredicate__blockSet__items::Ty;

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
	src: &a::ItemBlockPredicate::Ty,
	_cx: &mut Session,
) -> Result<Option<BlockSet>> {
	Ok(block_set(src.blockSet.clone(), |id| block(FROM, TO, id)))
}

pub fn tricky_predicate_blocks_down_1_21_1(
	src: &b::ItemBlockPredicate::Ty,
	_cx: &mut Session,
) -> Result<Option<BlockSet>> {
	Ok(block_set(src.blockSet.clone(), |id| block(TO, FROM, id)))
}

fn argb(r: f32, g: f32, b: f32) -> i32 {
	let c = |v: f32| ((v * 255.0) as i32).clamp(0, 255);
	(255 << 24) | (c(r) << 16) | (c(g) << 8) | c(b)
}

fn rgb(c: i32) -> (f32, f32, f32) {
	let ch = |s: u32| ((c >> s) & 0xff) as f32 / 255.0;
	(ch(16), ch(8), ch(0))
}

pub fn tricky_dust_1_21_3(p: a::Particle__dust::Ty, _cx: &mut Session) -> Result<ParticleB> {
	Ok(ParticleB::dust(b::Particle__dust::Ty {
		color: argb(p.red, p.green, p.blue),
		scale: p.scale,
	}))
}

pub fn tricky_dust_down_1_21_1(p: b::Particle__dust::Ty, _cx: &mut Session) -> Result<ParticleA> {
	let (red, green, blue) = rgb(p.color);
	Ok(ParticleA::dust(a::Particle__dust::Ty {
		red,
		green,
		blue,
		scale: p.scale,
	}))
}

pub fn tricky_dust_transition_1_21_3(
	p: a::Particle__dust_color_transition::Ty,
	_cx: &mut Session,
) -> Result<ParticleB> {
	Ok(ParticleB::dust_color_transition(
		b::Particle__dust_color_transition::Ty {
			fromColor: argb(p.fromRed, p.fromGreen, p.fromBlue),
			toColor: argb(p.toRed, p.toGreen, p.toBlue),
			scale: p.scale,
		},
	))
}

pub fn tricky_dust_transition_down_1_21_1(
	p: b::Particle__dust_color_transition::Ty,
	_cx: &mut Session,
) -> Result<ParticleA> {
	let (fromRed, fromGreen, fromBlue) = rgb(p.fromColor);
	let (toRed, toGreen, toBlue) = rgb(p.toColor);
	Ok(ParticleA::dust_color_transition(
		a::Particle__dust_color_transition::Ty {
			fromRed,
			fromGreen,
			fromBlue,
			scale: p.scale,
			toRed,
			toGreen,
			toBlue,
		},
	))
}

pub fn tricky_trail_down_1_21_1(p: b::Particle__trail::Ty, _cx: &mut Session) -> Result<ParticleA> {
	Ok(ParticleA::entity_effect(p.color))
}

pub fn tricky_block_crumble_down_1_21_1(state: i32, _cx: &mut Session) -> Result<ParticleA> {
	Ok(ParticleA::block(state_down(state)))
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
	block: tricky_particle_block_1_21_3, tricky_particle_block_down_1_21_1;
	block_marker: tricky_particle_block_marker_1_21_3, tricky_particle_block_marker_down_1_21_1;
	falling_dust: tricky_particle_falling_dust_1_21_3, tricky_particle_falling_dust_down_1_21_1;
	dust_pillar: tricky_particle_dust_pillar_1_21_3, tricky_particle_dust_pillar_down_1_21_1;
}

pub fn tricky_meta_entity_1_21_3(
	src: &a::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	metadata_of(cx, FROM, src.entityId);
	Ok(src.entityId)
}

pub fn tricky_meta_entity_down_1_21_1(
	src: &b::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	metadata_of(cx, TO, src.entityId);
	Ok(src.entityId)
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

pub fn tricky_meta_key_1_21_3(src: &a::entityMetadataEntry::Ty, cx: &mut Session) -> Result<u8> {
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
	src: &b::entityMetadataEntry::Ty,
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
	Ok(MetaB::block_state(state_up(state)))
}

pub fn tricky_meta_block_down_1_21_1(state: i32, _cx: &mut Session) -> Result<MetaA> {
	Ok(MetaA::block_state(state_down(state)))
}

pub fn tricky_meta_optional_block_1_21_3(state: i32, _cx: &mut Session) -> Result<MetaB> {
	Ok(MetaB::optional_block_state(state_up(state)))
}

pub fn tricky_meta_optional_block_down_1_21_1(state: i32, _cx: &mut Session) -> Result<MetaA> {
	Ok(MetaA::optional_block_state(state_down(state)))
}

pub fn tricky_meta_int_1_21_3(v: i32, cx: &mut Session) -> Result<MetaB> {
	Ok(MetaB::int(match current_entry(cx, FROM) {
		(ty, 11) if is_minecart(ty) => state_up(v),
		_ => v,
	}))
}

pub fn tricky_meta_int_down_1_21_1(v: i32, cx: &mut Session) -> Result<MetaA> {
	Ok(MetaA::int(match current_entry(cx, TO) {
		(ty, 11) if is_minecart(ty) => state_down(v),
		_ => v,
	}))
}

pub fn tricky_entity_type_1_21_3(
	src: &a::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<i32> {
	let id = remap("entities", FROM, TO, src.r#type).ok_or(Error::Dropped)?;
	spawned(cx, FROM, src.entityId, entity_name(FROM, src.r#type));
	spawned(cx, TO, src.entityId, entity_name(TO, id));
	Ok(id)
}

pub fn tricky_object_data_1_21_3(
	src: &a::play_toClient_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(match entity_name(FROM, src.r#type) {
		Some("falling_block") => state_up(src.objectData),
		_ => src.objectData,
	})
}

pub fn tricky_spawn_entity_down_1_21_1(
	mut p: b::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let name = entity_name(TO, p.r#type);
	let id = remap("entities", TO, FROM, p.r#type).ok_or(Error::Dropped)?;
	spawned(cx, TO, p.entityId, name);
	spawned(cx, FROM, p.entityId, entity_name(FROM, id));
	if name == Some("falling_block") {
		p.objectData = state_down(p.objectData);
	}
	p.r#type = id;
	let entity = p.entityId;
	let mut out = vec![ClientA::spawn_entity(p)];
	if let Some(name) = name.filter(|n| boat(n)) {
		let wood = BOAT_WOODS
			.iter()
			.position(|w| name.starts_with(&format!("{w}_")))
			.unwrap_or(0);
		out.push(ClientA::entity_metadata(
			a::play_toClient_packet_entity_metadata::Ty {
				entityId: entity,
				metadata: vec![a::entityMetadataEntry::Ty {
					key: 11,
					value: MetaA::int(wood as i32),
				}],
			},
		));
	}
	Ok(out)
}

pub fn tricky_block_change_1_21_3(
	src: &a::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state_up(src.r#type))
}

pub fn tricky_block_change_down_1_21_1(
	src: &b::play_toClient_packet_block_change::Ty,
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
	src: &a::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	Ok(records(&src.records, state_up))
}

pub fn tricky_multi_block_down_1_21_1(
	src: &b::play_toClient_packet_multi_block_change::Ty,
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
	mut p: a::play_toClient_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientB>> {
	p.chunkData = chunk(FROM, TO, &p.chunkData)?;
	for e in &mut p.blockEntities {
		e.r#type = block_entity_up(e.r#type);
	}
	let (x, z) = (p.x, p.z);
	if !tricky_resend_1_21_3(cx, x, z) {
		return Ok(vec![ClientB::map_chunk(p)]);
	}
	Ok(vec![
		ClientB::bundle_delimiter,
		ClientB::unload_chunk(b::play_toClient_packet_unload_chunk::Ty {
			chunkX: x,
			chunkZ: z,
		}),
		ClientB::map_chunk(p),
		ClientB::bundle_delimiter,
	])
}

pub fn tricky_chunk_down_1_21_1(
	src: &b::play_toClient_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	chunk(TO, FROM, &src.chunkData)
}

pub fn tricky_unload_chunk_1_21_3(
	p: a::play_toClient_packet_unload_chunk::Ty,
	cx: &mut Session,
) -> Result<b::play_toClient_packet_unload_chunk::Ty> {
	cx.state::<Chunks>().loaded.remove(&(p.chunkX, p.chunkZ));
	Ok(p)
}

pub fn tricky_bundle_1_21_3(_: (), cx: &mut Session) -> Result<Vec<ClientB>> {
	let chunks = cx.state::<Chunks>();
	chunks.bundling = !chunks.bundling;
	Ok(vec![ClientB::bundle_delimiter])
}

pub fn tricky_respawn_1_21_3(
	src: &a::play_toClient_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<u8> {
	cx.state::<Chunks>().loaded.clear();
	Ok(src.copyMetadata)
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

pub fn tricky_block_entity_1_21_3(src: &a::chunkBlockEntity::Ty, _cx: &mut Session) -> Result<i32> {
	Ok(block_entity_up(src.r#type))
}

pub fn tricky_block_entity_down_1_21_1(
	src: &b::chunkBlockEntity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block_entity_down(src.r#type)
}

pub fn tricky_tile_entity_1_21_3(
	src: &a::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(block_entity_up(src.action))
}

pub fn tricky_tile_entity_down_1_21_1(
	src: &b::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	block_entity_down(src.action)
}

pub fn tricky_world_event_1_21_3(
	src: &a::play_toClient_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(if src.effectId == 2001 {
		state_up(src.data)
	} else {
		src.data
	})
}

pub fn tricky_world_event_down_1_21_1(
	src: &b::play_toClient_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(if src.effectId == 2001 {
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
	src: &a::play_toClient_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<String> {
	item_name(TO, item_up(src.itemID)).ok_or(Error::Dropped)
}

pub fn tricky_cooldown_item_down_1_21_1(
	src: &b::play_toClient_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	item_id(TO, &src.cooldownGroup)
		.map(item_down)
		.ok_or(Error::Dropped)
}

pub fn tricky_tick_day_time_1_21_3(
	src: &a::play_toClient_packet_update_time::Ty,
	_cx: &mut Session,
) -> Result<bool> {
	Ok(src.time >= 0)
}

pub fn tricky_day_time_1_21_3(
	src: &a::play_toClient_packet_update_time::Ty,
	_cx: &mut Session,
) -> Result<i64> {
	Ok(src.time.wrapping_abs())
}

pub fn tricky_day_time_down_1_21_1(
	src: &b::play_toClient_packet_update_time::Ty,
	_cx: &mut Session,
) -> Result<i64> {
	Ok(match (src.tickDayTime, src.time) {
		(true, t) => t,
		(false, 0) => -1,
		(false, t) => -t,
	})
}

pub fn tricky_list_priority_1_21_3(
	_src: &a::play_toClient_packet_player_info__data__items::Ty,
	_cx: &mut Session,
) -> Result<b::play_toClient_packet_player_info__data__items__listPriority::Ty> {
	Ok(b::play_toClient_packet_player_info__data__items__listPriority::Ty::default(false))
}

#[derive(Default)]
struct Inventory {
	state_id: i32,
}

pub fn tricky_window_items_1_21_3(
	p: a::play_toClient_packet_window_items::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientB>> {
	let mut sources = p.items.clone();
	sources.push(p.carriedItem.clone());
	let mut slots = p.items;
	slots.push(p.carriedItem);
	let mut slots = equipment_up(slots, cx)?;
	for (s, t) in sources.iter().zip(slots.iter_mut()) {
		split_food(s, t, cx)?;
	}
	let carried = slots.pop().ok_or(Error::Dropped)?;
	Ok(vec![ClientB::window_items(
		b::play_toClient_packet_window_items::Ty {
			windowId: p.windowId as i32,
			stateId: p.stateId,
			items: slots,
			carriedItem: carried,
		},
	)])
}

pub fn tricky_window_items_down_1_21_1(
	p: b::play_toClient_packet_window_items::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	cx.state::<Inventory>().state_id = p.stateId;
	let mut slots = p.items;
	slots.push(p.carriedItem);
	let mut slots = equipment_down(slots, cx)?;
	let carried = slots.pop().ok_or(Error::Dropped)?;
	Ok(vec![ClientA::window_items(
		a::play_toClient_packet_window_items::Ty {
			windowId: p.windowId as u8,
			stateId: p.stateId,
			items: slots,
			carriedItem: carried,
		},
	)])
}

pub fn tricky_set_slot_1_21_3(
	p: a::play_toClient_packet_set_slot::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientB>> {
	let item = slot_up(p.item, cx)?;
	Ok(vec![match p.windowId as i8 {
		-1 => ClientB::set_cursor_item(item),
		-2 => ClientB::set_player_inventory(b::play_toClient_packet_set_player_inventory::Ty {
			slotId: p.slot as i32,
			contents: item,
		}),
		id => ClientB::set_slot(b::play_toClient_packet_set_slot::Ty {
			windowId: id as i32,
			stateId: p.stateId,
			slot: p.slot,
			item,
		}),
	}])
}

pub fn tricky_set_slot_down_1_21_1(
	p: b::play_toClient_packet_set_slot::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	cx.state::<Inventory>().state_id = p.stateId;
	let item = slot_down(p.item, cx)?;
	let mut out = vec![ClientA::set_slot(a::play_toClient_packet_set_slot::Ty {
		windowId: p.windowId as u8,
		stateId: p.stateId,
		slot: p.slot,
		item: item.clone(),
	})];
	if p.windowId == 0 && (5..=8).contains(&p.slot) {
		out.push(ClientA::entity_equipment(
			a::play_toClient_packet_entity_equipment::Ty {
				entityId: cx.state::<Movement>().player,
				equipments: vec![
					a::play_toClient_packet_entity_equipment__equipments__items::Ty {
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
	item: b::Slot::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let state_id = cx.state::<Inventory>().state_id;
	Ok(vec![ClientA::set_slot(
		a::play_toClient_packet_set_slot::Ty {
			windowId: 255,
			stateId: state_id,
			slot: -1,
			item: slot_down(item, cx)?,
		},
	)])
}

pub fn tricky_set_player_inventory_down_1_21_1(
	p: b::play_toClient_packet_set_player_inventory::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	Ok(vec![ClientA::set_slot(
		a::play_toClient_packet_set_slot::Ty {
			windowId: 254,
			stateId: 0,
			slot: p.slotId as i16,
			item: slot_down(p.contents, cx)?,
		},
	)])
}

pub fn tricky_teleport_1_21_3(
	p: a::play_toClient_packet_entity_teleport::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientB>> {
	Ok(vec![ClientB::sync_entity_position(
		b::play_toClient_packet_sync_entity_position::Ty {
			entityId: p.entityId,
			x: p.x,
			y: p.y,
			z: p.z,
			dx: 0.0,
			dy: 0.0,
			dz: 0.0,
			yaw: angle(p.yaw),
			pitch: angle(p.pitch),
			onGround: p.onGround,
		},
	)])
}

pub fn tricky_sync_position_down_1_21_1(
	p: b::play_toClient_packet_sync_entity_position::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientA>> {
	Ok(vec![ClientA::entity_teleport(
		a::play_toClient_packet_entity_teleport::Ty {
			entityId: p.entityId,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: to_angle(p.yaw),
			pitch: to_angle(p.pitch),
			onGround: p.onGround,
		},
	)])
}

fn velocity_packet(entity: i32, (x, y, z): (f64, f64, f64)) -> ClientA {
	ClientA::entity_velocity(a::play_toClient_packet_entity_velocity::Ty {
		entityId: entity,
		velocity: a::vec3i16::Ty {
			x: velocity(x),
			y: velocity(y),
			z: velocity(z),
		},
	})
}

fn knockback_packet((x, y, z): (f64, f64, f64)) -> ClientA {
	ClientA::explosion(a::play_toClient_packet_explosion::Ty {
		x: 0.0,
		y: 20000.0,
		z: 0.0,
		radius: 0.0,
		affectedBlockOffsets: vec![],
		playerMotionX: x as f32,
		playerMotionY: y as f32,
		playerMotionZ: z as f32,
		block_interaction_type: 0,
		small_explosion_particle: ParticleA::angry_villager,
		large_explosion_particle: ParticleA::angry_villager,
		sound: SoundHolder::Inline(v1_21_1::ItemSoundEvent::Ty {
			soundName: "minecraft:intentionally_empty".to_owned(),
			fixedRange: None,
		}),
	})
}

fn motion(
	entity: i32,
	flags: &b::play_toClient_PositionUpdateRelatives::Ty,
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
	p: b::play_toClient_packet_position::Ty,
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
	let mut out = vec![ClientA::position(a::play_toClient_packet_position::Ty {
		x: p.x,
		y: p.y,
		z: p.z,
		yaw: p.yaw,
		pitch: p.pitch,
		flags: a::play_toClient_PositionUpdateRelatives::Ty {
			x: f.x,
			y: f.y,
			z: f.z,
			yaw: f.yaw,
			pitch: f.pitch,
		},
		teleportId: p.teleportId,
	})];
	out.extend(motion(player, f, (p.dx, p.dy, p.dz), true));
	Ok(out)
}

pub fn tricky_teleport_down_1_21_1(
	p: b::play_toClient_packet_entity_teleport::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let f = &p.flags;
	let m = cx.state::<Movement>();
	let player = p.entityId == m.player;
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
	let mut out = vec![ClientA::entity_teleport(
		a::play_toClient_packet_entity_teleport::Ty {
			entityId: p.entityId,
			x,
			y,
			z,
			yaw: to_angle(yaw),
			pitch: to_angle(pitch),
			onGround: p.onGround,
		},
	)];
	out.extend(motion(p.entityId, f, (p.dx, p.dy, p.dz), player));
	Ok(out)
}

pub fn tricky_move_minecart_down_1_21_1(
	p: b::play_toClient_packet_move_minecart::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let Some(step) = p.steps.last() else {
		return Ok(vec![]);
	};
	Ok(vec![
		ClientA::entity_teleport(a::play_toClient_packet_entity_teleport::Ty {
			entityId: p.entityId,
			x: step.position.x,
			y: step.position.y,
			z: step.position.z,
			yaw: (128 - i32::from(step.yaw)) as i8,
			pitch: step.pitch,
			onGround: false,
		}),
		velocity_packet(
			p.entityId,
			(step.movement.x, step.movement.y, step.movement.z),
		),
	])
}

pub fn tricky_player_rotation_down_1_21_1(
	p: b::play_toClient_packet_player_rotation::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let m = cx.state::<Movement>();
	let (x, y, z) = m.position.unwrap_or((0.0, 0.0, 0.0));
	m.rotation = (p.yaw, p.pitch);
	let on_ground = m.on_ground;
	let (yaw, pitch) = (p.yaw.to_radians() as f64, p.pitch.to_radians() as f64);
	let factor = -(-pitch).cos();
	let look = ClientA::face_player(a::play_toClient_packet_face_player::Ty {
		feet_eyes: 0,
		x: x + (-yaw - std::f64::consts::PI).sin() * factor,
		y: y + (-pitch).sin(),
		z: z + (-yaw - std::f64::consts::PI).cos() * factor,
		isEntity: a::play_toClient_packet_face_player__isEntity::Ty::default(false),
	});
	let rotation = ServerB::look(b::play_toServer_packet_look::Ty {
		yaw: p.yaw,
		pitch: p.pitch,
		flags: ground_flags(on_ground),
	});
	send(
		cx,
		Root::PlayToServer,
		TO,
		&rotation,
		b::play_toServer_packet::serialize,
	)?;
	Ok(vec![look])
}

pub fn tricky_explosion_1_21_3(
	p: a::play_toClient_packet_explosion::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientB>> {
	let knockback = (p.playerMotionX != 0.0 || p.playerMotionY != 0.0 || p.playerMotionZ != 0.0)
		.then(|| b::vec3f64::Ty {
			x: p.playerMotionX as f64,
			y: p.playerMotionY as f64,
			z: p.playerMotionZ as f64,
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
	let mut out = vec![ClientB::explosion(b::play_toClient_packet_explosion::Ty {
		x: p.x,
		y: p.y,
		z: p.z,
		playerKnockback: knockback,
		explosionParticle: particle,
		sound: sound(FROM, TO, p.sound),
	})];
	if matches!(p.block_interaction_type, 1 | 2) {
		out.extend(p.affectedBlockOffsets.iter().map(|o| {
			ClientB::block_change(b::play_toClient_packet_block_change::Ty {
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
	p: b::play_toClient_packet_explosion::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let knockback = p.playerKnockback.unwrap_or(b::vec3f64::Ty {
		x: 0.0,
		y: 0.0,
		z: 0.0,
	});
	let particle = particle_down(p.explosionParticle, cx)?;
	Ok(vec![ClientA::explosion(
		a::play_toClient_packet_explosion::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
			radius: 0.0,
			affectedBlockOffsets: vec![],
			playerMotionX: knockback.x as f32,
			playerMotionY: knockback.y as f32,
			playerMotionZ: knockback.z as f32,
			block_interaction_type: 0,
			small_explosion_particle: particle.clone(),
			large_explosion_particle: particle,
			sound: sound(TO, FROM, p.sound),
		},
	)])
}

fn ingredient_items(ingredient: &[a::Slot::Ty]) -> Vec<i32> {
	ingredient
		.iter()
		.filter_map(|s| match s {
			a::Slot::Ty::default(_, d) => Some(item_up(d.itemId)),
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
			R::minecraft_smelting(r) => vec![(0, ingredient_items(&r.ingredient))],
			R::minecraft_blasting(r) => vec![(1, ingredient_items(&r.ingredient))],
			R::minecraft_smoking(r) => vec![(2, ingredient_items(&r.ingredient))],
			R::minecraft_campfire_cooking(r) => vec![(3, ingredient_items(&r.ingredient))],
			R::minecraft_smithing_transform(r) => vec![
				(4, ingredient_items(&r.template)),
				(5, ingredient_items(&r.base)),
				(6, ingredient_items(&r.addition)),
			],
			R::minecraft_smithing_trim(r) => vec![
				(4, ingredient_items(&r.template)),
				(5, ingredient_items(&r.base)),
				(6, ingredient_items(&r.addition)),
			],
			R::minecraft_stonecutting(r) => {
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
			b::play_toClient_packet_declare_recipes__stoneCutterRecipes__items::Ty {
				input: IdSet::Ids(input),
				slotDisplay: Display::item_stack(slot_up(result, cx)?),
			},
		);
	}
	Ok(vec![ClientB::declare_recipes(
		b::play_toClient_packet_declare_recipes::Ty {
			recipes: names
				.iter()
				.zip(sets)
				.map(
					|(name, items)| b::play_toClient_packet_declare_recipes__recipes__items::Ty {
						name: (*name).to_owned(),
						items,
					},
				)
				.collect(),
			stoneCutterRecipes: cutting,
		},
	)])
}

#[derive(Default)]
struct Recipes {
	stonecutter: Vec<RecipeA>,
	book: Vec<(i32, RecipeA)>,
	settings: [bool; 8],
}

fn plain(item: i32, count: i32) -> a::Slot::Ty {
	if item <= 0 || count <= 0 {
		return a::Slot::Ty::v0;
	}
	a::Slot::Ty::default(
		count,
		a::Slot__default::Ty {
			itemId: item,
			addedComponentCount: 0,
			removedComponentCount: 0,
			components: vec![],
			removeComponents: vec![],
		},
	)
}

fn display_items(d: &Display, cx: &mut Session) -> Result<Vec<a::Slot::Ty>> {
	Ok(match d {
		Display::item(id) => vec![plain(item_down(*id), 1)],
		Display::item_stack(s) => vec![slot_down(s.clone(), cx)?],
		Display::tag(t) => item_tag(cx, TO, t)
			.into_iter()
			.map(|i| plain(item_down(i), 1))
			.collect(),
		Display::with_remainder(r) => display_items(&r.input, cx)?,
		Display::composite(list) => {
			let mut out = Vec::new();
			for d in list {
				out.extend(display_items(d, cx)?);
			}
			out
		}
		_ => vec![],
	})
}

fn display_result(d: &Display, cx: &mut Session) -> Result<a::Slot::Ty> {
	Ok(display_items(d, cx)?
		.into_iter()
		.next()
		.unwrap_or(a::Slot::Ty::v0))
}

fn station(d: &Display) -> Option<&'static str> {
	let id = match d {
		Display::item(id) => *id,
		Display::item_stack(b::Slot::Ty::default(_, s)) => s.itemId,
		_ => return None,
	};
	registry::names(TO, "items")?.name(u32::try_from(id).ok()?)
}

fn book_category(
	c: &b::play_toClient_packet_recipe_book_add__entries__items__recipe__category::Ty,
) -> i32 {
	use b::play_toClient_packet_recipe_book_add__entries__items__recipe__category::Ty as C;
	match c {
		C::crafting_building_blocks | C::furnace_food | C::smoker_food | C::campfire => 0,
		C::crafting_redstone | C::furnace_blocks | C::blast_furnace_blocks => 1,
		C::crafting_equipment | C::furnace_misc | C::blast_furnace_misc => 2,
		_ => 3,
	}
}

fn recipe_down(
	id: i32,
	display: &b::play_toClient_RecipeDisplay::Ty,
	category: i32,
	cx: &mut Session,
) -> Result<Option<RecipeA>> {
	use b::play_toClient_RecipeDisplay::Ty as D;
	let data = match display {
		D::crafting_shaped(r) => {
			let mut cells = Vec::new();
			for d in &r.ingredients {
				cells.push(display_items(d, cx)?);
			}
			let height = r.height.max(1) as usize;
			RecipeDataA::minecraft_crafting_shaped(a::play_toClient_packet__declare_recipes__items__data__minecraft_crafting_shaped::Ty {
				group: String::new(),
				category,
				gridWidth: r.width,
				gridHeight: r.height,
				ingredients: cells.chunks(height).map(|c| c.to_vec()).collect(),
				result: display_result(&r.result, cx)?,
				showNotification: true,
			})
		}
		D::crafting_shapeless(r) => {
			let mut ingredients = Vec::new();
			for d in &r.ingredients {
				ingredients.push(display_items(d, cx)?);
			}
			RecipeDataA::minecraft_crafting_shapeless(a::play_toClient_packet__declare_recipes__items__data__minecraft_crafting_shapeless::Ty {
				group: String::new(),
				category,
				ingredients,
				result: display_result(&r.result, cx)?,
			})
		}
		D::furnace(r) => {
			let format = a::minecraft_smelting_format::Ty {
				group: String::new(),
				category,
				ingredient: display_items(&r.ingredient, cx)?,
				result: display_result(&r.result, cx)?,
				experience: r.experience,
				cookTime: r.duration,
			};
			match station(&r.craftingStation) {
				Some("blast_furnace") => RecipeDataA::minecraft_blasting(format),
				Some("smoker") => RecipeDataA::minecraft_smoking(format),
				Some("campfire" | "soul_campfire") => RecipeDataA::minecraft_campfire_cooking(format),
				_ => RecipeDataA::minecraft_smelting(format),
			}
		}
		D::stonecutter(r) => RecipeDataA::minecraft_stonecutting(a::play_toClient_packet__declare_recipes__items__data__minecraft_stonecutting::Ty {
			group: String::new(),
			ingredient: display_items(&r.ingredient, cx)?,
			result: display_result(&r.result, cx)?,
		}),
		D::smithing(r) => RecipeDataA::minecraft_smithing_transform(a::play_toClient_packet__declare_recipes__items__data__minecraft_smithing_transform::Ty {
			template: display_items(&r.template, cx)?,
			base: display_items(&r.base, cx)?,
			addition: display_items(&r.addition, cx)?,
			result: display_result(&r.result, cx)?,
		}),
		_ => return Ok(None),
	};
	Ok(Some(RecipeA {
		name: format!("minecraft:{id}"),
		data,
	}))
}

fn all_recipes(cx: &mut Session) -> ClientA {
	let r = cx.state::<Recipes>();
	ClientA::declare_recipes(
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
	action: a::play_toClient_packet_unlock_recipes__recipes2::Ty,
) -> ClientA {
	let s = cx.state::<Recipes>().settings;
	ClientA::unlock_recipes(a::play_toClient_packet_unlock_recipes::Ty {
		craftingBookOpen: s[0],
		filteringCraftable: s[1],
		smeltingBookOpen: s[2],
		filteringSmeltable: s[3],
		blastFurnaceOpen: s[4],
		filteringBlastFurnace: s[5],
		smokerBookOpen: s[6],
		filteringSmoker: s[7],
		recipes1: recipes,
		recipes2: action,
	})
}

pub fn tricky_declare_recipes_down_1_21_1(
	p: b::play_toClient_packet_declare_recipes::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let mut stonecutter = Vec::new();
	for (i, r) in p.stoneCutterRecipes.into_iter().enumerate() {
		let input: Vec<i32> = match r.input {
			IdSet::Ids(ids) => ids,
			IdSet::Named(tag) => item_tag(cx, TO, &tag),
		};
		let result = display_result(&r.slotDisplay, cx)?;
		stonecutter.push(RecipeA {
			name: format!("minecraft:stonecutter_{i}"),
			data: RecipeDataA::minecraft_stonecutting(
				a::play_toClient_packet__declare_recipes__items__data__minecraft_stonecutting::Ty {
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
	p: b::play_toClient_packet_recipe_book_add::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	if p.replace {
		cx.state::<Recipes>().book.clear();
	}
	let mut added = Vec::new();
	let mut highlighted = Vec::new();
	for e in p.entries {
		let id = e.recipe.displayId;
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
			a::play_toClient_packet_unlock_recipes__recipes2::Ty::v0(highlighted)
		}
		false => a::play_toClient_packet_unlock_recipes__recipes2::Ty::default(1),
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
		a::play_toClient_packet_unlock_recipes__recipes2::Ty::default(2),
	)])
}

pub fn tricky_recipe_book_settings_down_1_21_1(
	p: b::play_toClient_packet_recipe_book_settings::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	cx.state::<Recipes>().settings = [
		p.craftingGuiOpen,
		p.craftingFilteringCraftable,
		p.smeltingGuiOpen,
		p.smeltingFilteringCraftable,
		p.blastGuiOpen,
		p.blastFilteringCraftable,
		p.smokerGuiOpen,
		p.smokerFilteringCraftable,
	];
	Ok(vec![])
}

fn recipe_id(name: &str) -> Option<i32> {
	strip(name).parse().ok()
}

pub fn tricky_craft_recipe_request_1_21_3(
	p: a::play_toServer_packet_craft_recipe_request::Ty,
	_cx: &mut Session,
) -> Result<Vec<ServerB>> {
	let Some(id) = recipe_id(&p.recipe) else {
		return Ok(vec![]);
	};
	Ok(vec![ServerB::craft_recipe_request(
		b::play_toServer_packet_craft_recipe_request::Ty {
			windowId: p.windowId as i32,
			recipeId: id,
			makeAll: p.makeAll,
		},
	)])
}

pub fn tricky_displayed_recipe_1_21_3(p: String, _cx: &mut Session) -> Result<Vec<ServerB>> {
	Ok(recipe_id(&p)
		.map(ServerB::displayed_recipe)
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
	src: &a::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<i32> {
	cx.state::<Chunks>().loaded.clear();
	*cx.state::<Movement>() = Movement {
		player: src.entityId,
		..Default::default()
	};
	Ok(src.entityId)
}

pub fn tricky_login_entity_down_1_21_1(
	src: &b::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<i32> {
	*cx.state::<Movement>() = Movement {
		player: src.entityId,
		..Default::default()
	};
	Ok(src.entityId)
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
	p: a::play_toClient_packet_set_passengers::Ty,
	cx: &mut Session,
) -> Result<b::play_toClient_packet_set_passengers::Ty> {
	passengers(cx, p.entityId, &p.passengers);
	Ok(p)
}

pub fn tricky_passengers_down_1_21_1(
	p: b::play_toClient_packet_set_passengers::Ty,
	cx: &mut Session,
) -> Result<a::play_toClient_packet_set_passengers::Ty> {
	passengers(cx, p.entityId, &p.passengers);
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
	Ok(vec![ClientB::entity_destroy(p)])
}

pub fn tricky_destroy_down_1_21_1(p: Vec<i32>, cx: &mut Session) -> Result<Vec<ClientA>> {
	destroyed(cx, &p);
	Ok(vec![ClientA::entity_destroy(p)])
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
	src: &b::play_toServer_packet_look::Ty,
	cx: &mut Session,
) -> Result<bool> {
	moved(cx, None, Some((src.yaw, src.pitch)), src.flags.onGround);
	Ok(src.flags.onGround)
}

pub fn tricky_position_ground_down_1_21_1(
	src: &b::play_toServer_packet_position::Ty,
	cx: &mut Session,
) -> Result<bool> {
	moved(cx, Some((src.x, src.y, src.z)), None, src.flags.onGround);
	Ok(src.flags.onGround)
}

pub fn tricky_position_look_ground_down_1_21_1(
	src: &b::play_toServer_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<bool> {
	moved(
		cx,
		Some((src.x, src.y, src.z)),
		Some((src.yaw, src.pitch)),
		src.flags.onGround,
	);
	Ok(src.flags.onGround)
}

pub fn tricky_flying_down_1_21_1(flags: Flags, cx: &mut Session) -> Result<Vec<ServerA>> {
	moved(cx, None, None, flags.onGround);
	Ok(vec![ServerA::flying(flags.onGround)])
}

fn ground_flags(on_ground: bool) -> Flags {
	Flags {
		onGround: on_ground,
		hasHorizontalCollision: false,
	}
}

fn tick_end() -> ServerB {
	ServerB::tick_end(b::play_toServer_packet_tick_end::Ty {})
}

pub fn tricky_look_1_21_3(
	p: a::play_toServer_packet_look::Ty,
	cx: &mut Session,
) -> Result<Vec<ServerB>> {
	moved(cx, None, Some((p.yaw, p.pitch)), p.onGround);
	Ok(vec![
		ServerB::look(b::play_toServer_packet_look::Ty {
			yaw: p.yaw,
			pitch: p.pitch,
			flags: ground_flags(p.onGround),
		}),
		tick_end(),
	])
}

pub fn tricky_position_1_21_3(
	p: a::play_toServer_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<ServerB>> {
	moved(cx, Some((p.x, p.y, p.z)), None, p.onGround);
	Ok(vec![
		ServerB::position(b::play_toServer_packet_position::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
			flags: ground_flags(p.onGround),
		}),
		tick_end(),
	])
}

pub fn tricky_position_look_1_21_3(
	p: a::play_toServer_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<Vec<ServerB>> {
	moved(
		cx,
		Some((p.x, p.y, p.z)),
		Some((p.yaw, p.pitch)),
		p.onGround,
	);
	Ok(vec![
		ServerB::position_look(b::play_toServer_packet_position_look::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			flags: ground_flags(p.onGround),
		}),
		tick_end(),
	])
}

pub fn tricky_flying_1_21_3(on_ground: bool, cx: &mut Session) -> Result<Vec<ServerB>> {
	moved(cx, None, None, on_ground);
	Ok(vec![ServerB::flying(ground_flags(on_ground)), tick_end()])
}

const IMPULSE: f32 = 0.98;

fn steer(m: &Movement) -> ServerA {
	ServerA::steer_vehicle(a::play_toServer_packet_steer_vehicle::Ty {
		sideways: m.input.0,
		forward: m.input.1,
		jump: m.input.2,
	})
}

pub fn tricky_player_input_down_1_21_1(
	p: b::play_toServer_packet_player_input::Ty,
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
	use b::play_toServer_packet_entity_action__actionId::Ty as A;
	ServerB::entity_action(b::play_toServer_packet_entity_action::Ty {
		entityId: entity,
		actionId: if sneaking {
			A::start_sneaking
		} else {
			A::stop_sneaking
		},
		jumpBoost: 0,
	})
}

pub fn tricky_steer_vehicle_1_21_3(
	p: a::play_toServer_packet_steer_vehicle::Ty,
	cx: &mut Session,
) -> Result<Vec<ServerB>> {
	let shift = p.jump & 2 != 0;
	let m = cx.state::<Movement>();
	let mut out = vec![ServerB::player_input(
		b::play_toServer_packet_player_input::Ty {
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
	p: a::play_toServer_packet_entity_action::Ty,
	cx: &mut Session,
) -> Result<b::play_toServer_packet_entity_action::Ty> {
	use a::play_toServer_packet_entity_action__actionId::Ty as A;
	match p.actionId {
		A::start_sneaking => cx.state::<Movement>().sneaking = true,
		A::stop_sneaking => cx.state::<Movement>().sneaking = false,
		_ => {}
	}
	Ok(p)
}

pub fn tricky_tick_end_down_1_21_1(
	_: b::play_toServer_packet_tick_end::Ty,
	cx: &mut Session,
) -> Result<Vec<ServerA>> {
	let m = cx.state::<Movement>();
	let idle = !std::mem::replace(&mut m.moved, false);
	let mut out = Vec::new();
	if m.vehicle.is_some() {
		if idle {
			let (yaw, pitch) = m.rotation;
			out.push(ServerA::look(a::play_toServer_packet_look::Ty {
				yaw,
				pitch,
				onGround: m.on_ground,
			}));
		}
		out.push(steer(m));
	} else if idle {
		out.push(ServerA::flying(m.on_ground));
	}
	Ok(out)
}
