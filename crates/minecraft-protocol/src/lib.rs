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
pub mod pds;
pub mod pds_native;

#[cfg(test)]
mod tests {
	use crate::pds::{Input, Output};

	#[test]
	fn roundtrip_vec3f() {
		use crate::gen::vec3f as t;
		let v = t::Ty {
			x: 1.5,
			y: -2.0,
			z: 1024.25,
		};
		let mut out = Output::new();
		t::serialize(&v, &mut out).unwrap();
		let bytes = out.into_inner();
		assert_eq!(bytes.len(), t::size_of(&v));
		let v2 = t::deserialize(&mut Input::new(&bytes)).unwrap();
		assert_eq!((v.x, v.y, v.z), (v2.x, v2.y, v2.z));
	}

	#[test]
	fn roundtrip_position_bitfield() {
		use crate::gen::position as t;
		let v = t::Ty {
			x: -123456,
			z: 654321,
			y: -42,
		};
		let mut out = Output::new();
		t::serialize(&v, &mut out).unwrap();
		let bytes = out.into_inner();
		assert_eq!(bytes.len(), 8);
		assert_eq!(bytes.len(), t::size_of(&v));
		let v2 = t::deserialize(&mut Input::new(&bytes)).unwrap();
		assert_eq!((v.x, v.z, v.y), (v2.x, v2.z, v2.y));
	}

	#[test]
	fn roundtrip_inline_tag_mapper() {
		// soundSource is a mapper union: self-contained, reads its own varint tag,
		// all arms are the single deduplicated void type.
		use crate::gen::sound_source as t;
		// void arms are value-less enum variants
		let v = t::Ty::Hostile;
		let mut out = Output::new();
		t::serialize(&v, &mut out).unwrap();
		let bytes = out.into_inner();
		assert_eq!(bytes, vec![5]); // hostile has `match: 5`
		assert_eq!(bytes.len(), t::size_of(&v));
		let v2 = t::deserialize(&mut Input::new(&bytes)).unwrap();
		assert!(matches!(v2, t::Ty::Hostile));
	}

	#[test]
	fn roundtrip_merged_mapper_switch() {
		// SlotComponent_57: a mapper+switch merged into one self-contained inline-tag
		// union (mixed void / payload arms).
		use crate::gen::slot_component_30 as t;
		let v = t::Ty::Hidden; // tag 1, void payload
		let mut out = Output::new();
		t::serialize(&v, &mut out).unwrap();
		assert_eq!(out.into_inner(), vec![1]);
		let v2 = t::deserialize(&mut Input::new(&[1])).unwrap();
		assert!(matches!(v2, t::Ty::Hidden));
	}

	#[test]
	fn scalar_tag_merged_into_union() {
		use crate::gen::play::to_client::packet_hide_message as t;
		let v = t::Ty::V0(vec![7u8; 256]);
		let mut out = Output::new();
		t::serialize(&v, &mut out).unwrap();
		let bytes = out.into_inner();
		assert_eq!(bytes[0], 0);
		assert_eq!(bytes.len(), 257);
		assert_eq!(t::deserialize(&mut Input::new(&bytes)).unwrap(), v);

		let unknown = t::deserialize(&mut Input::new(&[5])).unwrap();
		assert_eq!(unknown, t::Ty::Default(5));
	}

	#[test]
	fn option_tag_derived_from_union() {
		use crate::gen::play::to_client::packet_scoreboard_score as t;
		use crate::gen::play::to_client::packet_scoreboard_score_38 as styling;
		let end = nbt::Tag {
			r#type: 0,
			value: nbt::Payload::End,
		};
		let v = t::Ty {
			item_name: "a".into(),
			score_name: "b".into(),
			value: 3,
			display_name: None,
			styling: styling::Ty::V1(end),
		};
		let mut out = Output::new();
		t::serialize(&v, &mut out).unwrap();
		let bytes = out.into_inner();
		assert_eq!(bytes, [1, b'a', 1, b'b', 3, 0, 1, 1, 0]);
		assert_eq!(t::size_of(&v), bytes.len());
		assert_eq!(t::deserialize(&mut Input::new(&bytes)).unwrap(), v);

		let none = t::Ty {
			styling: styling::Ty::Default(None),
			..v
		};
		let mut out = Output::new();
		t::serialize(&none, &mut out).unwrap();
		assert_eq!(out.into_inner(), [1, b'a', 1, b'b', 3, 0, 0]);
	}

	#[test]
	fn nbt_text_component() {
		use crate::pds_native::{pds_anonOptionalNbt, pds_anonymousNbt};
		let nbt: &[u8] = &[
			10, 8, 0, 4, b't', b'e', b'x', b't', 0, 2, b'h', b'i', 9, 0, 5, b'e', b'x', b't', b'r',
			b'a', 10, 0, 0, 0, 1, 0, 0,
		];
		let mut bytes = nbt.to_vec();
		bytes.push(0xff);
		let mut input = Input::new(&bytes);
		let v = pds_anonymousNbt::deserialize(&mut input).unwrap();
		assert_eq!(input.offset(), nbt.len());
		let nbt::Payload::Compound(entries) = &v.value else {
			panic!("{v:?}")
		};
		assert_eq!(entries[0].name, "text");
		let mut out = Output::new();
		pds_anonymousNbt::serialize(&v, &mut out).unwrap();
		assert_eq!(out.into_inner(), nbt);

		let empty = pds_anonOptionalNbt::deserialize(&mut Input::new(&[0, 0xff])).unwrap();
		assert!(matches!(empty.value, nbt::Payload::End));
	}

	#[test]
	fn top_bit_set_terminated_equipment() {
		use crate::gen::play::to_client::packet_entity_equipment as t;
		let bytes = [7, 0x80, 0, 0x85, 0, 3, 0];
		let mut input = Input::new(&bytes);
		let v = t::deserialize(&mut input).unwrap();
		assert_eq!(input.offset(), bytes.len());
		let slots: Vec<i8> = v.equipments.iter().map(|e| e.slot).collect();
		assert_eq!(slots, [0, 5, 3]);
		let mut out = Output::new();
		t::serialize(&v, &mut out).unwrap();
		assert_eq!(out.into_inner(), bytes);
		assert_eq!(t::size_of(&v), bytes.len());
	}

	#[test]
	fn lp_vec3_real_server_payloads() {
		use crate::pds_native::pds_lpVec3 as t;
		for hex in [
			"00",
			"f9ff7ffeebed",
			"59e7800cebed",
			"51e880011541",
			"09e98000d8fd",
		] {
			let bytes: Vec<u8> = (0..hex.len())
				.step_by(2)
				.map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
				.collect();
			let mut input = Input::new(&bytes);
			let v = t::deserialize(&mut input).unwrap();
			assert_eq!(input.offset(), bytes.len());
			let mut out = Output::new();
			t::serialize(&v, &mut out).unwrap();
			assert_eq!(out.into_inner(), bytes, "{hex}");
			assert_eq!(t::size_of(&v), bytes.len());
		}
		let v = t::deserialize(&mut Input::new(&[0xf9, 0xff, 0x7f, 0xfe, 0xeb, 0xed])).unwrap();
		let [x, y, z] = v.to_f64();
		assert!((y + 0.0784).abs() < 0.001, "{y}");
		assert!(x.abs() < 1.0 && z.abs() < 1.0);
	}

	#[test]
	fn roundtrip_handshake() {
		use crate::gen::handshaking::to_server::packet_set_protocol as t;
		let v = t::Ty {
			protocol_version: 774,
			server_host: "localhost".to_owned(),
			server_port: 25565,
			next_state: 2,
		};
		let mut out = Output::new();
		t::serialize(&v, &mut out).unwrap();
		let bytes = out.into_inner();
		assert_eq!(bytes.len(), t::size_of(&v));
		let v2 = t::deserialize(&mut Input::new(&bytes)).unwrap();
		assert_eq!(v.protocol_version, v2.protocol_version);
		assert_eq!(v.server_host, v2.server_host);
		assert_eq!(v.server_port, v2.server_port);
	}
}
