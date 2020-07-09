use byteorder::{BigEndian, ReadBytesExt};
use std::io::{Read, Result};
use protool::*;

pub struct Handshake {
	pub version: i32,
	pub host: String,
	pub port: i16,
	pub protocol: i32,
}

impl Packet for Handshake {
	fn read<R: Read>(buf: &mut R) -> Result<Self> {
		let version = buf.read_varint().unwrap().ans;
		let host = buf.read_string(255).unwrap();
		let port = buf.read_i16::<BigEndian>().unwrap();
		let protocol = buf.read_varint().unwrap().ans;
		Ok(Self {
			version,
			host,
			port,
			protocol,
		})
	}
}