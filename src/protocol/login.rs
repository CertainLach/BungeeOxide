use super::*;
use std::io::{Read, Result};

#[derive(Debug)]
pub struct LoginStart {
	pub name: String,
}

impl Packet for LoginStart {
	fn read<R: Read>(buf: &mut R) -> Result<Self> {
		let name = buf.read_string(16).unwrap();
		Ok(Self { name })
	}
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct EncryptionResponse {
	pub shared_secret: Vec<u8>,
	pub verify_token: Vec<u8>,
}

impl Packet for EncryptionResponse {
	fn read<R: Read>(buf: &mut R) -> Result<Self> {
		let shared_secret = buf.read_bytes(128)?;
		let verify_token = buf.read_bytes(128)?;
		Ok(Self {
			shared_secret,
			verify_token,
		})
	}
}
