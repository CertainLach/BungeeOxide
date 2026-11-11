use std::borrow::Cow;
use std::ops::Range;

use minecraft_versions::gen::play::to_client::packet_v26_1 as owned;
use minecraft_versions::gen::view::play::to_client::packet_v26_1 as view;
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::{convert, index_of, Root, Session};
use nbt::gen::view::{list_0 as items, payload};

fn borrowed_strings(p: &payload::Ty, within: &Range<*const u8>) -> usize {
	let check = |s: &Cow<str>| match s {
		Cow::Borrowed(s) => {
			assert!(within.contains(&s.as_ptr()));
			1
		}
		Cow::Owned(_) => 0,
	};
	match p {
		payload::Ty::String(s) => check(s),
		payload::Ty::Compound(c) => c
			.iter()
			.map(|e| check(&e.name) + borrowed_strings(&e.value, within))
			.sum(),
		payload::Ty::List(l) => match &l.items {
			items::Ty::String(v) => v.iter().map(check).sum(),
			items::Ty::Compound(v) => v
				.iter()
				.flatten()
				.map(|e| check(&e.name) + borrowed_strings(&e.value, within))
				.sum(),
			_ => 0,
		},
		_ => 0,
	}
}

fn capture() -> Vec<Vec<u8>> {
	let bytes = std::fs::read(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/tests/data/1.8-toClient.bin"
	))
	.unwrap();
	let mut packets = Vec::new();
	let mut at = 0;
	while at + 4 <= bytes.len() {
		let len = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
		packets.push(bytes[at + 4..at + 4 + len].to_vec());
		at += 4 + len;
	}
	packets
}

#[test]
fn latest_views_match_owned_packets() {
	let (from, to) = (index_of("1.8").unwrap(), index_of("26.1").unwrap());
	let mut cx = Session::new();
	let mut checked = 0;
	let mut chunks = 0;
	let mut texts = 0;
	for p in capture() {
		let Ok(out) = convert(Root::PlayToClient, from, to, &p, &mut cx) else {
			continue;
		};
		for bytes in out {
			let mut input = Input::new(&bytes);
			let parsed = view::deserialize(&mut input).unwrap();
			assert_eq!(input.remaining(), 0, "{}", view::name(&parsed));
			let mut written = Output::new();
			view::serialize(&parsed, &mut written).unwrap();
			assert_eq!(written.into_inner(), bytes, "{}", view::name(&parsed));
			assert_eq!(view::size_of(&parsed), bytes.len());

			let full = owned::deserialize(&mut Input::new(&bytes)).unwrap();
			let adapted = view::view(&full);
			assert_eq!(adapted, parsed, "{}", view::name(&parsed));

			let mut taken = full.clone();
			let mask = match &taken {
				owned::Ty::MapChunk(c) => Some(c.sky_light_mask.as_ptr()),
				_ => None,
			};
			let moved = view::into_view(&mut taken);
			assert_eq!(moved, parsed, "{}", view::name(&parsed));
			if let (Some(mask), view::Ty::MapChunk(c)) = (mask, &moved) {
				assert_eq!(c.sky_light_mask.as_ptr(), mask);
			}
			drop(moved);

			if let view::Ty::MapChunk(c) = &parsed {
				assert!(bytes.as_ptr_range().contains(&c.chunk_data.as_ptr()));
				let owned::Ty::MapChunk(o) = &full else {
					unreachable!()
				};
				let view::Ty::MapChunk(a) = &adapted else {
					unreachable!()
				};
				assert_eq!(a.chunk_data.as_ptr(), o.chunk_data.as_ptr());
				chunks += 1;
			}
			if let view::Ty::SystemChat(c) = &parsed {
				texts += borrowed_strings(&c.content.value, &bytes.as_ptr_range());
			}
			checked += 1;
		}
	}
	assert!(checked > 200, "{checked}");
	assert!(chunks > 0);
	assert!(texts > 0);
}
