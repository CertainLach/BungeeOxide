#![allow(
	non_camel_case_types,
	non_snake_case,
	dead_code,
	unused_imports,
	unused_variables,
	unused_parens,
	unreachable_patterns,
	clippy::all
)]

pub mod gen;
pub mod gzip;
pub mod json;
pub mod natives;
pub mod pds_native;
pub mod snbt;
mod value;
pub mod pds {
	pub use pds_runtime::*;
}

pub use value::{CompoundExt, CompoundVecExt, PayloadMut};

pub use gen::pds_Compound::Ty as Compound;
pub use gen::pds_Entry::Ty as Entry;
pub use gen::pds_List::Ty as List;
pub use gen::pds_List_0::Ty as ListItems;
pub use gen::pds_Payload::Ty as Payload;
pub use gen::pds_Tag::Ty as Tag;

#[cfg(test)]
mod tests {
	use crate::gen::pds_Tag;
	use crate::pds::{Input, Output};
	use crate::pds_native::pds_string;
	use crate::{Entry, ListItems, Payload, Tag};

	fn roundtrip(bytes: &[u8]) -> Tag {
		let mut input = Input::new(bytes);
		let tag = pds_Tag::deserialize(&mut input).unwrap();
		assert_eq!(input.offset(), bytes.len());
		let mut out = Output::new();
		pds_Tag::serialize(&tag, &mut out).unwrap();
		assert_eq!(out.into_inner(), bytes);
		assert_eq!(pds_Tag::size_of(&tag), bytes.len());
		tag
	}

	#[test]
	fn compound_with_nested_list() {
		let tag = roundtrip(&[
			10, 8, 0, 4, b't', b'e', b'x', b't', 0, 2, b'h', b'i', 9, 0, 5, b'e', b'x', b't', b'r',
			b'a', 10, 0, 0, 0, 1, 0, 11, 0, 1, b'n', 0, 0, 0, 2, 0, 0, 0, 1, 0xff, 0xff, 0xff,
			0xff, 0,
		]);
		let Payload::compound(entries) = &tag.value else {
			panic!("{tag:?}")
		};
		let [Entry {
			name: text,
			value: Payload::string(hi),
			..
		}, Entry {
			name: extra,
			value: Payload::list(list),
			..
		}, Entry {
			value: Payload::intArray(ints),
			..
		}] = entries.as_slice()
		else {
			panic!("{entries:?}")
		};
		assert_eq!(
			(text.as_str(), hi.as_str(), extra.as_str()),
			("text", "hi", "extra")
		);
		assert!(matches!(&list.items, ListItems::compound(c) if c.len() == 1 && c[0].is_empty()));
		assert_eq!(ints, &vec![1, -1]);
	}

	#[test]
	fn empty_root_is_end() {
		assert!(matches!(roundtrip(&[0]).value, Payload::end));
	}

	#[test]
	fn deep_nesting_fails_past_stack_budget() {
		use crate::pds::{stack, Error};
		let nested = |depth: usize| {
			let mut bytes = vec![9];
			for _ in 0..depth {
				bytes.extend([9, 0, 0, 0, 1]);
			}
			bytes.extend([0, 0, 0, 0, 0]);
			bytes
		};
		let decode = |bytes: &[u8]| {
			stack::budget(256 << 10, || pds_Tag::deserialize(&mut Input::new(bytes)))
		};
		assert!(decode(&nested(8)).is_ok());
		assert!(matches!(decode(&nested(1 << 20)), Err(Error::TooDeep)));
		let snbt = "[".repeat(1 << 20);
		assert!(stack::budget(256 << 10, || crate::snbt::parse(&snbt)).is_none());
	}

	#[test]
	fn modified_utf8() {
		let s = "a\u{0}\u{e9}\u{1f600}";
		let bytes = pds_string::encode(s);
		assert_eq!(
			bytes,
			[0x61, 0xc0, 0x80, 0xc3, 0xa9, 0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80]
		);
		assert_eq!(pds_string::decode(&bytes).unwrap(), s);
	}

	#[test]
	fn views_borrow_strings_and_byte_arrays() {
		use crate::gen::view::{pds_Payload as payload, pds_Tag as tag};
		use std::borrow::Cow;
		let bytes = [
			10, 8, 0, 4, b't', b'e', b'x', b't', 0, 2, b'h', b'i', 7, 0, 1, b'b', 0, 0, 0, 2, 5,
			0xfb, 8, 0, 1, b'n', 0, 2, 0xc0, 0x80, 0,
		];
		let range = bytes.as_ptr_range();
		let parsed = tag::deserialize(&mut Input::new(&bytes)).unwrap();
		let payload::Ty::compound(entries) = &parsed.value else {
			panic!("{parsed:?}")
		};
		let [text, blob, nul] = entries.as_slice() else {
			panic!("{entries:?}")
		};
		let (Cow::Borrowed(name), payload::Ty::string(Cow::Borrowed(hi))) = (&text.name, &text.value)
		else {
			panic!("{text:?}")
		};
		assert_eq!((*name, *hi), ("text", "hi"));
		assert!(range.contains(&hi.as_ptr()));
		let payload::Ty::byteArray(b) = &blob.value else {
			panic!("{blob:?}")
		};
		assert_eq!(*b, &[5, -5]);
		assert!(range.contains(&b.as_ptr().cast()));
		assert!(matches!(&nul.value, payload::Ty::string(Cow::Owned(s)) if s == "\0"));

		let owned = roundtrip(&bytes);
		assert_eq!(tag::view(&owned), parsed);
		let mut out = Output::new();
		tag::serialize(&parsed, &mut out).unwrap();
		assert_eq!(out.into_inner(), bytes);
		assert_eq!(tag::size_of(&parsed), bytes.len());
	}

	#[test]
	fn into_view_takes_arrays_and_borrows_strings() {
		use crate::gen::view::{pds_Payload as payload, pds_Tag as tag};
		use std::borrow::Cow;
		let bytes = [
			10, 12, 0, 1, b'l', 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 8, 0, 1,
			b's', 0, 2, b'h', b'i', 0,
		];
		let mut owned = roundtrip(&bytes);
		let expected = tag::deserialize(&mut Input::new(&bytes)).unwrap();
		let Payload::compound(entries) = &owned.value else {
			panic!("{owned:?}")
		};
		let Payload::longArray(longs) = &entries[0].value else {
			panic!("{entries:?}")
		};
		let (longs, text) = (longs.as_ptr(), entries[1].name.as_ptr());
		let moved = tag::into_view(&mut owned);
		assert_eq!(moved, expected);
		let payload::Ty::compound(entries) = &moved.value else {
			panic!("{moved:?}")
		};
		assert!(matches!(&entries[0].value, payload::Ty::longArray(v) if v.as_ptr() == longs));
		assert!(matches!(&entries[1].name, Cow::Borrowed(s) if s.as_ptr() == text));
		let Payload::compound(left) = &owned.value else {
			unreachable!()
		};
		assert!(matches!(&left[0].value, Payload::longArray(v) if v.is_empty()));
	}

	#[test]
	fn views_keep_the_stack_guard() {
		use crate::gen::view::pds_Tag as tag;
		use crate::pds::{stack, Error};
		let mut bytes = vec![9];
		for _ in 0..1 << 20 {
			bytes.extend([9, 0, 0, 0, 1]);
		}
		bytes.extend([0, 0, 0, 0, 0]);
		let r = stack::budget(256 << 10, || tag::deserialize(&mut Input::new(&bytes)).map(drop));
		assert!(matches!(r, Err(Error::TooDeep)));
	}
}
