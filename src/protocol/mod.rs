pub mod handshake;
pub mod login;
mod packet;
pub mod play;
pub mod status;

use std::io::{Read, Write};
use tokio::io;

pub use crate::ext::MinecraftReadExt;
pub use crate::ext::MinecraftWriteExt;
use derive_packetdata::PacketData;
pub use packet::*;

#[derive(Debug, Clone, Copy)]
pub enum State {
	Handshaking,
	Status,
	Login,
	Play,
}
impl PacketData for State {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		Ok(match buf.read_varint()?.ans {
			0 => Self::Handshaking,
			1 => Self::Status,
			2 => Self::Login,
			3 => Self::Play,
			_ => unreachable!(),
		})
	}
	fn write<W: std::io::Write>(&self, buf: &mut W) -> io::Result<()> {
		buf.write_varint(match self {
			State::Handshaking => 0,
			State::Status => 1,
			State::Login => 2,
			State::Play => 3,
		})?;
		Ok(())
	}
}
