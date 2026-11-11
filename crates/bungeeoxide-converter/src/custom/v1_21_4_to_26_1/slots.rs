macro_rules! untrusted {
	($v:ident, $to_slot:ident, $from_slot:ident) => {
		pub fn $to_slot(u: $v::untrusted_slot::Ty) -> crate::pds::convert::Result<$v::slot::Ty> {
			use super::util::{decode, encode};
			Ok(match u.anon {
				$v::untrusted_slot_anon::Ty::V0 => $v::slot::Ty::V0,
				$v::untrusted_slot_anon::Ty::Default(count, d) => {
					let components = d
						.components
						.iter()
						.map(|c| {
							let mut bytes = encode(&c.r#type, $v::slot_component_type::serialize)?;
							bytes.extend_from_slice(&c.data);
							decode(&bytes, $v::slot_component::deserialize)
						})
						.collect::<crate::pds::convert::Result<Vec<_>>>()?;
					$v::slot::Ty::Default(
						count,
						$v::slot_default::Ty {
							item_id: d.item_id,
							added_component_count: components.len() as i32,
							removed_component_count: d.remove_components.len() as i32,
							components,
							remove_components: d.remove_components,
						},
					)
				}
			})
		}

		pub fn $from_slot(s: $v::slot::Ty) -> crate::pds::convert::Result<$v::untrusted_slot::Ty> {
			use super::util::{decode, encode, read_varint};
			Ok(match s {
				$v::slot::Ty::V0 => $v::untrusted_slot::Ty {
					item_count: 0,
					anon: $v::untrusted_slot_anon::Ty::V0,
				},
				$v::slot::Ty::Default(count, d) => {
					let components = d
						.components
						.iter()
						.map(|c| {
							let bytes = encode(c, $v::slot_component::serialize)?;
							let mut at = 0;
							read_varint(&bytes, &mut at)?;
							Ok($v::untrusted_slot_component::Ty {
								r#type: decode(&bytes[..at], $v::slot_component_type::deserialize)?,
								data: bytes[at..].to_vec(),
							})
						})
						.collect::<crate::pds::convert::Result<Vec<_>>>()?;
					$v::untrusted_slot::Ty {
						item_count: count,
						anon: $v::untrusted_slot_anon::Ty::Default(
							count,
							$v::untrusted_slot_anon_default::Ty {
								item_id: d.item_id,
								added_component_count: components.len() as i32,
								removed_component_count: d.remove_components.len() as i32,
								components,
								remove_components: d.remove_components,
							},
						),
					}
				}
			})
		}
	};
}

macro_rules! untrusted_step {
	($name:ident: $a:ident => $b:ident, $from:path => $to:path, $to_slot:path, $from_slot:path) => {
		pub fn $name(
			v: $a::untrusted_slot::Ty,
			cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::untrusted_slot::Ty> {
			let slot = $to_slot(v)?;
			let packet = $a::play_to_client_packet::Ty::SetCursorItem(slot);
			let out = super::shared::via_step(
				$from,
				$to,
				crate::gen::convert::Root::PlayToClient,
				&packet,
				$a::play_to_client_packet::serialize,
				$b::play_to_client_packet::deserialize,
				cx,
			)?;
			match out {
				$b::play_to_client_packet::Ty::SetCursorItem(s) => $from_slot(s),
				_ => {
					crate::pds::convert::unconvertible("set_cursor_item turned into another packet")
				}
			}
		}
	};
}

pub(crate) use untrusted_step;

use crate::gen::convert::{v1_21_11, v1_21_5, v1_21_6, v1_21_8, v1_21_9, v26_1};

untrusted!(v1_21_5, late_to_slot_1_21_5, late_to_untrusted_1_21_5);
untrusted!(v1_21_6, late_to_slot_1_21_6, late_to_untrusted_1_21_6);
untrusted!(v1_21_8, late_to_slot_1_21_8, late_to_untrusted_1_21_8);
untrusted!(v1_21_9, late_to_slot_1_21_9, late_to_untrusted_1_21_9);
untrusted!(v1_21_11, late_to_slot_1_21_11, late_to_untrusted_1_21_11);
untrusted!(v26_1, late_to_slot_26_1, late_to_untrusted_26_1);
