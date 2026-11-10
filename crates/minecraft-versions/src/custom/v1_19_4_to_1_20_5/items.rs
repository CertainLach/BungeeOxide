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
type New = new::Slot::Ty;
type C = new::SlotComponent::Ty;

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

fn trim(c: &[Entry], show: bool) -> Option<new::SlotComponent__trim::Ty> {
	let t = c.field("Trim").and_then(Payload::as_compound)?;
	let index = |key: &str, material: bool| {
		trim_keys(material)
			.iter()
			.position(|k| strip(k) == strip(key))
			.map(|i| i as i32)
	};
	Some(new::SlotComponent__trim::Ty {
		material: new::SlotComponent__trim__material::Ty::Id(index(
			t.field("material").and_then(Payload::as_str)?,
			true,
		)?),
		pattern: new::SlotComponent__trim__pattern::Ty::Id(index(
			t.field("pattern").and_then(Payload::as_str)?,
			false,
		)?),
		showInTooltip: show,
	})
}

fn legacy_trim(t: &new::SlotComponent__trim::Ty) -> Option<Payload> {
	let new::SlotComponent__trim__material::Ty::Id(m) = &t.material else {
		return None;
	};
	let new::SlotComponent__trim__pattern::Ty::Id(p) = &t.pattern else {
		return None;
	};
	Some(Payload::compound(vec![
		Entry::new(
			"material",
			Payload::string(trim_keys(true).get(*m as usize)?.clone()),
		),
		Entry::new(
			"pattern",
			Payload::string(trim_keys(false).get(*p as usize)?.clone()),
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

fn enchantments(
	list: &[Payload],
) -> Vec<new::SlotComponent__enchantments__enchantments__items::Ty> {
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
				.then_some(new::SlotComponent__enchantments__enchantments__items::Ty { id, level })
		})
		.collect()
}

fn effect(c: &[Entry]) -> Option<new::ItemPotionEffect::Ty> {
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
	Some(new::ItemPotionEffect::Ty {
		id,
		details: new::ItemEffectDetail::Ty {
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
			showParticles: flag("show_particles", "ShowParticles", true),
			showIcon: flag("show_icon", "ShowIcon", true),
			hiddenEffect: None,
		},
	})
}

fn explosion(c: &[Entry]) -> new::ItemFireworkExplosion::Ty {
	let ints = |n: &str| match c.field(n) {
		Some(Payload::intArray(v)) => v.clone(),
		_ => vec![],
	};
	let shape = c.field("Type").and_then(Payload::to_i32).unwrap_or(0);
	new::ItemFireworkExplosion::Ty {
		shape: decode(&[shape.clamp(0, 4) as u8], |i| {
			new::ItemFireworkExplosion__shape::deserialize(i)
		})
		.unwrap_or(new::ItemFireworkExplosion__shape::Ty::small_ball),
		colors: ints("Colors"),
		fadeColors: ints("FadeColors"),
		hasTrail: c.field("Trail").and_then(Payload::to_i32).unwrap_or(0) != 0,
		hasTwinkle: c.field("Flicker").and_then(Payload::to_i32).unwrap_or(0) != 0,
	}
}

fn profile(p: &Payload) -> Option<new::SlotComponent__profile::Ty> {
	Some(match p {
		Payload::string(name) => new::SlotComponent__profile::Ty {
			name: Some(name.clone()),
			uuid: None,
			properties: vec![],
		},
		Payload::compound(c) => {
			let properties = c
				.field("Properties")
				.and_then(Payload::as_compound)
				.map(|props| {
					props
						.iter()
						.flat_map(|e| {
							e.value.values().into_iter().filter_map(|v| {
								let v = v.as_compound()?.clone();
								Some(new::SlotComponent__profile__properties__items::Ty {
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
			new::SlotComponent__profile::Ty {
				name: c.field("Name").and_then(Payload::as_str).map(str::to_owned),
				uuid: match c.field("Id") {
					Some(id @ Payload::intArray(_)) => id.as_uuid(),
					Some(Payload::string(s)) => nbt::json::parse_uuid(s),
					_ => None,
				},
				properties,
			}
		}
		_ => return None,
	})
}

fn banner(c: &[Entry]) -> Vec<new::BannerPatternLayer::Ty> {
	c.field("Patterns")
		.map(Payload::values)
		.unwrap_or_default()
		.iter()
		.filter_map(|p| {
			let p = p.as_compound()?;
			let code = p.field("Pattern").and_then(Payload::as_str)?;
			let key = PATTERNS.iter().find(|(c, _)| *c == code)?.1;
			let id = banner_patterns().iter().position(|k| strip(k) == key)? as i32;
			Some(new::BannerPatternLayer::Ty {
				pattern: new::BannerPatternLayer__pattern::Ty::Id(id),
				colorId: p.field("Color").and_then(Payload::to_i32).unwrap_or(0),
			})
		})
		.collect()
}

fn predicates(c: &[Entry], key: &str, show: bool) -> Option<new::SlotComponent__can_place_on::Ty> {
	let blocks = registry::blocks(NEW)?;
	let names = c.field(key).map(Payload::values).unwrap_or_default();
	if names.is_empty() {
		return None;
	}
	let predicates = names
		.iter()
		.filter_map(|n| {
			let Payload::string(n) = n else { return None };
			let block = blocks.block(n.split('[').next().unwrap_or(n))?;
			Some(new::ItemBlockPredicate::Ty {
				blockSet: Some(new::ItemBlockPredicate__blockSet__items::Ty::Ids(vec![
					block.id as i32,
				])),
				properties: None,
				nbt: None,
			})
		})
		.collect();
	Some(new::SlotComponent__can_place_on::Ty {
		predicates,
		showTooltip: show,
	})
}

fn attributes(c: &[Entry], show: bool) -> Option<new::SlotComponent__attribute_modifiers::Ty> {
	let list = c
		.field("AttributeModifiers")
		.map(Payload::values)
		.unwrap_or_default();
	if list.is_empty() && show {
		return None;
	}
	let attributes = list
		.iter()
		.filter_map(|a| {
			let a = a.as_compound()?;
			let name = strip(a.field("AttributeName").and_then(Payload::as_str)?);
			let name = if name == "horse.jump_strength" {
				"generic.jump_strength"
			} else {
				name
			};
			let typeId = ATTRIBUTES.iter().position(|n| *n == name)? as i32;
			let uuid = match a.field("UUID") {
				Some(id @ Payload::intArray(_)) => id.as_uuid()?,
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
				new::SlotComponent__attribute_modifiers__attributes__items::Ty {
					typeId,
					uuid,
					name: a
						.field("Name")
						.and_then(Payload::as_str)
						.unwrap_or_default()
						.to_owned(),
					value: a.field("Amount").and_then(Payload::as_f64).unwrap_or(0.0),
					operation: decode(&[operation], |i| {
						new::SlotComponent__attribute_modifiers__attributes__items__operation::deserialize(i)
					})
					.ok()?,
					slot: decode(&[slot], |i| {
						new::SlotComponent__attribute_modifiers__attributes__items__slot::deserialize(i)
					})
					.ok()?,
				},
			)
		})
		.collect();
	Some(new::SlotComponent__attribute_modifiers::Ty {
		attributes,
		showTooltip: show,
	})
}

fn components(name: &str, c: &[Entry]) -> Vec<C> {
	let mut out = Vec::new();
	let hide = c.field("HideFlags").and_then(Payload::to_i32).unwrap_or(0);
	let shown = |flag: i32| hide & flag == 0;
	if !shown(HIDE_ADDITIONAL) {
		out.push(C::hide_additional_tooltip);
	}
	if let Some(display) = c.field("display").and_then(Payload::as_compound) {
		if let Some(n) = display.field("Name").and_then(Payload::as_str) {
			out.push(C::custom_name(text::to_nbt(n)));
		}
		let lore = display
			.field("Lore")
			.map(Payload::values)
			.unwrap_or_default();
		if !lore.is_empty() {
			out.push(C::lore(
				lore.iter()
					.map(|l| Some(text::to_nbt(l.as_str().unwrap_or_default())))
					.collect(),
			));
		}
		if let Some(color) = display.field("color").and_then(Payload::to_i32) {
			out.push(C::dyed_color(new::SlotComponent__dyed_color::Ty {
				color,
				showTooltip: shown(HIDE_DYE),
			}));
		}
		if let Some(color) = display.field("MapColor").and_then(Payload::to_i32) {
			out.push(C::map_color(color));
		}
	}
	if let Some(d) = c
		.field("Damage")
		.and_then(Payload::to_i32)
		.filter(|d| *d > 0)
	{
		out.push(C::damage(d));
	}
	if let Some(r) = c
		.field("RepairCost")
		.and_then(Payload::to_i32)
		.filter(|r| *r > 0)
	{
		out.push(C::repair_cost(r));
	}
	if let Some(m) = c.field("CustomModelData").and_then(Payload::to_i32) {
		out.push(C::custom_model_data(m));
	}
	if c.field("Unbreakable")
		.and_then(Payload::to_i32)
		.unwrap_or(0)
		!= 0
	{
		out.push(C::unbreakable(shown(HIDE_UNBREAKABLE)));
	}
	for (key, stored) in [("Enchantments", false), ("StoredEnchantments", true)] {
		let Some(Payload::list(l)) = c.field(key) else {
			continue;
		};
		let raw = l.values();
		let list = enchantments(&raw);
		if !stored && list.is_empty() && !raw.is_empty() {
			out.push(C::enchantment_glint_override(true));
		}
		out.push(match stored {
			false => C::enchantments(new::SlotComponent__enchantments::Ty {
				enchantments: list,
				showTooltip: shown(HIDE_ENCHANTMENTS),
			}),
			true => C::stored_enchantments(new::SlotComponent__stored_enchantments::Ty {
				enchantments: list
					.into_iter()
					.map(
						|e| new::SlotComponent__stored_enchantments__enchantments__items::Ty {
							id: e.id,
							level: e.level,
						},
					)
					.collect(),
				showInTooltip: shown(HIDE_ADDITIONAL),
			}),
		});
	}
	if let Some(m) = c.field("map").and_then(Payload::to_i32) {
		out.push(C::map_id(m));
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
		out.push(C::potion_contents(
			new::SlotComponent__potion_contents::Ty {
				potionId: potion
					.and_then(|p| POTIONS.iter().position(|x| *x == p))
					.map(|i| i as i32),
				customColor: color,
				customEffects: effects,
				customName: None,
			},
		));
	}
	if let Some(p) = c.field("SkullOwner").and_then(profile) {
		out.push(C::profile(p));
	}
	if let Some(be) = c.field("BlockEntityTag").and_then(Payload::as_compound) {
		let patterns = banner(be);
		if !patterns.is_empty() {
			out.push(C::banner_patterns(patterns));
		}
		if let Some(base) = be.field("Base").and_then(Payload::to_i32) {
			out.push(C::base_color(base));
		}
		if be.field("id").is_some() {
			out.push(C::block_entity_data(Tag::new(Payload::compound(
				be.clone(),
			))));
		}
	}
	if let Some(entity) = c
		.field("EntityTag")
		.and_then(Payload::as_compound)
		.filter(|e| e.field("id").is_some())
	{
		out.push(C::entity_data(Tag::new(Payload::compound(entity.clone()))));
	}
	if let Some(f) = c.field("Fireworks").and_then(Payload::as_compound) {
		out.push(C::fireworks(new::SlotComponent__fireworks::Ty {
			flightDuration: f.field("Flight").and_then(Payload::to_i32).unwrap_or(0),
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
		out.push(C::firework_explosion(explosion(e)));
	}
	let pages = c.field("pages").map(Payload::values).unwrap_or_default();
	match strip(name) {
		"writable_book" if !pages.is_empty() => out.push(C::writable_book_content(
			pages
				.iter()
				.map(|p| new::ItemBookPage::Ty {
					content: p.as_str().unwrap_or_default().to_owned(),
					filteredContent: None,
				})
				.collect(),
		)),
		"written_book" => out.push(C::written_book_content(
			new::SlotComponent__written_book_content::Ty {
				rawTitle: c
					.field("title")
					.and_then(Payload::as_str)
					.unwrap_or_default()
					.to_owned(),
				filteredTitle: None,
				author: c
					.field("author")
					.and_then(Payload::as_str)
					.unwrap_or_default()
					.to_owned(),
				generation: c.field("generation").and_then(Payload::to_i32).unwrap_or(0),
				pages: pages
					.iter()
					.map(|p| new::ItemWrittenBookPage::Ty {
						content: text::to_nbt(p.as_str().unwrap_or_default()),
						filteredContent: None,
					})
					.collect(),
				resolved: c.field("resolved").and_then(Payload::to_i32).unwrap_or(0) != 0,
			},
		)),
		_ => {}
	}
	if let Some(p) = predicates(c, "CanPlaceOn", shown(HIDE_CAN_PLACE)) {
		out.push(C::can_place_on(p));
	}
	if let Some(p) = predicates(c, "CanDestroy", shown(HIDE_CAN_DESTROY)) {
		out.push(C::can_break(new::SlotComponent__can_break::Ty {
			predicates: p.predicates,
			showTooltip: p.showTooltip,
		}));
	}
	if let Some(a) = attributes(c, shown(HIDE_ATTRIBUTES)) {
		out.push(C::attribute_modifiers(a));
	}
	if let Some(t) = trim(c, shown(HIDE_TRIM)) {
		out.push(C::trim(t));
	}
	out
}

pub fn to_components(v: Old) -> New {
	let Old::r#true(s) = v else { return New::v0 };
	let Some(id) = item(OLD, NEW, s.itemId).filter(|_| s.itemCount > 0) else {
		return New::v0;
	};
	let name = registry::names(NEW, "items")
		.and_then(|n| n.name(id as u32))
		.unwrap_or_default();
	let mut added = Vec::new();
	if let Some(Tag {
		value: Payload::compound(c),
		..
	}) = s.nbtData
	{
		if let Some(Payload::byteArray(b)) = c.field(BACKUP) {
			let bytes: Vec<u8> = b.iter().map(|x| *x as u8).collect();
			if let Ok(mut d) = decode(&bytes, |i| new::Slot__default::deserialize(i)) {
				d.itemId = id;
				return New::default(s.itemCount, d);
			}
		}
		added = components(name, &c);
		let mut original = c;
		original.put(MARKER, Payload::byte(1));
		added.push(C::custom_data(Tag::new(Payload::compound(original))));
	}
	New::default(
		s.itemCount,
		new::Slot__default::Ty {
			itemId: id,
			addedComponentCount: added.len() as i32,
			removedComponentCount: 0,
			components: added,
			removeComponents: vec![],
		},
	)
}

pub fn non_empty(v: Old) -> New {
	match to_components(v) {
		New::v0 => New::default(
			1,
			new::Slot__default::Ty {
				itemId: 1,
				addedComponentCount: 0,
				removedComponentCount: 0,
				components: vec![],
				removeComponents: vec![],
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
		c.put("HideFlags", Payload::int(flags));
	}
}

fn legacy_explosion(e: &new::ItemFireworkExplosion::Ty) -> Payload {
	let shape = super::common::encode(&e.shape, new::ItemFireworkExplosion__shape::serialize)
		.map_or(0, |b| b.first().copied().unwrap_or(0));
	Payload::compound(vec![
		Entry::new("Type", Payload::byte(shape as i8)),
		Entry::new("Colors", Payload::intArray(e.colors.clone())),
		Entry::new("FadeColors", Payload::intArray(e.fadeColors.clone())),
		Entry::new("Trail", Payload::byte(e.hasTrail as i8)),
		Entry::new("Flicker", Payload::byte(e.hasTwinkle as i8)),
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
			C::custom_data(Tag {
				value: Payload::compound(mut d),
				..
			}) => {
				if d.take(MARKER).is_some() {
					return Some(Tag::new(Payload::compound(d)));
				}
				c.extend(d);
			}
			other => rest.push(other),
		}
	}
	for comp in rest {
		match comp {
			C::custom_name(t) => display(&mut c).put("Name", Payload::string(text::from_nbt(&t))),
			C::lore(lines) => display(&mut c).put(
				"Lore",
				List::of(
					lines
						.iter()
						.map(|l| {
							Payload::string(
								l.as_ref().map_or_else(|| "\"\"".to_owned(), text::from_nbt),
							)
						})
						.collect(),
				),
			),
			C::dyed_color(d) => {
				display(&mut c).put("color", Payload::int(d.color));
				hide(&mut c, HIDE_DYE, d.showTooltip);
			}
			C::map_color(m) => display(&mut c).put("MapColor", Payload::int(m)),
			C::damage(d) => c.put("Damage", Payload::int(d)),
			C::repair_cost(r) => c.put("RepairCost", Payload::int(r)),
			C::custom_model_data(m) => c.put("CustomModelData", Payload::int(m)),
			C::unbreakable(shown) => {
				c.put("Unbreakable", Payload::byte(1));
				hide(&mut c, HIDE_UNBREAKABLE, shown);
			}
			C::enchantments(e) => {
				let l = e
					.enchantments
					.iter()
					.filter_map(|x| {
						Some(Payload::compound(vec![
							Entry::new("id", Payload::string(enchantment_key(x.id)?)),
							Entry::new("lvl", Payload::short(x.level as i16)),
						]))
					})
					.collect();
				c.put("Enchantments", List::of(l));
				hide(&mut c, HIDE_ENCHANTMENTS, e.showTooltip);
			}
			C::stored_enchantments(e) => {
				let l = e
					.enchantments
					.iter()
					.filter_map(|x| {
						Some(Payload::compound(vec![
							Entry::new("id", Payload::string(enchantment_key(x.id)?)),
							Entry::new("lvl", Payload::short(x.level as i16)),
						]))
					})
					.collect();
				c.put("StoredEnchantments", List::of(l));
				hide(&mut c, HIDE_ADDITIONAL, e.showInTooltip);
			}
			C::hide_additional_tooltip => hide(&mut c, HIDE_ADDITIONAL, false),
			C::map_id(m) => c.put("map", Payload::int(m)),
			C::potion_contents(p) => {
				if let Some(potion) = p.potionId.and_then(|i| POTIONS.get(i as usize)) {
					c.put("Potion", Payload::string(format!("minecraft:{potion}")));
				}
				if let Some(color) = p.customColor {
					c.put("CustomPotionColor", Payload::int(color));
				}
				let effects: Vec<Payload> = p
					.customEffects
					.iter()
					.filter_map(|e| {
						Some(Payload::compound(vec![
							Entry::new("id", Payload::string(effect_name(NEW, e.id)?)),
							Entry::new("amplifier", Payload::byte(e.details.amplifier as i8)),
							Entry::new("duration", Payload::int(e.details.duration)),
							Entry::new("ambient", Payload::byte(e.details.ambient as i8)),
							Entry::new(
								"show_particles",
								Payload::byte(e.details.showParticles as i8),
							),
							Entry::new("show_icon", Payload::byte(e.details.showIcon as i8)),
						]))
					})
					.collect();
				if !effects.is_empty() {
					c.put("custom_potion_effects", List::of(effects));
				}
			}
			C::profile(p) => {
				let mut owner = Vec::new();
				if let Some(n) = p.name {
					owner.push(Entry::new("Name", Payload::string(n)));
				}
				if let Some(u) = p.uuid {
					owner.push(Entry::new("Id", Payload::uuid(u)));
				}
				let textures: Vec<Payload> = p
					.properties
					.iter()
					.filter(|x| x.name == "textures")
					.map(|x| {
						let mut v = vec![Entry::new("Value", Payload::string(x.value.clone()))];
						if let Some(s) = &x.signature {
							v.push(Entry::new("Signature", Payload::string(s.clone())));
						}
						Payload::compound(v)
					})
					.collect();
				if !textures.is_empty() {
					owner.push(Entry::new(
						"Properties",
						Payload::compound(vec![Entry::new("textures", List::of(textures))]),
					));
				}
				c.put("SkullOwner", Payload::compound(owner));
			}
			C::banner_patterns(layers) => {
				let patterns = layers
					.iter()
					.filter_map(|l| {
						let new::BannerPatternLayer__pattern::Ty::Id(id) = &l.pattern else {
							return None;
						};
						let key = strip(banner_patterns().get(*id as usize)?);
						let code = PATTERNS.iter().find(|(_, k)| *k == key)?.0;
						Some(Payload::compound(vec![
							Entry::new("Pattern", Payload::string(code.to_owned())),
							Entry::new("Color", Payload::int(l.colorId)),
						]))
					})
					.collect();
				block_entity(&mut c).put("Patterns", List::of(patterns));
			}
			C::base_color(b) => block_entity(&mut c).put("Base", Payload::int(b)),
			C::block_entity_data(Tag {
				value: Payload::compound(d),
				..
			}) => block_entity(&mut c).extend(d),
			C::entity_data(t) => c.put("EntityTag", t.value),
			C::fireworks(f) => c.put(
				"Fireworks",
				Payload::compound(vec![
					Entry::new("Flight", Payload::byte(f.flightDuration as i8)),
					Entry::new(
						"Explosions",
						List::of(f.explosions.iter().map(legacy_explosion).collect()),
					),
				]),
			),
			C::firework_explosion(e) => c.put("Explosion", legacy_explosion(&e)),
			C::writable_book_content(pages) => c.put(
				"pages",
				List::of(
					pages
						.into_iter()
						.map(|p| Payload::string(p.content))
						.collect(),
				),
			),
			C::written_book_content(b) => {
				c.put("title", Payload::string(b.rawTitle));
				c.put("author", Payload::string(b.author));
				c.put("generation", Payload::int(b.generation));
				c.put("resolved", Payload::byte(b.resolved as i8));
				c.put(
					"pages",
					List::of(
						b.pages
							.iter()
							.map(|p| Payload::string(text::from_nbt(&p.content)))
							.collect(),
					),
				);
			}
			C::trim(t) => {
				if let Some(trim) = legacy_trim(&t) {
					c.put("Trim", trim);
				}
				hide(&mut c, HIDE_TRIM, t.showInTooltip);
			}
			_ => {}
		}
	}
	(!c.is_empty()).then(|| Tag::new(Payload::compound(c)))
}

fn restored(components: &[C]) -> bool {
	components.iter().any(|c| matches!(c, C::custom_data(Tag { value: Payload::compound(d), .. }) if d.field(MARKER).is_some()))
}

pub fn to_legacy(v: New) -> Old {
	let New::default(count, s) = v else {
		return Old::r#false;
	};
	let Some(id) = item(NEW, OLD, s.itemId).filter(|_| count > 0) else {
		return Old::r#false;
	};
	let backup = (!restored(&s.components)
		&& !(s.components.is_empty() && s.removeComponents.is_empty()))
	.then(|| super::common::encode(&s, new::Slot__default::serialize).ok())
	.flatten();
	let mut nbt = legacy_tag(s.components);
	if let Some(bytes) = backup {
		let backup = Payload::byteArray(bytes.into_iter().map(|b| b as i8).collect());
		match &mut nbt {
			Some(Tag {
				value: Payload::compound(c),
				..
			}) => c.put(BACKUP, backup),
			_ => {
				nbt = Some(Tag::new(Payload::compound(vec![Entry::new(
					BACKUP, backup,
				)])))
			}
		}
	}
	Old::r#true(legacy::slot__true::Ty {
		itemId: id,
		itemCount: count,
		nbtData: nbt,
	})
}

pub fn block_entity_1_20_3(kind: &str, c: &mut Vec<Entry>) {
	match kind {
		"skull" => {
			let Some(Payload::compound(p)) = c.take("profile") else {
				return;
			};
			let mut owner = Vec::new();
			if let Some(n) = p.field("name").and_then(Payload::as_str) {
				owner.push(Entry::new("Name", Payload::string(n.to_owned())));
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
						Payload::string(x.field("value").and_then(Payload::as_str)?.to_owned()),
					)];
					if let Some(s) = x.field("signature").and_then(Payload::as_str) {
						v.push(Entry::new("Signature", Payload::string(s.to_owned())));
					}
					Some(Payload::compound(v))
				})
				.collect();
			if !textures.is_empty() {
				owner.push(Entry::new(
					"Properties",
					Payload::compound(vec![Entry::new("textures", List::of(textures))]),
				));
			}
			c.put("SkullOwner", Payload::compound(owner));
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
					Some(Payload::compound(vec![
						Entry::new("Pattern", Payload::string(code.to_owned())),
						Entry::new("Color", Payload::int(color as i32)),
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
					out.push(Entry::new("name", Payload::string(n)));
				}
				if let Some(u) = p.uuid {
					out.push(Entry::new("id", Payload::uuid(u)));
				}
				let props: Vec<Payload> = p
					.properties
					.into_iter()
					.map(|x| {
						let mut v = vec![
							Entry::new("name", Payload::string(x.name)),
							Entry::new("value", Payload::string(x.value)),
						];
						if let Some(s) = x.signature {
							v.push(Entry::new("signature", Payload::string(s)));
						}
						Payload::compound(v)
					})
					.collect();
				if !props.is_empty() {
					out.push(Entry::new("properties", List::of(props)));
				}
				c.put("profile", Payload::compound(out));
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
					Some(Payload::compound(vec![
						Entry::new("pattern", Payload::string(format!("minecraft:{key}"))),
						Entry::new("color", Payload::string((*color).to_owned())),
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
