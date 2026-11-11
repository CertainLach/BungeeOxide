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
}
