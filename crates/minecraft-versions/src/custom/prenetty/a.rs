use super::chunk::{self, Old, Shape};
use super::heights;
use super::ids::{self, V};
use super::{entities, item_id, legacy_slot, old_meta_item, Kind, OldMetaItem};
use crate::gen::convert::{v1_0_1, v1_1, v1_2_3};
use crate::pds::convert::Result;
use crate::session::Session;

macro_rules! legacy_items {
	($to:expr, $m:ident, $meta:ident, $equip:ident, $named:ident, $item:ident) => {
		pub fn $meta(v: OldMetaItem, _cx: &mut Session) -> Result<OldMetaItem> {
			Ok(old_meta_item(v, $to))
		}

		pub fn $equip(
			mut p: $m::play_toClient_packet_entity_equipment::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_toClient_packet::Ty>> {
			if p.itemId >= 0 {
				(p.itemId, p.itemDamage) = ids::item(p.itemId, p.itemDamage, $to);
			}
			Ok(vec![$m::play_toClient_packet::Ty::entity_equipment(p)])
		}

		pub fn $named(
			mut p: $m::play_toClient_packet_named_entity_spawn::Ty,
			cx: &mut Session,
		) -> Result<Vec<$m::play_toClient_packet::Ty>> {
			p.currentItem = item_id(p.currentItem, $to);
			if $to == V::R1_1 {
				heights::spawned(cx, p.entityId, (p.x, &mut p.y, p.z), Some((p.yaw, p.pitch)));
			}
			Ok(vec![$m::play_toClient_packet::Ty::named_entity_spawn(p)])
		}

		pub fn $item(
			mut p: $m::play_toClient_packet_spawn_entity_item::Ty,
			cx: &mut Session,
		) -> Result<Vec<$m::play_toClient_packet::Ty>> {
			(p.item.id, p.item.itemDamage) = ids::item(p.item.id, p.item.itemDamage, $to);
			if $to == V::R1_1 {
				heights::spawned(cx, p.entityId, (p.x, &mut p.y, p.z), None);
			}
			Ok(vec![$m::play_toClient_packet::Ty::spawn_entity_item(p)])
		}
	};
}

macro_rules! legacy_windows {
	($to:expr, $m:ident, $set_slot:ident, $items:ident, $creative:ident, $creative_server:ident, $click:ident, $place:ident) => {
		pub fn $set_slot(
			mut p: $m::play_toClient_packet_set_slot::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_toClient_packet::Ty>> {
			p.item = legacy_slot(p.item, $to);
			Ok(vec![$m::play_toClient_packet::Ty::set_slot(p)])
		}

		pub fn $items(
			mut p: $m::play_toClient_packet_window_items::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_toClient_packet::Ty>> {
			p.items = p.items.into_iter().map(|i| legacy_slot(i, $to)).collect();
			Ok(vec![$m::play_toClient_packet::Ty::window_items(p)])
		}

		pub fn $creative(
			mut p: $m::play_toClient_packet_set_creative_slot::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_toClient_packet::Ty>> {
			p.item = legacy_slot(p.item, $to);
			Ok(vec![$m::play_toClient_packet::Ty::set_creative_slot(p)])
		}

		pub fn $creative_server(
			mut p: $m::play_toServer_packet_set_creative_slot::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_toServer_packet::Ty>> {
			p.item = legacy_slot(p.item, $to);
			Ok(vec![$m::play_toServer_packet::Ty::set_creative_slot(p)])
		}

		pub fn $click(
			mut p: $m::play_toServer_packet_window_click::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_toServer_packet::Ty>> {
			p.item = legacy_slot(p.item, $to);
			Ok(vec![$m::play_toServer_packet::Ty::window_click(p)])
		}

		pub fn $place(
			mut p: $m::play_toServer_packet_block_place::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_toServer_packet::Ty>> {
			p.heldItem = legacy_slot(p.heldItem, $to);
			Ok(vec![$m::play_toServer_packet::Ty::block_place(p)])
		}
	};
}

legacy_windows!(
	V::R1_0,
	v1_0_1,
	prenetty_set_slot_1_0_1,
	prenetty_window_items_1_0_1,
	prenetty_set_creative_slot_1_0_1,
	prenetty_set_creative_slot_server_1_0_1,
	prenetty_window_click_1_0_1,
	prenetty_block_place_1_0_1
);

legacy_windows!(
	V::R1_1,
	v1_1,
	prenetty_set_slot_1_1,
	prenetty_window_items_1_1,
	prenetty_set_creative_slot_1_1,
	prenetty_set_creative_slot_server_1_1,
	prenetty_window_click_1_1,
	prenetty_block_place_1_1
);

legacy_windows!(
	V::R1_2_3,
	v1_2_3,
	prenetty_set_slot_1_2_3,
	prenetty_window_items_1_2_3,
	prenetty_set_creative_slot_1_2_3,
	prenetty_set_creative_slot_server_1_2_3,
	prenetty_window_click_1_2_3,
	prenetty_block_place_1_2_3
);

legacy_items!(
	V::R1_0,
	v1_0_1,
	prenetty_metadata_item_1_0_1,
	prenetty_entity_equipment_1_0_1,
	prenetty_named_entity_spawn_1_0_1,
	prenetty_spawn_entity_item_1_0_1
);

legacy_items!(
	V::R1_1,
	v1_1,
	prenetty_metadata_item_1_1,
	prenetty_entity_equipment_1_1,
	prenetty_named_entity_spawn_1_1,
	prenetty_spawn_entity_item_1_1
);

legacy_items!(
	V::R1_2_3,
	v1_2_3,
	prenetty_metadata_item_1_2_3,
	prenetty_entity_equipment_1_2_3,
	prenetty_named_entity_spawn_1_2_3,
	prenetty_spawn_entity_item_1_2_3
);

type Client11 = v1_1::play_toClient_packet::Ty;
type Client12 = v1_2_3::play_toClient_packet::Ty;
type Records = v1_2_3::play_toClient_packet_multi_block_change__records__items::Ty;

const MAX_RECORDS: usize = i16::MAX as usize;

fn records_packets(chunk_x: i32, chunk_z: i32, records: Vec<Records>) -> Vec<Client12> {
	records
		.chunks(MAX_RECORDS)
		.map(|part| {
			Client12::multi_block_change(v1_2_3::play_toClient_packet_multi_block_change::Ty {
				chunkX: chunk_x,
				chunkZ: chunk_z,
				dataLength: part.len() as i32 * 4,
				records: part.to_vec(),
			})
		})
		.collect()
}

fn record(x: i32, y: i32, z: i32, id: u8, meta: u8) -> Records {
	Records {
		horizontalPos: ((x & 15) << 4 | z & 15) as u8,
		y: y as u8,
		blockId: (id as u16) << 4 | (meta & 15) as u16,
	}
}

pub fn prenetty_map_chunk_1_2_3(
	p: v1_1::play_toClient_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client12>> {
	let old = Old {
		x: p.x,
		y: p.y,
		z: p.z,
		size_x: p.sizeX,
		size_y: p.sizeY,
		size_z: p.sizeZ,
		data: p.compressedChunkData,
	};
	let cuboid = chunk::from_old(&old, true)?;
	if let Some((x, z, data, bitmap)) = cuboid.full {
		return Ok(vec![Client12::map_chunk(
			v1_2_3::play_toClient_packet_map_chunk::Ty {
				x,
				z,
				groundUp: true,
				bitMap: bitmap,
				addBitMap: 0,
				unused: 0,
				compressedChunkData: chunk::deflate(&data)?,
			},
		)]);
	}
	let mut columns: std::collections::BTreeMap<(i32, i32), Vec<Records>> = Default::default();
	for (x, y, z, id, meta) in cuboid.blocks {
		if !(0..256).contains(&y) {
			continue;
		}
		columns
			.entry((x >> 4, z >> 4))
			.or_default()
			.push(record(x, y, z, id, meta));
	}
	Ok(columns
		.into_iter()
		.flat_map(|((cx, cz), records)| records_packets(cx, cz, records))
		.collect())
}

pub fn prenetty_map_chunk_1_1(
	p: v1_2_3::play_toClient_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	if heights::active(cx) {
		if let Some(out) = heights::map_chunk(&p, cx)? {
			return Ok(out);
		}
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
	let olds = if p.groundUp {
		vec![chunk::old_full(p.x, p.z, &raw, shape, V::R1_1)?]
	} else {
		chunk::old_sections(p.x, p.z, &raw, shape, V::R1_1)?
	};
	Ok(olds
		.into_iter()
		.map(|o| {
			Client11::map_chunk(v1_1::play_toClient_packet_map_chunk::Ty {
				x: o.x,
				y: o.y,
				z: o.z,
				sizeX: o.size_x,
				sizeY: o.size_y,
				sizeZ: o.size_z,
				compressedChunkData: o.data,
			})
		})
		.collect())
}

pub fn prenetty_multi_block_change_1_2_3(
	p: v1_1::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client12>> {
	let records = p
		.coordinates
		.iter()
		.zip(p.blockIds.iter().zip(&p.metadata))
		.map(|(c, (id, meta))| record(c.x as i32, c.y as i32, c.z as i32, *id, *meta & 15))
		.collect();
	Ok(records_packets(p.chunkX, p.chunkZ, records))
}

pub fn prenetty_multi_block_change_1_1(
	p: v1_2_3::play_toClient_packet_multi_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	if heights::active(cx) {
		return heights::multi_block_change(p, cx);
	}
	let mut coordinates = Vec::new();
	let mut block_ids = Vec::new();
	let mut metadata = Vec::new();
	for r in p.records.iter().filter(|r| r.y < 128) {
		let (id, meta) = ids::block(r.blockId >> 4, (r.blockId & 15) as u8, V::R1_1);
		coordinates.push(
			v1_1::play_toClient_packet_multi_block_change__coordinates__items::Ty {
				x: r.horizontalPos >> 4,
				z: r.horizontalPos & 15,
				y: r.y,
			},
		);
		block_ids.push(id as u8);
		metadata.push(meta);
	}
	if coordinates.is_empty() {
		return Ok(vec![]);
	}
	Ok(vec![Client11::multi_block_change(
		v1_1::play_toClient_packet_multi_block_change::Ty {
			chunkX: p.chunkX,
			chunkZ: p.chunkZ,
			coordinates,
			blockIds: block_ids,
			metadata,
		},
	)])
}

pub fn prenetty_block_change_1_1(
	mut p: v1_2_3::play_toClient_packet_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	if heights::active(cx) {
		return heights::block_change(p, cx);
	}
	if p.location.y >= 128 {
		return Ok(vec![]);
	}
	let (id, meta) = ids::block(p.r#type as u16, p.metadata, V::R1_1);
	p.r#type = id as u8;
	p.metadata = meta;
	Ok(vec![Client11::block_change(p)])
}

pub fn prenetty_spawn_entity_living_1_1(
	mut p: v1_2_3::play_toClient_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	match entities::mob_stand_in(p.r#type, V::R1_1) {
		Some(with) => {
			entities::stand_in(cx, p.entityId, Kind::Mob(with));
			p.r#type = with;
		}
		None if ids::mob_exists(p.r#type, V::R1_1) => {
			entities::track(cx, p.entityId, Kind::Mob(p.r#type))
		}
		None => return Ok(vec![]),
	}
	heights::spawned(cx, p.entityId, (p.x, &mut p.y, p.z), Some((p.yaw, p.pitch)));
	let metadata = entities::meta_1_1(cx, p.entityId, p.metadata);
	Ok(vec![Client11::spawn_entity_living(
		v1_1::play_toClient_packet_spawn_entity_living::Ty {
			entityId: p.entityId,
			r#type: p.r#type,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			metadata,
		},
	)])
}

pub fn prenetty_spawn_entity_1_1(
	mut p: v1_2_3::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	match entities::object_stand_in(p.r#type, V::R1_1) {
		Some(with) => {
			entities::stand_in(cx, p.entityId, Kind::Object(with, 0));
			p.r#type = with;
		}
		None if ids::object_exists(p.r#type, V::R1_1) => {}
		None => return Ok(vec![]),
	}
	heights::spawned(cx, p.entityId, (p.x, &mut p.y, p.z), None);
	Ok(vec![Client11::spawn_entity(p)])
}

pub fn prenetty_username_1_1(
	p: &v1_2_3::play_toServer_packet_set_protocol::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(p.usernameAndHost
		.split(';')
		.next()
		.unwrap_or_default()
		.to_owned())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn records_survive_the_parallel_arrays() {
		let mut cx = Session::new();
		let p = v1_2_3::play_toClient_packet_multi_block_change::Ty {
			chunkX: 1,
			chunkZ: -1,
			dataLength: 8,
			records: vec![record(5, 70, 10, 123, 0), record(0, 200, 0, 1, 0)],
		};
		let Client11::multi_block_change(old) =
			super::super::one(prenetty_multi_block_change_1_1(p, &mut cx).unwrap())
		else {
			panic!()
		};
		assert_eq!((old.coordinates.len(), old.blockIds[0]), (1, 1));
		let Client12::multi_block_change(back) =
			super::super::one(prenetty_multi_block_change_1_2_3(old, &mut cx).unwrap())
		else {
			panic!()
		};
		let r = &back.records[0];
		assert_eq!(
			(r.horizontalPos, r.y, r.blockId, back.dataLength),
			(0x5a, 70, 1 << 4, 4)
		);
	}
}
