pub mod handshake;
pub mod login;
mod packet;
pub mod play;
pub mod status;

use std::io::Read;
use tokio::io;

pub use crate::ext::MinecraftReadExt;
pub use packet::*;
#[derive(Debug)]
pub enum State {
	Handshaking,
	Status,
	Login,
}
impl PacketData for State {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		Ok(match buf.read_varint()?.ans {
			0 => Self::Handshaking,
			1 => Self::Status,
			2 => Self::Login,
			_ => todo!(),
		})
	}
	fn write<W: std::io::Write>(&self, buf: &mut W) -> io::Result<()> {
        todo!()
    }
}
