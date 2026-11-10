use std::collections::BTreeMap;

use ::nbt::{CompoundExt, Entry, Payload};

use super::chunk::{self, F1_16, F1_16_2};
use super::meta::{self, Meta, Slot};
use super::tag_data::NEW_TAGS_1_16_2;
use super::*;
use crate::gen::convert::{v1_16 as t16, v1_16_1 as old, v1_16_2 as new};
use crate::pds::convert::Result;
use crate::session::Session;

const FROM: Version = Version::V1_16_1;
const TO: Version = Version::V1_16_2;

fn slot_up(v: Slot) -> Slot {
	meta::slot(v, FROM, TO, |_, _| {})
}

fn slot_down(v: Slot) -> Slot {
	meta::slot(v, TO, FROM, |_, _| {})
}

pub(super) fn dimension_types(codec: &Entry) -> &[Vec<Entry>] {
	codec
		.value
		.as_compound()
		.and_then(|c| c.field("minecraft:dimension_type"))
		.and_then(Payload::as_compound)
		.and_then(|c| c.field("value"))
		.map_or(&[], Payload::compounds)
}

fn dimension_element(codec: &Entry, name: &str) -> Option<Entry> {
	let types = dimension_types(codec);
	let find = |name: &str| {
		types
			.iter()
			.find(|t| t.field("name").and_then(Payload::as_str) == Some(name))
			.and_then(|t| t.field("element"))
			.cloned()
	};
	find(name)
		.or_else(|| find("minecraft:overworld"))
		.map(|e| Entry::new("", e))
}

fn dimension_name(dimension: &Entry) -> String {
	if let Ok(codec) = codec(TO) {
		if let Some(t) = dimension_types(&codec)
			.iter()
			.find(|t| t.field("element") == Some(&dimension.value))
		{
			if let Some(name) = t.field("name").and_then(Payload::as_str) {
				return name.to_owned();
			}
		}
	}
	let fields = dimension.value.as_compound().map_or(&[][..], Vec::as_slice);
	let ceiling = fields.field("has_ceiling").and_then(Payload::as_i64) == Some(1);
	match fields.field("effects").and_then(Payload::as_str) {
		Some("minecraft:the_nether") => "minecraft:the_nether",
		Some("minecraft:the_end") => "minecraft:the_end",
		_ if ceiling => "minecraft:overworld_caves",
		_ => "minecraft:overworld",
	}
	.to_owned()
}

fn element(name: &str) -> Result<Entry> {
	match dimension_element(&codec(TO)?, name) {
		Some(e) => Ok(e),
		None => unconvertible(format!("no 1.16.2 dimension type for {name}")),
	}
}

pub fn nether_slot_up_1_16_2(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot_up(v))
}

pub fn nether_slot_1_16_1(v: Slot, _cx: &mut Session) -> Result<Slot> {
	Ok(slot_down(v))
}

pub fn nether_login_up_1_16_2(
	v: old::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<new::play_toClient_packet_login::Ty> {
	player(cx, TO, v.entityId);
	Ok(new::play_toClient_packet_login::Ty {
		entityId: v.entityId,
		isHardcore: v.gameMode & 8 != 0,
		gameMode: v.gameMode & !8,
		previousGameMode: v.previousGameMode,
		worldNames: v.worldNames,
		dimensionCodec: codec(TO)?,
		dimension: element(&v.dimension)?,
		worldName: v.worldName,
		hashedSeed: v.hashedSeed,
		maxPlayers: v.maxPlayers as i32,
		viewDistance: v.viewDistance,
		reducedDebugInfo: v.reducedDebugInfo,
		enableRespawnScreen: v.enableRespawnScreen,
		isDebug: v.isDebug,
		isFlat: v.isFlat,
	})
}

#[derive(Default)]
struct Biomes(std::collections::HashMap<i32, i32>);

fn biome_ids(codec: &Entry) -> std::collections::HashMap<i32, i32> {
	let known = registry::names(FROM, "biomes");
	let entries = codec
		.value
		.as_compound()
		.and_then(|c| c.field("minecraft:worldgen/biome"))
		.and_then(Payload::as_compound)
		.and_then(|c| c.field("value"))
		.map_or(&[][..], Payload::compounds);
	entries
		.iter()
		.filter_map(|e| {
			let id = e.field("id").and_then(Payload::as_i64)? as i32;
			let name = e.field("name").and_then(Payload::as_str)?;
			Some((id, known.and_then(|k| k.id(name)).map_or(1, |v| v as i32)))
		})
		.collect()
}

pub fn nether_login_1_16_1(
	v: new::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<old::play_toClient_packet_login::Ty> {
	player(cx, TO, v.entityId);
	cx.state::<Biomes>().0 = biome_ids(&v.dimensionCodec);
	Ok(old::play_toClient_packet_login::Ty {
		entityId: v.entityId,
		gameMode: v.gameMode | if v.isHardcore { 8 } else { 0 },
		previousGameMode: v.previousGameMode,
		worldNames: v.worldNames,
		dimensionCodec: codec(FROM)?,
		dimension: dimension_name(&v.dimension),
		worldName: v.worldName,
		hashedSeed: v.hashedSeed,
		maxPlayers: v.maxPlayers.clamp(0, 255) as u8,
		viewDistance: v.viewDistance,
		reducedDebugInfo: v.reducedDebugInfo,
		enableRespawnScreen: v.enableRespawnScreen,
		isDebug: v.isDebug,
		isFlat: v.isFlat,
	})
}

pub fn nether_respawn_up_1_16_2(
	v: old::play_toClient_packet_respawn::Ty,
	_cx: &mut Session,
) -> Result<new::play_toClient_packet_respawn::Ty> {
	Ok(new::play_toClient_packet_respawn::Ty {
		dimension: element(&v.dimension)?,
		worldName: v.worldName,
		hashedSeed: v.hashedSeed,
		gamemode: v.gamemode,
		previousGamemode: v.previousGamemode,
		isDebug: v.isDebug,
		isFlat: v.isFlat,
		copyMetadata: v.copyMetadata,
	})
}

pub fn nether_respawn_1_16_1(
	v: new::play_toClient_packet_respawn::Ty,
	_cx: &mut Session,
) -> Result<old::play_toClient_packet_respawn::Ty> {
	Ok(old::play_toClient_packet_respawn::Ty {
		dimension: dimension_name(&v.dimension),
		worldName: v.worldName,
		hashedSeed: v.hashedSeed,
		gamemode: v.gamemode,
		previousGamemode: v.previousGamemode,
		isDebug: v.isDebug,
		isFlat: v.isFlat,
		copyMetadata: v.copyMetadata,
	})
}

pub fn nether_map_chunk_up_1_16_2(
	v: old::play_toClient_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<new::play_toClient_packet_map_chunk::Ty> {
	use new::play_toClient_packet_map_chunk__biomes::Ty as B;
	let present = (v.bitMap as u32).count_ones() as usize;
	Ok(new::play_toClient_packet_map_chunk::Ty {
		x: v.x,
		z: v.z,
		bitMap: v.bitMap,
		heightmaps: v.heightmaps,
		biomes: match v.biomes {
			old::play_toClient_packet_map_chunk__biomes::Ty::r#true(b) => B::r#true(b),
			old::play_toClient_packet_map_chunk__biomes::Ty::r#false => B::r#false,
		},
		chunkData: chunk::convert_sections(&v.chunkData, present, F1_16, F1_16_2)?,
		blockEntities: v.blockEntities,
	})
}

pub fn nether_biomes_1_16_1(cx: &mut Session, biomes: Vec<i32>) -> Vec<i32> {
	let ids = &cx.state::<Biomes>().0;
	biomes
		.into_iter()
		.map(|id| ids.get(&id).copied().unwrap_or(id))
		.collect()
}

pub fn nether_map_chunk_1_16_1(
	v: new::play_toClient_packet_map_chunk::Ty,
	cx: &mut Session,
) -> Result<old::play_toClient_packet_map_chunk::Ty> {
	use old::play_toClient_packet_map_chunk__biomes::Ty as B;
	let present = (v.bitMap as u32).count_ones() as usize;
	let full = matches!(
		v.biomes,
		new::play_toClient_packet_map_chunk__biomes::Ty::r#true(_)
	);
	Ok(old::play_toClient_packet_map_chunk::Ty {
		x: v.x,
		z: v.z,
		ignoreOldData: full,
		bitMap: v.bitMap,
		heightmaps: v.heightmaps,
		biomes: match v.biomes {
			new::play_toClient_packet_map_chunk__biomes::Ty::r#true(b) => {
				B::r#true(nether_biomes_1_16_1(cx, b))
			}
			new::play_toClient_packet_map_chunk__biomes::Ty::r#false => B::r#false,
		},
		chunkData: chunk::convert_sections(&v.chunkData, present, F1_16_2, F1_16)?,
		blockEntities: v.blockEntities,
	})
}

pub fn nether_block_change_up_1_16_2(v: BlockChange, _cx: &mut Session) -> Result<BlockChange> {
	Ok(block_change(v, FROM, TO))
}

pub fn nether_block_change_1_16_1(v: BlockChange, _cx: &mut Session) -> Result<BlockChange> {
	Ok(block_change(v, TO, FROM))
}

pub fn nether_acknowledge_player_digging_up_1_16_2(
	v: Digging,
	_cx: &mut Session,
) -> Result<Digging> {
	Ok(digging(v, FROM, TO))
}

pub fn nether_acknowledge_player_digging_1_16_1(v: Digging, _cx: &mut Session) -> Result<Digging> {
	Ok(digging(v, TO, FROM))
}

type Particles = t16::play_toClient_packet_world_particles::Ty;

fn particles(
	mut v: Particles,
	from: Version,
	to: Version,
	slot: fn(Slot) -> Slot,
) -> Result<Particles> {
	let p = kept(meta::particle(
		meta::particle_16(v.particleId, v.data),
		from,
		to,
		slot,
	))?;
	v.particleId = p.id;
	v.data = kept(meta::data_16(p))?;
	Ok(v)
}

pub fn nether_world_particles_up_1_16_2(v: Particles, _cx: &mut Session) -> Result<Particles> {
	particles(v, FROM, TO, slot_up)
}

pub fn nether_world_particles_1_16_1(v: Particles, _cx: &mut Session) -> Result<Particles> {
	particles(v, TO, FROM, slot_down)
}

pub(super) fn record(state: i32, x: i32, y: i32, z: i32) -> i64 {
	((state as i64) << 12) | ((x as i64 & 15) << 8) | ((z as i64 & 15) << 4) | (y as i64 & 15)
}

pub fn nether_multi_block_change_up_1_16_2(
	p: old::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<new::play_toClient_packet::Ty>> {
	let mut sections: BTreeMap<u8, Vec<i64>> = BTreeMap::new();
	for r in p.records {
		let (x, z) = ((r.horizontalPos >> 4) as i32, (r.horizontalPos & 15) as i32);
		sections.entry(r.y >> 4).or_default().push(record(
			state(FROM, TO, r.blockId),
			x,
			r.y as i32,
			z,
		));
	}
	Ok(sections
		.into_iter()
		.map(|(y, records)| {
			new::play_toClient_packet::Ty::multi_block_change(
				new::play_toClient_packet_multi_block_change::Ty {
					chunkCoordinates:
						new::play_toClient_packet_multi_block_change__chunkCoordinates::Ty {
							x: p.chunkX,
							z: p.chunkZ,
							y: y as u32,
						},
					notTrustEdges: false,
					records,
				},
			)
		})
		.collect())
}

pub(super) fn section_y(y: u32) -> i32 {
	((y << 12) as i32) >> 12
}

pub fn nether_multi_block_change_1_16_1(
	p: new::play_toClient_packet_multi_block_change::Ty,
	_cx: &mut Session,
) -> Result<Vec<old::play_toClient_packet::Ty>> {
	let base = section_y(p.chunkCoordinates.y) * 16;
	let records: Vec<_> = p
		.records
		.into_iter()
		.filter_map(|r| {
			let y = base + (r & 15) as i32;
			(0..256).contains(&y).then(|| {
				old::play_toClient_packet_multi_block_change__records__items::Ty {
					horizontalPos: ((((r >> 8) & 15) << 4) | ((r >> 4) & 15)) as u8,
					y: y as u8,
					blockId: state(TO, FROM, (r >> 12) as i32),
				}
			})
		})
		.collect();
	if records.is_empty() {
		return Ok(vec![]);
	}
	Ok(vec![old::play_toClient_packet::Ty::multi_block_change(
		old::play_toClient_packet_multi_block_change::Ty {
			chunkX: p.chunkCoordinates.x,
			chunkZ: p.chunkCoordinates.z,
			records,
		},
	)])
}

pub fn nether_statistics_up_1_16_2(
	p: Vec<Stat>,
	_cx: &mut Session,
) -> Result<Vec<new::play_toClient_packet::Ty>> {
	Ok(vec![new::play_toClient_packet::Ty::statistics(statistics(
		p, FROM, TO,
	))])
}

pub fn nether_statistics_1_16_1(
	p: Vec<Stat>,
	_cx: &mut Session,
) -> Result<Vec<old::play_toClient_packet::Ty>> {
	Ok(vec![old::play_toClient_packet::Ty::statistics(statistics(
		p, TO, FROM,
	))])
}

pub fn nether_spawn_entity_up_1_16_2(
	p: SpawnObject,
	cx: &mut Session,
) -> Result<Vec<new::play_toClient_packet::Ty>> {
	Ok(spawn_object(p, FROM, TO, cx)
		.map(new::play_toClient_packet::Ty::spawn_entity)
		.into_iter()
		.collect())
}

pub fn nether_spawn_entity_1_16_1(
	p: SpawnObject,
	cx: &mut Session,
) -> Result<Vec<old::play_toClient_packet::Ty>> {
	Ok(spawn_object(p, TO, FROM, cx)
		.map(old::play_toClient_packet::Ty::spawn_entity)
		.into_iter()
		.collect())
}

pub fn nether_spawn_entity_living_up_1_16_2(p: SpawnMob, cx: &mut Session) -> Result<SpawnMob> {
	spawn_mob(p, FROM, TO, cx)
}

pub fn nether_spawn_entity_living_1_16_1(p: SpawnMob, cx: &mut Session) -> Result<SpawnMob> {
	spawn_mob(p, TO, FROM, cx)
}

fn piglin_swap(items: &mut [(u8, Meta)], kind: &str) {
	if matches!(kind, "piglin" | "piglin_brute") {
		for (k, _) in items.iter_mut() {
			*k = match *k {
				15 => 16,
				16 => 15,
				k => k,
			};
		}
	}
}

type Metadata = t16::play_toClient_packet_entity_metadata::Ty;

pub fn nether_entity_metadata_up_1_16_2(v: Metadata, cx: &mut Session) -> Result<Metadata> {
	let kind = kind_of(cx, TO, v.entityId).name;
	let mut items = meta::remap_values(meta::decode_16(v.metadata), FROM, TO, &mut slot_up);
	meta::minecart_block(&mut items, kind, 10, FROM, TO);
	piglin_swap(&mut items, kind);
	Ok(Metadata {
		entityId: v.entityId,
		metadata: meta::encode_16(items),
	})
}

pub fn nether_entity_metadata_1_16_1(v: Metadata, cx: &mut Session) -> Result<Metadata> {
	let tracked = kind_of(cx, TO, v.entityId);
	let kind = tracked.name;
	let mut items = meta::remap_values(meta::decode_16(v.metadata), TO, FROM, &mut slot_down);
	meta::minecart_block(&mut items, kind, 10, TO, FROM);
	piglin_swap(&mut items, kind);
	meta::substituted(&mut items, tracked);
	Ok(Metadata {
		entityId: v.entityId,
		metadata: meta::encode_16(items),
	})
}

pub fn nether_world_event_up_1_16_2(v: WorldEvent, _cx: &mut Session) -> Result<WorldEvent> {
	world_event(v, FROM, TO)
}

pub fn nether_world_event_1_16_1(v: WorldEvent, _cx: &mut Session) -> Result<WorldEvent> {
	world_event(v, TO, FROM)
}

pub fn nether_sound_effect_up_1_16_2(v: Sound, _cx: &mut Session) -> Result<Sound> {
	sound_effect(v, FROM, TO)
}

pub fn nether_sound_effect_1_16_1(v: Sound, _cx: &mut Session) -> Result<Sound> {
	sound_effect(v, TO, FROM)
}

pub fn nether_entity_sound_effect_up_1_16_2(
	v: EntitySound,
	_cx: &mut Session,
) -> Result<EntitySound> {
	entity_sound(v, FROM, TO)
}

pub fn nether_entity_sound_effect_1_16_1(v: EntitySound, _cx: &mut Session) -> Result<EntitySound> {
	entity_sound(v, TO, FROM)
}

pub fn nether_set_cooldown_up_1_16_2(v: Cooldown, _cx: &mut Session) -> Result<Cooldown> {
	cooldown(v, FROM, TO)
}

pub fn nether_set_cooldown_1_16_1(v: Cooldown, _cx: &mut Session) -> Result<Cooldown> {
	cooldown(v, TO, FROM)
}

pub fn nether_tags_up_1_16_2(v: Tags, _cx: &mut Session) -> Result<Tags> {
	Ok(all_tags(
		v,
		FROM,
		TO,
		NEW_TAGS_1_16_2,
		&["furnace_materials"],
	))
}

pub fn nether_tags_1_16_1(v: Tags, _cx: &mut Session) -> Result<Tags> {
	Ok(all_tags(v, TO, FROM, &[], &[]))
}

#[derive(Default)]
struct Held(i16);

pub fn nether_client_held_item_slot_up_1_16_2(
	slot: i8,
	cx: &mut Session,
) -> Result<Vec<new::play_toClient_packet::Ty>> {
	cx.state::<Held>().0 = slot as i16;
	Ok(vec![new::play_toClient_packet::Ty::held_item_slot(slot)])
}

pub fn nether_client_held_item_slot_1_16_1(
	slot: i8,
	cx: &mut Session,
) -> Result<Vec<old::play_toClient_packet::Ty>> {
	cx.state::<Held>().0 = slot as i16;
	Ok(vec![old::play_toClient_packet::Ty::held_item_slot(slot)])
}

pub fn nether_server_held_item_slot_up_1_16_2(
	slot: i16,
	cx: &mut Session,
) -> Result<Vec<new::play_toServer_packet::Ty>> {
	cx.state::<Held>().0 = slot;
	Ok(vec![new::play_toServer_packet::Ty::held_item_slot(slot)])
}

pub fn nether_server_held_item_slot_1_16_1(
	slot: i16,
	cx: &mut Session,
) -> Result<Vec<old::play_toServer_packet::Ty>> {
	cx.state::<Held>().0 = slot;
	Ok(vec![old::play_toServer_packet::Ty::held_item_slot(slot)])
}

type EditBook = t16::play_toServer_packet_edit_book::Ty;

pub fn nether_edit_book_up_1_16_2(v: EditBook, cx: &mut Session) -> Result<EditBook> {
	Ok(EditBook {
		new_book: slot_up(v.new_book),
		signing: v.signing,
		hand: if v.hand == 1 {
			40
		} else {
			cx.state::<Held>().0 as i32
		},
	})
}

pub fn nether_edit_book_1_16_1(v: EditBook, _cx: &mut Session) -> Result<EditBook> {
	Ok(EditBook {
		new_book: slot_down(v.new_book),
		signing: v.signing,
		hand: (v.hand == 40) as i32,
	})
}

pub fn nether_crafting_book_data_up_1_16_2(
	p: old::play_toServer_packet_crafting_book_data::Ty,
	_cx: &mut Session,
) -> Result<Vec<new::play_toServer_packet::Ty>> {
	use old::play_toServer_packet_crafting_book_data::Ty as D;
	Ok(match p {
		D::v0(recipe) => vec![new::play_toServer_packet::Ty::displayed_recipe(recipe)],
		D::v1(s) => [
			(s.craftingBookOpen, s.craftingFilter),
			(s.smeltingBookOpen, s.smeltingFilter),
			(s.blastingBookOpen, s.blastingFilter),
			(s.smokingBookOpen, s.smokingFilter),
		]
		.into_iter()
		.enumerate()
		.map(|(i, (bookOpen, filterActive))| {
			new::play_toServer_packet::Ty::recipe_book(new::play_toServer_packet_recipe_book::Ty {
				bookId: i as i32,
				bookOpen,
				filterActive,
			})
		})
		.collect(),
		D::default(_) => vec![],
	})
}

pub fn nether_displayed_recipe_1_16_1(
	recipe: String,
	_cx: &mut Session,
) -> Result<Vec<old::play_toServer_packet::Ty>> {
	Ok(vec![old::play_toServer_packet::Ty::crafting_book_data(
		old::play_toServer_packet_crafting_book_data::Ty::v0(recipe),
	)])
}

pub fn nether_recipe_book_1_16_1(
	p: new::play_toServer_packet_recipe_book::Ty,
	_cx: &mut Session,
) -> Result<Vec<old::play_toServer_packet::Ty>> {
	let open = |i| p.bookId == i && p.bookOpen;
	Ok(vec![old::play_toServer_packet::Ty::crafting_book_data(
		old::play_toServer_packet_crafting_book_data::Ty::v1(
			old::play_toServer_packet_crafting_book_data__v1::Ty {
				craftingBookOpen: open(0),
				craftingFilter: p.filterActive,
				smeltingBookOpen: open(1),
				smeltingFilter: p.filterActive,
				blastingBookOpen: open(2),
				blastingFilter: p.filterActive,
				smokingBookOpen: open(3),
				smokingFilter: p.filterActive,
			},
		),
	)])
}
