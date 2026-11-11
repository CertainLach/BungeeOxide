use std::collections::HashMap;
use std::sync::OnceLock;

use bungeeoxide_nbt::{CompoundExt, CompoundVecExt, Entry, Payload};

use crate::gen::convert::{v1_8, Version};
use crate::registry;

pub(super) type Slot = v1_8::slot::Ty;

fn introduced_block(id: u16) -> Option<Version> {
	Some(match id {
		198..=212 | 255 => Version::V1_9,
		213..=217 => Version::V1_10,
		218..=234 => Version::V1_11,
		235..=252 => Version::V1_12,
		_ => return None,
	})
}

pub(super) fn block_back(target: Version, id: u16, meta: u8) -> (u16, u8) {
	if id == 137 && meta != 0 && target < Version::V1_9 {
		return (137, 0);
	}
	match introduced_block(id) {
		Some(since) if target < since => {}
		_ => return (id, meta),
	}
	match id {
		198 => (50, 5),
		199 => (35, 10),
		200 => (35, 2),
		201 => (155, 0),
		202 => (155, 2),
		203 => (156, meta),
		204 => (43, 7),
		205 => (44, 7 | (meta & 8)),
		206 => (121, 0),
		207 => (141, meta),
		208 => (2, 0),
		209 => (119, 0),
		210 | 211 | 255 => (137, 0),
		212 => (79, 0),
		213 => (159, 1),
		214 => (159, 14),
		215 => (112, 0),
		216 => (155, 0),
		218 => (23, meta),
		219..=234 => (158, meta & 7),
		235..=250 => (159, (id - 235) as u8),
		251 => (159, meta),
		252 => (35, meta),
		_ => (0, 0),
	}
}

pub(super) fn state_back(target: Version, state: i32) -> i32 {
	let (id, meta) = block_back(target, (state & 0xfff) as u16, (state >> 12 & 15) as u8);
	id as i32 | (meta as i32) << 12
}

pub(super) fn packed_back(target: Version, state: i32) -> i32 {
	let (id, meta) = block_back(target, (state >> 4 & 0xfff) as u16, (state & 15) as u8);
	(id as i32) << 4 | meta as i32
}

fn introduced_item(id: i16, damage: i16) -> Option<Version> {
	Some(match (id, damage) {
		(198..=212 | 255 | 426..=448, _) | (397, 5) => Version::V1_9,
		(213..=217, _) => Version::V1_10,
		(218..=234 | 449..=452, _) => Version::V1_11,
		(235..=252 | 453, _) => Version::V1_12,
		(355, d) if d != 0 => Version::V1_12,
		_ => return None,
	})
}

fn item_back(target: Version, id: i16, damage: i16) -> Option<(i16, i16)> {
	match introduced_item(id, damage) {
		Some(since) if target < since => {}
		_ => return None,
	}
	Some(match id {
		198 => (50, 0),
		199 => (35, 10),
		200 => (35, 2),
		201 => (155, 0),
		202 => (155, 2),
		203 => (156, 0),
		204 => (43, 7),
		205 => (44, 7),
		206 => (121, 0),
		208 => (2, 0),
		209 => (119, 0),
		210 | 211 | 255 => (137, 0),
		212 => (79, 0),
		213 => (159, 1),
		214 => (159, 14),
		215 => (112, 0),
		216 => (155, 0),
		217 => (287, 0),
		218 => (23, 0),
		219..=234 => (158, 0),
		235..=250 => (159, id - 235),
		251 => (159, damage),
		252 => (35, damage),
		355 => (355, 0),
		397 => (397, 0),
		426 => (410, 0),
		432 => (322, 0),
		433 => (393, 0),
		434 => (391, 0),
		435 => (361, 0),
		436 => (282, 0),
		437 => (373, 0),
		439 | 440 => (262, 0),
		442 => (425, 0),
		443 => (299, 0),
		444..=448 => (333, 0),
		449 => (418, 0),
		450 => (433, 0),
		452 => (265, 0),
		453 => (340, 0),
		_ => (1, 0),
	})
}

pub(super) fn item_pair_back(target: Version, data: &[i32]) -> Vec<i32> {
	let (id, damage) = (
		data.first().copied().unwrap_or(1) as i16,
		data.get(1).copied().unwrap_or(0) as i16,
	);
	let (id, damage) = item_back(target, id, damage).unwrap_or((id, damage));
	vec![id as i32, damage as i32]
}

const POTIONS: [(i16, &str); 36] = [
	(0, "water"),
	(64, "mundane"),
	(32, "thick"),
	(16, "awkward"),
	(8198, "night_vision"),
	(8262, "long_night_vision"),
	(8206, "invisibility"),
	(8270, "long_invisibility"),
	(8203, "leaping"),
	(8267, "long_leaping"),
	(8235, "strong_leaping"),
	(8195, "fire_resistance"),
	(8259, "long_fire_resistance"),
	(8194, "swiftness"),
	(8258, "long_swiftness"),
	(8226, "strong_swiftness"),
	(8202, "slowness"),
	(8266, "long_slowness"),
	(8205, "water_breathing"),
	(8269, "long_water_breathing"),
	(8261, "healing"),
	(8229, "strong_healing"),
	(8204, "harming"),
	(8236, "strong_harming"),
	(8196, "poison"),
	(8260, "long_poison"),
	(8228, "strong_poison"),
	(8193, "regeneration"),
	(8257, "long_regeneration"),
	(8225, "strong_regeneration"),
	(8201, "strength"),
	(8265, "long_strength"),
	(8233, "strong_strength"),
	(8200, "weakness"),
	(8264, "long_weakness"),
	(8203, "luck"),
];

fn potion_damage(name: &str) -> i16 {
	let name = name.strip_prefix("minecraft:").unwrap_or(name);
	POTIONS
		.iter()
		.find(|(_, n)| *n == name)
		.map_or(0, |(d, _)| *d)
}

fn potion_name(damage: i16) -> &'static str {
	if let Some((_, n)) = POTIONS.iter().find(|(d, _)| *d == damage) {
		return n;
	}
	let (effect, enhanced, extended) = (damage & 15, damage & 0x20 != 0, damage & 0x40 != 0);
	let base: [(&str, bool, bool); 15] = [
		("", false, false),
		("regeneration", true, true),
		("swiftness", true, true),
		("fire_resistance", false, true),
		("poison", true, true),
		("healing", true, false),
		("night_vision", false, true),
		("", false, false),
		("weakness", false, true),
		("strength", true, true),
		("slowness", false, true),
		("leaping", true, true),
		("harming", true, false),
		("water_breathing", false, true),
		("invisibility", false, true),
	];
	match base[effect as usize] {
		("", ..) => match damage & 0x3f {
			0 => "mundane",
			16 => "awkward",
			32 => "thick",
			_ => "empty",
		},
		(name, can_enhance, can_extend) => {
			let key = if can_enhance && enhanced {
				format!("strong_{name}")
			} else if can_extend && extended {
				format!("long_{name}")
			} else {
				name.to_owned()
			};
			POTIONS
				.iter()
				.map(|(_, n)| *n)
				.find(|n| *n == key)
				.unwrap_or(name)
		}
	}
}

struct Mobs {
	by_name: HashMap<String, u32>,
	by_id: HashMap<u32, String>,
}

fn mobs(version: Version) -> &'static Mobs {
	static CACHE: OnceLock<Vec<Mobs>> = OnceLock::new();
	let all = CACHE.get_or_init(|| {
		Version::ALL
			.iter()
			.map(|v| {
				let text = registry::folder(*v, "entities")
					.and_then(|f| registry::file(&format!("{f}/entities.txt")))
					.unwrap_or("");
				let mut m = Mobs {
					by_name: HashMap::new(),
					by_id: HashMap::new(),
				};
				for line in text.lines() {
					let mut parts = line.split(' ');
					let (Some(id), Some(name), Some("mob")) =
						(parts.next(), parts.next(), parts.next())
					else {
						continue;
					};
					let Ok(id) = id.parse::<u32>() else { continue };
					m.by_name.insert(name.to_owned(), id);
					m.by_id.insert(id, name.to_owned());
				}
				m
			})
			.collect()
	});
	&all[version as usize]
}

pub(super) fn mob_name(version: Version, id: u32) -> Option<&'static str> {
	mobs(version).by_id.get(&id).map(String::as_str)
}

pub(super) fn mob_id(version: Version, name: &str) -> Option<u32> {
	mobs(version)
		.by_name
		.get(name.strip_prefix("minecraft:").unwrap_or(name))
		.copied()
}

fn mark_key(newer: Version) -> String {
	format!("pds2|{newer}")
}

fn pretty(version: Version, id: i16) -> String {
	let name = registry::names(version, "items")
		.and_then(|n| n.name(id as u32))
		.unwrap_or("item");
	name.split('_')
		.map(|w| {
			let mut c = w.chars();
			c.next()
				.map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
				.unwrap_or_default()
		})
		.collect::<Vec<_>>()
		.join(" ")
}

fn compound(tag: &mut Option<Entry>) -> Option<&mut Vec<Entry>> {
	tag.as_mut()?.value.as_compound_mut()
}

fn entity_tag_id(tag: &Option<Entry>) -> Option<String> {
	let Some(Entry {
		value: Payload::Compound(c),
		..
	}) = tag.as_ref()
	else {
		return None;
	};
	let Some(Payload::Compound(e)) = c.field("EntityTag") else {
		return None;
	};
	match e.field("id") {
		Some(Payload::String(s)) => Some(s.clone()),
		_ => None,
	}
}

fn set_entity_tag_id(tag: &mut Option<Entry>, id: String) {
	let c = Entry::root(tag);
	let mut e = match c.take("EntityTag") {
		Some(Payload::Compound(e)) => e,
		_ => Vec::new(),
	};
	e.put_last("id", Payload::String(id));
	c.push(Entry::new("EntityTag", Payload::Compound(e)));
}

pub(super) fn down(target: Version, newer: Version, slot: Slot, mob: impl Fn(u32) -> u32) -> Slot {
	let Slot::Default(id, mut s) = slot else {
		return slot;
	};
	let (original, damage) = (id, s.item_damage);
	let mut new = (id, damage);
	let mut named = None;
	if let Some(mapped) = item_back(target, id, damage) {
		new = mapped;
		named = Some(format!("\u{a7}r{} {}", newer, pretty(newer, id)));
	}
	if target == Version::V1_8 {
		if matches!(id, 373 | 438 | 441) {
			let potion = match compound(&mut s.nbt_data).and_then(|c| c.field("Potion").cloned()) {
				Some(Payload::String(n)) => potion_damage(&n),
				_ => 0,
			};
			new = (373, if id == 373 { potion } else { potion + 8192 });
		}
		if id == 383 && damage == 0 {
			let egg = entity_tag_id(&s.nbt_data).and_then(|n| {
				mob_id(Version::V1_8, &n).or_else(|| mob_id(Version::V1_9, &n).map(&mob))
			});
			new = (383, egg.unwrap_or(0) as i16);
		}
	}
	if target == Version::V1_10 && id == 383 {
		if let Some(name) = entity_tag_id(&s.nbt_data) {
			let old = mob_id(Version::V1_11, &name)
				.map(&mob)
				.and_then(|m| mob_name(Version::V1_10, m));
			if let Some(old) = old {
				set_entity_tag_id(&mut s.nbt_data, old.to_owned());
			}
		}
	}
	if new == (original, damage) && named.is_none() {
		return Slot::Default(id, s);
	}
	let c = Entry::root(&mut s.nbt_data);
	let mut mark = vec![
		Entry::new("id", Payload::Short(original)),
		Entry::new("Damage", Payload::Short(damage)),
	];
	if let Some(name) = named {
		if !c
			.iter()
			.any(|e| e.name == "display" && matches!(e.value, Payload::Compound(_)))
		{
			c.put_last("display", Payload::Compound(Vec::new()));
		}
		if let Some(display) = c.compound_mut("display") {
			if display.field("Name").is_none() {
				display.push(Entry::new("Name", Payload::String(name)));
				mark.push(Entry::new("Named", Payload::Byte(1)));
			}
		}
	}
	c.put_last(&mark_key(newer), Payload::Compound(mark));
	s.item_damage = new.1;
	Slot::Default(new.0, s)
}

pub(super) fn up(target: Version, older: Version, slot: Slot) -> Slot {
	let Slot::Default(mut id, mut s) = slot else {
		return slot;
	};
	let mut restored = false;
	if let Some(c) = compound(&mut s.nbt_data) {
		if let Some(Payload::Compound(mark)) = c.take(&mark_key(target)) {
			restored = true;
			if let Some(Payload::Short(i)) = mark.field("id") {
				id = *i;
			}
			if let Some(Payload::Short(d)) = mark.field("Damage") {
				s.item_damage = *d;
			}
			if mark.field("Named").is_some() {
				if let Some(display) = c.compound_mut("display") {
					display.take("Name");
				}
				c.retain(|e| {
					!(e.name == "display"
						&& matches!(&e.value, Payload::Compound(d) if d.is_empty()))
				});
			}
		}
		if c.is_empty() {
			s.nbt_data = None;
		}
	}
	if restored {
		return Slot::Default(id, s);
	}
	if older == Version::V1_8 {
		if id == 373 {
			let splash = s.item_damage >= 16384;
			let name = potion_name(if splash {
				s.item_damage - 8192
			} else {
				s.item_damage
			});
			Entry::root(&mut s.nbt_data)
				.put_last("Potion", Payload::String(format!("minecraft:{name}")));
			id = if splash { 438 } else { 373 };
			s.item_damage = 0;
		}
		if id == 383 && s.item_damage != 0 {
			if entity_tag_id(&s.nbt_data).is_none() {
				if let Some(name) = mob_name(Version::V1_8, s.item_damage as u32) {
					set_entity_tag_id(&mut s.nbt_data, name.to_owned());
				}
			}
			s.item_damage = 0;
		}
	}
	if older == Version::V1_10 && id == 383 {
		if let Some(name) = entity_tag_id(&s.nbt_data) {
			if let Some(new) =
				mob_id(Version::V1_10, &name).and_then(|m| mob_name(Version::V1_11, m))
			{
				set_entity_tag_id(&mut s.nbt_data, format!("minecraft:{new}"));
			}
		}
	}
	Slot::Default(id, s)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn item(id: i16, damage: i16, tag: Option<Entry>) -> Slot {
		Slot::Default(
			id,
			v1_8::slot_default::Ty {
				item_count: 1,
				item_damage: damage,
				nbt_data: tag,
			},
		)
	}

	#[test]
	fn new_items_round_trip_through_every_step() {
		let original = item(450, 0, None);
		let at11 = down(Version::V1_10, Version::V1_11, original.clone(), |m| m);
		assert!(matches!(&at11, Slot::Default(433, _)));
		let at8 = down(Version::V1_8, Version::V1_9, at11.clone(), |m| m);
		assert!(matches!(&at8, Slot::Default(393, _)));
		assert_eq!(up(Version::V1_9, Version::V1_8, at8), at11);
		assert_eq!(up(Version::V1_11, Version::V1_10, at11), original);
	}

	#[test]
	fn potions_and_eggs_use_damage_values_on_1_8() {
		let potion = |id| {
			item(
				id,
				0,
				Some(Entry::new(
					"",
					Payload::Compound(vec![Entry::new(
						"Potion",
						Payload::String("minecraft:strong_healing".into()),
					)]),
				)),
			)
		};
		assert!(
			matches!(down(Version::V1_8, Version::V1_9, potion(373), |m| m), Slot::Default(373, s) if s.item_damage == 8229)
		);
		assert!(
			matches!(down(Version::V1_8, Version::V1_9, potion(438), |m| m), Slot::Default(373, s) if s.item_damage == 16421)
		);
		let created = up(Version::V1_9, Version::V1_8, item(373, 16421, None));
		let Slot::Default(438, s) = &created else {
			panic!("{created:?}")
		};
		let Some(Entry {
			value: Payload::Compound(c),
			..
		}) = &s.nbt_data
		else {
			panic!()
		};
		assert_eq!(
			c.field("Potion"),
			Some(&Payload::String("minecraft:strong_healing".into()))
		);
		let egg = item(
			383,
			0,
			Some(Entry::new(
				"",
				Payload::Compound(vec![Entry::new(
					"EntityTag",
					Payload::Compound(vec![Entry::new("id", Payload::String("Creeper".into()))]),
				)]),
			)),
		);
		assert!(
			matches!(down(Version::V1_8, Version::V1_9, egg, |m| m), Slot::Default(383, s) if s.item_damage == 50)
		);
		let back = up(Version::V1_9, Version::V1_8, item(383, 54, None));
		assert_eq!(
			entity_tag_id(match &back {
				Slot::Default(_, s) => &s.nbt_data,
				_ => panic!(),
			})
			.as_deref(),
			Some("Zombie")
		);
	}

	#[test]
	fn blocks_fall_back_per_target() {
		assert_eq!(block_back(Version::V1_8, 251, 3), (159, 3));
		assert_eq!(block_back(Version::V1_11, 251, 3), (159, 3));
		assert_eq!(block_back(Version::V1_12, 251, 3), (251, 3));
		assert_eq!(block_back(Version::V1_9_4, 201, 0), (201, 0));
		assert_eq!(state_back(Version::V1_8, 205 | 8 << 12), 44 | 15 << 12);
	}
}
