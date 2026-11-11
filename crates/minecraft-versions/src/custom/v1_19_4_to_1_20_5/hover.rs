use super::common::translate;
use crate::gen::convert::Version;
use crate::pds::convert::Result;
use crate::Session;
use nbt::{snbt, CompoundExt, CompoundVecExt, Entry, List, ListItems, Payload, Tag};

const OLD: Version = Version::V1_20_3;
const NEW: Version = Version::V1_20_5;

pub fn trails_text_1_20_3_to_1_20_5(mut t: Tag, _cx: &mut Session) -> Result<Tag> {
	walk(&mut t.value, true);
	Ok(t)
}

pub fn trails_text_1_20_5_to_1_20_3(mut t: Tag, _cx: &mut Session) -> Result<Tag> {
	walk(&mut t.value, false);
	Ok(t)
}

pub fn walk(p: &mut Payload, up: bool) {
	match p {
		Payload::Compound(c) => {
			for e in c.iter_mut() {
				match e.name.as_str() {
					"hoverEvent" => hover(&mut e.value, up),
					"extra" | "with" | "separator" => walk(&mut e.value, up),
					_ => {}
				}
			}
		}
		Payload::List(l) if matches!(l.items, ListItems::List(_) | ListItems::Compound(_)) => {
			let mut values = l.values();
			values.iter_mut().for_each(|v| walk(v, up));
			*l = List::of(values);
		}
		_ => {}
	}
}

fn hover(p: &mut Payload, up: bool) {
	let Payload::Compound(h) = p else { return };
	match h.field("action").and_then(Payload::as_str) {
		Some("show_text") => {
			for key in ["contents", "value"] {
				if let Some(mut v) = h.field_mut(key) {
					walk(&mut v, up);
				}
			}
		}
		Some("show_item") => match h.field_mut("contents").as_deref_mut() {
			Some(Payload::Compound(item)) => match up {
				true => item_up(item),
				false => item_down(item),
			},
			Some(Payload::String(id)) => *id = rename(id, up),
			_ => {}
		},
		_ => {}
	}
}

fn rename(id: &str, up: bool) -> String {
	let name = id.strip_prefix("minecraft:").unwrap_or(id);
	let (from, to) = if up { (OLD, NEW) } else { (NEW, OLD) };
	format!("minecraft:{}", translate("items", from, to, name))
}

fn enchantment(id: &str, up: bool) -> String {
	match (id.strip_prefix("minecraft:").unwrap_or(id), up) {
		("sweeping", true) => "minecraft:sweeping_edge".to_owned(),
		("sweeping_edge", false) => "minecraft:sweeping".to_owned(),
		(k, _) => format!("minecraft:{k}"),
	}
}

fn item_up(item: &mut Vec<Entry>) {
	if let Some(Payload::String(id)) = item.field_mut("id").as_deref_mut() {
		*id = rename(id, true);
	}
	let Some(Payload::String(text)) = item.take("tag") else {
		return;
	};
	let Some(Payload::Compound(c)) = snbt::parse(&text) else {
		return;
	};
	let mut out = Vec::new();
	if let Some(d) = c.field("display").and_then(Payload::as_compound) {
		if let Some(n) = d.field("Name") {
			out.push(Entry::new("minecraft:custom_name", n.clone()));
		}
		if let Some(l @ Payload::List(_)) = d.field("Lore") {
			out.push(Entry::new("minecraft:lore", l.clone()));
		}
		if let Some(rgb) = d.field("color").and_then(Payload::to_i32) {
			out.push(Entry::new(
				"minecraft:dyed_color",
				Payload::Compound(vec![Entry::new("rgb", Payload::Int(rgb))]),
			));
		}
	}
	let levels: Vec<Entry> = c
		.field("Enchantments")
		.map(Payload::values)
		.unwrap_or_default()
		.iter()
		.filter_map(|e| {
			let e = e.as_compound()?;
			Some(Entry::new(
				&enchantment(e.field("id").and_then(Payload::as_str)?, true),
				Payload::Int(e.field("lvl").and_then(Payload::to_i32).unwrap_or(1)),
			))
		})
		.collect();
	if !levels.is_empty() {
		out.push(Entry::new(
			"minecraft:enchantments",
			Payload::Compound(vec![Entry::new("levels", Payload::Compound(levels))]),
		));
	}
	if let Some(d) = c
		.field("Damage")
		.and_then(Payload::to_i32)
		.filter(|d| *d > 0)
	{
		out.push(Entry::new("minecraft:damage", Payload::Int(d)));
	}
	if c.field("Unbreakable")
		.and_then(Payload::to_i32)
		.unwrap_or(0)
		!= 0
	{
		out.push(Entry::new(
			"minecraft:unbreakable",
			Payload::Compound(vec![]),
		));
	}
	if let Some(m) = c.field("CustomModelData").and_then(Payload::to_i32) {
		out.push(Entry::new("minecraft:custom_model_data", Payload::Int(m)));
	}
	out.push(Entry::new("minecraft:custom_data", Payload::Compound(c)));
	item.put("components", Payload::Compound(out));
}

fn legacy(c: &[Entry]) -> Vec<Entry> {
	let mut out = Vec::new();
	let mut display = Vec::new();
	if let Some(n) = c.field("minecraft:custom_name") {
		display.push(Entry::new("Name", n.clone()));
	}
	if let Some(l) = c.field("minecraft:lore") {
		display.push(Entry::new("Lore", l.clone()));
	}
	if let Some(d) = c.field("minecraft:dyed_color") {
		if let Some(rgb) = d
			.as_compound()
			.and_then(|d| d.field("rgb"))
			.and_then(Payload::to_i32)
			.or_else(|| d.to_i32())
		{
			display.push(Entry::new("color", Payload::Int(rgb)));
		}
	}
	if !display.is_empty() {
		out.push(Entry::new("display", Payload::Compound(display)));
	}
	if let Some(e) = c
		.field("minecraft:enchantments")
		.and_then(Payload::as_compound)
	{
		let levels = e
			.field("levels")
			.and_then(Payload::as_compound)
			.unwrap_or(e);
		let list_items = levels
			.iter()
			.filter_map(|l| {
				Some(Payload::Compound(vec![
					Entry::new("id", Payload::String(enchantment(&l.name, false))),
					Entry::new("lvl", Payload::Short(l.value.to_i32()? as i16)),
				]))
			})
			.collect();
		out.push(Entry::new("Enchantments", List::of(list_items)));
	}
	if let Some(d) = c.field("minecraft:damage").and_then(Payload::to_i32) {
		out.push(Entry::new("Damage", Payload::Int(d)));
	}
	if c.field("minecraft:unbreakable").is_some() {
		out.push(Entry::new("Unbreakable", Payload::Byte(1)));
	}
	if let Some(m) = c
		.field("minecraft:custom_model_data")
		.and_then(Payload::to_i32)
	{
		out.push(Entry::new("CustomModelData", Payload::Int(m)));
	}
	out
}

fn item_down(item: &mut Vec<Entry>) {
	if let Some(Payload::String(id)) = item.field_mut("id").as_deref_mut() {
		*id = rename(id, false);
	}
	let Some(Payload::Compound(mut components)) = item.take("components") else {
		return;
	};
	let mut tag = match components.take("minecraft:custom_data") {
		Some(Payload::Compound(c)) => c,
		_ => vec![],
	};
	for e in legacy(&components) {
		if tag.field(&e.name).is_none() {
			tag.push(e);
		}
	}
	if !tag.is_empty() {
		item.put("tag", Payload::String(snbt::write(&Payload::Compound(tag))));
	}
}
