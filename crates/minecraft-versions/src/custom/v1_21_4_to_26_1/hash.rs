use super::components;
use super::remap;
use super::util::{decode, write_varint};
use crate::gen::convert::{v1_21_5 as b, Version};
use nbt::Payload;
use std::collections::HashMap;
use std::sync::LazyLock;

const V: Version = Version::V1_21_5;
const UNKNOWN: i32 = 399825415;

pub enum H {
	Empty,
	Bool(bool),
	Byte(i8),
	Short(i16),
	Int(i32),
	Long(i64),
	Float(f32),
	Double(f64),
	Str(String),
	List(Vec<H>),
	Map(Vec<(String, H)>),
	Bytes(Vec<i8>),
	Ints(Vec<i32>),
	Longs(Vec<i64>),
}

static TABLE: LazyLock<[u32; 256]> = LazyLock::new(|| {
	let mut t = [0u32; 256];
	for (i, slot) in t.iter_mut().enumerate() {
		let mut c = i as u32;
		for _ in 0..8 {
			c = if c & 1 != 0 {
				c >> 1 ^ 0x82F6_3B78
			} else {
				c >> 1
			};
		}
		*slot = c;
	}
	t
});

fn crc32c(data: &[u8]) -> u32 {
	!data.iter().fold(!0u32, |c, b| {
		TABLE[((c ^ *b as u32) & 0xff) as usize] ^ c >> 8
	})
}

pub fn hash(h: &H) -> i32 {
	let mut out = Vec::new();
	write(h, &mut out);
	crc32c(&out) as i32
}

fn write(h: &H, out: &mut Vec<u8>) {
	match h {
		H::Empty => out.push(1),
		H::Bool(v) => out.extend([13, *v as u8]),
		H::Byte(v) => out.extend([6, *v as u8]),
		H::Short(v) => {
			out.push(7);
			out.extend(v.to_le_bytes());
		}
		H::Int(v) => {
			out.push(8);
			out.extend(v.to_le_bytes());
		}
		H::Long(v) => {
			out.push(9);
			out.extend(v.to_le_bytes());
		}
		H::Float(v) => {
			out.push(10);
			out.extend(v.to_bits().to_le_bytes());
		}
		H::Double(v) => {
			out.push(11);
			out.extend(v.to_bits().to_le_bytes());
		}
		H::Str(s) => {
			let units: Vec<u16> = s.encode_utf16().collect();
			out.push(12);
			out.extend((units.len() as i32).to_le_bytes());
			units.iter().for_each(|u| out.extend(u.to_le_bytes()));
		}
		H::List(items) => {
			out.push(4);
			items.iter().for_each(|i| out.extend(hash(i).to_le_bytes()));
			out.push(5);
		}
		H::Map(entries) => {
			let mut pairs: Vec<(u32, u32)> = entries
				.iter()
				.map(|(k, v)| (hash(&H::Str(k.clone())) as u32, hash(v) as u32))
				.collect();
			pairs.sort_unstable();
			out.push(2);
			for (k, v) in pairs {
				out.extend(k.to_le_bytes());
				out.extend(v.to_le_bytes());
			}
			out.push(3);
		}
		H::Bytes(v) => {
			out.push(14);
			out.extend(v.iter().map(|b| *b as u8));
			out.push(15);
		}
		H::Ints(v) => {
			out.push(16);
			v.iter().for_each(|i| out.extend(i.to_le_bytes()));
			out.push(17);
		}
		H::Longs(v) => {
			out.push(18);
			v.iter().for_each(|i| out.extend(i.to_le_bytes()));
			out.push(19);
		}
	}
}

#[derive(Default)]
pub struct Keys(pub HashMap<String, Vec<String>>);

impl Keys {
	pub fn get(&self, registry: &str, id: i32) -> Option<String> {
		self.0
			.get(registry)?
			.get(usize::try_from(id).ok()?)
			.cloned()
	}
}

const POTIONS: &[&str] = &[
	"water",
	"mundane",
	"thick",
	"awkward",
	"night_vision",
	"long_night_vision",
	"invisibility",
	"long_invisibility",
	"leaping",
	"long_leaping",
	"strong_leaping",
	"fire_resistance",
	"long_fire_resistance",
	"swiftness",
	"long_swiftness",
	"strong_swiftness",
	"slowness",
	"long_slowness",
	"strong_slowness",
	"turtle_master",
	"long_turtle_master",
	"strong_turtle_master",
	"water_breathing",
	"long_water_breathing",
	"healing",
	"strong_healing",
	"harming",
	"strong_harming",
	"poison",
	"long_poison",
	"strong_poison",
	"regeneration",
	"long_regeneration",
	"strong_regeneration",
	"strength",
	"long_strength",
	"strong_strength",
	"weakness",
	"long_weakness",
	"luck",
	"slow_falling",
	"long_slow_falling",
	"wind_charged",
	"weaving",
	"oozing",
	"infested",
];

const ATTRIBUTES: &[&str] = &[
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

const DYES: &[&str] = &[
	"white",
	"orange",
	"magenta",
	"light_blue",
	"yellow",
	"lime",
	"pink",
	"gray",
	"light_gray",
	"cyan",
	"purple",
	"blue",
	"brown",
	"green",
	"red",
	"black",
];

const VILLAGER_TYPES: &[&str] = &[
	"desert", "jungle", "plains", "savanna", "snow", "swamp", "taiga",
];
const FOXES: &[&str] = &["red", "snow"];
const SALMON: &[&str] = &["small", "medium", "large"];
const PARROTS: &[&str] = &["red_blue", "blue", "green", "yellow_blue", "gray"];
const MOOSHROOMS: &[&str] = &["red", "brown"];
const HORSES: &[&str] = &[
	"white",
	"creamy",
	"chestnut",
	"brown",
	"black",
	"gray",
	"dark_brown",
];
const LLAMAS: &[&str] = &["creamy", "white", "brown", "gray"];
const AXOLOTLS: &[&str] = &["lucy", "wild", "gold", "cyan", "blue"];
const RABBITS: &[&str] = &["brown", "white", "black", "white_splotched", "gold", "salt"];
const FISH: &[(i32, &str)] = &[
	(0, "kob"),
	(256, "sunstreak"),
	(512, "snooper"),
	(768, "dasher"),
	(1024, "brinely"),
	(1280, "spotty"),
	(1, "flopper"),
	(257, "stripey"),
	(513, "glitter"),
	(769, "blockfish"),
	(1025, "betty"),
	(1281, "clayfish"),
];

const BOOLS: &[&str] = &[
	"bold",
	"italic",
	"underlined",
	"strikethrough",
	"obfuscated",
	"interpret",
];

fn str(s: &str) -> H {
	H::Str(s.to_owned())
}

fn key_str(s: &str) -> String {
	match s.split_once(':') {
		Some(("", path)) => format!("minecraft:{path}"),
		Some(_) => s.to_owned(),
		None => format!("minecraft:{s}"),
	}
}

fn key(s: &str) -> H {
	H::Str(key_str(s))
}

fn pick(list: &[&str], id: i32) -> Option<H> {
	list.get(usize::try_from(id).ok()?).map(|s| str(s))
}

fn map(entries: Vec<(&str, Option<H>)>) -> H {
	H::Map(
		entries
			.into_iter()
			.filter_map(|(k, v)| Some((k.to_owned(), v?)))
			.collect(),
	)
}

fn unless<T: PartialEq>(v: T, default: T, f: impl FnOnce(T) -> H) -> Option<H> {
	(v != default).then(|| f(v))
}

fn list<T>(v: &[T], f: impl Fn(&T) -> Option<H>) -> Option<H> {
	v.iter().map(f).collect::<Option<Vec<H>>>().map(H::List)
}

fn non_empty<T>(v: &[T], f: impl Fn(&T) -> Option<H>) -> Option<Option<H>> {
	if v.is_empty() {
		Some(None)
	} else {
		list(v, f).map(Some)
	}
}

pub fn nbt(p: &Payload) -> H {
	match p {
		Payload::end => H::Empty,
		Payload::byte(v) => H::Byte(*v),
		Payload::short(v) => H::Short(*v),
		Payload::int(v) => H::Int(*v),
		Payload::long(v) => H::Long(*v),
		Payload::float(v) => H::Float(*v),
		Payload::double(v) => H::Double(*v),
		Payload::byteArray(v) => H::Bytes(v.clone()),
		Payload::string(s) => H::Str(s.clone()),
		Payload::list(l) => H::List(l.values().iter().map(nbt).collect()),
		Payload::compound(entries) => H::Map(
			entries
				.iter()
				.map(|e| (e.name.clone(), nbt(&e.value)))
				.collect(),
		),
		Payload::intArray(v) => H::Ints(v.clone()),
		Payload::longArray(v) => H::Longs(v.clone()),
	}
}

fn hover(p: &Payload) -> H {
	let Payload::compound(entries) = p else {
		return nbt(p);
	};
	H::Map(
		entries
			.iter()
			.map(|e| {
				let v = match e.name.as_str() {
					"value" | "name" => text(&e.value),
					_ => nbt(&e.value),
				};
				(e.name.clone(), v)
			})
			.collect(),
	)
}

pub fn text(p: &Payload) -> H {
	match p {
		Payload::compound(entries) => {
			if let [e] = entries.as_slice() {
				if let ("text", Payload::string(s)) = (e.name.as_str(), &e.value) {
					return H::Str(s.clone());
				}
			}
			H::Map(
				entries
					.iter()
					.map(|e| {
						let v = match (e.name.as_str(), &e.value) {
							(k, Payload::byte(b)) if BOOLS.contains(&k) => H::Bool(*b != 0),
							("extra" | "with", Payload::list(l)) => {
								H::List(l.values().iter().map(text).collect())
							}
							("separator", v) => text(v),
							("hover_event", v) => hover(v),
							(_, v) => nbt(v),
						};
						(e.name.clone(), v)
					})
					.collect(),
			)
		}
		Payload::list(l) => H::List(l.values().iter().map(text).collect()),
		other => nbt(other),
	}
}

fn name(kind: &str, id: i32) -> Option<H> {
	crate::registry::names(V, kind)?
		.name(u32::try_from(id).ok()?)
		.map(key)
}

fn sound(s: &b::ItemSoundHolder::Ty) -> Option<H> {
	Some(match s {
		b::ItemSoundHolder::Ty::Id(id) => key(remap::sound_name(V, *id)?),
		b::ItemSoundHolder::Ty::Inline(e) => map(vec![
			("sound_id", Some(key(&e.soundName))),
			("range", e.fixedRange.map(H::Float)),
		]),
	})
}

fn sound_or(s: &b::ItemSoundHolder::Ty, default: &str) -> Option<Option<H>> {
	match s {
		b::ItemSoundHolder::Ty::Id(id) if remap::sound_name(V, *id) == Some(default) => Some(None),
		s => sound(s).map(Some),
	}
}

fn set(s: &b::IDSet::Ty, kind: &str) -> Option<H> {
	match s {
		b::IDSet::Ty::Named(tag) => Some(H::Str(format!("#{}", key_str(tag)))),
		b::IDSet::Ty::Ids(ids) if ids.len() == 1 => name(kind, ids[0]),
		b::IDSet::Ty::Ids(ids) => list(ids, |i| name(kind, *i)),
	}
}

fn effect_detail(d: &b::ItemEffectDetail::Ty) -> Option<H> {
	Some(map(vec![
		("amplifier", unless(d.amplifier, 0, |v| H::Byte(v as i8))),
		("duration", unless(d.duration, 0, H::Int)),
		("ambient", unless(d.ambient, false, H::Bool)),
		("show_particles", unless(d.showParticles, true, H::Bool)),
		("show_icon", Some(H::Bool(d.showIcon))),
		(
			"hidden_effect",
			match &d.hiddenEffect {
				Some(h) => Some(effect_detail(h)?),
				None => None,
			},
		),
	]))
}

fn effect(e: &b::ItemPotionEffect::Ty) -> Option<H> {
	let H::Map(mut entries) = effect_detail(&e.details)? else {
		return None;
	};
	entries.push(("id".to_owned(), name("effects", e.id)?));
	Some(H::Map(entries))
}

fn consume_effect(e: &b::ItemConsumeEffect::Ty) -> Option<H> {
	use b::ItemConsumeEffect::Ty as E;
	Some(match e {
		E::apply_effects(a) => map(vec![
			("type", Some(str("minecraft:apply_effects"))),
			("effects", Some(list(&a.effects, effect)?)),
			("probability", unless(a.probability, 1.0, H::Float)),
		]),
		E::remove_effects(s) => map(vec![
			("type", Some(str("minecraft:remove_effects"))),
			("effects", Some(set(s, "effects")?)),
		]),
		E::clear_all_effects => map(vec![("type", Some(str("minecraft:clear_all_effects")))]),
		E::teleport_randomly(d) => map(vec![
			("type", Some(str("minecraft:teleport_randomly"))),
			("diameter", unless(*d, 16.0, H::Float)),
		]),
		E::play_sound(s) => map(vec![
			("type", Some(str("minecraft:play_sound"))),
			("sound", Some(sound(s)?)),
		]),
		E::default(_) => return None,
	})
}

fn explosion(e: &b::ItemFireworkExplosion::Ty) -> Option<H> {
	use b::ItemFireworkExplosion__shape::Ty as S;
	let shape = match e.shape {
		S::small_ball => "small_ball",
		S::large_ball => "large_ball",
		S::star => "star",
		S::creeper => "creeper",
		S::burst => "burst",
		S::default(_) => return None,
	};
	Some(map(vec![
		("shape", Some(str(shape))),
		("colors", non_empty(&e.colors, |c| Some(H::Int(*c)))?),
		(
			"fade_colors",
			non_empty(&e.fadeColors, |c| Some(H::Int(*c)))?,
		),
		("has_trail", unless(e.hasTrail, false, H::Bool)),
		("has_twinkle", unless(e.hasTwinkle, false, H::Bool)),
	]))
}

fn filterable(raw: H, filtered: Option<H>) -> H {
	map(vec![("raw", Some(raw)), ("filtered", filtered)])
}

fn stack(s: &b::Slot::Ty, keys: &Keys) -> Option<Option<H>> {
	let b::Slot::Ty::default(count, d) = s else {
		return Some(None);
	};
	let mut patch = Vec::new();
	for c in &d.components {
		let tag = b::SlotComponent::virtual_tag(c);
		patch.push((
			format!("minecraft:{}", components::name(V, tag)?),
			component(c, keys)?,
		));
	}
	for r in &d.removeComponents {
		let tag = b::SlotComponentType::virtual_tag(r);
		patch.push((
			format!("!minecraft:{}", components::name(V, tag)?),
			H::Map(Vec::new()),
		));
	}
	Some(Some(map(vec![
		("id", Some(name("items", d.itemId)?)),
		("count", Some(H::Int(*count))),
		("components", (!patch.is_empty()).then_some(H::Map(patch))),
	])))
}

fn stacks(v: &[b::Slot::Ty], keys: &Keys) -> Option<H> {
	let mut out = Vec::new();
	for s in v {
		out.extend(stack(s, keys)?);
	}
	Some(H::List(out))
}

pub fn component(c: &b::SlotComponent::Ty, keys: &Keys) -> Option<H> {
	use b::SlotComponent::Ty as C;
	let unit = || H::Map(Vec::new());
	Some(match c {
		C::custom_data(t)
		| C::debug_stick_state(t)
		| C::entity_data(t)
		| C::bucket_entity_data(t)
		| C::block_entity_data(t)
		| C::recipes(t)
		| C::lock(t)
		| C::container_loot(t)
		| C::map_decorations(t)
		| C::intangible_projectile(t) => nbt(&t.value),
		C::max_stack_size(v)
		| C::max_damage(v)
		| C::damage(v)
		| C::repair_cost(v)
		| C::map_id(v)
		| C::ominous_bottle_amplifier(v) => H::Int(*v),
		C::dyed_color(v) | C::map_color(v) => H::Int(*v),
		C::enchantable(v) => map(vec![("value", Some(H::Int(*v)))]),
		C::unbreakable | C::glider => unit(),
		C::enchantment_glint_override(v) => H::Bool(*v),
		C::potion_duration_scale(v) => H::Float(*v),
		C::custom_name(t) | C::item_name(t) => text(&t.value),
		C::lore(lines) => H::List(lines.iter().map(|l| text(&l.value)).collect()),
		C::item_model(s) | C::tooltip_style(s) | C::note_block_sound(s) => key(s),
		C::damage_resistant(s) => map(vec![("types", Some(H::Str(format!("#{}", key_str(s)))))]),
		C::provides_banner_patterns(s) => H::Str(format!("#{}", key_str(s))),
		C::rarity(r) => str(match r {
			b::SlotComponent__rarity::Ty::common => "common",
			b::SlotComponent__rarity::Ty::uncommon => "uncommon",
			b::SlotComponent__rarity::Ty::rare => "rare",
			b::SlotComponent__rarity::Ty::epic => "epic",
			b::SlotComponent__rarity::Ty::default(_) => return None,
		}),
		C::enchantments(e) => H::Map(
			e.iter()
				.map(|x| Some((keys.get("enchantment", x.id)?, H::Int(x.level))))
				.collect::<Option<_>>()?,
		),
		C::stored_enchantments(e) => H::Map(
			e.iter()
				.map(|x| Some((keys.get("enchantment", x.id)?, H::Int(x.level))))
				.collect::<Option<_>>()?,
		),
		C::attribute_modifiers(m) => list(m, |x| {
			use b::SlotComponent__attribute_modifiers__items__operation::Ty as O;
			use b::SlotComponent__attribute_modifiers__items__slot::Ty as S;
			let operation = match x.operation {
				O::add => "add_value",
				O::multiply_base => "add_multiplied_base",
				O::multiply_total => "add_multiplied_total",
				O::default(_) => return None,
			};
			let slot = match x.slot {
				S::any => "any",
				S::main_hand => "mainhand",
				S::off_hand => "offhand",
				S::hand => "hand",
				S::feet => "feet",
				S::legs => "legs",
				S::chest => "chest",
				S::head => "head",
				S::armor => "armor",
				S::body => "body",
				S::saddle => "saddle",
				S::default(_) => return None,
			};
			Some(map(vec![
				(
					"type",
					Some(key(ATTRIBUTES.get(usize::try_from(x.typeId).ok()?)?)),
				),
				("id", Some(key(&x.name))),
				("amount", Some(H::Double(x.value))),
				("operation", Some(str(operation))),
				("slot", (slot != "any").then(|| str(slot))),
			]))
		})?,
		C::custom_model_data(d) => map(vec![
			("floats", non_empty(&d.floats, |v| Some(H::Float(*v)))?),
			("flags", non_empty(&d.flags, |v| Some(H::Bool(*v)))?),
			("strings", non_empty(&d.strings, |v| Some(str(v)))?),
			("colors", non_empty(&d.colors, |v| Some(H::Int(*v)))?),
		]),
		C::tooltip_display(t) => map(vec![
			("hide_tooltip", unless(t.hideTooltip, false, H::Bool)),
			(
				"hidden_components",
				non_empty(&t.hiddenComponents, |id| {
					Some(key(components::name(V, *id)?))
				})?,
			),
		]),
		C::food(f) => map(vec![
			("nutrition", Some(H::Int(f.nutrition))),
			("saturation", Some(H::Float(f.saturationModifier))),
			("can_always_eat", unless(f.canAlwaysEat, false, H::Bool)),
		]),
		C::consumable(c) => {
			use b::SlotComponent__consumable__animation::Ty as A;
			let animation = match c.animation {
				A::none => "none",
				A::eat => "eat",
				A::bundle => "bundle",
				A::drink => "drink",
				A::block => "block",
				A::bow => "bow",
				A::spear => "spear",
				A::crossbow => "crossbow",
				A::spyglass => "spyglass",
				A::toot_horn => "toot_horn",
				A::brush => "brush",
				A::default(_) => return None,
			};
			map(vec![
				("consume_seconds", unless(c.consume_seconds, 1.6, H::Float)),
				("animation", (animation != "eat").then(|| str(animation))),
				("sound", sound_or(&c.sound, "entity.generic.eat")?),
				(
					"has_consume_particles",
					unless(c.makes_particles, true, H::Bool),
				),
				("on_consume_effects", non_empty(&c.effects, consume_effect)?),
			])
		}
		C::death_protection(e) => map(vec![("death_effects", non_empty(e, consume_effect)?)]),
		C::use_remainder(s) => stack(s, keys)??,
		C::use_cooldown(u) => map(vec![
			("seconds", Some(H::Float(u.seconds))),
			("cooldown_group", u.cooldownGroup.as_deref().map(key)),
		]),
		C::tool(t) => map(vec![
			(
				"rules",
				Some(list(&t.rules, |r| {
					Some(map(vec![
						("blocks", Some(set(&r.blocks, "blocks")?)),
						("speed", r.speed.map(H::Float)),
						("correct_for_drops", r.correctDropForBlocks.map(H::Bool)),
					]))
				})?),
			),
			(
				"default_mining_speed",
				unless(t.defaultMiningSpeed, 1.0, H::Float),
			),
			("damage_per_block", unless(t.damagePerBlock, 1, H::Int)),
			(
				"can_destroy_blocks_in_creative",
				unless(t.canDestroyBlocksInCreative, true, H::Bool),
			),
		]),
		C::weapon(w) => map(vec![
			(
				"item_damage_per_attack",
				unless(w.itemDamagePerAttack, 1, H::Int),
			),
			(
				"disable_blocking_for_seconds",
				unless(w.disableBlockingForSeconds, 0.0, H::Float),
			),
		]),
		C::equippable(e) => {
			use b::SlotComponent__equippable__slot::Ty as S;
			let slot = match e.slot {
				S::main_hand => "mainhand",
				S::off_hand => "offhand",
				S::feet => "feet",
				S::legs => "legs",
				S::chest => "chest",
				S::head => "head",
				S::body => "body",
				S::saddle => "saddle",
				S::default(_) => return None,
			};
			map(vec![
				("slot", Some(str(slot))),
				(
					"equip_sound",
					sound_or(&e.sound, "item.armor.equip_generic")?,
				),
				("asset_id", e.model.as_deref().map(key)),
				("camera_overlay", e.cameraOverlay.as_deref().map(key)),
				(
					"allowed_entities",
					match &e.allowedEntities {
						Some(s) => Some(set(s, "entities")?),
						None => None,
					},
				),
				("dispensable", unless(e.dispensable, true, H::Bool)),
				("swappable", unless(e.swappable, true, H::Bool)),
				("damage_on_hurt", unless(e.damageable, true, H::Bool)),
				(
					"equip_on_interact",
					unless(e.equipOnInteract, false, H::Bool),
				),
			])
		}
		C::repairable(s) => map(vec![("items", Some(set(s, "items")?))]),
		C::charged_projectiles(v) | C::bundle_contents(v) => stacks(v, keys)?,
		C::container(v) => {
			let mut out = Vec::new();
			for (i, s) in v.iter().enumerate() {
				if let Some(item) = stack(s, keys)? {
					out.push(map(vec![
						("slot", Some(H::Int(i as i32))),
						("item", Some(item)),
					]));
				}
			}
			H::List(out)
		}
		C::potion_contents(p) => map(vec![
			(
				"potion",
				match p.potionId {
					Some(id) => Some(key(POTIONS.get(usize::try_from(id).ok()?)?)),
					None => None,
				},
			),
			("custom_color", p.customColor.map(H::Int)),
			("custom_effects", non_empty(&p.customEffects, effect)?),
			("custom_name", p.customName.as_deref().map(str)),
		]),
		C::suspicious_stew_effects(v) => list(v, |e| {
			Some(map(vec![
				("id", Some(name("effects", e.effect)?)),
				("duration", unless(e.duration, 160, H::Int)),
			]))
		})?,
		C::writable_book_content(pages) => map(vec![(
			"pages",
			non_empty(pages, |p| {
				Some(filterable(
					str(&p.content),
					p.filteredContent.as_deref().map(str),
				))
			})?,
		)]),
		C::written_book_content(w) => map(vec![
			(
				"title",
				Some(filterable(
					str(&w.rawTitle),
					w.filteredTitle.as_deref().map(str),
				)),
			),
			("author", Some(str(&w.author))),
			("generation", unless(w.generation, 0, H::Int)),
			(
				"pages",
				non_empty(&w.pages, |p| {
					Some(filterable(
						text(&p.content.value),
						p.filteredContent.as_ref().map(|t| text(&t.value)),
					))
				})?,
			),
			("resolved", unless(w.resolved, false, H::Bool)),
		]),
		C::trim(t) => {
			let material = match &t.material {
				b::SlotComponent__trim__material::Ty::Id(id) => keys.get("trim_material", *id)?,
				b::SlotComponent__trim__material::Ty::Inline(_) => return None,
			};
			let pattern = match &t.pattern {
				b::SlotComponent__trim__pattern::Ty::Id(id) => keys.get("trim_pattern", *id)?,
				b::SlotComponent__trim__pattern::Ty::Inline(_) => return None,
			};
			map(vec![
				("material", Some(key(&material))),
				("pattern", Some(key(&pattern))),
			])
		}
		C::instrument(i) => match i {
			b::SlotComponent__instrument::Ty::r#false(k) => key(k),
			b::SlotComponent__instrument::Ty::r#true(
				b::SlotComponent__instrument__true::Ty::Id(id),
			) => key(&keys.get("instrument", *id)?),
			b::SlotComponent__instrument::Ty::r#true(_) => return None,
		},
		C::provides_trim_material(i) => match i {
			b::SlotComponent__provides_trim_material::Ty::r#false(k) => key(k),
			b::SlotComponent__provides_trim_material::Ty::r#true(
				b::SlotComponent__provides_trim_material__true::Ty::Id(id),
			) => key(&keys.get("trim_material", *id)?),
			b::SlotComponent__provides_trim_material::Ty::r#true(_) => return None,
		},
		C::jukebox_playable(j) => match j {
			b::SlotComponent__jukebox_playable::Ty::r#false(k) => key(k),
			b::SlotComponent__jukebox_playable::Ty::r#true(
				b::SlotComponent__jukebox_playable__true::Ty::Id(id),
			) => key(&keys.get("jukebox_song", *id)?),
			b::SlotComponent__jukebox_playable::Ty::r#true(_) => return None,
		},
		C::chicken_variant(v) => match v {
			b::SlotComponent__chicken_variant::Ty::r#false(k) => key(k),
			b::SlotComponent__chicken_variant::Ty::r#true(id) => {
				key(&keys.get("chicken_variant", *id)?)
			}
		},
		C::lodestone_tracker(l) => map(vec![
			(
				"target",
				match &l.globalPosition {
					Some(g) => Some(map(vec![
						("dimension", Some(key(&g.dimension))),
						(
							"pos",
							Some(H::Ints(vec![
								g.position.x,
								g.position.y as i32,
								g.position.z,
							])),
						),
					])),
					None => None,
				},
			),
			("tracked", unless(l.tracked, true, H::Bool)),
		]),
		C::firework_explosion(e) => explosion(e)?,
		C::fireworks(f) => map(vec![
			(
				"flight_duration",
				unless(f.flightDuration, 0, |v| H::Byte(v as i8)),
			),
			("explosions", non_empty(&f.explosions, explosion)?),
		]),
		C::profile(p) => map(vec![
			("name", p.name.as_deref().map(str)),
			(
				"id",
				p.uuid.map(|u| {
					H::Ints(vec![
						(u >> 96) as i32,
						(u >> 64) as i32,
						(u >> 32) as i32,
						u as i32,
					])
				}),
			),
			(
				"properties",
				non_empty(&p.properties, |x| {
					Some(map(vec![
						("name", Some(str(&x.name))),
						("value", Some(str(&x.value))),
						("signature", x.signature.as_deref().map(str)),
					]))
				})?,
			),
		]),
		C::banner_patterns(layers) => list(layers, |l| {
			let pattern = match &l.pattern {
				b::BannerPatternLayer__pattern::Ty::Id(id) => {
					key(&keys.get("banner_pattern", *id)?)
				}
				b::BannerPatternLayer__pattern::Ty::Inline(p) => map(vec![
					("asset_id", Some(key(&p.assetId))),
					("translation_key", Some(str(&p.translationKey))),
				]),
			};
			Some(map(vec![
				("pattern", Some(pattern)),
				("color", Some(pick(DYES, l.colorId)?)),
			]))
		})?,
		C::base_color(v)
		| C::wolf_collar(v)
		| C::cat_collar(v)
		| C::sheep_color(v)
		| C::shulker_color(v)
		| C::tropical_fish_base_color(v)
		| C::tropical_fish_pattern_color(v) => pick(DYES, *v)?,
		C::pot_decorations(v) => list(v, |i| name("items", *i))?,
		C::block_state(v) => H::Map(v.iter().map(|p| (p.name.clone(), str(&p.value))).collect()),
		C::bees(v) => list(v, |bee| {
			let empty = matches!(&bee.nbtData.value, Payload::compound(e) if e.is_empty());
			Some(map(vec![
				("entity_data", (!empty).then(|| nbt(&bee.nbtData.value))),
				("ticks_in_hive", Some(H::Int(bee.ticksInHive))),
				("min_ticks_in_hive", Some(H::Int(bee.minTicksInHive))),
			]))
		})?,
		C::break_sound(s) => sound(s)?,
		C::villager_variant(v) => key(VILLAGER_TYPES.get(usize::try_from(*v).ok()?)?),
		C::wolf_variant(v) => key(&keys.get("wolf_variant", *v)?),
		C::wolf_sound_variant(v) => key(&keys.get("wolf_sound_variant", *v)?),
		C::pig_variant(v) => key(&keys.get("pig_variant", *v)?),
		C::cow_variant(v) => key(&keys.get("cow_variant", *v)?),
		C::frog_variant(v) => key(&keys.get("frog_variant", *v)?),
		C::cat_variant(v) => key(&keys.get("cat_variant", *v)?),
		C::painting_variant(b::SlotComponent__painting_variant::Ty::Id(id)) => {
			key(&keys.get("painting_variant", *id)?)
		}
		C::painting_variant(_) => return None,
		C::fox_variant(v) => pick(FOXES, *v)?,
		C::salmon_size(v) => pick(SALMON, *v)?,
		C::parrot_variant(v) => pick(PARROTS, *v)?,
		C::mooshroom_variant(v) => pick(MOOSHROOMS, *v)?,
		C::horse_variant(v) => pick(HORSES, *v)?,
		C::llama_variant(v) => pick(LLAMAS, *v)?,
		C::axolotl_variant(v) => pick(AXOLOTLS, *v)?,
		C::rabbit_variant(99) => str("evil"),
		C::rabbit_variant(v) => pick(RABBITS, *v)?,
		C::tropical_fish_pattern(v) => str(FISH.iter().find(|(id, _)| id == v)?.1),
		C::can_place_on(_)
		| C::can_break(_)
		| C::blocks_attacks(_)
		| C::creative_slot_lock
		| C::map_post_processing(_)
		| C::default(_) => {
			return None;
		}
	})
}

fn component_type(id: i32) -> Option<b::SlotComponentType::Ty> {
	let mut bytes = Vec::new();
	write_varint(&mut bytes, id);
	decode(&bytes, b::SlotComponentType::deserialize).ok()
}

pub fn slot(s: &b::Slot::Ty, keys: &Keys) -> Option<b::HashedSlot::Ty> {
	let b::Slot::Ty::default(count, d) = s else {
		return None;
	};
	Some(b::HashedSlot::Ty {
		itemId: d.itemId,
		itemCount: *count,
		components: d
			.components
			.iter()
			.filter_map(|c| {
				Some(b::HashedSlot__components__items::Ty {
					r#type: component_type(b::SlotComponent::virtual_tag(c))?,
					hash: component(c, keys).map_or(UNKNOWN, |h| hash(&h)),
				})
			})
			.collect(),
		removeComponents: d.removeComponents.clone(),
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn m(entries: Vec<(&str, H)>) -> H {
		H::Map(
			entries
				.into_iter()
				.map(|(k, v)| (k.to_owned(), v))
				.collect(),
		)
	}

	fn s(v: &str) -> H {
		H::Str(v.to_owned())
	}

	#[test]
	fn crc_matches_unknown_marker() {
		assert_eq!(crc32c(b"123456789"), 0xE306_9283);
		assert_eq!(crc32c(&(i32::MIN + 1).to_le_bytes()) as i32, UNKNOWN);
	}

	#[test]
	fn vanilla_vectors() {
		let mut bad = Vec::new();
		let check = |bad: &mut Vec<(i32, i32)>, want: i32, got: i32| {
			if want != got {
				bad.push((want, got))
			}
		};
		check(
			&mut bad,
			-524579578,
			hash(&m(vec![
				("seconds", H::Float(0.5)),
				("cooldown_group", s("minecraft:stick")),
			])),
		);
		let compound = m(vec![
			("id", H::Int(1)),
			("type", s("minecraft:stick")),
			("wow", m(vec![])),
			("list", H::List(vec![H::Float(0.3)])),
			("emptylist", H::List(vec![])),
		]);
		check(&mut bad, -859008863, hash(&compound));
		check(
			&mut bad,
			2007278159,
			hash(&m(vec![("floats", H::List(vec![H::Float(1.0)]))])),
		);
		let target = m(vec![
			("dimension", s("minecraft:wow")),
			("pos", H::Ints(vec![5, 6, 7])),
		]);
		check(&mut bad, 1876575779, hash(&m(vec![("target", target)])));
		let stone = |slot: i32| {
			m(vec![
				("slot", H::Int(slot)),
				(
					"item",
					m(vec![("id", s("minecraft:stone")), ("count", H::Int(1))]),
				),
			])
		};
		check(&mut bad, 231516551, hash(&H::List(vec![stone(0)])));
		check(&mut bad, 1506540737, hash(&H::List(vec![stone(1)])));
		let glint = m(vec![
			("id", s("minecraft:stone")),
			("count", H::Int(1)),
			(
				"components",
				m(vec![(
					"minecraft:enchantment_glint_override",
					H::Bool(true),
				)]),
			),
		]);
		check(&mut bad, -95388252, hash(&glint));
		let inline = m(vec![
			("asset_id", s("minecraft:bricks")),
			("translation_key", s("some_key")),
		]);
		check(
			&mut bad,
			-2006113844,
			hash(&H::List(vec![m(vec![
				("pattern", inline),
				("color", s("cyan")),
			])])),
		);
		check(
			&mut bad,
			606382024,
			hash(&H::List(vec![m(vec![
				("pattern", s("minecraft:base")),
				("color", s("white")),
			])])),
		);
		assert!(bad.is_empty(), "{bad:?}");
	}
}
