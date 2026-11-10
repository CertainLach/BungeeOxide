use super::chunk::{self, Section};
use super::*;
use crate::gen::convert::{v1_18, v1_18_2};

type Client17 = v1_17_1::play_toClient_packet::Ty;
type Client18 = v1_18::play_toClient_packet::Ty;
type Chunk17 = v1_17_1::play_toClient_packet_map_chunk::Ty;
type Chunk18 = v1_18::play_toClient_packet_map_chunk::Ty;
type BlockEntity18 = v1_18::chunkBlockEntity::Ty;

const OLD: V = V::V1_17_1;
const NEW: V = V::V1_18;

pub fn caves_slot_1_18(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot(OLD, NEW, v))
}

pub fn caves_slot_down_1_17_1(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot(NEW, OLD, v))
}

pub fn caves_particle_1_18(
	v: v1_17_1::particle::Ty,
	_cx: &mut Session,
) -> Result<v1_18::particle::Ty> {
	particle(
		OLD,
		NEW,
		&v,
		v1_17_1::particle::serialize,
		v1_18::particle::deserialize,
	)
}

pub fn caves_particle_down_1_17_1(
	v: v1_18::particle::Ty,
	_cx: &mut Session,
) -> Result<v1_17_1::particle::Ty> {
	particle(
		NEW,
		OLD,
		&v,
		v1_18::particle::serialize,
		v1_17_1::particle::deserialize,
	)
}

pub fn caves_world_particles_1_18(
	v: v1_17_1::play_toClient_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<v1_18::play_toClient_packet_world_particles::Ty> {
	let data = encode(&v.data, v1_17_1::particleData::serialize)?;
	let Some((id, data)) = particle_data(OLD, NEW, v.particleId, &data)? else {
		return Err(Error::Dropped);
	};
	let data = v1_18::particleData::deserialize(
		&mut Input::new(&data),
		v1_18::particleData::Args { compareTo: &id },
	)
	.map_err(encoding)?;
	Ok(v1_18::play_toClient_packet_world_particles::Ty {
		particleId: id,
		longDistance: v.longDistance,
		x: v.x,
		y: v.y,
		z: v.z,
		offsetX: v.offsetX,
		offsetY: v.offsetY,
		offsetZ: v.offsetZ,
		particleData: v.particleData,
		particles: v.particles,
		data,
	})
}

pub fn caves_world_particles_down_1_17_1(
	v: v1_18::play_toClient_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<v1_17_1::play_toClient_packet_world_particles::Ty> {
	let data = encode(&v.data, v1_18::particleData::serialize)?;
	let Some((id, data)) = particle_data(NEW, OLD, v.particleId, &data)? else {
		return Err(Error::Dropped);
	};
	let data = v1_17_1::particleData::deserialize(
		&mut Input::new(&data),
		v1_17_1::particleData::Args { compareTo: &id },
	)
	.map_err(encoding)?;
	Ok(v1_17_1::play_toClient_packet_world_particles::Ty {
		particleId: id,
		longDistance: v.longDistance,
		x: v.x,
		y: v.y,
		z: v.z,
		offsetX: v.offsetX,
		offsetY: v.offsetY,
		offsetZ: v.offsetZ,
		particleData: v.particleData,
		particles: v.particles,
		data,
	})
}

pub fn caves_simulation_distance_1_18(
	src: &v1_17_1::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<i32> {
	let w = world(cx);
	w.track_codec(&src.dimensionCodec);
	w.track_dimension(&src.dimension);
	w.forget_chunks();
	Ok(src.viewDistance)
}

pub fn caves_respawn_1_18(
	v: v1_17_1::play_toClient_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<v1_18::play_toClient_packet_respawn::Ty> {
	let w = world(cx);
	w.track_dimension(&v.dimension);
	w.forget_chunks();
	Ok(v)
}

pub fn caves_unload_chunk_1_18(
	v: v1_17_1::play_toClient_packet_unload_chunk::Ty,
	cx: &mut Session,
) -> Result<v1_18::play_toClient_packet_unload_chunk::Ty> {
	let w = world(cx);
	w.loaded.remove(&(v.chunkX, v.chunkZ));
	w.light.remove(&(v.chunkX, v.chunkZ));
	Ok(v)
}

pub fn caves_update_light_1_18(p: Light, cx: &mut Session) -> Result<Vec<Client18>> {
	let w = world(cx);
	let at = (p.chunkX, p.chunkZ);
	if w.loaded.contains(&at) {
		return Ok(vec![Client18::update_light(p)]);
	}
	w.light.insert(at, p);
	Ok(vec![])
}

fn empty_light(x: i32, z: i32, sections: usize) -> Light {
	let all: Vec<i64> = (0..(sections + 2).div_ceil(64))
		.map(|i| match (sections + 2 - i * 64).min(64) {
			64 => -1,
			n => (1i64 << n) - 1,
		})
		.collect();
	Light {
		chunkX: x,
		chunkZ: z,
		trustEdges: false,
		skyLightMask: vec![],
		blockLightMask: vec![],
		emptySkyLightMask: all.clone(),
		emptyBlockLightMask: all,
		skyLight: vec![],
		blockLight: vec![],
	}
}

fn wrap_spawn_data(tag: &mut Entry) {
	if let Payload::compound(c) = &mut tag.value {
		if let Some(spawn) = c.take("SpawnData") {
			c.push(Entry::new("SpawnData", vec![Entry::new("entity", spawn)]));
		}
	}
}

fn unwrap_spawn_data(c: &mut Vec<Entry>) {
	if let Some(entity) = c
		.compound_mut("SpawnData")
		.and_then(|spawn| spawn.take("entity"))
	{
		c.put("SpawnData", entity);
	}
}

pub fn caves_tile_entity_nbt_1_18(
	src: &v1_17_1::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<Option<Entry>> {
	let mut tag = src.nbtData.clone();
	if let (1, Some(t)) = (src.action, &mut tag) {
		wrap_spawn_data(t);
	}
	Ok(tag)
}

pub fn caves_tile_entity_nbt_down_1_17_1(
	src: &v1_18::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<Option<Entry>> {
	let mut tag = src.nbtData.clone();
	if let (
		8,
		Some(Entry {
			value: Payload::compound(c),
			..
		}),
	) = (src.action, &mut tag)
	{
		unwrap_spawn_data(c);
	}
	Ok(tag)
}

fn block_entity_1_18(mut tag: Entry) -> Option<BlockEntity18> {
	let c = tag.value.as_compound()?;
	let id = c
		.field("id")
		.and_then(Payload::as_str)?
		.trim_start_matches("minecraft:");
	let ty = block_entity_types(NEW).iter().position(|n| *n == id)? as i32;
	let (bx, by, bz) = (
		c.field("x").and_then(Payload::as_i64)?,
		c.field("y").and_then(Payload::as_i64)?,
		c.field("z").and_then(Payload::as_i64)?,
	);
	if id == "mob_spawner" {
		wrap_spawn_data(&mut tag);
	}
	Some(BlockEntity18 {
		ANON: v1_18::chunkBlockEntity__ANON::Ty {
			x: (bx & 15) as u8,
			z: (bz & 15) as u8,
		},
		y: by as i16,
		r#type: ty,
		nbtData: Some(tag),
	})
}

fn block_entity_down_1_17_1(x: i32, z: i32, be: BlockEntity18) -> Option<Entry> {
	let id = *block_entity_types(NEW).get(be.r#type as usize)?;
	let mut c = match be.nbtData.map(|e| e.value) {
		Some(Payload::compound(c)) => c,
		_ => Vec::new(),
	};
	if id == "mob_spawner" {
		unwrap_spawn_data(&mut c);
	}
	c.put("id", Payload::string(format!("minecraft:{id}")));
	c.put("x", Payload::int(x * 16 + be.ANON.x as i32));
	c.put("y", Payload::int(be.y as i32));
	c.put("z", Payload::int(z * 16 + be.ANON.z as i32));
	Some(Entry::new("", Payload::compound(c)))
}

pub fn caves_block_entities_1_18(list: Vec<Entry>) -> Vec<BlockEntity18> {
	list.into_iter().filter_map(block_entity_1_18).collect()
}

pub fn caves_block_entities_1_17_1(x: i32, z: i32, list: Vec<BlockEntity18>) -> Vec<Entry> {
	list.into_iter()
		.filter_map(|b| block_entity_down_1_17_1(x, z, b))
		.collect()
}

pub struct CavesColumn {
	pub sections: usize,
	pub biome_bits: u32,
	pub light: Light,
}

pub fn caves_column_1_18(cx: &mut Session, x: i32, z: i32) -> CavesColumn {
	let w = world(cx);
	let sections = w.sections();
	let light = w
		.light
		.remove(&(x, z))
		.unwrap_or_else(|| empty_light(x, z, sections));
	w.loaded.insert((x, z));
	CavesColumn {
		sections,
		biome_bits: w.biome_bits(),
		light,
	}
}

pub fn caves_map_chunk_1_18(p: Chunk17, cx: &mut Session) -> Result<Vec<Client18>> {
	let n = world(cx).sections();
	let old = chunk::read_1_17(&p.chunkData, &p.bitMap, n)?;
	let biome = |i: usize| p.biomes.get(i).map_or(0, |b| (*b).max(0) as u32);
	let sections: Vec<Section> = old
		.into_iter()
		.enumerate()
		.map(|(s, blocks)| {
			let (count, blocks) = blocks.unwrap_or_else(|| (0, vec![0; chunk::BLOCKS]));
			Section {
				count,
				blocks,
				biomes: (0..chunk::BIOMES)
					.map(|i| biome(s * chunk::BIOMES + i))
					.collect(),
				raw_biomes: vec![],
			}
		})
		.collect();
	let column = caves_column_1_18(cx, p.x, p.z);
	let data = chunk::write_1_18(&sections, chunk::global_bits(NEW), column.biome_bits);
	let light = column.light;
	Ok(vec![Client18::map_chunk(Chunk18 {
		x: p.x,
		z: p.z,
		heightmaps: p.heightmaps,
		chunkData: data,
		blockEntities: caves_block_entities_1_18(p.blockEntities),
		trustEdges: light.trustEdges,
		skyLightMask: light.skyLightMask,
		blockLightMask: light.blockLightMask,
		emptySkyLightMask: light.emptySkyLightMask,
		emptyBlockLightMask: light.emptyBlockLightMask,
		skyLight: light.skyLight,
		blockLight: light.blockLight,
	})])
}

pub fn caves_map_chunk_down_1_17_1(p: Chunk18, cx: &mut Session) -> Result<Vec<Client17>> {
	let sections = chunk::read_1_18(&p.chunkData, NEW, world(cx).known_sections())?;
	let biomes = sections
		.iter()
		.flat_map(|s| s.biomes.iter().map(|b| *b as i32))
		.collect();
	let (mask, data) = chunk::write_1_17(&sections, chunk::global_bits(OLD));
	let light = Light {
		chunkX: p.x,
		chunkZ: p.z,
		trustEdges: p.trustEdges,
		skyLightMask: p.skyLightMask,
		blockLightMask: p.blockLightMask,
		emptySkyLightMask: p.emptySkyLightMask,
		emptyBlockLightMask: p.emptyBlockLightMask,
		skyLight: p.skyLight,
		blockLight: p.blockLight,
	};
	Ok(vec![
		Client17::update_light(light),
		Client17::map_chunk(Chunk17 {
			x: p.x,
			z: p.z,
			bitMap: mask,
			heightmaps: p.heightmaps,
			biomes,
			chunkData: data,
			blockEntities: caves_block_entities_1_17_1(p.x, p.z, p.blockEntities),
		}),
	])
}

const TILE_ACTIONS: [(u8, &str); 13] = [
	(1, "mob_spawner"),
	(2, "command_block"),
	(3, "beacon"),
	(4, "skull"),
	(5, "conduit"),
	(6, "banner"),
	(7, "structure_block"),
	(8, "end_gateway"),
	(9, "sign"),
	(11, "bed"),
	(12, "jigsaw"),
	(13, "campfire"),
	(14, "beehive"),
];

pub fn caves_tile_entity_action_1_18(
	src: &v1_17_1::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	let name = TILE_ACTIONS
		.iter()
		.find(|(a, _)| *a == src.action)
		.ok_or(Error::Dropped)?
		.1;
	block_entity_types(NEW)
		.iter()
		.position(|n| *n == name)
		.map(|i| i as i32)
		.ok_or(Error::Dropped)
}

pub fn caves_tile_entity_action_down_1_17_1(
	src: &v1_18::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<u8> {
	let name = block_entity_types(NEW)
		.get(src.action as usize)
		.copied()
		.ok_or(Error::Dropped)?;
	TILE_ACTIONS
		.iter()
		.find(|(_, n)| *n == name)
		.map(|(a, _)| *a)
		.ok_or(Error::Dropped)
}

pub fn caves_sound_1_18(
	src: &v1_17_1::play_toClient_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound(OLD, NEW, src.soundId)
}

pub fn caves_sound_down_1_17_1(
	src: &v1_18::play_toClient_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound(NEW, OLD, src.soundId)
}

pub fn caves_entity_sound_1_18(
	src: &v1_17_1::play_toClient_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound(OLD, NEW, src.soundId)
}

pub fn caves_entity_sound_down_1_17_1(
	src: &v1_18::play_toClient_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	sound(NEW, OLD, src.soundId)
}

pub fn caves_cooldown_item_1_18(
	src: &v1_17_1::play_toClient_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	item(OLD, NEW, src.itemID).ok_or(Error::Dropped)
}

pub fn caves_cooldown_item_down_1_17_1(
	src: &v1_18::play_toClient_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	item(NEW, OLD, src.itemID).ok_or(Error::Dropped)
}

const LAVA_POOL: (&str, &str) = (
	"minecraft:lava_pool_stone_replaceables",
	"minecraft:lava_pool_stone_cannot_replace",
);

pub fn caves_tags_1_18(p: Tags, _cx: &mut Session) -> Result<Vec<Client18>> {
	Ok(vec![Client18::tags(tags(
		OLD,
		NEW,
		p,
		&[LAVA_POOL],
		super::new_tags::NEW_1_18,
	))])
}

pub fn caves_tags_down_1_17_1(p: Tags, _cx: &mut Session) -> Result<Vec<Client17>> {
	Ok(vec![Client17::tags(tags(
		NEW,
		OLD,
		p,
		&[(LAVA_POOL.1, LAVA_POOL.0)],
		&[],
	))])
}

pub fn caves_statistics_1_18(p: Stats, _cx: &mut Session) -> Result<Vec<Client18>> {
	Ok(vec![Client18::statistics(statistics(OLD, NEW, p))])
}

pub fn caves_statistics_down_1_17_1(p: Stats, _cx: &mut Session) -> Result<Vec<Client17>> {
	Ok(vec![Client17::statistics(statistics(NEW, OLD, p))])
}

fn infiniburn(c: &mut [Entry], up: bool) {
	if let Some(Payload::string(s)) = c.field_mut("infiniburn").as_deref_mut() {
		match (up, s.strip_prefix('#')) {
			(true, None) => s.insert(0, '#'),
			(false, Some(rest)) => *s = rest.to_owned(),
			_ => {}
		}
	}
}

fn dimension_infiniburn(dimension: &mut Entry, up: bool) {
	if let Payload::compound(c) = &mut dimension.value {
		infiniburn(c, up);
	}
}

fn codec_infiniburn(codec: &mut Entry, up: bool) {
	for d in registry_entries(codec, "minecraft:dimension_type")
		.into_iter()
		.flatten()
	{
		if let Some(element) = d.compound_mut("element") {
			infiniburn(element, up);
		}
	}
}

pub fn caves_codec_1_18_2(
	src: &v1_18::play_toClient_packet_login::Ty,
	_cx: &mut Session,
) -> Result<Entry> {
	let mut codec = src.dimensionCodec.clone();
	codec_infiniburn(&mut codec, true);
	Ok(codec)
}

pub fn caves_codec_down_1_18(
	src: &v1_18::play_toClient_packet_login::Ty,
	_cx: &mut Session,
) -> Result<Entry> {
	let mut codec = src.dimensionCodec.clone();
	codec_infiniburn(&mut codec, false);
	Ok(codec)
}

fn tracked_dimension(cx: &mut Session, dimension: &Entry, up: bool) -> Result<Entry> {
	world(cx).track_dimension(dimension);
	let mut d = dimension.clone();
	dimension_infiniburn(&mut d, up);
	Ok(d)
}

pub fn caves_login_dimension_1_18_2(
	src: &v1_18::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	tracked_dimension(cx, &src.dimension, true)
}

pub fn caves_login_dimension_down_1_18(
	src: &v1_18::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	tracked_dimension(cx, &src.dimension, false)
}

pub fn caves_respawn_dimension_1_18_2(
	src: &v1_18::play_toClient_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	tracked_dimension(cx, &src.dimension, true)
}

pub fn caves_respawn_dimension_down_1_18(
	src: &v1_18::play_toClient_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Entry> {
	tracked_dimension(cx, &src.dimension, false)
}

const FALL_DAMAGE_RESETTING: &[&str] = &[
	"ladder",
	"vine",
	"scaffolding",
	"weeping_vines",
	"weeping_vines_plant",
	"twisting_vines",
	"twisting_vines_plant",
	"cave_vines",
	"cave_vines_plant",
	"sweet_berry_bush",
	"cobweb",
];

pub fn caves_tags_1_18_2(
	p: Tags,
	_cx: &mut Session,
) -> Result<Vec<v1_18_2::play_toClient_packet::Ty>> {
	let added = [(
		"minecraft:block",
		"minecraft:fall_damage_resetting",
		FALL_DAMAGE_RESETTING,
	)];
	Ok(vec![v1_18_2::play_toClient_packet::Ty::tags(tags(
		NEW,
		V::V1_18_2,
		p,
		&[],
		&added,
	))])
}
