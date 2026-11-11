use super::common::{decode, effect_by_key, effect_name, item};
use super::text;
use crate::gen::convert::Version;
use crate::gen::convert::{v1_20_2 as legacy, v1_20_5 as new};
use crate::registry;
use nbt::{CompoundExt, CompoundVecExt, Entry, List, Payload, Tag};
use std::sync::OnceLock;

const OLD: Version = Version::V1_20_3;
const NEW: Version = Version::V1_20_5;
const MARKER: &str = "pds2:legacy_nbt";

type Old = legacy::slot::Ty;
type New = new::slot::Ty;
type C = new::slot_component::Ty;

const HIDE_ENCHANTMENTS: i32 = 1;
const HIDE_ATTRIBUTES: i32 = 2;
const HIDE_UNBREAKABLE: i32 = 4;
const HIDE_CAN_DESTROY: i32 = 8;
const HIDE_CAN_PLACE: i32 = 16;
const HIDE_ADDITIONAL: i32 = 32;
const HIDE_DYE: i32 = 64;
const HIDE_TRIM: i32 = 128;
const BACKUP: &str = "pds2:components";

pub const POTIONS: &[&str] = &[
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

pub const ATTRIBUTES: &[&str] = &[
	"generic.armor",
	"generic.armor_toughness",
	"generic.attack_damage",
	"generic.attack_knockback",
	"generic.attack_speed",
	"player.block_break_speed",
	"player.block_interaction_range",
	"player.entity_interaction_range",
	"generic.fall_damage_multiplier",
	"generic.flying_speed",
	"generic.follow_range",
	"generic.gravity",
	"generic.jump_strength",
	"generic.knockback_resistance",
	"generic.luck",
	"generic.max_absorption",
	"generic.max_health",
	"generic.movement_speed",
	"generic.safe_fall_distance",
	"generic.scale",
	"zombie.spawn_reinforcements",
	"generic.step_height",
];

const PATTERNS: &[(&str, &str)] = &[
	("b", "base"),
	("bl", "square_bottom_left"),
	("br", "square_bottom_right"),
	("tl", "square_top_left"),
	("tr", "square_top_right"),
	("bs", "stripe_bottom"),
	("ts", "stripe_top"),
	("ls", "stripe_left"),
	("rs", "stripe_right"),
	("cs", "stripe_center"),
	("ms", "stripe_middle"),
	("drs", "stripe_downright"),
	("dls", "stripe_downleft"),
	("ss", "small_stripes"),
	("cr", "cross"),
	("sc", "straight_cross"),
	("bt", "triangle_bottom"),
	("tt", "triangle_top"),
	("bts", "triangles_bottom"),
	("tts", "triangles_top"),
	("ld", "diagonal_left"),
	("rd", "diagonal_up_right"),
	("lud", "diagonal_up_left"),
	("rud", "diagonal_right"),
	("mc", "circle"),
	("mr", "rhombus"),
	("vh", "half_vertical"),
	("hh", "half_horizontal"),
	("vhr", "half_vertical_right"),
	("hhb", "half_horizontal_bottom"),
	("bo", "border"),
	("cbo", "curly_border"),
	("gra", "gradient"),
	("gru", "gradient_up"),
	("bri", "bricks"),
	("glb", "globe"),
	("cre", "creeper"),
	("sku", "skull"),
	("flo", "flower"),
	("moj", "mojang"),
	("pig", "piglin"),
	("flw", "flow"),
	("gus", "guster"),
];

const SLOTS: &[&str] = &[
	"any", "mainhand", "offhand", "hand", "feet", "legs", "chest", "head", "armor", "body",
];

fn vanilla_keys(registry: &str) -> Vec<String> {
	super::codec::login(NEW)
		.ok()
		.and_then(|v| {
			v["dimensionCodec"][format!("minecraft:{registry}").as_str()]["entries"]
				.as_array()
				.map(|a| {
					a.iter()
						.filter_map(|e| e["key"].as_str().map(str::to_owned))
						.collect()
				})
		})
		.unwrap_or_default()
}

pub fn banner_patterns() -> &'static [String] {
	static KEYS: OnceLock<Vec<String>> = OnceLock::new();
	KEYS.get_or_init(|| vanilla_keys("banner_pattern"))
}

fn trim_keys(material: bool) -> &'static [String] {
	static MATERIALS: OnceLock<Vec<String>> = OnceLock::new();
	static PATTERNS: OnceLock<Vec<String>> = OnceLock::new();
	match material {
		true => MATERIALS.get_or_init(|| vanilla_keys("trim_material")),
		false => PATTERNS.get_or_init(|| vanilla_keys("trim_pattern")),
	}
}

fn trim(c: &[Entry], show: bool) -> Option<new::slot_component_trim::Ty> {
	let t = c.field("Trim").and_then(Payload::as_compound)?;
	let index = |key: &str, material: bool| {
		trim_keys(material)
			.iter()
			.position(|k| strip(k) == strip(key))
			.map(|i| i as i32)
	};
	Some(new::slot_component_trim::Ty {
		material: new::slot_component_trim_material::Ty::Id(index(
			t.field("material").and_then(Payload::as_str)?,
			true,
		)?),
		pattern: new::slot_component_trim_pattern::Ty::Id(index(
			t.field("pattern").and_then(Payload::as_str)?,
			false,
		)?),
		show_in_tooltip: show,
	})
}

fn legacy_trim(t: &new::slot_component_trim::Ty) -> Option<Payload> {
	let new::slot_component_trim_material::Ty::Id(m) = &t.material else {
		return None;
	};
	let new::slot_component_trim_pattern::Ty::Id(p) = &t.pattern else {
		return None;
	};
	Some(Payload::Compound(vec![
		Entry::new(
			"material",
			Payload::String(trim_keys(true).get(*m as usize)?.clone()),
		),
		Entry::new(
			"pattern",
			Payload::String(trim_keys(false).get(*p as usize)?.clone()),
		),
	]))
}

fn strip(s: &str) -> &str {
	s.strip_prefix("minecraft:").unwrap_or(s)
}

fn enchantment_id(key: &str) -> Option<i32> {
	let key = match strip(key) {
		"sweeping" => "sweeping_edge",
		k => k,
	};
	registry::names(NEW, "enchantments")?
		.id(key)
		.map(|i| i as i32)
}

fn enchantment_key(id: i32) -> Option<String> {
	let name = registry::names(NEW, "enchantments")?.name(id as u32)?;
	Some(format!(
		"minecraft:{}",
		if name == "sweeping_edge" {
			"sweeping"
		} else {
			name
		}
	))
}

fn enchantments(list: &[Payload]) -> Vec<new::slot_component_enchantments_enchantments_items::Ty> {
	list.iter()
		.filter_map(|e| {
			let c = e.as_compound()?;
			let id = enchantment_id(c.field("id").and_then(Payload::as_str)?)?;
			let level = c
				.field("lvl")
				.and_then(Payload::to_i32)
				.unwrap_or(1)
				.clamp(0, 255);
			(level > 0)
				.then_some(new::slot_component_enchantments_enchantments_items::Ty { id, level })
		})
		.collect()
}

fn effect(c: &[Entry]) -> Option<new::item_potion_effect::Ty> {
	let id = match (
		c.field("id").and_then(Payload::as_str),
		c.field("Id").and_then(Payload::to_i32),
	) {
		(Some(key), _) => effect_by_key(NEW, key)?,
		(None, Some(legacy)) => effect_by_key(NEW, &effect_name(Version::V1_20, legacy)?)?,
		_ => return None,
	};
	let flag = |a: &str, b: &str, default: bool| {
		c.field(a)
			.or(c.field(b))
			.and_then(Payload::as_f64)
			.map_or(default, |v| v != 0.0)
	};
	Some(new::item_potion_effect::Ty {
		id,
		details: new::item_effect_detail::Ty {
			amplifier: c
				.field("amplifier")
				.or(c.field("Amplifier"))
				.and_then(Payload::to_i32)
				.unwrap_or(0) as u8 as i32,
			duration: c
				.field("duration")
				.or(c.field("Duration"))
				.and_then(Payload::to_i32)
				.unwrap_or(0),
			ambient: flag("ambient", "Ambient", false),
			show_particles: flag("show_particles", "ShowParticles", true),
			show_icon: flag("show_icon", "ShowIcon", true),
			hidden_effect: None,
		},
	})
}

fn explosion(c: &[Entry]) -> new::item_firework_explosion::Ty {
	let ints = |n: &str| match c.field(n) {
		Some(Payload::IntArray(v)) => v.clone(),
		_ => vec![],
	};
	let shape = c.field("Type").and_then(Payload::to_i32).unwrap_or(0);
	new::item_firework_explosion::Ty {
		shape: decode(&[shape.clamp(0, 4) as u8], |i| {
			new::item_firework_explosion_shape::deserialize(i)
		})
		.unwrap_or(new::item_firework_explosion_shape::Ty::SmallBall),
		colors: ints("Colors"),
		fade_colors: ints("FadeColors"),
		has_trail: c.field("Trail").and_then(Payload::to_i32).unwrap_or(0) != 0,
		has_twinkle: c.field("Flicker").and_then(Payload::to_i32).unwrap_or(0) != 0,
	}
}

fn profile(p: &Payload) -> Option<new::slot_component_profile::Ty> {
	Some(match p {
		Payload::String(name) => new::slot_component_profile::Ty {
			name: Some(name.clone()),
			uuid: None,
			properties: vec![],
		},
		Payload::Compound(c) => {
			let properties = c
				.field("Properties")
				.and_then(Payload::as_compound)
				.map(|props| {
					props
						.iter()
						.flat_map(|e| {
							e.value.values().into_iter().filter_map(|v| {
								let v = v.as_compound()?.clone();
								Some(new::slot_component_profile_properties_items::Ty {
									name: e.name.clone(),
									value: v.field("Value").and_then(Payload::as_str)?.to_owned(),
									signature: v
										.field("Signature")
										.and_then(Payload::as_str)
										.map(str::to_owned),
								})
							})
						})
						.collect()
				})
				.unwrap_or_default();
			new::slot_component_profile::Ty {
				name: c.field("Name").and_then(Payload::as_str).map(str::to_owned),
				uuid: match c.field("Id") {
					Some(id @ Payload::IntArray(_)) => id.as_uuid(),
					Some(Payload::String(s)) => nbt::json::parse_uuid(s),
					_ => None,
				},
				properties,
			}
		}
		_ => return None,
	})
}

fn banner(c: &[Entry]) -> Vec<new::banner_pattern_layer::Ty> {
	c.field("Patterns")
		.map(Payload::values)
		.unwrap_or_default()
		.iter()
		.filter_map(|p| {
			let p = p.as_compound()?;
			let code = p.field("Pattern").and_then(Payload::as_str)?;
			let key = PATTERNS.iter().find(|(c, _)| *c == code)?.1;
			let id = banner_patterns().iter().position(|k| strip(k) == key)? as i32;
			Some(new::banner_pattern_layer::Ty {
				pattern: new::banner_pattern_layer_pattern::Ty::Id(id),
				color_id: p.field("Color").and_then(Payload::to_i32).unwrap_or(0),
			})
		})
		.collect()
}

fn predicates(c: &[Entry], key: &str, show: bool) -> Option<new::slot_component_can_place_on::Ty> {
	let blocks = registry::blocks(NEW)?;
	let names = c.field(key).map(Payload::values).unwrap_or_default();
	if names.is_empty() {
		return None;
	}
	let predicates = names
		.iter()
		.filter_map(|n| {
			let Payload::String(n) = n else { return None };
			let block = blocks.block(n.split('[').next().unwrap_or(n))?;
			Some(new::item_block_predicate::Ty {
				block_set: Some(new::item_block_predicate_block_set_items::Ty::Ids(vec![
					block.id as i32,
				])),
				properties: None,
				nbt: None,
			})
		})
		.collect();
	Some(new::slot_component_can_place_on::Ty {
		predicates,
		show_tooltip: show,
	})
}

fn attributes(c: &[Entry], show: bool) -> Option<new::slot_component_attribute_modifiers::Ty> {
	let list = c
		.field("AttributeModifiers")
		.map(Payload::values)
		.unwrap_or_default();
	if list.is_empty() && show {
		return None;
	}
	let attributes =
		list.iter()
			.filter_map(|a| {
				let a = a.as_compound()?;
				let name = strip(a.field("AttributeName").and_then(Payload::as_str)?);
				let name = if name == "horse.jump_strength" {
					"generic.jump_strength"
				} else {
					name
				};
				let type_id = ATTRIBUTES.iter().position(|n| *n == name)? as i32;
				let uuid = match a.field("UUID") {
					Some(id @ Payload::IntArray(_)) => id.as_uuid()?,
					_ => {
						let half = |key: &str| {
							a.field(key)
								.and_then(Payload::as_f64)
								.map_or(0, |v| v as i32) as u64 as u128
						};
						(half("UUIDMost") << 64) | half("UUIDLeast")
					}
				};
				let operation = a
					.field("Operation")
					.and_then(Payload::to_i32)
					.unwrap_or(0)
					.clamp(0, 2) as u8;
				let slot = a
					.field("Slot")
					.and_then(Payload::as_str)
					.and_then(|s| SLOTS.iter().position(|x| *x == s))
					.unwrap_or(0) as u8;
				Some(
					new::slot_component_attribute_modifiers_attributes_items::Ty {
						type_id,
						uuid,
						name: a
							.field("Name")
							.and_then(Payload::as_str)
							.unwrap_or_default()
							.to_owned(),
						value: a.field("Amount").and_then(Payload::as_f64).unwrap_or(0.0),
						operation: decode(&[operation], |i| {
							new::slot_component_attribute_modifiers_attributes_items_operation::deserialize(i)
						})
						.ok()?,
						slot: decode(&[slot], |i| {
							new::slot_component_attribute_modifiers_attributes_items_slot::deserialize(i)
						})
						.ok()?,
					},
				)
			})
			.collect();
	Some(new::slot_component_attribute_modifiers::Ty {
		attributes,
		show_tooltip: show,
	})
}

fn components(name: &str, c: &[Entry]) -> Vec<C> {
	let mut out = Vec::new();
	let hide = c.field("HideFlags").and_then(Payload::to_i32).unwrap_or(0);
	let shown = |flag: i32| hide & flag == 0;
	if !shown(HIDE_ADDITIONAL) {
		out.push(C::HideAdditionalTooltip);
	}
	if let Some(display) = c.field("display").and_then(Payload::as_compound) {
		if let Some(n) = display.field("Name").and_then(Payload::as_str) {
			out.push(C::CustomName(text::to_nbt(n)));
		}
		let lore = display
			.field("Lore")
			.map(Payload::values)
			.unwrap_or_default();
		if !lore.is_empty() {
			out.push(C::Lore(
				lore.iter()
					.map(|l| Some(text::to_nbt(l.as_str().unwrap_or_default())))
					.collect(),
			));
		}
		if let Some(color) = display.field("color").and_then(Payload::to_i32) {
			out.push(C::DyedColor(new::slot_component_dyed_color::Ty {
				color,
				show_tooltip: shown(HIDE_DYE),
			}));
		}
		if let Some(color) = display.field("MapColor").and_then(Payload::to_i32) {
			out.push(C::MapColor(color));
		}
	}
	if let Some(d) = c
		.field("Damage")
		.and_then(Payload::to_i32)
		.filter(|d| *d > 0)
	{
		out.push(C::Damage(d));
	}
	if let Some(r) = c
		.field("RepairCost")
		.and_then(Payload::to_i32)
		.filter(|r| *r > 0)
	{
		out.push(C::RepairCost(r));
	}
	if let Some(m) = c.field("CustomModelData").and_then(Payload::to_i32) {
		out.push(C::CustomModelData(m));
	}
	if c.field("Unbreakable")
		.and_then(Payload::to_i32)
		.unwrap_or(0)
		!= 0
	{
		out.push(C::Unbreakable(shown(HIDE_UNBREAKABLE)));
	}
	for (key, stored) in [("Enchantments", false), ("StoredEnchantments", true)] {
		let Some(Payload::List(l)) = c.field(key) else {
			continue;
		};
		let raw = l.values();
		let list = enchantments(&raw);
		if !stored && list.is_empty() && !raw.is_empty() {
			out.push(C::EnchantmentGlintOverride(true));
		}
		out.push(match stored {
			false => C::Enchantments(new::slot_component_enchantments::Ty {
				enchantments: list,
				show_tooltip: shown(HIDE_ENCHANTMENTS),
			}),
			true => C::StoredEnchantments(new::slot_component_stored_enchantments::Ty {
				enchantments: list
					.into_iter()
					.map(
						|e| new::slot_component_stored_enchantments_enchantments_items::Ty {
							id: e.id,
							level: e.level,
						},
					)
					.collect(),
				show_in_tooltip: shown(HIDE_ADDITIONAL),
			}),
		});
	}
	if let Some(m) = c.field("map").and_then(Payload::to_i32) {
		out.push(C::MapId(m));
	}
	let potion = c.field("Potion").and_then(Payload::as_str).map(strip);
	let color = c.field("CustomPotionColor").and_then(Payload::to_i32);
	let effects: Vec<_> = c
		.field("custom_potion_effects")
		.or(c.field("CustomPotionEffects"))
		.map(Payload::values)
		.unwrap_or_default()
		.iter()
		.filter_map(|e| effect(e.as_compound()?))
		.collect();
	if potion.is_some() || color.is_some() || !effects.is_empty() {
		out.push(C::PotionContents(new::slot_component_potion_contents::Ty {
			potion_id: potion
				.and_then(|p| POTIONS.iter().position(|x| *x == p))
				.map(|i| i as i32),
			custom_color: color,
			custom_effects: effects,
			custom_name: None,
		}));
	}
	if let Some(p) = c.field("SkullOwner").and_then(profile) {
		out.push(C::Profile(p));
	}
	if let Some(be) = c.field("BlockEntityTag").and_then(Payload::as_compound) {
		let patterns = banner(be);
		if !patterns.is_empty() {
			out.push(C::BannerPatterns(patterns));
		}
		if let Some(base) = be.field("Base").and_then(Payload::to_i32) {
			out.push(C::BaseColor(base));
		}
		if be.field("id").is_some() {
			out.push(C::BlockEntityData(Tag::new(Payload::Compound(be.clone()))));
		}
	}
	if let Some(entity) = c
		.field("EntityTag")
		.and_then(Payload::as_compound)
		.filter(|e| e.field("id").is_some())
	{
		out.push(C::EntityData(Tag::new(Payload::Compound(entity.clone()))));
	}
	if let Some(f) = c.field("Fireworks").and_then(Payload::as_compound) {
		out.push(C::Fireworks(new::slot_component_fireworks::Ty {
			flight_duration: f.field("Flight").and_then(Payload::to_i32).unwrap_or(0),
			explosions: f
				.field("Explosions")
				.map(Payload::values)
				.unwrap_or_default()
				.iter()
				.filter_map(|e| e.as_compound().map(|e| explosion(e)))
				.collect(),
		}));
	}
	if let Some(e) = c.field("Explosion").and_then(Payload::as_compound) {
		out.push(C::FireworkExplosion(explosion(e)));
	}
	let pages = c.field("pages").map(Payload::values).unwrap_or_default();
	match strip(name) {
		"writable_book" if !pages.is_empty() => out.push(C::WritableBookContent(
			pages
				.iter()
				.map(|p| new::item_book_page::Ty {
					content: p.as_str().unwrap_or_default().to_owned(),
					filtered_content: None,
				})
				.collect(),
		)),
		"written_book" => out.push(C::WrittenBookContent(
			new::slot_component_written_book_content::Ty {
				raw_title: c
					.field("title")
					.and_then(Payload::as_str)
					.unwrap_or_default()
					.to_owned(),
				filtered_title: None,
				author: c
					.field("author")
					.and_then(Payload::as_str)
					.unwrap_or_default()
					.to_owned(),
				generation: c.field("generation").and_then(Payload::to_i32).unwrap_or(0),
				pages: pages
					.iter()
					.map(|p| new::item_written_book_page::Ty {
						content: text::to_nbt(p.as_str().unwrap_or_default()),
						filtered_content: None,
					})
					.collect(),
				resolved: c.field("resolved").and_then(Payload::to_i32).unwrap_or(0) != 0,
			},
		)),
		_ => {}
	}
	if let Some(p) = predicates(c, "CanPlaceOn", shown(HIDE_CAN_PLACE)) {
		out.push(C::CanPlaceOn(p));
	}
	if let Some(p) = predicates(c, "CanDestroy", shown(HIDE_CAN_DESTROY)) {
		out.push(C::CanBreak(new::slot_component_can_break::Ty {
			predicates: p.predicates,
			show_tooltip: p.show_tooltip,
		}));
	}
	if let Some(a) = attributes(c, shown(HIDE_ATTRIBUTES)) {
		out.push(C::AttributeModifiers(a));
	}
	if let Some(t) = trim(c, shown(HIDE_TRIM)) {
		out.push(C::Trim(t));
	}
	out
}

pub fn to_components(v: Old) -> New {
	let Old::True(s) = v else { return New::V0 };
	let Some(id) = item(OLD, NEW, s.item_id).filter(|_| s.item_count > 0) else {
		return New::V0;
	};
	let name = registry::names(NEW, "items")
		.and_then(|n| n.name(id as u32))
		.unwrap_or_default();
	let mut added = Vec::new();
	if let Some(Tag {
		value: Payload::Compound(c),
		..
	}) = s.nbt_data
	{
		if let Some(Payload::ByteArray(b)) = c.field(BACKUP) {
			let bytes: Vec<u8> = b.iter().map(|x| *x as u8).collect();
			if let Ok(mut d) = decode(&bytes, |i| new::slot_default::deserialize(i)) {
				d.item_id = id;
				return New::Default(s.item_count, d);
			}
		}
		added = components(name, &c);
		let mut original = c;
		original.put(MARKER, Payload::Byte(1));
		added.push(C::CustomData(Tag::new(Payload::Compound(original))));
	}
	New::Default(
		s.item_count,
		new::slot_default::Ty {
			item_id: id,
			added_component_count: added.len() as i32,
			removed_component_count: 0,
			components: added,
			remove_components: vec![],
		},
	)
}

pub fn non_empty(v: Old) -> New {
	match to_components(v) {
		New::V0 => New::Default(
			1,
			new::slot_default::Ty {
				item_id: 1,
				added_component_count: 0,
				removed_component_count: 0,
				components: vec![],
				remove_components: vec![],
			},
		),
		v => v,
	}
}

fn display(c: &mut Vec<Entry>) -> &mut Vec<Entry> {
	c.compound_or_insert("display")
}

fn hide(c: &mut Vec<Entry>, flag: i32, shown: bool) {
	if !shown {
		let flags = c.field("HideFlags").and_then(Payload::to_i32).unwrap_or(0) | flag;
		c.put("HideFlags", Payload::Int(flags));
	}
}

fn legacy_explosion(e: &new::item_firework_explosion::Ty) -> Payload {
	let shape = super::common::encode(&e.shape, new::item_firework_explosion_shape::serialize)
		.map_or(0, |b| b.first().copied().unwrap_or(0));
	Payload::Compound(vec![
		Entry::new("Type", Payload::Byte(shape as i8)),
		Entry::new("Colors", Payload::IntArray(e.colors.clone())),
		Entry::new("FadeColors", Payload::IntArray(e.fade_colors.clone())),
		Entry::new("Trail", Payload::Byte(e.has_trail as i8)),
		Entry::new("Flicker", Payload::Byte(e.has_twinkle as i8)),
	])
}

fn block_entity(c: &mut Vec<Entry>) -> &mut Vec<Entry> {
	c.compound_or_insert("BlockEntityTag")
}

fn legacy_tag(components: Vec<C>) -> Option<Tag> {
	let mut c = Vec::new();
	let mut rest = Vec::new();
	for comp in components {
		match comp {
			C::CustomData(Tag {
				value: Payload::Compound(mut d),
				..
			}) => {
				if d.take(MARKER).is_some() {
					return Some(Tag::new(Payload::Compound(d)));
				}
				c.extend(d);
			}
			other => rest.push(other),
		}
	}
	for comp in rest {
		match comp {
			C::CustomName(t) => display(&mut c).put("Name", Payload::String(text::from_nbt(&t))),
			C::Lore(lines) => display(&mut c).put(
				"Lore",
				List::of(
					lines
						.iter()
						.map(|l| {
							Payload::String(
								l.as_ref().map_or_else(|| "\"\"".to_owned(), text::from_nbt),
							)
						})
						.collect(),
				),
			),
			C::DyedColor(d) => {
				display(&mut c).put("color", Payload::Int(d.color));
				hide(&mut c, HIDE_DYE, d.show_tooltip);
			}
			C::MapColor(m) => display(&mut c).put("MapColor", Payload::Int(m)),
			C::Damage(d) => c.put("Damage", Payload::Int(d)),
			C::RepairCost(r) => c.put("RepairCost", Payload::Int(r)),
			C::CustomModelData(m) => c.put("CustomModelData", Payload::Int(m)),
			C::Unbreakable(shown) => {
				c.put("Unbreakable", Payload::Byte(1));
				hide(&mut c, HIDE_UNBREAKABLE, shown);
			}
			C::Enchantments(e) => {
				let l = e
					.enchantments
					.iter()
					.filter_map(|x| {
						Some(Payload::Compound(vec![
							Entry::new("id", Payload::String(enchantment_key(x.id)?)),
							Entry::new("lvl", Payload::Short(x.level as i16)),
						]))
					})
					.collect();
				c.put("Enchantments", List::of(l));
				hide(&mut c, HIDE_ENCHANTMENTS, e.show_tooltip);
			}
			C::StoredEnchantments(e) => {
				let l = e
					.enchantments
					.iter()
					.filter_map(|x| {
						Some(Payload::Compound(vec![
							Entry::new("id", Payload::String(enchantment_key(x.id)?)),
							Entry::new("lvl", Payload::Short(x.level as i16)),
						]))
					})
					.collect();
				c.put("StoredEnchantments", List::of(l));
				hide(&mut c, HIDE_ADDITIONAL, e.show_in_tooltip);
			}
			C::HideAdditionalTooltip => hide(&mut c, HIDE_ADDITIONAL, false),
			C::MapId(m) => c.put("map", Payload::Int(m)),
			C::PotionContents(p) => {
				if let Some(potion) = p.potion_id.and_then(|i| POTIONS.get(i as usize)) {
					c.put("Potion", Payload::String(format!("minecraft:{potion}")));
				}
				if let Some(color) = p.custom_color {
					c.put("CustomPotionColor", Payload::Int(color));
				}
				let effects: Vec<Payload> = p
					.custom_effects
					.iter()
					.filter_map(|e| {
						Some(Payload::Compound(vec![
							Entry::new("id", Payload::String(effect_name(NEW, e.id)?)),
							Entry::new("amplifier", Payload::Byte(e.details.amplifier as i8)),
							Entry::new("duration", Payload::Int(e.details.duration)),
							Entry::new("ambient", Payload::Byte(e.details.ambient as i8)),
							Entry::new(
								"show_particles",
								Payload::Byte(e.details.show_particles as i8),
							),
							Entry::new("show_icon", Payload::Byte(e.details.show_icon as i8)),
						]))
					})
					.collect();
				if !effects.is_empty() {
					c.put("custom_potion_effects", List::of(effects));
				}
			}
			C::Profile(p) => {
				let mut owner = Vec::new();
				if let Some(n) = p.name {
					owner.push(Entry::new("Name", Payload::String(n)));
				}
				if let Some(u) = p.uuid {
					owner.push(Entry::new("Id", Payload::uuid(u)));
				}
				let textures: Vec<Payload> = p
					.properties
					.iter()
					.filter(|x| x.name == "textures")
					.map(|x| {
						let mut v = vec![Entry::new("Value", Payload::String(x.value.clone()))];
						if let Some(s) = &x.signature {
							v.push(Entry::new("Signature", Payload::String(s.clone())));
						}
						Payload::Compound(v)
					})
					.collect();
				if !textures.is_empty() {
					owner.push(Entry::new(
						"Properties",
						Payload::Compound(vec![Entry::new("textures", List::of(textures))]),
					));
				}
				c.put("SkullOwner", Payload::Compound(owner));
			}
			C::BannerPatterns(layers) => {
				let patterns = layers
					.iter()
					.filter_map(|l| {
						let new::banner_pattern_layer_pattern::Ty::Id(id) = &l.pattern else {
							return None;
						};
						let key = strip(banner_patterns().get(*id as usize)?);
						let code = PATTERNS.iter().find(|(_, k)| *k == key)?.0;
						Some(Payload::Compound(vec![
							Entry::new("Pattern", Payload::String(code.to_owned())),
							Entry::new("Color", Payload::Int(l.color_id)),
						]))
					})
					.collect();
				block_entity(&mut c).put("Patterns", List::of(patterns));
			}
			C::BaseColor(b) => block_entity(&mut c).put("Base", Payload::Int(b)),
			C::BlockEntityData(Tag {
				value: Payload::Compound(d),
				..
			}) => block_entity(&mut c).extend(d),
			C::EntityData(t) => c.put("EntityTag", t.value),
			C::Fireworks(f) => c.put(
				"Fireworks",
				Payload::Compound(vec![
					Entry::new("Flight", Payload::Byte(f.flight_duration as i8)),
					Entry::new(
						"Explosions",
						List::of(f.explosions.iter().map(legacy_explosion).collect()),
					),
				]),
			),
			C::FireworkExplosion(e) => c.put("Explosion", legacy_explosion(&e)),
			C::WritableBookContent(pages) => c.put(
				"pages",
				List::of(
					pages
						.into_iter()
						.map(|p| Payload::String(p.content))
						.collect(),
				),
			),
			C::WrittenBookContent(b) => {
				c.put("title", Payload::String(b.raw_title));
				c.put("author", Payload::String(b.author));
				c.put("generation", Payload::Int(b.generation));
				c.put("resolved", Payload::Byte(b.resolved as i8));
				c.put(
					"pages",
					List::of(
						b.pages
							.iter()
							.map(|p| Payload::String(text::from_nbt(&p.content)))
							.collect(),
					),
				);
			}
			C::Trim(t) => {
				if let Some(trim) = legacy_trim(&t) {
					c.put("Trim", trim);
				}
				hide(&mut c, HIDE_TRIM, t.show_in_tooltip);
			}
			_ => {}
		}
	}
	(!c.is_empty()).then(|| Tag::new(Payload::Compound(c)))
}

fn restored(components: &[C]) -> bool {
	components.iter().any(|c| matches!(c, C::CustomData(Tag { value: Payload::Compound(d), .. }) if d.field(MARKER).is_some()))
}

pub fn to_legacy(v: New) -> Old {
	let New::Default(count, s) = v else {
		return Old::False;
	};
	let Some(id) = item(NEW, OLD, s.item_id).filter(|_| count > 0) else {
		return Old::False;
	};
	let backup = (!restored(&s.components)
		&& !(s.components.is_empty() && s.remove_components.is_empty()))
	.then(|| super::common::encode(&s, new::slot_default::serialize).ok())
	.flatten();
	let mut nbt = legacy_tag(s.components);
	if let Some(bytes) = backup {
		let backup = Payload::ByteArray(bytes.into_iter().map(|b| b as i8).collect());
		match &mut nbt {
			Some(Tag {
				value: Payload::Compound(c),
				..
			}) => c.put(BACKUP, backup),
			_ => {
				nbt = Some(Tag::new(Payload::Compound(vec![Entry::new(
					BACKUP, backup,
				)])))
			}
		}
	}
	Old::True(legacy::slot_true::Ty {
		item_id: id,
		item_count: count,
		nbt_data: nbt,
	})
}

pub fn block_entity_1_20_3(kind: &str, c: &mut Vec<Entry>) {
	match kind {
		"skull" => {
			let Some(Payload::Compound(p)) = c.take("profile") else {
				return;
			};
			let mut owner = Vec::new();
			if let Some(n) = p.field("name").and_then(Payload::as_str) {
				owner.push(Entry::new("Name", Payload::String(n.to_owned())));
			}
			if let Some(id) = p.field("id") {
				owner.push(Entry::new("Id", id.clone()));
			}
			let textures: Vec<Payload> = p
				.field("properties")
				.map(Payload::values)
				.unwrap_or_default()
				.iter()
				.filter_map(|x| {
					let x = x.as_compound()?;
					(x.field("name").and_then(Payload::as_str) == Some("textures")).then_some(())?;
					let mut v = vec![Entry::new(
						"Value",
						Payload::String(x.field("value").and_then(Payload::as_str)?.to_owned()),
					)];
					if let Some(s) = x.field("signature").and_then(Payload::as_str) {
						v.push(Entry::new("Signature", Payload::String(s.to_owned())));
					}
					Some(Payload::Compound(v))
				})
				.collect();
			if !textures.is_empty() {
				owner.push(Entry::new(
					"Properties",
					Payload::Compound(vec![Entry::new("textures", List::of(textures))]),
				));
			}
			c.put("SkullOwner", Payload::Compound(owner));
		}
		"banner" => {
			let patterns: Vec<Payload> = c
				.take("patterns")
				.as_ref()
				.map(Payload::values)
				.unwrap_or_default()
				.iter()
				.filter_map(|p| {
					let p = p.as_compound()?;
					let key = strip(p.field("pattern").and_then(Payload::as_str)?);
					let code = PATTERNS.iter().find(|(_, k)| *k == key)?.0;
					let color = DYES
						.iter()
						.position(|d| Some(*d) == p.field("color").and_then(Payload::as_str))?;
					Some(Payload::Compound(vec![
						Entry::new("Pattern", Payload::String(code.to_owned())),
						Entry::new("Color", Payload::Int(color as i32)),
					]))
				})
				.collect();
			if !patterns.is_empty() {
				c.put("Patterns", List::of(patterns));
			}
		}
		_ => {}
	}
}

pub fn block_entity_1_20_5(kind: &str, c: &mut Vec<Entry>) {
	match kind {
		"skull" => {
			if let Some(p) = c.take("SkullOwner").as_ref().and_then(profile) {
				let mut out = Vec::new();
				if let Some(n) = p.name {
					out.push(Entry::new("name", Payload::String(n)));
				}
				if let Some(u) = p.uuid {
					out.push(Entry::new("id", Payload::uuid(u)));
				}
				let props: Vec<Payload> = p
					.properties
					.into_iter()
					.map(|x| {
						let mut v = vec![
							Entry::new("name", Payload::String(x.name)),
							Entry::new("value", Payload::String(x.value)),
						];
						if let Some(s) = x.signature {
							v.push(Entry::new("signature", Payload::String(s)));
						}
						Payload::Compound(v)
					})
					.collect();
				if !props.is_empty() {
					out.push(Entry::new("properties", List::of(props)));
				}
				c.put("profile", Payload::Compound(out));
			}
		}
		"banner" => {
			let patterns: Vec<Payload> = c
				.take("Patterns")
				.as_ref()
				.map(Payload::values)
				.unwrap_or_default()
				.iter()
				.filter_map(|p| {
					let p = p.as_compound()?;
					let key = PATTERNS
						.iter()
						.find(|(code, _)| {
							Some(*code) == p.field("Pattern").and_then(Payload::as_str)
						})?
						.1;
					let color =
						DYES.get(p.field("Color").and_then(Payload::to_i32).unwrap_or(0) as usize)?;
					Some(Payload::Compound(vec![
						Entry::new("pattern", Payload::String(format!("minecraft:{key}"))),
						Entry::new("color", Payload::String((*color).to_owned())),
					]))
				})
				.collect();
			if !patterns.is_empty() {
				c.put("patterns", List::of(patterns));
			}
			c.take("Base");
		}
		_ => {}
	}
}

pub const DYES: &[&str] = &[
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
