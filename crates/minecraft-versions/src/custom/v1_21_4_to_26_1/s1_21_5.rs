use super::items5::{components_down, components_up, slot_down, slot_up, slots_up};
use super::remap::{self, remaps};
use super::rules::{blocks, statistics, tags};
use super::shared;
use super::util::{encode, recode};
use super::{chunk, hash, text, world};
use crate::gen::convert::{v1_21_4 as a, v1_21_5 as b, Version};
use crate::pds::convert::{Error, Result};
use crate::Session;
use nbt::{Entry, List, ListItems, Payload, Tag};
use std::collections::HashMap;

const OLD: Version = Version::V1_21_4;
const NEW: Version = Version::V1_21_5;

remaps!(Version::V1_21_4 => Version::V1_21_5 { late_sound_up_1_21_5: a::ItemSoundHolder::Ty = remap::sound; });
remaps!(Version::V1_21_5 => Version::V1_21_4 { late_sound_down_1_21_4: a::ItemSoundHolder::Ty = remap::sound; });

blocks!(a => b, Version::V1_21_4 => Version::V1_21_5 {
	block_change: late_block_change_up_1_21_5, multi_block: late_multi_block_up_1_21_5, block_action: late_block_action_up_1_21_5,
	world_event: late_world_event_up_1_21_5, chunk: late_chunk_data_up_1_21_5, tool: late_tool_up_1_21_5, predicate: late_predicate_up_1_21_5,
	particle: [late_particle_block_up_1_21_5, late_particle_marker_up_1_21_5, late_particle_dust_up_1_21_5, late_particle_pillar_up_1_21_5, late_particle_crumble_up_1_21_5],
	meta: [late_meta_state_up_1_21_5, late_meta_optional_state_up_1_21_5],
});
blocks!(b => a, Version::V1_21_5 => Version::V1_21_4 {
	block_change: late_block_change_down_1_21_4, multi_block: late_multi_block_down_1_21_4, block_action: late_block_action_down_1_21_4,
	world_event: late_world_event_down_1_21_4, chunk: late_chunk_data_down_1_21_4, tool: late_tool_down_1_21_4, predicate: late_predicate_down_1_21_4,
	particle: [late_particle_block_down_1_21_4, late_particle_marker_down_1_21_4, late_particle_dust_down_1_21_4, late_particle_pillar_down_1_21_4, late_particle_crumble_down_1_21_4],
	meta: [late_meta_state_down_1_21_4, late_meta_optional_state_down_1_21_4],
});

statistics!(a => b, Version::V1_21_4 => Version::V1_21_5 { late_statistics_up_1_21_5 });
statistics!(b => a, Version::V1_21_5 => Version::V1_21_4 { late_statistics_down_1_21_4 });

const TAG_RENAMES_UP: &[(&str, &str, &str)] = &[(
	"block",
	"dead_bush_may_place_on",
	"dry_vegetation_may_place_on",
)];
const TAG_RENAMES_DOWN: &[(&str, &str, &str)] = &[(
	"block",
	"dry_vegetation_may_place_on",
	"dead_bush_may_place_on",
)];

tags!(a => b, Version::V1_21_4 => Version::V1_21_5, TAG_RENAMES_UP, { config: late_config_tags_up_1_21_5, play: late_play_tags_up_1_21_5 });
tags!(b => a, Version::V1_21_5 => Version::V1_21_4, TAG_RENAMES_DOWN, { config: late_config_tags_down_1_21_4, play: late_play_tags_down_1_21_4 });

pub fn late_slot_up_1_21_5(v: a::Slot::Ty, _cx: &mut Session) -> Result<b::Slot::Ty> {
	slot_up(v)
}

#[derive(Default)]
struct Sent(HashMap<Vec<u8>, b::Slot__default::Ty>);

pub fn late_slot_down_1_21_4(v: b::Slot::Ty, cx: &mut Session) -> Result<a::Slot::Ty> {
	let down = slot_down(v.clone())?;
	if let (b::Slot::Ty::default(_, original), a::Slot::Ty::default(_, d)) = (v, &down) {
		if original.components.is_empty() && original.removeComponents.is_empty() {
			return Ok(down);
		}
		let key = encode(d, a::Slot__default::serialize)?;
		let sent = &mut cx.state::<Sent>().0;
		if sent.len() >= 4096 {
			sent.clear();
		}
		sent.insert(key, original);
	}
	Ok(down)
}

pub fn late_display_item_up_1_21_5(
	v: i32,
	_cx: &mut Session,
) -> Result<b::play_toClient_SlotDisplay::Ty> {
	Ok(b::play_toClient_SlotDisplay::Ty::item(remap::item(
		OLD, NEW, v,
	)?))
}

pub fn late_display_item_down_1_21_4(
	v: i32,
	_cx: &mut Session,
) -> Result<a::play_toClient_SlotDisplay::Ty> {
	Ok(a::play_toClient_SlotDisplay::Ty::item(remap::item(
		NEW, OLD, v,
	)?))
}

pub fn late_trim_pattern_up_1_21_5(
	_v: &a::play_toClient_SlotDisplay__smithing_trim::Ty,
	_cx: &mut Session,
) -> Result<b::play_toClient_SlotDisplay__smithing_trim__pattern::Ty> {
	Ok(b::play_toClient_SlotDisplay__smithing_trim__pattern::Ty::Id(0))
}

pub fn late_trim_pattern_down_1_21_4(
	_v: &b::play_toClient_SlotDisplay__smithing_trim::Ty,
	_cx: &mut Session,
) -> Result<a::play_toClient_SlotDisplay::Ty> {
	Ok(a::play_toClient_SlotDisplay::Ty::empty)
}

pub fn late_trade_input1_up_1_21_5(
	v: &a::play_toClient_packet_trade_list__trades__items::Ty,
	_cx: &mut Session,
) -> Result<b::play_toClient_packet_trade_list__trades__items__inputItem1::Ty> {
	let i = &v.inputItem1;
	Ok(
		b::play_toClient_packet_trade_list__trades__items__inputItem1::Ty {
			itemId: remap::item(OLD, NEW, i.itemId)?,
			itemCount: i.itemCount,
			components: components_up(i.components.clone())?,
		},
	)
}

pub fn late_trade_input2_up_1_21_5(
	v: &a::play_toClient_packet_trade_list__trades__items::Ty,
	_cx: &mut Session,
) -> Result<Option<b::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty>> {
	v.inputItem2
		.as_ref()
		.map(|i| {
			Ok(
				b::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty {
					itemId: remap::item(OLD, NEW, i.itemId)?,
					itemCount: i.itemCount,
					components: components_up(i.components.clone())?,
				},
			)
		})
		.transpose()
}

pub fn late_trade_input1_down_1_21_4(
	v: &b::play_toClient_packet_trade_list__trades__items::Ty,
	_cx: &mut Session,
) -> Result<a::play_toClient_packet_trade_list__trades__items__inputItem1::Ty> {
	let i = &v.inputItem1;
	let components = components_down(i.components.clone())?;
	Ok(
		a::play_toClient_packet_trade_list__trades__items__inputItem1::Ty {
			itemId: remap::item(NEW, OLD, i.itemId)?,
			itemCount: i.itemCount,
			addedComponentCount: components.len() as i32,
			components,
		},
	)
}

pub fn late_trade_input2_down_1_21_4(
	v: &b::play_toClient_packet_trade_list__trades__items::Ty,
	_cx: &mut Session,
) -> Result<Option<a::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty>> {
	v.inputItem2
		.as_ref()
		.map(|i| {
			let components = components_down(i.components.clone())?;
			Ok(
				a::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty {
					itemId: remap::item(NEW, OLD, i.itemId)?,
					itemCount: i.itemCount,
					addedComponentCount: components.len() as i32,
					components,
				},
			)
		})
		.transpose()
}

pub fn late_recipe_items_up_1_21_5(
	v: &a::play_toClient_packet_declare_recipes__recipes__items::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	remap::items(OLD, NEW, v.items.clone())
}

pub fn late_recipe_items_down_1_21_4(
	v: &b::play_toClient_packet_declare_recipes__recipes__items::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	remap::items(NEW, OLD, v.items.clone())
}

pub fn late_stonecutter_up_1_21_5(
	v: &a::play_toClient_packet_declare_recipes__stoneCutterRecipes__items::Ty,
	_cx: &mut Session,
) -> Result<b::IDSet::Ty> {
	remap::item_set(OLD, NEW, v.input.clone())
}

pub fn late_stonecutter_down_1_21_4(
	v: &b::play_toClient_packet_declare_recipes__stoneCutterRecipes__items::Ty,
	_cx: &mut Session,
) -> Result<a::IDSet::Ty> {
	remap::item_set(NEW, OLD, v.input.clone())
}

pub fn late_requirements_up_1_21_5(
	v: &a::play_toClient_packet_recipe_book_add__entries__items__recipe::Ty,
	_cx: &mut Session,
) -> Result<Option<Vec<b::IDSet::Ty>>> {
	v.craftingRequirements
		.clone()
		.map(|s| {
			s.into_iter()
				.map(|s| remap::item_set(OLD, NEW, s))
				.collect()
		})
		.transpose()
}

pub fn late_requirements_down_1_21_4(
	v: &b::play_toClient_packet_recipe_book_add__entries__items__recipe::Ty,
	_cx: &mut Session,
) -> Result<Option<Vec<a::IDSet::Ty>>> {
	v.craftingRequirements
		.clone()
		.map(|s| {
			s.into_iter()
				.map(|s| remap::item_set(NEW, OLD, s))
				.collect()
		})
		.transpose()
}

pub fn late_spawn_type_up_1_21_5(
	v: &a::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::spawned(cx, v.entityId, OLD, v.r#type);
	remap::entity(OLD, NEW, v.r#type)
}

pub fn late_object_data_up_1_21_5(
	v: &a::play_toClient_packet_spawn_entity::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	match crate::registry::names(OLD, "entities").and_then(|n| n.name(v.r#type.max(0) as u32)) {
		Some("falling_block") => remap::state(OLD, NEW, v.objectData),
		_ => Ok(v.objectData),
	}
}

pub fn late_experience_orb_up_1_21_5(
	p: a::play_toClient_packet_spawn_entity_experience_orb::Ty,
	cx: &mut Session,
) -> Result<Vec<b::play_toClient_packet::Ty>> {
	let r#type = crate::registry::names(NEW, "entities")
		.and_then(|n| n.id("experience_orb"))
		.unwrap_or(0) as i32;
	world::spawned(cx, p.entityId, NEW, r#type);
	let uuid = 0x7064_7332_0000_4000_8000_0000_0000_0000u128
		| (p.entityId as u32 as u128)
		| 0x0001_0000_0000u128;
	Ok(vec![b::play_toClient_packet::Ty::spawn_entity(
		b::play_toClient_packet_spawn_entity::Ty {
			entityId: p.entityId,
			objectUUID: uuid,
			r#type,
			x: p.x,
			y: p.y,
			z: p.z,
			pitch: 0,
			yaw: 0,
			headPitch: 0,
			objectData: p.count as i32,
			velocity: b::vec3i16::Ty { x: 0, y: 0, z: 0 },
		},
	)])
}

const HEIGHTMAPS: &[&str] = &[
	"WORLD_SURFACE_WG",
	"WORLD_SURFACE",
	"OCEAN_FLOOR_WG",
	"OCEAN_FLOOR",
	"MOTION_BLOCKING",
	"MOTION_BLOCKING_NO_LEAVES",
];

pub fn late_heightmap_type_1_21_5(name: &str) -> Option<i32> {
	HEIGHTMAPS.iter().position(|n| *n == name).map(|i| i as i32)
}

pub fn late_heightmap_name_1_21_4(id: i32) -> Option<&'static str> {
	HEIGHTMAPS.get(id as usize).copied()
}

pub fn late_heightmaps_up_1_21_5(
	v: &a::play_toClient_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_toClient_packet_map_chunk__heightmaps__items::Ty>> {
	let Payload::compound(entries) = &v.heightmaps.value else {
		return Ok(Vec::new());
	};
	Ok(entries
		.iter()
		.filter_map(|e| {
			let r#type = late_heightmap_type_1_21_5(&e.name)?;
			match &e.value {
				Payload::longArray(data) => {
					Some(b::play_toClient_packet_map_chunk__heightmaps__items::Ty {
						r#type,
						data: data.clone(),
					})
				}
				_ => None,
			}
		})
		.collect())
}

pub fn late_heightmaps_down_1_21_4(
	v: &b::play_toClient_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<nbt::Tag> {
	Ok(Tag::new(Payload::compound(
		v.heightmaps
			.iter()
			.filter_map(|h| {
				Some(Entry::new(
					late_heightmap_name_1_21_4(h.r#type)?,
					Payload::longArray(h.data.clone()),
				))
			})
			.collect(),
	)))
}

fn sign_text(side: &mut Payload) {
	for field in ["messages", "filtered_messages"] {
		if let Some(Payload::list(nbt::List {
			items: ListItems::string(lines),
			..
		})) = side.field(field)
		{
			let converted = lines
				.iter()
				.map(|l| match text::from_json(l) {
					Payload::compound(c) => c,
					_ => Vec::new(),
				})
				.collect();
			side.put(field, List::new(ListItems::compound(converted)));
		}
	}
}

fn block_entity(tag: Option<nbt::Tag>) -> Option<nbt::Tag> {
	let mut tag = tag?;
	for side in ["front_text", "back_text"] {
		if let Some(mut s) = tag.value.take(side) {
			sign_text(&mut s);
			tag.value.put(side, s);
		}
	}
	if let Some(Payload::string(name)) = tag.value.field("CustomName").cloned() {
		tag.value.put("CustomName", text::from_json(&name));
	}
	Some(Tag::new(tag.value))
}

pub fn late_chunk_block_entity_up_1_21_5(
	v: &a::chunkBlockEntity::Ty,
	_cx: &mut Session,
) -> Result<Option<nbt::Tag>> {
	Ok(block_entity(v.nbtData.clone()))
}

pub fn late_tile_entity_up_1_21_5(
	v: &a::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<Option<nbt::Tag>> {
	Ok(block_entity(v.nbtData.clone()))
}

pub fn late_chunk_biomes_up_1_21_5(
	v: Vec<a::play_toClient_packet__chunk_biomes__items::Ty>,
	cx: &mut Session,
) -> Result<Vec<b::play_toClient_packet::Ty>> {
	let biomes = world::biomes(cx);
	let items = v
		.into_iter()
		.map(|c| {
			Ok(b::play_toClient_packet__chunk_biomes__items::Ty {
				data: chunk::biomes_step(OLD, NEW, &c.data, biomes)?,
				position: c.position,
			})
		})
		.collect::<Result<_>>()?;
	Ok(vec![b::play_toClient_packet::Ty::chunk_biomes(items)])
}

#[derive(Default)]
struct Chat {
	index: i32,
}

pub fn late_login_up_1_21_5(
	p: &a::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<i32> {
	cx.state::<Chat>().index = 0;
	world::world(cx).player = Some(p.entityId);
	Ok(p.entityId)
}

pub fn late_login_down_1_21_4(
	p: &b::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<i32> {
	world::world(cx).player = Some(p.entityId);
	Ok(p.entityId)
}

pub fn late_chat_index_up_1_21_5(
	_p: &a::play_toClient_packet_player_chat::Ty,
	cx: &mut Session,
) -> Result<i32> {
	let chat = cx.state::<Chat>();
	let index = chat.index;
	chat.index += 1;
	Ok(index)
}

fn visibility(s: &str) -> i32 {
	match s {
		"never" => 1,
		"hideForOtherTeams" => 2,
		"hideForOwnTeam" => 3,
		_ => 0,
	}
}

fn collision(s: &str) -> i32 {
	match s {
		"never" => 1,
		"pushOtherTeams" => 2,
		"pushOwnTeam" => 3,
		_ => 0,
	}
}

const VISIBILITY: &[&str] = &["always", "never", "hideForOtherTeams", "hideForOwnTeam"];
const COLLISION: &[&str] = &["always", "never", "pushOtherTeams", "pushOwnTeam"];

pub fn late_teams_up_1_21_5(
	v: a::play_toClient_packet_teams::Ty,
	_cx: &mut Session,
) -> Result<b::play_toClient_packet_teams::Ty> {
	use a::play_toClient_packet_teams__mode::Ty as A;
	use b::play_toClient_packet_teams__mode::Ty as B;
	let mode = match v.mode {
		A::v0(t) => B::v0(b::play_toClient_packet_teams__mode__v0::Ty {
			name: text::up(t.name),
			friendlyFire: t.friendlyFire,
			nameTagVisibility: visibility(&t.nameTagVisibility),
			collisionRule: collision(&t.collisionRule),
			formatting: t.formatting,
			prefix: text::up(t.prefix),
			suffix: text::up(t.suffix),
			players: t.players,
		}),
		A::v2(t) => B::v2(b::play_toClient_packet_teams__mode__v2::Ty {
			name: text::up(t.name),
			friendlyFire: t.friendlyFire,
			nameTagVisibility: visibility(&t.nameTagVisibility),
			collisionRule: collision(&t.collisionRule),
			formatting: t.formatting,
			prefix: text::up(t.prefix),
			suffix: text::up(t.suffix),
		}),
		other => recode(
			&other,
			a::play_toClient_packet_teams__mode::serialize,
			b::play_toClient_packet_teams__mode::deserialize,
		)?,
	};
	Ok(b::play_toClient_packet_teams::Ty { team: v.team, mode })
}

pub fn late_teams_down_1_21_4(
	v: b::play_toClient_packet_teams::Ty,
	_cx: &mut Session,
) -> Result<a::play_toClient_packet_teams::Ty> {
	use a::play_toClient_packet_teams__mode::Ty as A;
	use b::play_toClient_packet_teams__mode::Ty as B;
	let name = |list: &[&str], i: i32| {
		list.get(i.max(0) as usize)
			.copied()
			.unwrap_or("always")
			.to_owned()
	};
	let mode = match v.mode {
		B::v0(t) => A::v0(a::play_toClient_packet_teams__mode__v0::Ty {
			name: text::down(t.name),
			friendlyFire: t.friendlyFire,
			nameTagVisibility: name(VISIBILITY, t.nameTagVisibility),
			collisionRule: name(COLLISION, t.collisionRule),
			formatting: t.formatting,
			prefix: text::down(t.prefix),
			suffix: text::down(t.suffix),
			players: t.players,
		}),
		B::v2(t) => A::v2(a::play_toClient_packet_teams__mode__v2::Ty {
			name: text::down(t.name),
			friendlyFire: t.friendlyFire,
			nameTagVisibility: name(VISIBILITY, t.nameTagVisibility),
			collisionRule: name(COLLISION, t.collisionRule),
			formatting: t.formatting,
			prefix: text::down(t.prefix),
			suffix: text::down(t.suffix),
		}),
		other => recode(
			&other,
			b::play_toClient_packet_teams__mode::serialize,
			a::play_toClient_packet_teams__mode::deserialize,
		)?,
	};
	Ok(a::play_toClient_packet_teams::Ty { team: v.team, mode })
}

pub fn late_entity_status_up_1_21_5(
	p: a::play_toClient_packet_entity_status::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_toClient_packet::Ty>> {
	let (sound, volume) = match p.entityStatus {
		29 => ("item.shield.block", 1.0),
		30 => ("item.shield.break", 0.8),
		_ => return Ok(vec![b::play_toClient_packet::Ty::entity_status(p)]),
	};
	let Some(id) = remap::sound_id(NEW, sound) else {
		return Ok(vec![]);
	};
	let seed = (p.entityId as i64).wrapping_mul(0x5DEECE66D) ^ 0x2545F491;
	Ok(vec![b::play_toClient_packet::Ty::entity_sound_effect(
		b::play_toClient_packet_entity_sound_effect::Ty {
			sound: b::ItemSoundHolder::Ty::Id(id),
			soundCategory: recode(
				&7,
				crate::pds_native::pds_varint::serialize,
				b::soundSource::deserialize,
			)?,
			entityId: p.entityId,
			volume,
			pitch: 0.8 + ((seed >> 8) & 0xff) as f32 / 255.0 * 0.4,
			seed,
		},
	)])
}

pub fn late_effect_flags_up_1_21_5(
	p: &a::play_toClient_packet_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<u8> {
	Ok(if p.effectId == 8 {
		p.flags | 0x08
	} else {
		p.flags
	})
}

pub(super) type Text = fn(nbt::Tag) -> nbt::Tag;

pub fn late_text_up_1_21_5(v: nbt::Tag, _cx: &mut Session) -> Result<nbt::Tag> {
	Ok(text::up(v))
}

pub fn late_text_down_1_21_4(v: nbt::Tag, _cx: &mut Session) -> Result<nbt::Tag> {
	Ok(text::down(v))
}

macro_rules! text_arms {
	($($arm:ident: $up:ident, $down:ident;)*) => {
		$(
			pub fn $up(v: nbt::Tag, _cx: &mut Session) -> Result<Vec<b::play_toClient_packet::Ty>> {
				Ok(vec![b::play_toClient_packet::Ty::$arm(text::up(v))])
			}

			pub fn $down(v: nbt::Tag, _cx: &mut Session) -> Result<Vec<a::play_toClient_packet::Ty>> {
				Ok(vec![a::play_toClient_packet::Ty::$arm(text::down(v))])
			}
		)*
	};
}

text_arms! {
	action_bar: late_action_bar_up_1_21_5, late_action_bar_down_1_21_4;
	set_title_text: late_title_up_1_21_5, late_title_down_1_21_4;
	set_title_subtitle: late_subtitle_up_1_21_5, late_subtitle_down_1_21_4;
	kick_disconnect: late_kick_up_1_21_5, late_kick_down_1_21_4;
}

pub fn late_config_disconnect_up_1_21_5(
	v: nbt::Tag,
	_cx: &mut Session,
) -> Result<Vec<b::configuration_toClient_packet::Ty>> {
	Ok(vec![b::configuration_toClient_packet::Ty::disconnect(
		text::up(v),
	)])
}

pub fn late_config_disconnect_down_1_21_4(
	v: nbt::Tag,
	_cx: &mut Session,
) -> Result<Vec<a::configuration_toClient_packet::Ty>> {
	Ok(vec![a::configuration_toClient_packet::Ty::disconnect(
		text::down(v),
	)])
}

pub(super) type Links = Vec<a::configuration_toClient_packet__server_links__items::Ty>;

pub(super) fn links(mut v: Links, f: Text) -> Links {
	for l in &mut v {
		if let a::configuration_toClient_packet__server_links__items__hasKnownType::Ty::r#false(t) =
			&mut l.hasKnownType
		{
			*t = f(t.clone());
		}
	}
	v
}

pub fn late_config_links_up_1_21_5(
	v: Links,
	_cx: &mut Session,
) -> Result<Vec<b::configuration_toClient_packet::Ty>> {
	Ok(vec![b::configuration_toClient_packet::Ty::server_links(
		links(v, text::up),
	)])
}

pub fn late_config_links_down_1_21_4(
	v: Links,
	_cx: &mut Session,
) -> Result<Vec<a::configuration_toClient_packet::Ty>> {
	Ok(vec![a::configuration_toClient_packet::Ty::server_links(
		links(v, text::down),
	)])
}

pub fn late_links_up_1_21_5(
	v: Links,
	_cx: &mut Session,
) -> Result<Vec<b::play_toClient_packet::Ty>> {
	Ok(vec![b::play_toClient_packet::Ty::server_links(links(
		v,
		text::up,
	))])
}

pub fn late_links_down_1_21_4(
	v: Links,
	_cx: &mut Session,
) -> Result<Vec<a::play_toClient_packet::Ty>> {
	Ok(vec![a::play_toClient_packet::Ty::server_links(links(
		v,
		text::down,
	))])
}

fn opt(v: Option<nbt::Tag>, f: Text) -> Option<nbt::Tag> {
	v.map(f)
}

pub(super) fn system_chat(
	mut v: a::play_toClient_packet_system_chat::Ty,
	f: Text,
) -> a::play_toClient_packet_system_chat::Ty {
	v.content = f(v.content);
	v
}

pub(super) fn header(
	mut v: a::play_toClient_packet_playerlist_header::Ty,
	f: Text,
) -> a::play_toClient_packet_playerlist_header::Ty {
	v.header = f(v.header);
	v.footer = f(v.footer);
	v
}

pub(super) fn death(
	mut v: a::play_toClient_packet_death_combat_event::Ty,
	f: Text,
) -> a::play_toClient_packet_death_combat_event::Ty {
	v.message = f(v.message);
	v
}

pub(super) fn profileless(
	mut v: a::play_toClient_packet_profileless_chat::Ty,
	f: Text,
) -> a::play_toClient_packet_profileless_chat::Ty {
	v.message = f(v.message);
	v.name = f(v.name);
	v.target = opt(v.target, f);
	v
}

pub(super) fn objective(
	v: a::play_toClient_packet_scoreboard_objective::Ty,
	f: Text,
) -> a::play_toClient_packet_scoreboard_objective::Ty {
	use a::play_toClient_packet_scoreboard_objective__action::Ty as A;
	use a::play_toClient_packet_scoreboard_objective__action__v0__styling::Ty as S0;
	use a::play_toClient_packet_scoreboard_objective__action__v2__styling::Ty as S2;
	let action = match v.action {
		A::v0(mut o) => {
			o.displayText = f(o.displayText);
			o.styling = match o.styling {
				S0::v1(t) => S0::v1(f(t)),
				S0::v2(t) => S0::v2(f(t)),
				other => other,
			};
			A::v0(o)
		}
		A::v2(mut o) => {
			o.displayText = f(o.displayText);
			o.styling = match o.styling {
				S2::v1(t) => S2::v1(f(t)),
				S2::v2(t) => S2::v2(f(t)),
				other => other,
			};
			A::v2(o)
		}
		other => other,
	};
	a::play_toClient_packet_scoreboard_objective::Ty { action, ..v }
}

pub(super) fn score(
	mut v: a::play_toClient_packet_scoreboard_score::Ty,
	f: Text,
) -> a::play_toClient_packet_scoreboard_score::Ty {
	use a::play_toClient_packet_scoreboard_score__styling::Ty as S;
	v.display_name = opt(v.display_name, f);
	v.styling = match v.styling {
		S::v1(t) => S::v1(f(t)),
		S::v2(t) => S::v2(f(t)),
		other => other,
	};
	v
}

pub(super) fn player_info(
	mut v: a::play_toClient_packet_player_info::Ty,
	f: Text,
) -> a::play_toClient_packet_player_info::Ty {
	use a::play_toClient_packet_player_info__data__items__displayName::Ty as Name;
	for d in &mut v.data {
		if let Name::r#true(Some(name)) = &mut d.displayName {
			*name = f(name.clone());
		}
	}
	v
}

pub(super) fn boss_bar(
	mut v: a::play_toClient_packet_boss_bar::Ty,
	f: Text,
) -> a::play_toClient_packet_boss_bar::Ty {
	use a::play_toClient_packet_boss_bar__action::Ty as A;
	v.action = match v.action {
		A::v0(mut b) => {
			b.title = f(b.title);
			A::v0(b)
		}
		A::v3(t) => A::v3(f(t)),
		other => other,
	};
	v
}

pub(super) fn map(
	mut v: a::play_toClient_packet_map::Ty,
	f: Text,
) -> a::play_toClient_packet_map::Ty {
	for icon in v.icons.iter_mut().flatten() {
		icon.displayName = opt(icon.displayName.take(), f);
	}
	v
}

pub(super) fn tab_complete(
	mut v: a::play_toClient_packet_tab_complete::Ty,
	f: Text,
) -> a::play_toClient_packet_tab_complete::Ty {
	for m in &mut v.matches {
		m.tooltip = opt(m.tooltip.take(), f);
	}
	v
}

pub(super) fn server_data(
	mut v: a::play_toClient_packet_server_data::Ty,
	f: Text,
) -> a::play_toClient_packet_server_data::Ty {
	v.motd = f(v.motd);
	v
}

pub(super) fn resource_pack(
	mut v: a::packet_common_add_resource_pack::Ty,
	f: Text,
) -> a::packet_common_add_resource_pack::Ty {
	v.promptMessage = opt(v.promptMessage, f);
	v
}

macro_rules! texts {
	($($ty:ident: $visit:ident => $up:ident, $down:ident;)*) => {
		$(
			pub fn $up(v: a::$ty::Ty, _cx: &mut Session) -> Result<b::$ty::Ty> {
				Ok($visit(v, text::up))
			}

			pub fn $down(v: b::$ty::Ty, _cx: &mut Session) -> Result<a::$ty::Ty> {
				Ok($visit(v, text::down))
			}
		)*
	};
}

texts! {
	play_toClient_packet_system_chat: system_chat => late_system_chat_up_1_21_5, late_system_chat_down_1_21_4;
	play_toClient_packet_playerlist_header: header => late_header_up_1_21_5, late_header_down_1_21_4;
	play_toClient_packet_death_combat_event: death => late_death_up_1_21_5, late_death_down_1_21_4;
	play_toClient_packet_profileless_chat: profileless => late_profileless_up_1_21_5, late_profileless_down_1_21_4;
	play_toClient_packet_scoreboard_objective: objective => late_objective_up_1_21_5, late_objective_down_1_21_4;
	play_toClient_packet_scoreboard_score: score => late_score_up_1_21_5, late_score_down_1_21_4;
	play_toClient_packet_player_info: player_info => late_player_info_up_1_21_5, late_player_info_down_1_21_4;
	play_toClient_packet_boss_bar: boss_bar => late_boss_bar_up_1_21_5, late_boss_bar_down_1_21_4;
	play_toClient_packet_map: map => late_map_up_1_21_5, late_map_down_1_21_4;
	play_toClient_packet_tab_complete: tab_complete => late_tab_complete_up_1_21_5, late_tab_complete_down_1_21_4;
	play_toClient_packet_server_data: server_data => late_server_data_up_1_21_5, late_server_data_down_1_21_4;
	packet_common_add_resource_pack: resource_pack => late_resource_pack_up_1_21_5, late_resource_pack_down_1_21_4;
}

pub fn late_open_window_up_1_21_5(
	mut v: a::play_toClient_packet_open_window::Ty,
	cx: &mut Session,
) -> Result<b::play_toClient_packet_open_window::Ty> {
	v.windowTitle = text::up(v.windowTitle);
	cx.state::<Inventory>().windows.remove(&v.windowId);
	Ok(v)
}

pub fn late_open_window_down_1_21_4(
	mut v: b::play_toClient_packet_open_window::Ty,
	_cx: &mut Session,
) -> Result<a::play_toClient_packet_open_window::Ty> {
	v.windowTitle = text::down(v.windowTitle);
	Ok(v)
}

pub fn late_chat_content_up_1_21_5(
	v: &a::play_toClient_packet_player_chat::Ty,
	_cx: &mut Session,
) -> Result<Option<nbt::Tag>> {
	Ok(v.unsignedChatContent.clone().map(text::up))
}

pub fn late_chat_name_up_1_21_5(
	v: &a::play_toClient_packet_player_chat::Ty,
	_cx: &mut Session,
) -> Result<nbt::Tag> {
	Ok(text::up(v.networkName.clone()))
}

pub fn late_chat_target_up_1_21_5(
	v: &a::play_toClient_packet_player_chat::Ty,
	_cx: &mut Session,
) -> Result<Option<nbt::Tag>> {
	Ok(v.networkTargetName.clone().map(text::up))
}

pub fn late_chat_content_down_1_21_4(
	v: &b::play_toClient_packet_player_chat::Ty,
	_cx: &mut Session,
) -> Result<Option<nbt::Tag>> {
	Ok(v.unsignedChatContent.clone().map(text::down))
}

pub fn late_chat_name_down_1_21_4(
	v: &b::play_toClient_packet_player_chat::Ty,
	_cx: &mut Session,
) -> Result<nbt::Tag> {
	Ok(text::down(v.networkName.clone()))
}

pub fn late_chat_target_down_1_21_4(
	v: &b::play_toClient_packet_player_chat::Ty,
	_cx: &mut Session,
) -> Result<Option<nbt::Tag>> {
	Ok(v.networkTargetName.clone().map(text::down))
}

#[derive(Default)]
struct Inventory {
	windows: HashMap<i32, Vec<a::Slot::Ty>>,
	cursor: Option<a::Slot::Ty>,
}

fn player_slot(index: i32) -> Option<usize> {
	Some(match index {
		0..=8 => index as usize + 36,
		9..=35 => index as usize,
		36..=39 => (44 - index) as usize,
		40 => 45,
		_ => return None,
	})
}

fn track(cx: &mut Session, window: i32, slot: i32, item: a::Slot::Ty) {
	let inv = cx.state::<Inventory>();
	if window == -1 && slot == -1 {
		inv.cursor = Some(item);
		return;
	}
	let window = if window == -2 { 0 } else { window };
	let Ok(slot) = usize::try_from(slot) else {
		return;
	};
	let slots = inv.windows.entry(window).or_default();
	if slots.len() <= slot {
		slots.resize(slot + 1, a::Slot::Ty::v0);
	}
	slots[slot] = item;
}

pub fn late_window_items_up_1_21_5(
	p: &a::play_toClient_packet_window_items::Ty,
	cx: &mut Session,
) -> Result<Vec<b::Slot::Ty>> {
	let inv = cx.state::<Inventory>();
	inv.windows.insert(p.windowId, p.items.clone());
	inv.cursor = Some(p.carriedItem.clone());
	slots_up(p.items.clone())
}

pub fn late_set_slot_up_1_21_5(
	p: &a::play_toClient_packet_set_slot::Ty,
	cx: &mut Session,
) -> Result<b::Slot::Ty> {
	track(cx, p.windowId, p.slot as i32, p.item.clone());
	slot_up(p.item.clone())
}

pub fn late_player_inventory_up_1_21_5(
	p: &a::play_toClient_packet_set_player_inventory::Ty,
	cx: &mut Session,
) -> Result<b::Slot::Ty> {
	if let Some(slot) = player_slot(p.slotId) {
		track(cx, 0, slot as i32, p.contents.clone());
	}
	slot_up(p.contents.clone())
}

pub fn late_cursor_up_1_21_5(
	v: a::Slot::Ty,
	cx: &mut Session,
) -> Result<Vec<b::play_toClient_packet::Ty>> {
	cx.state::<Inventory>().cursor = Some(v.clone());
	Ok(vec![b::play_toClient_packet::Ty::set_cursor_item(slot_up(
		v,
	)?)])
}

fn item_of(s: &a::Slot::Ty) -> Option<i32> {
	match s {
		a::Slot::Ty::default(_, d) => Some(d.itemId),
		a::Slot::Ty::v0 => None,
	}
}

fn component_types(s: &a::Slot::Ty) -> Vec<i32> {
	let Ok(b::Slot::Ty::default(_, d)) = slot_up(s.clone()) else {
		return Vec::new();
	};
	let mut types: Vec<i32> = d
		.components
		.iter()
		.filter_map(|c| {
			let bytes = encode(c, b::SlotComponent::serialize).ok()?;
			let mut at = 0;
			super::util::read_varint(&bytes, &mut at).ok()
		})
		.collect();
	types.sort();
	types
}

fn recover(inv: &Inventory, window: i32, slot: i16, h: &Option<b::HashedSlot::Ty>) -> a::Slot::Ty {
	let Some(h) = h else { return a::Slot::Ty::v0 };
	let id = remap::item(NEW, OLD, h.itemId).unwrap_or(0);
	let mut wanted: Vec<i32> = h
		.components
		.iter()
		.filter_map(|c| {
			let bytes = encode(&c.r#type, b::SlotComponentType::serialize).ok()?;
			let mut at = 0;
			super::util::read_varint(&bytes, &mut at).ok()
		})
		.collect();
	wanted.sort();
	let window_slots = inv.windows.get(&window);
	let mut candidates: Vec<&a::Slot::Ty> = Vec::new();
	if let Some(s) = window_slots
		.and_then(|w| w.get(slot.max(0) as usize))
		.filter(|_| slot >= 0)
	{
		candidates.push(s);
	}
	candidates.extend(inv.cursor.iter());
	candidates.extend(window_slots.into_iter().flatten());
	if window != 0 {
		candidates.extend(inv.windows.get(&0).into_iter().flatten());
	}
	let same: Vec<&a::Slot::Ty> = candidates
		.into_iter()
		.filter(|s| item_of(s) == Some(id))
		.collect();
	let best = same
		.iter()
		.find(|s| component_types(s) == wanted)
		.or(same.first());
	match best {
		Some(a::Slot::Ty::default(_, d)) => a::Slot::Ty::default(h.itemCount, d.clone()),
		_ => a::Slot::Ty::default(
			h.itemCount,
			a::Slot__default::Ty {
				itemId: id,
				addedComponentCount: 0,
				removedComponentCount: 0,
				components: Vec::new(),
				removeComponents: Vec::new(),
			},
		),
	}
}

pub fn late_window_click_down_1_21_4(
	p: b::play_toServer_packet_window_click::Ty,
	cx: &mut Session,
) -> Result<a::play_toServer_packet_window_click::Ty> {
	let inv = cx.state::<Inventory>();
	let changed: Vec<(i16, a::Slot::Ty)> = p
		.changedSlots
		.iter()
		.map(|c| (c.location, recover(inv, p.windowId, c.location, &c.item)))
		.collect();
	let cursor = recover(inv, p.windowId, -1, &p.cursorItem);
	for (slot, item) in &changed {
		track(cx, p.windowId, *slot as i32, item.clone());
	}
	cx.state::<Inventory>().cursor = Some(cursor.clone());
	Ok(a::play_toServer_packet_window_click::Ty {
		windowId: p.windowId,
		stateId: p.stateId,
		slot: p.slot,
		mouseButton: p.mouseButton,
		mode: p.mode,
		changedSlots: changed
			.into_iter()
			.map(
				|(location, item)| a::play_toServer_packet_window_click__changedSlots__items::Ty {
					location,
					item,
				},
			)
			.collect(),
		cursorItem: cursor,
	})
}

pub fn late_creative_slot_down_1_21_4(
	p: &b::play_toServer_packet_set_creative_slot::Ty,
	cx: &mut Session,
) -> Result<a::Slot::Ty> {
	let item = slot_down(super::slots::late_to_slot_1_21_5(p.item.clone())?)?;
	track(cx, 0, p.slot as i32, item.clone());
	Ok(item)
}

pub fn late_creative_slot_up_1_21_5(
	p: &a::play_toServer_packet_set_creative_slot::Ty,
	_cx: &mut Session,
) -> Result<b::UntrustedSlot::Ty> {
	super::slots::late_to_untrusted_1_21_5(slot_up(p.item.clone())?)
}

fn variant(entries: &[&str], asset: impl Fn(&str) -> String) -> Vec<(String, Option<nbt::Tag>)> {
	entries
		.iter()
		.map(|k| {
			(
				format!("minecraft:{k}"),
				Some(Tag::new(Payload::compound(vec![Entry::new(
					"asset_id",
					Payload::string(asset(k)),
				)]))),
			)
		})
		.collect()
}

#[derive(Default)]
struct Registries {
	seen: Vec<String>,
}

pub fn late_registry_data_up_1_21_5(
	mut p: a::configuration_toClient_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_toClient_packet::Ty>> {
	shared::count_biomes(&p, cx);
	cx.state::<Registries>()
		.seen
		.push(p.id.strip_prefix("minecraft:").unwrap_or(&p.id).to_owned());
	shared::registry_entries(&mut p, "wolf_variant", |_, e| {
		let mut assets = Payload::compound(vec![]);
		for (old, new) in [
			("wild_texture", "wild"),
			("tame_texture", "tame"),
			("angry_texture", "angry"),
		] {
			if let Some(v) = e.take(old) {
				assets.put(new, v);
			}
		}
		e.take("biomes");
		e.put("assets", assets);
	});
	for (registry, field) in [
		("trim_material", "description"),
		("trim_pattern", "description"),
		("enchantment", "description"),
		("jukebox_song", "description"),
		("instrument", "description"),
	] {
		shared::registry_entries(&mut p, registry, |_, e| {
			if let Some(d) = e.take(field) {
				e.put(field, text::payload(d));
			}
		});
	}
	Ok(vec![b::configuration_toClient_packet::Ty::registry_data(p)])
}

pub fn late_finish_configuration_up_1_21_5(
	p: a::configuration_toClient_packet_finish_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_toClient_packet::Ty>> {
	let seen = std::mem::take(&mut cx.state::<Registries>().seen);
	let classic = Payload::compound(vec![
		Entry::new("ambient_sound", Payload::from("entity.wolf.ambient")),
		Entry::new("death_sound", Payload::from("entity.wolf.death")),
		Entry::new("growl_sound", Payload::from("entity.wolf.growl")),
		Entry::new("hurt_sound", Payload::from("entity.wolf.hurt")),
		Entry::new("pant_sound", Payload::from("entity.wolf.pant")),
		Entry::new("whine_sound", Payload::from("entity.wolf.whine")),
	]);
	let added: Vec<(&str, Vec<(String, Option<nbt::Tag>)>)> = vec![
		(
			"frog_variant",
			variant(&["temperate", "warm", "cold"], |k| {
				format!("minecraft:entity/frog/{k}_frog")
			}),
		),
		(
			"cat_variant",
			variant(
				&[
					"tabby",
					"black",
					"red",
					"siamese",
					"british_shorthair",
					"calico",
					"persian",
					"ragdoll",
					"white",
					"jellie",
					"all_black",
				],
				|k| format!("minecraft:entity/cat/{k}"),
			),
		),
		(
			"pig_variant",
			variant(&["temperate"], |k| format!("minecraft:entity/pig/{k}_pig")),
		),
		(
			"cow_variant",
			variant(&["temperate"], |k| format!("minecraft:entity/cow/{k}_cow")),
		),
		(
			"chicken_variant",
			variant(&["temperate"], |k| {
				format!("minecraft:entity/chicken/{k}_chicken")
			}),
		),
		(
			"wolf_sound_variant",
			vec![("minecraft:classic".to_owned(), Some(Tag::new(classic)))],
		),
	];
	let mut out: Vec<b::configuration_toClient_packet::Ty> = added
		.into_iter()
		.filter(|(id, _)| !seen.iter().any(|s| s == id))
		.map(|(id, entries)| {
			b::configuration_toClient_packet::Ty::registry_data(shared::registry(id, entries))
		})
		.collect();
	out.push(b::configuration_toClient_packet::Ty::finish_configuration(
		p,
	));
	Ok(out)
}

const HORSES: &[&str] = &[
	"horse",
	"donkey",
	"mule",
	"llama",
	"trader_llama",
	"camel",
	"skeleton_horse",
	"zombie_horse",
];
const MINECARTS: &[&str] = &[
	"minecart",
	"chest_minecart",
	"hopper_minecart",
	"furnace_minecart",
	"tnt_minecart",
	"spawner_minecart",
	"command_block_minecart",
];

fn particle_up(p: a::Particle::Ty) -> Result<b::Particle::Ty> {
	use a::Particle::Ty as A;
	use b::Particle::Ty as B;
	let state = |s: i32| remap::state(OLD, NEW, s);
	Ok(match p {
		A::block(s) => B::block(state(s)?),
		A::block_marker(s) => B::block_marker(state(s)?),
		A::falling_dust(s) => B::falling_dust(state(s)?),
		A::dust_pillar(s) => B::dust_pillar(state(s)?),
		A::block_crumble(s) => B::block_crumble(state(s)?),
		A::item(s) => B::item(slot_up(s)?),
		other => super::components::retag(
			|t| super::util::remap("particles", OLD, NEW, t),
			&other,
			a::Particle::serialize,
			b::Particle::deserialize,
		)?
		.ok_or(Error::Dropped)?,
	})
}

fn value_up(v: a::entityMetadataEntry__value::Ty) -> Result<b::entityMetadataEntry__value::Ty> {
	use a::entityMetadataEntry__value::Ty as A;
	use b::entityMetadataEntry__value::Ty as B;
	Ok(match v {
		A::byte(x) => B::byte(x),
		A::int(x) => B::int(x),
		A::long(x) => B::long(x),
		A::float(x) => B::float(x),
		A::string(x) => B::string(x),
		A::component(x) => B::component(text::up(x)),
		A::optional_component(x) => B::optional_component(x.map(text::up)),
		A::item_stack(x) => B::item_stack(slot_up(x)?),
		A::boolean(x) => B::boolean(x),
		A::rotations(x) => B::rotations(b::entityMetadataEntry__value__rotations::Ty {
			pitch: x.pitch,
			yaw: x.yaw,
			roll: x.roll,
		}),
		A::block_pos(x) => B::block_pos(x),
		A::optional_block_pos(x) => B::optional_block_pos(x),
		A::direction(x) => B::direction(x),
		A::optional_uuid(x) => B::optional_uuid(x),
		A::block_state(x) => B::block_state(remap::state(OLD, NEW, x)?),
		A::optional_block_state(x) => B::optional_block_state(remap::state(OLD, NEW, x)?),
		A::compound_tag(x) => B::compound_tag(x),
		A::particle(x) => B::particle(particle_up(x)?),
		A::particles(x) => B::particles(x.into_iter().map(particle_up).collect::<Result<_>>()?),
		A::villager_data(x) => B::villager_data(b::entityMetadataEntry__value__villager_data::Ty {
			villagerType: x.villagerType,
			villagerProfession: x.villagerProfession,
			level: x.level,
		}),
		A::optional_unsigned_int(x) => B::optional_unsigned_int(x),
		A::pose(x) => B::pose(x),
		A::cat_variant(x) => B::cat_variant(x),
		A::wolf_variant(x) => B::wolf_variant(match x {
			a::entityMetadataEntry__value__wolf_variant::Ty::Id(id) => id,
			a::entityMetadataEntry__value__wolf_variant::Ty::Inline(_) => 0,
		}),
		A::frog_variant(x) => B::frog_variant(x),
		A::optional_global_pos(None) => B::optional_global_pos(None),
		A::optional_global_pos(Some(_)) => return Err(Error::Dropped),
		A::painting_variant(x) => B::painting_variant(recode(
			&x,
			a::entityMetadataEntry__value__painting_variant::serialize,
			b::entityMetadataEntry__value__painting_variant::deserialize,
		)?),
		A::sniffer_state(x) => B::sniffer_state(x),
		A::armadillo_state(x) => B::armadillo_state(x),
		A::vector3(x) => B::vector3(x),
		A::quaternion(x) => B::quaternion(x),
		A::default(_) => return Err(Error::Dropped),
	})
}

fn saddle(entity: i32, on: bool) -> b::play_toClient_packet::Ty {
	let item = match crate::registry::names(NEW, "items").and_then(|n| n.id("saddle")) {
		Some(id) if on => b::Slot::Ty::default(
			1,
			b::Slot__default::Ty {
				itemId: id as i32,
				addedComponentCount: 0,
				removedComponentCount: 0,
				components: Vec::new(),
				removeComponents: Vec::new(),
			},
		),
		_ => b::Slot::Ty::v0,
	};
	b::play_toClient_packet::Ty::entity_equipment(b::play_toClient_packet_entity_equipment::Ty {
		entityId: entity,
		equipments: vec![
			b::play_toClient_packet_entity_equipment__equipments__items::Ty { slot: 7, item },
		],
	})
}

pub fn late_metadata_up_1_21_5(
	p: a::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<b::play_toClient_packet::Ty>> {
	let kind = world::kind(cx, OLD, p.entityId).unwrap_or_default();
	let kind = kind.as_str();
	let mut extra = Vec::new();
	let mut out = Vec::new();
	for e in p.metadata {
		use a::entityMetadataEntry__value::Ty as A;
		let mut key = e.key;
		let value = match (kind, key, e.value) {
			(k, 11, A::int(state)) if MINECARTS.contains(&k) => {
				match remap::state(OLD, NEW, state)? {
					0 => continue,
					s => b::entityMetadataEntry__value::Ty::optional_block_state(s),
				}
			}
			(k, 13, _) if MINECARTS.contains(&k) => continue,
			(k, 14 | 15, v) if MINECARTS.contains(&k) => {
				key -= 1;
				value_up(v)?
			}
			("mooshroom", 17, A::string(s)) => {
				b::entityMetadataEntry__value::Ty::int(if s == "red" { 0 } else { 1 })
			}
			("pig", 17, A::boolean(on)) | ("strider", 19, A::boolean(on)) => {
				extra.push(saddle(p.entityId, on));
				continue;
			}
			("pig", 18, v) => {
				key = 17;
				value_up(v)?
			}
			(k, 17, A::byte(flags)) if HORSES.contains(&k) => {
				extra.push(saddle(p.entityId, flags & 0x04 != 0));
				b::entityMetadataEntry__value::Ty::byte(flags)
			}
			("dolphin", 17, _) => continue,
			("dolphin", 18 | 19, v) => {
				key -= 1;
				value_up(v)?
			}
			("turtle", 17 | 20 | 21 | 22, _) => continue,
			("turtle", 18 | 19, v) => {
				key -= 1;
				value_up(v)?
			}
			(_, _, v) => match value_up(v) {
				Ok(v) => v,
				Err(Error::Dropped) => continue,
				Err(e) => return Err(e),
			},
		};
		out.push(b::entityMetadataEntry::Ty { key, value });
	}
	let mut packets = vec![b::play_toClient_packet::Ty::entity_metadata(
		b::play_toClient_packet_entity_metadata::Ty {
			entityId: p.entityId,
			metadata: out,
		},
	)];
	packets.extend(extra);
	Ok(packets)
}

pub fn late_untrusted_down_1_21_4(
	v: b::UntrustedSlot::Ty,
	_cx: &mut Session,
) -> Result<a::Slot::Ty> {
	slot_down(super::slots::late_to_slot_1_21_5(v)?)
}

pub fn late_untrusted_up_1_21_5(v: a::Slot::Ty, _cx: &mut Session) -> Result<b::UntrustedSlot::Ty> {
	super::slots::late_to_untrusted_1_21_5(slot_up(v)?)
}

fn hashed(s: &a::Slot::Ty, cx: &mut Session) -> Result<Option<b::HashedSlot::Ty>> {
	let a::Slot::Ty::default(count, d) = s else {
		return Ok(None);
	};
	let key = encode(d, a::Slot__default::serialize)?;
	let slot = match cx.state::<Sent>().0.get(&key) {
		Some(original) => b::Slot::Ty::default(*count, original.clone()),
		None => slot_up(s.clone())?,
	};
	Ok(hash::slot(&slot, cx.state::<hash::Keys>()))
}

pub fn late_window_click_up_1_21_5(
	p: a::play_toServer_packet_window_click::Ty,
	cx: &mut Session,
) -> Result<b::play_toServer_packet_window_click::Ty> {
	let mut changedSlots = Vec::with_capacity(p.changedSlots.len());
	for c in &p.changedSlots {
		changedSlots.push(
			b::play_toServer_packet_window_click__changedSlots__items::Ty {
				location: c.location,
				item: hashed(&c.item, cx)?,
			},
		);
	}
	Ok(b::play_toServer_packet_window_click::Ty {
		windowId: p.windowId,
		stateId: p.stateId,
		slot: p.slot,
		mouseButton: p.mouseButton,
		mode: p.mode,
		changedSlots,
		cursorItem: hashed(&p.cursorItem, cx)?,
	})
}

const NEW_REGISTRIES: &[&str] = &[
	"pig_variant",
	"cow_variant",
	"frog_variant",
	"cat_variant",
	"chicken_variant",
	"wolf_sound_variant",
	"test_environment",
	"test_instance",
];

const TEXT_FIELDS: &[(&str, &str)] = &[
	("trim_material", "description"),
	("trim_pattern", "description"),
	("enchantment", "description"),
	("jukebox_song", "description"),
	("instrument", "description"),
	("painting_variant", "title"),
	("painting_variant", "author"),
];

pub fn late_registry_data_down_1_21_4(
	mut p: b::configuration_toClient_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<a::configuration_toClient_packet::Ty>> {
	shared::count_biomes(&p, cx);
	let id = p.id.strip_prefix("minecraft:").unwrap_or(&p.id).to_owned();
	cx.state::<hash::Keys>().0.insert(
		id.clone(),
		p.entries.iter().map(|e| e.key.clone()).collect(),
	);
	if NEW_REGISTRIES.contains(&id.as_str()) {
		return Ok(Vec::new());
	}
	shared::registry_entries(&mut p, "wolf_variant", |_, e| {
		if let Some(assets) = e.take("assets") {
			for (new, old) in [
				("wild", "wild_texture"),
				("tame", "tame_texture"),
				("angry", "angry_texture"),
			] {
				if let Some(v) = assets.field(new) {
					e.put(old, v.clone());
				}
			}
		}
		e.take("spawn_conditions");
		e.put("biomes", List::new(ListItems::string(Vec::new())));
	});
	for (registry, field) in [
		("trim_material", "ingredient"),
		("trim_pattern", "template_item"),
	] {
		shared::registry_entries(&mut p, registry, |_, e| {
			if e.field(field).is_none() {
				e.put(field, Payload::from("minecraft:stone"));
			}
		});
	}
	shared::registry_entries(&mut p, "enchantment", |_, e| {
		if let Some(Payload::list(nbt::List {
			items: ListItems::string(slots),
			..
		})) = e.take("slots")
		{
			e.put(
				"slots",
				List::new(ListItems::string(
					slots.into_iter().filter(|s| s != "saddle").collect(),
				)),
			);
		}
	});
	for (registry, field) in TEXT_FIELDS {
		shared::registry_entries(&mut p, registry, |_, e| {
			if let Some(d) = e.take(field) {
				e.put(field, text::payload_down(d));
			}
		});
	}
	Ok(vec![a::configuration_toClient_packet::Ty::registry_data(p)])
}

const CATS: &[&str] = &[
	"tabby",
	"black",
	"red",
	"siamese",
	"british_shorthair",
	"calico",
	"persian",
	"ragdoll",
	"white",
	"jellie",
	"all_black",
];
const FROGS: &[&str] = &["temperate", "warm", "cold"];

fn static_variant(cx: &mut Session, registry: &str, names: &[&str], id: i32, fallback: i32) -> i32 {
	cx.state::<hash::Keys>()
		.get(registry, id)
		.and_then(|k| {
			names
				.iter()
				.position(|n| *n == k.strip_prefix("minecraft:").unwrap_or(&k))
		})
		.map_or(fallback, |i| i as i32)
}

fn particle_down(p: b::Particle::Ty) -> Result<a::Particle::Ty> {
	use a::Particle::Ty as A;
	use b::Particle::Ty as B;
	let state = |s: i32| remap::state(NEW, OLD, s);
	Ok(match p {
		B::block(s) => A::block(state(s)?),
		B::block_marker(s) => A::block_marker(state(s)?),
		B::falling_dust(s) => A::falling_dust(state(s)?),
		B::dust_pillar(s) => A::dust_pillar(state(s)?),
		B::block_crumble(s) => A::block_crumble(state(s)?),
		B::item(s) => A::item(slot_down(s)?),
		other => super::components::retag(
			|t| super::util::remap("particles", NEW, OLD, t),
			&other,
			b::Particle::serialize,
			a::Particle::deserialize,
		)?
		.ok_or(Error::Dropped)?,
	})
}

fn value_down(
	v: b::entityMetadataEntry__value::Ty,
	cx: &mut Session,
) -> Result<a::entityMetadataEntry__value::Ty> {
	use a::entityMetadataEntry__value::Ty as A;
	use b::entityMetadataEntry__value::Ty as B;
	Ok(match v {
		B::byte(x) => A::byte(x),
		B::int(x) => A::int(x),
		B::long(x) => A::long(x),
		B::float(x) => A::float(x),
		B::string(x) => A::string(x),
		B::component(x) => A::component(text::down(x)),
		B::optional_component(x) => A::optional_component(x.map(text::down)),
		B::item_stack(x) => A::item_stack(slot_down(x)?),
		B::boolean(x) => A::boolean(x),
		B::rotations(x) => A::rotations(a::entityMetadataEntry__value__rotations::Ty {
			pitch: x.pitch,
			yaw: x.yaw,
			roll: x.roll,
		}),
		B::block_pos(x) => A::block_pos(x),
		B::optional_block_pos(x) => A::optional_block_pos(x),
		B::direction(x) => A::direction(x),
		B::optional_uuid(x) => A::optional_uuid(x),
		B::block_state(x) => A::block_state(remap::state(NEW, OLD, x)?),
		B::optional_block_state(x) => A::optional_block_state(remap::state(NEW, OLD, x)?),
		B::compound_tag(x) => A::compound_tag(x),
		B::particle(x) => A::particle(particle_down(x)?),
		B::particles(x) => A::particles(
			x.into_iter()
				.filter_map(|p| particle_down(p).ok())
				.collect(),
		),
		B::villager_data(x) => A::villager_data(a::entityMetadataEntry__value__villager_data::Ty {
			villagerType: x.villagerType,
			villagerProfession: x.villagerProfession,
			level: x.level,
		}),
		B::optional_unsigned_int(x) => A::optional_unsigned_int(x),
		B::pose(x) => A::pose(x),
		B::cat_variant(x) => A::cat_variant(static_variant(cx, "cat_variant", CATS, x, 1)),
		B::frog_variant(x) => A::frog_variant(static_variant(cx, "frog_variant", FROGS, x, 0)),
		B::wolf_variant(x) => {
			A::wolf_variant(a::entityMetadataEntry__value__wolf_variant::Ty::Id(x))
		}
		B::optional_global_pos(None) => A::optional_global_pos(None),
		B::painting_variant(x) => A::painting_variant(recode(
			&x,
			b::entityMetadataEntry__value__painting_variant::serialize,
			a::entityMetadataEntry__value__painting_variant::deserialize,
		)?),
		B::sniffer_state(x) => A::sniffer_state(x),
		B::armadillo_state(x) => A::armadillo_state(x),
		B::vector3(x) => A::vector3(x),
		B::quaternion(x) => A::quaternion(x),
		B::optional_global_pos(Some(_))
		| B::cow_variant(_)
		| B::wolf_sound_variant(_)
		| B::pig_variant(_)
		| B::chicken_variant(_)
		| B::default(_) => {
			return Err(Error::Dropped);
		}
	})
}

#[derive(Default)]
struct Horses(HashMap<i32, (i8, bool)>);

fn entry(key: u8, value: a::entityMetadataEntry__value::Ty) -> a::entityMetadataEntry::Ty {
	a::entityMetadataEntry::Ty { key, value }
}

pub fn late_metadata_down_1_21_4(
	p: b::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<a::play_toClient_packet::Ty>> {
	use a::entityMetadataEntry__value::Ty as A;
	use b::entityMetadataEntry__value::Ty as B;
	let kind = world::kind(cx, NEW, p.entityId).unwrap_or_default();
	let kind = kind.as_str();
	let mut out = Vec::new();
	for e in p.metadata {
		let mut key = e.key;
		let v = match (kind, key, e.value) {
			(k, 11, B::optional_block_state(s)) if MINECARTS.contains(&k) => {
				if s != 0 {
					out.push(entry(11, A::int(remap::state(NEW, OLD, s)?)));
					out.push(entry(13, A::boolean(true)));
				}
				continue;
			}
			(k, 13 | 14, v) if MINECARTS.contains(&k) => {
				key += 1;
				v
			}
			("mooshroom", 17, B::int(t)) => {
				out.push(entry(
					17,
					A::string(if t == 0 { "red" } else { "brown" }.to_owned()),
				));
				continue;
			}
			(k, 17, B::byte(flags)) if HORSES.contains(&k) => {
				let horse = cx.state::<Horses>().0.entry(p.entityId).or_default();
				horse.0 = flags;
				out.push(entry(
					17,
					A::byte(if horse.1 { flags | 0x04 } else { flags }),
				));
				continue;
			}
			("pig", 17, v) | ("dolphin", 17 | 18, v) | ("turtle", 17 | 18, v) => {
				key += 1;
				v
			}
			("experience_orb", 8, _) => continue,
			(_, _, v) => v,
		};
		match value_down(v, cx) {
			Ok(value) => out.push(entry(key, value)),
			Err(Error::Dropped) => {}
			Err(e) => return Err(e),
		}
	}
	Ok(vec![a::play_toClient_packet::Ty::entity_metadata(
		a::play_toClient_packet_entity_metadata::Ty {
			entityId: p.entityId,
			metadata: out,
		},
	)])
}

pub fn late_equipment_down_1_21_4(
	p: b::play_toClient_packet_entity_equipment::Ty,
	cx: &mut Session,
) -> Result<Vec<a::play_toClient_packet::Ty>> {
	use a::entityMetadataEntry__value::Ty as A;
	let kind = world::kind(cx, NEW, p.entityId).unwrap_or_default();
	let mut equipments = Vec::new();
	let mut metadata = Vec::new();
	for e in p.equipments {
		if e.slot != 7 {
			equipments.push(
				a::play_toClient_packet_entity_equipment__equipments__items::Ty {
					slot: e.slot,
					item: slot_down(e.item)?,
				},
			);
			continue;
		}
		let on = !matches!(e.item, b::Slot::Ty::v0);
		match kind.as_str() {
			"pig" => metadata.push(entry(17, A::boolean(on))),
			"strider" => metadata.push(entry(19, A::boolean(on))),
			k if HORSES.contains(&k) => {
				let horse = cx.state::<Horses>().0.entry(p.entityId).or_default();
				horse.1 = on;
				metadata.push(entry(
					17,
					A::byte(if on { horse.0 | 0x04 } else { horse.0 & !0x04 }),
				));
			}
			_ => {}
		}
	}
	let mut out = Vec::new();
	if !equipments.is_empty() {
		out.push(a::play_toClient_packet::Ty::entity_equipment(
			a::play_toClient_packet_entity_equipment::Ty {
				entityId: p.entityId,
				equipments,
			},
		));
	}
	if !metadata.is_empty() {
		out.push(a::play_toClient_packet::Ty::entity_metadata(
			a::play_toClient_packet_entity_metadata::Ty {
				entityId: p.entityId,
				metadata,
			},
		));
	}
	Ok(out)
}

pub fn late_spawn_entity_down_1_21_4(
	p: b::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<a::play_toClient_packet::Ty>> {
	let name = shared::spawned(cx, p.entityId, NEW, p.r#type);
	if name == Some("experience_orb") {
		let mut out = vec![a::play_toClient_packet::Ty::spawn_entity_experience_orb(
			a::play_toClient_packet_spawn_entity_experience_orb::Ty {
				entityId: p.entityId,
				x: p.x,
				y: p.y,
				z: p.z,
				count: p.objectData.clamp(1, i16::MAX as i32) as i16,
			},
		)];
		let v = &p.velocity;
		if v.x != 0 || v.y != 0 || v.z != 0 {
			out.push(a::play_toClient_packet::Ty::entity_velocity(
				a::play_toClient_packet_entity_velocity::Ty {
					entityId: p.entityId,
					velocity: p.velocity,
				},
			));
		}
		return Ok(out);
	}
	let objectData = if name == Some("falling_block") {
		remap::state(NEW, OLD, p.objectData)?
	} else {
		p.objectData
	};
	Ok(vec![a::play_toClient_packet::Ty::spawn_entity(
		a::play_toClient_packet_spawn_entity::Ty {
			r#type: remap::entity(NEW, OLD, p.r#type)?,
			objectData,
			..p
		},
	)])
}

pub fn late_chunk_biomes_down_1_21_4(
	v: Vec<b::play_toClient_packet__chunk_biomes__items::Ty>,
	cx: &mut Session,
) -> Result<Vec<a::play_toClient_packet::Ty>> {
	let biomes = world::biomes(cx);
	let items = v
		.into_iter()
		.map(|c| {
			Ok(a::play_toClient_packet__chunk_biomes__items::Ty {
				data: chunk::biomes_step(NEW, OLD, &c.data, biomes)?,
				position: c.position,
			})
		})
		.collect::<Result<_>>()?;
	Ok(vec![a::play_toClient_packet::Ty::chunk_biomes(items)])
}

fn block_entity_down(tag: Option<nbt::Tag>) -> Option<nbt::Tag> {
	let mut tag = tag?;
	for side in ["front_text", "back_text"] {
		if let Some(mut s) = tag.value.take(side) {
			for field in ["messages", "filtered_messages"] {
				if let Some(lines) = s.field(field).and_then(text::json_lines) {
					s.put(field, List::new(ListItems::string(lines)));
				}
			}
			tag.value.put(side, s);
		}
	}
	if let Some(name) = tag.value.take("CustomName") {
		tag.value
			.put("CustomName", Payload::string(text::to_json(name)));
	}
	Some(Tag::new(tag.value))
}

pub fn late_chunk_block_entity_down_1_21_4(
	v: &b::chunkBlockEntity::Ty,
	_cx: &mut Session,
) -> Result<Option<nbt::Tag>> {
	Ok(block_entity_down(v.nbtData.clone()))
}

pub fn late_tile_entity_down_1_21_4(
	v: &b::play_toClient_packet_tile_entity_data::Ty,
	_cx: &mut Session,
) -> Result<Option<nbt::Tag>> {
	Ok(block_entity_down(v.nbtData.clone()))
}
