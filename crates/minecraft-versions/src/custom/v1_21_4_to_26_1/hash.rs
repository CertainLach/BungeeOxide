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
		Payload::End => H::Empty,
		Payload::Byte(v) => H::Byte(*v),
		Payload::Short(v) => H::Short(*v),
		Payload::Int(v) => H::Int(*v),
		Payload::Long(v) => H::Long(*v),
		Payload::Float(v) => H::Float(*v),
		Payload::Double(v) => H::Double(*v),
		Payload::ByteArray(v) => H::Bytes(v.clone()),
		Payload::String(s) => H::Str(s.clone()),
		Payload::List(l) => H::List(l.values().iter().map(nbt).collect()),
		Payload::Compound(entries) => H::Map(
			entries
				.iter()
				.map(|e| (e.name.clone(), nbt(&e.value)))
				.collect(),
		),
		Payload::IntArray(v) => H::Ints(v.clone()),
		Payload::LongArray(v) => H::Longs(v.clone()),
	}
}

fn hover(p: &Payload) -> H {
	let Payload::Compound(entries) = p else {
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
		Payload::Compound(entries) => {
			if let [e] = entries.as_slice() {
				if let ("text", Payload::String(s)) = (e.name.as_str(), &e.value) {
					return H::Str(s.clone());
				}
			}
			H::Map(
				entries
					.iter()
					.map(|e| {
						let v = match (e.name.as_str(), &e.value) {
							(k, Payload::Byte(b)) if BOOLS.contains(&k) => H::Bool(*b != 0),
							("extra" | "with", Payload::List(l)) => {
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
		Payload::List(l) => H::List(l.values().iter().map(text).collect()),
		other => nbt(other),
	}
}

fn name(kind: &str, id: i32) -> Option<H> {
	crate::registry::names(V, kind)?
		.name(u32::try_from(id).ok()?)
		.map(key)
}

fn sound(s: &b::item_sound_holder::Ty) -> Option<H> {
	Some(match s {
		b::item_sound_holder::Ty::Id(id) => key(remap::sound_name(V, *id)?),
		b::item_sound_holder::Ty::Inline(e) => map(vec![
			("sound_id", Some(key(&e.sound_name))),
			("range", e.fixed_range.map(H::Float)),
		]),
	})
}

fn sound_or(s: &b::item_sound_holder::Ty, default: &str) -> Option<Option<H>> {
	match s {
		b::item_sound_holder::Ty::Id(id) if remap::sound_name(V, *id) == Some(default) => {
			Some(None)
		}
		s => sound(s).map(Some),
	}
}

fn set(s: &b::id_set::Ty, kind: &str) -> Option<H> {
	match s {
		b::id_set::Ty::Named(tag) => Some(H::Str(format!("#{}", key_str(tag)))),
		b::id_set::Ty::Ids(ids) if ids.len() == 1 => name(kind, ids[0]),
		b::id_set::Ty::Ids(ids) => list(ids, |i| name(kind, *i)),
	}
}

fn effect_detail(d: &b::item_effect_detail::Ty) -> Option<H> {
	Some(map(vec![
		("amplifier", unless(d.amplifier, 0, |v| H::Byte(v as i8))),
		("duration", unless(d.duration, 0, H::Int)),
		("ambient", unless(d.ambient, false, H::Bool)),
		("show_particles", unless(d.show_particles, true, H::Bool)),
		("show_icon", Some(H::Bool(d.show_icon))),
		(
			"hidden_effect",
			match &d.hidden_effect {
				Some(h) => Some(effect_detail(h)?),
				None => None,
			},
		),
	]))
}

fn effect(e: &b::item_potion_effect::Ty) -> Option<H> {
	let H::Map(mut entries) = effect_detail(&e.details)? else {
		return None;
	};
	entries.push(("id".to_owned(), name("effects", e.id)?));
	Some(H::Map(entries))
}

fn consume_effect(e: &b::item_consume_effect::Ty) -> Option<H> {
	use b::item_consume_effect::Ty as E;
	Some(match e {
		E::ApplyEffects(a) => map(vec![
			("type", Some(str("minecraft:apply_effects"))),
			("effects", Some(list(&a.effects, effect)?)),
			("probability", unless(a.probability, 1.0, H::Float)),
		]),
		E::RemoveEffects(s) => map(vec![
			("type", Some(str("minecraft:remove_effects"))),
			("effects", Some(set(s, "effects")?)),
		]),
		E::ClearAllEffects => map(vec![("type", Some(str("minecraft:clear_all_effects")))]),
		E::TeleportRandomly(d) => map(vec![
			("type", Some(str("minecraft:teleport_randomly"))),
			("diameter", unless(*d, 16.0, H::Float)),
		]),
		E::PlaySound(s) => map(vec![
			("type", Some(str("minecraft:play_sound"))),
			("sound", Some(sound(s)?)),
		]),
		E::Default(_) => return None,
	})
}

fn explosion(e: &b::item_firework_explosion::Ty) -> Option<H> {
	use b::item_firework_explosion_shape::Ty as S;
	let shape = match e.shape {
		S::SmallBall => "small_ball",
		S::LargeBall => "large_ball",
		S::Star => "star",
		S::Creeper => "creeper",
		S::Burst => "burst",
		S::Default(_) => return None,
	};
	Some(map(vec![
		("shape", Some(str(shape))),
		("colors", non_empty(&e.colors, |c| Some(H::Int(*c)))?),
		(
			"fade_colors",
			non_empty(&e.fade_colors, |c| Some(H::Int(*c)))?,
		),
		("has_trail", unless(e.has_trail, false, H::Bool)),
		("has_twinkle", unless(e.has_twinkle, false, H::Bool)),
	]))
}

fn filterable(raw: H, filtered: Option<H>) -> H {
	map(vec![("raw", Some(raw)), ("filtered", filtered)])
}

fn stack(s: &b::slot::Ty, keys: &Keys) -> Option<Option<H>> {
	let b::slot::Ty::Default(count, d) = s else {
		return Some(None);
	};
	let mut patch = Vec::new();
	for c in &d.components {
		let tag = b::slot_component::virtual_tag(c);
		patch.push((
			format!("minecraft:{}", components::name(V, tag)?),
			component(c, keys)?,
		));
	}
	for r in &d.remove_components {
		let tag = b::slot_component_type::virtual_tag(r);
		patch.push((
			format!("!minecraft:{}", components::name(V, tag)?),
			H::Map(Vec::new()),
		));
	}
	Some(Some(map(vec![
		("id", Some(name("items", d.item_id)?)),
		("count", Some(H::Int(*count))),
		("components", (!patch.is_empty()).then_some(H::Map(patch))),
	])))
}

fn stacks(v: &[b::slot::Ty], keys: &Keys) -> Option<H> {
	let mut out = Vec::new();
	for s in v {
		out.extend(stack(s, keys)?);
	}
	Some(H::List(out))
}

pub fn component(c: &b::slot_component::Ty, keys: &Keys) -> Option<H> {
	use b::slot_component::Ty as C;
	let unit = || H::Map(Vec::new());
	Some(match c {
		C::CustomData(t)
		| C::DebugStickState(t)
		| C::EntityData(t)
		| C::BucketEntityData(t)
		| C::BlockEntityData(t)
		| C::Recipes(t)
		| C::Lock(t)
		| C::ContainerLoot(t)
		| C::MapDecorations(t)
		| C::IntangibleProjectile(t) => nbt(&t.value),
		C::MaxStackSize(v)
		| C::MaxDamage(v)
		| C::Damage(v)
		| C::RepairCost(v)
		| C::MapId(v)
		| C::OminousBottleAmplifier(v) => H::Int(*v),
		C::DyedColor(v) | C::MapColor(v) => H::Int(*v),
		C::Enchantable(v) => map(vec![("value", Some(H::Int(*v)))]),
		C::Unbreakable | C::Glider => unit(),
		C::EnchantmentGlintOverride(v) => H::Bool(*v),
		C::PotionDurationScale(v) => H::Float(*v),
		C::CustomName(t) | C::ItemName(t) => text(&t.value),
		C::Lore(lines) => H::List(lines.iter().map(|l| text(&l.value)).collect()),
		C::ItemModel(s) | C::TooltipStyle(s) | C::NoteBlockSound(s) => key(s),
		C::DamageResistant(s) => map(vec![("types", Some(H::Str(format!("#{}", key_str(s)))))]),
		C::ProvidesBannerPatterns(s) => H::Str(format!("#{}", key_str(s))),
		C::Rarity(r) => str(match r {
			b::slot_component_rarity::Ty::Common => "common",
			b::slot_component_rarity::Ty::Uncommon => "uncommon",
			b::slot_component_rarity::Ty::Rare => "rare",
			b::slot_component_rarity::Ty::Epic => "epic",
			b::slot_component_rarity::Ty::Default(_) => return None,
		}),
		C::Enchantments(e) => H::Map(
			e.iter()
				.map(|x| Some((keys.get("enchantment", x.id)?, H::Int(x.level))))
				.collect::<Option<_>>()?,
		),
		C::StoredEnchantments(e) => H::Map(
			e.iter()
				.map(|x| Some((keys.get("enchantment", x.id)?, H::Int(x.level))))
				.collect::<Option<_>>()?,
		),
		C::AttributeModifiers(m) => list(m, |x| {
			use b::slot_component_attribute_modifiers_items_operation::Ty as O;
			use b::slot_component_attribute_modifiers_items_slot::Ty as S;
			let operation = match x.operation {
				O::Add => "add_value",
				O::MultiplyBase => "add_multiplied_base",
				O::MultiplyTotal => "add_multiplied_total",
				O::Default(_) => return None,
			};
			let slot = match x.slot {
				S::Any => "any",
				S::MainHand => "mainhand",
				S::OffHand => "offhand",
				S::Hand => "hand",
				S::Feet => "feet",
				S::Legs => "legs",
				S::Chest => "chest",
				S::Head => "head",
				S::Armor => "armor",
				S::Body => "body",
				S::Saddle => "saddle",
				S::Default(_) => return None,
			};
			Some(map(vec![
				(
					"type",
					Some(key(ATTRIBUTES.get(usize::try_from(x.type_id).ok()?)?)),
				),
				("id", Some(key(&x.name))),
				("amount", Some(H::Double(x.value))),
				("operation", Some(str(operation))),
				("slot", (slot != "any").then(|| str(slot))),
			]))
		})?,
		C::CustomModelData(d) => map(vec![
			("floats", non_empty(&d.floats, |v| Some(H::Float(*v)))?),
			("flags", non_empty(&d.flags, |v| Some(H::Bool(*v)))?),
			("strings", non_empty(&d.strings, |v| Some(str(v)))?),
			("colors", non_empty(&d.colors, |v| Some(H::Int(*v)))?),
		]),
		C::TooltipDisplay(t) => map(vec![
			("hide_tooltip", unless(t.hide_tooltip, false, H::Bool)),
			(
				"hidden_components",
				non_empty(&t.hidden_components, |id| {
					Some(key(components::name(V, *id)?))
				})?,
			),
		]),
		C::Food(f) => map(vec![
			("nutrition", Some(H::Int(f.nutrition))),
			("saturation", Some(H::Float(f.saturation_modifier))),
			("can_always_eat", unless(f.can_always_eat, false, H::Bool)),
		]),
		C::Consumable(c) => {
			use b::slot_component_consumable_animation::Ty as A;
			let animation = match c.animation {
				A::None => "none",
				A::Eat => "eat",
				A::Bundle => "bundle",
				A::Drink => "drink",
				A::Block => "block",
				A::Bow => "bow",
				A::Spear => "spear",
				A::Crossbow => "crossbow",
				A::Spyglass => "spyglass",
				A::TootHorn => "toot_horn",
				A::Brush => "brush",
				A::Default(_) => return None,
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
		C::DeathProtection(e) => map(vec![("death_effects", non_empty(e, consume_effect)?)]),
		C::UseRemainder(s) => stack(s, keys)??,
		C::UseCooldown(u) => map(vec![
			("seconds", Some(H::Float(u.seconds))),
			("cooldown_group", u.cooldown_group.as_deref().map(key)),
		]),
		C::Tool(t) => map(vec![
			(
				"rules",
				Some(list(&t.rules, |r| {
					Some(map(vec![
						("blocks", Some(set(&r.blocks, "blocks")?)),
						("speed", r.speed.map(H::Float)),
						("correct_for_drops", r.correct_drop_for_blocks.map(H::Bool)),
					]))
				})?),
			),
			(
				"default_mining_speed",
				unless(t.default_mining_speed, 1.0, H::Float),
			),
			("damage_per_block", unless(t.damage_per_block, 1, H::Int)),
			(
				"can_destroy_blocks_in_creative",
				unless(t.can_destroy_blocks_in_creative, true, H::Bool),
			),
		]),
		C::Weapon(w) => map(vec![
			(
				"item_damage_per_attack",
				unless(w.item_damage_per_attack, 1, H::Int),
			),
			(
				"disable_blocking_for_seconds",
				unless(w.disable_blocking_for_seconds, 0.0, H::Float),
			),
		]),
		C::Equippable(e) => {
			use b::slot_component_equippable_slot::Ty as S;
			let slot = match e.slot {
				S::MainHand => "mainhand",
				S::OffHand => "offhand",
				S::Feet => "feet",
				S::Legs => "legs",
				S::Chest => "chest",
				S::Head => "head",
				S::Body => "body",
				S::Saddle => "saddle",
				S::Default(_) => return None,
			};
			map(vec![
				("slot", Some(str(slot))),
				(
					"equip_sound",
					sound_or(&e.sound, "item.armor.equip_generic")?,
				),
				("asset_id", e.model.as_deref().map(key)),
				("camera_overlay", e.camera_overlay.as_deref().map(key)),
				(
					"allowed_entities",
					match &e.allowed_entities {
						Some(s) => Some(set(s, "entities")?),
						None => None,
					},
				),
				("dispensable", unless(e.dispensable, true, H::Bool)),
				("swappable", unless(e.swappable, true, H::Bool)),
				("damage_on_hurt", unless(e.damageable, true, H::Bool)),
				(
					"equip_on_interact",
					unless(e.equip_on_interact, false, H::Bool),
				),
			])
		}
		C::Repairable(s) => map(vec![("items", Some(set(s, "items")?))]),
		C::ChargedProjectiles(v) | C::BundleContents(v) => stacks(v, keys)?,
		C::Container(v) => {
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
		C::PotionContents(p) => map(vec![
			(
				"potion",
				match p.potion_id {
					Some(id) => Some(key(POTIONS.get(usize::try_from(id).ok()?)?)),
					None => None,
				},
			),
			("custom_color", p.custom_color.map(H::Int)),
			("custom_effects", non_empty(&p.custom_effects, effect)?),
			("custom_name", p.custom_name.as_deref().map(str)),
		]),
		C::SuspiciousStewEffects(v) => list(v, |e| {
			Some(map(vec![
				("id", Some(name("effects", e.effect)?)),
				("duration", unless(e.duration, 160, H::Int)),
			]))
		})?,
		C::WritableBookContent(pages) => map(vec![(
			"pages",
			non_empty(pages, |p| {
				Some(filterable(
					str(&p.content),
					p.filtered_content.as_deref().map(str),
				))
			})?,
		)]),
		C::WrittenBookContent(w) => map(vec![
			(
				"title",
				Some(filterable(
					str(&w.raw_title),
					w.filtered_title.as_deref().map(str),
				)),
			),
			("author", Some(str(&w.author))),
			("generation", unless(w.generation, 0, H::Int)),
			(
				"pages",
				non_empty(&w.pages, |p| {
					Some(filterable(
						text(&p.content.value),
						p.filtered_content.as_ref().map(|t| text(&t.value)),
					))
				})?,
			),
			("resolved", unless(w.resolved, false, H::Bool)),
		]),
		C::Trim(t) => {
			let material = match &t.material {
				b::slot_component_trim_material::Ty::Id(id) => keys.get("trim_material", *id)?,
				b::slot_component_trim_material::Ty::Inline(_) => return None,
			};
			let pattern = match &t.pattern {
				b::slot_component_trim_pattern::Ty::Id(id) => keys.get("trim_pattern", *id)?,
				b::slot_component_trim_pattern::Ty::Inline(_) => return None,
			};
			map(vec![
				("material", Some(key(&material))),
				("pattern", Some(key(&pattern))),
			])
		}
		C::Instrument(i) => match i {
			b::slot_component_instrument::Ty::False(k) => key(k),
			b::slot_component_instrument::Ty::True(b::slot_component_instrument_true::Ty::Id(
				id,
			)) => key(&keys.get("instrument", *id)?),
			b::slot_component_instrument::Ty::True(_) => return None,
		},
		C::ProvidesTrimMaterial(i) => match i {
			b::slot_component_provides_trim_material::Ty::False(k) => key(k),
			b::slot_component_provides_trim_material::Ty::True(
				b::slot_component_provides_trim_material_true::Ty::Id(id),
			) => key(&keys.get("trim_material", *id)?),
			b::slot_component_provides_trim_material::Ty::True(_) => return None,
		},
		C::JukeboxPlayable(j) => match j {
			b::slot_component_jukebox_playable::Ty::False(k) => key(k),
			b::slot_component_jukebox_playable::Ty::True(
				b::slot_component_jukebox_playable_true::Ty::Id(id),
			) => key(&keys.get("jukebox_song", *id)?),
			b::slot_component_jukebox_playable::Ty::True(_) => return None,
		},
		C::ChickenVariant(v) => match v {
			b::slot_component_chicken_variant::Ty::False(k) => key(k),
			b::slot_component_chicken_variant::Ty::True(id) => {
				key(&keys.get("chicken_variant", *id)?)
			}
		},
		C::LodestoneTracker(l) => map(vec![
			(
				"target",
				match &l.global_position {
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
		C::FireworkExplosion(e) => explosion(e)?,
		C::Fireworks(f) => map(vec![
			(
				"flight_duration",
				unless(f.flight_duration, 0, |v| H::Byte(v as i8)),
			),
			("explosions", non_empty(&f.explosions, explosion)?),
		]),
		C::Profile(p) => map(vec![
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
		C::BannerPatterns(layers) => list(layers, |l| {
			let pattern = match &l.pattern {
				b::banner_pattern_layer_pattern::Ty::Id(id) => {
					key(&keys.get("banner_pattern", *id)?)
				}
				b::banner_pattern_layer_pattern::Ty::Inline(p) => map(vec![
					("asset_id", Some(key(&p.asset_id))),
					("translation_key", Some(str(&p.translation_key))),
				]),
			};
			Some(map(vec![
				("pattern", Some(pattern)),
				("color", Some(pick(DYES, l.color_id)?)),
			]))
		})?,
		C::BaseColor(v)
		| C::WolfCollar(v)
		| C::CatCollar(v)
		| C::SheepColor(v)
		| C::ShulkerColor(v)
		| C::TropicalFishBaseColor(v)
		| C::TropicalFishPatternColor(v) => pick(DYES, *v)?,
		C::PotDecorations(v) => list(v, |i| name("items", *i))?,
		C::BlockState(v) => H::Map(v.iter().map(|p| (p.name.clone(), str(&p.value))).collect()),
		C::Bees(v) => list(v, |bee| {
			let empty = matches!(&bee.nbt_data.value, Payload::Compound(e) if e.is_empty());
			Some(map(vec![
				("entity_data", (!empty).then(|| nbt(&bee.nbt_data.value))),
				("ticks_in_hive", Some(H::Int(bee.ticks_in_hive))),
				("min_ticks_in_hive", Some(H::Int(bee.min_ticks_in_hive))),
			]))
		})?,
		C::BreakSound(s) => sound(s)?,
		C::VillagerVariant(v) => key(VILLAGER_TYPES.get(usize::try_from(*v).ok()?)?),
		C::WolfVariant(v) => key(&keys.get("wolf_variant", *v)?),
		C::WolfSoundVariant(v) => key(&keys.get("wolf_sound_variant", *v)?),
		C::PigVariant(v) => key(&keys.get("pig_variant", *v)?),
		C::CowVariant(v) => key(&keys.get("cow_variant", *v)?),
		C::FrogVariant(v) => key(&keys.get("frog_variant", *v)?),
		C::CatVariant(v) => key(&keys.get("cat_variant", *v)?),
		C::PaintingVariant(b::slot_component_painting_variant::Ty::Id(id)) => {
			key(&keys.get("painting_variant", *id)?)
		}
		C::PaintingVariant(_) => return None,
		C::FoxVariant(v) => pick(FOXES, *v)?,
		C::SalmonSize(v) => pick(SALMON, *v)?,
		C::ParrotVariant(v) => pick(PARROTS, *v)?,
		C::MooshroomVariant(v) => pick(MOOSHROOMS, *v)?,
		C::HorseVariant(v) => pick(HORSES, *v)?,
		C::LlamaVariant(v) => pick(LLAMAS, *v)?,
		C::AxolotlVariant(v) => pick(AXOLOTLS, *v)?,
		C::RabbitVariant(99) => str("evil"),
		C::RabbitVariant(v) => pick(RABBITS, *v)?,
		C::TropicalFishPattern(v) => str(FISH.iter().find(|(id, _)| id == v)?.1),
		C::CanPlaceOn(_)
		| C::CanBreak(_)
		| C::BlocksAttacks(_)
		| C::CreativeSlotLock
		| C::MapPostProcessing(_)
		| C::Default(_) => {
			return None;
		}
	})
}

fn component_type(id: i32) -> Option<b::slot_component_type::Ty> {
	let mut bytes = Vec::new();
	write_varint(&mut bytes, id);
	decode(&bytes, b::slot_component_type::deserialize).ok()
}

pub fn slot(s: &b::slot::Ty, keys: &Keys) -> Option<b::hashed_slot::Ty> {
	let b::slot::Ty::Default(count, d) = s else {
		return None;
	};
	Some(b::hashed_slot::Ty {
		item_id: d.item_id,
		item_count: *count,
		components: d
			.components
			.iter()
			.filter_map(|c| {
				Some(b::hashed_slot_components_items::Ty {
					r#type: component_type(b::slot_component::virtual_tag(c))?,
					hash: component(c, keys).map_or(UNKNOWN, |h| hash(&h)),
				})
			})
			.collect(),
		remove_components: d.remove_components.clone(),
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
