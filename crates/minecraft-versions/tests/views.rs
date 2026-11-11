use std::borrow::Cow;
use std::ops::Range;

use minecraft_versions::gen::pds_play_toClient_packet_v26_1 as owned;
use minecraft_versions::gen::view::pds_play_toClient_packet_v26_1 as view;
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::{convert, index_of, Root, Session};
use nbt::gen::view::{pds_List_0 as items, pds_Payload as payload};

fn borrowed_strings(p: &payload::Ty, within: &Range<*const u8>) -> usize {
	let check = |s: &Cow<str>| match s {
		Cow::Borrowed(s) => {
			assert!(within.contains(&s.as_ptr()));
			1
		}
		Cow::Owned(_) => 0,
	};
	match p {
		payload::Ty::string(s) => check(s),
		payload::Ty::compound(c) => c
			.iter()
			.map(|e| check(&e.name) + borrowed_strings(&e.value, within))
			.sum(),
		payload::Ty::list(l) => match &l.items {
			items::Ty::string(v) => v.iter().map(check).sum(),
			items::Ty::compound(v) => v
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
				owned::Ty::map_chunk(c) => Some(c.skyLightMask.as_ptr()),
				_ => None,
			};
			let moved = view::into_view(&mut taken);
			assert_eq!(moved, parsed, "{}", view::name(&parsed));
			if let (Some(mask), view::Ty::map_chunk(c)) = (mask, &moved) {
				assert_eq!(c.skyLightMask.as_ptr(), mask);
			}
			drop(moved);

			if let view::Ty::map_chunk(c) = &parsed {
				assert!(bytes.as_ptr_range().contains(&c.chunkData.as_ptr()));
				let owned::Ty::map_chunk(o) = &full else {
					unreachable!()
				};
				let view::Ty::map_chunk(a) = &adapted else {
					unreachable!()
				};
				assert_eq!(a.chunkData.as_ptr(), o.chunkData.as_ptr());
				chunks += 1;
			}
			if let view::Ty::system_chat(c) = &parsed {
				texts += borrowed_strings(&c.content.value, &bytes.as_ptr_range());
			}
			checked += 1;
		}
	}
	assert!(checked > 200, "{checked}");
	assert!(chunks > 0);
	assert!(texts > 0);
}
