use std::sync::OnceLock;

use super::chunk;
use super::meta::Pos;
use crate::custom::chunks::{
	decode, encode_chunk, encode_light, settle, Column, Heightmap, Packet, Virtual,
};
use crate::custom::window::{Dialect, Emit, Window};
use crate::gen::convert::{v1_16_2, v1_17, Carrier, Root, Version};
use crate::pds::convert::{unconvertible, Error, Result};
use crate::pds::{Input, Output};
use crate::session::Session;

const OLD: Version = Version::V1_16_2;
const NEW: Version = Version::V1_17;
const LAYER: usize = 256;

pub(super) type Out = v1_16_2::play_to_client_packet::Ty;
pub(super) type Painting = v1_16_2::play_to_client_packet_spawn_entity_painting::Ty;
pub(super) type Tall = Window<Modern>;
type New = v1_17::play_to_client_packet::Ty;

pub(super) struct Modern;

fn airs() -> &'static [u32] {
	static AIRS: OnceLock<&'static [u32]> = OnceLock::new();
	AIRS.get_or_init(|| chunk::airs(OLD))
}

impl Dialect for Modern {
	type Painting = Painting;
	const AT: Version = OLD;

	fn air() -> u32 {
		airs().first().copied().unwrap_or(0)
	}

	fn is_air(state: u32) -> bool {
		airs().contains(&state)
	}

	fn heightmaps(
		maps: &[Heightmap],
		shift: i32,
		height: i32,
		window: i32,
		floor: &[bool],
	) -> Vec<Heightmap> {
		let (from, to) = (chunk::bits_for(height as usize + 1), chunk::bits_for(257));
		maps.iter()
			.map(|h| {
				let values: Vec<u32> = chunk::unpack(&h.data, from, LAYER, false)
					.into_iter()
					.enumerate()
					.map(|(i, v)| {
						let v = (v as i32 - shift).clamp(0, window);
						(if floor[i] { v.max(1) } else { v }) as u32
					})
					.collect();
				Heightmap {
					kind: h.kind.clone(),
					data: chunk::pack(&values, to, false),
				}
			})
			.collect()
	}
}

pub(super) fn window_for(cx: &Session) -> i32 {
	match cx.client_version() {
		Some(c) if c <= Version::V1_1 => 128,
		_ => 256,
	}
}

fn encoding(e: crate::pds::Error) -> Error {
	Error::Encoding(format!("{e:?}"))
}

pub(super) fn framed(p: &Out) -> Result<Vec<u8>> {
	let mut o = Output::new();
	v1_16_2::play_to_client_packet::serialize(p, &mut o).map_err(encoding)?;
	Ok(o.into_inner())
}

fn typed(bytes: &[u8]) -> Result<Out> {
	v1_16_2::play_to_client_packet::deserialize(&mut Input::new(bytes)).map_err(encoding)
}

fn packets(e: Emit<Painting>, view: i32) -> Vec<Out> {
	match e {
		Emit::Column(_) => vec![],
		Emit::Teleport(t) => vec![Out::Position(v1_16_2::play_to_client_packet_position::Ty {
			x: t.x,
			y: t.y,
			z: t.z,
			yaw: t.yaw,
			pitch: t.pitch,
			flags: t.flags,
			teleport_id: t.id,
		})],
		Emit::Vehicle {
			x,
			y,
			z,
			yaw,
			pitch,
		} => vec![Out::VehicleMove(
			v1_16_2::play_to_client_packet_vehicle_move::Ty {
				x,
				y,
				z,
				yaw,
				pitch,
			},
		)],
		Emit::Entity(m) => vec![Out::EntityTeleport(
			v1_16_2::play_to_client_packet_entity_teleport::Ty {
				entity_id: m.id,
				x: m.x,
				y: m.y,
				z: m.z,
				yaw: m.yaw,
				pitch: m.pitch,
				on_ground: m.ground,
			},
		)],
		Emit::Painting(id, p) => vec![
			Out::EntityDestroy(vec![id]),
			Out::SpawnEntityPainting(shifted_painting(p, view)),
		],
		Emit::Spawn(x, y, z) => vec![Out::SpawnPosition(Pos { x, y: y as i16, z })],
	}
}

pub(super) fn shifted_painting(mut p: Painting, view: i32) -> Painting {
	p.location.y = (p.location.y as i32 - view) as i16;
	p
}

pub(super) fn typed_emits(emits: Vec<Emit<Painting>>, view: i32) -> Result<Vec<Out>> {
	let mut out = Vec::new();
	for e in emits {
		match e {
			Emit::Column(col) => {
				for bytes in encode_chunk(Modern::AT, &col)? {
					out.push(typed(&bytes)?);
				}
			}
			e => out.extend(packets(e, view)),
		}
	}
	Ok(out)
}

pub(super) fn send(cx: &mut Session, emits: Vec<Emit<Painting>>, view: i32) -> Result<()> {
	for e in emits {
		match e {
			Emit::Column(col) => cx.send_model(
				Root::PlayToClient,
				Modern::AT,
				Virtual::Chunk(Box::new(col)),
			),
			e => {
				for p in packets(e, view) {
					cx.send(Root::PlayToClient, Modern::AT, framed(&p)?);
				}
			}
		}
	}
	Ok(())
}

pub(super) fn carriers(emits: Vec<Emit<Painting>>, view: i32) -> Result<Vec<Carrier>> {
	let mut out = Vec::new();
	for e in emits {
		match e {
			Emit::Column(col) => out.push(Carrier::Virtual(Virtual::Chunk(Box::new(col)))),
			e => {
				for p in packets(e, view) {
					out.push(Carrier::Wire(framed(&p)?));
				}
			}
		}
	}
	Ok(out)
}

fn remapped(mut col: Column) -> Column {
	if let Some(t) = chunk::state_table(NEW, OLD) {
		let air = Modern::air();
		for s in col.sections.iter_mut().flatten() {
			for v in s.states.iter_mut() {
				*v = t.get(*v as usize).copied().unwrap_or(air);
			}
		}
	}
	col.space = Modern::AT;
	col.at = Modern::AT;
	col
}

pub(super) fn native(mut col: Column, t: &mut Tall) -> Result<Vec<Carrier>> {
	settle(&mut col, Modern::AT);
	col.at = Modern::AT;
	Ok(t.column(col)
		.map(|c| Carrier::Virtual(Virtual::Chunk(Box::new(c))))
		.into_iter()
		.collect())
}

pub(super) fn wire_chunk(
	p: v1_17::play_to_client_packet_map_chunk::Ty,
	t: &mut Tall,
) -> Result<Vec<Out>> {
	let mut o = Output::new();
	v1_17::play_to_client_packet::serialize(&New::MapChunk(p), &mut o).map_err(encoding)?;
	let Some(Packet::Chunk(col)) = decode(NEW, &o.into_inner())? else {
		return unconvertible("1.17 map_chunk does not decode");
	};
	match t.column(remapped(col)) {
		Some(col) => typed_emits(vec![Emit::Column(col)], t.view()),
		None => Ok(vec![]),
	}
}

pub(super) fn wire_light(
	v: v1_17::play_to_client_packet_update_light::Ty,
	t: &mut Tall,
) -> Result<v1_16_2::play_to_client_packet_update_light::Ty> {
	let mut o = Output::new();
	v1_17::play_to_client_packet::serialize(&New::UpdateLight(v), &mut o).map_err(encoding)?;
	let Some(Packet::Light(x, z, light)) = decode(NEW, &o.into_inner())? else {
		return unconvertible("1.17 update_light does not decode");
	};
	let shown = t.light(x, z, &light);
	match typed(&encode_light(Modern::AT, x, z, &shown)?)? {
		Out::UpdateLight(l) => Ok(l),
		_ => unconvertible("1.16.2 update_light does not decode"),
	}
}
