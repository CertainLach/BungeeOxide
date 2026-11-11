use crate::gen::convert::{Root, Version};
use crate::pds::convert::{unconvertible, Error, Result};
use crate::pds::{Input, Output};
use crate::registry::{self, renames};
use crate::Session;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

#[derive(Default)]
pub struct State {
	pub entities: HashMap<i32, String>,
	pub player: Option<i32>,
	pub codec: Option<bungeeoxide_nbt::Tag>,
	pub dimensions: Vec<String>,
	pub secure_chat: Option<bool>,
	pub pending: Vec<bungeeoxide_nbt::Entry>,
	pub tags:
		Option<Vec<crate::gen::convert::v1_20_2::configuration_to_client_packet_tags_items::Ty>>,
	pub features: Option<Vec<String>>,
	pub payloads: Vec<(String, Vec<u8>)>,
	pub resource_pack: Option<(String, String, bool, Option<String>)>,
	pub packs: Vec<u128>,
	pub displays: HashSet<i32>,
	pub back_sign: Option<Vec<u8>>,
}

pub fn state(cx: &mut Session) -> &mut State {
	cx.state::<State>()
}

pub fn reply<T>(
	cx: &mut Session,
	root: Root,
	version: Version,
	packet: &T,
	serialize: fn(&T, &mut Output) -> crate::pds::Result<()>,
) -> Result<()> {
	let data = encode(packet, serialize)?;
	cx.send(root, version, data);
	Ok(())
}

pub fn meta_index(version: Version, kind: &str, key: &str) -> Option<u8> {
	let id = registry::names(version, "entities")?.id(kind)?;
	registry::entity_metadata_keys(version, id)?
		.iter()
		.position(|k| k == key)
		.map(|i| i as u8)
}

pub fn offline_uuid(name: &str) -> u128 {
	let mut b = md5(format!("OfflinePlayer:{name}").as_bytes());
	b[6] = b[6] & 0x0f | 0x30;
	b[8] = b[8] & 0x3f | 0x80;
	u128::from_be_bytes(b)
}

pub fn md5(data: &[u8]) -> [u8; 16] {
	const S: [u32; 16] = [7, 12, 17, 22, 5, 9, 14, 20, 4, 11, 16, 23, 6, 10, 15, 21];
	let k: [u32; 64] =
		std::array::from_fn(|i| ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32);
	let mut msg = data.to_vec();
	msg.push(0x80);
	while msg.len() % 64 != 56 {
		msg.push(0);
	}
	msg.extend_from_slice(&(data.len() as u64).wrapping_mul(8).to_le_bytes());
	let mut h = [0x6745_2301u32, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];
	for chunk in msg.chunks(64) {
		let m: [u32; 16] = std::array::from_fn(|i| {
			u32::from_le_bytes([
				chunk[i * 4],
				chunk[i * 4 + 1],
				chunk[i * 4 + 2],
				chunk[i * 4 + 3],
			])
		});
		let [mut a, mut b, mut c, mut d] = h;
		for i in 0..64 {
			let (f, g) = match i / 16 {
				0 => ((b & c) | (!b & d), i),
				1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
				2 => (b ^ c ^ d, (3 * i + 5) % 16),
				_ => (c ^ (b | !d), (7 * i) % 16),
			};
			let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
			a = d;
			d = c;
			c = b;
			b = b.wrapping_add(f.rotate_left(S[(i / 16) * 4 + i % 4]));
		}
		h = [
			h[0].wrapping_add(a),
			h[1].wrapping_add(b),
			h[2].wrapping_add(c),
			h[3].wrapping_add(d),
		];
	}
	let mut out = [0u8; 16];
	for (i, v) in h.iter().enumerate() {
		out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
	}
	out
}

pub fn later(from: Version, to: Version) -> bool {
	from < to
}

pub fn spawned(cx: &mut Session, version: Version, entity: i32, kind: i32) {
	if let Some(name) = registry::names(version, "entities").and_then(|n| n.name(kind as u32)) {
		let s = state(cx);
		s.entities.insert(entity, name.to_owned());
		s.displays.remove(&entity);
	}
}

pub fn entity_kind(cx: &mut Session, entity: i32) -> Option<String> {
	let s = state(cx);
	match s.entities.get(&entity) {
		Some(k) => Some(k.clone()),
		None if s.player == Some(entity) => Some("player".to_owned()),
		None => None,
	}
}

fn renamed<'a>(kind: &str, to: Version, name: &'a str) -> &'a str {
	renames::table(kind)
		.iter()
		.find(|(v, old, _)| *v == to && *old == name)
		.map_or(name, |(_, _, new)| new)
}

fn unrenamed<'a>(kind: &str, from: Version, name: &'a str) -> &'a str {
	renames::table(kind)
		.iter()
		.find(|(v, _, new)| *v == from && *new == name)
		.map_or(name, |(_, old, _)| old)
}

pub fn translate<'a>(kind: &str, from: Version, to: Version, name: &'a str) -> &'a str {
	if later(from, to) {
		renamed(kind, to, name)
	} else {
		unrenamed(kind, from, name)
	}
}

pub fn name_id(kind: &str, from: Version, to: Version, id: i32) -> Option<i32> {
	if from == to || registry::folder(from, kind) == registry::folder(to, kind) {
		return Some(id);
	}
	let name = registry::names(from, kind)?.name(u32::try_from(id).ok()?)?;
	let name = translate(kind, from, to, name);
	registry::names(to, kind)?.id(name).map(|i| i as i32)
}

pub fn item(from: Version, to: Version, id: i32) -> Option<i32> {
	name_id("items", from, to, id)
}

pub fn state_map(from: Version, to: Version) -> Option<&'static [u32]> {
	if registry::folder(from, "blocks") == registry::folder(to, "blocks") {
		return None;
	}
	if later(from, to) {
		return registry::state_table(from, to);
	}
	static DOWN: OnceLock<Mutex<HashMap<(Version, Version), &'static [u32]>>> = OnceLock::new();
	let mut map = DOWN
		.get_or_init(Default::default)
		.lock()
		.expect("state cache poisoned");
	if let Some(t) = map.get(&(from, to)) {
		return Some(t);
	}
	let (a, b) = (registry::blocks(from)?, registry::blocks(to)?);
	let air = b.block("air").map_or(0, |x| x.default);
	let table: &'static [u32] = Box::leak(
		(0..a.states())
			.map(|s| {
				let Some(block) = a.owner(s) else { return air };
				b.block(unrenamed("blocks", from, &block.name))
					.map_or(air, |target| target.state(&block.values(s)))
			})
			.collect::<Vec<_>>()
			.into_boxed_slice(),
	);
	map.insert((from, to), table);
	Some(table)
}

pub fn mapped(table: Option<&[u32]>, s: i32) -> i32 {
	match table {
		None => s,
		Some(t) => t.get(s as usize).copied().unwrap_or(0) as i32,
	}
}

pub fn block_state(from: Version, to: Version, s: i32) -> i32 {
	mapped(state_map(from, to), s)
}

pub fn block(from: Version, to: Version, id: i32) -> Option<i32> {
	let (a, b) = (registry::blocks(from)?, registry::blocks(to)?);
	let name = &a.list.iter().find(|x| x.id == id as u32)?.name;
	b.block(translate("blocks", from, to, name))
		.map(|x| x.id as i32)
}

pub fn states(version: Version) -> u32 {
	registry::blocks(version).map_or(1, |b| b.states())
}

pub fn particle_name(version: Version, id: i32) -> Option<&'static str> {
	registry::names(version, "particles")?.name(u32::try_from(id).ok()?)
}

pub fn particle_id(version: Version, name: &str) -> Option<i32> {
	registry::names(version, "particles")?
		.id(name)
		.map(|i| i as i32)
}

pub enum Sound {
	Id(i32),
	Named(String),
}

pub fn sound(from: Version, to: Version, id: i32) -> Result<Sound> {
	let Some(name) = registry::names(from, "sounds").and_then(|n| n.name(id as u32)) else {
		return Err(Error::Dropped);
	};
	Ok(sound_named(
		to,
		&format!("minecraft:{}", translate("sounds", from, to, name)),
	))
}

pub fn sound_named(to: Version, name: &str) -> Sound {
	match registry::names(to, "sounds").and_then(|n| n.id(name)) {
		Some(id) => Sound::Id(id as i32),
		None => Sound::Named(name.to_owned()),
	}
}

pub fn varint(out: &mut Vec<u8>, v: i32) {
	super::chunk::varint(out, v as u32)
}

pub fn read_varint(data: &[u8]) -> Result<(i32, &[u8])> {
	let mut r = super::chunk::Reader::new(data);
	let v = r.varint()? as i32;
	let used = {
		let mut probe = Vec::new();
		varint(&mut probe, v);
		probe.len()
	};
	Ok((v, &data[used..]))
}

pub fn mc_string(s: &str) -> Vec<u8> {
	let mut out = Vec::new();
	varint(&mut out, s.len() as i32);
	out.extend_from_slice(s.as_bytes());
	out
}

pub fn read_string(data: &[u8]) -> Result<(String, &[u8])> {
	let (n, rest) = read_varint(data)?;
	let n = usize::try_from(n).map_err(|_| Error::Encoding("negative string length".to_owned()))?;
	let Some(bytes) = rest.get(..n) else {
		return unconvertible("string runs past the data");
	};
	Ok((String::from_utf8_lossy(bytes).into_owned(), &rest[n..]))
}

pub fn encode<T>(v: &T, f: fn(&T, &mut Output) -> crate::pds::Result<()>) -> Result<Vec<u8>> {
	let mut o = Output::new();
	f(v, &mut o).map_err(|e| Error::Encoding(format!("{e:?}")))?;
	Ok(o.into_inner())
}

pub fn decode<T>(data: &[u8], f: impl FnOnce(&mut Input) -> crate::pds::Result<T>) -> Result<T> {
	let mut input = Input::new(data);
	let v = f(&mut input).map_err(|e| Error::Encoding(format!("{e:?}")))?;
	match input.remaining() {
		0 => Ok(v),
		n => unconvertible(format!("{n} bytes left after re-decoding")),
	}
}

pub struct ParticleMap<'a> {
	pub from: Version,
	pub to: Version,
	pub item: &'a dyn Fn(&[u8]) -> Result<Vec<u8>>,
}

pub const BLOCK_PARTICLES: &[&str] = &["block", "block_marker", "falling_dust", "dust_pillar"];

pub fn vibration(from: Version, to: Version, payload: &[u8]) -> Result<Vec<u8>> {
	let keyed = |v: Version| v <= Version::V1_20_2;
	if keyed(from) == keyed(to) {
		return Ok(payload.to_vec());
	}
	let mut out = Vec::new();
	let rest = if keyed(from) {
		let (kind, rest) = read_string(payload)?;
		varint(
			&mut out,
			(kind.trim_start_matches("minecraft:") == "entity") as i32,
		);
		rest
	} else {
		let (kind, rest) = read_varint(payload)?;
		out.extend(mc_string(if kind == 1 {
			"minecraft:entity"
		} else {
			"minecraft:block"
		}));
		rest
	};
	out.extend_from_slice(rest);
	Ok(out)
}

pub fn particle(m: &ParticleMap, id: i32, payload: &[u8]) -> Result<(i32, Vec<u8>)> {
	let Some(name) = particle_name(m.from, id) else {
		return Err(Error::Dropped);
	};
	let target = translate("particles", m.from, m.to, name);
	let Some(new) = particle_id(m.to, target) else {
		return Err(Error::Dropped);
	};
	let payload = if BLOCK_PARTICLES.contains(&name) {
		let (s, rest) = read_varint(payload)?;
		let mut out = Vec::new();
		varint(&mut out, block_state(m.from, m.to, s));
		out.extend_from_slice(rest);
		out
	} else if name == "item" {
		(m.item)(payload)?
	} else if name == "vibration" {
		vibration(m.from, m.to, payload)?
	} else {
		payload.to_vec()
	};
	Ok((new, payload))
}

pub fn meta_key(
	cx: &mut Session,
	from: Version,
	to: Version,
	entity: i32,
	key: u8,
	renames: &[(&str, &str)],
) -> Option<u8> {
	let Some(kind) = entity_kind(cx, entity) else {
		return Some(key);
	};
	let keys = |version: Version| {
		let id = registry::names(version, "entities")?.id(&kind)?;
		registry::entity_metadata_keys(version, id)
	};
	let (Some(a), Some(b)) = (keys(from), keys(to)) else {
		return Some(key);
	};
	let Some(name) = a.get(key as usize) else {
		return Some(key);
	};
	let up = later(from, to);
	let name = renames
		.iter()
		.find(|(old, new)| if up { old == name } else { new == name })
		.map_or(name.as_str(), |(old, new)| if up { new } else { old });
	b.iter().position(|k| k == name).map(|i| i as u8)
}

pub fn effect_key(display: &str) -> String {
	if display == "BadLuck" {
		return "unluck".to_owned();
	}
	let mut out = String::new();
	for (i, c) in display.chars().enumerate() {
		if c.is_ascii_uppercase() {
			if i > 0 {
				out.push('_');
			}
			out.push(c.to_ascii_lowercase());
		} else {
			out.push(c);
		}
	}
	out
}

pub fn effect_by_key(version: Version, key: &str) -> Option<i32> {
	let key = key.strip_prefix("minecraft:").unwrap_or(key);
	let names = registry::names(version, "effects")?;
	names
		.names
		.iter()
		.find(|(_, n)| effect_key(n) == key)
		.map(|(id, _)| *id as i32)
}

pub fn effect_name(version: Version, id: i32) -> Option<String> {
	registry::names(version, "effects")?
		.name(id as u32)
		.map(|n| format!("minecraft:{}", effect_key(n)))
}
