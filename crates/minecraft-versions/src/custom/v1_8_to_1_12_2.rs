use crate::gen::convert::{v1_10, v1_11, v1_8, v1_9, v1_9_2, v1_9_4};
use crate::pds::convert::{unconvertible, Error, Result};
use crate::session::{Kind, Session};

type Meta8 = v1_8::entity_metadata_item::Ty;
type Meta9 = v1_9::entity_metadata_item::Ty;
type Entry8 = v1_8::entity_metadata_items::Ty;
type Entry9 = v1_9::entity_metadata_items::Ty;
type Client9 = v1_9::play_to_client_packet::Ty;
type Client94 = v1_9_4::play_to_client_packet::Ty;
type Server8 = v1_8::play_to_server_packet::Ty;
type Server9 = v1_9::play_to_server_packet::Ty;
type Server94 = v1_9_4::play_to_server_packet::Ty;
type Server10 = v1_10::play_to_server_packet::Ty;

fn uuid_of(entity: i32) -> u128 {
	(0x7064_7332_0000_4000_8000_0000_0000_0000u128) | (entity as u32 as u128)
}

fn entry(key: u8, value: Meta9) -> Entry9 {
	let ty = match &value {
		Meta9::V0(_) => 0,
		Meta9::V1(_) => 1,
		Meta9::V2(_) => 2,
		Meta9::V3(_) => 3,
		Meta9::V4(_) => 4,
		Meta9::V5(_) => 5,
		Meta9::V6(_) => 6,
		Meta9::V7(_) => 7,
		Meta9::V8(_) => 8,
		Meta9::V9(_) => 9,
		Meta9::V10(_) => 10,
		Meta9::V11(_) => 11,
		Meta9::V12(_) => 12,
		Meta9::Default(t) => *t,
	};
	Entry9 {
		key,
		r#type: ty,
		value,
	}
}

fn int(v: &Meta8) -> Option<i32> {
	match v {
		Meta8::V0(n) => Some(*n as i32),
		Meta8::V1(n) => Some(*n as i32),
		Meta8::V2(n) => Some(*n),
		_ => None,
	}
}

const AGEABLE: &[u8] = &[54, 57, 90, 91, 92, 93, 95, 96, 98, 100, 101, 120];

fn remap_1_9(kind: Kind, items: &[Entry8]) -> Vec<Entry9> {
	let living = matches!(kind, Kind::Player | Kind::Mob(_) | Kind::Object(78));
	let mut out = Vec::new();
	for item in items {
		let key = item.anon.key;
		let v = &item.value;
		let mapped = match (key, v) {
			(0, Meta8::V0(flags)) => {
				if living && flags & 0x10 != 0 {
					out.push(entry(5, Meta9::V0(1)));
				}
				Some((0, Meta9::V0(flags & !0x10)))
			}
			(1, _) => int(v).map(|n| (1, Meta9::V1(n))),
			(2, Meta8::V4(s)) => Some((2, Meta9::V3(s.clone()))),
			(3, _) => int(v).map(|n| (3, Meta9::V6(n != 0))),
			(4, _) => int(v).map(|n| (4, Meta9::V6(n != 0))),
			(6, Meta8::V3(h)) if living => Some((6, Meta9::V2(*h))),
			(7, _) if living => int(v).map(|n| (7, Meta9::V1(n))),
			(8, _) if living => int(v).map(|n| (8, Meta9::V6(n != 0))),
			(9, _) if living => int(v).map(|n| (9, Meta9::V1(n))),
			_ => match kind {
				Kind::Player => match (key, v) {
					(10, Meta8::V0(parts)) => Some((12, Meta9::V0(*parts))),
					(17, Meta8::V3(f)) => Some((10, Meta9::V2(*f))),
					(18, _) => int(v).map(|n| (11, Meta9::V1(n))),
					_ => None,
				},
				Kind::Mob(ty) => match (key, v) {
					(15, _) => int(v).map(|n| (10, Meta9::V0(n as i8))),
					(12, _) if AGEABLE.contains(&ty) => int(v).map(|n| (11, Meta9::V6(n < 0))),
					_ => None,
				},
				Kind::Object(2) => match (key, v) {
					(10, Meta8::V5(slot)) => Some((5, Meta9::V5(slot.clone()))),
					_ => None,
				},
				Kind::Object(71) => match (key, v) {
					(8, Meta8::V5(slot)) => Some((5, Meta9::V5(slot.clone()))),
					(9, _) => int(v).map(|n| (6, Meta9::V1(n))),
					_ => None,
				},
				Kind::Object(78) => match (key, v) {
					(10, Meta8::V0(flags)) => Some((10, Meta9::V0(*flags))),
					(11..=16, Meta8::V7(r)) => Some((
						key,
						Meta9::V7(v1_9::entity_metadata_item_v7::Ty {
							pitch: r.pitch,
							yaw: r.yaw,
							roll: r.roll,
						}),
					)),
					_ => None,
				},
				Kind::Object(_) => None,
			},
		};
		if let Some((key, value)) = mapped {
			out.push(entry(key, value));
		}
	}
	out
}

pub fn entity_metadata_1_9(
	src: &v1_8::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<v1_9::entity_metadata::Ty> {
	Ok(remap_1_9(cx.kind(src.entity_id), &src.metadata))
}

pub fn player_metadata_1_9(
	src: &v1_8::play_to_client_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<v1_9::entity_metadata::Ty> {
	cx.spawned(src.entity_id, Kind::Player);
	Ok(remap_1_9(Kind::Player, &src.metadata))
}

pub fn mob_uuid_1_9(
	src: &v1_8::play_to_client_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<u128> {
	cx.spawned(src.entity_id, Kind::Mob(src.r#type));
	Ok(uuid_of(src.entity_id))
}

pub fn mob_metadata_1_9(
	src: &v1_8::play_to_client_packet_spawn_entity_living::Ty,
	_cx: &mut Session,
) -> Result<v1_9::entity_metadata::Ty> {
	Ok(remap_1_9(Kind::Mob(src.r#type), &src.metadata))
}

pub fn object_uuid_1_9(
	src: &v1_8::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<u128> {
	cx.spawned(src.entity_id, Kind::Object(src.r#type as u8));
	Ok(uuid_of(src.entity_id))
}

pub fn painting_uuid_1_9(
	src: &v1_8::play_to_client_packet_spawn_entity_painting::Ty,
	cx: &mut Session,
) -> Result<u128> {
	cx.spawned(src.entity_id, Kind::Object(83));
	Ok(uuid_of(src.entity_id))
}

pub fn object_data_1_9(
	src: &v1_8::play_to_client_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(v1_8::play_to_client_packet_spawn_entity_velocity::virtual_tag(&src.velocity))
}

pub fn object_velocity_1_9(
	src: &v1_8::play_to_client_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<v1_9::vec3i16::Ty> {
	Ok(match &src.velocity {
		v1_8::play_to_client_packet_spawn_entity_velocity::Ty::Default(_, v) => v.clone(),
		v1_8::play_to_client_packet_spawn_entity_velocity::Ty::V0 => {
			v1_9::vec3i16::Ty { x: 0, y: 0, z: 0 }
		}
	})
}

pub fn teleport_id_1_9(
	_src: &v1_8::play_to_client_packet_position::Ty,
	cx: &mut Session,
) -> Result<i32> {
	Ok(cx.next_teleport_id())
}

pub fn effect_flags_1_9(
	src: &v1_8::play_to_client_packet_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i8> {
	Ok(if src.hide_particles { 0 } else { 2 })
}

pub fn effect_hide_particles_1_8(
	src: &v1_9::play_to_client_packet_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<bool> {
	Ok(src.hide_particles & 2 == 0)
}

pub fn equipment_slot_1_9(
	src: &v1_8::play_to_client_packet_entity_equipment::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(match src.slot {
		0 => 0,
		n => n as i32 + 1,
	})
}

pub fn equipment_slot_1_8(
	src: &v1_9::play_to_client_packet_entity_equipment::Ty,
	_cx: &mut Session,
) -> Result<i16> {
	match src.slot {
		0 => Ok(0),
		1 => Err(Error::Dropped),
		n => Ok(n as i16 - 1),
	}
}

pub fn attach_entity_1_9(
	p: v1_8::play_to_client_packet_attach_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client9>> {
	if p.leash {
		return Ok(vec![Client9::AttachEntity(
			v1_9::play_to_client_packet_attach_entity::Ty {
				entity_id: p.entity_id,
				vehicle_id: p.vehicle_id,
			},
		)]);
	}
	let passengers = |vehicle: i32, cx: &Session| {
		Client9::SetPassengers(v1_9::play_to_client_packet_set_passengers::Ty {
			entity_id: vehicle,
			passengers: cx.passengers(vehicle),
		})
	};
	let mut out = Vec::new();
	if let Some(old) = cx.dismount(p.entity_id) {
		out.push(passengers(old, cx));
	}
	if p.vehicle_id != -1 {
		cx.mount(p.entity_id, p.vehicle_id);
		out.push(passengers(p.vehicle_id, cx));
	}
	Ok(out)
}

const BLOCKS: usize = 4096;
const LIGHT: usize = 2048;
const BIOMES: usize = 256;

fn write_varint(out: &mut Vec<u8>, mut v: u32) {
	loop {
		if v & !0x7f == 0 {
			out.push(v as u8);
			return;
		}
		out.push((v as u8 & 0x7f) | 0x80);
		v >>= 7;
	}
}

fn sections_1_9(bitmap: u16, sky: bool, full: bool, data: &[u8]) -> Result<Vec<u8>> {
	let n = bitmap.count_ones() as usize;
	let need =
		n * (BLOCKS * 2 + LIGHT + if sky { LIGHT } else { 0 }) + if full { BIOMES } else { 0 };
	if data.len() < need {
		return unconvertible(format!(
			"chunk data is {} bytes, expected {need}",
			data.len()
		));
	}
	let (blocks, rest) = data.split_at(n * BLOCKS * 2);
	let (block_light, rest) = rest.split_at(n * LIGHT);
	let (sky_light, rest) = rest.split_at(if sky { n * LIGHT } else { 0 });
	let mut out = Vec::with_capacity(n * (6700 + 2 * LIGHT) + BIOMES);
	for s in 0..n {
		let states: Vec<u64> = (0..BLOCKS)
			.map(|i| {
				let at = (s * BLOCKS + i) * 2;
				u16::from_le_bytes([blocks[at], blocks[at + 1]]) as u64 & 0x1fff
			})
			.collect();
		let mut palette: Vec<u64> = Vec::new();
		for state in &states {
			if palette.len() <= 256 && !palette.contains(state) {
				palette.push(*state);
			}
		}
		let (bits, values): (usize, Vec<u64>) = if palette.len() <= 256 {
			let bits = (usize::BITS - (palette.len() - 1).leading_zeros()).max(4) as usize;
			let index = |v: &u64| {
				palette
					.iter()
					.position(|p| p == v)
					.expect("state in palette") as u64
			};
			(bits, states.iter().map(index).collect())
		} else {
			palette.clear();
			(13, states)
		};
		let mut longs = vec![0u64; (BLOCKS * bits).div_ceil(64)];
		for (i, value) in values.into_iter().enumerate() {
			let bit = i * bits;
			let (word, off) = (bit / 64, bit % 64);
			longs[word] |= value << off;
			if off + bits > 64 {
				longs[word + 1] |= value >> (64 - off);
			}
		}
		out.push(bits as u8);
		write_varint(&mut out, palette.len() as u32);
		for p in &palette {
			write_varint(&mut out, *p as u32);
		}
		write_varint(&mut out, longs.len() as u32);
		for l in longs {
			out.extend_from_slice(&l.to_be_bytes());
		}
		out.extend_from_slice(&block_light[s * LIGHT..(s + 1) * LIGHT]);
		if sky {
			out.extend_from_slice(&sky_light[s * LIGHT..(s + 1) * LIGHT]);
		}
	}
	if full {
		out.extend_from_slice(&rest[..BIOMES]);
	}
	Ok(out)
}

fn column_size(bitmap: u16, sky: bool, full: bool) -> usize {
	let n = bitmap.count_ones() as usize;
	n * (BLOCKS * 2 + LIGHT + if sky { LIGHT } else { 0 }) + if full { BIOMES } else { 0 }
}

fn column_1_9(x: i32, z: i32, full: bool, bitmap: u16, sky: bool, data: &[u8]) -> Result<Client9> {
	if full && bitmap == 0 {
		return Ok(Client9::UnloadChunk(
			v1_9::play_to_client_packet_unload_chunk::Ty {
				chunk_x: x,
				chunk_z: z,
			},
		));
	}
	Ok(Client9::MapChunk(
		v1_9::play_to_client_packet_map_chunk::Ty {
			x,
			z,
			ground_up: full,
			bit_map: bitmap as i32,
			chunk_data: sections_1_9(bitmap, sky, full, data)?,
		},
	))
}

pub fn map_chunk_1_9(
	p: v1_8::play_to_client_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client9>> {
	let sky = p.chunk_data.len() == column_size(p.bit_map, true, p.ground_up);
	Ok(vec![column_1_9(
		p.x,
		p.z,
		p.ground_up,
		p.bit_map,
		sky,
		&p.chunk_data,
	)?])
}

pub fn map_chunk_bulk_1_9(
	p: v1_8::play_to_client_packet_map_chunk_bulk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client9>> {
	let mut at = 0;
	let mut out = Vec::with_capacity(p.meta.len());
	for m in &p.meta {
		let size = column_size(m.bit_map, p.sky_light_sent, true);
		let Some(data) = p.data.get(at..at + size) else {
			return unconvertible("map_chunk_bulk data is shorter than its columns");
		};
		out.push(column_1_9(
			m.x,
			m.z,
			true,
			m.bit_map,
			p.sky_light_sent,
			data,
		)?);
		at += size;
	}
	Ok(out)
}

pub fn update_sign_1_9_4(
	p: v1_9_2::play_to_client_packet_update_sign::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client94>> {
	use nbt::gen::payload::Ty as P;
	let loc = &p.location;
	let compound = vec![
		nbt::Entry::new("id", P::String("Sign".to_owned())),
		nbt::Entry::new("x", P::Int(loc.x)),
		nbt::Entry::new("y", P::Int(loc.y as i32)),
		nbt::Entry::new("z", P::Int(loc.z)),
		nbt::Entry::new("Text1", P::String(p.text1)),
		nbt::Entry::new("Text2", P::String(p.text2)),
		nbt::Entry::new("Text3", P::String(p.text3)),
		nbt::Entry::new("Text4", P::String(p.text4)),
	];
	Ok(vec![Client94::TileEntityData(
		v1_9_4::play_to_client_packet_tile_entity_data::Ty {
			location: p.location,
			action: 9,
			nbt_data: Some(nbt::Entry::new("", P::Compound(compound))),
		},
	)])
}

fn shift_keys(v: v1_9::entity_metadata::Ty, up: bool) -> v1_9::entity_metadata::Ty {
	v.into_iter()
		.filter_map(|mut e| {
			match (up, e.key) {
				(true, k) if k >= 5 => e.key = k + 1,
				(false, 5) => return None,
				(false, k) if k > 5 => e.key = k - 1,
				_ => {}
			}
			Some(e)
		})
		.collect()
}

pub fn metadata_1_10(
	v: v1_9::entity_metadata::Ty,
	_cx: &mut Session,
) -> Result<v1_9::entity_metadata::Ty> {
	Ok(shift_keys(v, true))
}

pub fn metadata_1_9_4(
	v: v1_9::entity_metadata::Ty,
	_cx: &mut Session,
) -> Result<v1_9::entity_metadata::Ty> {
	Ok(shift_keys(v, false))
}

pub fn title_1_11(
	v: v1_10::play_to_client_packet_title::Ty,
	_cx: &mut Session,
) -> Result<v1_11::play_to_client_packet_title::Ty> {
	use v1_10::play_to_client_packet_title::Ty as Old;
	use v1_11::play_to_client_packet_title::Ty as New;
	Ok(match v {
		Old::V0(s) => New::V0(s),
		Old::V1(s) => New::V1(s),
		Old::V2(t) => New::V3(v1_11::play_to_client_packet_title_v3::Ty {
			fade_in: t.fade_in,
			stay: t.stay,
			fade_out: t.fade_out,
		}),
		Old::Default(tag) => New::Default(tag + 1),
	})
}

pub fn title_1_10(
	v: v1_11::play_to_client_packet_title::Ty,
	_cx: &mut Session,
) -> Result<v1_10::play_to_client_packet_title::Ty> {
	use v1_10::play_to_client_packet_title::Ty as Old;
	use v1_11::play_to_client_packet_title::Ty as New;
	Ok(match v {
		New::V0(s) => Old::V0(s),
		New::V1(s) => Old::V1(s),
		New::V2(_) => return Err(Error::Dropped),
		New::V3(t) => Old::V2(v1_10::play_to_client_packet_title_v2::Ty {
			fade_in: t.fade_in,
			stay: t.stay,
			fade_out: t.fade_out,
		}),
		New::Default(tag) => Old::Default(tag - 1),
	})
}

pub fn arm_animation_1_9(
	_p: v1_8::play_to_server_packet_arm_animation::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server9>> {
	Ok(vec![Server9::ArmAnimation(0)])
}

pub fn arm_animation_1_8(hand: i32, _cx: &mut Session) -> Result<Vec<Server8>> {
	Ok(match hand {
		0 => vec![Server8::ArmAnimation(
			v1_8::play_to_server_packet_arm_animation::Ty {},
		)],
		_ => vec![],
	})
}

pub fn use_item_1_8(hand: i32, _cx: &mut Session) -> Result<Vec<Server8>> {
	if hand != 0 {
		return Ok(vec![]);
	}
	Ok(vec![Server8::BlockPlace(
		v1_8::play_to_server_packet_block_place::Ty {
			location: v1_8::position::Ty {
				x: -1,
				y: -1,
				z: -1,
			},
			direction: -1,
			held_item: v1_8::slot::Ty::_1,
			cursor_x: 0,
			cursor_y: 0,
			cursor_z: 0,
		},
	)])
}

pub fn held_item_1_8(
	_src: &v1_9::play_to_server_packet_block_place::Ty,
	_cx: &mut Session,
) -> Result<v1_8::slot::Ty> {
	Ok(v1_8::slot::Ty::_1)
}

pub fn resource_pack_receive_1_10(
	p: v1_9_4::play_to_server_packet_resource_pack_receive::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server10>> {
	Ok(vec![Server10::ResourcePackReceive(p.result)])
}

pub fn resource_pack_receive_1_9_4(result: i32, _cx: &mut Session) -> Result<Vec<Server94>> {
	Ok(vec![Server94::ResourcePackReceive(
		v1_9_4::play_to_server_packet_resource_pack_receive::Ty {
			hash: String::new(),
			result,
		},
	)])
}
