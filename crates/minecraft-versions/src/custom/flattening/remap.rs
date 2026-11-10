use super::blocks::{
	block_earlier, block_later, item_earlier, item_later, state_earlier, state_later,
};
use super::entities::forget_later;
use super::items::slot_earlier;
use super::text::legacy_title;
use crate::gen::convert::{v1_13 as b, v1_13_1 as c};
use crate::pds::convert::Result;
use crate::session::Session;

#[derive(Default)]
struct Later {
	dimension: i32,
}

fn state(s: i32, f: fn(u32) -> u32) -> i32 {
	f(s.max(0) as u32) as i32
}

pub fn flattening_block_change_state_1_13_1(
	src: &b::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(src.r#type, state_later))
}

pub fn flattening_block_change_state_1_13(
	src: &c::play_toClient_packet_block_change::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(src.r#type, state_earlier))
}

pub fn flattening_record_state_1_13_1(
	src: &b::play_toClient_packet_multi_block_change__records__items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(src.blockId, state_later))
}

pub fn flattening_record_state_1_13(
	src: &c::play_toClient_packet_multi_block_change__records__items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(state(src.blockId, state_earlier))
}

pub fn flattening_block_action_id_1_13_1(
	src: &b::play_toClient_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(block_later(src.blockId))
}

pub fn flattening_block_action_id_1_13(
	src: &c::play_toClient_packet_block_action::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(block_earlier(src.blockId))
}

pub fn flattening_cooldown_item_1_13_1(
	src: &b::play_toClient_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_later(src.itemID).unwrap_or(0))
}

pub fn flattening_cooldown_item_1_13(
	src: &c::play_toClient_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item_earlier(src.itemID).unwrap_or(0))
}

fn retag(
	p: b::play_toClient_packet_tags::Ty,
	block: fn(i32) -> i32,
	item: fn(i32) -> Option<i32>,
) -> b::play_toClient_packet_tags::Ty {
	let blocks = |tags: Vec<b::play_toClient_packet_tags__blockTags__items::Ty>| {
		tags.into_iter()
			.map(|mut t| {
				t.entries = t.entries.into_iter().map(block).collect();
				t
			})
			.collect()
	};
	let items = |tags: Vec<b::play_toClient_packet_tags__blockTags__items::Ty>| {
		tags.into_iter()
			.map(|mut t| {
				t.entries = t.entries.into_iter().filter_map(item).collect();
				t
			})
			.collect()
	};
	b::play_toClient_packet_tags::Ty {
		blockTags: blocks(p.blockTags),
		itemTags: items(p.itemTags),
		fluidTags: p.fluidTags,
	}
}

pub fn flattening_tag_ids_1_13_1(
	p: b::play_toClient_packet_tags::Ty,
	_cx: &mut Session,
) -> Result<c::play_toClient_packet_tags::Ty> {
	Ok(retag(p, block_later, item_later))
}

pub fn flattening_tag_ids_1_13(
	p: c::play_toClient_packet_tags::Ty,
	_cx: &mut Session,
) -> Result<b::play_toClient_packet_tags::Ty> {
	Ok(retag(p, block_earlier, item_earlier))
}

fn fog(flags: u8) -> u8 {
	if flags & 2 != 0 {
		flags | 4
	} else {
		flags
	}
}

pub fn flattening_boss_bar_flags_1_13_1(
	src: &b::play_toClient_packet_boss_bar__action__v0::Ty,
	_cx: &mut Session,
) -> Result<u8> {
	Ok(fog(src.flags))
}

fn dragon(flags: u8) -> u8 {
	if flags & 4 != 0 {
		flags | 2
	} else {
		flags
	}
}

pub fn flattening_boss_bar_flags_1_13(
	src: &c::play_toClient_packet_boss_bar__action__v0::Ty,
	_cx: &mut Session,
) -> Result<u8> {
	Ok(dragon(src.flags))
}

pub fn flattening_boss_bar_update_1_13_1(
	flags: u8,
	_cx: &mut Session,
) -> Result<c::play_toClient_packet_boss_bar__action::Ty> {
	Ok(c::play_toClient_packet_boss_bar__action::Ty::v5(fog(flags)))
}

pub fn flattening_boss_bar_update_1_13(
	flags: u8,
	_cx: &mut Session,
) -> Result<b::play_toClient_packet_boss_bar__action::Ty> {
	Ok(b::play_toClient_packet_boss_bar__action::Ty::v5(dragon(
		flags,
	)))
}

pub fn flattening_window_title_1_13(
	src: &c::play_toClient_packet_open_window::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(legacy_title(&src.windowTitle))
}

type Properties = b::command_node__extraNodeData__v2__properties::Ty;

pub fn flattening_declare_commands_1_13(
	p: c::play_toClient_packet_declare_commands::Ty,
	_cx: &mut Session,
) -> Result<b::play_toClient_packet_declare_commands::Ty> {
	let nodes = p
		.nodes
		.into_iter()
		.map(|mut n| {
			if let b::command_node__extraNodeData::Ty::v2(arg) = &mut n.extraNodeData {
				let (parser, properties) = match arg.parser.as_str() {
					"minecraft:column_pos" => ("minecraft:vec2", Properties::minecraft_vec2),
					"minecraft:dimension" => (
						"brigadier:string",
						Properties::brigadier_string(b::command_node__extraNodeData__v2__properties__brigadier_string::Ty::SINGLE_WORD),
					),
					_ => return n,
				};
				arg.parser = parser.to_owned();
				arg.properties = properties;
			}
			n
		})
		.collect();
	Ok(b::play_toClient_packet_declare_commands::Ty {
		nodes,
		rootIndex: p.rootIndex,
	})
}

fn custom_stat_later(id: i32) -> Option<i32> {
	match id {
		0..=21 => Some(id),
		22 => Some(24),
		23..=34 => Some(id + 5),
		35..=51 => Some(id + 6),
		_ => None,
	}
}

fn custom_stat_earlier(id: i32) -> Option<i32> {
	(0..=51).find(|old| custom_stat_later(*old) == Some(id))
}

type Stat = b::play_toClient_packet__statistics__items::Ty;

fn restat(stats: Vec<Stat>, up: bool) -> Vec<Stat> {
	stats
		.into_iter()
		.filter_map(|mut s| {
			s.statisticId = match (s.categoryId, up) {
				(0, true) => block_later(s.statisticId),
				(0, false) => block_earlier(s.statisticId),
				(1..=5, true) => item_later(s.statisticId)?,
				(1..=5, false) => item_earlier(s.statisticId)?,
				(8, true) => custom_stat_later(s.statisticId)?,
				(8, false) => custom_stat_earlier(s.statisticId)?,
				_ => s.statisticId,
			};
			Some(s)
		})
		.collect()
}

pub fn flattening_statistic_ids_1_13_1(
	stats: Vec<Stat>,
	_cx: &mut Session,
) -> Result<Vec<c::play_toClient_packet::Ty>> {
	Ok(vec![c::play_toClient_packet::Ty::statistics(restat(
		stats, true,
	))])
}

pub fn flattening_statistic_ids_1_13(
	stats: Vec<Stat>,
	_cx: &mut Session,
) -> Result<Vec<b::play_toClient_packet::Ty>> {
	Ok(vec![b::play_toClient_packet::Ty::statistics(restat(
		stats, false,
	))])
}

fn dimension_change(cx: &mut Session, dimension: i32) {
	let later = cx.state::<Later>();
	if later.dimension != dimension {
		later.dimension = dimension;
		forget_later(cx);
	}
}

pub fn flattening_dimension_change_1_13_1(
	p: b::play_toClient_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Vec<c::play_toClient_packet::Ty>> {
	dimension_change(cx, p.dimension);
	Ok(vec![c::play_toClient_packet::Ty::respawn(p)])
}

pub fn flattening_dimension_change_1_13(
	p: c::play_toClient_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Vec<b::play_toClient_packet::Ty>> {
	dimension_change(cx, p.dimension);
	Ok(vec![b::play_toClient_packet::Ty::respawn(p)])
}

pub fn flattening_edit_book_hand_1_13(
	p: c::play_toServer_packet_edit_book::Ty,
	_cx: &mut Session,
) -> Result<Vec<b::play_toServer_packet::Ty>> {
	if p.hand == 1 {
		return Ok(vec![]);
	}
	Ok(vec![b::play_toServer_packet::Ty::edit_book(
		b::play_toServer_packet_edit_book::Ty {
			new_book: slot_earlier(p.new_book),
			signing: p.signing,
		},
	)])
}

pub fn flattening_command_slash_1_13_1(
	src: &b::play_toServer_packet_tab_complete::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(if src.text.starts_with('/') {
		src.text.clone()
	} else {
		format!("/{}", src.text)
	})
}

pub fn flattening_command_slash_1_13(
	src: &c::play_toServer_packet_tab_complete::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(src.text.strip_prefix('/').unwrap_or(&src.text).to_owned())
}
