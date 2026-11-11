use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::gen::convert::Version;
use crate::pds::convert::{unconvertible, Error, Result};
use crate::registry;
use crate::session::Session;

mod chunk;
mod meta;
mod step_1_16;
mod step_1_16_2;
mod step_1_17;
mod step_1_17_1;
mod substitutes;
mod tag_data;
mod tall;
mod text;

pub use step_1_16::*;
pub use step_1_16_2::*;
pub use step_1_17::*;
pub use step_1_17_1::*;

#[derive(Clone, Copy, Default)]
struct Kind {
	name: &'static str,
	original: &'static str,
	substitute: bool,
}

#[derive(Default)]
struct Entities {
	kinds: HashMap<i32, [Option<Kind>; 3]>,
}

fn step(v: Version) -> usize {
	match v {
		Version::V1_16 => 0,
		Version::V1_16_2 => 1,
		_ => 2,
	}
}

fn track(cx: &mut Session, at: Version, entity: i32, kind: Option<Kind>) {
	cx.state::<Entities>().kinds.entry(entity).or_default()[step(at)] = kind;
}

fn player(cx: &mut Session, at: Version, entity: i32) {
	track(
		cx,
		at,
		entity,
		Some(Kind {
			name: "player",
			original: "player",
			substitute: false,
		}),
	);
}

fn kind_of(cx: &mut Session, at: Version, entity: i32) -> Kind {
	cx.state::<Entities>()
		.kinds
		.get(&entity)
		.and_then(|k| k[step(at)])
		.unwrap_or_default()
}

fn untrack(cx: &mut Session, entities: &[i32]) {
	let kinds = &mut cx.state::<Entities>().kinds;
	for e in entities {
		kinds.remove(e);
	}
}

fn same_registry(kind: &str, from: Version, to: Version) -> bool {
	matches!((registry::folder(from, kind), registry::folder(to, kind)), (Some(a), Some(b)) if a == b)
}

fn renamed<'a>(kind: &str, to: Version, name: &'a str) -> &'a str {
	registry::renames::table(kind)
		.iter()
		.find(|(v, old, _)| *v == to && *old == name)
		.map_or(name, |(_, _, new)| new)
}

fn substitute(kind: &str, to: Version, name: &str) -> Option<&'static str> {
	substitutes::SUBSTITUTES
		.iter()
		.find(|(v, k, n, _)| *v == to && *k == kind && *n == name)
		.map(|e| e.3)
}

fn direct(kind: &str, from: Version, to: Version, id: i32) -> Option<i32> {
	if id < 0 {
		return None;
	}
	if same_registry(kind, from, to) {
		return Some(id);
	}
	registry::remap_name(kind, from, to, id as u32).map(|v| v as i32)
}

fn remap(kind: &str, from: Version, to: Version, id: i32) -> Option<i32> {
	direct(kind, from, to, id).or_else(|| {
		let name = registry::names(from, kind)?.name(id as u32)?;
		registry::names(to, kind)?
			.id(substitute(kind, to, name)?)
			.map(|v| v as i32)
	})
}

fn item(from: Version, to: Version, id: i32) -> Option<i32> {
	if id == 0 {
		return Some(0);
	}
	remap("items", from, to, id)
}

fn entity_name(version: Version, id: i32) -> Option<&'static str> {
	registry::names(version, "entities")?.name(u32::try_from(id).ok()?)
}

fn entity_id(version: Version, name: &str) -> Option<i32> {
	registry::names(version, "entities")?
		.id(name)
		.map(|v| v as i32)
}

fn wall_side(value: &str, up: bool) -> &str {
	match (up, value) {
		(true, "true") => "low",
		(true, _) => "none",
		(false, "none") => "false",
		(false, _) => "true",
	}
}

fn adjusted(
	from: Version,
	to: Version,
	name: &str,
	values: Vec<(&str, &str)>,
) -> (String, Vec<(String, String)>) {
	let own = |v: Vec<(&str, &str)>| {
		v.into_iter()
			.map(|(k, v)| (k.to_owned(), v.to_owned()))
			.collect::<Vec<_>>()
	};
	let level = values
		.iter()
		.find(|(k, _)| *k == "level")
		.map_or("0", |(_, v)| *v)
		.to_owned();
	match (from, to) {
		(Version::V1_15_2, Version::V1_16) | (Version::V1_16, Version::V1_15_2)
			if name.ends_with("_wall") =>
		{
			let up = from == Version::V1_15_2;
			let values = values
				.into_iter()
				.map(|(k, v)| match k {
					"east" | "west" | "north" | "south" => {
						(k.to_owned(), wall_side(v, up).to_owned())
					}
					_ => (k.to_owned(), v.to_owned()),
				})
				.collect();
			(name.to_owned(), values)
		}
		(Version::V1_15_2, Version::V1_16) if name == "jigsaw" => {
			let facing = values
				.iter()
				.find(|(k, _)| *k == "facing")
				.map_or("north", |(_, v)| *v);
			let orientation = match facing {
				"up" | "down" => format!("{facing}_north"),
				f => format!("{f}_up"),
			};
			(
				name.to_owned(),
				vec![("orientation".to_owned(), orientation)],
			)
		}
		(Version::V1_16, Version::V1_15_2) if name == "jigsaw" => {
			let orientation = values
				.iter()
				.find(|(k, _)| *k == "orientation")
				.map_or("north_up", |(_, v)| *v);
			let facing = orientation.split('_').next().unwrap_or("north").to_owned();
			(name.to_owned(), vec![("facing".to_owned(), facing)])
		}
		(Version::V1_16_2, Version::V1_17) if name == "cauldron" && level != "0" => (
			"water_cauldron".to_owned(),
			vec![("level".to_owned(), level)],
		),
		(Version::V1_16_2, Version::V1_17) if name == "cauldron" => (name.to_owned(), vec![]),
		(Version::V1_17, Version::V1_16_2) if name == "lava_cauldron" => (
			"cauldron".to_owned(),
			vec![("level".to_owned(), "3".to_owned())],
		),
		(Version::V1_17, Version::V1_16_2) if name.ends_with("_cauldron") => {
			("cauldron".to_owned(), vec![("level".to_owned(), level)])
		}
		_ => (name.to_owned(), own(values)),
	}
}

fn build_states(from: Version, to: Version) -> Option<Vec<u32>> {
	let (a, b) = (registry::blocks(from)?, registry::blocks(to)?);
	let air = b.block("air").map_or(0, |x| x.default);
	Some(
		(0..a.states())
			.map(|s| {
				let Some(block) = a.owner(s) else { return air };
				let (name, values) = adjusted(from, to, &block.name, block.values(s));
				let name = renamed("blocks", to, &name);
				let target = b.block(name).or_else(|| {
					(from > to)
						.then(|| substitute("blocks", to, name).and_then(|n| b.block(n)))
						.flatten()
				});
				let values: Vec<(&str, &str)> = values
					.iter()
					.map(|(k, v)| (k.as_str(), v.as_str()))
					.collect();
				target.map_or(air, |t| t.state(&values))
			})
			.collect(),
	)
}

fn states(from: Version, to: Version) -> Option<&'static [u32]> {
	if same_registry("blocks", from, to) {
		return None;
	}
	static TABLES: OnceLock<Mutex<HashMap<(Version, Version), &'static [u32]>>> = OnceLock::new();
	let mut tables = TABLES
		.get_or_init(Default::default)
		.lock()
		.expect("state tables poisoned");
	if let Some(t) = tables.get(&(from, to)) {
		return Some(t);
	}
	let table: &'static [u32] = Box::leak(build_states(from, to)?.into_boxed_slice());
	tables.insert((from, to), table);
	Some(table)
}

fn state(from: Version, to: Version, id: i32) -> i32 {
	match states(from, to) {
		Some(t) => t.get(id.max(0) as usize).copied().unwrap_or(0) as i32,
		None => id,
	}
}

fn custom_stat(from: Version, to: Version, id: i32) -> Option<i32> {
	match (from, to) {
		(Version::V1_15_2, Version::V1_16) => Some(if id >= 19 { id + 1 } else { id }),
		(Version::V1_16, Version::V1_15_2) => match id {
			19 | 72.. => None,
			i if i > 19 => Some(i - 1),
			i => Some(i),
		},
		(Version::V1_16_2, Version::V1_17) => Some(if id >= 2 { id + 1 } else { id }),
		(Version::V1_17, Version::V1_16_2) => match id {
			2 => None,
			i if i > 2 => Some(i - 1),
			i => Some(i),
		},
		_ => Some(id),
	}
}

fn codec(version: Version) -> Result<::bungeeoxide_nbt::Entry> {
	static CACHE: OnceLock<Mutex<HashMap<Version, Option<::bungeeoxide_nbt::Entry>>>> =
		OnceLock::new();
	let mut cache = CACHE
		.get_or_init(Default::default)
		.lock()
		.expect("codec cache poisoned");
	let codec = cache.entry(version).or_insert_with(|| {
		let json: serde_json::Value =
			serde_json::from_str(registry::login_packet(version)?).ok()?;
		::bungeeoxide_nbt::json::typed::entry(json.get("dimensionCodec")?)
	});
	match codec {
		Some(c) => Ok(c.clone()),
		None => unconvertible(format!("no vanilla dimension codec for {}", version.name())),
	}
}

type Tag = crate::gen::convert::v1_16::play_to_client_packet_tags_block_tags_items::Ty;
type SpawnObject = crate::gen::convert::v1_16::play_to_client_packet_spawn_entity::Ty;
type SpawnMob = crate::gen::convert::v1_16::play_to_client_packet_spawn_entity_living::Ty;
type Stat = crate::gen::convert::v1_16::play_to_client_packet_statistics_items::Ty;

fn strip(name: &str) -> &str {
	name.strip_prefix("minecraft:").unwrap_or(name)
}

fn tags(
	list: Vec<Tag>,
	kind: &str,
	from: Version,
	to: Version,
	added: &[(&str, &str, &[&str])],
	removed: &[&str],
) -> Vec<Tag> {
	let mut out: Vec<Tag> = list
		.into_iter()
		.filter(|t| !removed.contains(&strip(&t.tag_name)))
		.map(|mut t| {
			if kind != "fluids" {
				t.entries = t
					.entries
					.into_iter()
					.filter_map(|id| direct(kind, from, to, id))
					.collect();
			}
			t
		})
		.collect();
	for (_, name, members) in added.iter().filter(|(k, ..)| *k == kind) {
		if out.iter().any(|t| strip(&t.tag_name) == *name) {
			continue;
		}
		let entries = members
			.iter()
			.filter_map(|m| registry::names(to, kind)?.id(m).map(|v| v as i32))
			.collect();
		out.push(Tag {
			tag_name: format!("minecraft:{name}"),
			entries,
		});
	}
	out
}

fn spawned(from: Version, to: Version, id: i32) -> Option<(i32, Kind)> {
	let original = entity_name(from, id)?;
	let name = renamed("entities", to, original);
	if let Some(ty) = entity_id(to, name) {
		let name = entity_name(to, ty)?;
		return Some((
			ty,
			Kind {
				name,
				original: name,
				substitute: false,
			},
		));
	}
	let name = substitute("entities", to, name).filter(|_| from > to)?;
	Some((
		entity_id(to, name)?,
		Kind {
			name,
			original,
			substitute: true,
		},
	))
}

fn spawn_object(
	mut p: SpawnObject,
	from: Version,
	to: Version,
	cx: &mut Session,
) -> Result<SpawnObject> {
	let at = from.max(to);
	let Some((ty, kind)) = spawned(from, to, p.r#type) else {
		track(cx, at, p.entity_id, None);
		return dropped();
	};
	if kind.name == "falling_block" {
		p.object_data = state(from, to, p.object_data);
	}
	p.r#type = ty;
	track(cx, at, p.entity_id, Some(kind));
	Ok(p)
}

fn spawn_mob(mut p: SpawnMob, from: Version, to: Version, cx: &mut Session) -> Result<SpawnMob> {
	let at = from.max(to);
	let Some((ty, kind)) = spawned(from, to, p.r#type) else {
		track(cx, at, p.entity_id, None);
		return dropped();
	};
	p.r#type = ty;
	track(cx, at, p.entity_id, Some(kind));
	Ok(p)
}

fn statistics(list: Vec<Stat>, from: Version, to: Version) -> Vec<Stat> {
	list.into_iter()
		.filter_map(|mut s| {
			s.statistic_id = match s.category_id {
				0 => remap("blocks", from, to, s.statistic_id)?,
				1..=5 => item(from, to, s.statistic_id)?,
				6 | 7 => remap("entities", from, to, s.statistic_id)?,
				_ => custom_stat(from, to, s.statistic_id)?,
			};
			Some(s)
		})
		.collect()
}

type BlockChange = crate::gen::convert::v1_16::play_to_client_packet_block_change::Ty;
type Digging = crate::gen::convert::v1_16::play_to_client_packet_acknowledge_player_digging::Ty;
type BlockAction = crate::gen::convert::v1_16::play_to_client_packet_block_action::Ty;
type WorldEvent = crate::gen::convert::v1_16::play_to_client_packet_world_event::Ty;
type Sound = crate::gen::convert::v1_16::play_to_client_packet_sound_effect::Ty;
type EntitySound = crate::gen::convert::v1_16::play_to_client_packet_entity_sound_effect::Ty;
type Cooldown = crate::gen::convert::v1_16::play_to_client_packet_set_cooldown::Ty;
type Tags = crate::gen::convert::v1_16::play_to_client_packet_tags::Ty;

fn block_change(mut v: BlockChange, from: Version, to: Version) -> BlockChange {
	v.r#type = state(from, to, v.r#type);
	v
}

fn digging(mut v: Digging, from: Version, to: Version) -> Digging {
	v.block = state(from, to, v.block);
	v
}

fn block_action(mut v: BlockAction, from: Version, to: Version) -> Result<BlockAction> {
	v.block_id = kept(remap("blocks", from, to, v.block_id))?;
	Ok(v)
}

fn world_event(mut v: WorldEvent, from: Version, to: Version) -> Result<WorldEvent> {
	match v.effect_id {
		2001 => v.data = state(from, to, v.data),
		1010 if v.data != 0 => v.data = kept(item(from, to, v.data))?,
		_ => {}
	}
	Ok(v)
}

fn sound_effect(mut v: Sound, from: Version, to: Version) -> Result<Sound> {
	v.sound_id = kept(remap("sounds", from, to, v.sound_id))?;
	Ok(v)
}

fn entity_sound(mut v: EntitySound, from: Version, to: Version) -> Result<EntitySound> {
	v.sound_id = kept(remap("sounds", from, to, v.sound_id))?;
	Ok(v)
}

fn cooldown(mut v: Cooldown, from: Version, to: Version) -> Result<Cooldown> {
	v.item_id = kept(item(from, to, v.item_id))?;
	Ok(v)
}

fn all_tags(
	v: Tags,
	from: Version,
	to: Version,
	added: &[(&str, &str, &[&str])],
	removed: &[&str],
) -> Tags {
	Tags {
		block_tags: tags(v.block_tags, "blocks", from, to, added, &[]),
		item_tags: tags(v.item_tags, "items", from, to, added, removed),
		fluid_tags: tags(v.fluid_tags, "fluids", from, to, added, &[]),
		entity_tags: tags(v.entity_tags, "entities", from, to, added, &[]),
	}
}

fn dropped<T>() -> Result<T> {
	Err(Error::Dropped)
}

fn kept<T>(value: Option<T>) -> Result<T> {
	value.ok_or(Error::Dropped)
}
