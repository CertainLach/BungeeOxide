use std::collections::{HashMap, VecDeque};

use super::chunk::{self, Shape};
use super::ids;
use crate::custom::chunks::{legacy_column, legacy_data, Column};
use crate::custom::window::{Dialect, Emit, Teleport, Window};
use crate::gen::convert::{v1_1, v1_2_3, Root, Version};
use crate::pds::convert::{Error, Result};
use crate::pds::Output;
use crate::session::Session;

type Client11 = v1_1::play_to_client_packet::Ty;
type Server12 = v1_2_3::play_to_server_packet::Ty;
type Painting = v1_1::play_to_client_packet_spawn_entity_painting::Ty;
type Sign = v1_1::play_to_client_packet_update_sign::Ty;

const WINDOW: i32 = 128;
const HEIGHT: i32 = 256;
const EYES: f64 = 1.6200000047683716;
const BACKLOG: usize = 2;
const SENT: usize = 16;
const NEAR: f64 = 0.01;

pub(super) struct Old;

impl Dialect for Old {
	type Painting = Painting;
	const AT: Version = Version::V1_2_3;

	fn air() -> u32 {
		0
	}

	fn is_air(state: u32) -> bool {
		state >> 4 == 0
	}
}

#[derive(Default)]
struct Heights {
	w: Window<Old>,
	active: bool,
	ids: i32,
	sent: VecDeque<(i32, f64, f64, f64)>,
	client: (f64, f64, f64),
	look: (f32, f32),
	signs: HashMap<(i32, i32, i32), Sign>,
}

fn heights(cx: &mut Session) -> &mut Heights {
	cx.state::<Heights>()
}

pub(super) fn active(cx: &mut Session) -> bool {
	heights(cx).active
}

fn reset(cx: &mut Session) {
	let server = cx
		.server_version()
		.is_some_and(|s| (Version::V1_2_3..Version::V1_17).contains(&s));
	let client = cx.client_version().is_some_and(|c| c <= Version::V1_1);
	let st = heights(cx);
	*st = Heights {
		ids: st.ids,
		active: server && client,
		..Heights::default()
	};
	if st.active {
		st.w.reset(0, HEIGHT, WINDOW);
	}
}

pub fn prenetty_login_dimension_1_1(
	p: &v1_2_3::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<i8> {
	reset(cx);
	Ok(p.dimension as i8)
}

pub fn prenetty_respawn_dimension_1_1(
	p: &v1_2_3::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<i8> {
	reset(cx);
	Ok(p.dimension as i8)
}

fn fixed(v: f64) -> i32 {
	(v * 32.0).floor() as i32
}

fn column_packets(col: &Column) -> Result<Vec<Client11>> {
	let (mask, add, sky, raw) = legacy_data(col);
	let shape = Shape {
		bitmap: mask as u16,
		add: add as u16,
		sky,
		full: col.full,
	};
	let olds = if col.full {
		vec![chunk::old_full(col.x, col.z, &raw, shape, Version::V1_1)?]
	} else {
		chunk::old_sections(col.x, col.z, &raw, shape, Version::V1_1)?
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

fn signs_in(st: &Heights, x: i32, z: i32) -> Vec<Client11> {
	let view = st.w.view();
	let mut signs: Vec<&Sign> = st
		.signs
		.iter()
		.filter(|((sx, sy, sz), _)| sx >> 4 == x && sz >> 4 == z && st.w.inside(*sy))
		.map(|(_, s)| s)
		.collect();
	signs.sort_by_key(|s| (s.location.x, s.location.y, s.location.z));
	signs
		.into_iter()
		.map(|s| {
			let mut s = s.clone();
			s.location.y = (s.location.y as i32 - view) as i16;
			Client11::UpdateSign(s)
		})
		.collect()
}

fn client_packets(
	st: &mut Heights,
	emits: Vec<Emit<Painting>>,
	eyes: f64,
) -> Result<Vec<Client11>> {
	let view = st.w.view();
	let mut out = Vec::new();
	for e in emits {
		match e {
			Emit::Column(col) => {
				out.extend(column_packets(&col)?);
				out.extend(signs_in(st, col.x, col.z));
			}
			Emit::Teleport(t) => {
				let (x, y, z, yaw, pitch) = match t.flags & 0x1f == 0x1f {
					true => (
						st.client.0 + t.x,
						st.client.1 + t.y,
						st.client.2 + t.z,
						st.look.0,
						st.look.1,
					),
					false => (t.x, t.y, t.z, t.yaw, t.pitch),
				};
				st.sent.push_back((t.id, x, y, z));
				if st.sent.len() > SENT {
					st.sent.pop_front();
				}
				st.client = (x, y, z);
				out.push(Client11::Position(
					v1_1::play_to_client_packet_position::Ty {
						x,
						y: y + eyes,
						stance: y,
						z,
						yaw,
						pitch,
						on_ground: false,
					},
				));
			}
			Emit::Vehicle { .. } => {}
			Emit::Entity(m) => out.push(Client11::EntityTeleport(
				v1_1::play_to_client_packet_entity_teleport::Ty {
					entity_id: m.id,
					x: fixed(m.x),
					y: fixed(m.y),
					z: fixed(m.z),
					yaw: m.yaw,
					pitch: m.pitch,
				},
			)),
			Emit::Painting(id, mut p) => {
				p.location.y -= view;
				out.push(Client11::EntityDestroy(id));
				out.push(Client11::SpawnEntityPainting(p));
			}
			Emit::Spawn(x, y, z) => {
				out.push(Client11::SpawnPosition(v1_1::position_iii::Ty { x, y, z }))
			}
		}
	}
	Ok(out)
}

fn send(cx: &mut Session, packets: Vec<Client11>) -> Result<()> {
	for p in packets {
		let mut o = Output::new();
		v1_1::play_to_client_packet::serialize(&p, &mut o)
			.map_err(|e| Error::Encoding(format!("{e:?}")))?;
		cx.send(Root::PlayToClient, Version::V1_1, o.into_inner());
	}
	Ok(())
}

fn backlog(st: &mut Heights) -> Result<Vec<Client11>> {
	let emits = st.w.backlog(BACKLOG);
	client_packets(st, emits, EYES)
}

pub(super) fn map_chunk(
	p: &v1_2_3::play_to_client_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<Option<Vec<Client11>>> {
	let st = heights(cx);
	if p.ground_up && p.bit_map == 0 {
		st.w.unload(p.x, p.z);
		st.signs
			.retain(|(x, _, z), _| (x >> 4, z >> 4) != (p.x, p.z));
		return Ok(None);
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
	let col = legacy_column(
		p.x,
		p.z,
		p.ground_up,
		Old::AT,
		&raw,
		p.bit_map as u32,
		p.add_bit_map as u32,
		shape.sky,
	)?;
	let Some(out) = st.w.column(col) else {
		return Ok(Some(vec![]));
	};
	client_packets(st, vec![Emit::Column(out)], EYES).map(Some)
}

fn block(state: u32) -> (u8, u8) {
	let (id, meta) = ids::block((state >> 4) as u16, (state & 15) as u8, Version::V1_1);
	(id as u8, meta)
}

pub(super) fn multi_block_change(
	p: v1_2_3::play_to_client_packet_multi_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let changes: Vec<(i32, i32, i32, u32)> = p
		.records
		.iter()
		.map(|r| {
			(
				p.chunk_x * 16 + (r.horizontal_pos >> 4) as i32,
				r.y as i32,
				p.chunk_z * 16 + (r.horizontal_pos & 15) as i32,
				r.block_id as u32,
			)
		})
		.collect();
	let shown = heights(cx).w.blocks_changed(&changes);
	if shown.is_empty() {
		return Ok(vec![]);
	}
	let mut coordinates = Vec::with_capacity(shown.len());
	let (mut ids, mut metadata) = (
		Vec::with_capacity(shown.len()),
		Vec::with_capacity(shown.len()),
	);
	for (x, y, z, state) in shown {
		let (id, meta) = block(state);
		coordinates.push(
			v1_1::play_to_client_packet_multi_block_change_coordinates_items::Ty {
				x: (x & 15) as u8,
				z: (z & 15) as u8,
				y: y as u8,
			},
		);
		ids.push(id);
		metadata.push(meta);
	}
	Ok(vec![Client11::MultiBlockChange(
		v1_1::play_to_client_packet_multi_block_change::Ty {
			chunk_x: p.chunk_x,
			chunk_z: p.chunk_z,
			coordinates,
			block_ids: ids,
			metadata,
		},
	)])
}

pub(super) fn block_change(
	mut p: v1_2_3::play_to_client_packet_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let state = (p.r#type as u32) << 4 | (p.metadata & 15) as u32;
	let Some((y, state)) =
		heights(cx)
			.w
			.block_changed(p.location.x, p.location.y as i32, p.location.z, state)
	else {
		return Ok(vec![]);
	};
	p.location.y = y as u8;
	(p.r#type, p.metadata) = block(state);
	Ok(vec![Client11::BlockChange(p)])
}

pub(super) fn spawned(
	cx: &mut Session,
	id: i32,
	(x, y, z): (i32, &mut i32, i32),
	look: Option<(i8, i8)>,
) {
	let st = heights(cx);
	if st.active {
		st.w.moved(
			id,
			x as f64 / 32.0,
			*y as f64 / 32.0,
			z as f64 / 32.0,
			look,
			None,
		);
		*y -= st.w.view() * 32;
	}
}

pub fn prenetty_position_1_1(
	p: v1_2_3::play_to_client_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let st = heights(cx);
	if !st.active {
		return Ok(vec![Client11::Position(p)]);
	}
	st.ids = (st.ids + 1) & 0x00ff_ffff;
	let emits = st.w.server_teleport(Teleport {
		x: p.x,
		y: p.stance,
		z: p.z,
		yaw: p.yaw,
		pitch: p.pitch,
		flags: 0,
		id: st.ids,
	});
	st.look = (p.yaw, p.pitch);
	client_packets(st, emits, p.y - p.stance)
}

pub fn prenetty_position_only_1_1(
	mut p: v1_2_3::play_to_client_packet_position_only::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let st = heights(cx);
	if st.active {
		let view = st.w.view() as f64;
		(p.y, p.stance) = (p.y - view, p.stance - view);
	}
	Ok(vec![Client11::PositionOnly(p)])
}

pub fn prenetty_spawn_position_1_1(
	mut p: v1_2_3::position_iii::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let st = heights(cx);
	if st.active {
		p.y = st.w.spawn(p.x, p.y, p.z);
	}
	Ok(vec![Client11::SpawnPosition(p)])
}

fn inside(cx: &mut Session, y: i32) -> Option<i32> {
	let st = heights(cx);
	match st.active {
		true => st.w.inside(y).then(|| y - st.w.view()),
		false => Some(y),
	}
}

pub fn prenetty_bed_1_1(
	mut p: v1_2_3::play_to_client_packet_bed::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let Some(y) = inside(cx, p.location.y as i32) else {
		return Ok(vec![]);
	};
	p.location.y = y as u8;
	Ok(vec![Client11::Bed(p)])
}

pub fn prenetty_world_event_1_1(
	mut p: v1_2_3::play_to_client_packet_world_event::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let Some(y) = inside(cx, p.location.y as i32) else {
		return Ok(vec![]);
	};
	p.location.y = y as u8;
	Ok(vec![Client11::WorldEvent(p)])
}

pub fn prenetty_block_action_1_1(
	mut p: v1_2_3::play_to_client_packet_block_action::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let Some(y) = inside(cx, p.location.y as i32) else {
		return Ok(vec![]);
	};
	p.location.y = y as i16;
	Ok(vec![Client11::BlockAction(p)])
}

pub fn prenetty_update_sign_1_1(
	mut p: v1_2_3::play_to_client_packet_update_sign::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let st = heights(cx);
	if !st.active {
		return Ok(vec![Client11::UpdateSign(p)]);
	}
	let (x, y, z) = (p.location.x, p.location.y as i32, p.location.z);
	st.signs.insert((x, y, z), p.clone());
	if !st.w.inside(y) || !st.w.shown(x >> 4, z >> 4) {
		return Ok(vec![]);
	}
	p.location.y = (y - st.w.view()) as i16;
	Ok(vec![Client11::UpdateSign(p)])
}

pub fn prenetty_explosion_1_1(
	mut p: v1_2_3::play_to_client_packet_explosion::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let st = heights(cx);
	if st.active {
		p.y -= st.w.view() as f64;
	}
	Ok(vec![Client11::Explosion(p)])
}

pub fn prenetty_spawn_entity_painting_1_1(
	p: v1_2_3::play_to_client_packet_spawn_entity_painting::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let st = heights(cx);
	if !st.active {
		return Ok(vec![Client11::SpawnEntityPainting(p)]);
	}
	st.w.painting(p.entity_id, p.clone());
	let mut p = p;
	p.location.y -= st.w.view();
	Ok(vec![Client11::SpawnEntityPainting(p)])
}

pub fn prenetty_spawn_entity_experience_orb_1_1(
	mut p: v1_2_3::play_to_client_packet_spawn_entity_experience_orb::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	spawned(cx, p.entity_id, (p.x, &mut p.y, p.z), None);
	Ok(vec![Client11::SpawnEntityExperienceOrb(p)])
}

pub fn prenetty_spawn_entity_weather_1_1(
	mut p: v1_2_3::play_to_client_packet_spawn_entity_weather::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let st = heights(cx);
	if st.active {
		p.y -= st.w.view() * 32;
	}
	Ok(vec![Client11::SpawnEntityWeather(p)])
}

pub fn prenetty_entity_teleport_1_1(
	mut p: v1_2_3::play_to_client_packet_entity_teleport::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let st = heights(cx);
	if !st.active {
		return Ok(vec![Client11::EntityTeleport(p)]);
	}
	st.w.moved(
		p.entity_id,
		p.x as f64 / 32.0,
		p.y as f64 / 32.0,
		p.z as f64 / 32.0,
		Some((p.yaw, p.pitch)),
		None,
	);
	p.y -= st.w.view() * 32;
	let mut out = vec![Client11::EntityTeleport(p)];
	out.extend(backlog(st)?);
	Ok(out)
}

fn moved_by(
	cx: &mut Session,
	id: i32,
	d: (i8, i8, i8),
	look: Option<(i8, i8)>,
) -> Result<Vec<Client11>> {
	let st = heights(cx);
	if !st.active {
		return Ok(vec![]);
	}
	st.w.moved_by(
		id,
		(d.0 as f64 / 32.0, d.1 as f64 / 32.0, d.2 as f64 / 32.0),
		look,
		false,
	);
	backlog(st)
}

pub fn prenetty_rel_entity_move_1_1(
	p: v1_2_3::play_to_client_packet_rel_entity_move::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let mut out = moved_by(cx, p.entity_id, (p.d_x, p.d_y, p.d_z), None)?;
	out.insert(0, Client11::RelEntityMove(p));
	Ok(out)
}

pub fn prenetty_entity_move_look_1_1(
	p: v1_2_3::play_to_client_packet_entity_move_look::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let mut out = moved_by(
		cx,
		p.entity_id,
		(p.d_x, p.d_y, p.d_z),
		Some((p.yaw, p.pitch)),
	)?;
	out.insert(0, Client11::EntityMoveLook(p));
	Ok(out)
}

pub fn prenetty_entity_look_1_1(
	p: v1_2_3::play_to_client_packet_entity_look::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let st = heights(cx);
	let mut out = vec![];
	if st.active {
		st.w.looked(p.entity_id, (p.yaw, p.pitch), false);
		out = backlog(st)?;
	}
	out.insert(0, Client11::EntityLook(p));
	Ok(out)
}

pub fn prenetty_entity_destroy_1_1(id: i32, cx: &mut Session) -> Result<Vec<Client11>> {
	heights(cx).w.forget(id);
	Ok(vec![Client11::EntityDestroy(id)])
}

pub fn prenetty_pre_chunk_1_1(
	p: v1_2_3::play_to_client_packet_pre_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client11>> {
	let st = heights(cx);
	if st.active && !p.mode {
		st.w.unload(p.x, p.z);
		st.signs
			.retain(|(x, _, z), _| (x >> 4, z >> 4) != (p.x, p.z));
	}
	Ok(vec![Client11::PreChunk(p)])
}

fn echoed(st: &mut Heights, x: f64, y: f64, z: f64) {
	let near = |a: f64, b: f64| (a - b).abs() < NEAR;
	if let Some(i) = st
		.sent
		.iter()
		.rposition(|(_, sx, sy, sz)| near(*sx, x) && near(*sy, y) && near(*sz, z))
	{
		let id = st.sent[i].0;
		st.sent.drain(..=i);
		st.w.confirmed(id);
	}
}

fn moved(cx: &mut Session, x: f64, y: f64, z: f64) -> Result<f64> {
	let st = heights(cx);
	if !st.active {
		return Ok(y);
	}
	echoed(st, x, y, z);
	st.client = (x, y, z);
	let (server_y, emits) = st.w.client_moved(x, y, z, None);
	let packets = match emits.is_empty() {
		true => backlog(st)?,
		false => client_packets(st, emits, EYES)?,
	};
	send(cx, packets)?;
	Ok(server_y)
}

fn idle(cx: &mut Session) -> Result<()> {
	let st = heights(cx);
	if !st.active {
		return Ok(());
	}
	let packets = backlog(st)?;
	send(cx, packets)
}

pub fn prenetty_position_1_2_3(
	mut p: v1_1::play_to_server_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<Server12>> {
	let y = moved(cx, p.x, p.y, p.z)?;
	(p.stance, p.y) = (p.stance + y - p.y, y);
	Ok(vec![Server12::Position(p)])
}

pub fn prenetty_position_look_1_2_3(
	mut p: v1_1::play_to_server_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<Vec<Server12>> {
	heights(cx).look = (p.yaw, p.pitch);
	let y = moved(cx, p.x, p.y, p.z)?;
	(p.stance, p.y) = (p.stance + y - p.y, y);
	Ok(vec![Server12::PositionLook(p)])
}

pub fn prenetty_look_1_2_3(
	p: v1_1::play_to_server_packet_look::Ty,
	cx: &mut Session,
) -> Result<Vec<Server12>> {
	heights(cx).look = (p.yaw, p.pitch);
	idle(cx)?;
	Ok(vec![Server12::Look(p)])
}

pub fn prenetty_flying_1_2_3(on_ground: bool, cx: &mut Session) -> Result<Vec<Server12>> {
	idle(cx)?;
	Ok(vec![Server12::Flying(on_ground)])
}

fn input(cx: &mut Session, y: i32) -> i32 {
	let st = heights(cx);
	match st.active {
		true => y + st.w.input(),
		false => y,
	}
}

pub fn prenetty_block_dig_1_2_3(
	mut p: v1_1::play_to_server_packet_block_dig::Ty,
	cx: &mut Session,
) -> Result<Vec<Server12>> {
	if (0..4).contains(&p.status) {
		p.location.y = input(cx, p.location.y as i32).clamp(0, 255) as u8;
	}
	Ok(vec![Server12::BlockDig(p)])
}

pub fn prenetty_place_1_2_3(
	mut p: v1_1::play_to_server_packet_block_place::Ty,
	cx: &mut Session,
) -> Result<Vec<Server12>> {
	if p.direction != -1 {
		p.location.y = input(cx, p.location.y as i32).clamp(0, 255) as u8;
	}
	Ok(vec![Server12::BlockPlace(p)])
}

pub fn prenetty_update_sign_server_1_2_3(
	mut p: v1_1::play_to_server_packet_update_sign::Ty,
	cx: &mut Session,
) -> Result<Vec<Server12>> {
	p.location.y = input(cx, p.location.y as i32) as i16;
	Ok(vec![Server12::UpdateSign(p)])
}
