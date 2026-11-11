use super::chunk::{self, Shape};
use super::ids::{self, V};
use super::{
	entities, falling_block, item_id, mid_meta_item, old_meta_item, slot, state, world_event_data,
	Kind, MidMetaItem, OldMetaItem, Pending, Slot,
};
use crate::gen::convert::{v1_3_2, v1_4_2, v1_4_5, v1_4_7, v1_5_1};
use crate::pds::convert::Result;
use crate::session::Session;

macro_rules! slot_down {
	($name:ident, $to:expr) => {
		pub fn $name(v: Slot, _cx: &mut Session) -> Result<Slot> {
			Ok(slot(v, $to))
		}
	};
}

slot_down!(prenetty_slot_down_1_3_2, V::R1_3);
slot_down!(prenetty_slot_down_1_4_2, V::R1_4_2);
slot_down!(prenetty_slot_down_1_4_5, V::R1_4_5);
slot_down!(prenetty_slot_down_1_4_7, V::R1_4_7);
slot_down!(prenetty_slot_down_1_5_2, V::R1_5);
slot_down!(prenetty_slot_down_1_6_4, V::R1_6_4);

macro_rules! blocks {
	($to:expr, $m:ident, $block_change:ident, $multi:ident, $map_chunk:ident, $world_event:ident) => {
		pub fn $block_change(
			mut p: $m::play_to_client_packet_block_change::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_client_packet::Ty>> {
			let (id, meta) = ids::block(p.r#type as u16, p.metadata, $to);
			p.r#type = id as _;
			p.metadata = meta;
			Ok(vec![$m::play_to_client_packet::Ty::BlockChange(p)])
		}

		pub fn $multi(
			mut p: $m::play_to_client_packet_multi_block_change::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_client_packet::Ty>> {
			for r in &mut p.records {
				r.block_id = super::remap_record(r.block_id, $to);
			}
			Ok(vec![$m::play_to_client_packet::Ty::MultiBlockChange(p)])
		}

		pub fn $map_chunk(
			mut p: $m::play_to_client_packet_map_chunk::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_client_packet::Ty>> {
			p.compressed_chunk_data = super::c::remap_single(
				&p.compressed_chunk_data,
				p.bit_map,
				p.add_bit_map,
				p.ground_up,
				$to,
			)?;
			Ok(vec![$m::play_to_client_packet::Ty::MapChunk(p)])
		}

		pub fn $world_event(
			mut p: $m::play_to_client_packet_world_event::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_client_packet::Ty>> {
			p.data = super::world_event_data(p.effect_id, p.data, $to);
			Ok(vec![$m::play_to_client_packet::Ty::WorldEvent(p)])
		}
	};
}

pub(super) use blocks;

macro_rules! named {
	($named:ident, $m:ident, $to:expr) => {
		pub fn $named(
			mut p: $m::play_to_client_packet_named_entity_spawn::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$m::play_to_client_packet::Ty>> {
			p.current_item = item_id(p.current_item, $to);
			Ok(vec![$m::play_to_client_packet::Ty::NamedEntitySpawn(p)])
		}
	};
}

pub(super) fn remap_single(
	data: &[u8],
	bitmap: u16,
	add: u16,
	ground_up: bool,
	to: V,
) -> Result<Vec<u8>> {
	if !ids::block_changes(to) || bitmap == 0 {
		return Ok(data.to_vec());
	}
	let mut raw = chunk::inflate(data)?;
	let mut shape = Shape {
		bitmap,
		add,
		sky: true,
		full: ground_up,
	};
	if raw.len() < shape.len() {
		shape.sky = false;
	}
	chunk::remap(&mut raw, shape, to)?;
	chunk::deflate(&raw)
}

pub(super) fn remap_bulk(
	data: &[u8],
	columns: &[(i32, i32, u16, u16)],
	sky: bool,
	to: V,
) -> Result<Vec<u8>> {
	if !ids::block_changes(to) {
		return Ok(data.to_vec());
	}
	let raw = chunk::inflate(data)?;
	let mut out = Vec::with_capacity(raw.len());
	for (_, _, bitmap, add, mut column) in chunk::split(&raw, columns, sky)? {
		chunk::remap(
			&mut column,
			Shape {
				bitmap,
				add,
				sky,
				full: true,
			},
			to,
		)?;
		out.extend_from_slice(&column);
	}
	chunk::deflate(&out)
}

blocks!(
	V::R1_3,
	v1_3_2,
	prenetty_block_change_1_3_2_down,
	prenetty_multi_block_change_1_3_2_down,
	prenetty_map_chunk_1_3_2_down,
	prenetty_world_event_1_3_2_down
);

pub(super) fn meta_1_3_2(m: v1_4_2::entity_metadata::Ty) -> v1_3_2::entity_metadata::Ty {
	m.into_iter()
		.map(|e| v1_3_2::entity_metadata_items::Ty {
			anon: v1_3_2::entity_metadata_items_anon::Ty {
				r#type: e.anon.r#type,
				key: e.anon.key,
			},
			value: meta_item_1_3_2(e.value),
		})
		.collect()
}

fn meta_1_4_2(m: v1_4_5::entity_metadata::Ty) -> v1_4_2::entity_metadata::Ty {
	m.into_iter()
		.map(|e| v1_4_2::entity_metadata_items::Ty {
			anon: v1_4_2::entity_metadata_items_anon::Ty {
				r#type: e.anon.r#type,
				key: e.anon.key,
			},
			value: meta_item_1_4_2(e.value),
		})
		.collect()
}

pub fn prenetty_named_entity_spawn_1_3_2_down(
	p: v1_4_2::play_to_client_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Vec<v1_3_2::play_to_client_packet::Ty>> {
	entities::track(cx, p.entity_id, Kind::Player);
	Ok(vec![v1_3_2::play_to_client_packet::Ty::NamedEntitySpawn(
		v1_3_2::play_to_client_packet_named_entity_spawn::Ty {
			entity_id: p.entity_id,
			player_name: p.player_name,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			current_item: item_id(p.current_item, V::R1_3),
			metadata: meta_1_3_2(p.metadata),
		},
	)])
}

pub fn prenetty_spawn_entity_living_1_3_2_down(
	p: v1_4_2::play_to_client_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<v1_3_2::play_to_client_packet::Ty>> {
	let ty = match entities::mob_stand_in(p.r#type, V::R1_3) {
		Some(with) => {
			entities::stand_in(cx, p.entity_id, Kind::Mob(with));
			with
		}
		None if ids::mob_exists(p.r#type, V::R1_3) => {
			entities::track(cx, p.entity_id, Kind::Mob(p.r#type));
			p.r#type
		}
		None => return Ok(vec![]),
	};
	let metadata = meta_1_3_2(p.metadata)
		.into_iter()
		.filter(|e| entities::keeps(cx, p.entity_id, e.anon.key, e.anon.r#type, V::R1_3))
		.collect();
	Ok(vec![v1_3_2::play_to_client_packet::Ty::SpawnEntityLiving(
		v1_3_2::play_to_client_packet_spawn_entity_living::Ty {
			entity_id: p.entity_id,
			r#type: ty,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			head_pitch: p.head_pitch,
			velocity: p.velocity,
			metadata,
		},
	)])
}
named!(prenetty_named_entity_spawn_1_4_7_down, v1_4_7, V::R1_4_7);

blocks!(
	V::R1_4_7,
	v1_4_7,
	prenetty_block_change_1_4_7_down,
	prenetty_multi_block_change_1_4_7_down,
	prenetty_map_chunk_1_4_7_down,
	prenetty_world_event_1_4_7_down
);

pub fn prenetty_named_entity_spawn_1_4_2_down(
	p: v1_4_5::play_to_client_packet_named_entity_spawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_4_2::play_to_client_packet::Ty>> {
	Ok(vec![v1_4_2::play_to_client_packet::Ty::NamedEntitySpawn(
		v1_4_2::play_to_client_packet_named_entity_spawn::Ty {
			entity_id: p.entity_id,
			player_name: p.player_name,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			current_item: item_id(p.current_item, V::R1_4_2),
			metadata: meta_1_4_2(p.metadata),
		},
	)])
}

pub fn prenetty_named_entity_spawn_1_4_5_down(
	mut p: v1_4_7::play_to_client_packet_named_entity_spawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_4_5::play_to_client_packet::Ty>> {
	p.current_item = item_id(p.current_item, V::R1_4_5);
	Ok(vec![v1_4_5::play_to_client_packet::Ty::NamedEntitySpawn(p)])
}

pub fn prenetty_map_chunk_bulk_1_3_2_down(
	mut p: v1_4_2::play_to_client_packet_map_chunk_bulk::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_3_2::play_to_client_packet::Ty>> {
	let columns: Vec<_> = p
		.meta
		.iter()
		.map(|m| (m.x, m.z, m.bit_map, m.add_bit_map))
		.collect();
	p.compressed_chunk_data = remap_bulk(&p.compressed_chunk_data, &columns, true, V::R1_3)?;
	Ok(vec![v1_3_2::play_to_client_packet::Ty::MapChunkBulk(p)])
}

pub fn prenetty_map_chunk_bulk_1_4_7_down(
	mut p: v1_5_1::play_to_client_packet_map_chunk_bulk::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_4_7::play_to_client_packet::Ty>> {
	let columns: Vec<_> = p
		.meta
		.iter()
		.map(|m| (m.x, m.z, m.bit_map, m.add_bit_map))
		.collect();
	p.compressed_chunk_data = remap_bulk(
		&p.compressed_chunk_data,
		&columns,
		p.sky_light_sent,
		V::R1_4_7,
	)?;
	Ok(vec![v1_4_7::play_to_client_packet::Ty::MapChunkBulk(p)])
}

pub fn prenetty_spawn_entity_1_3_2_down(
	mut p: v1_4_2::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<v1_3_2::play_to_client_packet::Ty>> {
	let data = match &p.object_data {
		v1_4_2::play_to_client_packet_spawn_entity_object_data::Ty::Default(d, _) => *d,
		_ => 0,
	};
	match entities::object_stand_in(p.r#type, V::R1_3) {
		Some(with) => {
			entities::stand_in(cx, p.entity_id, Kind::Object(with, data));
			p.r#type = with;
		}
		None if ids::object_exists(p.r#type, V::R1_3) => {
			entities::track(cx, p.entity_id, Kind::Object(p.r#type, data))
		}
		None => return Ok(vec![]),
	}
	if p.r#type == 70 {
		if let v1_4_2::play_to_client_packet_spawn_entity_object_data::Ty::Default(data, v) =
			p.object_data
		{
			p.object_data = v1_4_2::play_to_client_packet_spawn_entity_object_data::Ty::Default(
				falling_block(data, V::R1_3, false),
				v,
			);
		}
	}
	Ok(vec![v1_3_2::play_to_client_packet::Ty::SpawnEntity(p)])
}

pub fn prenetty_metadata_item_1_4_2(v: OldMetaItem, _cx: &mut Session) -> Result<MidMetaItem> {
	use v1_4_2::entity_metadata_item_v5 as Item;
	Ok(match v {
		OldMetaItem::V0(x) => MidMetaItem::V0(x),
		OldMetaItem::V1(x) => MidMetaItem::V1(x),
		OldMetaItem::V2(x) => MidMetaItem::V2(x),
		OldMetaItem::V3(x) => MidMetaItem::V3(x),
		OldMetaItem::V4(x) => MidMetaItem::V4(x),
		OldMetaItem::V5(item) if item.id < 0 => MidMetaItem::V5(Item::Ty::_1),
		OldMetaItem::V5(item) => MidMetaItem::V5(Item::Ty::Default(
			item.id,
			v1_4_2::entity_metadata_item_v5_default::Ty {
				item_count: item.item_count,
				item_damage: item.item_damage,
			},
		)),
		OldMetaItem::V6(p) => MidMetaItem::V6(v1_4_2::entity_metadata_item_v6::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
		}),
		OldMetaItem::Default(t) => MidMetaItem::Default(t),
	})
}

pub fn prenetty_metadata_item_1_3_2(v: MidMetaItem, _cx: &mut Session) -> Result<OldMetaItem> {
	Ok(meta_item_1_3_2(v))
}

fn meta_item_1_3_2(v: MidMetaItem) -> OldMetaItem {
	use v1_4_2::entity_metadata_item_v5 as Item;
	let v = mid_meta_item(v, V::R1_3);
	match v {
		MidMetaItem::V0(x) => OldMetaItem::V0(x),
		MidMetaItem::V1(x) => OldMetaItem::V1(x),
		MidMetaItem::V2(x) => OldMetaItem::V2(x),
		MidMetaItem::V3(x) => OldMetaItem::V3(x),
		MidMetaItem::V4(x) => OldMetaItem::V4(x),
		MidMetaItem::V5(Item::Ty::Default(id, item)) => {
			OldMetaItem::V5(v1_3_2::entity_metadata_item_v5::Ty {
				id,
				item_count: item.item_count,
				item_damage: item.item_damage,
			})
		}
		MidMetaItem::V5(_) => OldMetaItem::V5(v1_3_2::entity_metadata_item_v5::Ty {
			id: -1,
			item_count: 0,
			item_damage: 0,
		}),
		MidMetaItem::V6(p) => OldMetaItem::V6(v1_3_2::entity_metadata_item_v6::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
		}),
		MidMetaItem::Default(t) => OldMetaItem::Default(t),
	}
}

type NewMetaItem = v1_4_5::entity_metadata_item::Ty;

pub fn prenetty_metadata_item_1_4_5(v: MidMetaItem, _cx: &mut Session) -> Result<NewMetaItem> {
	use v1_4_2::entity_metadata_item_v5 as Item;
	Ok(match v {
		MidMetaItem::V0(x) => NewMetaItem::V0(x),
		MidMetaItem::V1(x) => NewMetaItem::V1(x),
		MidMetaItem::V2(x) => NewMetaItem::V2(x),
		MidMetaItem::V3(x) => NewMetaItem::V3(x),
		MidMetaItem::V4(x) => NewMetaItem::V4(x),
		MidMetaItem::V5(Item::Ty::Default(id, item)) => NewMetaItem::V5(Slot::Default(
			id,
			v1_4_5::slot_default::Ty {
				item_count: item.item_count,
				item_damage: item.item_damage,
				nbt_data: None,
			},
		)),
		MidMetaItem::V5(_) => NewMetaItem::V5(Slot::_1),
		MidMetaItem::V6(p) => NewMetaItem::V6(v1_4_5::entity_metadata_item_v6::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
		}),
		MidMetaItem::Default(t) => NewMetaItem::Default(t),
	})
}

pub fn prenetty_metadata_item_1_4_2_down(v: NewMetaItem, _cx: &mut Session) -> Result<MidMetaItem> {
	Ok(meta_item_1_4_2(v))
}

fn meta_item_1_4_2(v: NewMetaItem) -> MidMetaItem {
	use v1_4_2::entity_metadata_item_v5 as Item;
	match v {
		NewMetaItem::V0(x) => MidMetaItem::V0(x),
		NewMetaItem::V1(x) => MidMetaItem::V1(x),
		NewMetaItem::V2(x) => MidMetaItem::V2(x),
		NewMetaItem::V3(x) => MidMetaItem::V3(x),
		NewMetaItem::V4(x) => MidMetaItem::V4(x),
		NewMetaItem::V5(s) => match slot(s, V::R1_4_2) {
			Slot::Default(id, item) => MidMetaItem::V5(Item::Ty::Default(
				id,
				v1_4_2::entity_metadata_item_v5_default::Ty {
					item_count: item.item_count,
					item_damage: item.item_damage,
				},
			)),
			_ => MidMetaItem::V5(Item::Ty::_1),
		},
		NewMetaItem::V6(p) => MidMetaItem::V6(v1_4_2::entity_metadata_item_v6::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
		}),
		NewMetaItem::Default(t) => MidMetaItem::Default(t),
	}
}

pub fn prenetty_old_metadata_item_1_3_2(v: OldMetaItem, _cx: &mut Session) -> Result<OldMetaItem> {
	Ok(old_meta_item(v, V::R1_3))
}

pub fn prenetty_spawn_entity_item_1_4_2(
	p: v1_3_2::play_to_client_packet_spawn_entity_item::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_4_2::play_to_client_packet::Ty>> {
	let item = if p.item.id < 0 {
		Slot::_1
	} else {
		Slot::Default(
			p.item.id,
			v1_4_2::slot_default::Ty {
				item_count: p.item.item_count,
				item_damage: p.item.item_damage,
				nbt_data: None,
			},
		)
	};
	Ok(vec![v1_4_2::play_to_client_packet::Ty::SpawnEntityItem(
		v1_4_2::play_to_client_packet_spawn_entity_item::Ty {
			entity_id: p.entity_id,
			item,
			x: p.x,
			y: p.y,
			z: p.z,
			velocity_x: p.velocity_x,
			velocity_y: p.velocity_y,
			velocity_z: p.velocity_z,
		},
	)])
}

pub fn prenetty_spawn_entity_item_1_3_2(
	p: v1_4_2::play_to_client_packet_spawn_entity_item::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_3_2::play_to_client_packet::Ty>> {
	let (id, count, damage) = match slot(p.item, V::R1_3) {
		Slot::Default(id, item) => (id, item.item_count, item.item_damage),
		_ => (-1, 0, 0),
	};
	Ok(vec![v1_3_2::play_to_client_packet::Ty::SpawnEntityItem(
		v1_3_2::play_to_client_packet_spawn_entity_item::Ty {
			entity_id: p.entity_id,
			item: v1_3_2::play_to_client_packet_spawn_entity_item_item::Ty {
				id,
				item_count: count,
				item_damage: damage,
			},
			x: p.x,
			y: p.y,
			z: p.z,
			velocity_x: p.velocity_x,
			velocity_y: p.velocity_y,
			velocity_z: p.velocity_z,
		},
	)])
}

pub fn prenetty_world_event_1_3_2(
	p: v1_4_2::play_to_client_packet_world_event::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_3_2::play_to_client_packet::Ty>> {
	Ok(vec![v1_3_2::play_to_client_packet::Ty::WorldEvent(
		v1_3_2::play_to_client_packet_world_event::Ty {
			effect_id: p.effect_id,
			location: p.location,
			data: world_event_data(p.effect_id, p.data, V::R1_3),
		},
	)])
}

pub fn prenetty_painting_direction(direction: i32) -> i32 {
	match direction {
		0 => 2,
		2 => 0,
		d => d,
	}
}

pub fn prenetty_painting_up_1_4_2(
	p: &v1_3_2::play_to_client_packet_spawn_entity_painting::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(prenetty_painting_direction(p.direction))
}

pub fn prenetty_painting_down_1_3_2(
	p: &v1_4_2::play_to_client_packet_spawn_entity_painting::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(prenetty_painting_direction(p.direction))
}

fn swap_icons(mut data: Vec<u8>) -> Vec<u8> {
	if data.first() == Some(&1) {
		for b in data[1..].chunks_mut(3) {
			b[0] = b[0] << 4 | b[0] >> 4;
		}
	}
	data
}

pub fn prenetty_map_up_1_4_2(
	p: &v1_3_2::play_to_client_packet_map::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	Ok(swap_icons(p.data.clone()))
}

pub fn prenetty_map_down_1_3_2(
	p: &v1_4_2::play_to_client_packet_map::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	Ok(swap_icons(p.data.clone()))
}

fn skip_slot(data: &[u8], at: &mut usize) -> Option<()> {
	let id = i16::from_be_bytes(data.get(*at..*at + 2)?.try_into().ok()?);
	*at += 2;
	if id >= 0 {
		*at += 3;
		let len = i16::from_be_bytes(data.get(*at..*at + 2)?.try_into().ok()?);
		*at += 2;
		if len > 0 {
			*at += len as usize;
		}
	}
	(*at <= data.len()).then_some(())
}

fn trades(data: &[u8], unavailable_in: bool, unavailable_out: bool) -> Option<Vec<u8>> {
	let mut out = data.get(..5)?.to_vec();
	let count = data[4];
	let mut at = 5;
	for _ in 0..count {
		let start = at;
		skip_slot(data, &mut at)?;
		skip_slot(data, &mut at)?;
		let three = *data.get(at)? != 0;
		at += 1;
		if three {
			skip_slot(data, &mut at)?;
		}
		out.extend_from_slice(&data[start..at]);
		if unavailable_in {
			at += 1;
		}
		if unavailable_out {
			out.push(0);
		}
	}
	(at <= data.len()).then_some(out)
}

pub fn prenetty_custom_payload_up_1_4_2(
	p: &v1_3_2::play_to_client_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	if p.channel != "MC|TrList" {
		return Ok(p.data.clone());
	}
	Ok(trades(&p.data, false, true).unwrap_or_else(|| p.data.clone()))
}

pub fn prenetty_custom_payload_down_1_3_2(
	p: &v1_4_2::play_to_client_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<Vec<u8>> {
	if p.channel != "MC|TrList" {
		return Ok(p.data.clone());
	}
	Ok(trades(&p.data, true, false).unwrap_or_else(|| p.data.clone()))
}

pub fn prenetty_ping_up_1_4_2(
	_p: v1_3_2::play_to_server_packet_legacy_server_list_ping::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_4_2::play_to_server_packet::Ty>> {
	Ok(vec![
		v1_4_2::play_to_server_packet::Ty::LegacyServerListPing(1),
	])
}

pub fn prenetty_ping_down_1_3_2(
	_p: u8,
	_cx: &mut Session,
) -> Result<Vec<v1_3_2::play_to_server_packet::Ty>> {
	Ok(vec![
		v1_3_2::play_to_server_packet::Ty::LegacyServerListPing(
			v1_3_2::play_to_server_packet_legacy_server_list_ping::Ty {},
		),
	])
}

pub fn prenetty_spawn_entity_item_1_4_7(
	p: v1_4_5::play_to_client_packet_spawn_entity_item::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_4_7::play_to_client_packet::Ty>> {
	let velocity = |v: i8| (v as f32 / 128.0 * 8000.0) as i16;
	let spawn = v1_4_7::play_to_client_packet_spawn_entity::Ty {
		entity_id: p.entity_id,
		r#type: 2,
		x: p.x,
		y: p.y,
		z: p.z,
		pitch: 0,
		yaw: 0,
		object_data: v1_4_7::play_to_client_packet_spawn_entity_object_data::Ty::Default(
			1,
			v1_4_7::play_to_client_packet_spawn_entity_object_data_default::Ty {
				velocity_x: velocity(p.velocity_x),
				velocity_y: velocity(p.velocity_y),
				velocity_z: velocity(p.velocity_z),
			},
		),
	};
	let metadata = v1_4_7::play_to_client_packet_entity_metadata::Ty {
		entity_id: p.entity_id,
		metadata: vec![v1_4_7::entity_metadata_items::Ty {
			anon: v1_4_7::entity_metadata_items_anon::Ty { r#type: 5, key: 10 },
			value: NewMetaItem::V5(p.item),
		}],
	};
	Ok(vec![
		v1_4_7::play_to_client_packet::Ty::SpawnEntity(spawn),
		v1_4_7::play_to_client_packet::Ty::EntityMetadata(metadata),
	])
}

pub fn prenetty_spawn_entity_1_4_5(
	p: v1_4_7::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<v1_4_5::play_to_client_packet::Ty>> {
	if !ids::object_exists(p.r#type, V::R1_4_5) {
		return Ok(vec![]);
	}
	if p.r#type == 2 {
		let velocity = match &p.object_data {
			v1_4_7::play_to_client_packet_spawn_entity_object_data::Ty::Default(_, v) => {
				(v.velocity_x, v.velocity_y, v.velocity_z)
			}
			_ => (0, 0, 0),
		};
		state(cx).pending.insert(
			p.entity_id,
			Pending {
				x: p.x,
				y: p.y,
				z: p.z,
				velocity,
			},
		);
		return Ok(vec![]);
	}
	Ok(vec![v1_4_5::play_to_client_packet::Ty::SpawnEntity(
		v1_4_5::play_to_client_packet_spawn_entity::Ty {
			entity_id: p.entity_id,
			r#type: p.r#type,
			x: p.x,
			y: p.y,
			z: p.z,
			object_data: match p.object_data {
				v1_4_7::play_to_client_packet_spawn_entity_object_data::Ty::V0 => {
					v1_4_5::play_to_client_packet_spawn_entity_object_data::Ty::V0
				}
				v1_4_7::play_to_client_packet_spawn_entity_object_data::Ty::Default(t, v) => {
					v1_4_5::play_to_client_packet_spawn_entity_object_data::Ty::Default(
						t,
						v1_4_5::play_to_client_packet_spawn_entity_object_data_default::Ty {
							velocity_x: v.velocity_x,
							velocity_y: v.velocity_y,
							velocity_z: v.velocity_z,
						},
					)
				}
			},
		},
	)])
}

pub fn prenetty_entity_metadata_1_4_5(
	mut p: v1_4_7::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<v1_4_5::play_to_client_packet::Ty>> {
	let mut out = Vec::new();
	if state(cx).pending.contains_key(&p.entity_id) {
		let at = p.metadata.iter().position(|e| {
			e.anon.key == 10 && matches!(e.value, NewMetaItem::V5(Slot::Default(..)))
		});
		if let Some(at) = at {
			let pending = state(cx).pending.remove(&p.entity_id).unwrap_or_default();
			let NewMetaItem::V5(item) = p.metadata.remove(at).value else {
				unreachable!()
			};
			let velocity = |v: i16| (v as f32 / 8000.0 * 128.0).clamp(-128.0, 127.0) as i8;
			out.push(v1_4_5::play_to_client_packet::Ty::SpawnEntityItem(
				v1_4_5::play_to_client_packet_spawn_entity_item::Ty {
					entity_id: p.entity_id,
					item: slot(item, V::R1_4_5),
					x: pending.x,
					y: pending.y,
					z: pending.z,
					velocity_x: velocity(pending.velocity.0),
					velocity_y: velocity(pending.velocity.1),
					velocity_z: velocity(pending.velocity.2),
				},
			));
		}
	}
	if !p.metadata.is_empty() || out.is_empty() {
		out.push(v1_4_5::play_to_client_packet::Ty::EntityMetadata(p));
	}
	Ok(out)
}

pub fn prenetty_entity_destroy_1_4_5(
	ids: Vec<i32>,
	cx: &mut Session,
) -> Result<Vec<v1_4_5::play_to_client_packet::Ty>> {
	let st = state(cx);
	for id in &ids {
		st.pending.remove(id);
		st.stand_ins.remove(id);
	}
	Ok(vec![v1_4_5::play_to_client_packet::Ty::EntityDestroy(ids)])
}

pub fn prenetty_map_chunk_bulk_1_4_5(
	p: v1_4_7::play_to_client_packet_map_chunk_bulk::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_4_5::play_to_client_packet::Ty>> {
	let data = if p.sky_light_sent {
		p.compressed_chunk_data
	} else {
		let raw = chunk::inflate(&p.compressed_chunk_data)?;
		let columns: Vec<_> = p
			.meta
			.iter()
			.map(|m| (m.x, m.z, m.bit_map, m.add_bit_map))
			.collect();
		let mut out = Vec::new();
		for (_, _, bitmap, add, column) in chunk::split(&raw, &columns, false)? {
			out.extend(chunk::with_sky(
				&column,
				Shape {
					bitmap,
					add,
					sky: false,
					full: true,
				},
			)?);
		}
		chunk::deflate(&out)?
	};
	Ok(vec![v1_4_5::play_to_client_packet::Ty::MapChunkBulk(
		v1_4_5::play_to_client_packet_map_chunk_bulk::Ty {
			compressed_chunk_data: data,
			meta: p
				.meta
				.into_iter()
				.map(
					|m| v1_4_5::play_to_client_packet_map_chunk_bulk_meta_items::Ty {
						x: m.x,
						z: m.z,
						bit_map: m.bit_map,
						add_bit_map: m.add_bit_map,
					},
				)
				.collect(),
		},
	)])
}

pub fn prenetty_block_dig_1_4_5(
	mut p: v1_4_7::play_to_server_packet_block_dig::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_4_5::play_to_server_packet::Ty>> {
	if p.status == 3 {
		p.status = 4;
	}
	Ok(vec![v1_4_5::play_to_server_packet::Ty::BlockDig(p)])
}

pub fn prenetty_spawn_entity_1_5_1(
	mut p: v1_4_7::play_to_client_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_5_1::play_to_client_packet::Ty>> {
	type Data = v1_4_7::play_to_client_packet_spawn_entity_object_data::Ty;
	if let 10..=12 = p.r#type {
		let kind = (p.r#type - 10) as i32;
		let velocity = match p.object_data {
			Data::Default(_, v) => v,
			_ => v1_4_7::play_to_client_packet_spawn_entity_object_data_default::Ty {
				velocity_x: 0,
				velocity_y: 0,
				velocity_z: 0,
			},
		};
		p.r#type = 10;
		p.object_data = if kind == 0 {
			Data::V0
		} else {
			Data::Default(kind, velocity)
		};
	}
	Ok(vec![v1_5_1::play_to_client_packet::Ty::SpawnEntity(p)])
}

pub fn prenetty_spawn_entity_1_4_7(
	mut p: v1_5_1::play_to_client_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_4_7::play_to_client_packet::Ty>> {
	type Data = v1_4_7::play_to_client_packet_spawn_entity_object_data::Ty;
	if p.r#type == 10 {
		let kind = match &p.object_data {
			Data::Default(kind, _) if (1..=2).contains(kind) => *kind as i8,
			_ => 0,
		};
		p.r#type = 10 + kind;
		p.object_data = Data::V0;
	} else if p.r#type == 70 {
		if let Data::Default(data, v) = p.object_data {
			p.object_data = Data::Default(falling_block(data, V::R1_4_7, true), v);
		}
	}
	if !ids::object_exists(p.r#type, V::R1_4_7) {
		return Ok(vec![]);
	}
	Ok(vec![v1_4_7::play_to_client_packet::Ty::SpawnEntity(p)])
}

pub fn prenetty_window_click_1_4_7(
	mut p: v1_5_1::play_to_server_packet_window_click::Ty,
	_cx: &mut Session,
) -> Result<Vec<v1_4_7::play_to_server_packet::Ty>> {
	if p.mode > 3 {
		let (mut drop_q, mut start, mut end, mut add) = (false, false, false, false);
		match p.mode {
			4 => drop_q = p.mouse_button as i32 + if p.slot != -999 { 2 } else { 0 } == 2,
			5 => {
				start = p.mouse_button == 0;
				end = p.mouse_button == 2;
				add = p.mouse_button == 1;
			}
			_ => {}
		}
		let left = start || add || end;
		let outside = p.slot == -999 && p.mode != 5;
		if drop_q || (p.slot < 0 && !outside) {
			return Ok(vec![]);
		}
		p.mouse_button = if left { 0 } else { 1 };
		p.mode = 0;
		p.item = Slot::Default(
			34,
			v1_4_7::slot_default::Ty {
				item_count: 0,
				item_damage: 0,
				nbt_data: None,
			},
		);
	}
	Ok(vec![v1_4_7::play_to_server_packet::Ty::WindowClick(p)])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn trades_gain_and_lose_the_unavailable_flag() {
		let mut old = vec![0, 0, 0, 1, 1];
		old.extend_from_slice(&[0, 1, 1, 0, 0, 255, 255]);
		old.extend_from_slice(&[255, 255]);
		old.push(0);
		let new = trades(&old, false, true).unwrap();
		assert_eq!(new.len(), old.len() + 1);
		assert_eq!(trades(&new, true, false).unwrap(), old);
	}

	#[test]
	fn icons_swap_nibbles() {
		assert_eq!(swap_icons(vec![1, 0x21, 5, 6]), vec![1, 0x12, 5, 6]);
		assert_eq!(swap_icons(vec![0, 0x21]), vec![0, 0x21]);
	}
}
