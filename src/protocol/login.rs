use super::*;
use std::io::{Read, Result};

pub struct LoginStart {
	pub name: String,
}

impl Packet for LoginStart {
	fn read<R: Read>(buf: &mut R) -> Result<Self> {
		let name = buf.read_string(16).unwrap();
		Ok(Self { name })
	}
}

pub struct LoginSuccess {
	pub uuid: String,
	pub username: String,
}

impl Packet for LoginSuccess {
	fn read<R: Read>(buf: &mut R) -> Result<Self> {
		let uuid = buf.read_string(36).unwrap();
		let username = buf.read_string(16).unwrap();
		Ok(Self { uuid, username })
	}
}
