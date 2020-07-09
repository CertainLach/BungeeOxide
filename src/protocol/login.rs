use super::*;
use std::io::{Read, Result};

pub struct LoginStart {
	pub name: String,
}
impl Packet for LoginStart {
	const ID: i32 = 0;
}
impl PacketData for LoginStart {
	fn read<R: Read>(buf: &mut R) -> Result<Self> {
		let name = buf.read_string(16).unwrap();
		Ok(Self { name })
	}
	fn write<W: std::io::Write>(&self, buf: &mut W) -> io::Result<()> {
		todo!()
	}
}

pub struct LoginSuccess {
	pub uuid: String,
	pub username: String,
}
impl Packet for LoginSuccess {
	const ID: i32 = 2;
}
impl PacketData for LoginSuccess {
	fn read<R: Read>(buf: &mut R) -> Result<Self> {
		let uuid = buf.read_string(36).unwrap();
		let username = buf.read_string(16).unwrap();
		Ok(Self { uuid, username })
	}
	fn write<W: std::io::Write>(&self, buf: &mut W) -> io::Result<()> {
		todo!()
	}
}
