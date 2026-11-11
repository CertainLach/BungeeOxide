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

use crate::gen::convert::Version;
use crate::gen::convert::{step, Carrier, Root};
use crate::pds::convert::Result;
use crate::Session;

pub enum Virtual {
	Chunk(Box<Column>),
	Bulk(Vec<Column>, bool),
}

pub fn virtual_start(
	root: Root,
	from: Version,
	to: Version,
	data: &[u8],
	cx: &mut Session,
) -> Result<Option<Vec<Carrier>>> {
	if root != Root::PlayToClient || from == to || from < Version::V1_7 {
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
	version: Version,
	model: Virtual,
	cx: &mut Session,
) -> Vec<crate::session::Outgoing> {
	let target = if root.to_client() {
		cx.client_version()
	} else {
		cx.server_version()
	};
	let (to, packets) = match target {
		Some(to) if to != version => (
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
	version: Version,
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
	at: Version,
	next: Version,
	data: &[u8],
	cx: &mut Session,
) -> Result<Option<Vec<Carrier>>> {
	match (root, at, next) {
		(Root::PlayToClient, Version::V1_17, Version::V1_16_2) => {
			crate::custom::nether_wire_1_16_2(data, cx)
		}
		_ => Ok(None),
	}
}

pub fn virtual_step(
	root: Root,
	from: Version,
	to: Version,
	model: Virtual,
	cx: &mut Session,
) -> Result<Vec<Carrier>> {
	if to < Version::V1_7 {
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
	to: Version,
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
