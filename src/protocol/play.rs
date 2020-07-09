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
		let entity_id = buf.read_i32::<BigEndian>().unwrap();
		let game_mode = buf.read_u8().unwrap();
		let dimension = buf.read_i32::<BigEndian>().unwrap();
		let difficulty = buf.read_u8().unwrap();
		let max_players = buf.read_u8().unwrap();
		let level_type = buf.read_string(32767).unwrap();
		let reduced_debug_info = buf.read_u8().unwrap() == 1;
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
