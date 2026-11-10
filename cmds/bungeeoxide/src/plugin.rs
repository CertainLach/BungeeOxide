use std::net::SocketAddr;

use impl_trait_for_tuples::impl_for_tuples;

use crate::{LoggedInInfo, protocol::login::{EncryptionRequest, EncryptionResponse}};

#[derive(PartialEq)]
pub struct TargetServer {
	pub addr: SocketAddr,
	pub handshake_address: String,
	pub handshake_port: i16,
}

pub trait Plugin {
	fn get_initial_target(&self) -> Option<TargetServer> {
		None
	}
}
