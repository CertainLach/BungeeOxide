use super::*;
use std::io::Read;

#[derive(Debug)]
pub struct Handshake {
	pub protocol: VarInt,
	pub address: String,
	pub port: i16,
	pub next_state: State,
}
impl Packet for Handshake {
	const ID: i32 = 0;
}
impl PacketData for Handshake {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		Ok(Handshake {
			protocol: VarInt::read(buf)?,
			address: String::read(buf)?,
			port: i16::read(buf)?,
			next_state: State::read(buf)?,
		})
	}
	fn write<W: std::io::Write>(&self, buf: &mut W) -> io::Result<()> {
		self.protocol.write(buf)?;
		self.address.write(buf)?;
		self.port.write(buf)?;
		self.next_state.write(buf)?;
		Ok(())
	}
}
