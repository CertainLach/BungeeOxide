use super::*;
use std::io::{Read, Result};

#[derive(Debug, PacketData)]
pub struct LoginStart {
	pub name: String,
}
impl Packet for LoginStart {
	const ID: i32 = 0;
}

#[derive(Debug, PacketData)]
pub struct LoginSuccess {
	pub uuid: String,
	pub username: String,
}
impl Packet for LoginSuccess {
	const ID: i32 = 2;
}

#[derive(PacketData)]
pub struct EncryptionResponse {
	pub shared_secret: Vec<u8>,
	pub verify_token: Vec<u8>,
}
impl Packet for EncryptionResponse {
	const ID: i32 = 0x01;
}
