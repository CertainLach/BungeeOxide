mod chunk;
pub(super) mod item;
mod meta;
pub(super) mod text;

use std::collections::{HashMap, HashSet};

use crate::gen::convert::{v1_7, v1_8};
use crate::pds::convert::{Error, Result};
use crate::pds::{Input, Output};
use crate::session::{Kind, Session};

pub use item::{legacy_slot_1_8, slot_1_7};

type Client7 = v1_7::play_toClient_packet::Ty;
type Client8 = v1_8::play_toClient_packet::Ty;
type Server7 = v1_7::play_toServer_packet::Ty;
type Server8 = v1_8::play_toServer_packet::Ty;
type Pos8 = v1_8::position::Ty;
type Prop8 =
	v1_8::play_toClient_packet_player_info__data__items__ANON__add_player__properties__items::Ty;
type Icon8 = v1_8::play_toClient_packet_map__icons__items::Ty;

const EYES: f64 = 1.6200000047683716;

struct Profile {
	name: String,
	properties: Vec<Prop8>,
	listed: Option<String>,
	ping: i32,
}

#[derive(Default)]
struct Player {
	x: f64,
	y: f64,
	z: f64,
	yaw: f32,
	pitch: f32,
	on_ground: bool,
}

#[derive(Default)]
struct Legacy {
	profiles: HashMap<u128, Profile>,
	tab: HashMap<String, u128>,
	windows: HashMap<u8, u8>,
	cook: HashMap<u8, i16>,
	offsets: HashMap<i32, i32>,
	hidden: HashSet<i32>,
	maps: HashMap<i32, (i8, Vec<Icon8>)>,
	player: Player,
}

fn state(cx: &mut Session) -> &mut Legacy {
	cx.state::<Legacy>()
}

fn uuid_string(u: u128) -> String {
	let h = format!("{u:032x}");
	format!(
		"{}-{}-{}-{}-{}",
		&h[..8],
		&h[8..12],
		&h[12..16],
		&h[16..20],
		&h[20..]
	)
}

fn parse_uuid(s: &str) -> Option<u128> {
	let hex: String = s.chars().filter(|c| *c != '-').collect();
	(hex.len() == 32)
		.then(|| u128::from_str_radix(&hex, 16).ok())
		.flatten()
}

fn name_uuid(name: &str) -> u128 {
	let hash = |seed: u64| {
		name.bytes().fold(seed, |h, b| {
			(h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
		})
	};
	let u = (hash(0xcbf2_9ce4_8422_2325) as u128) << 64 | hash(0x6c62_272e_07bb_0142) as u128;
	u & !(0xf << 76) & !(0x3 << 62) | 0x3 << 76 | 0x2 << 62
}

fn read_varint(input: &Input) -> Option<i32> {
	let mut v = 0u32;
	for shift in (0..35).step_by(7) {
		let b = input.read().ok()?;
		v |= ((b & 0x7f) as u32) << shift;
		if b & 0x80 == 0 {
			return Some(v as i32);
		}
	}
	None
}

fn write_varint(out: &mut Vec<u8>, v: i32) {
	let mut v = v as u32;
	loop {
		if v & !0x7f == 0 {
			out.push(v as u8);
			return;
		}
		out.push((v as u8 & 0x7f) | 0x80);
		v >>= 7;
	}
}

fn read_string(input: &Input) -> Option<String> {
	let n = read_varint(input)?;
	let bytes = (0..n)
		.map(|_| input.read().ok())
		.collect::<Option<Vec<u8>>>()?;
	String::from_utf8(bytes).ok()
}

fn string_bytes(s: &str) -> Vec<u8> {
	let mut out = Vec::with_capacity(s.len() + 2);
	write_varint(&mut out, s.len() as i32);
	out.extend_from_slice(s.as_bytes());
	out
}

fn take(input: &Input, n: usize) -> Option<Vec<u8>> {
	(0..n).map(|_| input.read().ok()).collect()
}

fn clamp16(v: i32) -> i16 {
	v.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

pub fn legacy_position_from_iii_1_8(v: v1_7::position_iii::Ty, _cx: &mut Session) -> Result<Pos8> {
	Ok(Pos8 {
		x: v.x,
		y: v.y as i16,
		z: v.z,
	})
}

pub fn legacy_position_from_ibi_1_8(v: v1_7::position_ibi::Ty, _cx: &mut Session) -> Result<Pos8> {
	let y = if (v.x, v.y, v.z) == (-1, 255, -1) {
		-1
	} else {
		v.y as i16
	};
	Ok(Pos8 { x: v.x, y, z: v.z })
}

pub fn legacy_position_from_isi_1_8(v: v1_7::position_isi::Ty, _cx: &mut Session) -> Result<Pos8> {
	Ok(Pos8 {
		x: v.x,
		y: v.y,
		z: v.z,
	})
}

pub fn position_iii_1_7(v: Pos8, _cx: &mut Session) -> Result<v1_7::position_iii::Ty> {
	Ok(v1_7::position_iii::Ty {
		x: v.x,
		y: v.y as i32,
		z: v.z,
	})
}

pub fn position_ibi_1_7(v: Pos8, _cx: &mut Session) -> Result<v1_7::position_ibi::Ty> {
	Ok(v1_7::position_ibi::Ty {
		x: v.x,
		y: v.y as u8,
		z: v.z,
	})
}

pub fn position_isi_1_7(v: Pos8, _cx: &mut Session) -> Result<v1_7::position_isi::Ty> {
	Ok(v1_7::position_isi::Ty {
		x: v.x,
		y: v.y,
		z: v.z,
	})
}

pub fn chat_1_7(p: v1_8::play_toClient_packet_chat::Ty, _cx: &mut Session) -> Result<Vec<Client7>> {
	if p.position == 2 {
		return Ok(vec![]);
	}
	Ok(vec![Client7::chat(text::chat_1_7(&p.message))])
}

pub fn legacy_chat_1_8(message: String, _cx: &mut Session) -> Result<Vec<Client8>> {
	Ok(vec![Client8::chat(v1_8::play_toClient_packet_chat::Ty {
		message,
		position: 0,
	})])
}

pub fn entity_destroy_1_7(ids: Vec<i32>, cx: &mut Session) -> Result<Vec<Client7>> {
	let st = state(cx);
	for id in &ids {
		st.offsets.remove(id);
		st.hidden.remove(id);
	}
	Ok(ids
		.chunks(i8::MAX as usize)
		.map(|c| Client7::entity_destroy(c.to_vec()))
		.collect())
}

pub fn legacy_entity_destroy_1_8(ids: Vec<i32>, cx: &mut Session) -> Result<Vec<Client8>> {
	let st = state(cx);
	for id in &ids {
		st.offsets.remove(id);
	}
	Ok(vec![Client8::entity_destroy(ids)])
}

pub fn resource_pack_1_7(
	p: v1_8::play_toClient_packet_resource_pack_send::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client7>> {
	Ok(vec![Client7::custom_payload(
		v1_7::play_toClient_packet_custom_payload::Ty {
			channel: "MC|RPack".to_owned(),
			data: p.url.into_bytes(),
		},
	)])
}

pub fn map_chunk_1_7(
	p: v1_8::play_toClient_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client7>> {
	Ok(vec![Client7::map_chunk(chunk::single_1_7(p)?)])
}

pub fn legacy_map_chunk_1_8(
	p: v1_7::play_toClient_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client8>> {
	Ok(vec![Client8::map_chunk(chunk::single_1_8(p)?)])
}

pub fn map_chunk_bulk_1_7(
	p: v1_8::play_toClient_packet_map_chunk_bulk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client7>> {
	Ok(vec![Client7::map_chunk_bulk(chunk::bulk_1_7(p)?)])
}

pub fn legacy_map_chunk_bulk_1_8(
	p: v1_7::play_toClient_packet_map_chunk_bulk::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client8>> {
	Ok(vec![Client8::map_chunk_bulk(chunk::bulk_1_8(p)?)])
}

pub fn record_state_1_7(
	src: &v1_8::play_toClient_packet_multi_block_change__records__items::Ty,
	_cx: &mut Session,
) -> Result<u16> {
	Ok(item::state_1_7(src.blockId as u16))
}

pub fn records_length_1_7(
	src: &v1_8::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(src.records.len() as i32 * 4)
}

pub fn block_change_1_7(
	p: v1_8::play_toClient_packet_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	let s = item::state_1_7(p.r#type as u16);
	Ok(vec![Client7::block_change(
		v1_7::play_toClient_packet_block_change::Ty {
			location: position_ibi_1_7(p.location, cx)?,
			r#type: (s >> 4) as i32,
			metadata: (s & 15) as u8,
		},
	)])
}

pub fn legacy_block_change_1_8(
	p: v1_7::play_toClient_packet_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	Ok(vec![Client8::block_change(
		v1_8::play_toClient_packet_block_change::Ty {
			location: legacy_position_from_ibi_1_8(p.location, cx)?,
			r#type: p.r#type << 4 | (p.metadata & 15) as i32,
		},
	)])
}

pub fn world_event_1_7(
	p: v1_8::play_toClient_packet_world_event::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	let data = match p.effectId {
		2001 => {
			let s = item::state_1_7(((p.data & 0xfff) << 4 | (p.data >> 12 & 15)) as u16) as i32;
			s >> 4 | (s & 15) << 12
		}
		_ => p.data,
	};
	Ok(vec![Client7::world_event(
		v1_7::play_toClient_packet_world_event::Ty {
			effectId: p.effectId,
			location: position_ibi_1_7(p.location, cx)?,
			data,
			global: p.global,
		},
	)])
}

pub fn legacy_world_event_1_8(
	p: v1_7::play_toClient_packet_world_event::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	Ok(vec![Client8::world_event(
		v1_8::play_toClient_packet_world_event::Ty {
			effectId: p.effectId,
			location: legacy_position_from_ibi_1_8(p.location, cx)?,
			data: p.data,
			global: p.global,
		},
	)])
}

const PARTICLES: [&str; 42] = [
	"explode",
	"largeexplode",
	"hugeexplosion",
	"fireworksSpark",
	"bubble",
	"splash",
	"wake",
	"suspended",
	"depthsuspend",
	"crit",
	"magicCrit",
	"smoke",
	"largesmoke",
	"spell",
	"instantSpell",
	"mobSpell",
	"mobSpellAmbient",
	"witchMagic",
	"dripWater",
	"dripLava",
	"angryVillager",
	"happyVillager",
	"townaura",
	"note",
	"portal",
	"enchantmenttable",
	"flame",
	"lava",
	"footstep",
	"cloud",
	"reddust",
	"snowballpoof",
	"snowshovel",
	"slime",
	"heart",
	"barrier",
	"iconcrack",
	"blockcrack",
	"blockdust",
	"droplet",
	"take",
	"mobappearance",
];

pub fn world_particles_1_7(
	p: v1_8::play_toClient_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client7>> {
	use v1_8::play_toClient_packet_world_particles__data::Ty as D;
	let block = |kind: &str, d: &[i32]| {
		let state = d.first().copied().unwrap_or(0);
		let s = item::state_1_7(((state & 0xfff) << 4 | (state >> 12 & 15)) as u16);
		format!("{kind}_{}_{}", s >> 4, s & 15)
	};
	let particleName = match &p.data {
		D::v36(d) => {
			let (id, damage) = (
				d.first().copied().unwrap_or(1) as i16,
				d.get(1).copied().unwrap_or(0) as i16,
			);
			let (id, damage) = item::item_1_7(id, damage).unwrap_or((id, damage));
			format!("iconcrack_{id}_{damage}")
		}
		D::v37(d) => block("blockcrack", d),
		D::v38(d) => block("blockdust", d),
		D::default(id @ 0..=34) => PARTICLES[*id as usize].to_owned(),
		D::default(_) => return Ok(vec![]),
	};
	Ok(vec![Client7::world_particles(
		v1_7::play_toClient_packet_world_particles::Ty {
			particleName,
			x: p.x,
			y: p.y,
			z: p.z,
			offsetX: p.offsetX,
			offsetY: p.offsetY,
			offsetZ: p.offsetZ,
			particleData: p.particleData,
			particles: p.particles,
		},
	)])
}

pub fn legacy_world_particles_1_8(
	p: v1_7::play_toClient_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client8>> {
	use v1_8::play_toClient_packet_world_particles__data::Ty as D;
	let mut parts = p.particleName.splitn(3, '_');
	let base = parts.next().unwrap_or_default();
	let mut num = || {
		parts
			.next()
			.and_then(|n| n.parse::<i32>().ok())
			.unwrap_or(0)
	};
	let data = match base {
		"iconcrack" => D::v36(vec![num(), num()]),
		"blockcrack" => D::v37(vec![num() | num() << 12]),
		"blockdust" => D::v38(vec![num() | num() << 12]),
		name => match PARTICLES.iter().position(|n| *n == name) {
			Some(id) => D::default(id as i32),
			None => return Ok(vec![]),
		},
	};
	Ok(vec![Client8::world_particles(
		v1_8::play_toClient_packet_world_particles::Ty {
			longDistance: false,
			x: p.x,
			y: p.y,
			z: p.z,
			offsetX: p.offsetX,
			offsetY: p.offsetY,
			offsetZ: p.offsetZ,
			particleData: p.particleData,
			particles: p.particles,
			data,
		},
	)])
}

pub fn map_1_7(p: v1_8::play_toClient_packet_map::Ty, _cx: &mut Session) -> Result<Vec<Client7>> {
	use v1_8::play_toClient_packet_map__columns::Ty as C;
	let packet = |data: Vec<u8>| {
		Client7::map(v1_7::play_toClient_packet_map::Ty {
			itemDamage: p.itemDamage,
			data,
		})
	};
	let mut out = Vec::new();
	if let C::default(columns, c) = &p.columns {
		let (n, rows) = (*columns as u8 as usize, c.rows as u8 as usize);
		for col in 0..n {
			let mut data = vec![0, (c.x as u8).wrapping_add(col as u8), c.y as u8];
			data.extend((0..rows).map(|r| c.data.get(col + r * n).copied().unwrap_or(0)));
			out.push(packet(data));
		}
	}
	let mut icons = vec![1u8];
	for i in &p.icons {
		icons.extend([i.directionAndType as u8, i.x as u8, i.z as u8]);
	}
	out.push(packet(icons));
	out.push(packet(vec![2, p.scale as u8]));
	Ok(out)
}

pub fn legacy_map_1_8(
	p: v1_7::play_toClient_packet_map::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	use v1_8::play_toClient_packet_map__columns::Ty as C;
	let st = state(cx);
	let (scale, icons) = st.maps.entry(p.itemDamage).or_default();
	let mut columns = C::v0;
	match p.data.as_slice() {
		[0, x, y, rows @ ..] => {
			columns = C::default(
				1,
				v1_8::play_toClient_packet_map__columns__default::Ty {
					rows: rows.len() as i8,
					x: *x as i8,
					y: *y as i8,
					data: rows.to_vec(),
				},
			)
		}
		[1, rest @ ..] => {
			*icons = rest
				.chunks_exact(3)
				.map(|c| Icon8 {
					directionAndType: c[0] as i8,
					x: c[1] as i8,
					z: c[2] as i8,
				})
				.collect()
		}
		[2, s, ..] => *scale = *s as i8,
		_ => return Ok(vec![]),
	}
	Ok(vec![Client8::map(v1_8::play_toClient_packet_map::Ty {
		itemDamage: p.itemDamage,
		scale: *scale,
		icons: icons.clone(),
		columns,
	})])
}

fn listed(name: &str, display: Option<&str>) -> String {
	text::fit(display.map_or_else(|| name.to_owned(), text::to_legacy), 16)
}

fn tab_7(name: &str, online: bool, ping: i32) -> Client7 {
	Client7::player_info(v1_7::play_toClient_packet_player_info::Ty {
		playerName: name.to_owned(),
		online,
		ping: ping.clamp(0, i16::MAX as i32) as i16,
	})
}

pub fn player_info_1_7(
	p: v1_8::play_toClient_packet_player_info::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	use v1_8::play_toClient_packet_player_info__data__items__ANON::Ty as A;
	let st = state(cx);
	let mut out = Vec::new();
	for item in p.data {
		match item.ANON {
			A::add_player(a) => {
				let shown = listed(&a.name, a.displayName.as_deref());
				let old = st.profiles.get(&item.uuid).and_then(|pr| pr.listed.clone());
				if let Some(old) = old.filter(|o| *o != shown) {
					out.push(tab_7(&old, false, 0));
				}
				out.push(tab_7(&shown, true, a.ping));
				st.profiles.insert(
					item.uuid,
					Profile {
						name: a.name,
						properties: a.properties,
						listed: Some(shown),
						ping: a.ping,
					},
				);
			}
			A::update_latency(ping) => {
				if let Some(pr) = st.profiles.get_mut(&item.uuid) {
					pr.ping = ping;
					if let Some(shown) = &pr.listed {
						out.push(tab_7(shown, true, ping));
					}
				}
			}
			A::update_display_name(display) => {
				if let Some(pr) = st.profiles.get_mut(&item.uuid) {
					let shown = listed(&pr.name, display.as_deref());
					if let Some(old) = pr.listed.as_ref().filter(|o| **o != shown) {
						out.push(tab_7(old, false, 0));
						out.push(tab_7(&shown, true, pr.ping));
						pr.listed = Some(shown);
					}
				}
			}
			A::remove_player => {
				if let Some(old) = st
					.profiles
					.get_mut(&item.uuid)
					.and_then(|pr| pr.listed.take())
				{
					out.push(tab_7(&old, false, 0));
				}
			}
			A::update_game_mode(_) | A::default(_) => {}
		}
	}
	Ok(out)
}

fn tab_8(
	action: v1_8::play_toClient_packet_player_info__action::Ty,
	uuid: u128,
	entry: v1_8::play_toClient_packet_player_info__data__items__ANON::Ty,
) -> Client8 {
	Client8::player_info(v1_8::play_toClient_packet_player_info::Ty {
		action,
		data: vec![v1_8::play_toClient_packet_player_info__data__items::Ty { uuid, ANON: entry }],
	})
}

fn add_8(uuid: u128, name: String, properties: Vec<Prop8>, ping: i32) -> Client8 {
	use v1_8::play_toClient_packet_player_info__action::Ty as Action;
	use v1_8::play_toClient_packet_player_info__data__items__ANON::Ty as A;
	tab_8(
		Action::add_player,
		uuid,
		A::add_player(
			v1_8::play_toClient_packet_player_info__data__items__ANON__add_player::Ty {
				name,
				properties,
				gamemode: 0,
				ping,
				displayName: None,
			},
		),
	)
}

pub fn legacy_player_info_1_8(
	p: v1_7::play_toClient_packet_player_info::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	use v1_8::play_toClient_packet_player_info__action::Ty as Action;
	use v1_8::play_toClient_packet_player_info__data__items__ANON::Ty as A;
	let st = state(cx);
	let ping = p.ping as i32;
	Ok(match (st.tab.get(&p.playerName).copied(), p.online) {
		(None, true) => {
			let uuid = name_uuid(&p.playerName);
			st.tab.insert(p.playerName.clone(), uuid);
			let name = text::truncate(&text::strip(&p.playerName), 16);
			vec![add_8(uuid, name, vec![], ping)]
		}
		(Some(uuid), true) => vec![tab_8(Action::update_latency, uuid, A::update_latency(ping))],
		(Some(uuid), false) => {
			st.tab.remove(&p.playerName);
			vec![tab_8(Action::remove_player, uuid, A::remove_player)]
		}
		(None, false) => vec![],
	})
}

pub fn named_entity_spawn_1_7(
	p: v1_8::play_toClient_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	cx.spawned(p.entityId, Kind::Player);
	let (playerName, data) = match state(cx).profiles.get(&p.playerUUID) {
		Some(pr) => (
			text::truncate(&pr.name, 16),
			pr.properties
				.iter()
				.map(
					|q| v1_7::play_toClient_packet_named_entity_spawn__data__items::Ty {
						name: q.name.clone(),
						value: q.value.clone(),
						signature: q.signature.clone().unwrap_or_default(),
					},
				)
				.collect(),
		),
		None => (String::new(), vec![]),
	};
	Ok(vec![Client7::named_entity_spawn(
		v1_7::play_toClient_packet_named_entity_spawn::Ty {
			entityId: p.entityId,
			playerUUID: uuid_string(p.playerUUID),
			playerName,
			data,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			currentItem: item::item_1_7(p.currentItem, 0).map_or(p.currentItem, |(id, _)| id),
			metadata: meta::down(Kind::Player, p.metadata)?,
		},
	)])
}

pub fn legacy_named_entity_spawn_1_8(
	p: v1_7::play_toClient_packet_named_entity_spawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	use v1_8::play_toClient_packet_player_info__action::Ty as Action;
	use v1_8::play_toClient_packet_player_info__data__items__ANON::Ty as A;
	cx.spawned(p.entityId, Kind::Player);
	let uuid = parse_uuid(&p.playerUUID).unwrap_or_else(|| name_uuid(&p.playerName));
	let mut out = Vec::new();
	let st = state(cx);
	if let Some(fake) = st.tab.get(&p.playerName).copied().filter(|u| *u != uuid) {
		out.push(tab_8(Action::remove_player, fake, A::remove_player));
		st.tab.insert(p.playerName.clone(), uuid);
	}
	let properties = p
		.data
		.into_iter()
		.map(|q| Prop8 {
			name: q.name,
			value: q.value,
			signature: (!q.signature.is_empty()).then_some(q.signature),
		})
		.collect();
	out.push(add_8(uuid, p.playerName, properties, 0));
	out.push(Client8::named_entity_spawn(
		v1_8::play_toClient_packet_named_entity_spawn::Ty {
			entityId: p.entityId,
			playerUUID: uuid,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			currentItem: p.currentItem,
			metadata: meta::up(Kind::Player, p.metadata)?,
		},
	));
	Ok(out)
}

fn frame_offset(facing: i32) -> Option<(i32, i32, i8)> {
	Some(match facing {
		0 => (0, 32, 0),
		1 => (-32, 0, 64),
		2 => (0, -32, -128),
		3 => (32, 0, -64),
		_ => return None,
	})
}

fn y_offset(ty: u8) -> i32 {
	match ty {
		50 | 70 => 16,
		51 => 32,
		10 => 11,
		1 => 10,
		2 => 4,
		_ => 0,
	}
}

pub fn spawn_entity_1_7(
	p: v1_8::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	use v1_7::play_toClient_packet_spawn_entity__objectData::Ty as V7;
	use v1_8::play_toClient_packet_spawn_entity__velocity::Ty as V8;
	let ty = p.r#type as u8;
	if ty == 78 {
		state(cx).hidden.insert(p.entityId);
		return Ok(vec![]);
	}
	cx.spawned(p.entityId, Kind::Object(ty));
	let (mut data, velocity) = match p.velocity {
		V8::v0 => (0, None),
		V8::default(d, v) => (d, Some(v)),
	};
	let (mut x, mut y, mut z, mut yaw) = (p.x, p.y, p.z, p.yaw);
	match ty {
		70 => {
			let s = item::state_1_7(((data & 0xfff) << 4 | (data >> 12 & 15)) as u16) as i32;
			data = s >> 4 | (s & 15) << 16;
		}
		71 => {
			if let Some((dx, dz, facing)) = frame_offset(data) {
				(x, z, yaw) = (x - dx, z - dz, facing);
			}
		}
		_ => {}
	}
	let offset = y_offset(ty);
	if offset != 0 {
		y = y.wrapping_add(offset);
		state(cx).offsets.insert(p.entityId, offset);
	}
	let objectData = match data {
		0 => V7::v0,
		d => {
			let v = velocity.unwrap_or(v1_8::vec3i16::Ty { x: 0, y: 0, z: 0 });
			V7::default(
				d,
				v1_7::play_toClient_packet_spawn_entity__objectData__default::Ty {
					velocityX: v.x,
					velocityY: v.y,
					velocityZ: v.z,
				},
			)
		}
	};
	Ok(vec![Client7::spawn_entity(
		v1_7::play_toClient_packet_spawn_entity::Ty {
			entityId: p.entityId,
			r#type: p.r#type,
			x,
			y,
			z,
			pitch: p.pitch,
			yaw,
			objectData,
		},
	)])
}

pub fn legacy_spawn_entity_1_8(
	p: v1_7::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	use v1_7::play_toClient_packet_spawn_entity__objectData::Ty as V7;
	use v1_8::play_toClient_packet_spawn_entity__velocity::Ty as V8;
	let ty = p.r#type as u8;
	cx.spawned(p.entityId, Kind::Object(ty));
	let (mut data, velocity) = match p.objectData {
		V7::v0 => (0, None),
		V7::default(d, v) => (
			d,
			Some(v1_8::vec3i16::Ty {
				x: v.velocityX,
				y: v.velocityY,
				z: v.velocityZ,
			}),
		),
	};
	let (mut x, mut y, mut z, mut yaw) = (p.x, p.y, p.z, p.yaw);
	match ty {
		70 => data = (data & 0xffff) | (data >> 16 & 15) << 12,
		71 => {
			if let Some((dx, dz, facing)) = frame_offset(data) {
				(x, z, yaw) = (x + dx, z + dz, facing);
			}
		}
		_ => {}
	}
	let offset = y_offset(ty);
	if offset != 0 {
		y = y.wrapping_sub(offset);
		state(cx).offsets.insert(p.entityId, offset);
	}
	let velocity = match data {
		0 => V8::v0,
		d => V8::default(
			d,
			velocity.unwrap_or(v1_8::vec3i16::Ty { x: 0, y: 0, z: 0 }),
		),
	};
	Ok(vec![Client8::spawn_entity(
		v1_8::play_toClient_packet_spawn_entity::Ty {
			entityId: p.entityId,
			r#type: p.r#type,
			x,
			y,
			z,
			pitch: p.pitch,
			yaw,
			velocity,
		},
	)])
}

pub fn spawn_entity_living_1_7(
	p: v1_8::play_toClient_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	if p.r#type == 30 {
		state(cx).hidden.insert(p.entityId);
		return Ok(vec![]);
	}
	cx.spawned(p.entityId, Kind::Mob(p.r#type));
	let r#type = match p.r#type {
		67 => 60,
		68 => 94,
		101 => 93,
		t => t,
	};
	Ok(vec![Client7::spawn_entity_living(
		v1_7::play_toClient_packet_spawn_entity_living::Ty {
			entityId: p.entityId,
			r#type,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			headPitch: p.headPitch,
			velocity: p.velocity,
			metadata: meta::down(Kind::Mob(p.r#type), p.metadata)?,
		},
	)])
}

pub fn legacy_spawn_entity_living_1_8(
	p: v1_7::play_toClient_packet_spawn_entity_living::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	cx.spawned(p.entityId, Kind::Mob(p.r#type));
	Ok(vec![Client8::spawn_entity_living(
		v1_8::play_toClient_packet_spawn_entity_living::Ty {
			entityId: p.entityId,
			r#type: p.r#type,
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			headPitch: p.headPitch,
			velocity: p.velocity,
			metadata: meta::up(Kind::Mob(p.r#type), p.metadata)?,
		},
	)])
}

fn painting_offset(direction: i32) -> (i32, i32) {
	match direction {
		0 => (0, -1),
		1 => (1, 0),
		2 => (0, 1),
		3 => (-1, 0),
		_ => (0, 0),
	}
}

pub fn spawn_entity_painting_1_7(
	p: v1_8::play_toClient_packet_spawn_entity_painting::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	cx.spawned(p.entityId, Kind::Object(83));
	let (dx, dz) = painting_offset(p.direction as i32);
	Ok(vec![Client7::spawn_entity_painting(
		v1_7::play_toClient_packet_spawn_entity_painting::Ty {
			entityId: p.entityId,
			title: p.title,
			location: v1_7::position_iii::Ty {
				x: p.location.x + dx,
				y: p.location.y as i32,
				z: p.location.z + dz,
			},
			direction: p.direction as i32,
		},
	)])
}

pub fn legacy_spawn_entity_painting_1_8(
	p: v1_7::play_toClient_packet_spawn_entity_painting::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	cx.spawned(p.entityId, Kind::Object(83));
	let (dx, dz) = painting_offset(p.direction);
	Ok(vec![Client8::spawn_entity_painting(
		v1_8::play_toClient_packet_spawn_entity_painting::Ty {
			entityId: p.entityId,
			title: p.title,
			location: Pos8 {
				x: p.location.x - dx,
				y: p.location.y as i16,
				z: p.location.z - dz,
			},
			direction: p.direction as u8,
		},
	)])
}

pub fn spawn_entity_experience_orb_1_7(
	mut p: v1_8::play_toClient_packet_spawn_entity_experience_orb::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	p.y = p.y.wrapping_add(8);
	state(cx).offsets.insert(p.entityId, 8);
	Ok(vec![Client7::spawn_entity_experience_orb(p)])
}

pub fn legacy_spawn_entity_experience_orb_1_8(
	mut p: v1_7::play_toClient_packet_spawn_entity_experience_orb::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	p.y = p.y.wrapping_sub(8);
	state(cx).offsets.insert(p.entityId, 8);
	Ok(vec![Client8::spawn_entity_experience_orb(p)])
}

pub fn entity_teleport_1_7(
	p: v1_8::play_toClient_packet_entity_teleport::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	let offset = state(cx).offsets.get(&p.entityId).copied().unwrap_or(0);
	Ok(vec![Client7::entity_teleport(
		v1_7::play_toClient_packet_entity_teleport::Ty {
			entityId: p.entityId,
			x: p.x,
			y: p.y.wrapping_add(offset),
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
		},
	)])
}

pub fn legacy_entity_teleport_1_8(
	p: v1_7::play_toClient_packet_entity_teleport::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	let offset = state(cx).offsets.get(&p.entityId).copied().unwrap_or(0);
	Ok(vec![Client8::entity_teleport(
		v1_8::play_toClient_packet_entity_teleport::Ty {
			entityId: p.entityId,
			x: p.x,
			y: p.y.wrapping_sub(offset),
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			onGround: true,
		},
	)])
}

pub fn entity_metadata_1_7(
	p: v1_8::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	if state(cx).hidden.contains(&p.entityId) {
		return Ok(vec![]);
	}
	let metadata = meta::down(cx.kind(p.entityId), p.metadata)?;
	if metadata.is_empty() {
		return Ok(vec![]);
	}
	Ok(vec![Client7::entity_metadata(
		v1_7::play_toClient_packet_entity_metadata::Ty {
			entityId: p.entityId,
			metadata,
		},
	)])
}

pub fn legacy_entity_metadata_1_8(
	p: v1_7::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	let metadata = meta::up(cx.kind(p.entityId), p.metadata)?;
	Ok(vec![Client8::entity_metadata(
		v1_8::play_toClient_packet_entity_metadata::Ty {
			entityId: p.entityId,
			metadata,
		},
	)])
}

pub fn position_1_7(
	p: v1_8::play_toClient_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	let me = &mut state(cx).player;
	let rel = |bit: i8, v: f64, base: f64| if p.flags & bit != 0 { v + base } else { v };
	(me.x, me.y, me.z) = (rel(1, p.x, me.x), rel(2, p.y, me.y), rel(4, p.z, me.z));
	me.yaw = if p.flags & 8 != 0 {
		p.yaw + me.yaw
	} else {
		p.yaw
	};
	me.pitch = if p.flags & 16 != 0 {
		p.pitch + me.pitch
	} else {
		p.pitch
	};
	Ok(vec![Client7::position(
		v1_7::play_toClient_packet_position::Ty {
			x: me.x,
			y: me.y + EYES,
			z: me.z,
			yaw: me.yaw,
			pitch: me.pitch,
			onGround: me.on_ground,
		},
	)])
}

pub fn legacy_position_1_8(
	p: v1_7::play_toClient_packet_position::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client8>> {
	Ok(vec![Client8::position(
		v1_8::play_toClient_packet_position::Ty {
			x: p.x,
			y: p.y - EYES,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			flags: 0,
		},
	)])
}

fn moved(cx: &mut Session, x: f64, y: f64, z: f64, on_ground: bool) {
	let me = &mut state(cx).player;
	if y != -999.0 {
		(me.x, me.y, me.z) = (x, y, z);
	}
	me.on_ground = on_ground;
}

fn turned(cx: &mut Session, yaw: f32, pitch: f32) {
	let me = &mut state(cx).player;
	(me.yaw, me.pitch) = (yaw, pitch);
}

pub fn legacy_flying_1_8(on_ground: bool, cx: &mut Session) -> Result<Vec<Server8>> {
	state(cx).player.on_ground = on_ground;
	Ok(vec![Server8::flying(on_ground)])
}

pub fn legacy_look_1_8(
	p: v1_7::play_toServer_packet_look::Ty,
	cx: &mut Session,
) -> Result<Vec<Server8>> {
	turned(cx, p.yaw, p.pitch);
	state(cx).player.on_ground = p.onGround;
	Ok(vec![Server8::look(p)])
}

pub fn legacy_position_server_1_8(
	p: v1_7::play_toServer_packet_position::Ty,
	cx: &mut Session,
) -> Result<Vec<Server8>> {
	moved(cx, p.x, p.y, p.z, p.onGround);
	Ok(vec![Server8::position(
		v1_8::play_toServer_packet_position::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
			onGround: p.onGround,
		},
	)])
}

pub fn legacy_position_look_1_8(
	p: v1_7::play_toServer_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<Vec<Server8>> {
	moved(cx, p.x, p.y, p.z, p.onGround);
	turned(cx, p.yaw, p.pitch);
	Ok(vec![Server8::position_look(
		v1_8::play_toServer_packet_position_look::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
			yaw: p.yaw,
			pitch: p.pitch,
			onGround: p.onGround,
		},
	)])
}

const WINDOWS: [&str; 11] = [
	"minecraft:chest",
	"minecraft:crafting_table",
	"minecraft:furnace",
	"minecraft:dispenser",
	"minecraft:enchanting_table",
	"minecraft:brewing_stand",
	"minecraft:villager",
	"minecraft:beacon",
	"minecraft:anvil",
	"minecraft:hopper",
	"minecraft:dropper",
];

const ENCHANTING: u8 = 4;
const FURNACE: u8 = 2;

pub fn open_window_1_7(
	p: v1_8::play_toClient_packet_open_window::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	use v1_7::play_toClient_packet_open_window__entityId::Ty as E7;
	use v1_8::play_toClient_packet_open_window__entityId::Ty as E8;
	let ty = match p.inventoryType.as_str() {
		"EntityHorse" => 11,
		name => WINDOWS.iter().position(|w| *w == name).unwrap_or(0) as u8,
	};
	state(cx).windows.insert(p.windowId, ty);
	let (windowTitle, useProvidedTitle) = text::title_1_7(&p.windowTitle);
	let entityId = match p.entityId {
		E8::EntityHorse(id) => E7::v11(id),
		E8::default(_) => E7::default(ty),
	};
	Ok(vec![Client7::open_window(
		v1_7::play_toClient_packet_open_window::Ty {
			windowId: p.windowId,
			windowTitle,
			slotCount: p.slotCount,
			useProvidedTitle,
			entityId,
		},
	)])
}

pub fn legacy_open_window_1_8(
	p: v1_7::play_toClient_packet_open_window::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	use v1_7::play_toClient_packet_open_window__entityId::Ty as E7;
	use v1_8::play_toClient_packet_open_window__entityId::Ty as E8;
	let ty = v1_7::play_toClient_packet_open_window__entityId::virtual_tag(&p.entityId);
	state(cx).windows.insert(p.windowId, ty);
	let inventoryType = match ty {
		11 => "EntityHorse".to_owned(),
		t => WINDOWS.get(t as usize).unwrap_or(&WINDOWS[0]).to_string(),
	};
	let windowTitle = if p.useProvidedTitle {
		text::from_legacy(&p.windowTitle)
	} else {
		format!("{{\"translate\":{}}}", text::quote(&p.windowTitle))
	};
	let entityId = match p.entityId {
		E7::v11(id) => E8::EntityHorse(id),
		E7::default(_) => E8::default(inventoryType.clone()),
	};
	Ok(vec![Client8::open_window(
		v1_8::play_toClient_packet_open_window::Ty {
			windowId: p.windowId,
			inventoryType,
			windowTitle,
			slotCount: p.slotCount,
			entityId,
		},
	)])
}

fn closed(cx: &mut Session, window: u8) {
	let st = state(cx);
	st.windows.remove(&window);
	st.cook.remove(&window);
}

pub fn close_window_1_7(window: u8, cx: &mut Session) -> Result<Vec<Client7>> {
	closed(cx, window);
	Ok(vec![Client7::close_window(window)])
}

pub fn legacy_close_window_1_8(window: u8, cx: &mut Session) -> Result<Vec<Client8>> {
	closed(cx, window);
	Ok(vec![Client8::close_window(window)])
}

pub fn legacy_close_window_server_1_8(window: u8, cx: &mut Session) -> Result<Vec<Server8>> {
	closed(cx, window);
	Ok(vec![Server8::close_window(window)])
}

fn window(cx: &mut Session, id: i8) -> Option<u8> {
	state(cx).windows.get(&(id as u8)).copied()
}

pub fn set_slot_1_7(
	p: v1_8::play_toClient_packet_set_slot::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	let mut slot = p.slot;
	if window(cx, p.windowId) == Some(ENCHANTING) {
		match slot {
			1 => return Ok(vec![]),
			s if s > 1 => slot -= 1,
			_ => {}
		}
	}
	Ok(vec![Client7::set_slot(
		v1_7::play_toClient_packet_set_slot::Ty {
			windowId: p.windowId,
			slot,
			item: item::down(p.item)?,
		},
	)])
}

pub fn legacy_set_slot_1_8(
	p: v1_7::play_toClient_packet_set_slot::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	let mut slot = p.slot;
	if window(cx, p.windowId) == Some(ENCHANTING) && slot >= 1 {
		slot = slot.saturating_add(1);
	}
	Ok(vec![Client8::set_slot(
		v1_8::play_toClient_packet_set_slot::Ty {
			windowId: p.windowId,
			slot,
			item: item::up(p.item)?,
		},
	)])
}

pub fn window_items_1_7(
	p: v1_8::play_toClient_packet_window_items::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	let mut items = p.items;
	if window(cx, p.windowId as i8) == Some(ENCHANTING) && items.len() > 1 {
		items.remove(1);
	}
	Ok(vec![Client7::window_items(
		v1_7::play_toClient_packet_window_items::Ty {
			windowId: p.windowId,
			items: items.into_iter().map(item::down).collect::<Result<_>>()?,
		},
	)])
}

pub fn legacy_window_items_1_8(
	p: v1_7::play_toClient_packet_window_items::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	let mut items = p
		.items
		.into_iter()
		.map(item::up)
		.collect::<Result<Vec<_>>>()?;
	if window(cx, p.windowId as i8) == Some(ENCHANTING) && !items.is_empty() {
		let lapis = v1_8::slot__default::Ty {
			itemCount: 3,
			itemDamage: 4,
			nbtData: None,
		};
		items.insert(1, v1_8::slot::Ty::default(351, lapis));
	}
	Ok(vec![Client8::window_items(
		v1_8::play_toClient_packet_window_items::Ty {
			windowId: p.windowId,
			items,
		},
	)])
}

type Bar = v1_8::play_toClient_packet_craft_progress_bar::Ty;

pub fn craft_progress_bar_1_7(mut p: Bar, cx: &mut Session) -> Result<Vec<Client7>> {
	let st = state(cx);
	match st.windows.get(&p.windowId).copied() {
		Some(FURNACE) => match p.property {
			0 => p.property = 1,
			1 => p.property = 2,
			2 => {
				let max = st.cook.get(&p.windowId).copied().unwrap_or(200).max(1) as i32;
				p.property = 0;
				p.value = (p.value as i32 * 200 / max) as i16;
			}
			3 => {
				st.cook.insert(p.windowId, p.value);
				return Ok(vec![]);
			}
			_ => return Ok(vec![]),
		},
		Some(ENCHANTING) if p.property > 2 => return Ok(vec![]),
		_ => {}
	}
	Ok(vec![Client7::craft_progress_bar(p)])
}

pub fn legacy_craft_progress_bar_1_8(mut p: Bar, cx: &mut Session) -> Result<Vec<Client8>> {
	let mut out = Vec::new();
	if window(cx, p.windowId as i8) == Some(FURNACE) {
		p.property = match p.property {
			0 => {
				out.push(Client8::craft_progress_bar(Bar {
					windowId: p.windowId,
					property: 3,
					value: 200,
				}));
				2
			}
			1 => 0,
			2 => 1,
			n => n,
		};
	}
	out.push(Client8::craft_progress_bar(p));
	Ok(out)
}

fn sign_line_1_7(json: &str) -> String {
	text::fit(text::to_legacy(json), 15)
}

pub fn update_sign_1_7(
	p: v1_8::play_toClient_packet_update_sign::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	Ok(vec![Client7::update_sign(
		v1_7::play_toClient_packet_update_sign::Ty {
			location: position_isi_1_7(p.location, cx)?,
			text1: sign_line_1_7(&p.text1),
			text2: sign_line_1_7(&p.text2),
			text3: sign_line_1_7(&p.text3),
			text4: sign_line_1_7(&p.text4),
		},
	)])
}

pub fn legacy_update_sign_1_8(
	p: v1_7::play_toClient_packet_update_sign::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	Ok(vec![Client8::update_sign(
		v1_8::play_toClient_packet_update_sign::Ty {
			location: legacy_position_from_isi_1_8(p.location, cx)?,
			text1: text::from_legacy(&p.text1),
			text2: text::from_legacy(&p.text2),
			text3: text::from_legacy(&p.text3),
			text4: text::from_legacy(&p.text4),
		},
	)])
}

pub fn legacy_update_sign_server_1_8(
	p: v1_7::play_toServer_packet_update_sign::Ty,
	cx: &mut Session,
) -> Result<Vec<Server8>> {
	Ok(vec![Server8::update_sign(
		v1_8::play_toServer_packet_update_sign::Ty {
			location: legacy_position_from_isi_1_8(p.location, cx)?,
			text1: text::from_legacy(&p.text1),
			text2: text::from_legacy(&p.text2),
			text3: text::from_legacy(&p.text3),
			text4: text::from_legacy(&p.text4),
		},
	)])
}

pub fn update_sign_server_1_7(
	p: v1_8::play_toServer_packet_update_sign::Ty,
	cx: &mut Session,
) -> Result<Vec<Server7>> {
	Ok(vec![Server7::update_sign(
		v1_7::play_toServer_packet_update_sign::Ty {
			location: position_isi_1_7(p.location, cx)?,
			text1: sign_line_1_7(&p.text1),
			text2: sign_line_1_7(&p.text2),
			text3: sign_line_1_7(&p.text3),
			text4: sign_line_1_7(&p.text4),
		},
	)])
}

pub fn tile_entity_data_1_7(
	p: v1_8::play_toClient_packet_tile_entity_data::Ty,
	cx: &mut Session,
) -> Result<Vec<Client7>> {
	if p.action == 6 {
		return Ok(vec![]);
	}
	let mut tag = p.nbtData;
	if p.action == 5 {
		item::flower_pot_1_7(&mut tag);
	}
	Ok(vec![Client7::tile_entity_data(
		v1_7::play_toClient_packet_tile_entity_data::Ty {
			location: position_isi_1_7(p.location, cx)?,
			action: p.action,
			nbtData: item::compress(&tag)?,
		},
	)])
}

pub fn legacy_tile_entity_data_1_8(
	p: v1_7::play_toClient_packet_tile_entity_data::Ty,
	cx: &mut Session,
) -> Result<Vec<Client8>> {
	let mut tag = item::decompress(&p.nbtData)?;
	if p.action == 5 {
		item::flower_pot_1_8(&mut tag);
	}
	Ok(vec![Client8::tile_entity_data(
		v1_8::play_toClient_packet_tile_entity_data::Ty {
			location: legacy_position_from_isi_1_8(p.location, cx)?,
			action: p.action,
			nbtData: tag,
		},
	)])
}

pub fn scoreboard_objective_1_7(
	p: v1_8::play_toClient_packet_scoreboard_objective::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client7>> {
	use v1_8::play_toClient_packet_scoreboard_objective__action::Ty as A;
	let (displayText, action) = match p.action {
		A::v0(a) => (a.displayText, 0),
		A::v2(a) => (a.displayText, 2),
		A::default(t) => (String::new(), t),
	};
	Ok(vec![Client7::scoreboard_objective(
		v1_7::play_toClient_packet_scoreboard_objective::Ty {
			name: text::truncate(&p.name, 16),
			displayText: text::truncate(&displayText, 32),
			action,
		},
	)])
}

pub fn legacy_scoreboard_objective_1_8(
	p: v1_7::play_toClient_packet_scoreboard_objective::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client8>> {
	use v1_8::play_toClient_packet_scoreboard_objective__action::Ty as A;
	let action = match p.action {
		0 => A::v0(
			v1_8::play_toClient_packet_scoreboard_objective__action__v0::Ty {
				displayText: p.displayText,
				r#type: "integer".to_owned(),
			},
		),
		2 => A::v2(
			v1_8::play_toClient_packet_scoreboard_objective__action__v2::Ty {
				displayText: p.displayText,
				r#type: "integer".to_owned(),
			},
		),
		t => A::default(t),
	};
	Ok(vec![Client8::scoreboard_objective(
		v1_8::play_toClient_packet_scoreboard_objective::Ty {
			name: p.name,
			action,
		},
	)])
}

pub fn scoreboard_score_1_7(
	p: v1_8::play_toClient_packet_scoreboard_score::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client7>> {
	use v1_7::play_toClient_packet_scoreboard_score__action::Ty as A7;
	use v1_8::play_toClient_packet_scoreboard_score__value::Ty as V8;
	let action = match p.value {
		V8::v1 => A7::v1,
		V8::default(t, value) => A7::default(
			t as i8,
			v1_7::play_toClient_packet_scoreboard_score__action__default::Ty {
				scoreName: text::truncate(&p.scoreName, 16),
				value,
			},
		),
	};
	Ok(vec![Client7::scoreboard_score(
		v1_7::play_toClient_packet_scoreboard_score::Ty {
			itemName: text::fit(p.itemName, 16),
			action,
		},
	)])
}

pub fn legacy_scoreboard_score_1_8(
	p: v1_7::play_toClient_packet_scoreboard_score::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client8>> {
	use v1_7::play_toClient_packet_scoreboard_score__action::Ty as A7;
	use v1_8::play_toClient_packet_scoreboard_score__value::Ty as V8;
	let (scoreName, value) = match p.action {
		A7::v1 => (String::new(), V8::v1),
		A7::default(t, d) => (d.scoreName, V8::default(t as i32, d.value)),
	};
	Ok(vec![Client8::scoreboard_score(
		v1_8::play_toClient_packet_scoreboard_score::Ty {
			itemName: p.itemName,
			scoreName,
			value,
		},
	)])
}

fn trades(data: &[u8], down: bool) -> Option<Vec<u8>> {
	let input = Input::new(data);
	let mut out = Output::new();
	out.write(&take(&input, 4)?);
	let count = input.read().ok()?;
	out.write(&[count]);
	let slot = |out: &mut Output| -> Option<()> {
		let mut probe = Input::new(data.get(input.offset()..)?);
		if down {
			let s = item::down(v1_8::slot::deserialize(&mut probe).ok()?).ok()?;
			v1_7::slot::serialize(&s, out).ok()?;
		} else {
			let s = item::up(v1_7::slot::deserialize(&mut probe).ok()?).ok()?;
			v1_8::slot::serialize(&s, out).ok()?;
		}
		take(&input, probe.offset())?;
		Some(())
	};
	for _ in 0..count {
		slot(&mut out)?;
		slot(&mut out)?;
		let third = input.read().ok()?;
		out.write(&[third]);
		if third != 0 {
			slot(&mut out)?;
		}
		out.write(&[input.read().ok()?]);
		if down {
			take(&input, 8)?;
		} else {
			out.write(&[0, 0, 0, 0, 0x7f, 0xff, 0xff, 0xff]);
		}
	}
	Some(out.into_inner())
}

pub fn custom_payload_1_7(
	p: v1_8::play_toClient_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client7>> {
	let data = match p.channel.as_str() {
		"MC|Brand" => read_string(&Input::new(&p.data)).map(String::into_bytes),
		"MC|TrList" => trades(&p.data, true),
		_ => None,
	}
	.unwrap_or(p.data);
	if data.len() > i16::MAX as usize {
		return Ok(vec![]);
	}
	Ok(vec![Client7::custom_payload(
		v1_7::play_toClient_packet_custom_payload::Ty {
			channel: p.channel,
			data,
		},
	)])
}

pub fn legacy_custom_payload_1_8(
	p: v1_7::play_toClient_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client8>> {
	let data = match p.channel.as_str() {
		"MC|Brand" => Some(string_bytes(&String::from_utf8_lossy(&p.data))),
		"MC|TrList" => trades(&p.data, false),
		"MC|RPack" => {
			return Ok(vec![Client8::resource_pack_send(
				v1_8::play_toClient_packet_resource_pack_send::Ty {
					url: String::from_utf8_lossy(&p.data).into_owned(),
					hash: "legacy".to_owned(),
				},
			)]);
		}
		_ => None,
	}
	.unwrap_or(p.data);
	Ok(vec![Client8::custom_payload(
		v1_8::play_toClient_packet_custom_payload::Ty {
			channel: p.channel,
			data,
		},
	)])
}

fn book_1_8(data: &[u8], sign: bool) -> Option<Vec<u8>> {
	let mut slot = item::up(v1_7::slot::deserialize(&mut Input::new(data)).ok()?).ok()?;
	if sign {
		item::book_pages_1_8(&mut slot);
	}
	let mut out = Output::new();
	v1_8::slot::serialize(&slot, &mut out).ok()?;
	Some(out.into_inner())
}

fn book_1_7(data: &[u8]) -> Option<Vec<u8>> {
	let slot = item::down(v1_8::slot::deserialize(&mut Input::new(data)).ok()?).ok()?;
	let mut out = Output::new();
	v1_7::slot::serialize(&slot, &mut out).ok()?;
	Some(out.into_inner())
}

fn command_block_1_8(data: &[u8]) -> Option<Vec<u8>> {
	let input = Input::new(data);
	let kind = input.read().ok()?;
	let mut out = vec![kind];
	out.extend(take(&input, if kind == 0 { 12 } else { 4 })?);
	out.extend(string_bytes(&read_string(&input)?));
	out.push(1);
	Some(out)
}

pub fn legacy_custom_payload_server_1_8(
	p: v1_7::play_toServer_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server8>> {
	let raw = || Some(string_bytes(&String::from_utf8_lossy(&p.data)));
	let data = match p.channel.as_str() {
		"MC|Brand" | "MC|ItemName" => raw(),
		"MC|BEdit" => book_1_8(&p.data, false),
		"MC|BSign" => book_1_8(&p.data, true),
		"MC|TrSel" => p.data.get(..4).map(<[u8]>::to_vec),
		"MC|Beacon" => p.data.get(..8).map(<[u8]>::to_vec),
		"MC|AdvCdm" => command_block_1_8(&p.data),
		_ => None,
	}
	.unwrap_or(p.data);
	Ok(vec![Server8::custom_payload(
		v1_8::play_toServer_packet_custom_payload::Ty {
			channel: p.channel,
			data,
		},
	)])
}

pub fn custom_payload_server_1_7(
	p: v1_8::play_toServer_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server7>> {
	let data = match p.channel.as_str() {
		"MC|Brand" | "MC|ItemName" => read_string(&Input::new(&p.data)).map(String::into_bytes),
		"MC|BEdit" | "MC|BSign" => book_1_7(&p.data),
		"MC|AdvCdm" => p.data.split_last().map(|(_, rest)| rest.to_vec()),
		_ => None,
	}
	.unwrap_or(p.data);
	if data.len() > i16::MAX as usize {
		return Ok(vec![]);
	}
	Ok(vec![Server7::custom_payload(
		v1_7::play_toServer_packet_custom_payload::Ty {
			channel: p.channel,
			data,
		},
	)])
}

pub fn legacy_tab_complete_1_8(
	text: String,
	_cx: &mut Session,
) -> Result<v1_8::play_toServer_packet_tab_complete::Ty> {
	Ok(v1_8::play_toServer_packet_tab_complete::Ty { text, block: None })
}

pub fn tab_complete_1_7(
	p: v1_8::play_toServer_packet_tab_complete::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(p.text)
}

pub fn legacy_arm_animation_1_8(
	p: v1_7::play_toServer_packet_arm_animation::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server8>> {
	Ok(match p.animation {
		1 => vec![Server8::arm_animation(
			v1_8::play_toServer_packet_arm_animation::Ty {},
		)],
		_ => vec![],
	})
}

pub fn arm_animation_1_7(
	_p: v1_8::play_toServer_packet_arm_animation::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server7>> {
	Ok(vec![Server7::arm_animation(
		v1_7::play_toServer_packet_arm_animation::Ty {
			entityId: 0,
			animation: 1,
		},
	)])
}

pub fn legacy_window_click_1_8(
	p: v1_7::play_toServer_packet_window_click::Ty,
	cx: &mut Session,
) -> Result<Vec<Server8>> {
	let mut slot = p.slot;
	if window(cx, p.windowId) == Some(ENCHANTING) && slot > 0 {
		slot = slot.saturating_add(1);
	}
	Ok(vec![Server8::window_click(
		v1_8::play_toServer_packet_window_click::Ty {
			windowId: p.windowId as u8,
			slot,
			mouseButton: p.mouseButton,
			action: p.action,
			mode: p.mode,
			item: item::up(p.item)?,
		},
	)])
}

pub fn window_click_1_7(
	p: v1_8::play_toServer_packet_window_click::Ty,
	cx: &mut Session,
) -> Result<Vec<Server7>> {
	let mut slot = p.slot;
	if window(cx, p.windowId as i8) == Some(ENCHANTING) {
		match slot {
			1 => return Ok(vec![]),
			s if s > 1 => slot -= 1,
			_ => {}
		}
	}
	Ok(vec![Server7::window_click(
		v1_7::play_toServer_packet_window_click::Ty {
			windowId: p.windowId as i8,
			slot,
			mouseButton: p.mouseButton,
			action: p.action,
			mode: p.mode,
			item: item::down(p.item)?,
		},
	)])
}

pub fn legacy_dig_face_1_8(
	src: &v1_7::play_toServer_packet_block_dig::Ty,
	_cx: &mut Session,
) -> Result<i8> {
	Ok(if (0..=5).contains(&src.face) {
		src.face
	} else {
		0
	})
}

pub fn legacy_skin_parts_1_8(
	src: &v1_7::play_toServer_packet_settings::Ty,
	_cx: &mut Session,
) -> Result<u8> {
	Ok(if src.showCape { 0x7f } else { 0x7e })
}

pub fn show_cape_1_7(
	src: &v1_8::play_toServer_packet_settings::Ty,
	_cx: &mut Session,
) -> Result<bool> {
	Ok(src.skinParts & 1 != 0)
}

pub fn legacy_steer_flags_1_8(
	src: &v1_7::play_toServer_packet_steer_vehicle::Ty,
	_cx: &mut Session,
) -> Result<u8> {
	Ok(src.jump as u8 | (src.unmount as u8) << 1)
}

pub fn steer_jump_1_7(
	src: &v1_8::play_toServer_packet_steer_vehicle::Ty,
	_cx: &mut Session,
) -> Result<bool> {
	Ok(src.jump & 1 != 0)
}

pub fn steer_unmount_1_7(
	src: &v1_8::play_toServer_packet_steer_vehicle::Ty,
	_cx: &mut Session,
) -> Result<bool> {
	Ok(src.jump & 2 != 0)
}

pub fn effect_duration_1_7(
	src: &v1_8::play_toClient_packet_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<i16> {
	Ok(clamp16(src.duration))
}

pub fn experience_level_1_7(
	src: &v1_8::play_toClient_packet_experience::Ty,
	_cx: &mut Session,
) -> Result<i16> {
	Ok(clamp16(src.level))
}

pub fn experience_total_1_7(
	src: &v1_8::play_toClient_packet_experience::Ty,
	_cx: &mut Session,
) -> Result<i16> {
	Ok(clamp16(src.totalExperience))
}
