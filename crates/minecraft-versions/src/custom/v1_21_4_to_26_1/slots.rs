macro_rules! untrusted {
	($v:ident, $to_slot:ident, $from_slot:ident) => {
		pub fn $to_slot(u: $v::UntrustedSlot::Ty) -> crate::pds::convert::Result<$v::Slot::Ty> {
			use super::util::{decode, encode};
			Ok(match u.ANON {
				$v::UntrustedSlot__ANON::Ty::v0 => $v::Slot::Ty::v0,
				$v::UntrustedSlot__ANON::Ty::default(count, d) => {
					let components = d
						.components
						.iter()
						.map(|c| {
							let mut bytes = encode(&c.r#type, $v::SlotComponentType::serialize)?;
							bytes.extend_from_slice(&c.data);
							decode(&bytes, $v::SlotComponent::deserialize)
						})
						.collect::<crate::pds::convert::Result<Vec<_>>>()?;
					$v::Slot::Ty::default(
						count,
						$v::Slot__default::Ty {
							itemId: d.itemId,
							addedComponentCount: components.len() as i32,
							removedComponentCount: d.removeComponents.len() as i32,
							components,
							removeComponents: d.removeComponents,
						},
					)
				}
			})
		}

		pub fn $from_slot(s: $v::Slot::Ty) -> crate::pds::convert::Result<$v::UntrustedSlot::Ty> {
			use super::util::{decode, encode, read_varint};
			Ok(match s {
				$v::Slot::Ty::v0 => $v::UntrustedSlot::Ty {
					itemCount: 0,
					ANON: $v::UntrustedSlot__ANON::Ty::v0,
				},
				$v::Slot::Ty::default(count, d) => {
					let components = d
						.components
						.iter()
						.map(|c| {
							let bytes = encode(c, $v::SlotComponent::serialize)?;
							let mut at = 0;
							read_varint(&bytes, &mut at)?;
							Ok($v::UntrustedSlotComponent::Ty {
								r#type: decode(&bytes[..at], $v::SlotComponentType::deserialize)?,
								data: bytes[at..].to_vec(),
							})
						})
						.collect::<crate::pds::convert::Result<Vec<_>>>()?;
					$v::UntrustedSlot::Ty {
						itemCount: count,
						ANON: $v::UntrustedSlot__ANON::Ty::default(
							count,
							$v::UntrustedSlot__ANON__default::Ty {
								itemId: d.itemId,
								addedComponentCount: components.len() as i32,
								removedComponentCount: d.removeComponents.len() as i32,
								components,
								removeComponents: d.removeComponents,
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
			v: $a::UntrustedSlot::Ty,
			cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::UntrustedSlot::Ty> {
			let slot = $to_slot(v)?;
			let packet = $a::play_toClient_packet::Ty::set_cursor_item(slot);
			let out = super::shared::via_step(
				$from,
				$to,
				crate::gen::convert::Root::PlayToClient,
				&packet,
				$a::play_toClient_packet::serialize,
				$b::play_toClient_packet::deserialize,
				cx,
			)?;
			match out {
				$b::play_toClient_packet::Ty::set_cursor_item(s) => $from_slot(s),
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
