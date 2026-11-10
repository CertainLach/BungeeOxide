use std::sync::OnceLock;

use nbt::{CompoundExt, CompoundVecExt, Entry, List, ListItems, Payload};

use super::text;
use crate::gen::convert::{v1_7, v1_8, Version};
use crate::pds::convert::{Error, Result};
use crate::registry;
use crate::session::Session;

type Slot7 = v1_7::slot::Ty;
type Slot8 = v1_8::slot::Ty;

const MARK: &str = "pds2|1.8";

pub(super) fn block_1_7(id: u16, meta: u8) -> (u16, u8) {
	match (id, meta) {
		(1, 1..=6) => (1, 0),
		(77 | 143, 0) => (69, 0),
		(77 | 143, 5) => (69, 6),
		(77 | 143, 8) => (69, 8),
		(77 | 143, 13) => (69, 14),
		(165, _) => (95, 5),
		(166, _) => (20, 0),
		(167, m) => (96, m),
		(168, 0) => (1, 0),
		(168, 1) => (98, 0),
		(168, _) => (98, 1),
		(169, _) => (89, 0),
		(176, m) => (63, m),
		(177, m) => (68, m),
		(178, m) => (151, m),
		(179, m) => (24, m),
		(180, m) => (128, m),
		(181, _) => (43, 1),
		(182, m) => (44, if m & 8 != 0 { 9 } else { 1 }),
		(183..=187, m) => (107, m),
		(188..=192, m) => (85, m),
		(193..=197, m) => (64, m),
		(id, m) if known_block_1_7(id) => (id, m),
		_ => (0, 0),
	}
}

fn known_block_1_7(id: u16) -> bool {
	static KNOWN: OnceLock<Vec<bool>> = OnceLock::new();
	let known = KNOWN.get_or_init(|| {
		let mut known = vec![false; 4096];
		for b in registry::blocks(Version::V1_7).map_or(&[][..], |b| b.list.as_slice()) {
			known[b.id as usize & 4095] = true;
		}
		known
	});
	known[id as usize & 4095]
}

pub(super) fn state_1_7(state: u16) -> u16 {
	static TABLE: OnceLock<Vec<u16>> = OnceLock::new();
	let table = TABLE.get_or_init(|| {
		(0..=u16::MAX)
			.map(|s| {
				let (id, meta) = block_1_7(s >> 4, (s & 15) as u8);
				id << 4 | meta as u16
			})
			.collect()
	});
	table[state as usize]
}

pub(super) fn item_1_7(id: i16, damage: i16) -> Option<(i16, i16)> {
	Some(match (id, damage) {
		(1, 1..=6) => (1, 0),
		(19, 1) => (19, 0),
		(165, _) => (95, 5),
		(166, _) => (20, 0),
		(167, d) => (96, d),
		(168, 0) => (1, 0),
		(168, 1) => (98, 0),
		(168, _) => (98, 1),
		(169, _) => (89, 0),
		(179, d) => (24, d),
		(180, d) => (128, d),
		(181 | 182, _) => (44, 1),
		(183..=187, d) => (107, d),
		(188..=192, d) => (85, d),
		(409 | 410, _) => (406, 0),
		(411, _) => (365, 0),
		(412, _) => (366, 0),
		(413, _) => (282, 0),
		(414, _) => (375, 0),
		(415, _) => (334, 0),
		(416, _) => (280, 0),
		(423, _) => (363, 0),
		(424, _) => (364, 0),
		(425, _) => (323, 0),
		(427..=431, d) => (324, d),
		(373, 8203 | 8235 | 8267 | 16395 | 16427 | 16459) => (373, 0),
		(383, 30 | 67 | 68 | 101) => (383, 0),
		_ if registry::names(Version::V1_7, "items")
			.is_some_and(|n| n.name(id as u32).is_none()) =>
		{
			(1, 0)
		}
		_ => return None,
	})
}

fn encoding(e: impl std::fmt::Debug) -> Error {
	Error::Encoding(format!("{e:?}"))
}

pub(super) fn compress(tag: &Option<Entry>) -> Result<Option<Vec<u8>>> {
	tag.as_ref()
		.map(nbt::gzip::write)
		.transpose()
		.map_err(encoding)
}

pub(super) fn decompress(data: &Option<Vec<u8>>) -> Result<Option<Entry>> {
	data.as_deref()
		.map(nbt::gzip::read)
		.transpose()
		.map_err(encoding)
}

fn pretty(id: i16) -> String {
	let name = registry::names(Version::V1_8, "items")
		.and_then(|n| n.name(id as u32))
		.unwrap_or("item");
	name.split('_')
		.map(|w| {
			let mut c = w.chars();
			c.next()
				.map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
				.unwrap_or_default()
		})
		.collect::<Vec<_>>()
		.join(" ")
}

pub(super) fn down(v: Slot8) -> Result<Slot7> {
	let (id, s) = match v {
		Slot8::_1 => return Ok(Slot7::_1),
		Slot8::default(id, s) => (id, s),
	};
	let mut tag = s.nbtData;
	let mut mark = Vec::new();
	let (mut new_id, mut damage) = (id, s.itemDamage);
	if let Some((i, d)) = item_1_7(id, s.itemDamage) {
		(new_id, damage) = (i, d);
		mark.push(Entry::new("id", Payload::short(id)));
		mark.push(Entry::new("Damage", Payload::short(s.itemDamage)));
		let c = Entry::root(&mut tag);
		let mut display = match c.take("display") {
			Some(Payload::compound(d)) => d,
			_ => Vec::new(),
		};
		if display.field("Name").is_none() {
			display.push(Entry::new(
				"Name",
				Payload::string(format!("\u{a7}r1.8 {}", pretty(id))),
			));
			mark.push(Entry::new("Named", Payload::byte(1)));
		}
		c.push(Entry::new("display", Payload::compound(display)));
	}
	if id == 387 {
		if let Some(Entry {
			value: Payload::compound(c),
			..
		}) = tag.as_mut()
		{
			if let Some(Payload::list(pages)) = c.take("pages") {
				if let Some(old) = pages.strings() {
					let legacy = old.iter().map(|p| text::to_legacy(p)).collect();
					c.push(Entry::new("pages", ListItems::string(legacy)));
					mark.push(Entry::new("pages", Payload::list(pages)));
				} else {
					c.push(Entry::new("pages", Payload::list(pages)));
				}
			}
		}
	}
	if !mark.is_empty() {
		Entry::root(&mut tag).push(Entry::new(MARK, Payload::compound(mark)));
	}
	Ok(Slot7::default(
		new_id,
		v1_7::slot__default::Ty {
			itemCount: s.itemCount,
			itemDamage: damage,
			nbtData: compress(&tag)?,
		},
	))
}

pub(super) fn up(v: Slot7) -> Result<Slot8> {
	let (mut id, s) = match v {
		Slot7::_1 => return Ok(Slot8::_1),
		Slot7::default(id, s) => (id, s),
	};
	let mut tag = decompress(&s.nbtData)?;
	let mut damage = s.itemDamage;
	if let Some(Entry {
		value: Payload::compound(c),
		..
	}) = tag.as_mut()
	{
		if let Some(Payload::compound(mut mark)) = c.take(MARK) {
			if let Some(Payload::short(i)) = mark.field("id") {
				id = *i;
			}
			if let Some(Payload::short(d)) = mark.field("Damage") {
				damage = *d;
			}
			if mark.field("Named").is_some() {
				if let Some(Payload::compound(mut display)) = c.take("display") {
					display.take("Name");
					if !display.is_empty() {
						c.push(Entry::new("display", Payload::compound(display)));
					}
				}
			}
			if let Some(pages) = mark.take("pages") {
				c.put_last("pages", pages);
			}
		}
		if c.is_empty() {
			tag = None;
		}
	}
	Ok(Slot8::default(
		id,
		v1_8::slot__default::Ty {
			itemCount: s.itemCount,
			itemDamage: damage,
			nbtData: tag,
		},
	))
}

pub(super) fn book_pages_1_8(slot: &mut Slot8) {
	let Slot8::default(_, s) = slot else { return };
	let Some(Entry {
		value: Payload::compound(c),
		..
	}) = s.nbtData.as_mut()
	else {
		return;
	};
	let Some(Payload::list(pages)) = c.take("pages") else {
		return;
	};
	let pages = match pages.strings() {
		Some(p) => List::new(ListItems::string(
			p.iter().map(|p| text::from_legacy(p)).collect(),
		)),
		None => pages,
	};
	c.push(Entry::new("pages", Payload::list(pages)));
}

pub fn slot_1_7(v: Slot8, _cx: &mut Session) -> Result<Slot7> {
	down(v)
}

pub fn legacy_slot_1_8(v: Slot7, _cx: &mut Session) -> Result<Slot8> {
	up(v)
}

pub(super) fn flower_pot_1_7(tag: &mut Option<Entry>) {
	let Some(Entry {
		value: Payload::compound(c),
		..
	}) = tag.as_mut()
	else {
		return;
	};
	if let Some(Payload::string(name)) = c.field("Item") {
		let id = registry::names(Version::V1_8, "items")
			.and_then(|n| n.id(name))
			.unwrap_or(0);
		let id = item_1_7(id as i16, 0).map_or(id as i32, |(i, _)| i as i32);
		c.put_last("Item", Payload::int(id));
	}
}

pub(super) fn flower_pot_1_8(tag: &mut Option<Entry>) {
	let Some(Entry {
		value: Payload::compound(c),
		..
	}) = tag.as_mut()
	else {
		return;
	};
	if let Some(Payload::int(id)) = c.field("Item") {
		let name = registry::names(Version::V1_8, "items")
			.and_then(|n| n.name(*id as u32))
			.unwrap_or("air");
		c.put_last("Item", Payload::string(format!("minecraft:{name}")));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn item(id: i16, damage: i16, tag: Option<Entry>) -> Slot8 {
		Slot8::default(
			id,
			v1_8::slot__default::Ty {
				itemCount: 3,
				itemDamage: damage,
				nbtData: tag,
			},
		)
	}

	#[test]
	fn new_items_are_replaced_and_restored() {
		let original = item(409, 0, None);
		let old = down(original.clone()).unwrap();
		let Slot7::default(id, s) = &old else {
			panic!("{old:?}")
		};
		assert_eq!((*id, s.itemDamage, s.itemCount), (406, 0, 3));
		let tag = decompress(&s.nbtData).unwrap().unwrap();
		let Payload::compound(c) = &tag.value else {
			panic!("{tag:?}")
		};
		let Some(Payload::compound(display)) = c.field("display") else {
			panic!("{c:?}")
		};
		assert_eq!(
			display.field("Name"),
			Some(&Payload::string("\u{a7}r1.8 Prismarine Shard".to_owned()))
		);
		assert_eq!(up(old).unwrap(), original);
	}

	#[test]
	fn plain_items_keep_their_nbt() {
		let tag = Entry::new(
			"",
			vec![Entry::new(
				"ench",
				ListItems::compound(vec![vec![
					Entry::new("id", Payload::short(16)),
					Entry::new("lvl", Payload::short(2)),
				]]),
			)],
		);
		let original = item(276, 5, Some(tag));
		let old = down(original.clone()).unwrap();
		assert!(
			matches!(&old, Slot7::default(276, s) if s.itemDamage == 5 && s.nbtData.as_ref().is_some_and(|g| g.starts_with(&[0x1f, 0x8b])))
		);
		assert_eq!(up(old).unwrap(), original);
		assert_eq!(down(Slot8::_1).unwrap(), Slot7::_1);
	}

	#[test]
	fn blocks_without_1_7_ids_are_replaced() {
		assert_eq!(state_1_7(1 << 4 | 3), 1 << 4);
		assert_eq!(state_1_7(169 << 4), 89 << 4);
		assert_eq!(state_1_7(193 << 4 | 9), 64 << 4 | 9);
		assert_eq!(state_1_7(54 << 4 | 2), 54 << 4 | 2);
	}
}
