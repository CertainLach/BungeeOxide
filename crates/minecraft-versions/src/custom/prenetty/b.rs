use nbt::{Entry, Payload};

use super::chunk::{self, Shape};
use super::ids::{self, V};
use super::{
	falling_block, item_id, legacy_slot, old_meta_item, state, world_event_data, LegacySlot,
	OldMetaItem, Slot,
};
use crate::gen::convert::{v1_2_5, v1_3_2};
use crate::pds::convert::{Error, Result};
use crate::pds_native::pds_legacySlot::{self, Item};
use crate::session::Session;

type Client125 = v1_2_5::play_toClient_packet::Ty;
type Client132 = v1_3_2::play_toClient_packet::Ty;
type Server125 = v1_2_5::play_toServer_packet::Ty;
type Server132 = v1_3_2::play_toServer_packet::Ty;
type Object125 = v1_2_5::play_toClient_packet_spawn_entity__objectData::Ty;

const TO: V = V::R1_2_5;
const EVENT_BLOCKS: [u8; 4] = [25, 29, 33, 54];

const ENTITY_NAMES: [(&str, i32); 41] = [
	("Item", 1),
	("XPOrb", 2),
	("Painting", 9),
	("Arrow", 10),
	("Snowball", 11),
	("Fireball", 12),
	("SmallFireball", 13),
	("ThrownEnderpearl", 14),
	("EyeOfEnderSignal", 15),
	("ThrownPotion", 16),
	("ThrownExpBottle", 17),
	("PrimedTnt", 20),
	("FallingSand", 21),
	("Minecart", 40),
	("Boat", 41),
	("Mob", 48),
	("Monster", 49),
	("Creeper", 50),
	("Skeleton", 51),
	("Spider", 52),
	("Giant", 53),
	("Zombie", 54),
	("Slime", 55),
	("Ghast", 56),
	("PigZombie", 57),
	("Enderman", 58),
	("CaveSpider", 59),
	("Silverfish", 60),
	("Blaze", 61),
	("LavaSlime", 62),
	("EnderDragon", 63),
	("Pig", 90),
	("Sheep", 91),
	("Cow", 92),
	("Chicken", 93),
	("Squid", 94),
	("Wolf", 95),
	("MushroomCow", 96),
	("SnowMan", 97),
	("Ozelot", 98),
	("VillagerGolem", 99),
];

fn slot_up(v: LegacySlot) -> Slot {
	match v {
		None => Slot::_1,
		Some(item) => Slot::default(
			item.id,
			v1_3_2::slot__default::Ty {
				itemCount: item.itemCount,
				itemDamage: item.itemDamage,
				nbtData: item.nbtData,
			},
		),
	}
}

fn slot_down(v: Slot) -> LegacySlot {
	match v {
		Slot::default(id, item) if id >= 0 => {
			let mut out = legacy_slot(
				Some(Item {
					id,
					itemCount: item.itemCount,
					itemDamage: item.itemDamage,
					nbtData: item.nbtData,
				}),
				TO,
			);
			if let Some(item) = &mut out {
				if !pds_legacySlot::has_nbt(item.id) {
					item.nbtData = None;
				}
			}
			out
		}
		_ => None,
	}
}

macro_rules! windows {
	($set_slot:ident, $items:ident, $creative:ident, $creative_server:ident, $click:ident, $place:ident, $from:ident, $to:ident, $conv:ident) => {
		pub fn $set_slot(
			p: $from::play_toClient_packet_set_slot::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_toClient_packet::Ty>> {
			Ok(vec![$to::play_toClient_packet::Ty::set_slot(
				$to::play_toClient_packet_set_slot::Ty {
					windowId: p.windowId,
					slot: p.slot,
					item: $conv(p.item),
				},
			)])
		}

		pub fn $items(
			p: $from::play_toClient_packet_window_items::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_toClient_packet::Ty>> {
			Ok(vec![$to::play_toClient_packet::Ty::window_items(
				$to::play_toClient_packet_window_items::Ty {
					windowId: p.windowId,
					items: p.items.into_iter().map($conv).collect(),
				},
			)])
		}

		pub fn $creative(
			p: $from::play_toClient_packet_set_creative_slot::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_toClient_packet::Ty>> {
			Ok(vec![$to::play_toClient_packet::Ty::set_creative_slot(
				$to::play_toClient_packet_set_creative_slot::Ty {
					slot: p.slot,
					item: $conv(p.item),
				},
			)])
		}

		pub fn $creative_server(
			p: $from::play_toServer_packet_set_creative_slot::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_toServer_packet::Ty>> {
			Ok(vec![$to::play_toServer_packet::Ty::set_creative_slot(
				$to::play_toServer_packet_set_creative_slot::Ty {
					slot: p.slot,
					item: $conv(p.item),
				},
			)])
		}

		pub fn $click(
			p: $from::play_toServer_packet_window_click::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_toServer_packet::Ty>> {
			Ok(vec![$to::play_toServer_packet::Ty::window_click(
				$to::play_toServer_packet_window_click::Ty {
					windowId: p.windowId,
					slot: p.slot,
					mouseButton: p.mouseButton,
					action: p.action,
					mode: p.mode,
					item: $conv(p.item),
				},
			)])
		}

		pub fn $place(
			p: $from::play_toServer_packet_block_place::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_toServer_packet::Ty>> {
			Ok(vec![$to::play_toServer_packet::Ty::block_place(place(
				p.location,
				p.direction,
				$conv(p.heldItem),
			))])
		}
	};
}

fn place_up(
	location: v1_2_5::position_ibi::Ty,
	direction: i8,
	held: Slot,
) -> v1_3_2::play_toServer_packet_block_place::Ty {
	v1_3_2::play_toServer_packet_block_place::Ty {
		location,
		direction,
		heldItem: held,
		cursorX: 8,
		cursorY: 8,
		cursorZ: 8,
	}
}

fn place_down(
	location: v1_2_5::position_ibi::Ty,
	direction: i8,
	held: LegacySlot,
) -> v1_2_5::play_toServer_packet_block_place::Ty {
	v1_2_5::play_toServer_packet_block_place::Ty {
		location,
		direction,
		heldItem: held,
	}
}

mod up {
	use super::*;
	use place_up as place;
	windows!(
		prenetty_set_slot_1_3_2,
		prenetty_window_items_1_3_2,
		prenetty_set_creative_slot_1_3_2,
		prenetty_set_creative_slot_server_1_3_2,
		prenetty_window_click_1_3_2,
		prenetty_block_place_1_3_2,
		v1_2_5,
		v1_3_2,
		slot_up
	);
}

mod down {
	use super::*;
	use place_down as place;
	windows!(
		prenetty_set_slot_1_2_5,
		prenetty_window_items_1_2_5,
		prenetty_set_creative_slot_1_2_5,
		prenetty_set_creative_slot_server_1_2_5,
		prenetty_window_click_1_2_5,
		prenetty_block_place_1_2_5,
		v1_3_2,
		v1_2_5,
		slot_down
	);
}

pub use down::*;
pub use up::*;

pub fn prenetty_metadata_item_1_2_5(v: OldMetaItem, _cx: &mut Session) -> Result<OldMetaItem> {
	Ok(old_meta_item(v, TO))
}

pub fn prenetty_entity_equipment_1_3_2(
	p: v1_2_5::play_toClient_packet_entity_equipment::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client132>> {
	let item = if p.itemId < 0 {
		None
	} else {
		Some(Item {
			id: p.itemId,
			itemCount: 1,
			itemDamage: p.itemDamage,
			nbtData: None,
		})
	};
	Ok(vec![Client132::entity_equipment(
		v1_3_2::play_toClient_packet_entity_equipment::Ty {
			entityId: p.entityId,
			slot: p.slot,
			item: slot_up(item),
		},
	)])
}

pub fn prenetty_entity_equipment_1_2_5(
	p: v1_3_2::play_toClient_packet_entity_equipment::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	let (item_id, item_damage) = match slot_down(p.item) {
		Some(item) => (item.id, item.itemDamage),
		None => (-1, 0),
	};
	Ok(vec![Client125::entity_equipment(
		v1_2_5::play_toClient_packet_entity_equipment::Ty {
			entityId: p.entityId,
			slot: p.slot,
			itemId: item_id,
			itemDamage: item_damage,
		},
	)])
}

pub fn prenetty_named_entity_spawn_1_2_5(
	p: v1_3_2::play_toClient_packet_named_entity_spawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	Ok(vec![Client125::named_entity_spawn(
		v1_2_5::play_toClient_packet_named_entity_spawn::Ty {
			entityId: p.entityId,
			playerName: p.playerName,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			currentItem: item_id(p.currentItem, TO),
		},
	)])
}

pub fn prenetty_spawn_entity_item_1_2_5(
	mut p: v1_3_2::play_toClient_packet_spawn_entity_item::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	if p.item.id >= 0 {
		(p.item.id, p.item.itemDamage) = ids::item(p.item.id, p.item.itemDamage, TO);
	}
	Ok(vec![Client125::spawn_entity_item(p)])
}

pub fn prenetty_entity_destroy_1_3_2(id: i32, _cx: &mut Session) -> Result<Vec<Client132>> {
	Ok(vec![Client132::entity_destroy(vec![id])])
}

pub fn prenetty_entity_destroy_1_2_5(ids: Vec<i32>, _cx: &mut Session) -> Result<Vec<Client125>> {
	Ok(ids.into_iter().map(Client125::entity_destroy).collect())
}

fn track_column(cx: &mut Session, x: i32, z: i32, data: Option<(&[u8], Shape)>) {
	let events = &mut state(cx).events;
	events.retain(|pos, _| (pos.0 >> 4, pos.2 >> 4) != (x, z));
	let Some((raw, shape)) = data else { return };
	for (k, s) in (0..16).filter(|s| shape.bitmap & 1 << s != 0).enumerate() {
		let base = k * chunk::BLOCKS;
		let Some(blocks) = raw.get(base..base + chunk::BLOCKS) else {
			return;
		};
		for (i, id) in blocks.iter().enumerate() {
			if EVENT_BLOCKS.contains(id) {
				let pos = (
					x * 16 + (i & 15) as i32,
					(s * 16 + (i >> 8)) as i32,
					z * 16 + (i >> 4 & 15) as i32,
				);
				events.insert(pos, *id as u16);
			}
		}
	}
}

fn track_block(cx: &mut Session, pos: (i32, i32, i32), id: u16) {
	let events = &mut state(cx).events;
	if id < 256 && EVENT_BLOCKS.contains(&(id as u8)) {
		events.insert(pos, id);
	} else {
		events.remove(&pos);
	}
}

pub fn prenetty_pre_chunk_1_3_2(
	p: v1_2_5::play_toClient_packet_pre_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	if p.mode {
		return Ok(vec![]);
	}
	track_column(cx, p.x, p.z, None);
	Ok(vec![Client132::map_chunk(
		v1_3_2::play_toClient_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			groundUp: true,
			bitMap: 0,
			addBitMap: 0,
			compressedChunkData: chunk::deflate(&[])?,
		},
	)])
}

pub fn prenetty_map_chunk_1_3_2(
	p: v1_2_5::play_toClient_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	if p.groundUp && p.bitMap == 0 {
		track_column(cx, p.x, p.z, None);
		return Ok(vec![Client132::map_chunk(
			v1_3_2::play_toClient_packet_map_chunk::Ty {
				x: p.x,
				z: p.z,
				groundUp: true,
				bitMap: 1,
				addBitMap: 0,
				compressedChunkData: chunk::deflate(&chunk::empty_column(true))?,
			},
		)]);
	}
	let raw = chunk::inflate(&p.compressedChunkData)?;
	let shape = Shape {
		bitmap: p.bitMap,
		add: p.addBitMap,
		sky: true,
		full: p.groundUp,
	};
	if p.groundUp {
		track_column(cx, p.x, p.z, Some((&raw, shape)));
	} else {
		let events = &mut state(cx).events;
		let (x, z) = (p.x, p.z);
		events.retain(|pos, _| {
			(pos.0 >> 4, pos.2 >> 4) != (x, z) || p.bitMap & 1 << (pos.1 >> 4) == 0
		});
		for (k, s) in (0..16).filter(|s| p.bitMap & 1 << s != 0).enumerate() {
			let base = k * chunk::BLOCKS;
			if let Some(blocks) = raw.get(base..base + chunk::BLOCKS) {
				for (i, id) in blocks.iter().enumerate() {
					if EVENT_BLOCKS.contains(id) {
						let pos = (
							x * 16 + (i & 15) as i32,
							(s * 16 + (i >> 8)) as i32,
							z * 16 + (i >> 4 & 15) as i32,
						);
						events.insert(pos, *id as u16);
					}
				}
			}
		}
	}
	Ok(vec![Client132::map_chunk(
		v1_3_2::play_toClient_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			groundUp: p.groundUp,
			bitMap: p.bitMap,
			addBitMap: p.addBitMap,
			compressedChunkData: p.compressedChunkData,
		},
	)])
}

fn column_1_2_5(
	x: i32,
	z: i32,
	ground_up: bool,
	bitmap: u16,
	add: u16,
	data: Vec<u8>,
) -> Vec<Client125> {
	if ground_up && bitmap == 0 {
		return vec![Client125::pre_chunk(
			v1_2_5::play_toClient_packet_pre_chunk::Ty { x, z, mode: false },
		)];
	}
	let mut out = Vec::with_capacity(2);
	if ground_up {
		out.push(Client125::pre_chunk(
			v1_2_5::play_toClient_packet_pre_chunk::Ty { x, z, mode: true },
		));
	}
	out.push(Client125::map_chunk(
		v1_2_5::play_toClient_packet_map_chunk::Ty {
			x,
			z,
			groundUp: ground_up,
			bitMap: bitmap,
			addBitMap: add,
			unused: 0,
			compressedChunkData: data,
		},
	));
	out
}

pub fn prenetty_map_chunk_1_2_5(
	p: v1_3_2::play_toClient_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	if p.groundUp && p.bitMap == 0 {
		return Ok(column_1_2_5(p.x, p.z, true, 0, 0, vec![]));
	}
	let raw = chunk::inflate(&p.compressedChunkData)?;
	let mut shape = Shape {
		bitmap: p.bitMap,
		add: p.addBitMap,
		sky: true,
		full: p.groundUp,
	};
	if raw.len() < shape.len() {
		shape.sky = false;
	}
	let mut raw = raw;
	chunk::remap(&mut raw, shape, TO)?;
	Ok(column_1_2_5(
		p.x,
		p.z,
		p.groundUp,
		p.bitMap,
		p.addBitMap,
		chunk::deflate(&raw)?,
	))
}

pub fn prenetty_map_chunk_bulk_1_2_5(
	p: v1_3_2::play_toClient_packet_map_chunk_bulk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	let raw = chunk::inflate(&p.compressedChunkData)?;
	let columns: Vec<_> = p
		.meta
		.iter()
		.map(|m| (m.x, m.z, m.bitMap, m.addBitMap))
		.collect();
	let mut out = Vec::new();
	for (x, z, bitmap, add, mut data) in chunk::split(&raw, &columns, true)? {
		let shape = Shape {
			bitmap,
			add,
			sky: true,
			full: true,
		};
		chunk::remap(&mut data, shape, TO)?;
		out.extend(column_1_2_5(
			x,
			z,
			true,
			bitmap,
			add,
			chunk::deflate(&data)?,
		));
	}
	Ok(out)
}

pub fn prenetty_block_change_1_3_2(
	p: v1_2_5::play_toClient_packet_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	track_block(
		cx,
		(p.location.x, p.location.y as i32, p.location.z),
		p.r#type as u16,
	);
	Ok(vec![Client132::block_change(
		v1_3_2::play_toClient_packet_block_change::Ty {
			location: p.location,
			r#type: p.r#type as i16,
			metadata: p.metadata,
		},
	)])
}

pub fn prenetty_block_change_1_2_5(
	p: v1_3_2::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	let (id, meta) = ids::block(p.r#type as u16, p.metadata, TO);
	Ok(vec![Client125::block_change(
		v1_2_5::play_toClient_packet_block_change::Ty {
			location: p.location,
			r#type: id as u8,
			metadata: meta,
		},
	)])
}

pub fn prenetty_multi_block_change_1_3_2(
	p: v1_2_5::play_toClient_packet_multi_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	for r in &p.records {
		let pos = (
			p.chunkX * 16 + (r.horizontalPos >> 4) as i32,
			r.y as i32,
			p.chunkZ * 16 + (r.horizontalPos & 15) as i32,
		);
		track_block(cx, pos, r.blockId >> 4);
	}
	Ok(vec![Client132::multi_block_change(p)])
}

pub fn prenetty_multi_block_change_1_2_5(
	mut p: v1_3_2::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	for r in &mut p.records {
		r.blockId = super::remap_record(r.blockId, TO);
	}
	Ok(vec![Client125::multi_block_change(p)])
}

pub fn prenetty_block_action_1_3_2(
	p: v1_2_5::play_toClient_packet_block_action::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	let pos = (p.location.x, p.location.y as i32, p.location.z);
	let block = state(cx)
		.events
		.get(&pos)
		.copied()
		.unwrap_or(if p.byte1 == 1 && p.byte2 > 5 { 54 } else { 25 });
	Ok(vec![Client132::block_action(
		v1_3_2::play_toClient_packet_block_action::Ty {
			location: p.location,
			byte1: p.byte1,
			byte2: p.byte2,
			blockId: block as i16,
		},
	)])
}

pub fn prenetty_block_action_1_2_5(
	p: v1_3_2::play_toClient_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	Ok(vec![Client125::block_action(
		v1_2_5::play_toClient_packet_block_action::Ty {
			location: p.location,
			byte1: p.byte1,
			byte2: p.byte2,
		},
	)])
}

pub fn prenetty_spawn_entity_1_3_2(
	mut p: v1_2_5::play_toClient_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client132>> {
	let block = match p.r#type {
		70 => Some(12),
		71 => Some(13),
		74 => Some(122),
		_ => None,
	};
	if let Some(block) = block {
		p.r#type = 70;
		let velocity = match p.objectData {
			Object125::default(_, v) => v,
			_ => v1_2_5::play_toClient_packet_spawn_entity__objectData__default::Ty {
				velocityX: 0,
				velocityY: 0,
				velocityZ: 0,
			},
		};
		p.objectData = Object125::default(block, velocity);
	}
	Ok(vec![Client132::spawn_entity(p)])
}

pub fn prenetty_spawn_entity_1_2_5(
	mut p: v1_3_2::play_toClient_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	if p.r#type == 70 {
		let data = match &p.objectData {
			Object125::default(data, _) => falling_block(*data, TO, false),
			_ => 12,
		};
		p.r#type = match data {
			13 => 71,
			122 => 74,
			_ => 70,
		};
		p.objectData = Object125::v0;
	}
	if !ids::object_exists(p.r#type, TO) {
		return Ok(vec![]);
	}
	Ok(vec![Client125::spawn_entity(p)])
}

pub fn prenetty_world_event_1_2_5(
	mut p: v1_3_2::play_toClient_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	p.data = world_event_data(p.effectId, p.data, TO);
	Ok(vec![Client125::world_event(p)])
}

pub fn prenetty_abilities_1_3_2(
	p: v1_2_5::play_toClient_packet_abilities::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client132>> {
	Ok(vec![Client132::abilities(flags_up(
		p.invulnerable,
		p.flying,
		p.canFly,
		p.instantBuild,
	))])
}

pub fn prenetty_abilities_1_2_5(
	p: v1_3_2::play_toClient_packet_abilities::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	let f = p.flags;
	Ok(vec![Client125::abilities(
		v1_2_5::play_toClient_packet_abilities::Ty {
			invulnerable: f & 1 != 0,
			flying: f & 2 != 0,
			canFly: f & 4 != 0,
			instantBuild: f & 8 != 0,
		},
	)])
}

pub fn prenetty_abilities_server_1_3_2(
	p: v1_2_5::play_toServer_packet_abilities::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server132>> {
	let a = flags_up(p.invulnerable, p.flying, p.canFly, p.instantBuild);
	Ok(vec![Server132::abilities(
		v1_3_2::play_toServer_packet_abilities::Ty {
			flags: a.flags,
			flyingSpeed: a.flyingSpeed,
			walkingSpeed: a.walkingSpeed,
		},
	)])
}

pub fn prenetty_abilities_server_1_2_5(
	p: v1_3_2::play_toServer_packet_abilities::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server125>> {
	let f = p.flags;
	Ok(vec![Server125::abilities(
		v1_2_5::play_toServer_packet_abilities::Ty {
			invulnerable: f & 1 != 0,
			flying: f & 2 != 0,
			canFly: f & 4 != 0,
			instantBuild: f & 8 != 0,
		},
	)])
}

fn flags_up(
	invulnerable: bool,
	flying: bool,
	can_fly: bool,
	creative: bool,
) -> v1_3_2::play_toClient_packet_abilities::Ty {
	let flags =
		invulnerable as i8 | (flying as i8) << 1 | (can_fly as i8) << 2 | (creative as i8) << 3;
	v1_3_2::play_toClient_packet_abilities::Ty {
		flags,
		flyingSpeed: (0.05f32 * 255.0) as i8,
		walkingSpeed: (0.1f32 * 255.0) as i8,
	}
}

fn spawner_nbt(x: i32, y: i32, z: i32, entity: &str) -> Entry {
	Entry::new(
		"",
		vec![
			Entry::new("id", "MobSpawner"),
			Entry::new("EntityId", entity),
			Entry::new("Delay", 20i16),
			Entry::new("x", x),
			Entry::new("y", y),
			Entry::new("z", z),
		],
	)
}

fn entity_id(data: &[u8]) -> Result<Option<String>> {
	match nbt::gzip::read(data) {
		Ok(tag) => Ok(tag
			.value
			.field("EntityId")
			.and_then(Payload::as_str)
			.map(str::to_owned)),
		Err(nbt::gzip::Error::Io(e)) => Err(Error::Encoding(format!("bad gzip nbt: {e}"))),
		Err(nbt::gzip::Error::Nbt(_)) => Ok(None),
	}
}

pub fn prenetty_tile_entity_data_1_3_2(
	p: v1_2_5::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client132>> {
	if p.action != 1 {
		return Ok(vec![]);
	}
	let name = ENTITY_NAMES
		.iter()
		.find(|(_, id)| *id == p.custom1)
		.map_or("Pig", |(name, _)| *name);
	let nbt = spawner_nbt(p.location.x, p.location.y as i32, p.location.z, name);
	Ok(vec![Client132::tile_entity_data(
		v1_3_2::play_toClient_packet_tile_entity_data::Ty {
			location: p.location,
			action: p.action,
			nbtData: Some(nbt::gzip::write(&nbt).map_err(|e| Error::Encoding(e.to_string()))?),
		},
	)])
}

pub fn prenetty_tile_entity_data_1_2_5(
	p: v1_3_2::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	if p.action != 1 {
		return Ok(vec![]);
	}
	let Some(nbt) = p.nbtData else {
		return Ok(vec![]);
	};
	let Some(name) = entity_id(&nbt)? else {
		return Ok(vec![]);
	};
	let Some((_, id)) = ENTITY_NAMES.iter().find(|(n, _)| *n == name) else {
		return Ok(vec![]);
	};
	Ok(vec![Client125::tile_entity_data(
		v1_2_5::play_toClient_packet_tile_entity_data::Ty {
			location: p.location,
			action: p.action,
			custom1: *id,
			custom2: 0,
			custom3: 0,
		},
	)])
}

pub fn prenetty_set_protocol_1_3_2(
	p: v1_2_5::play_toServer_packet_set_protocol::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server132>> {
	let (username, address) = p
		.usernameAndHost
		.split_once(';')
		.unwrap_or((&p.usernameAndHost, ""));
	let (host, port) = address.rsplit_once(':').unwrap_or((address, "25565"));
	Ok(vec![Server132::set_protocol(
		v1_3_2::play_toServer_packet_set_protocol::Ty {
			protocolVersion: 39,
			username: username.to_owned(),
			serverHost: host.to_owned(),
			serverPort: port.parse().unwrap_or(25565),
		},
	)])
}

pub fn prenetty_set_protocol_1_2_5(
	p: v1_3_2::play_toServer_packet_set_protocol::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server125>> {
	Ok(vec![Server125::set_protocol(
		v1_2_5::play_toServer_packet_set_protocol::Ty {
			usernameAndHost: format!("{};{}:{}", p.username, p.serverHost, p.serverPort),
		},
	)])
}

pub fn prenetty_client_command_1_2_5(
	p: v1_3_2::play_toServer_packet_client_command::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server125>> {
	if !matches!(
		p,
		v1_3_2::play_toServer_packet_client_command::Ty::perform_respawn
	) {
		return Ok(vec![]);
	}
	Ok(vec![Server125::respawn(
		v1_2_5::play_toServer_packet_respawn::Ty {
			dimension: 0,
			difficulty: 0,
			gamemode: 0,
			worldHeight: 0,
			levelType: String::new(),
		},
	)])
}

pub fn prenetty_respawn_server_1_3_2(
	_p: v1_2_5::play_toServer_packet_respawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server132>> {
	Ok(vec![Server132::client_command(
		v1_3_2::play_toServer_packet_client_command::Ty::perform_respawn,
	)])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn spawner_nbt_round_trips_its_entity() {
		let nbt = spawner_nbt(1, 2, 3, "Zombie");
		let gz = nbt::gzip::write(&nbt).unwrap();
		assert_eq!(entity_id(&gz).unwrap().as_deref(), Some("Zombie"));
		assert_eq!(
			nbt.value.field("id").and_then(Payload::as_str),
			Some("MobSpawner")
		);
		let mut raw = vec![10, 0, 0, 8, 0, 2, b'i', b'd', 0, 10];
		raw.extend(b"MobSpawner");
		raw.extend([8, 0, 8]);
		raw.extend(b"EntityId");
		raw.extend([0, 6]);
		raw.extend(b"Zombie");
		raw.extend([2, 0, 5]);
		raw.extend(b"Delay");
		raw.extend([
			0, 20, 3, 0, 1, b'x', 0, 0, 0, 1, 3, 0, 1, b'y', 0, 0, 0, 2, 3, 0, 1, b'z', 0, 0, 0, 3,
			0,
		]);
		assert_eq!(nbt::gzip::decompress(&gz).unwrap(), raw);
		assert!(entity_id(b"not gzip").is_err());
		assert_eq!(
			entity_id(&nbt::gzip::compress(&[10, 0, 0, 99]).unwrap()).unwrap(),
			None
		);
	}

	#[test]
	fn handshake_strings_split_into_fields() {
		let mut cx = Session::new();
		let p = v1_2_5::play_toServer_packet_set_protocol::Ty {
			usernameAndHost: "Notch;example.com:25566".into(),
		};
		let Server132::set_protocol(hs) =
			super::super::one(prenetty_set_protocol_1_3_2(p, &mut cx).unwrap())
		else {
			panic!()
		};
		assert_eq!(
			(hs.username.as_str(), hs.serverHost.as_str(), hs.serverPort),
			("Notch", "example.com", 25566)
		);
		let Server125::set_protocol(back) =
			super::super::one(prenetty_set_protocol_1_2_5(hs, &mut cx).unwrap())
		else {
			panic!()
		};
		assert_eq!(back.usernameAndHost, "Notch;example.com:25566");
	}
}
