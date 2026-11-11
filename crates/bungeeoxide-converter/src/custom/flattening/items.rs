use std::collections::HashMap;
use std::sync::OnceLock;

use bungeeoxide_nbt::{CompoundExt, CompoundVecExt, Entry, List, ListItems, Payload};

use super::blocks::{
	item_earlier, item_later, new_blocks, new_items, old_blocks, raw_of_state, state_of_raw, OLD,
};
use super::nbtx::{self, children, root};
use super::text::{json_to_legacy, legacy_to_json};
use super::wire::{put as put_value, Reader};
use crate::gen::convert::{v1_12_2 as a, v1_13 as b, v1_13_1 as c, v1_13_2 as d};
use crate::pds::convert::Result;
use crate::registry;
use crate::session::Session;

const ORIGINAL: &str = "pds2|id";
const FLAT: &str = "pds2|item";
const LATER_FLAT: &str = "pds2|item_1_13_1";
const NAME: &str = "pds2|Name";
const NAME_ADDED: &str = "pds2|NameAdded";
const PLACE: &str = "pds2|CanPlaceOn";
const DESTROY: &str = "pds2|CanDestroy";
const ENCHANTS: &str = "pds2|Enchantments";
const STORED: &str = "pds2|StoredEnchantments";
const LORE: &str = "pds2|Lore";
const LORE_ADDED: &str = "pds2|LoreAdded";
const LEGACY_ENCHANTMENT: &str = "pds2:legacy_";

fn damageable(id: i16) -> bool {
	matches!(id, 256..=259 | 261 | 267..=279 | 283..=286 | 290..=294 | 298..=317 | 346 | 359 | 398 | 442 | 443)
}

pub(super) const SPAWN_EGGS: &[(&str, &str)] = &[
	("bat", "bat"),
	("blaze", "blaze"),
	("cave_spider", "cave_spider"),
	("chicken", "chicken"),
	("cow", "cow"),
	("creeper", "creeper"),
	("donkey", "donkey"),
	("elder_guardian", "elder_guardian"),
	("enderman", "enderman"),
	("endermite", "endermite"),
	("evocation_illager", "evoker"),
	("ghast", "ghast"),
	("guardian", "guardian"),
	("horse", "horse"),
	("husk", "husk"),
	("llama", "llama"),
	("magma_cube", "magma_cube"),
	("mooshroom", "mooshroom"),
	("mule", "mule"),
	("ocelot", "ocelot"),
	("parrot", "parrot"),
	("pig", "pig"),
	("polar_bear", "polar_bear"),
	("rabbit", "rabbit"),
	("sheep", "sheep"),
	("shulker", "shulker"),
	("silverfish", "silverfish"),
	("skeleton", "skeleton"),
	("skeleton_horse", "skeleton_horse"),
	("slime", "slime"),
	("spider", "spider"),
	("squid", "squid"),
	("stray", "stray"),
	("vex", "vex"),
	("villager", "villager"),
	("vindication_illager", "vindicator"),
	("witch", "witch"),
	("wither_skeleton", "wither_skeleton"),
	("wolf", "wolf"),
	("zombie", "zombie"),
	("zombie_horse", "zombie_horse"),
	("zombie_pigman", "zombie_pigman"),
	("zombie_villager", "zombie_villager"),
];

fn strip(s: &str) -> &str {
	s.strip_prefix("minecraft:").unwrap_or(s)
}

pub(super) fn legacy_item_name(id: u16, meta: u8) -> Option<&'static str> {
	registry::legacy_item(id, meta)
}

fn legacy_item(id: i16, damage: i16) -> Option<u32> {
	let meta = u8::try_from(damage).ok()?;
	legacy_item_name(u16::try_from(id).ok()?, meta).and_then(|n| new_items().id(n))
}

fn item_named(name: &str) -> Option<u32> {
	new_items().id(name)
}

pub(super) fn legacy_of_item(id: u32) -> Option<(i16, i16)> {
	static TABLE: OnceLock<HashMap<u32, (i16, i16)>> = OnceLock::new();
	TABLE
		.get_or_init(|| {
			new_items()
				.names
				.iter()
				.filter_map(|(id, name)| {
					let (old, meta) = registry::legacy_item_of(&format!("minecraft:{name}"))?;
					Some((*id, (old as i16, meta as i16)))
				})
				.collect()
		})
		.get(&id)
		.copied()
}

fn substitute_of_item(id: u32) -> Option<(i16, i16)> {
	let name = new_items().name(id)?;
	legacy_of_item(new_items().id(super::fallback::item(name)?)?)
}

fn pretty(name: &str) -> String {
	strip(name)
		.split('_')
		.map(|w| {
			let mut c = w.chars();
			c.next().map_or_else(String::new, |f| {
				f.to_ascii_uppercase().to_string() + c.as_str()
			})
		})
		.collect::<Vec<_>>()
		.join(" ")
}

fn spawn_egg(tag: &mut Vec<Entry>, damage: i16) -> u32 {
	let bat = || item_named("bat_spawn_egg").unwrap_or(0);
	let from_tag = tag.compound_mut("EntityTag").and_then(|e| {
		e.field("id")
			.and_then(Payload::as_str)
			.map(|s| strip(s).to_owned())
	});
	let entity = from_tag
		.clone()
		.or_else(|| super::entities::legacy_mob_name(damage as i32).map(str::to_owned));
	let Some(egg) = entity
		.and_then(|e| SPAWN_EGGS.iter().find(|(old, _)| *old == e))
		.and_then(|(_, egg)| item_named(&format!("{egg}_spawn_egg")))
	else {
		return bat();
	};
	if from_tag.is_some() {
		if let Some(e) = tag.compound_mut("EntityTag") {
			e.take("id");
			if e.is_empty() {
				tag.take("EntityTag");
			}
		}
	}
	egg
}

fn roman(n: i32) -> String {
	match n {
		1..=10 => {
			["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"][n as usize - 1].to_owned()
		}
		n => n.to_string(),
	}
}

fn enchantments_up(list: &mut Payload) {
	let Some(entries) = list.compounds_mut() else {
		return;
	};
	let names = registry::names(OLD, "enchantments");
	for e in entries.iter_mut() {
		let id = e.field("id").and_then(Payload::to_i32).unwrap_or(0);
		let name = names.and_then(|n| n.name(id as u32)).map_or_else(
			|| format!("{LEGACY_ENCHANTMENT}{id}"),
			|n| format!("minecraft:{n}"),
		);
		e.put("id", Payload::String(name));
		let lvl = e.field("lvl").and_then(Payload::to_i32).unwrap_or(0);
		e.put("lvl", Payload::Short(lvl as i16));
	}
}

fn enchantments_down(list: &mut Payload) -> Vec<Vec<Entry>> {
	let Some(entries) = list.compounds_mut() else {
		return Vec::new();
	};
	let names = registry::names(OLD, "enchantments");
	let mut unknown = Vec::new();
	entries.retain_mut(|e| {
		let id = e
			.field("id")
			.and_then(Payload::as_str)
			.map(str::to_owned)
			.unwrap_or_default();
		let old = match id.strip_prefix(LEGACY_ENCHANTMENT) {
			Some(n) => n.parse().ok(),
			None => names.and_then(|n| n.id(strip(&id))),
		};
		let Some(old) = old else {
			unknown.push(e.clone());
			return false;
		};
		e.put("id", Payload::Short(old as i16));
		let lvl = e.field("lvl").and_then(Payload::to_i32).unwrap_or(0);
		e.put("lvl", Payload::Short(lvl as i16));
		true
	});
	unknown
}

fn predicates_up(tag: &mut Vec<Entry>, key: &str, backup: &str) {
	let Some(list) = tag.field(key).cloned() else {
		return;
	};
	if let Some(original) = tag.take(backup) {
		tag.put(key, original);
		return;
	}
	tag.put(backup, list.clone());
	let mut out: Vec<String> = Vec::new();
	for old in nbtx::strings(&list) {
		let name = strip(&old).to_ascii_lowercase();
		let id = name
			.parse::<u32>()
			.ok()
			.or_else(|| old_blocks().block(&name).map(|b| b.id));
		let Some(id) = id else {
			out.push(name);
			continue;
		};
		let blocks = new_blocks();
		for meta in 0..16 {
			if let Some(block) = blocks.owner(state_of_raw(id << 4 | meta)) {
				let n = format!("minecraft:{}", block.name);
				if !out.contains(&n) {
					out.push(n);
				}
			}
		}
	}
	tag.put(key, ListItems::String(out));
}

fn predicates_down(tag: &mut Vec<Entry>, key: &str, backup: &str) {
	let Some(list) = tag.field(key).cloned() else {
		return;
	};
	if let Some(original) = tag.take(backup) {
		tag.put(key, original);
		return;
	}
	tag.put(backup, list.clone());
	let mut out: Vec<String> = Vec::new();
	for name in nbtx::strings(&list) {
		let old = new_blocks()
			.block(&name)
			.map(|b| raw_of_state(b.default) >> 4)
			.and_then(|id| old_blocks().list.iter().find(|b| b.id == id))
			.map_or_else(|| name.clone(), |b| format!("minecraft:{}", b.name));
		if !out.contains(&old) {
			out.push(old);
		}
	}
	tag.put(key, ListItems::String(out));
}

fn name_up(tag: &mut Vec<Entry>) {
	if tag.take(NAME_ADDED).is_some() {
		if let Some(display) = tag.compound_mut("display") {
			display.take("Name");
			if display.is_empty() {
				tag.take("display");
			}
		}
		return;
	}
	if let Some(display) = tag.compound_mut("display") {
		let backup = display.take(NAME);
		if let Some(name) = display
			.field("Name")
			.and_then(Payload::as_str)
			.map(str::to_owned)
		{
			let json = backup
				.as_ref()
				.and_then(Payload::as_str)
				.map_or_else(|| legacy_to_json(&name, true), str::to_owned);
			if backup.is_none() {
				display.put(NAME, Payload::String(name));
			}
			display.put("Name", Payload::String(json));
		}
	}
}

fn name_down(tag: &mut Vec<Entry>) {
	if let Some(display) = tag.compound_mut("display") {
		let backup = display.take(NAME);
		if let Some(name) = display
			.field("Name")
			.and_then(Payload::as_str)
			.map(str::to_owned)
		{
			let legacy = backup
				.as_ref()
				.and_then(Payload::as_str)
				.map_or_else(|| json_to_legacy(&name), str::to_owned);
			if backup.is_none() {
				display.put(NAME, Payload::String(name));
			}
			display.put("Name", Payload::String(legacy));
		}
	}
}

fn lore_down(tag: &mut Vec<Entry>, lines: Vec<String>) {
	if lines.is_empty() {
		return;
	}
	let display = match tag.compound_mut("display") {
		Some(d) => d,
		None => {
			tag.put("display", Payload::Compound(Vec::new()));
			tag.compound_mut("display").expect("display was just added")
		}
	};
	let old = display.field("Lore").cloned();
	let mut lore = lines;
	lore.extend(old.as_ref().map(nbtx::strings).unwrap_or_default());
	display.put("Lore", ListItems::String(lore));
	match old {
		Some(old) => tag.put(LORE, old),
		None => tag.put(LORE_ADDED, Payload::Byte(1)),
	}
}

fn lore_up(tag: &mut Vec<Entry>) {
	let old = tag.take(LORE);
	let added = tag.take(LORE_ADDED).is_some();
	if old.is_none() && !added {
		return;
	}
	let Some(display) = tag.compound_mut("display") else {
		return;
	};
	match old {
		Some(old) => display.put("Lore", old),
		None => {
			display.take("Lore");
		}
	}
	if display.is_empty() {
		tag.take("display");
	}
}

fn unknown_back(tag: &mut Vec<Entry>, key: &str, side: &str) {
	let Some(Payload::List(List {
		items: ListItems::Compound(extra),
		..
	})) = tag.take(side)
	else {
		return;
	};
	match tag.compounds_mut(key) {
		Some(list) => list.extend(extra),
		None => tag.put(key, ListItems::Compound(extra)),
	}
}

pub(super) fn item_up(id: i16, damage: i16, nbt: Option<Entry>) -> (i16, Option<Entry>) {
	if id == 0 {
		return (0, nbt);
	}
	let mut tag = children(nbt);
	let flat = tag
		.take(FLAT)
		.as_ref()
		.and_then(Payload::as_str)
		.and_then(item_named);
	let original = (id as i32) << 16 | damage as u16 as i32;
	if damageable(id) {
		tag.put("Damage", Payload::Int(damage as i32));
	}
	if id == 358 {
		tag.put("map", Payload::Int(damage as i32));
	}
	let mut key = (id, damage);
	if id == 425 || id == 442 {
		if let Some(entity) = tag.compound_mut("BlockEntityTag") {
			if let (425, Some(base)) = (id, entity.field("Base").and_then(Payload::to_i32)) {
				key = (425, base as i16);
			}
			nbtx::invert_colors(entity);
		}
	}
	name_up(&mut tag);
	lore_up(&mut tag);
	if let Some(mut ench) = tag.take("ench") {
		enchantments_up(&mut ench);
		tag.put("Enchantments", ench);
	}
	unknown_back(&mut tag, "Enchantments", ENCHANTS);
	if let Some(mut stored) = tag.field_mut("StoredEnchantments") {
		enchantments_up(&mut stored);
	}
	unknown_back(&mut tag, "StoredEnchantments", STORED);
	predicates_up(&mut tag, "CanPlaceOn", PLACE);
	predicates_up(&mut tag, "CanDestroy", DESTROY);
	let mapped = match (flat, id) {
		(Some(flat), _) => Some(flat),
		(None, 383) => Some(spawn_egg(&mut tag, damage)),
		(None, _) => legacy_item(key.0, key.1),
	};
	let new = match mapped {
		Some(n) => n,
		None => {
			if !damageable(id) && id != 358 {
				tag.put(ORIGINAL, Payload::Int(original));
			}
			match (id, damage) {
				(31, 0) => legacy_item(32, 0),
				_ => legacy_item(id, 0),
			}
			.or_else(|| item_named("stone"))
			.unwrap_or(1)
		}
	};
	(new as i16, root(tag))
}

pub(super) fn item_down(id: i16, nbt: Option<Entry>) -> (i16, i16, Option<Entry>) {
	if id == 0 {
		return (0, 0, nbt);
	}
	let mut tag = children(nbt);
	let stored = tag.take(ORIGINAL).as_ref().and_then(Payload::to_i32);
	let (old, mut damage) = match stored {
		Some(raw) => ((raw >> 16) as i16, raw as u16 as i16),
		None => {
			let name = new_items().name(id.max(0) as u32).unwrap_or("stone");
			match name.strip_suffix("_spawn_egg") {
				Some(egg) => {
					let entity = SPAWN_EGGS
						.iter()
						.find(|(_, e)| *e == egg)
						.map_or(egg, |(old, _)| *old);
					let mut entity_tag = match tag.take("EntityTag") {
						Some(Payload::Compound(c)) => c,
						_ => Vec::new(),
					};
					if entity_tag.field("id").is_none() {
						entity_tag.put("id", Payload::String(format!("minecraft:{entity}")));
					}
					tag.put("EntityTag", Payload::Compound(entity_tag));
					(383, 0)
				}
				None => match legacy_of_item(id.max(0) as u32) {
					Some(found) => found,
					None => {
						tag.put(FLAT, Payload::String(format!("minecraft:{name}")));
						if tag
							.compound_mut("display")
							.and_then(|d| d.field("Name").cloned())
							.is_none()
						{
							let display =
								format!("{{\"text\":\"1.13 {}\",\"italic\":false}}", pretty(name));
							match tag.compound_mut("display") {
								Some(d) => d.put("Name", Payload::String(display.clone())),
								None => tag.put(
									"display",
									Payload::Compound(vec![Entry::new(
										"Name",
										Payload::String(display.clone()),
									)]),
								),
							}
							tag.put(NAME_ADDED, Payload::Byte(1));
						}
						substitute_of_item(id.max(0) as u32).unwrap_or((1, 0))
					}
				},
			}
		}
	};
	if damageable(old) {
		if let Some(d) = tag.take("Damage").as_ref().and_then(Payload::to_i32) {
			if stored.is_none() {
				damage = d as i16;
			}
		}
	}
	if old == 358 {
		if let Some(m) = tag.take("map").as_ref().and_then(Payload::to_i32) {
			if stored.is_none() {
				damage = (m & 0x7fff) as i16;
			}
			if m > i16::MAX as i32 {
				tag.put("map", Payload::Int(m));
			}
		}
	}
	if old == 425 || old == 442 {
		if let Some(entity) = tag.compound_mut("BlockEntityTag") {
			nbtx::invert_colors(entity);
		}
	}
	name_down(&mut tag);
	let mut lore = Vec::new();
	if let Some(mut ench) = tag.take("Enchantments") {
		let unknown = enchantments_down(&mut ench);
		for e in &unknown {
			let id = e.field("id").and_then(Payload::as_str).unwrap_or_default();
			lore.push(format!(
				"\u{a7}7{} {}",
				pretty(id),
				roman(e.field("lvl").and_then(Payload::to_i32).unwrap_or(1))
			));
		}
		if !unknown.is_empty() {
			tag.put(ENCHANTS, ListItems::Compound(unknown));
		}
		tag.put("ench", ench);
	}
	let unknown = tag
		.field_mut("StoredEnchantments")
		.map(|mut s| enchantments_down(&mut s))
		.unwrap_or_default();
	if !unknown.is_empty() {
		tag.put(STORED, ListItems::Compound(unknown));
	}
	lore_down(&mut tag, lore);
	predicates_down(&mut tag, "CanPlaceOn", PLACE);
	predicates_down(&mut tag, "CanDestroy", DESTROY);
	(old, damage, root(tag))
}

pub(super) fn slot_up(v: a::slot::Ty) -> b::slot::Ty {
	match v {
		a::slot::Ty::Default(id, s) => {
			let (id, nbt_data) = item_up(id, s.item_damage, s.nbt_data);
			b::slot::Ty::Default(
				id,
				b::slot_default::Ty {
					item_count: s.item_count,
					nbt_data,
				},
			)
		}
		_ => b::slot::Ty::_1,
	}
}

pub(super) fn slot_down(v: b::slot::Ty) -> a::slot::Ty {
	match v {
		b::slot::Ty::Default(id, s) => {
			let (id, item_damage, nbt_data) = item_down(id, s.nbt_data);
			a::slot::Ty::Default(
				id,
				a::slot_default::Ty {
					item_count: s.item_count,
					item_damage,
					nbt_data,
				},
			)
		}
		_ => a::slot::Ty::_1,
	}
}

pub fn flattening_slot_1_13(v: a::slot::Ty, _cx: &mut Session) -> Result<b::slot::Ty> {
	Ok(slot_up(v))
}

pub fn flattening_slot_1_12_2(v: b::slot::Ty, _cx: &mut Session) -> Result<a::slot::Ty> {
	Ok(slot_down(v))
}

fn later_items() -> &'static registry::Names {
	registry::names(super::blocks::LATER, "items").expect("1.13.1 items")
}

pub(super) fn slot_later(v: b::slot::Ty) -> c::slot::Ty {
	let b::slot::Ty::Default(id, s) = v else {
		return c::slot::Ty::_1;
	};
	let mut tag = children(s.nbt_data);
	let restored = tag
		.take(LATER_FLAT)
		.as_ref()
		.and_then(Payload::as_str)
		.and_then(|n| later_items().id(n));
	match restored.map(|i| i as i32).or_else(|| item_later(id as i32)) {
		Some(id) => c::slot::Ty::Default(
			id as i16,
			c::slot_default::Ty {
				item_count: s.item_count,
				nbt_data: root(tag),
			},
		),
		None => c::slot::Ty::_1,
	}
}

pub(super) fn slot_earlier(v: c::slot::Ty) -> b::slot::Ty {
	let c::slot::Ty::Default(id, s) = v else {
		return b::slot::Ty::_1;
	};
	let name = later_items().name(id.max(0) as u32).unwrap_or("stone");
	if let Some(same) = item_named(name) {
		return b::slot::Ty::Default(same as i16, s);
	}
	let substitute = item_earlier(id as i32)
		.map(|i| i as u32)
		.or_else(|| item_named("stone"))
		.unwrap_or(1);
	let mut tag = children(s.nbt_data);
	tag.put(LATER_FLAT, Payload::String(format!("minecraft:{name}")));
	b::slot::Ty::Default(
		substitute as i16,
		b::slot_default::Ty {
			item_count: s.item_count,
			nbt_data: root(tag),
		},
	)
}

pub fn flattening_item_ids_1_13_1(v: b::slot::Ty, _cx: &mut Session) -> Result<c::slot::Ty> {
	Ok(slot_later(v))
}

pub fn flattening_item_ids_1_13(v: c::slot::Ty, _cx: &mut Session) -> Result<b::slot::Ty> {
	Ok(slot_earlier(v))
}

fn present(v: c::slot::Ty) -> d::slot::Ty {
	match v {
		c::slot::Ty::Default(id, s) if id >= 0 => d::slot::Ty::True(d::slot_true::Ty {
			item_id: id as i32,
			item_count: s.item_count,
			nbt_data: s.nbt_data,
		}),
		_ => d::slot::Ty::False,
	}
}

fn absent(v: d::slot::Ty) -> c::slot::Ty {
	match v {
		d::slot::Ty::True(s) => c::slot::Ty::Default(
			s.item_id as i16,
			c::slot_default::Ty {
				item_count: s.item_count,
				nbt_data: s.nbt_data,
			},
		),
		d::slot::Ty::False => c::slot::Ty::_1,
	}
}

pub fn flattening_slot_1_13_2(v: c::slot::Ty, _cx: &mut Session) -> Result<d::slot::Ty> {
	Ok(present(v))
}

pub fn flattening_slot_1_13_1(v: d::slot::Ty, _cx: &mut Session) -> Result<c::slot::Ty> {
	Ok(absent(v))
}

pub(super) fn trader_list(
	data: &[u8],
	slot: &dyn Fn(&mut Reader, &mut Vec<u8>) -> Result<()>,
) -> Result<Vec<u8>> {
	let mut r = Reader::new(data);
	let mut out = Vec::with_capacity(data.len());
	out.extend(r.i32()?.to_be_bytes());
	let count = r.u8()?;
	out.push(count);
	for _ in 0..count {
		slot(&mut r, &mut out)?;
		slot(&mut r, &mut out)?;
		let second = r.u8()?;
		out.push(second);
		if second != 0 {
			slot(&mut r, &mut out)?;
		}
		out.push(r.u8()?);
		out.extend(r.i32()?.to_be_bytes());
		out.extend(r.i32()?.to_be_bytes());
	}
	out.extend(r.rest()?);
	Ok(out)
}

pub(super) fn trader_up(data: &[u8]) -> Result<Vec<u8>> {
	trader_list(data, &|r, out| {
		put_value(
			out,
			&slot_up(r.with(a::slot::deserialize)?),
			b::slot::serialize,
		)
	})
}

pub(super) fn trader_down(data: &[u8]) -> Result<Vec<u8>> {
	trader_list(data, &|r, out| {
		put_value(
			out,
			&slot_down(r.with(b::slot::deserialize)?),
			a::slot::serialize,
		)
	})
}

type Plugin = b::play_to_client_packet_custom_payload::Ty;

fn trader(mut p: Plugin, f: &dyn Fn(&mut Reader, &mut Vec<u8>) -> Result<()>) -> Result<Plugin> {
	if p.channel == "minecraft:trader_list" {
		p.data = trader_list(&p.data, f)?;
	}
	Ok(p)
}

pub fn flattening_trader_ids_1_13_1(
	p: Plugin,
	_cx: &mut Session,
) -> Result<c::play_to_client_packet_custom_payload::Ty> {
	trader(p, &|r, out| {
		put_value(
			out,
			&slot_later(r.with(b::slot::deserialize)?),
			c::slot::serialize,
		)
	})
}

pub fn flattening_trader_ids_1_13(
	p: c::play_to_client_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<Plugin> {
	trader(p, &|r, out| {
		put_value(
			out,
			&slot_earlier(r.with(c::slot::deserialize)?),
			b::slot::serialize,
		)
	})
}

pub fn flattening_trader_slots_1_13_2(
	p: c::play_to_client_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<d::play_to_client_packet_custom_payload::Ty> {
	trader(p, &|r, out| {
		put_value(
			out,
			&present(r.with(c::slot::deserialize)?),
			d::slot::serialize,
		)
	})
}

pub fn flattening_trader_slots_1_13_1(
	p: d::play_to_client_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<c::play_to_client_packet_custom_payload::Ty> {
	trader(p, &|r, out| {
		put_value(
			out,
			&absent(r.with(d::slot::deserialize)?),
			c::slot::serialize,
		)
	})
}
