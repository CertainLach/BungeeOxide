use bungeeoxide_converter::Version;
use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow, bail, ensure};
use bungeeoxide_converter::Root;
use bungeeoxide_converter::generated::convert::v26_3::{
	login_to_client_packet as login_c, login_to_client_packet_encryption_begin as begin,
	login_to_client_packet_success as success, login_to_server_packet as login_s,
	login_to_server_packet_encryption_begin as response, login_to_server_packet_login_start,
};
use bungeeoxide_converter::generated::convert::v26_4_snapshot_10::{
	login_to_client_connection_options as options, login_to_client_packet as key_c,
	login_to_client_packet_key as key, login_to_server_packet as key_s,
	login_to_server_packet_encryption_begin as key_response,
	login_to_server_packet_options_applied as applied,
};
use bungeeoxide_converter::generated::convert::{packet_name, v1_19, v1_19_2};
use rsa::pkcs8::{DecodePublicKey, EncodePublicKey};
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};
use tracing::{info, warn};

use crate::codec::Cipher;
use crate::exchange::{self, Keys, Params, Peer, SERVER_NAME, Side};
use crate::signing::Chat;
use crate::{KEY_EXCHANGE, PIVOT, Proxy, decode, encode, legacy, next};
use bungeeoxide_authlib::accounts::{Account, Credentials};
use bungeeoxide_authlib::{self as auth, Endpoints, GameProfile, PlayerKeys, offline_uuid_bits};

#[derive(Default)]
pub struct Online {
	account: Option<Arc<Account>>,
	credentials: Option<Credentials>,
	keys: Option<Arc<PlayerKeys>>,
	signed_start: bool,
	encrypted: bool,
	name: String,
	pending: Option<Keys>,
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

	pub async fn prepare(&mut self, sv: Version) -> Result<()> {
		self.credentials = None;
		self.keys = None;
		self.signed_start = false;
		self.encrypted = false;
		self.pending = None;
		let Some(account) = self.account.clone() else {
			return Ok(());
		};
		if legacy::is_legacy(sv) {
			bail!(
				"{} servers check logins with Mojang's legacy session service, which is gone, so online mode needs a 1.7.2+ server",
				sv
			);
		}
		let mut credentials = blocking({
			let account = account.clone();
			move || account.credentials(false)
		})
		.await?;
		if sv >= Version::V1_19 {
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

	pub fn login_start(&mut self, sv: Version, username: &str) -> Result<(Version, Vec<u8>)> {
		let (name, uuid) = match &self.credentials {
			Some(c) => (c.name.clone(), c.uuid),
			None => (username.to_owned(), offline_uuid_bits(username)),
		};
		self.name = name.clone();
		if let Some(keys) = self
			.keys
			.as_ref()
			.filter(|_| (Version::V1_19..Version::V1_19_3).contains(&sv))
		{
			self.signed_start = true;
			let data = if sv < Version::V1_19_2 {
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
		Ok((PIVOT, encode(&start, login_s::serialize)?))
	}

	pub fn joined(&mut self, sv: Version, profile: &success::Ty) {
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
			session_id: rand::random(),
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

async fn has_joined(endpoints: &Endpoints, username: &str, hash: String) -> Result<GameProfile> {
	let (endpoints, name) = (endpoints.clone(), username.to_owned());
	blocking(move || auth::has_joined(&endpoints, &name, &hash))
		.await?
		.ok_or_else(|| {
			anyhow!(
				"the session server did not see {username} join, restart the game and try again"
			)
		})
}

async fn join_session(p: &mut Proxy, hash: String, username: &str) -> Result<()> {
	let Some(account) = p.online.account.clone() else {
		bail!(
			"the server is in online mode, give {username} an upstream account in --users or set online-mode=false"
		);
	};
	let credentials = p
		.online
		.credentials
		.clone()
		.ok_or_else(|| anyhow!("no credentials for {username}"))?;
	let credentials = blocking(move || account.join(&credentials, &hash)).await?;
	p.online.credentials = Some(credentials);
	Ok(())
}

async fn authenticate_key_exchange(
	p: &mut Proxy,
	endpoints: &Endpoints,
	username: &str,
	protocol: i32,
) -> Result<GameProfile> {
	let secret = exchange::secret();
	let params = Params {
		protocol,
		salt: rand::random(),
		authenticate: true,
	};
	let request = key_c::Ty::Key(key::Ty {
		salt: params.salt,
		server_id: SERVER_NAME.to_owned(),
		public_key: exchange::der(&secret),
		should_authenticate: params.authenticate,
	});
	p.send(
		Root::LoginToClient,
		KEY_EXCHANGE,
		encode(&request, key_c::serialize)?,
	)
	.await?;
	let data = next(&mut p.client).await?;
	let cv = p.cv;
	let reply = p
		.convert(Root::LoginToServer, cv, KEY_EXCHANGE, data)
		.iter()
		.find_map(|packet| match decode(packet, key_s::deserialize) {
			Ok(key_s::Ty::EncryptionBegin(reply)) => Some(reply),
			_ => None,
		})
		.ok_or_else(|| anyhow!("expected the client's key"))?;
	let peer = Peer {
		name: username,
		key: &reply.public_key,
	};
	let keys = Keys::derive(&params, &secret, Side::Server, SERVER_NAME, &peer)
		.context("reading the client's key")?;
	ensure!(
		keys.verify(&reply.signature),
		"the client signed a different key exchange"
	);
	let profile = has_joined(endpoints, username, keys.authentication.clone()).await?;
	let apply = key_c::Ty::ApplyOptions(options::Ty {
		encryption: true,
		compression: false,
	});
	p.send(
		Root::LoginToClient,
		KEY_EXCHANGE,
		encode(&apply, key_c::serialize)?,
	)
	.await?;
	let data = next(&mut p.client).await?;
	ensure!(
		packet_name(Root::LoginToServer, cv, &data) == Some("options_applied"),
		"expected the client to apply encryption"
	);
	p.client.codec_mut().cipher = Some(keys.cipher(Side::Server));
	Ok(profile)
}

pub async fn authenticate(
	p: &mut Proxy,
	endpoints: &Endpoints,
	username: &str,
	protocol: i32,
) -> Result<GameProfile> {
	if p.cv >= KEY_EXCHANGE {
		return authenticate_key_exchange(p, endpoints, username, protocol).await;
	}
	let key = tokio::task::spawn_blocking(legacy::rsa_key).await?;
	let public = key
		.to_public_key()
		.to_public_key_der()
		.context("encoding the public key")?
		.into_vec();
	let token: [u8; 4] = rand::random();
	let request = login_c::Ty::EncryptionBegin(begin::Ty {
		server_id: String::new(),
		public_key: public.clone(),
		verify_token: token.to_vec(),
		should_authenticate: true,
	});
	p.send(
		Root::LoginToClient,
		PIVOT,
		encode(&request, login_c::serialize)?,
	)
	.await?;
	let data = next(&mut p.client).await?;
	let (shared, verify) = if (Version::V1_19..Version::V1_19_3).contains(&p.cv) {
		let reply = match p.cv < Version::V1_19_2 {
			true => match decode(&data, v1_19::login_to_server_packet::deserialize)? {
				v1_19::login_to_server_packet::Ty::EncryptionBegin(reply) => reply,
				_ => bail!("expected the encryption response"),
			},
			false => match decode(&data, v1_19_2::login_to_server_packet::deserialize)? {
				v1_19_2::login_to_server_packet::Ty::EncryptionBegin(reply) => reply,
				_ => bail!("expected the encryption response"),
			},
		};
		let verify = match reply.crypto {
			v1_19::login_to_server_packet_encryption_begin_crypto::Ty::True(verify) => Some(verify),
			v1_19::login_to_server_packet_encryption_begin_crypto::Ty::False(_) => None,
		};
		(reply.shared_secret, verify)
	} else {
		let cv = p.cv;
		p.convert(Root::LoginToServer, cv, PIVOT, data)
			.iter()
			.find_map(|packet| match decode(packet, login_s::deserialize) {
				Ok(login_s::Ty::EncryptionBegin(reply)) => {
					Some((reply.shared_secret, Some(reply.verify_token)))
				}
				_ => None,
			})
			.ok_or_else(|| anyhow!("expected the encryption response"))?
	};
	let secret = key
		.decrypt(Pkcs1v15Encrypt, &shared)
		.context("decrypting the shared secret")?;
	if let Some(verify) = verify {
		let verify = key
			.decrypt(Pkcs1v15Encrypt, &verify)
			.context("decrypting the verify token")?;
		if verify != token {
			bail!("the client answered with the wrong verify token");
		}
	}
	let secret: [u8; 16] = secret
		.try_into()
		.map_err(|_| anyhow!("the shared secret is not 16 bytes"))?;
	p.client.codec_mut().cipher = Some(Cipher::new(&secret));
	has_joined(endpoints, username, auth::server_hash("", &secret, &public)).await
}

async fn exchange_keys(
	p: &mut Proxy,
	request: key::Ty,
	protocol: i32,
	username: &str,
) -> Result<()> {
	let secret = exchange::secret();
	let params = Params {
		protocol,
		salt: request.salt,
		authenticate: request.should_authenticate,
	};
	let peer = Peer {
		name: &request.server_id,
		key: &request.public_key,
	};
	let keys = Keys::derive(&params, &secret, Side::Client, &p.online.name, &peer)
		.context("reading the server key")?;
	if request.should_authenticate {
		join_session(p, keys.authentication.clone(), username).await?;
	}
	let reply = key_s::Ty::EncryptionBegin(key_response::Ty {
		public_key: exchange::der(&secret),
		signature: keys.signature().to_vec(),
	});
	p.online.pending = Some(keys);
	p.send(
		Root::LoginToServer,
		KEY_EXCHANGE,
		encode(&reply, key_s::serialize)?,
	)
	.await
}

async fn apply_options(p: &mut Proxy, options: options::Ty) -> Result<()> {
	if options.compression {
		bail!(
			"{} asked for compression ahead of the login, which is not supported",
			p.target
		);
	}
	let keys =
		match options.encryption {
			true => Some(p.online.pending.take().ok_or_else(|| {
				anyhow!("{} applied encryption before exchanging keys", p.target)
			})?),
			false => None,
		};
	let reply = key_s::Ty::OptionsApplied(applied::Ty {});
	p.send(
		Root::LoginToServer,
		KEY_EXCHANGE,
		encode(&reply, key_s::serialize)?,
	)
	.await?;
	if let Some(keys) = keys {
		p.server()?.codec_mut().cipher = Some(keys.cipher(Side::Client));
		p.online.encrypted = true;
	}
	Ok(())
}

pub async fn key_exchange(
	p: &mut Proxy,
	data: &[u8],
	protocol: i32,
	username: &str,
) -> Result<bool> {
	let Some("key" | "apply_options") = packet_name(Root::LoginToClient, p.sv, data) else {
		return Ok(false);
	};
	let sv = p.sv;
	for packet in p.convert(Root::LoginToClient, sv, KEY_EXCHANGE, data.to_vec()) {
		match decode(&packet, key_c::deserialize)? {
			key_c::Ty::Key(request) => exchange_keys(p, request, protocol, username).await?,
			key_c::Ty::ApplyOptions(options) => apply_options(p, options).await?,
			_ => {}
		}
	}
	Ok(true)
}

pub async fn encrypt(p: &mut Proxy, request: begin::Ty, username: &str) -> Result<()> {
	let key =
		RsaPublicKey::from_public_key_der(&request.public_key).context("reading the server key")?;
	let secret: [u8; 16] = rand::random();
	if request.should_authenticate {
		let hash = auth::server_hash(&request.server_id, &secret, &request.public_key);
		join_session(p, hash, username).await?;
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
			let data = if p.sv < Version::V1_19_2 {
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
			(PIVOT, encode(&reply, login_s::serialize)?)
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

	#[test]
	fn connection_options_use_the_vanilla_flag_bits() {
		for (encryption, compression, bits) in [(true, false, 1), (false, true, 2), (true, true, 3)]
		{
			let apply = key_c::Ty::ApplyOptions(options::Ty {
				encryption,
				compression,
			});
			assert_eq!(encode(&apply, key_c::serialize).unwrap(), [6, bits]);
		}
	}
	use bungeeoxide_converter::{Session, convert};

	#[test]
	fn verify_token_responses_reach_signing_versions_unsigned() {
		let reply = login_s::Ty::EncryptionBegin(response::Ty {
			shared_secret: vec![1, 2],
			verify_token: vec![3, 4],
		});
		let data = encode(&reply, login_s::serialize).unwrap();
		for (version, signed) in [(Version::V1_19, false), (Version::V1_19_2, true)] {
			let out = convert(
				Root::LoginToServer,
				PIVOT,
				version,
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
		let (from, start) = online.login_start(Version::V1_8, "Steve").unwrap();
		assert_eq!(from, PIVOT);
		for version in [
			Version::V1_7,
			Version::V1_8,
			Version::V1_12_2,
			Version::V1_19,
			Version::V1_19_2,
			Version::V1_20_2,
			Version::V26_1,
		] {
			let out = convert(
				Root::LoginToServer,
				from,
				version,
				&start,
				&mut Session::new(),
			)
			.unwrap();
			assert_eq!(out.len(), 1, "{version}");
		}
	}
}
