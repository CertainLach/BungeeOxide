use ::nbt::{CompoundExt, Entry, List, ListItems as Items, Payload, Payload as P};

use super::meta::Slot;
use super::*;
use crate::gen::convert::{v1_17 as old, v1_17_1 as new};
use crate::pds::convert::Result;
use crate::session::Session;

pub fn nether_destroy_entity_1_17_1(
	id: i32,
	cx: &mut Session,
) -> Result<Vec<new::play_to_client_packet::Ty>> {
	untrack(cx, &[id]);
	Ok(vec![new::play_to_client_packet::Ty::EntityDestroy(vec![
		id,
	])])
}

#[derive(Default)]
struct StateIds(std::collections::HashMap<i32, i32>);

pub fn nether_set_slot_1_17_1(
	v: old::play_to_client_packet_set_slot::Ty,
	_cx: &mut Session,
) -> Result<new::play_to_client_packet_set_slot::Ty> {
	Ok(new::play_to_client_packet_set_slot::Ty {
		window_id: v.window_id,
		state_id: 0,
		slot: v.slot,
		item: v.item,
	})
}

pub fn nether_set_slot_1_17(
	v: new::play_to_client_packet_set_slot::Ty,
	cx: &mut Session,
) -> Result<old::play_to_client_packet_set_slot::Ty> {
	cx.state::<StateIds>()
		.0
		.insert(v.window_id as i32, v.state_id);
	Ok(old::play_to_client_packet_set_slot::Ty {
		window_id: v.window_id,
		slot: v.slot,
		item: v.item,
	})
}

pub fn nether_window_items_1_17_1(
	v: old::play_to_client_packet_window_items::Ty,
	_cx: &mut Session,
) -> Result<Vec<new::play_to_client_packet::Ty>> {
	Ok(vec![new::play_to_client_packet::Ty::WindowItems(
		new::play_to_client_packet_window_items::Ty {
			window_id: v.window_id,
			state_id: 0,
			items: v.items,
			carried_item: Slot::False,
		},
	)])
}

pub fn nether_window_items_1_17(
	v: new::play_to_client_packet_window_items::Ty,
	cx: &mut Session,
) -> Result<Vec<old::play_to_client_packet::Ty>> {
	cx.state::<StateIds>()
		.0
		.insert(v.window_id as i8 as i32, v.state_id);
	Ok(vec![
		old::play_to_client_packet::Ty::WindowItems(old::play_to_client_packet_window_items::Ty {
			window_id: v.window_id,
			items: v.items,
		}),
		old::play_to_client_packet::Ty::SetSlot(old::play_to_client_packet_set_slot::Ty {
			window_id: -1,
			slot: -1,
			item: v.carried_item,
		}),
	])
}

pub fn nether_state_id_1_17_1(
	src: &old::play_to_server_packet_window_click::Ty,
	cx: &mut Session,
) -> Result<i32> {
	Ok(cx
		.state::<StateIds>()
		.0
		.get(&(src.window_id as i8 as i32))
		.copied()
		.unwrap_or(0))
}

pub fn nether_edit_book_1_17_1(
	v: old::play_to_server_packet_edit_book::Ty,
	_cx: &mut Session,
) -> Result<new::play_to_server_packet_edit_book::Ty> {
	let tag = match &v.new_book {
		Slot::True(s) => s
			.nbt_data
			.as_ref()
			.and_then(|e| e.value.as_compound())
			.cloned()
			.unwrap_or_default(),
		Slot::False => Vec::new(),
	};
	let pages = match tag.field("pages") {
		Some(P::List(List {
			items: Items::String(pages),
			..
		})) => pages.clone(),
		_ => Vec::new(),
	};
	let title = tag
		.field("title")
		.and_then(Payload::as_str)
		.filter(|_| v.signing)
		.map(str::to_owned);
	Ok(new::play_to_server_packet_edit_book::Ty {
		hand: v.hand,
		pages,
		title,
	})
}

pub fn nether_edit_book_1_17(
	v: new::play_to_server_packet_edit_book::Ty,
	_cx: &mut Session,
) -> Result<old::play_to_server_packet_edit_book::Ty> {
	let Some(id) = registry::names(Version::V1_17, "items").and_then(|n| n.id("writable_book"))
	else {
		return unconvertible("1.17 has no writable_book item");
	};
	let mut pages: Vec<String> = v.pages.into_iter().take(200).collect();
	if pages.is_empty() {
		pages.push(String::new());
	}
	let mut tag = vec![Entry::new("pages", Items::String(pages))];
	let signing = v.title.is_some();
	if let Some(title) = v.title {
		tag.push(Entry::new("title", P::String(title)));
		tag.push(Entry::new("author", P::String(String::new())));
	}
	Ok(old::play_to_server_packet_edit_book::Ty {
		new_book: Slot::True(crate::gen::convert::v1_17::slot_true::Ty {
			item_id: id as i32,
			item_count: 1,
			nbt_data: Some(Entry::new("", tag)),
		}),
		signing,
		hand: v.hand,
	})
}
