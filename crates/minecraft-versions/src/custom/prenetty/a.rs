use super::chunk::{self, Old, Shape};
use super::heights;
use super::ids;
use super::{entities, item_id, legacy_slot, old_meta_item, Kind, OldMetaItem};
use crate::gen::convert::Version;
use crate::gen::convert::{v1_0_1, v1_1, v1_2_3};
use crate::pds::convert::Result;
use crate::session::Session;

macro_rules! legacy_items {
	($to:expr, $m:ident, $meta:ident, $equip:ident, $named:ident, $item:ident) => {
		pub fn $meta(v: OldMetaItem, _cx: &mut Session) -> Result<OldMetaItem> {
			Ok(old_meta_item(v, $to))
		}

		pub fn $equip(
			mut p: $m::play_to_client_packet_entity_equipment::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_client_packet::Ty>> {
			if p.item_id >= 0 {
				(p.item_id, p.item_damage) = ids::item(p.item_id, p.item_damage, $to);
			}
			Ok(vec![$m::play_to_client_packet::Ty::EntityEquipment(p)])
		}

		pub fn $named(
			mut p: $m::play_to_client_packet_named_entity_spawn::Ty,
			cx: &mut Session,
		) -> Result<Vec<$m::play_to_client_packet::Ty>> {
			p.current_item = item_id(p.current_item, $to);
			if $to == Version::V1_1 {
				heights::spawned(
					cx,
					p.entity_id,
					(p.x, &mut p.y, p.z),
					Some((p.yaw, p.pitch)),
				);
			}
			Ok(vec![$m::play_to_client_packet::Ty::NamedEntitySpawn(p)])
		}

		pub fn $item(
			mut p: $m::play_to_client_packet_spawn_entity_item::Ty,
			cx: &mut Session,
		) -> Result<Vec<$m::play_to_client_packet::Ty>> {
			(p.item.id, p.item.item_damage) = ids::item(p.item.id, p.item.item_damage, $to);
			if $to == Version::V1_1 {
				heights::spawned(cx, p.entity_id, (p.x, &mut p.y, p.z), None);
			}
			Ok(vec![$m::play_to_client_packet::Ty::SpawnEntityItem(p)])
		}
	};
}

macro_rules! legacy_windows {
	($to:expr, $m:ident, $set_slot:ident, $items:ident, $creative:ident, $creative_server:ident, $click:ident, $place:ident) => {
		pub fn $set_slot(
			mut p: $m::play_to_client_packet_set_slot::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_client_packet::Ty>> {
			p.item = legacy_slot(p.item, $to);
			Ok(vec![$m::play_to_client_packet::Ty::SetSlot(p)])
		}

		pub fn $items(
			mut p: $m::play_to_client_packet_window_items::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_client_packet::Ty>> {
			p.items = p.items.into_iter().map(|i| legacy_slot(i, $to)).collect();
			Ok(vec![$m::play_to_client_packet::Ty::WindowItems(p)])
		}

		pub fn $creative(
			mut p: $m::play_to_client_packet_set_creative_slot::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_client_packet::Ty>> {
			p.item = legacy_slot(p.item, $to);
			Ok(vec![$m::play_to_client_packet::Ty::SetCreativeSlot(p)])
		}

		pub fn $creative_server(
			mut p: $m::play_to_server_packet_set_creative_slot::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_server_packet::Ty>> {
			p.item = legacy_slot(p.item, $to);
			Ok(vec![$m::play_to_server_packet::Ty::SetCreativeSlot(p)])
		}

		pub fn $click(
			mut p: $m::play_to_server_packet_window_click::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_server_packet::Ty>> {
			p.item = legacy_slot(p.item, $to);
			Ok(vec![$m::play_to_server_packet::Ty::WindowClick(p)])
		}

		pub fn $place(
			mut p: $m::play_to_server_packet_block_place::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_server_packet::Ty>> {
			p.held_item = legacy_slot(p.held_item, $to);
			Ok(vec![$m::play_to_server_packet::Ty::BlockPlace(p)])
		}
	};
}

legacy_windows!(
	Version::V1_0_1,
	v1_0_1,
	prenetty_set_slot_1_0_1,
	prenetty_window_items_1_0_1,
	prenetty_set_creative_slot_1_0_1,
	prenetty_set_creative_slot_server_1_0_1,
	prenetty_window_click_1_0_1,
	prenetty_block_place_1_0_1
);

legacy_windows!(
	Version::V1_1,
	v1_1,
	prenetty_set_slot_1_1,
	prenetty_window_items_1_1,
	prenetty_set_creative_slot_1_1,
	prenetty_set_creative_slot_server_1_1,
	prenetty_window_click_1_1,
	prenetty_block_place_1_1
);

legacy_windows!(
	Version::V1_2_3,
	v1_2_3,
	prenetty_set_slot_1_2_3,
	prenetty_window_items_1_2_3,
	prenetty_set_creative_slot_1_2_3,
	prenetty_set_creative_slot_server_1_2_3,
	prenetty_window_click_1_2_3,
	prenetty_block_place_1_2_3
);

legacy_items!(
	Version::V1_0_1,
	v1_0_1,
	prenetty_metadata_item_1_0_1,
	prenetty_entity_equipment_1_0_1,
	prenetty_named_entity_spawn_1_0_1,
	prenetty_spawn_entity_item_1_0_1
);

legacy_items!(
	Version::V1_1,
	v1_1,
	prenetty_metadata_item_1_1,
	prenetty_entity_equipment_1_1,
	prenetty_named_entity_spawn_1_1,
	prenetty_spawn_entity_item_1_1
);

legacy_items!(
	Version::V1_2_3,
	v1_2_3,
	prenetty_metadata_item_1_2_3,
	prenetty_entity_equipment_1_2_3,
	prenetty_named_entity_spawn_1_2_3,
	prenetty_spawn_entity_item_1_2_3
);

type Client11 = v1_1::play_to_client_packet::Ty;
type Client12 = v1_2_3::play_to_client_packet::Ty;
type Records = v1_2_3::play_to_client_packet_multi_block_change_records_items::Ty;

const MAX_RECORDS: usize = i16::MAX as usize;

fn records_packets(chunk_x: i32, chunk_z: i32, records: Vec<Records>) -> Vec<Client12> {
	records
		.chunks(MAX_RECORDS)
		.map(|part| {
			Client12::MultiBlockChange(v1_2_3::play_to_client_packet_multi_block_change::Ty {
				chunk_x: chunk_x,
				chunk_z: chunk_z,
				data_length: part.len() as i32 * 4,
				records: part.to_vec(),
			})
		})
		.collect()
}

fn record(x: i32, y: i32, z: i32, id: u8, meta: u8) -> Records {
	Records {
		horizontal_pos: ((x & 15) << 4 | z & 15) as u8,
		y: y as u8,
		block_id: (id as u16) << 4 | (meta & 15) as u16,
	}
}

pub fn prenetty_map_chunk_1_2_3(
	p: v1_1::play_to_client_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client12>> {
	let old = Old {
		x: p.x,
		y: p.y,
		z: p.z,
		size_x: p.size_x,
		size_y: p.size_y,
		size_z: p.size_z,
		data: p.compressed_chunk_data,
	};
	let cuboid = chunk::from_old(&old, true)?;
	if let Some((x, z, data, bitmap)) = cuboid.full {
		return Ok(vec![Client12::MapChunk(
			v1_2_3::play_to_client_packet_map_chunk::Ty {
				x,
				z,
				ground_up: true,
				bit_map: bitmap,
				add_bit_map: 0,
				unused: 0,
				compressed_chunk_data: chunk::deflate(&data)?,
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
	p: v1_2_3::play_to_client_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	if heights::active(cx) {
		if let Some(out) = heights::map_chunk(&p, cx)? {
			return Ok(out);
		}
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
	let olds = if p.ground_up {
		vec![chunk::old_full(p.x, p.z, &raw, shape, Version::V1_1)?]
	} else {
		chunk::old_sections(p.x, p.z, &raw, shape, Version::V1_1)?
	};
	Ok(olds
		.into_iter()
		.map(|o| {
			Client11::MapChunk(v1_1::play_to_client_packet_map_chunk::Ty {
				x: o.x,
				y: o.y,
				z: o.z,
				size_x: o.size_x,
				size_y: o.size_y,
				size_z: o.size_z,
				compressed_chunk_data: o.data,
			})
		})
		.collect())
}

pub fn prenetty_multi_block_change_1_2_3(
	p: v1_1::play_to_client_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client12>> {
	let records = p
		.coordinates
		.iter()
		.zip(p.block_ids.iter().zip(&p.metadata))
		.map(|(c, (id, meta))| record(c.x as i32, c.y as i32, c.z as i32, *id, *meta & 15))
		.collect();
	Ok(records_packets(p.chunk_x, p.chunk_z, records))
}

pub fn prenetty_multi_block_change_1_1(
	p: v1_2_3::play_to_client_packet_multi_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	if heights::active(cx) {
		return heights::multi_block_change(p, cx);
	}
	let mut coordinates = Vec::new();
	let mut block_ids = Vec::new();
	let mut metadata = Vec::new();
	for r in p.records.iter().filter(|r| r.y < 128) {
		let (id, meta) = ids::block(r.block_id >> 4, (r.block_id & 15) as u8, Version::V1_1);
		coordinates.push(
			v1_1::play_to_client_packet_multi_block_change_coordinates_items::Ty {
				x: r.horizontal_pos >> 4,
				z: r.horizontal_pos & 15,
				y: r.y,
			},
		);
		block_ids.push(id as u8);
		metadata.push(meta);
	}
	if coordinates.is_empty() {
		return Ok(vec![]);
	}
	Ok(vec![Client11::MultiBlockChange(
		v1_1::play_to_client_packet_multi_block_change::Ty {
			chunk_x: p.chunk_x,
			chunk_z: p.chunk_z,
			coordinates,
			block_ids: block_ids,
			metadata,
		},
	)])
}

pub fn prenetty_block_change_1_1(
	mut p: v1_2_3::play_to_client_packet_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	if heights::active(cx) {
		return heights::block_change(p, cx);
	}
	if p.location.y >= 128 {
		return Ok(vec![]);
	}
	let (id, meta) = ids::block(p.r#type as u16, p.metadata, Version::V1_1);
	p.r#type = id as u8;
	p.metadata = meta;
	Ok(vec![Client11::BlockChange(p)])
}

pub fn prenetty_spawn_entity_living_1_1(
	mut p: v1_2_3::play_to_client_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	match entities::mob_stand_in(p.r#type, Version::V1_1) {
		Some(with) => {
			entities::stand_in(cx, p.entity_id, Kind::Mob(with));
			p.r#type = with;
		}
		None if ids::mob_exists(p.r#type, Version::V1_1) => {
			entities::track(cx, p.entity_id, Kind::Mob(p.r#type))
		}
		None => return Ok(vec![]),
	}
	heights::spawned(
		cx,
		p.entity_id,
		(p.x, &mut p.y, p.z),
		Some((p.yaw, p.pitch)),
	);
	let metadata = entities::meta_1_1(cx, p.entity_id, p.metadata);
	Ok(vec![Client11::SpawnEntityLiving(
		v1_1::play_to_client_packet_spawn_entity_living::Ty {
			entity_id: p.entity_id,
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
	mut p: v1_2_3::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	match entities::object_stand_in(p.r#type, Version::V1_1) {
		Some(with) => {
			entities::stand_in(cx, p.entity_id, Kind::Object(with, 0));
			p.r#type = with;
		}
		None if ids::object_exists(p.r#type, Version::V1_1) => {}
		None => return Ok(vec![]),
	}
	heights::spawned(cx, p.entity_id, (p.x, &mut p.y, p.z), None);
	Ok(vec![Client11::SpawnEntity(p)])
}

pub fn prenetty_username_1_1(
	p: &v1_2_3::play_to_server_packet_set_protocol::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(p.username_and_host
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
		let p = v1_2_3::play_to_client_packet_multi_block_change::Ty {
			chunk_x: 1,
			chunk_z: -1,
			data_length: 8,
			records: vec![record(5, 70, 10, 123, 0), record(0, 200, 0, 1, 0)],
		};
		let Client11::MultiBlockChange(old) =
			super::super::one(prenetty_multi_block_change_1_1(p, &mut cx).unwrap())
		else {
			panic!()
		};
		assert_eq!((old.coordinates.len(), old.block_ids[0]), (1, 1));
		let Client12::MultiBlockChange(back) =
			super::super::one(prenetty_multi_block_change_1_2_3(old, &mut cx).unwrap())
		else {
			panic!()
		};
		let r = &back.records[0];
		assert_eq!(
			(r.horizontal_pos, r.y, r.block_id, back.data_length),
			(0x5a, 70, 1 << 4, 4)
		);
	}
}
