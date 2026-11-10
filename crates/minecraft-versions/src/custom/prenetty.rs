mod a;
mod b;
mod c;
mod chunk;
mod d;
mod e;
mod entities;
mod f;
mod heights;
mod ids;
mod tables;
mod text;

use std::collections::{HashMap, HashSet};

use crate::gen::convert::{v1_0_1, v1_4_2, v1_6_4};
use crate::pds_native::pds_legacySlot;
use crate::session::Session;

pub use a::*;
pub use b::*;
pub use c::*;
pub use d::*;
pub use e::*;
pub use entities::*;
pub use f::*;
pub use heights::*;

use ids::V;

type LegacySlot = pds_legacySlot::Ty;
type Slot = v1_6_4::slot::Ty;
type OldMetaItem = v1_0_1::entityMetadataItem::Ty;
type MidMetaItem = v1_4_2::entityMetadataItem::Ty;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
	Player,
	Mob(u8),
	Object(i8, i32),
}

#[derive(Default)]
struct Pending {
	x: i32,
	y: i32,
	z: i32,
	velocity: (i16, i16, i16),
}

#[derive(Default)]
struct State {
	player: i32,
	joined: bool,
	kinds: HashMap<i32, Kind>,
	stand_ins: HashSet<i32>,
	x: f64,
	y: f64,
	z: f64,
	yaw: f32,
	pitch: f32,
	stats: HashMap<i32, i32>,
	stat_values: HashMap<String, i32>,
	vehicle: Option<i32>,
	sneaking: bool,
	pending: HashMap<i32, Pending>,
	events: HashMap<(i32, i32, i32), u16>,
}

fn state(cx: &mut Session) -> &mut State {
	cx.state::<State>()
}

fn legacy_slot(v: LegacySlot, to: V) -> LegacySlot {
	v.map(|mut item| {
		let (id, damage) = ids::item(item.id, item.itemDamage, to);
		if id != item.id && !pds_legacySlot::has_nbt(id) {
			item.nbtData = None;
		}
		item.id = id;
		item.itemDamage = damage;
		item
	})
}

fn slot(v: Slot, to: V) -> Slot {
	match v {
		Slot::default(id, mut item) => {
			let (id, damage) = ids::item(id, item.itemDamage, to);
			item.itemDamage = damage;
			Slot::default(id, item)
		}
		other => other,
	}
}

fn old_meta_item(v: OldMetaItem, to: V) -> OldMetaItem {
	match v {
		OldMetaItem::v5(mut item) => {
			if item.id >= 0 {
				(item.id, item.itemDamage) = ids::item(item.id, item.itemDamage, to);
			}
			OldMetaItem::v5(item)
		}
		other => other,
	}
}

fn mid_meta_item(v: MidMetaItem, to: V) -> MidMetaItem {
	use crate::gen::convert::v1_4_2::entityMetadataItem__v5 as Item;
	match v {
		MidMetaItem::v5(Item::Ty::default(id, mut item)) => {
			let (id, damage) = ids::item(id, item.itemDamage, to);
			item.itemDamage = damage;
			MidMetaItem::v5(Item::Ty::default(id, item))
		}
		other => other,
	}
}

fn item_id(id: i16, to: V) -> i16 {
	if id <= 0 {
		return id;
	}
	ids::item(id, 0, to).0
}

fn falling_block(data: i32, to: V, packed: bool) -> i32 {
	let (id, meta) = if packed {
		((data & 0xffff) as u16, (data >> 16) as u8)
	} else {
		(data as u16, 0)
	};
	let (id, meta) = ids::block(id, meta, to);
	if packed {
		id as i32 | (meta as i32) << 16
	} else {
		id as i32
	}
}

fn remap_record(block: u16, to: V) -> u16 {
	let (id, meta) = ids::block(block >> 4, (block & 15) as u8, to);
	id << 4 | meta as u16
}

fn world_event_data(effect: i32, data: i32, to: V) -> i32 {
	if effect != 2001 {
		return data;
	}
	let (id, meta) = ids::block((data & 4095) as u16, (data >> 12 & 255) as u8, to);
	id as i32 | (meta as i32) << 12
}

#[cfg(test)]
fn one<T>(mut v: Vec<T>) -> T {
	assert_eq!(v.len(), 1);
	v.pop().expect("one element")
}
