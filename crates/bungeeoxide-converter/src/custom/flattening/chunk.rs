use std::collections::HashMap;
use std::sync::OnceLock;

use bungeeoxide_nbt::{CompoundExt, CompoundVecExt, Entry, Payload};

use super::blocks::{
	block_of_legacy, earlier_states, later_states, legacy_of_block, new_blocks, old_blocks,
	old_items, raw_of_state, state_of_raw, DYES,
};
use super::connect::{self, Columns};
use super::nbtx::{children, invert_colors, root};
use super::text::legacy_to_json;
use super::wire::{put_varint, Reader};
use crate::gen::convert::{v1_12_2 as a, v1_13 as b, v1_13_1 as c};
use crate::pds::convert::{unconvertible, Result};
use crate::registry;
use crate::session::Session;

type Client12 = a::play_to_client_packet::Ty;
type Client13 = b::play_to_client_packet::Ty;
type Chunk = b::play_to_client_packet_map_chunk::Ty;
type Position = b::position::Ty;

const BLOCKS: usize = 4096;
const LIGHT: usize = 2048;

struct Section {
	index: usize,
	states: Vec<u32>,
	light: Vec<u8>,
}

#[derive(Clone, Copy, PartialEq)]
enum Format {
	Legacy,
	Flat,
}

impl Format {
	fn global_bits(self) -> usize {
		match self {
			Format::Legacy => 13,
			Format::Flat => 14,
		}
	}
}

struct Column {
	sections: Vec<Section>,
	biomes: Option<Vec<i32>>,
}

fn read_section(r: &mut Reader, format: Format) -> Result<Vec<u32>> {
	let bits = r.u8()? as usize;
	let palette: Option<Vec<u32>> = if bits <= 8 || format == Format::Legacy {
		let n = r.varint()?;
		let entries = (0..n)
			.map(|_| r.varint().map(|v| v as u32))
			.collect::<Result<Vec<_>>>()?;
		(bits <= 8).then_some(entries)
	} else {
		None
	};
	if bits == 0 || bits > 32 {
		return unconvertible(format!("chunk section with {bits} bits per block"));
	}
	let longs = r.varint()? as usize;
	if longs * 64 < BLOCKS * bits {
		return unconvertible(format!("chunk section has {longs} longs for {bits} bits"));
	}
	let data = (0..longs).map(|_| r.u64()).collect::<Result<Vec<_>>>()?;
	let mask = (1u64 << bits) - 1;
	(0..BLOCKS)
		.map(|i| {
			let bit = i * bits;
			let (word, off) = (bit / 64, bit % 64);
			let mut v = data[word] >> off;
			if off + bits > 64 {
				v |= data[word + 1] << (64 - off);
			}
			let v = (v & mask) as u32;
			match &palette {
				Some(p) => p
					.get(v as usize)
					.copied()
					.map_or_else(|| unconvertible("chunk palette index out of range"), Ok),
				None => Ok(v),
			}
		})
		.collect()
}

fn write_section(out: &mut Vec<u8>, states: &[u32], format: Format) {
	let mut palette: Vec<u32> = Vec::new();
	let mut index: HashMap<u32, u32> = HashMap::new();
	for s in states {
		if index.len() > 256 {
			break;
		}
		index.entry(*s).or_insert_with(|| {
			palette.push(*s);
			palette.len() as u32 - 1
		});
	}
	let (bits, values): (usize, Vec<u64>) = if palette.len() <= 256 {
		let bits = (usize::BITS - (palette.len().max(1) - 1).leading_zeros()).max(4) as usize;
		out.push(bits as u8);
		put_varint(out, palette.len() as i32);
		for p in &palette {
			put_varint(out, *p as i32);
		}
		(bits, states.iter().map(|s| index[s] as u64).collect())
	} else {
		let bits = format.global_bits();
		out.push(bits as u8);
		if format == Format::Legacy {
			put_varint(out, 0);
		}
		(bits, states.iter().map(|s| *s as u64).collect())
	};
	let mut longs = vec![0u64; (BLOCKS * bits).div_ceil(64)];
	for (i, v) in values.into_iter().enumerate() {
		let bit = i * bits;
		let (word, off) = (bit / 64, bit % 64);
		longs[word] |= v << off;
		if off + bits > 64 {
			longs[word + 1] |= v >> (64 - off);
		}
	}
	put_varint(out, longs.len() as i32);
	for l in longs {
		out.extend_from_slice(&l.to_be_bytes());
	}
}

fn parse_with(data: &[u8], bitmap: i32, full: bool, sky: bool, format: Format) -> Result<Column> {
	let mut r = Reader::new(data);
	let mut sections = Vec::new();
	for index in 0..16 {
		if bitmap & (1 << index) == 0 {
			continue;
		}
		let states = read_section(&mut r, format)?;
		let light = r.bytes(if sky { LIGHT * 2 } else { LIGHT })?;
		sections.push(Section {
			index,
			states,
			light,
		});
	}
	let biomes = match (full, format) {
		(false, _) => None,
		(true, Format::Legacy) => Some(r.bytes(256)?.into_iter().map(|b| b as i32).collect()),
		(true, Format::Flat) => Some((0..256).map(|_| r.i32()).collect::<Result<Vec<_>>>()?),
	};
	if r.remaining() != 0 {
		return unconvertible(format!("{} bytes left after chunk data", r.remaining()));
	}
	Ok(Column { sections, biomes })
}

fn parse(data: &[u8], bitmap: i32, full: bool, sky: &mut bool, format: Format) -> Result<Column> {
	match parse_with(data, bitmap, full, *sky, format) {
		Ok(c) => Ok(c),
		Err(e) => match parse_with(data, bitmap, full, !*sky, format) {
			Ok(c) => {
				*sky = !*sky;
				Ok(c)
			}
			Err(_) => Err(e),
		},
	}
}

fn write(column: &Column, format: Format) -> Vec<u8> {
	let mut out = Vec::new();
	for s in &column.sections {
		write_section(&mut out, &s.states, format);
		out.extend_from_slice(&s.light);
	}
	if let Some(biomes) = &column.biomes {
		for b in biomes {
			match format {
				Format::Legacy => out.push(*b as u8),
				Format::Flat => out.extend_from_slice(&b.to_be_bytes()),
			}
		}
	}
	out
}

#[derive(Clone, Copy, PartialEq)]
enum Special {
	None,
	Pot,
	Bed,
	Banner,
	WallBanner,
	Skull,
	WallSkull,
}

fn special(state: u32) -> Special {
	static TABLE: OnceLock<Vec<Special>> = OnceLock::new();
	let table = TABLE.get_or_init(|| {
		let b = new_blocks();
		(0..b.states())
			.map(|s| match b.owner(s).map(|o| o.name.as_str()) {
				Some("flower_pot") => Special::Pot,
				Some(n) if n.starts_with("potted_") => Special::Pot,
				Some("red_bed") => Special::Bed,
				Some("white_banner") => Special::Banner,
				Some("white_wall_banner") => Special::WallBanner,
				Some("skeleton_skull") => Special::Skull,
				Some("skeleton_wall_skull") => Special::WallSkull,
				_ => Special::None,
			})
			.collect()
	});
	table.get(state as usize).copied().unwrap_or(Special::None)
}

const SKULLS: [(&str, &str); 6] = [
	("skeleton_skull", "skeleton_wall_skull"),
	("wither_skeleton_skull", "wither_skeleton_wall_skull"),
	("zombie_head", "zombie_wall_head"),
	("player_head", "player_wall_head"),
	("creeper_head", "creeper_wall_head"),
	("dragon_head", "dragon_wall_head"),
];

fn with_block(orig: u32, name: &str, overrides: &[(&str, String)]) -> Option<u32> {
	let b = new_blocks();
	let block = b.owner(orig)?;
	let target = b.block(name)?;
	let mut values: Vec<(&str, &str)> = block.values(orig);
	for (k, v) in overrides {
		match values.iter_mut().find(|(n, _)| n == k) {
			Some(slot) => slot.1 = v,
			None => values.push((k, v)),
		}
	}
	Some(target.state(&values))
}

fn pot_content(te: &[Entry]) -> u32 {
	let b = new_blocks();
	let empty = b.block("flower_pot").map_or(0, |p| p.default);
	let item = match te.field("Item") {
		Some(Payload::String(s)) => old_items().id(s).map(|i| i as i32),
		Some(p) => p.to_i32(),
		None => None,
	};
	let data = te.field("Data").and_then(Payload::to_i32).unwrap_or(0);
	let Some(item) = item.and_then(|i| u16::try_from(i).ok()) else {
		return empty;
	};
	let name = u8::try_from(data)
		.ok()
		.and_then(|d| registry::legacy_item(item, d))
		.or_else(|| registry::legacy_item(item, 0));
	name.and_then(|n| b.block(&format!("potted_{}", super::blocks::base(n))))
		.map_or(empty, |p| p.default)
}

fn replacement(orig: u32, te: &[Entry]) -> Option<u32> {
	let color = |key: &str, invert: bool| {
		let c = te
			.field(key)
			.and_then(Payload::to_i32)
			.unwrap_or(0)
			.clamp(0, 15);
		DYES[if invert { 15 - c } else { c } as usize]
	};
	match special(orig) {
		Special::None => None,
		Special::Pot => Some(pot_content(te)),
		Special::Bed => with_block(orig, &format!("{}_bed", color("color", false)), &[]),
		Special::Banner => with_block(orig, &format!("{}_banner", color("Base", true)), &[]),
		Special::WallBanner => {
			with_block(orig, &format!("{}_wall_banner", color("Base", true)), &[])
		}
		Special::Skull | Special::WallSkull => {
			let kind = te
				.field("SkullType")
				.and_then(Payload::to_i32)
				.unwrap_or(0)
				.clamp(0, 5) as usize;
			let (floor, wall) = SKULLS[kind];
			if special(orig) == Special::WallSkull {
				with_block(orig, wall, &[])
			} else {
				let rot = te.field("Rot").and_then(Payload::to_i32).unwrap_or(0) & 15;
				with_block(orig, floor, &[("rotation", rot.to_string())])
			}
		}
	}
}

fn te_id(te: &[Entry]) -> String {
	let id = te.field("id").and_then(Payload::as_str).unwrap_or_default();
	let id = id
		.strip_prefix("minecraft:")
		.unwrap_or(id)
		.to_ascii_lowercase();
	match id.as_str() {
		"flowerpot" => "flower_pot",
		"mobspawner" => "mob_spawner",
		"control" => "command_block",
		"enchanttable" => "enchanting_table",
		"trap" => "dispenser",
		"musicbox" => "noteblock",
		_ => return id,
	}
	.to_owned()
}

fn fix_te(te: &mut Vec<Entry>) {
	let id = te_id(te);
	let named =
		matches!(
			id.as_str(),
			"chest"
				| "dispenser"
				| "dropper" | "enchanting_table"
				| "furnace" | "hopper"
				| "shulker_box"
				| "banner" | "command_block"
		);
	if named {
		if let Some(name) = te
			.field("CustomName")
			.and_then(Payload::as_str)
			.map(str::to_owned)
		{
			te.put("CustomName", Payload::String(legacy_to_json(&name, false)));
		}
	}
	if id == "banner" {
		invert_colors(te);
	}
	if id == "mob_spawner" {
		if let Some(spawn) = te.compound_mut("SpawnData") {
			if let Some(name) = spawn
				.field("id")
				.and_then(Payload::as_str)
				.map(str::to_owned)
			{
				spawn.put(
					"id",
					Payload::String(super::entities::flat_entity_name(&name)),
				);
			}
		}
	}
}

#[derive(Clone, Copy)]
struct Stored {
	original: u32,
	replaced: Option<u32>,
}

#[derive(Default)]
pub(super) struct World {
	pub no_sky: bool,
	stored: HashMap<(i32, i32), HashMap<(i32, i32, i32), Stored>>,
	columns: Columns,
}

impl World {
	pub fn clear(&mut self) {
		self.stored.clear();
		self.columns.clear();
	}

	fn check(&mut self, (x, y, z): (i32, i32, i32), state: u32) -> u32 {
		let chunk = self.stored.entry((x >> 4, z >> 4)).or_default();
		if let Some(s) = chunk.get(&(x, y, z)).copied() {
			if s.original == state {
				return s.replaced.unwrap_or(state);
			}
			chunk.remove(&(x, y, z));
		}
		if special(state) != Special::None {
			chunk.insert(
				(x, y, z),
				Stored {
					original: state,
					replaced: None,
				},
			);
		}
		state
	}
}

fn position(p: &Position) -> (i32, i32, i32) {
	(p.x, p.y as i32, p.z)
}

fn sky_of(cx: &mut Session) -> bool {
	!cx.state::<World>().no_sky
}

#[derive(Clone, Copy)]
pub struct FlatChunk {
	pub x: i32,
	pub z: i32,
	pub full: bool,
	pub sky: bool,
}

fn flatten(
	cx: &mut Session,
	at: FlatChunk,
	sections: &mut [Section],
	biomes: Option<&mut Vec<i32>>,
	block_entities: Vec<Entry>,
) -> (Vec<Entry>, Vec<Client13>) {
	let FlatChunk { x, z, full, sky } = at;
	let world = cx.state::<World>();
	world.no_sky = !sky;
	let key = (x, z);
	if full {
		world.stored.remove(&key);
	}
	let stored = world.stored.entry(key).or_default();
	for section in sections.iter_mut() {
		let y0 = section.index as i32 * 16;
		stored.retain(|(_, y, _), _| *y < y0 || *y >= y0 + 16);
		for (i, s) in section.states.iter_mut().enumerate() {
			*s = state_of_raw(*s);
			if special(*s) != Special::None {
				let pos = (
					x * 16 + (i & 15) as i32,
					y0 + (i >> 8) as i32,
					z * 16 + (i >> 4 & 15) as i32,
				);
				stored.insert(
					pos,
					Stored {
						original: *s,
						replaced: None,
					},
				);
			}
		}
	}
	if let Some(biomes) = biomes {
		for b in biomes.iter_mut() {
			if !super::blocks::biome_valid(*b as u8) {
				*b = 1;
			}
		}
	}
	let mut entities = Vec::with_capacity(block_entities.len());
	for te in block_entities {
		let mut tag = children(Some(te));
		let at = (
			tag.field("x").and_then(Payload::to_i32),
			tag.field("y").and_then(Payload::to_i32),
			tag.field("z").and_then(Payload::to_i32),
		);
		if let (Some(x), Some(y), Some(z)) = at {
			let section = sections.iter_mut().find(|s| s.index as i32 == y >> 4);
			if let Some(section) = section {
				let i = ((y & 15) << 8 | (z & 15) << 4 | (x & 15)) as usize;
				let original = section.states[i];
				if let Some(new) = replacement(original, &tag) {
					section.states[i] = new;
					stored.insert(
						(x, y, z),
						Stored {
							original,
							replaced: Some(new),
						},
					);
				}
			}
		}
		let id = te_id(&tag);
		if id == "flower_pot" || id == "noteblock" {
			continue;
		}
		fix_te(&mut tag);
		entities.extend(root(tag));
	}
	let neighbors = world.connect_column(x, z, full, sections);
	(entities, neighbors)
}

pub fn flattening_map_chunk_1_13(
	p: a::play_to_client_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let mut sky = sky_of(cx);
	let mut column = parse(
		&p.chunk_data,
		p.bit_map,
		p.ground_up,
		&mut sky,
		Format::Legacy,
	)?;
	let at = FlatChunk {
		x: p.x,
		z: p.z,
		full: p.ground_up,
		sky,
	};
	let (entities, neighbors) = flatten(
		cx,
		at,
		&mut column.sections,
		column.biomes.as_mut(),
		p.block_entities,
	);
	let mut out = vec![Client13::MapChunk(Chunk {
		x: p.x,
		z: p.z,
		ground_up: p.ground_up,
		bit_map: p.bit_map,
		chunk_data: write(&column, Format::Flat),
		block_entities: entities,
	})];
	out.extend(neighbors);
	Ok(out)
}

pub struct FlatColumn {
	pub sections: Vec<(usize, Vec<u32>)>,
	pub block_entities: Vec<Entry>,
	pub neighbors: Vec<Client13>,
}

pub fn flattening_column_1_13(
	cx: &mut Session,
	at: FlatChunk,
	sections: Vec<(usize, Vec<u32>)>,
	biomes: Option<&mut Vec<i32>>,
	block_entities: Vec<Entry>,
) -> FlatColumn {
	let mut sections: Vec<Section> = sections
		.into_iter()
		.map(|(index, states)| Section {
			index,
			states,
			light: Vec::new(),
		})
		.collect();
	let (block_entities, neighbors) = flatten(cx, at, &mut sections, biomes, block_entities);
	FlatColumn {
		sections: sections.into_iter().map(|s| (s.index, s.states)).collect(),
		block_entities,
		neighbors,
	}
}

fn changes(updates: Vec<((i32, i32, i32), u32)>) -> Vec<Client13> {
	let mut chunks: Vec<b::play_to_client_packet_multi_block_change::Ty> = Vec::new();
	for ((x, y, z), state) in updates {
		let record = b::play_to_client_packet_multi_block_change_records_items::Ty {
			horizontal_pos: ((x & 15) << 4 | (z & 15)) as u8,
			y: y as u8,
			block_id: state as i32,
		};
		match chunks
			.iter_mut()
			.find(|c| (c.chunk_x, c.chunk_z) == (x >> 4, z >> 4))
		{
			Some(c) => c.records.push(record),
			None => chunks.push(b::play_to_client_packet_multi_block_change::Ty {
				chunk_x: x >> 4,
				chunk_z: z >> 4,
				records: vec![record],
			}),
		}
	}
	chunks.into_iter().map(Client13::MultiBlockChange).collect()
}

impl World {
	fn connect_column(
		&mut self,
		cx: i32,
		cz: i32,
		full: bool,
		sections: &mut [Section],
	) -> Vec<Client13> {
		if full {
			self.columns.unload(cx, cz);
		}
		for s in sections.iter() {
			self.columns.load(cx, cz, s.index, &s.states);
		}
		for s in sections.iter_mut() {
			let y0 = s.index as i32 * 16;
			for (i, state) in s.states.iter_mut().enumerate() {
				if !connect::handled(*state) {
					continue;
				}
				let pos = (
					cx * 16 + (i & 15) as i32,
					y0 + (i >> 8) as i32,
					cz * 16 + (i >> 4 & 15) as i32,
				);
				let new = connect::connect(&self.columns, pos, *state);
				if new != *state {
					*state = new;
					self.columns.set(pos.0, pos.1, pos.2, new);
				}
			}
		}
		let mut updates = Vec::new();
		for s in sections.iter() {
			let y0 = s.index as i32 * 16;
			for y in y0..y0 + 16 {
				for i in 0..16 {
					for (x, z) in [(-1, i), (16, i), (i, -1), (i, 16)] {
						let pos = (cx * 16 + x, y, cz * 16 + z);
						let state = self.columns.get(pos.0, pos.1, pos.2);
						if !connect::handled(state) {
							continue;
						}
						let new = connect::connect(&self.columns, pos, state);
						if new != state {
							self.columns.set(pos.0, pos.1, pos.2, new);
							updates.push((pos, new));
						}
					}
				}
			}
		}
		changes(updates)
	}

	fn place(&mut self, pos: (i32, i32, i32), state: u32) -> u32 {
		self.columns.set(pos.0, pos.1, pos.2, state);
		let new = connect::connect(&self.columns, pos, state);
		if new != state {
			self.columns.set(pos.0, pos.1, pos.2, new);
		}
		new
	}

	fn around(&mut self, positions: &[(i32, i32, i32)]) -> Vec<Client13> {
		let mut updates = Vec::new();
		for pos in positions {
			connect::update_around(&mut self.columns, *pos, &mut updates);
		}
		updates.retain(|(p, _)| !positions.contains(p));
		changes(updates)
	}
}

pub fn flattening_explosion_1_13(
	p: a::play_to_client_packet_explosion::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let world = cx.state::<World>();
	let (x, y, z) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
	let positions: Vec<(i32, i32, i32)> = p
		.affected_block_offsets
		.iter()
		.map(|o| (x + o.x as i32, y + o.y as i32, z + o.z as i32))
		.collect();
	for pos in &positions {
		world.columns.set(pos.0, pos.1, pos.2, 0);
	}
	let mut out = vec![Client13::Explosion(p)];
	out.extend(world.around(&positions));
	Ok(out)
}

pub fn flattening_map_chunk_1_12_2(p: Chunk, cx: &mut Session) -> Result<Vec<Client12>> {
	let mut sky = sky_of(cx);
	let mut column = parse(
		&p.chunk_data,
		p.bit_map,
		p.ground_up,
		&mut sky,
		Format::Flat,
	)?;
	cx.state::<World>().no_sky = !sky;
	for section in &mut column.sections {
		for s in section.states.iter_mut() {
			*s = raw_of_state(*s);
		}
	}
	Ok(vec![Client12::MapChunk(
		a::play_to_client_packet_map_chunk::Ty {
			x: p.x,
			z: p.z,
			ground_up: p.ground_up,
			bit_map: p.bit_map,
			chunk_data: write(&column, Format::Legacy),
			block_entities: p.block_entities,
		},
	)])
}

fn restate(p: Chunk, cx: &mut Session, table: &[u32]) -> Result<Chunk> {
	let mut sky = sky_of(cx);
	let mut column = parse(
		&p.chunk_data,
		p.bit_map,
		p.ground_up,
		&mut sky,
		Format::Flat,
	)?;
	cx.state::<World>().no_sky = !sky;
	for section in &mut column.sections {
		for s in section.states.iter_mut() {
			*s = table.get(*s as usize).copied().unwrap_or(0);
		}
	}
	Ok(Chunk {
		chunk_data: write(&column, Format::Flat),
		..p
	})
}

pub fn flattening_chunk_states_1_13_1(
	p: Chunk,
	cx: &mut Session,
) -> Result<Vec<c::play_to_client_packet::Ty>> {
	Ok(vec![c::play_to_client_packet::Ty::MapChunk(restate(
		p,
		cx,
		later_states(),
	)?)])
}

pub fn flattening_chunk_states_1_13(p: Chunk, cx: &mut Session) -> Result<Vec<Client13>> {
	Ok(vec![Client13::MapChunk(restate(p, cx, earlier_states())?)])
}

pub fn flattening_unload_chunk_1_13(
	p: a::play_to_client_packet_unload_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let world = cx.state::<World>();
	world.stored.remove(&(p.chunk_x, p.chunk_z));
	world.columns.unload(p.chunk_x, p.chunk_z);
	Ok(vec![Client13::UnloadChunk(p)])
}

pub fn flattening_block_change_1_13(
	p: a::play_to_client_packet_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let world = cx.state::<World>();
	let pos = position(&p.location);
	let state = world.check(pos, state_of_raw(p.r#type.max(0) as u32));
	let state = world.place(pos, state);
	let mut out = vec![Client13::BlockChange(
		b::play_to_client_packet_block_change::Ty {
			location: p.location,
			r#type: state as i32,
		},
	)];
	out.extend(world.around(&[pos]));
	Ok(out)
}

#[derive(Default)]
pub(super) struct Backwards {
	blocks: HashMap<(i32, i32, i32), u32>,
}

impl Backwards {
	pub fn clear(&mut self) {
		self.blocks.clear();
	}

	fn store(&mut self, pos: (i32, i32, i32), state: u32) {
		let keep = new_blocks().owner(state).is_some_and(|b| {
			let n = b.name.as_str();
			n == "flower_pot"
				|| n == "note_block"
				|| n == "spawner"
				|| n.starts_with("potted_")
				|| n.ends_with("_bed")
				|| n.ends_with("_banner")
				|| n.ends_with("_skull")
				|| n.ends_with("_head")
		});
		if keep {
			self.blocks.insert(pos, state);
		} else {
			self.blocks.remove(&pos);
		}
	}
}

fn block_name(state: u32) -> Option<&'static str> {
	new_blocks().owner(state).map(|b| b.name.as_str())
}

fn property(state: u32, key: &str) -> Option<&'static str> {
	let b = new_blocks().owner(state)?;
	b.values(state)
		.into_iter()
		.find(|(k, _)| *k == key)
		.map(|(_, v)| v)
}

fn pot_te(pos: (i32, i32, i32), state: u32) -> Option<bungeeoxide_nbt::Entry> {
	let name = block_name(state)?;
	let (item, data) = match name.strip_prefix("potted_") {
		Some(plant) => {
			let id = new_blocks()
				.block(plant)
				.map(|b| raw_of_state(b.default))
				.unwrap_or(0);
			let old = old_blocks()
				.list
				.iter()
				.find(|b| b.id == id >> 4)
				.map_or("air", |b| b.name.as_str());
			(format!("minecraft:{old}"), (id & 15) as i32)
		}
		None if name == "flower_pot" => ("minecraft:air".to_owned(), 0),
		None => return None,
	};
	root(vec![
		Entry::new("id", Payload::String("minecraft:flower_pot".to_owned())),
		Entry::new("x", Payload::Int(pos.0)),
		Entry::new("y", Payload::Int(pos.1)),
		Entry::new("z", Payload::Int(pos.2)),
		Entry::new("Item", Payload::String(item)),
		Entry::new("Data", Payload::Int(data)),
	])
}

fn legacy_block_packets(
	location: &Position,
	state: u32,
	backwards: &mut Backwards,
) -> Vec<Client12> {
	let pos = position(location);
	backwards.store(pos, state);
	let raw = raw_of_state(state) as i32;
	let change = |r#type: i32| {
		Client12::BlockChange(a::play_to_client_packet_block_change::Ty {
			location: location.clone(),
			r#type,
		})
	};
	match pot_te(pos, state) {
		Some(te) => vec![
			change(0),
			change(raw),
			Client12::TileEntityData(a::play_to_client_packet_tile_entity_data::Ty {
				location: location.clone(),
				action: 5,
				nbt_data: Some(te),
			}),
		],
		None => vec![change(raw)],
	}
}

pub fn flattening_block_change_1_12_2(
	p: b::play_to_client_packet_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client12>> {
	Ok(legacy_block_packets(
		&p.location,
		p.r#type.max(0) as u32,
		cx.state::<Backwards>(),
	))
}

pub fn flattening_unload_chunk_1_12_2(
	p: b::play_to_client_packet_unload_chunk::Ty,
	cx: &mut Session,
) -> Result<Vec<Client12>> {
	cx.state::<Backwards>()
		.blocks
		.retain(|(x, _, z), _| (x >> 4, z >> 4) != (p.chunk_x, p.chunk_z));
	Ok(vec![Client12::UnloadChunk(p)])
}

const SKULL_TYPES: [&str; 6] = [
	"skeleton",
	"wither_skeleton",
	"zombie",
	"player",
	"creeper",
	"dragon",
];

pub fn flattening_tile_entity_data_1_12_2(
	p: b::play_to_client_packet_tile_entity_data::Ty,
	cx: &mut Session,
) -> Result<Vec<Client12>> {
	if p.action == 5 {
		return Ok(vec![]);
	}
	let mut tag = children(p.nbt_data);
	let state = cx
		.state::<Backwards>()
		.blocks
		.get(&position(&p.location))
		.copied();
	let name = state.and_then(block_name).unwrap_or_default();
	let color = DYES.iter().position(|c| {
		[
			format!("{c}_bed"),
			format!("{c}_banner"),
			format!("{c}_wall_banner"),
		]
		.iter()
		.any(|n| n == name)
	});
	match te_id(&tag).as_str() {
		"bed" => {
			if let Some(c) = color {
				tag.put("color", Payload::Int(c as i32));
			}
		}
		"banner" => {
			if let Some(c) = color {
				tag.put("Base", Payload::Int(15 - c as i32));
			}
			if let Some(patterns) = tag.compounds_mut("Patterns") {
				for pat in patterns {
					if let Some(c) = pat.field("Color").and_then(Payload::to_i32) {
						pat.put("Color", Payload::Int(15 - c));
					}
				}
			}
			unname(&mut tag);
		}
		"skull" => {
			let kind = SKULL_TYPES
				.iter()
				.position(|k| name.starts_with(&format!("{k}_")))
				.unwrap_or(0);
			tag.put("SkullType", Payload::Byte(kind as i8));
			if let Some(rot) = state
				.and_then(|s| property(s, "rotation"))
				.and_then(|r| r.parse::<i8>().ok())
			{
				tag.put("Rot", Payload::Byte(rot));
			}
		}
		"mob_spawner" => {
			if let Some(spawn) = tag.compound_mut("SpawnData") {
				if let Some(id) = spawn
					.field("id")
					.and_then(Payload::as_str)
					.map(str::to_owned)
				{
					spawn.put(
						"id",
						Payload::String(super::entities::legacy_entity_name(&id)),
					);
				}
			}
		}
		"piston" => {
			let block = tag.compound_mut("blockState").map(|s| piston_state(s));
			if let Some(raw) = block.flatten() {
				tag.put("blockId", Payload::Int((raw >> 4) as i32));
				tag.put("blockData", Payload::Int((raw & 15) as i32));
			}
		}
		_ => unname(&mut tag),
	}
	Ok(vec![Client12::TileEntityData(
		a::play_to_client_packet_tile_entity_data::Ty {
			location: p.location,
			action: p.action,
			nbt_data: root(tag),
		},
	)])
}

fn unname(tag: &mut Vec<bungeeoxide_nbt::Entry>) {
	if let Some(name) = tag
		.field("CustomName")
		.and_then(Payload::as_str)
		.map(str::to_owned)
	{
		tag.put(
			"CustomName",
			Payload::String(super::text::json_to_legacy(&name)),
		);
	}
}

fn piston_state(state: &[bungeeoxide_nbt::Entry]) -> Option<u32> {
	let name = state.field("Name").and_then(Payload::as_str)?;
	let props: Vec<String> = match state.field("Properties") {
		Some(Payload::Compound(p)) => p
			.iter()
			.filter_map(|e| Some(format!("{}={}", e.name, e.value.as_str()?)))
			.collect(),
		_ => Vec::new(),
	};
	let text = if props.is_empty() {
		name.to_owned()
	} else {
		format!("{name}[{}]", props.join(","))
	};
	Some(raw_of_state(new_blocks().parse_state(&text)?))
}

pub fn flattening_multi_block_change_1_13(
	mut p: a::play_to_client_packet_multi_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let world = cx.state::<World>();
	let mut positions = Vec::with_capacity(p.records.len());
	for r in &mut p.records {
		let pos = (
			p.chunk_x * 16 + (r.horizontal_pos >> 4) as i32,
			r.y as i32,
			p.chunk_z * 16 + (r.horizontal_pos & 15) as i32,
		);
		let state = world.check(pos, state_of_raw(r.block_id.max(0) as u32));
		world.columns.set(pos.0, pos.1, pos.2, state);
		r.block_id = state as i32;
		positions.push(pos);
	}
	for (r, pos) in p.records.iter_mut().zip(&positions) {
		r.block_id = world.place(*pos, r.block_id as u32) as i32;
	}
	let mut out = vec![Client13::MultiBlockChange(p)];
	out.extend(world.around(&positions));
	Ok(out)
}

pub fn flattening_multi_block_change_1_12_2(
	mut p: b::play_to_client_packet_multi_block_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client12>> {
	let backwards = cx.state::<Backwards>();
	let mut extra = Vec::new();
	for r in &mut p.records {
		let location = Position {
			x: p.chunk_x * 16 + (r.horizontal_pos >> 4) as i32,
			y: r.y as i16,
			z: p.chunk_z * 16 + (r.horizontal_pos & 15) as i32,
		};
		let state = r.block_id.max(0) as u32;
		r.block_id = raw_of_state(state) as i32;
		let mut packets = legacy_block_packets(&location, state, backwards);
		if packets.len() > 1 {
			extra.append(&mut packets);
		}
	}
	let mut out = vec![Client12::MultiBlockChange(p)];
	out.extend(extra);
	Ok(out)
}

const INSTRUMENTS: [&str; 10] = [
	"harp",
	"basedrum",
	"snare",
	"hat",
	"bass",
	"flute",
	"bell",
	"guitar",
	"chime",
	"xylophone",
];

pub fn flattening_block_action_1_13(
	p: a::play_to_client_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client13>> {
	let block = block_of_legacy(p.block_id);
	let mut out = Vec::new();
	if p.block_id == 25 {
		let note = new_blocks().block("note_block").map(|n| {
			n.state(&[
				(
					"instrument",
					INSTRUMENTS.get(p.byte1 as usize).copied().unwrap_or("harp"),
				),
				("note", &(p.byte2 as u32 % 25).to_string()),
				("powered", "false"),
			])
		});
		if let Some(state) = note {
			out.push(Client13::BlockChange(
				b::play_to_client_packet_block_change::Ty {
					location: p.location.clone(),
					r#type: state as i32,
				},
			));
		}
	}
	out.push(Client13::BlockAction(
		b::play_to_client_packet_block_action::Ty {
			block_id: block,
			..p
		},
	));
	Ok(out)
}

pub fn flattening_block_action_1_12_2(
	p: b::play_to_client_packet_block_action::Ty,
	cx: &mut Session,
) -> Result<Vec<Client12>> {
	let block_id = legacy_of_block(p.block_id);
	let state = cx
		.state::<Backwards>()
		.blocks
		.get(&position(&p.location))
		.copied();
	let (byte1, byte2) = match state.filter(|s| block_name(*s) == Some("note_block")) {
		Some(s) => (
			property(s, "instrument")
				.and_then(|i| INSTRUMENTS.iter().position(|n| *n == i))
				.unwrap_or(0) as u8,
			property(s, "note")
				.and_then(|n| n.parse().ok())
				.unwrap_or(p.byte2),
		),
		None => (p.byte1, p.byte2),
	};
	Ok(vec![Client12::BlockAction(
		a::play_to_client_packet_block_action::Ty {
			block_id,
			byte1,
			byte2,
			..p
		},
	)])
}

pub fn flattening_tile_entity_data_1_13(
	p: a::play_to_client_packet_tile_entity_data::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let mut tag = children(p.nbt_data);
	let pos = position(&p.location);
	let world = cx.state::<World>();
	let mut out = Vec::new();
	let stored = world
		.stored
		.get_mut(&(pos.0 >> 4, pos.2 >> 4))
		.and_then(|c| c.get_mut(&pos));
	if let Some(stored) = stored {
		if let Some(new) = replacement(stored.original, &tag) {
			stored.replaced = Some(new);
			out.push(Client13::BlockChange(
				b::play_to_client_packet_block_change::Ty {
					location: p.location.clone(),
					r#type: new as i32,
				},
			));
		}
	}
	if p.action == 5 || te_id(&tag) == "flower_pot" {
		return Ok(out);
	}
	fix_te(&mut tag);
	out.push(Client13::TileEntityData(
		b::play_to_client_packet_tile_entity_data::Ty {
			location: p.location,
			action: p.action,
			nbt_data: root(tag),
		},
	));
	Ok(out)
}

pub(super) fn reset_world(cx: &mut Session, dimension: i32) {
	let world = cx.state::<World>();
	world.clear();
	world.no_sky = dimension != 0;
	cx.state::<Backwards>().clear();
}
