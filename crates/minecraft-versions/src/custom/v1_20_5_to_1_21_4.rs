use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, OnceLock};

use nbt::{CompoundExt, CompoundVecExt, Entry, List, ListItems, Payload, Tag};
use serde_json::Value;

use super::chat_types::{ChatType, Decoration, Parameter};
use super::localize::localize;
use crate::gen::convert::{v1_21_1, Root, Version};
use crate::pds::convert::{Error, Result};
use crate::pds::{Input, Output};
use crate::registry;
use crate::Session;

mod chunk;
mod step_1_21_1;
mod step_1_21_3;
mod step_1_21_4;

pub use step_1_21_1::*;
pub use step_1_21_3::*;
pub use step_1_21_4::*;

type RegistryData = v1_21_1::configuration_to_client_packet_registry_data::Ty;
type RegistryEntry = v1_21_1::configuration_to_client_packet_registry_data_entries_items::Ty;
type TagList = v1_21_1::configuration_to_client_packet_tags_items_tags_items::Ty;
type SoundHolder = v1_21_1::item_sound_holder::Ty;
type IdSet = v1_21_1::id_set::Ty;
type Fix = fn(&str, &mut Tag);

fn remap(kind: &str, from: Version, to: Version, id: i32) -> Option<i32> {
	registry::remap_name(kind, from, to, u32::try_from(id).ok()?).map(|v| v as i32)
}

fn item(from: Version, to: Version, id: i32) -> i32 {
	remap("items", from, to, id).unwrap_or(0)
}

fn state(from: Version, to: Version, id: i32) -> i32 {
	u32::try_from(id).map_or(0, |s| registry::remap_state(from, to, s) as i32)
}

fn block(from: Version, to: Version, id: i32) -> Option<i32> {
	remap("blocks", from, to, id)
}

fn item_name(version: Version, id: i32) -> Option<String> {
	let name = registry::names(version, "items")?.name(u32::try_from(id).ok()?)?;
	Some(format!("minecraft:{name}"))
}

fn item_id(version: Version, name: &str) -> Option<i32> {
	registry::names(version, "items")?
		.id(name)
		.map(|v| v as i32)
}

fn sound_base(version: Version) -> u32 {
	registry::names(version, "sounds").map_or(0, |n| u32::from(n.name(0).is_none()))
}

fn sound_name(version: Version, id: i32) -> Option<&'static str> {
	registry::names(version, "sounds")?.name(u32::try_from(id).ok()? + sound_base(version))
}

fn sound_id(version: Version, name: &str) -> Option<i32> {
	registry::names(version, "sounds")?
		.id(name)
		.map(|v| (v - sound_base(version)) as i32)
}

fn sound(from: Version, to: Version, holder: SoundHolder) -> SoundHolder {
	let SoundHolder::Id(id) = holder else {
		return holder;
	};
	match sound_name(from, id) {
		Some(name) => match sound_id(to, name) {
			Some(id) => SoundHolder::Id(id),
			None => SoundHolder::Inline(v1_21_1::item_sound_event::Ty {
				sound_name: format!("minecraft:{name}"),
				fixed_range: None,
			}),
		},
		None => SoundHolder::Id(id),
	}
}

fn id_set(set: IdSet, map: impl Fn(i32) -> Option<i32>) -> IdSet {
	match set {
		IdSet::Ids(ids) => IdSet::Ids(ids.into_iter().filter_map(map).collect()),
		named => named,
	}
}

fn tag_kind(registry: &str) -> Option<&'static str> {
	Some(match strip(registry) {
		"item" => "items",
		"block" => "blocks",
		"entity_type" => "entities",
		_ => return None,
	})
}

fn remap_tags(from: Version, to: Version, registry: &str, tags: &mut [TagList]) {
	let Some(kind) = tag_kind(registry) else {
		return;
	};
	for tag in tags {
		tag.entries = std::mem::take(&mut tag.entries)
			.into_iter()
			.filter_map(|id| remap(kind, from, to, id))
			.collect();
	}
}

fn ensure_tags(tags: &mut Vec<TagList>, wanted: &[(&str, Vec<i32>)]) {
	for (name, entries) in wanted {
		if !tags.iter().any(|t| strip(&t.tag_name) == *name) {
			tags.push(TagList {
				tag_name: format!("minecraft:{name}"),
				entries: entries.clone(),
			});
		}
	}
}

#[derive(Default)]
struct ItemTags(HashMap<Version, HashMap<String, Vec<i32>>>);

fn store_item_tags(cx: &mut Session, version: Version, registry: &str, tags: &[TagList]) {
	if strip(registry) != "item" {
		return;
	}
	let stored = cx.state::<ItemTags>().0.entry(version).or_default();
	for t in tags {
		stored.insert(strip(&t.tag_name).to_owned(), t.entries.clone());
	}
}

fn item_tag(cx: &mut Session, version: Version, name: &str) -> Vec<i32> {
	cx.state::<ItemTags>()
		.0
		.get(&version)
		.and_then(|t| t.get(strip(name.trim_start_matches('#'))))
		.cloned()
		.unwrap_or_default()
}

fn features(mut flags: Vec<String>, add: &[&str], remove: &[&str]) -> Vec<String> {
	flags.retain(|f| !remove.contains(&strip(f)));
	for f in add {
		if !flags.iter().any(|x| strip(x) == *f) {
			flags.push(format!("minecraft:{f}"));
		}
	}
	flags
}

fn write_varint(out: &mut Vec<u8>, v: i32) {
	let mut v = v as u32;
	loop {
		if v & !0x7f == 0 {
			out.push(v as u8);
			return;
		}
		out.push((v as u8 & 0x7f) | 0x80);
		v >>= 7;
	}
}

fn read_varint(data: &[u8], at: &mut usize) -> Option<i32> {
	let mut v = 0u32;
	for shift in (0..35).step_by(7) {
		let b = *data.get(*at)?;
		*at += 1;
		v |= ((b & 0x7f) as u32) << shift;
		if b & 0x80 == 0 {
			return Some(v as i32);
		}
	}
	None
}

struct Vanilla {
	id: String,
	entries: Vec<(String, Tag)>,
}

static VANILLA: LazyLock<Vec<OnceLock<Vec<Vanilla>>>> =
	LazyLock::new(|| Version::ALL.iter().map(|_| OnceLock::new()).collect());

fn vanilla(version: Version) -> &'static [Vanilla] {
	VANILLA[version.index()].get_or_init(|| {
		let parsed =
			registry::login_packet(version).and_then(|t| serde_json::from_str::<Value>(t).ok());
		let codec = parsed
			.as_ref()
			.and_then(|j| j.get("dimensionCodec")?.as_object().cloned())
			.unwrap_or_default();
		codec
			.iter()
			.filter_map(|(id, reg)| {
				let entries = reg
					.get("entries")?
					.as_array()?
					.iter()
					.filter_map(|e| {
						let value = nbt::json::typed::read(e.get("value")?)?;
						Some((e.get("key")?.as_str()?.to_owned(), Tag::new(value)))
					})
					.collect();
				Some(Vanilla {
					id: id.clone(),
					entries,
				})
			})
			.collect()
	})
}

fn strip(name: &str) -> &str {
	name.strip_prefix("minecraft:").unwrap_or(name)
}

const VANILLA_ONLY: &[&str] = &[
	"painting_variant",
	"enchantment",
	"jukebox_song",
	"instrument",
];

#[derive(Default)]
struct Registries {
	seen: HashMap<Version, HashSet<String>>,
	keys: HashMap<(Version, String), Vec<(String, Option<Tag>)>>,
}

fn remember(cx: &mut Session, version: Version, p: &RegistryData) {
	let entries = p
		.entries
		.iter()
		.map(|e| (strip(&e.key).to_owned(), e.value.clone()))
		.collect();
	cx.state::<Registries>()
		.keys
		.insert((version, strip(&p.id).to_owned()), entries);
}

fn registry_key(cx: &mut Session, version: Version, registry: &str, id: i32) -> Option<String> {
	let index = usize::try_from(id).ok()?;
	if let Some(keys) = cx
		.state::<Registries>()
		.keys
		.get(&(version, registry.to_owned()))
	{
		return keys.get(index).map(|(k, _)| k.clone());
	}
	let reg = vanilla(version).iter().find(|r| strip(&r.id) == registry)?;
	reg.entries.get(index).map(|(k, _)| strip(k).to_owned())
}

fn registry_entry(cx: &mut Session, version: Version, registry: &str, id: i32) -> Option<Tag> {
	let index = usize::try_from(id).ok()?;
	let key = match cx
		.state::<Registries>()
		.keys
		.get(&(version, registry.to_owned()))
	{
		Some(keys) => match keys.get(index)? {
			(_, Some(value)) => return Some(value.clone()),
			(key, None) => Some(key.clone()),
		},
		None => None,
	};
	let reg = vanilla(version).iter().find(|r| strip(&r.id) == registry)?;
	match key {
		Some(key) => reg.entries.iter().find(|(k, _)| strip(k) == key),
		None => reg.entries.get(index),
	}
	.map(|(_, v)| v.clone())
}

fn registry_index(cx: &mut Session, version: Version, registry: &str, key: &str) -> Option<i32> {
	if let Some(keys) = cx
		.state::<Registries>()
		.keys
		.get(&(version, registry.to_owned()))
	{
		return keys
			.iter()
			.position(|(k, _)| k == strip(key))
			.map(|i| i as i32);
	}
	let reg = vanilla(version).iter().find(|r| strip(&r.id) == registry)?;
	reg.entries
		.iter()
		.position(|(k, _)| strip(k) == strip(key))
		.map(|i| i as i32)
}

fn registry_remap(
	cx: &mut Session,
	(from, to): (Version, Version),
	registry: &str,
	id: i32,
) -> Option<i32> {
	let key = registry_key(cx, from, registry, id)?;
	registry_index(cx, to, registry, &key)
}

fn registry_up(
	cx: &mut Session,
	(from, to): (Version, Version),
	mut p: RegistryData,
	fix: Fix,
) -> RegistryData {
	remember(cx, from, &p);
	cx.state::<Registries>()
		.seen
		.entry(to)
		.or_default()
		.insert(strip(&p.id).to_owned());
	if let Some(reg) = vanilla(to).iter().find(|r| strip(&r.id) == strip(&p.id)) {
		if VANILLA_ONLY.contains(&strip(&p.id)) {
			for e in &mut p.entries {
				if let Some((_, tag)) = reg.entries.iter().find(|(k, _)| strip(k) == strip(&e.key))
				{
					e.value = Some(tag.clone());
				}
			}
		}
		for (key, tag) in &reg.entries {
			if !p.entries.iter().any(|e| strip(&e.key) == strip(key)) {
				p.entries.push(RegistryEntry {
					key: key.clone(),
					value: Some(tag.clone()),
				});
			}
		}
	}
	for e in &mut p.entries {
		if let Some(tag) = &mut e.value {
			fix(&p.id, tag);
		}
	}
	remember(cx, to, &p);
	p
}

fn missing_registries(cx: &mut Session, version: Version, fix: Fix) -> Vec<RegistryData> {
	let seen = cx
		.state::<Registries>()
		.seen
		.remove(&version)
		.unwrap_or_default();
	let out: Vec<RegistryData> = vanilla(version)
		.iter()
		.filter(|r| !seen.contains(strip(&r.id)))
		.map(|r| RegistryData {
			id: r.id.clone(),
			entries: r
				.entries
				.iter()
				.map(|(key, tag)| {
					let mut tag = tag.clone();
					fix(&r.id, &mut tag);
					RegistryEntry {
						key: key.clone(),
						value: Some(tag),
					}
				})
				.collect(),
		})
		.collect();
	for p in &out {
		remember(cx, version, p);
	}
	out
}

fn no_fix(_: &str, _: &mut Tag) {}

fn registry_down(
	cx: &mut Session,
	(from, to): (Version, Version),
	mut p: RegistryData,
	fix: Fix,
) -> Result<RegistryData> {
	remember(cx, from, &p);
	if !vanilla(to).iter().any(|r| strip(&r.id) == strip(&p.id)) {
		return Err(Error::Dropped);
	}
	for e in &mut p.entries {
		if let Some(tag) = &mut e.value {
			fix(&p.id, tag);
		}
	}
	remember(cx, to, &p);
	Ok(p)
}

fn effects<'a>(registry: &str, tag: &'a mut Tag) -> Option<&'a mut Vec<Entry>> {
	if strip(registry) != "worldgen/biome" {
		return None;
	}
	tag.value.as_compound_mut()?.compound_mut("effects")
}

fn wrap_music(registry: &str, tag: &mut Tag) {
	let Some(m) = effects(registry, tag).and_then(|e| {
		e.iter_mut()
			.find(|e| e.name == "music" && matches!(e.value, Payload::Compound(_)))
	}) else {
		return;
	};
	let data = std::mem::replace(&mut m.value, Payload::End);
	m.set(ListItems::Compound(vec![vec![
		Entry::new("data", data),
		Entry::new("weight", Payload::Int(1)),
	]]));
}

fn unwrap_music(registry: &str, tag: &mut Tag) {
	let Some(effects) = effects(registry, tag) else {
		return;
	};
	let Some(at) = effects
		.iter()
		.position(|e| e.name == "music" && matches!(e.value, Payload::List(_)))
	else {
		return;
	};
	let first = effects[at]
		.value
		.compounds_mut()
		.and_then(|list| list.first_mut())
		.and_then(|e| {
			e.iter_mut()
				.find(|f| f.name == "data" && matches!(f.value, Payload::Compound(_)))
		})
		.and_then(|f| f.value.as_compound_mut())
		.map(std::mem::take);
	match first {
		Some(value) => effects[at] = Entry::new("music", value),
		None => {
			effects.remove(at);
		}
	}
}

fn rename_strings(value: &mut Payload, from: &str, to: &str) {
	match value {
		Payload::String(s) if strip(s) == from => *s = format!("minecraft:{to}"),
		Payload::Compound(entries) => {
			for e in entries {
				rename_strings(&mut e.value, from, to);
			}
		}
		Payload::List(List {
			items: ListItems::Compound(list),
			..
		}) => {
			for c in list.iter_mut().flatten() {
				rename_strings(&mut c.value, from, to);
			}
		}
		Payload::List(List {
			items: ListItems::String(list),
			..
		}) => {
			for s in list.iter_mut().filter(|s| strip(s) == from) {
				*s = format!("minecraft:{to}");
			}
		}
		_ => {}
	}
}

fn encoding(e: impl std::fmt::Debug) -> Error {
	Error::Encoding(format!("{e:?}"))
}

fn carry<A, B, T>(
	root: Root,
	(from, to): (Version, Version),
	packet: &A,
	ser: fn(&A, &mut Output) -> crate::pds::Result<()>,
	de: fn(&mut Input) -> crate::pds::Result<B>,
	pick: impl Fn(B) -> Option<T>,
	cx: &mut Session,
) -> Result<T> {
	let mut out = Output::new();
	ser(packet, &mut out).map_err(encoding)?;
	for bytes in crate::gen::convert::step(root, from.index(), to.index(), &out.into_inner(), cx)? {
		if let Some(v) = pick(de(&mut Input::new(&bytes)).map_err(encoding)?) {
			return Ok(v);
		}
	}
	Err(Error::Dropped)
}

fn send<A>(
	cx: &mut Session,
	root: Root,
	version: Version,
	packet: &A,
	ser: fn(&A, &mut Output) -> crate::pds::Result<()>,
) -> Result<()> {
	let mut out = Output::new();
	ser(packet, &mut out).map_err(encoding)?;
	cx.send(root, version.index(), out.into_inner());
	Ok(())
}

fn entity_name(version: Version, id: i32) -> Option<&'static str> {
	registry::names(version, "entities")?.name(u32::try_from(id).ok()?)
}

#[derive(Default)]
struct Tracker {
	types: HashMap<(Version, i32), &'static str>,
	entity: HashMap<Version, i32>,
	key: HashMap<Version, u8>,
}

fn spawned(cx: &mut Session, version: Version, entity: i32, ty: Option<&'static str>) {
	let t = cx.state::<Tracker>();
	match ty {
		Some(ty) => t.types.insert((version, entity), ty),
		None => t.types.remove(&(version, entity)),
	};
}

fn tracked(cx: &mut Session, version: Version, entity: i32) -> Option<&'static str> {
	cx.state::<Tracker>().types.get(&(version, entity)).copied()
}

fn metadata_of(cx: &mut Session, version: Version, entity: i32) {
	cx.state::<Tracker>().entity.insert(version, entity);
}

fn metadata_key(cx: &mut Session, version: Version, key: u8) -> Option<&'static str> {
	let t = cx.state::<Tracker>();
	t.key.insert(version, key);
	let entity = *t.entity.get(&version)?;
	t.types.get(&(version, entity)).copied()
}

fn current_entry(cx: &mut Session, version: Version) -> (Option<&'static str>, u8) {
	let t = cx.state::<Tracker>();
	let ty = t
		.entity
		.get(&version)
		.and_then(|e| t.types.get(&(version, *e)).copied());
	(ty, t.key.get(&version).copied().unwrap_or(u8::MAX))
}

fn is_minecart(ty: Option<&str>) -> bool {
	ty.is_some_and(|t| t.ends_with("minecart"))
}

fn angle(v: i8) -> f32 {
	v as f32 * 360.0 / 256.0
}

fn to_angle(v: f32) -> i8 {
	(v * 256.0 / 360.0).floor() as i32 as i8
}

fn velocity(v: f64) -> i16 {
	(v.clamp(-3.9, 3.9) * 8000.0) as i16
}

#[derive(Default)]
struct LegacyCombat(bool);

pub fn tricky_enable_legacy_combat(cx: &mut Session) {
	cx.state::<LegacyCombat>().0 = true;
}

fn legacy_combat(cx: &mut Session) -> bool {
	cx.state::<LegacyCombat>().0
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn vanilla_registries_parse() {
		let regs = vanilla(Version::V1_21_1);
		let enchantments = regs.iter().find(|r| strip(&r.id) == "enchantment").unwrap();
		assert_eq!(enchantments.entries.len(), 42);
		assert!(regs.iter().any(|r| strip(&r.id) == "painting_variant"));
		let (_, tag) = &enchantments.entries[0];
		assert_eq!(tag.r#type, 10);
		let mut cx = Session::new();
		assert_eq!(
			registry_index(&mut cx, Version::V1_21_1, "jukebox_song", "minecraft:cat"),
			Some(4)
		);
	}

	#[test]
	fn sound_tables_are_zero_based() {
		for v in [
			Version::V1_20_5,
			Version::V1_21_1,
			Version::V1_21_3,
			Version::V1_21_4,
		] {
			assert_eq!(
				sound_name(v, 0),
				Some("entity.allay.ambient_with_item"),
				"{v:?}"
			);
			assert_eq!(
				sound_id(v, "entity.allay.ambient_with_item"),
				Some(0),
				"{v:?}"
			);
		}
		assert_eq!(sound_id(Version::V1_20_5, "entity.armadillo.eat"), Some(54));
	}
}
