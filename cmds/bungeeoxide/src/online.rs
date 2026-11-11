use std::sync::Arc;

use anyhow::{anyhow, bail, Context as _, Result};
use minecraft_versions::gen::convert::v26_1::{
	login_to_client_packet_encryption_begin as begin, login_to_client_packet_success as success,
	login_to_server_packet as login_s, login_to_server_packet_encryption_begin as response,
	login_to_server_packet_login_start,
};
use minecraft_versions::gen::convert::{v1_19, v1_19_2};
use minecraft_versions::{Root, VERSIONS};
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};
use tracing::{info, warn};

use crate::accounts::{Account, Credentials};
use crate::auth::{self, PlayerKeys};
use crate::codec::Cipher;
use crate::signing::Chat;
use crate::{at, encode, legacy, offline_uuid_bits, pivot, Proxy};

#[derive(Default)]
pub struct Online {
	account: Option<Arc<Account>>,
	credentials: Option<Credentials>,
	keys: Option<Arc<PlayerKeys>>,
	signed_start: bool,
	encrypted: bool,
	profile: Option<success::Ty>,
	pub(crate) chat: Option<Chat>,
}

async fn blocking<T: Send + 'static>(
	work: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
	tokio::task::spawn_blocking(work)
		.await
		.map_err(|e| anyhow!("{e}"))?
}

impl Online {
	pub fn new(account: Option<Arc<Account>>, username: &str) -> Self {
		if let Some(account) = &account {
			info!("{username} joins upstream as {}", account.label);
		}
		Self {
			account,
			..Self::default()
		}
	}

	pub async fn prepare(&mut self, sv: usize) -> Result<()> {
		self.credentials = None;
		self.keys = None;
		self.signed_start = false;
		self.encrypted = false;
		let Some(account) = self.account.clone() else {
			return Ok(());
		};
		if legacy::is_legacy(sv) {
			bail!(
				"{} servers check logins with Mojang's legacy session service, which is gone, so online mode needs a 1.7.2+ server",
				VERSIONS[sv]
			);
		}
		let mut credentials = blocking({
			let account = account.clone();
			move || account.credentials(false)
		})
		.await?;
		if sv >= at("1.19") {
			let fetched = blocking({
				let credentials = credentials.clone();
				move || account.keys(&credentials)
			})
			.await;
			match fetched {
				Ok((refreshed, keys)) => {
					credentials = refreshed;
					self.keys = Some(keys);
				}
				Err(e) => warn!(
					"{} has no chat signing key, signed chat will fail: {e:#}",
					credentials.name
				),
			}
		}
		self.credentials = Some(credentials);
		Ok(())
	}

	pub fn login_start(&mut self, sv: usize, username: &str) -> Result<(usize, Vec<u8>)> {
		let (name, uuid) = match &self.credentials {
			Some(c) => (c.name.clone(), c.uuid),
			None => (username.to_owned(), offline_uuid_bits(username)),
		};
		if let Some(keys) = self
			.keys
			.as_ref()
			.filter(|_| (at("1.19")..at("1.19.3")).contains(&sv))
		{
			self.signed_start = true;
			let data = if sv < at("1.19.2") {
				let start = v1_19::login_to_server_packet::Ty::LoginStart(
					v1_19::login_to_server_packet_login_start::Ty {
						username: name,
						signature: Some(
							v1_19::login_to_server_packet_login_start_signature_items::Ty {
								timestamp: keys.expires_at,
								public_key: keys.public.clone(),
								signature: keys.legacy_signature.clone(),
							},
						),
					},
				);
				encode(&start, v1_19::login_to_server_packet::serialize)?
			} else {
				let start = v1_19_2::login_to_server_packet::Ty::LoginStart(
					v1_19_2::login_to_server_packet_login_start::Ty {
						username: name,
						signature: Some(
							v1_19_2::login_to_server_packet_login_start_signature_items::Ty {
								timestamp: keys.expires_at,
								public_key: keys.public.clone(),
								signature: keys.signature.clone(),
							},
						),
						player_uuid: Some(uuid),
					},
				);
				encode(&start, v1_19_2::login_to_server_packet::serialize)?
			};
			return Ok((sv, data));
		}
		let start = login_s::Ty::LoginStart(login_to_server_packet_login_start::Ty {
			username: name,
			player_uuid: uuid,
		});
		Ok((pivot(), encode(&start, login_s::serialize)?))
	}

	pub fn joined(&mut self, sv: usize, profile: &success::Ty) {
		self.chat = None;
		if self.account.is_none() {
			return;
		}
		self.profile = Some(profile.clone());
		let Some(keys) = self.keys.clone() else {
			return;
		};
		if !(self.encrypted || self.signed_start) {
			return;
		}
		let sender = Some(profile.uuid)
			.filter(|uuid| *uuid != 0)
			.or(self.credentials.as_ref().map(|c| c.uuid));
		self.chat = sender.and_then(|sender| Chat::new(sv, keys, sender));
	}

	pub fn success(&self, username: &str) -> success::Ty {
		self.profile.clone().unwrap_or_else(|| success::Ty {
			uuid: offline_uuid_bits(username),
			username: username.to_owned(),
			properties: Vec::new(),
		})
	}

	pub fn observe(&mut self, root: Root, data: &[u8]) {
		if let (Root::PlayToClient, Some(chat)) = (root, self.chat.as_mut()) {
			chat.observe(data);
		}
	}

	pub fn sign(&mut self, root: Root, packets: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
		match (root, self.chat.as_mut()) {
			(Root::PlayToServer, Some(chat)) => chat.outgoing(packets),
			_ => packets,
		}
	}
}

pub async fn encrypt(p: &mut Proxy, request: begin::Ty, username: &str) -> Result<()> {
	let key =
		RsaPublicKey::from_public_key_der(&request.public_key).context("reading the server key")?;
	let secret: [u8; 16] = rand::random();
	if request.should_authenticate {
		let Some(account) = p.online.account.clone() else {
			bail!("the server is in online mode, map {username} to an account with --accounts or set online-mode=false");
		};
		let credentials = p
			.online
			.credentials
			.clone()
			.ok_or_else(|| anyhow!("no credentials for {username}"))?;
		let hash = auth::server_hash(&request.server_id, &secret, &request.public_key);
		let credentials = blocking(move || account.join(&credentials, &hash)).await?;
		p.online.credentials = Some(credentials);
	}
	let shared = key
		.encrypt(&mut rand::thread_rng(), Pkcs1v15Encrypt, &secret)
		.context("encrypting the shared secret")?;
	let signer = p.online.keys.clone().filter(|_| p.online.signed_start);
	let (from, data) = match signer {
		Some(keys) => {
			let salt: i64 = rand::random();
			let mut nonce = request.verify_token.clone();
			nonce.extend_from_slice(&salt.to_be_bytes());
			let reply = v1_19::login_to_server_packet_encryption_begin::Ty {
				shared_secret: shared,
				crypto: v1_19::login_to_server_packet_encryption_begin_crypto::Ty::False(
					v1_19::login_to_server_packet_encryption_begin_crypto_false::Ty {
						salt,
						message_signature: keys.sign(&nonce)?,
					},
				),
			};
			let data = if p.sv < at("1.19.2") {
				encode(
					&v1_19::login_to_server_packet::Ty::EncryptionBegin(reply),
					v1_19::login_to_server_packet::serialize,
				)?
			} else {
				encode(
					&v1_19_2::login_to_server_packet::Ty::EncryptionBegin(reply),
					v1_19_2::login_to_server_packet::serialize,
				)?
			};
			(p.sv, data)
		}
		None => {
			let token = key
				.encrypt(
					&mut rand::thread_rng(),
					Pkcs1v15Encrypt,
					&request.verify_token,
				)
				.context("encrypting the verify token")?;
			let reply = login_s::Ty::EncryptionBegin(response::Ty {
				shared_secret: shared,
				verify_token: token,
			});
			(pivot(), encode(&reply, login_s::serialize)?)
		}
	};
	p.send(Root::LoginToServer, from, data).await?;
	p.server()?.codec_mut().cipher = Some(Cipher::new(&secret));
	p.online.encrypted = true;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::decode;
	use minecraft_versions::{convert, Session};

	#[test]
	fn verify_token_responses_reach_signing_versions_unsigned() {
		let reply = login_s::Ty::EncryptionBegin(response::Ty {
			shared_secret: vec![1, 2],
			verify_token: vec![3, 4],
		});
		let data = encode(&reply, login_s::serialize).unwrap();
		for (version, signed) in [("1.19", false), ("1.19.2", true)] {
			let out = convert(
				Root::LoginToServer,
				pivot(),
				at(version),
				&data,
				&mut Session::new(),
			)
			.unwrap();
			let [packet] = out.as_slice() else {
				panic!("{version}: {out:?}")
			};
			let begin = if signed {
				let v1_19_2::login_to_server_packet::Ty::EncryptionBegin(b) =
					decode(packet, v1_19_2::login_to_server_packet::deserialize).unwrap()
				else {
					panic!()
				};
				b
			} else {
				let v1_19::login_to_server_packet::Ty::EncryptionBegin(b) =
					decode(packet, v1_19::login_to_server_packet::deserialize).unwrap()
				else {
					panic!()
				};
				b
			};
			assert_eq!(begin.shared_secret, vec![1, 2], "{version}");
			assert!(
				matches!(begin.crypto, v1_19::login_to_server_packet_encryption_begin_crypto::Ty::True(ref t) if *t == vec![3, 4]),
				"{version}: {:?}",
				begin.crypto
			);
		}
	}

	#[test]
	fn login_starts_reach_every_netty_version() {
		let mut online = Online::default();
		let (from, start) = online.login_start(at("1.8"), "Steve").unwrap();
		assert_eq!(from, pivot());
		for version in ["1.7", "1.8", "1.12.2", "1.19", "1.19.2", "1.20.2", "26.1"] {
			let out = convert(
				Root::LoginToServer,
				from,
				at(version),
				&start,
				&mut Session::new(),
			)
			.unwrap();
			assert_eq!(out.len(), 1, "{version}");
		}
	}
}
