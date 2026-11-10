use std::collections::HashMap;
use std::sync::OnceLock;

use crate::gen::convert::Version;
use crate::registry::{self, Blocks, Names};

pub(super) const OLD: Version = Version::V1_12_2;
pub(super) const FLAT: Version = Version::V1_13;
pub(super) const LATER: Version = Version::V1_13_1;

pub(super) fn new_blocks() -> &'static Blocks {
	registry::blocks(FLAT).expect("1.13 blocks")
}

pub(super) fn later_blocks() -> &'static Blocks {
	registry::blocks(LATER).expect("1.13.1 blocks")
}

pub(super) fn old_blocks() -> &'static Blocks {
	registry::blocks(OLD).expect("1.12.2 blocks")
}

pub(super) fn new_items() -> &'static Names {
	registry::names(FLAT, "items").expect("1.13 items")
}

pub(super) fn old_items() -> &'static Names {
	registry::names(OLD, "items").expect("1.12.2 items")
}

pub(super) fn base(state: &str) -> &str {
	let s = state.strip_prefix("minecraft:").unwrap_or(state);
	s.split_once('[').map_or(s, |(n, _)| n)
}

struct Legacy {
	up: Vec<u32>,
	down: Vec<u32>,
}

const RAW: usize = 8192;

const IDENTITY: [&str; 4] = ["half", "part", "type", "face"];

fn axis(facing: &str) -> u8 {
	match facing {
		"north" | "south" => 1,
		"east" | "west" => 2,
		_ => 0,
	}
}

fn legacy() -> &'static Legacy {
	static TABLE: OnceLock<Legacy> = OnceLock::new();
	TABLE.get_or_init(|| {
		let b = new_blocks();
		let exact = |raw: usize| {
			registry::legacy_block((raw >> 4) as u16, (raw & 15) as u8)
				.and_then(|s| b.parse_state(s))
		};
		let mut up = vec![0; RAW];
		let mut down = vec![u32::MAX; b.states() as usize];
		let mut by_block: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
		let mut by_legacy: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
		for raw in (0..RAW).rev() {
			let Some(state) = exact(raw) else { continue };
			up[raw] = state;
			if down[state as usize] == u32::MAX {
				down[state as usize] = raw as u32;
				if let Some(owner) = b.owner(state) {
					by_block
						.entry(owner.id)
						.or_default()
						.push((state, raw as u32));
				}
				by_legacy
					.entry(raw as u32 >> 4)
					.or_default()
					.push((state, raw as u32));
			}
		}
		for raw in 0..RAW {
			if exact(raw).is_none() {
				up[raw] = up[raw & !15];
			}
		}
		for (state, raw) in down.iter_mut().enumerate() {
			if *raw != u32::MAX {
				continue;
			}
			let Some(block) = b.owner(state as u32) else {
				*raw = 0;
				continue;
			};
			let target = match by_block.get(&block.id) {
				Some(c) => Some(c),
				None => super::fallback::substitute(&block.name)
					.and_then(|n| b.block(n))
					.and_then(|t| by_block.get(&t.id)),
			};
			let values = block.values(state as u32);
			let identity = values.iter().filter(|v| IDENTITY.contains(&v.0)).count();
			let score = |s: u32| {
				let theirs = b.owner(s).map(|o| o.values(s)).unwrap_or_default();
				let mut ident = 0;
				let mut total = 0;
				for (k, v) in &values {
					let Some((_, t)) = theirs.iter().find(|(n, _)| n == k) else {
						continue;
					};
					let points = if t == v {
						2
					} else {
						usize::from(*k == "facing" && axis(t) == axis(v))
					};
					total += points;
					if IDENTITY.contains(k) && points == 2 {
						ident += 1;
					}
				}
				(ident, total)
			};
			let best = |pool: &[(u32, u32)]| pool.iter().map(|(s, r)| (score(*s), *r)).max();
			let mut found = target.and_then(|t| best(t));
			if found.is_some_and(|((ident, _), _)| ident < identity) {
				let wider: Vec<(u32, u32)> = target
					.into_iter()
					.flatten()
					.flat_map(|(_, r)| by_legacy.get(&(r >> 4)).into_iter().flatten().copied())
					.collect();
				found = best(&wider).max(found);
			}
			*raw = found.map_or(0, |(_, r)| r);
		}
		Legacy { up, down }
	})
}

pub(super) fn state_of_raw(raw: u32) -> u32 {
	legacy().up.get(raw as usize).copied().unwrap_or(0)
}

pub(super) fn raw_of_state(state: u32) -> u32 {
	legacy().down.get(state as usize).copied().unwrap_or(0)
}

pub(super) fn state_of_packed(packed: i32) -> u32 {
	let packed = packed.max(0) as u32;
	state_of_raw(((packed & 4095) << 4) | (packed >> 12 & 15))
}

pub(super) fn packed_of_state(state: u32) -> i32 {
	let raw = raw_of_state(state);
	((raw >> 4) | ((raw & 15) << 12)) as i32
}

pub(super) fn block_of_legacy(id: i32) -> i32 {
	let state = state_of_raw((id.max(0) as u32) << 4);
	new_blocks().owner(state).map_or(0, |b| b.id as i32)
}

pub(super) fn legacy_of_block(id: i32) -> i32 {
	let b = new_blocks();
	let Some(block) = b.list.get(id.max(0) as usize) else {
		return 0;
	};
	(raw_of_state(block.default) >> 4) as i32
}

pub(super) fn later_states() -> &'static [u32] {
	registry::state_table(FLAT, LATER).expect("1.13 to 1.13.1 states")
}

pub(super) fn earlier_states() -> &'static [u32] {
	registry::state_table(LATER, FLAT).expect("1.13.1 to 1.13 states")
}

pub(super) fn state_later(state: u32) -> u32 {
	later_states().get(state as usize).copied().unwrap_or(0)
}

pub(super) fn state_earlier(state: u32) -> u32 {
	earlier_states().get(state as usize).copied().unwrap_or(0)
}

fn block_remap(id: i32, from: &Blocks, to: &Blocks) -> i32 {
	from.list
		.get(id.max(0) as usize)
		.and_then(|b| to.block(&b.name))
		.map_or(0, |b| b.id as i32)
}

pub(super) fn block_later(id: i32) -> i32 {
	block_remap(id, new_blocks(), later_blocks())
}

pub(super) fn block_earlier(id: i32) -> i32 {
	block_remap(id, later_blocks(), new_blocks())
}

pub(super) fn item_later(id: i32) -> Option<i32> {
	registry::remap_name("items", FLAT, LATER, id.max(0) as u32).map(|i| i as i32)
}

pub(super) fn item_earlier(id: i32) -> Option<i32> {
	registry::remap_name("items", LATER, FLAT, id.max(0) as u32).map(|i| i as i32)
}

pub(super) fn biome_valid(id: u8) -> bool {
	registry::names(FLAT, "biomes").is_some_and(|n| n.name(id as u32).is_some())
}

pub(super) const DYES: [&str; 16] = [
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
