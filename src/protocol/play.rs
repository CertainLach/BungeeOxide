use super::*;
use byteorder::{BigEndian, ReadBytesExt};

pub struct JoinGame {
	pub entity_id: i32,
	pub game_mode: u8,
	pub dimension: i32,
	pub difficulty: u8,
	pub max_players: u8,
	pub level_type: String,
	pub reduced_debug_info: bool,
}

impl Packet for JoinGame {
	const ID: i32 = 0x26;
}
impl PacketData for JoinGame {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		let entity_id = buf.read_i32::<BigEndian>()?;
		let game_mode = buf.read_u8()?;
		let dimension = buf.read_i32::<BigEndian>()?;
		let difficulty = buf.read_u8()?;
		let max_players = buf.read_u8()?;
		let level_type = buf.read_string(32767)?;
		let reduced_debug_info = buf.read_u8()? == 1;
		Ok(Self {
			entity_id,
			game_mode,
			dimension,
			difficulty,
			max_players,
			level_type,
			reduced_debug_info,
		})
	}
	fn write<W: std::io::Write>(&self, buf: &mut W) -> io::Result<()> {
		todo!()
	}
}

#[derive(Debug)]
pub struct Chat {
	message: String,
	position: u8,
}
impl Packet for Chat {
	const ID: i32 = 0x0F;
}
impl PacketData for Chat {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		let message = buf.read_string(256)?;
		let position = buf.read_u8()?;
		Ok(Self { message, position })
	}
	fn write<W: std::io::Write>(&self, buf: &mut W) -> io::Result<()> {
        todo!()
    }
}

#[derive(Debug)]
pub struct KeepAlive {
	pub random_id: i16,
}
impl Packet for KeepAlive {
	const ID: i32 = 0x21;
}
impl PacketData for KeepAlive {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		let random_id = buf.read_i16::<BigEndian>()?;
		Ok(Self { random_id })
	}
	fn write<W: std::io::Write>(&self, buf: &mut W) -> io::Result<()> {
        todo!()
    }
}
