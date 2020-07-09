use super::*;
use byteorder::{BigEndian, ReadBytesExt};
use std::io::Read;

#[derive(Debug)]
pub struct Handshake {
	pub protocol: i32,
	pub address: String,
	pub port: i16,
	pub next_state: State,
}
impl Packet for Handshake {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		Ok(Handshake {
			protocol: buf.read_varint()?.ans,
			address: buf.read_string(64)?,
			port: buf.read_i16::<BigEndian>()?,
			next_state: State::read(buf)?,
		})
	}
}
