use std::collections::HashMap;
use std::sync::{LazyLock, OnceLock};

mod files;
pub mod renames;
mod sounds;

use crate::gen::convert::{index_of, Version, VERSIONS};

pub trait AsVersion: Copy {
	fn version_index(self) -> Option<usize>;

	fn version_name(self) -> Option<&'static str> {
		self.version_index().map(|i| VERSIONS[i])
	}
}

impl AsVersion for Version {
	fn version_index(self) -> Option<usize> {
		Some(self as usize)
	}
}

impl AsVersion for &str {
	fn version_index(self) -> Option<usize> {
		index_of(self)
	}
}

impl AsVersion for &String {
	fn version_index(self) -> Option<usize> {
		index_of(self)
	}
}

pub fn file(path: &str) -> Option<&'static str> {
	files::FILES
		.iter()
		.find(|(p, _)| *p == path)
		.map(|(_, t)| *t)
}

const KINDS: &[&str] = &[
	"blocks",
	"items",
	"entities",
	"objects",
	"particles",
	"biomes",
	"effects",
	"enchantments",
	"sounds",
	"loginPacket",
];

struct Slot {
	folder: &'static str,
	blocks: OnceLock<Option<Blocks>>,
	names: OnceLock<Option<Names>>,
}

type Tables = OnceLock<Box<[OnceLock<Option<Box<[u32]>>>]>>;

struct Registry {
	slots: Vec<Slot>,
	at: Vec<[Option<usize>; KINDS.len()]>,
	tables: Vec<Tables>,
}

static REGISTRY: LazyLock<Registry> = LazyLock::new(|| {
	let mut slots: Vec<Slot> = Vec::new();
	let mut owned: HashMap<(&str, &str), usize> = HashMap::new();
	let mut at: Vec<[Option<usize>; KINDS.len()]> = vec![[None; KINDS.len()]; VERSIONS.len()];
	let mut add = |version: &str, kind: &str, folder: &'static str| {
		let Some(v) = index_of(version) else { return };
		let Some(k) = KINDS.iter().position(|x| *x == kind) else {
			return;
		};
		let slot = *owned.entry((folder, KINDS[k])).or_insert_with(|| {
			slots.push(Slot {
				folder,
				blocks: OnceLock::new(),
				names: OnceLock::new(),
			});
			slots.len() - 1
		});
		at[v][k] = Some(slot);
	};
	for line in file("versions.tsv").unwrap_or_default().lines() {
		let mut parts = line.split('\t');
		if let (Some(v), Some(k), Some(f)) = (parts.next(), parts.next(), parts.next()) {
			add(v, k, f);
			if k == "entities" {
				add(v, "objects", f);
			}
		}
	}
	Registry {
		slots,
		at,
		tables: (0..=KINDS.len()).map(|_| OnceLock::new()).collect(),
	}
});

fn kind_index(kind: &str) -> Option<usize> {
	KINDS.iter().position(|k| *k == kind)
}

fn slot(version: impl AsVersion, kind: &str) -> Option<&'static Slot> {
	let r = &*REGISTRY;
	Some(&r.slots[r.at[version.version_index()?][kind_index(kind)?]?])
}

pub fn folder(version: impl AsVersion, kind: &str) -> Option<&'static str> {
	Some(slot(version, kind)?.folder)
}

fn data(version: impl AsVersion, kind: &str, ext: &str) -> Option<&'static str> {
	file(&format!("{}/{kind}.{ext}", folder(version, kind)?))
}

fn strip(name: &str) -> &str {
	name.strip_prefix("minecraft:").unwrap_or(name)
}

#[derive(Debug)]
pub struct Prop {
	pub name: String,
	pub values: Vec<String>,
}

#[derive(Debug)]
pub struct Block {
	pub id: u32,
	pub name: String,
	pub min: u32,
	pub default: u32,
	pub props: Vec<Prop>,
}

impl Block {
	pub fn states(&self) -> u32 {
		self.props.iter().map(|p| p.values.len() as u32).product()
	}

	pub fn values(&self, state: u32) -> Vec<(&str, &str)> {
		let mut index = state - self.min;
		let mut out = Vec::with_capacity(self.props.len());
		for p in self.props.iter().rev() {
			let n = p.values.len() as u32;
			out.push((p.name.as_str(), p.values[(index % n) as usize].as_str()));
			index /= n;
		}
		out.reverse();
		out
	}

	pub fn state(&self, values: &[(&str, &str)]) -> u32 {
		let defaults = self.values(self.default);
		let mut index = 0;
		for (p, (_, default)) in self.props.iter().zip(defaults) {
			let wanted = values
				.iter()
				.find(|(k, _)| *k == p.name)
				.map_or(default, |(_, v)| v);
			let at = p
				.values
				.iter()
				.position(|v| v == wanted)
				.unwrap_or_else(|| {
					p.values
						.iter()
						.position(|v| v == default)
						.expect("default value of the block")
				});
			index = index * p.values.len() as u32 + at as u32;
		}
		self.min + index
	}
}

#[derive(Debug)]
pub struct Blocks {
	pub list: Vec<Block>,
	pub flattened: bool,
	by_name: HashMap<String, usize>,
	by_state: Vec<u32>,
}

impl Blocks {
	fn parse(text: &str) -> Self {
		let mut list = Vec::new();
		let mut flattened = false;
		for line in text.lines().filter(|l| !l.is_empty()) {
			let mut parts = line.split(' ');
			let id: u32 = parts.next().and_then(|s| s.parse().ok()).expect("block id");
			let name = parts.next().expect("block name").to_owned();
			let (min, default) = match (parts.next(), parts.next()) {
				(Some(min), Some(default)) => {
					flattened = true;
					(
						min.parse().expect("min state"),
						default.parse().expect("default state"),
					)
				}
				_ => (id << 4, id << 4),
			};
			let props = parts
				.map(|p| {
					let (name, values) = p.split_once('=').expect("property");
					Prop {
						name: name.to_owned(),
						values: values.split(',').map(str::to_owned).collect(),
					}
				})
				.collect();
			list.push(Block {
				id,
				name,
				min,
				default,
				props,
			});
		}
		let by_name = list
			.iter()
			.enumerate()
			.map(|(i, b)| (b.name.clone(), i))
			.collect();
		let mut by_state = Vec::new();
		if flattened {
			for (i, b) in list.iter().enumerate() {
				let end = (b.min + b.states()) as usize;
				if by_state.len() < end {
					by_state.resize(end, u32::MAX);
				}
				for s in b.min..b.min + b.states() {
					by_state[s as usize] = i as u32;
				}
			}
		}
		Self {
			list,
			flattened,
			by_name,
			by_state,
		}
	}

	pub fn states(&self) -> u32 {
		self.by_state.len() as u32
	}

	pub fn block(&self, name: &str) -> Option<&Block> {
		self.by_name.get(strip(name)).map(|i| &self.list[*i])
	}

	pub fn owner(&self, state: u32) -> Option<&Block> {
		let i = *self.by_state.get(state as usize)?;
		self.list.get(i as usize)
	}

	pub fn describe(&self, state: u32) -> Option<String> {
		let b = self.owner(state)?;
		let values = b.values(state);
		if values.is_empty() {
			return Some(format!("minecraft:{}", b.name));
		}
		let props: Vec<String> = values.iter().map(|(k, v)| format!("{k}={v}")).collect();
		Some(format!("minecraft:{}[{}]", b.name, props.join(",")))
	}

	pub fn parse_state(&self, text: &str) -> Option<u32> {
		let (name, props) = match text.split_once('[') {
			Some((name, rest)) => (name, rest.trim_end_matches(']')),
			None => (text, ""),
		};
		let block = self.block(name)?;
		let values: Vec<(&str, &str)> =
			props.split(',').filter_map(|p| p.split_once('=')).collect();
		Some(block.state(&values))
	}
}

pub fn blocks(version: impl AsVersion) -> Option<&'static Blocks> {
	let slot = slot(version, "blocks")?;
	slot.blocks
		.get_or_init(|| Some(Blocks::parse(file(&format!("{}/blocks.txt", slot.folder))?)))
		.as_ref()
}

#[derive(Debug)]
pub struct Names {
	pub names: HashMap<u32, String>,
	pub extra: HashMap<u32, Vec<String>>,
	by_name: HashMap<String, u32>,
}

impl Names {
	fn parse(text: &str, keep: impl Fn(Option<&str>) -> bool) -> Self {
		let mut names = HashMap::new();
		let mut extra = HashMap::new();
		let mut by_name = HashMap::new();
		for line in text
			.lines()
			.filter(|l| !l.is_empty() && keep(l.split(' ').nth(2)))
		{
			let mut parts = line.split(' ');
			let id: u32 = parts
				.next()
				.and_then(|s| s.parse().ok())
				.expect("registry id");
			let name = strip(parts.next().expect("registry name")).to_owned();
			by_name.entry(name.clone()).or_insert(id);
			names.insert(id, name);
			let rest: Vec<String> = parts.map(str::to_owned).collect();
			if !rest.is_empty() {
				extra.insert(id, rest);
			}
		}
		Self {
			names,
			extra,
			by_name,
		}
	}

	pub fn name(&self, id: u32) -> Option<&str> {
		self.names.get(&id).map(String::as_str)
	}

	pub fn id(&self, name: &str) -> Option<u32> {
		self.by_name.get(strip(name)).copied()
	}
}

pub fn names(version: impl AsVersion, kind: &str) -> Option<&'static Names> {
	let slot = slot(version, kind)?;
	let source = if kind == "objects" { "entities" } else { kind };
	slot.names
		.get_or_init(|| {
			let text = file(&format!("{}/{source}.txt", slot.folder))?;
			Some(match kind {
				"entities" => Names::parse(text, |t| t != Some("object")),
				"objects" => Names::parse(text, |t| t == Some("object")),
				_ => Names::parse(text, |_| true),
			})
		})
		.as_ref()
}

pub fn entity_metadata_keys(version: impl AsVersion, entity: u32) -> Option<&'static [String]> {
	let extra = names(version, "entities")?.extra.get(&entity)?;
	Some(&extra[1.min(extra.len())..])
}

pub fn login_packet(version: impl AsVersion) -> Option<&'static str> {
	data(version, "loginPacket", "json")
}

fn renamed<'a>(table: &[(&str, &str, &'a str)], to: &str, name: &'a str) -> &'a str {
	table
		.iter()
		.find(|(v, old, _)| *v == to && *old == name)
		.map_or(name, |(_, _, new)| new)
}

fn table(
	slot: usize,
	from: impl AsVersion,
	to: impl AsVersion,
	make: impl FnOnce() -> Option<Vec<u32>>,
) -> Option<&'static [u32]> {
	let r = &*REGISTRY;
	let (a, b) = (from.version_index()?, to.version_index()?);
	let n = VERSIONS.len();
	let pairs = r.tables[slot].get_or_init(|| (0..n * n).map(|_| OnceLock::new()).collect());
	pairs[a * n + b]
		.get_or_init(|| make().map(Vec::into_boxed_slice))
		.as_deref()
}

pub fn name_table(kind: &str, from: impl AsVersion, to: impl AsVersion) -> Option<&'static [u32]> {
	table(kind_index(kind)?, from, to, || {
		let (a, b) = (names(from, kind)?, names(to, kind)?);
		let to = to.version_name()?;
		let len = a.names.keys().max().map_or(0, |m| m + 1);
		Some(
			(0..len)
				.map(|id| {
					a.name(id)
						.and_then(|name| {
							let r = renamed(renames::table(kind), to, name);
							b.id(r).or_else(|| {
								(kind == "sounds")
									.then(|| b.id(renamed(sounds::SOUNDS, to, name)))
									.flatten()
							})
						})
						.unwrap_or(u32::MAX)
				})
				.collect(),
		)
	})
}

pub fn sound_fallback(name: &str, to: impl AsVersion) -> Option<&'static str> {
	let target = to.version_index()?;
	let mut current = name.to_owned();
	for at in (target..VERSIONS.len()).rev() {
		let Some(list) = names(Version::at(at)?, "sounds") else {
			continue;
		};
		if list.id(&current).is_some() {
			continue;
		}
		let version = VERSIONS[at];
		let next = renamed(renames::table("sounds"), version, &current);
		let next = if next == current {
			renamed(sounds::SOUNDS, version, &current)
		} else {
			next
		};
		current = next.to_owned();
	}
	let list = names(Version::at(target)?, "sounds")?;
	list.name(list.id(&current)?)
}

pub fn remap_name(kind: &str, from: impl AsVersion, to: impl AsVersion, id: u32) -> Option<u32> {
	name_table(kind, from, to)?
		.get(id as usize)
		.copied()
		.filter(|v| *v != u32::MAX)
}

pub fn state_table(from: impl AsVersion, to: impl AsVersion) -> Option<&'static [u32]> {
	table(KINDS.len(), from, to, || {
		let (a, b) = (blocks(from)?, blocks(to)?);
		let to = to.version_name()?;
		let air = b.block("air").map_or(0, |x| x.default);
		Some(
			(0..a.states())
				.map(|s| {
					let Some(block) = a.owner(s) else { return air };
					let name = renamed(renames::table("blocks"), to, &block.name);
					match b.block(name) {
						Some(target) => target.state(&block.values(s)),
						None => air,
					}
				})
				.collect(),
		)
	})
}

pub fn remap_state(from: impl AsVersion, to: impl AsVersion, state: u32) -> u32 {
	state_table(from, to)
		.and_then(|t| t.get(state as usize).copied())
		.unwrap_or(0)
}

static LEGACY: OnceLock<(
	HashMap<(u16, u8), &'static str>,
	HashMap<(u16, u8), &'static str>,
)> = OnceLock::new();

fn legacy() -> &'static (
	HashMap<(u16, u8), &'static str>,
	HashMap<(u16, u8), &'static str>,
) {
	LEGACY.get_or_init(|| {
		let mut blocks = HashMap::new();
		let mut items = HashMap::new();
		for line in file("legacy.txt").unwrap_or_default().lines() {
			let mut parts = line.splitn(3, ' ');
			let (Some(kind), Some(key), Some(value)) = (parts.next(), parts.next(), parts.next())
			else {
				continue;
			};
			let Some((id, meta)) = key.split_once(':') else {
				continue;
			};
			let (Ok(id), Ok(meta)) = (id.parse(), meta.parse()) else {
				continue;
			};
			match kind {
				"block" => blocks.insert((id, meta), value),
				_ => items.insert((id, meta), value),
			};
		}
		(blocks, items)
	})
}

pub fn legacy_block(id: u16, meta: u8) -> Option<&'static str> {
	legacy().0.get(&(id, meta)).copied()
}

pub fn legacy_item(id: u16, meta: u8) -> Option<&'static str> {
	legacy().1.get(&(id, meta)).copied()
}

pub fn legacy_block_of(state: &str) -> Option<(u16, u8)> {
	legacy()
		.0
		.iter()
		.filter(|(_, v)| **v == state)
		.map(|(k, _)| *k)
		.min()
}

pub fn legacy_item_of(name: &str) -> Option<(u16, u8)> {
	legacy()
		.1
		.iter()
		.filter(|(_, v)| **v == name)
		.map(|(k, _)| *k)
		.min()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn block_states_round_trip() {
		let b = blocks("1.21.4").unwrap();
		let stairs = b.block("oak_stairs").unwrap();
		assert_eq!(stairs.state(&[]), stairs.default);
		assert_eq!(
			b.describe(stairs.default).unwrap(),
			"minecraft:oak_stairs[facing=north,half=bottom,shape=straight,waterlogged=false]"
		);
		assert_eq!(
			b.parse_state("minecraft:oak_stairs[half=bottom,facing=north]"),
			Some(stairs.default)
		);
	}

	#[test]
	fn flattening_uses_legacy_names() {
		let log = legacy_block(17, 0).unwrap();
		let b = blocks("1.13").unwrap();
		let state = b.parse_state(log).unwrap();
		assert_eq!(b.describe(state).unwrap(), log);
	}

	#[test]
	fn states_survive_newer_versions() {
		let (old, new) = (blocks("1.13").unwrap(), blocks("26.1").unwrap());
		let state = old.parse_state("minecraft:oak_log[axis=x]").unwrap();
		let moved = remap_state("1.13", "26.1", state);
		assert_eq!(new.describe(moved).unwrap(), "minecraft:oak_log[axis=x]");
	}
}
