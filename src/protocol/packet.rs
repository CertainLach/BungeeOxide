use super::*;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::ops::Deref;

pub trait PacketData: Sized {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self>;
	fn write<W: Write>(&self, buf: &mut W) -> io::Result<()>;
}
impl PacketData for String {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		Ok(buf.read_string(i32::MAX)?)
	}
	fn write<W: Write>(&self, buf: &mut W) -> io::Result<()> {
		buf.write_string(self)
	}
}
impl PacketData for i16 {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		buf.read_i16::<BigEndian>()
	}
	fn write<W: Write>(&self, buf: &mut W) -> io::Result<()> {
		buf.write_i16::<BigEndian>(*self)
	}
}

#[derive(Debug)]
pub struct VarInt(i32);
impl Deref for VarInt {
	type Target = i32;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl From<i32> for VarInt {
	fn from(v: i32) -> Self {
		VarInt(v)
	}
}
impl PacketData for VarInt {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		Ok(buf.read_varint()?.ans.into())
	}
	fn write<W: Write>(&self, buf: &mut W) -> io::Result<()> {
		buf.write_varint(self.0)
	}
}

pub trait Packet: PacketData {
	const ID: i32;
}
