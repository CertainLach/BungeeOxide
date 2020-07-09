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
impl PacketData for i32 {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		buf.read_i32::<BigEndian>()
	}
	fn write<W: Write>(&self, buf: &mut W) -> io::Result<()> {
		buf.write_i32::<BigEndian>(*self)
	}
}
impl PacketData for bool {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		Ok(buf.read_u8()? == 1)
	}
	fn write<W: Write>(&self, buf: &mut W) -> io::Result<()> {
		buf.write_u8(if *self { 1 } else { 0 })?;
		Ok(())
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
impl PacketData for u8 {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		buf.read_u8()
	}
	fn write<W: Write>(&self, buf: &mut W) -> io::Result<()> {
		buf.write_u8(*self)
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

impl<T> PacketData for Vec<T>
where
	T: PacketData + Default + Clone,
{
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		let len = buf.read_varint()?.ans;
		let out = vec![T::default(); len as usize];
		Ok(out)
	}
	fn write<W: Write>(&self, buf: &mut W) -> io::Result<()> {
		VarInt(self.len() as i32).write(buf);
		for v in self.iter() {
			v.write(buf);
		}
		Ok(())
	}
}

pub trait Packet: PacketData {
	const ID: i32;
}
