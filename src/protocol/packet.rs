use std::io::{Read, Result, Write};

pub trait PacketData: Sized {
	fn read<R: Read>(buf: &mut R) -> Result<Self>;
	fn write<W: Write>(&self, buf: &mut W) -> Result<()>;
}

pub trait Packet: PacketData {
	const ID: i32;
}
