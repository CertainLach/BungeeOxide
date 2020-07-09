use super::*;
use std::io::{Read, Result};

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct EncryptionResponse {
	pub shared_secret: Vec<u8>,
	pub verify_token: Vec<u8>,
}
impl Packet for EncryptionResponse {
    const ID: i32 = 0x01;
}
impl PacketData for EncryptionResponse {
	fn read<R: Read>(buf: &mut R) -> Result<Self> {
		let shared_secret = buf.read_bytes(128)?;
		let verify_token = buf.read_bytes(128)?;
		Ok(Self {
			shared_secret,
			verify_token,
		})
	}
	fn write<W: std::io::Write>(&self, buf: &mut W) -> Result<()> {
        todo!()
    }
}
