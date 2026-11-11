use crate::gen::convert::Version;
use bungeeoxide_nbt::{Entry, Payload};

use super::chunk::{self, Shape};
use super::ids;
use super::{
	falling_block, item_id, legacy_slot, old_meta_item, state, world_event_data, LegacySlot,
	OldMetaItem, Slot,
};
use crate::gen::convert::{v1_2_5, v1_3_2};
use crate::pds::convert::{Error, Result};
use crate::pds_native::pds_legacySlot::{self, Item};
use crate::session::Session;

type Client125 = v1_2_5::play_to_client_packet::Ty;
type Client132 = v1_3_2::play_to_client_packet::Ty;
type Server125 = v1_2_5::play_to_server_packet::Ty;
type Server132 = v1_3_2::play_to_server_packet::Ty;
type Object125 = v1_2_5::play_to_client_packet_spawn_entity_object_data::Ty;

const TO: Version = Version::V1_2_5;
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
		Some(item) => Slot::Default(
			item.id,
			v1_3_2::slot_default::Ty {
				item_count: item.itemCount,
				item_damage: item.itemDamage,
				nbt_data: item.nbtData,
			},
		),
	}
}

fn slot_down(v: Slot) -> LegacySlot {
	match v {
		Slot::Default(id, item) if id >= 0 => {
			let mut out = legacy_slot(
				Some(Item {
					id,
					itemCount: item.item_count,
					itemDamage: item.item_damage,
					nbtData: item.nbt_data,
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
			p: $from::play_to_client_packet_set_slot::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_to_client_packet::Ty>> {
			Ok(vec![$to::play_to_client_packet::Ty::SetSlot(
				$to::play_to_client_packet_set_slot::Ty {
					window_id: p.window_id,
					slot: p.slot,
					item: $conv(p.item),
				},
			)])
		}

		pub fn $items(
			p: $from::play_to_client_packet_window_items::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_to_client_packet::Ty>> {
			Ok(vec![$to::play_to_client_packet::Ty::WindowItems(
				$to::play_to_client_packet_window_items::Ty {
					window_id: p.window_id,
					items: p.items.into_iter().map($conv).collect(),
				},
			)])
		}

		pub fn $creative(
			p: $from::play_to_client_packet_set_creative_slot::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_to_client_packet::Ty>> {
			Ok(vec![$to::play_to_client_packet::Ty::SetCreativeSlot(
				$to::play_to_client_packet_set_creative_slot::Ty {
					slot: p.slot,
					item: $conv(p.item),
				},
			)])
		}

		pub fn $creative_server(
			p: $from::play_to_server_packet_set_creative_slot::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_to_server_packet::Ty>> {
			Ok(vec![$to::play_to_server_packet::Ty::SetCreativeSlot(
				$to::play_to_server_packet_set_creative_slot::Ty {
					slot: p.slot,
					item: $conv(p.item),
				},
			)])
		}

		pub fn $click(
			p: $from::play_to_server_packet_window_click::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_to_server_packet::Ty>> {
			Ok(vec![$to::play_to_server_packet::Ty::WindowClick(
				$to::play_to_server_packet_window_click::Ty {
					window_id: p.window_id,
					slot: p.slot,
					mouse_button: p.mouse_button,
					action: p.action,
					mode: p.mode,
					item: $conv(p.item),
				},
			)])
		}

		pub fn $place(
			p: $from::play_to_server_packet_block_place::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_to_server_packet::Ty>> {
			Ok(vec![$to::play_to_server_packet::Ty::BlockPlace(place(
				p.location,
				p.direction,
				$conv(p.held_item),
			))])
		}
	};
}

fn place_up(
	location: v1_2_5::position_ibi::Ty,
	direction: i8,
	held: Slot,
) -> v1_3_2::play_to_server_packet_block_place::Ty {
	v1_3_2::play_to_server_packet_block_place::Ty {
		location,
		direction,
		held_item: held,
		cursor_x: 8,
		cursor_y: 8,
		cursor_z: 8,
	}
}

fn place_down(
	location: v1_2_5::position_ibi::Ty,
	direction: i8,
	held: LegacySlot,
) -> v1_2_5::play_to_server_packet_block_place::Ty {
	v1_2_5::play_to_server_packet_block_place::Ty {
		location,
		direction,
		held_item: held,
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
	p: v1_2_5::play_to_client_packet_entity_equipment::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client132>> {
	let item = if p.item_id < 0 {
		None
	} else {
		Some(Item {
			id: p.item_id,
			itemCount: 1,
			itemDamage: p.item_damage,
			nbtData: None,
		})
	};
	Ok(vec![Client132::EntityEquipment(
		v1_3_2::play_to_client_packet_entity_equipment::Ty {
			entity_id: p.entity_id,
			slot: p.slot,
			item: slot_up(item),
		},
	)])
}

pub fn prenetty_entity_equipment_1_2_5(
	p: v1_3_2::play_to_client_packet_entity_equipment::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	let (item_id, item_damage) = match slot_down(p.item) {
		Some(item) => (item.id, item.itemDamage),
		None => (-1, 0),
	};
	Ok(vec![Client125::EntityEquipment(
		v1_2_5::play_to_client_packet_entity_equipment::Ty {
			entity_id: p.entity_id,
			slot: p.slot,
			item_id: item_id,
			item_damage: item_damage,
		},
	)])
}

pub fn prenetty_named_entity_spawn_1_2_5(
	p: v1_3_2::play_to_client_packet_named_entity_spawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	Ok(vec![Client125::NamedEntitySpawn(
		v1_2_5::play_to_client_packet_named_entity_spawn::Ty {
			entity_id: p.entity_id,
			player_name: p.player_name,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			current_item: item_id(p.current_item, TO),
		},
	)])
}

pub fn prenetty_spawn_entity_item_1_2_5(
	mut p: v1_3_2::play_to_client_packet_spawn_entity_item::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	if p.item.id >= 0 {
		(p.item.id, p.item.item_damage) = ids::item(p.item.id, p.item.item_damage, TO);
	}
	Ok(vec![Client125::SpawnEntityItem(p)])
}

pub fn prenetty_entity_destroy_1_3_2(id: i32, _cx: &mut Session) -> Result<Vec<Client132>> {
	Ok(vec![Client132::EntityDestroy(vec![id])])
}

pub fn prenetty_entity_destroy_1_2_5(ids: Vec<i32>, _cx: &mut Session) -> Result<Vec<Client125>> {
	Ok(ids.into_iter().map(Client125::EntityDestroy).collect())
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
	p: v1_2_5::play_to_client_packet_pre_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	if p.mode {
		return Ok(vec![]);
	}
	track_column(cx, p.x, p.z, None);
	Ok(vec![Client132::MapChunk(
		v1_3_2::play_to_client_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			ground_up: true,
			bit_map: 0,
			add_bit_map: 0,
			compressed_chunk_data: chunk::deflate(&[])?,
		},
	)])
}

pub fn prenetty_map_chunk_1_3_2(
	p: v1_2_5::play_to_client_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	if p.ground_up && p.bit_map == 0 {
		track_column(cx, p.x, p.z, None);
		return Ok(vec![Client132::MapChunk(
			v1_3_2::play_to_client_packet_map_chunk::Ty {
				x: p.x,
				z: p.z,
				ground_up: true,
				bit_map: 1,
				add_bit_map: 0,
				compressed_chunk_data: chunk::deflate(&chunk::empty_column(true))?,
			},
		)]);
	}
	let raw = chunk::inflate(&p.compressed_chunk_data)?;
	let shape = Shape {
		bitmap: p.bit_map,
		add: p.add_bit_map,
		sky: true,
		full: p.ground_up,
	};
	if p.ground_up {
		track_column(cx, p.x, p.z, Some((&raw, shape)));
	} else {
		let events = &mut state(cx).events;
		let (x, z) = (p.x, p.z);
		events.retain(|pos, _| {
			(pos.0 >> 4, pos.2 >> 4) != (x, z) || p.bit_map & 1 << (pos.1 >> 4) == 0
		});
		for (k, s) in (0..16).filter(|s| p.bit_map & 1 << s != 0).enumerate() {
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
	Ok(vec![Client132::MapChunk(
		v1_3_2::play_to_client_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			ground_up: p.ground_up,
			bit_map: p.bit_map,
			add_bit_map: p.add_bit_map,
			compressed_chunk_data: p.compressed_chunk_data,
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
		return vec![Client125::PreChunk(
			v1_2_5::play_to_client_packet_pre_chunk::Ty { x, z, mode: false },
		)];
	}
	let mut out = Vec::with_capacity(2);
	if ground_up {
		out.push(Client125::PreChunk(
			v1_2_5::play_to_client_packet_pre_chunk::Ty { x, z, mode: true },
		));
	}
	out.push(Client125::MapChunk(
		v1_2_5::play_to_client_packet_map_chunk::Ty {
			x,
			z,
			ground_up: ground_up,
			bit_map: bitmap,
			add_bit_map: add,
			unused: 0,
			compressed_chunk_data: data,
		},
	));
	out
}

pub fn prenetty_map_chunk_1_2_5(
	p: v1_3_2::play_to_client_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	if p.ground_up && p.bit_map == 0 {
		return Ok(column_1_2_5(p.x, p.z, true, 0, 0, vec![]));
	}
	let raw = chunk::inflate(&p.compressed_chunk_data)?;
	let mut shape = Shape {
		bitmap: p.bit_map,
		add: p.add_bit_map,
		sky: true,
		full: p.ground_up,
	};
	if raw.len() < shape.len() {
		shape.sky = false;
	}
	let mut raw = raw;
	chunk::remap(&mut raw, shape, TO)?;
	Ok(column_1_2_5(
		p.x,
		p.z,
		p.ground_up,
		p.bit_map,
		p.add_bit_map,
		chunk::deflate(&raw)?,
	))
}

pub fn prenetty_map_chunk_bulk_1_2_5(
	p: v1_3_2::play_to_client_packet_map_chunk_bulk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	let raw = chunk::inflate(&p.compressed_chunk_data)?;
	let columns: Vec<_> = p
		.meta
		.iter()
		.map(|m| (m.x, m.z, m.bit_map, m.add_bit_map))
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
	p: v1_2_5::play_to_client_packet_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	track_block(
		cx,
		(p.location.x, p.location.y as i32, p.location.z),
		p.r#type as u16,
	);
	Ok(vec![Client132::BlockChange(
		v1_3_2::play_to_client_packet_block_change::Ty {
			location: p.location,
			r#type: p.r#type as i16,
			metadata: p.metadata,
		},
	)])
}

pub fn prenetty_block_change_1_2_5(
	p: v1_3_2::play_to_client_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	let (id, meta) = ids::block(p.r#type as u16, p.metadata, TO);
	Ok(vec![Client125::BlockChange(
		v1_2_5::play_to_client_packet_block_change::Ty {
			location: p.location,
			r#type: id as u8,
			metadata: meta,
		},
	)])
}

pub fn prenetty_multi_block_change_1_3_2(
	p: v1_2_5::play_to_client_packet_multi_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	for r in &p.records {
		let pos = (
			p.chunk_x * 16 + (r.horizontal_pos >> 4) as i32,
			r.y as i32,
			p.chunk_z * 16 + (r.horizontal_pos & 15) as i32,
		);
		track_block(cx, pos, r.block_id >> 4);
	}
	Ok(vec![Client132::MultiBlockChange(p)])
}

pub fn prenetty_multi_block_change_1_2_5(
	mut p: v1_3_2::play_to_client_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	for r in &mut p.records {
		r.block_id = super::remap_record(r.block_id, TO);
	}
	Ok(vec![Client125::MultiBlockChange(p)])
}

pub fn prenetty_block_action_1_3_2(
	p: v1_2_5::play_to_client_packet_block_action::Ty,
	cx: &mut Session,
) -> Result<Vec<Client132>> {
	let pos = (p.location.x, p.location.y as i32, p.location.z);
	let block = state(cx)
		.events
		.get(&pos)
		.copied()
		.unwrap_or(if p.byte1 == 1 && p.byte2 > 5 { 54 } else { 25 });
	Ok(vec![Client132::BlockAction(
		v1_3_2::play_to_client_packet_block_action::Ty {
			location: p.location,
			byte1: p.byte1,
			byte2: p.byte2,
			block_id: block as i16,
		},
	)])
}

pub fn prenetty_block_action_1_2_5(
	p: v1_3_2::play_to_client_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	Ok(vec![Client125::BlockAction(
		v1_2_5::play_to_client_packet_block_action::Ty {
			location: p.location,
			byte1: p.byte1,
			byte2: p.byte2,
		},
	)])
}

pub fn prenetty_spawn_entity_1_3_2(
	mut p: v1_2_5::play_to_client_packet_spawn_entity::Ty,
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
		let velocity = match p.object_data {
			Object125::Default(_, v) => v,
			_ => v1_2_5::play_to_client_packet_spawn_entity_object_data_default::Ty {
				velocity_x: 0,
				velocity_y: 0,
				velocity_z: 0,
			},
		};
		p.object_data = Object125::Default(block, velocity);
	}
	Ok(vec![Client132::SpawnEntity(p)])
}

pub fn prenetty_spawn_entity_1_2_5(
	mut p: v1_3_2::play_to_client_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	if p.r#type == 70 {
		let data = match &p.object_data {
			Object125::Default(data, _) => falling_block(*data, TO, false),
			_ => 12,
		};
		p.r#type = match data {
			13 => 71,
			122 => 74,
			_ => 70,
		};
		p.object_data = Object125::V0;
	}
	if !ids::object_exists(p.r#type, TO) {
		return Ok(vec![]);
	}
	Ok(vec![Client125::SpawnEntity(p)])
}

pub fn prenetty_world_event_1_2_5(
	mut p: v1_3_2::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	p.data = world_event_data(p.effect_id, p.data, TO);
	Ok(vec![Client125::WorldEvent(p)])
}

pub fn prenetty_abilities_1_3_2(
	p: v1_2_5::play_to_client_packet_abilities::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client132>> {
	Ok(vec![Client132::Abilities(flags_up(
		p.invulnerable,
		p.flying,
		p.can_fly,
		p.instant_build,
	))])
}

pub fn prenetty_abilities_1_2_5(
	p: v1_3_2::play_to_client_packet_abilities::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	let f = p.flags;
	Ok(vec![Client125::Abilities(
		v1_2_5::play_to_client_packet_abilities::Ty {
			invulnerable: f & 1 != 0,
			flying: f & 2 != 0,
			can_fly: f & 4 != 0,
			instant_build: f & 8 != 0,
		},
	)])
}

pub fn prenetty_abilities_server_1_3_2(
	p: v1_2_5::play_to_server_packet_abilities::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server132>> {
	let a = flags_up(p.invulnerable, p.flying, p.can_fly, p.instant_build);
	Ok(vec![Server132::Abilities(
		v1_3_2::play_to_server_packet_abilities::Ty {
			flags: a.flags,
			flying_speed: a.flying_speed,
			walking_speed: a.walking_speed,
		},
	)])
}

pub fn prenetty_abilities_server_1_2_5(
	p: v1_3_2::play_to_server_packet_abilities::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server125>> {
	let f = p.flags;
	Ok(vec![Server125::Abilities(
		v1_2_5::play_to_server_packet_abilities::Ty {
			invulnerable: f & 1 != 0,
			flying: f & 2 != 0,
			can_fly: f & 4 != 0,
			instant_build: f & 8 != 0,
		},
	)])
}

fn flags_up(
	invulnerable: bool,
	flying: bool,
	can_fly: bool,
	creative: bool,
) -> v1_3_2::play_to_client_packet_abilities::Ty {
	let flags =
		invulnerable as i8 | (flying as i8) << 1 | (can_fly as i8) << 2 | (creative as i8) << 3;
	v1_3_2::play_to_client_packet_abilities::Ty {
		flags,
		flying_speed: (0.05f32 * 255.0) as i8,
		walking_speed: (0.1f32 * 255.0) as i8,
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
	match bungeeoxide_nbt::gzip::read(data) {
		Ok(tag) => Ok(tag
			.value
			.field("EntityId")
			.and_then(Payload::as_str)
			.map(str::to_owned)),
		Err(bungeeoxide_nbt::gzip::Error::Io(e)) => {
			Err(Error::Encoding(format!("bad gzip nbt: {e}")))
		}
		Err(bungeeoxide_nbt::gzip::Error::Nbt(_)) => Ok(None),
	}
}

pub fn prenetty_tile_entity_data_1_3_2(
	p: v1_2_5::play_to_client_packet_tile_entity_data::Ty,
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
	Ok(vec![Client132::TileEntityData(
		v1_3_2::play_to_client_packet_tile_entity_data::Ty {
			location: p.location,
			action: p.action,
			nbt_data: Some(
				bungeeoxide_nbt::gzip::write(&nbt).map_err(|e| Error::Encoding(e.to_string()))?,
			),
		},
	)])
}

pub fn prenetty_tile_entity_data_1_2_5(
	p: v1_3_2::play_to_client_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client125>> {
	if p.action != 1 {
		return Ok(vec![]);
	}
	let Some(nbt) = p.nbt_data else {
		return Ok(vec![]);
	};
	let Some(name) = entity_id(&nbt)? else {
		return Ok(vec![]);
	};
	let Some((_, id)) = ENTITY_NAMES.iter().find(|(n, _)| *n == name) else {
		return Ok(vec![]);
	};
	Ok(vec![Client125::TileEntityData(
		v1_2_5::play_to_client_packet_tile_entity_data::Ty {
			location: p.location,
			action: p.action,
			custom1: *id,
			custom2: 0,
			custom3: 0,
		},
	)])
}

pub fn prenetty_set_protocol_1_3_2(
	p: v1_2_5::play_to_server_packet_set_protocol::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server132>> {
	let (username, address) = p
		.username_and_host
		.split_once(';')
		.unwrap_or((&p.username_and_host, ""));
	let (host, port) = address.rsplit_once(':').unwrap_or((address, "25565"));
	Ok(vec![Server132::SetProtocol(
		v1_3_2::play_to_server_packet_set_protocol::Ty {
			protocol_version: 39,
			username: username.to_owned(),
			server_host: host.to_owned(),
			server_port: port.parse().unwrap_or(25565),
		},
	)])
}

pub fn prenetty_set_protocol_1_2_5(
	p: v1_3_2::play_to_server_packet_set_protocol::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server125>> {
	Ok(vec![Server125::SetProtocol(
		v1_2_5::play_to_server_packet_set_protocol::Ty {
			username_and_host: format!("{};{}:{}", p.username, p.server_host, p.server_port),
		},
	)])
}

pub fn prenetty_client_command_1_2_5(
	p: v1_3_2::play_to_server_packet_client_command::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server125>> {
	if !matches!(
		p,
		v1_3_2::play_to_server_packet_client_command::Ty::PerformRespawn
	) {
		return Ok(vec![]);
	}
	Ok(vec![Server125::Respawn(
		v1_2_5::play_to_server_packet_respawn::Ty {
			dimension: 0,
			difficulty: 0,
			gamemode: 0,
			world_height: 0,
			level_type: String::new(),
		},
	)])
}

pub fn prenetty_respawn_server_1_3_2(
	_p: v1_2_5::play_to_server_packet_respawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server132>> {
	Ok(vec![Server132::ClientCommand(
		v1_3_2::play_to_server_packet_client_command::Ty::PerformRespawn,
	)])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn spawner_nbt_round_trips_its_entity() {
		let nbt = spawner_nbt(1, 2, 3, "Zombie");
		let gz = bungeeoxide_nbt::gzip::write(&nbt).unwrap();
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
		assert_eq!(bungeeoxide_nbt::gzip::decompress(&gz).unwrap(), raw);
		assert!(entity_id(b"not gzip").is_err());
		assert_eq!(
			entity_id(&bungeeoxide_nbt::gzip::compress(&[10, 0, 0, 99]).unwrap()).unwrap(),
			None
		);
	}

	#[test]
	fn handshake_strings_split_into_fields() {
		let mut cx = Session::new();
		let p = v1_2_5::play_to_server_packet_set_protocol::Ty {
			username_and_host: "Notch;example.com:25566".into(),
		};
		let Server132::SetProtocol(hs) =
			super::super::one(prenetty_set_protocol_1_3_2(p, &mut cx).unwrap())
		else {
			panic!()
		};
		assert_eq!(
			(
				hs.username.as_str(),
				hs.server_host.as_str(),
				hs.server_port
			),
			("Notch", "example.com", 25566)
		);
		let Server125::SetProtocol(back) =
			super::super::one(prenetty_set_protocol_1_2_5(hs, &mut cx).unwrap())
		else {
			panic!()
		};
		assert_eq!(back.username_and_host, "Notch;example.com:25566");
	}
}
