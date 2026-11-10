mod bits;
mod formats;
mod model;
mod packets;
mod steps;
mod tables;

pub(crate) use formats::padding;
pub use model::*;
pub use packets::{
	decode, decode_in, encode_bulk, encode_chunk, encode_light, encode_unload, ids, Ids, Packet,
};
pub use tables::{composed, probe, probe_column, settle, step_table};

use crate::gen::convert::{step, Carrier, Root, VERSIONS};
use crate::pds::convert::Result;
use crate::Session;

pub mod v {
	use crate::gen::convert::Version;

	pub const V1_7: usize = Version::V1_7 as usize;
	pub const V1_8: usize = Version::V1_8 as usize;
	pub const V1_9: usize = Version::V1_9 as usize;
	pub const V1_9_2: usize = Version::V1_9_2 as usize;
	pub const V1_9_4: usize = Version::V1_9_4 as usize;
	pub const V1_12_2: usize = Version::V1_12_2 as usize;
	pub const V1_13: usize = Version::V1_13 as usize;
	pub const V1_13_2: usize = Version::V1_13_2 as usize;
	pub const V1_14: usize = Version::V1_14 as usize;
	pub const V1_14_4: usize = Version::V1_14_4 as usize;
	pub const V1_15: usize = Version::V1_15 as usize;
	pub const V1_15_2: usize = Version::V1_15_2 as usize;
	pub const V1_16: usize = Version::V1_16 as usize;
	pub const V1_16_1: usize = Version::V1_16_1 as usize;
	pub const V1_16_2: usize = Version::V1_16_2 as usize;
	pub const V1_17: usize = Version::V1_17 as usize;
	pub const V1_17_1: usize = Version::V1_17_1 as usize;
	pub const V1_18: usize = Version::V1_18 as usize;
	pub const V1_18_2: usize = Version::V1_18_2 as usize;
	pub const V1_19: usize = Version::V1_19 as usize;
	pub const V1_19_2: usize = Version::V1_19_2 as usize;
	pub const V1_19_3: usize = Version::V1_19_3 as usize;
	pub const V1_19_4: usize = Version::V1_19_4 as usize;
	pub const V1_20: usize = Version::V1_20 as usize;
	pub const V1_20_2: usize = Version::V1_20_2 as usize;
	pub const V1_20_3: usize = Version::V1_20_3 as usize;
	pub const V1_20_5: usize = Version::V1_20_5 as usize;
	pub const V1_21_1: usize = Version::V1_21_1 as usize;
	pub const V1_21_3: usize = Version::V1_21_3 as usize;
	pub const V1_21_4: usize = Version::V1_21_4 as usize;
	pub const V1_21_5: usize = Version::V1_21_5 as usize;
	pub const V1_21_8: usize = Version::V1_21_8 as usize;
	pub const V1_21_9: usize = Version::V1_21_9 as usize;
	pub const V26_1: usize = Version::V26_1 as usize;
}

pub enum Virtual {
	Chunk(Box<Column>),
	Bulk(Vec<Column>, bool),
}

pub fn virtual_start(
	root: Root,
	from: usize,
	to: usize,
	data: &[u8],
	cx: &mut Session,
) -> Result<Option<Vec<Carrier>>> {
	if root != Root::PlayToClient || from == to || from < v::V1_7 || from.max(to) >= VERSIONS.len()
	{
		return Ok(None);
	}
	let known = ids(from);
	let id = packets::packet_id(data);
	if id.is_none() || (id != known.chunk && id != known.bulk) {
		return Ok(None);
	}
	Ok(
		match decode_in(from, data, crate::custom::caves_known_sections(cx)) {
			Ok(Some(Packet::Chunk(col))) => {
				Some(vec![Carrier::Virtual(Virtual::Chunk(Box::new(col)))])
			}
			Ok(Some(Packet::Bulk(columns, sky))) => {
				Some(vec![Carrier::Virtual(Virtual::Bulk(columns, sky))])
			}
			_ => None,
		},
	)
}

pub(crate) fn deliver(
	root: Root,
	version: usize,
	model: Virtual,
	cx: &mut Session,
) -> Vec<crate::session::Outgoing> {
	let target = if root.to_client() {
		cx.client_version()
	} else {
		cx.server_version()
	};
	let (to, packets) = match target {
		Some(to) if to < VERSIONS.len() && to != version => (
			to,
			crate::gen::convert::carry(root, version, to, vec![Carrier::Virtual(model)], cx),
		),
		_ => (version, virtual_finish(root, version, model, cx)),
	};
	packets
		.unwrap_or_default()
		.into_iter()
		.map(|data| crate::session::Outgoing {
			root,
			version: to,
			data,
		})
		.collect()
}

pub(crate) fn legacy_column(
	x: i32,
	z: i32,
	full: bool,
	version: usize,
	raw: &[u8],
	mask: u32,
	add: u32,
	sky: bool,
) -> Result<Column> {
	let mut col = Column::new(x, z, full, version);
	formats::decode_7(&mut col, raw, formats::Bitmask { mask, add }, sky)?;
	Ok(col)
}

pub(crate) fn legacy_data(col: &Column) -> (u32, u32, bool, Vec<u8>) {
	let (m, raw) = formats::encode_7(col);
	(m.mask, m.add, formats::has_sky(col), raw)
}

pub fn virtual_wire(
	root: Root,
	at: usize,
	next: usize,
	data: &[u8],
	cx: &mut Session,
) -> Result<Option<Vec<Carrier>>> {
	match (root, at, next) {
		(Root::PlayToClient, v::V1_17, v::V1_16_2) => crate::custom::nether_wire_1_16_2(data, cx),
		_ => Ok(None),
	}
}

pub fn virtual_step(
	root: Root,
	from: usize,
	to: usize,
	model: Virtual,
	cx: &mut Session,
) -> Result<Vec<Carrier>> {
	if to < v::V1_7 {
		let mut out = Vec::new();
		for bytes in virtual_finish(root, from, model, cx)? {
			out.extend(
				step(root, from, to, &bytes, cx)?
					.into_iter()
					.map(Carrier::Wire),
			);
		}
		return Ok(out);
	}
	match model {
		Virtual::Chunk(col) => steps::advance(from, to, *col, cx),
		Virtual::Bulk(columns, sky) => steps::advance_bulk(from, to, columns, sky),
	}
}

pub fn virtual_finish(
	_root: Root,
	to: usize,
	model: Virtual,
	_cx: &mut Session,
) -> Result<Vec<Vec<u8>>> {
	match model {
		Virtual::Chunk(mut col) => {
			settle(&mut col, to);
			encode_chunk(to, &col)
		}
		Virtual::Bulk(mut columns, sky) => {
			for c in &mut columns {
				settle(c, to);
			}
			Ok(vec![encode_bulk(to, &columns, sky)?])
		}
	}
}
