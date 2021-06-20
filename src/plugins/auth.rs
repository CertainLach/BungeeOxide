use async_trait::async_trait;
use num_bigint_dig::{BigInt, Sign};
use rand::{rngs::OsRng, thread_rng, Rng};
use reqwest::Client;
use rsa::{PaddingScheme, PublicKeyParts, RSAPrivateKey, RSAPublicKey};
use rsa_der::public_key_to_der;
use serde::Deserialize;
use std::convert::Infallible;
use thiserror::Error;
use uuid::Uuid;

use crate::protocol::login::{EncryptionRequest, EncryptionResponse};

#[derive(Debug, Error)]
pub enum AuthError {
	#[error("passed verify token is not same as generated")]
	BadVerifyToken,
	#[error("rsa error: {0}")]
	Rsa(#[from] rsa::errors::Error),
	#[error("bad shared secret")]
	BadSharedSecret,
	#[error("unexpected encryption response")]
	Unsupported,
	#[error(transparent)]
	Other(anyhow::Error),
}

pub struct AuthSucceeded {
	pub username: String,
	pub uuid: String,
}

pub enum EncryptionStartResult<D = Infallible> {
	BeginEncryption(EncryptionRequest, D),
	Skip(AuthSucceeded),
}

#[async_trait]
pub trait AuthPlugin: Sync {
	type AuthData: Send;
	fn encryption_start(&self, name: String) -> EncryptionStartResult<Self::AuthData>;
	async fn encryption_response(
		&self,
		_data: Self::AuthData,
		_res: EncryptionResponse,
	) -> Result<AuthSucceeded, AuthError> {
		Err(AuthError::Unsupported)
	}
}

pub struct OfflineAuthPlugin;
impl AuthPlugin for OfflineAuthPlugin {
	type AuthData = Infallible;
	fn encryption_start(&self, name: String) -> EncryptionStartResult {
		let input = format!("OfflinePlayer:{}", name);
		let mut hash = md5::compute(input).0;
		hash[6] = hash[6] & 0x0f | 0x30;
		hash[8] = hash[8] & 0x3f | 0x80;
		EncryptionStartResult::Skip(AuthSucceeded {
			username: name,
			uuid: Uuid::from_slice(&hash).unwrap().to_string(),
		})
	}
}

pub struct MojangAuthPlugin {
	private: RSAPrivateKey,
	// public: RSAPublicKey,
	public_der: Vec<u8>,
	client: Client,
}
impl MojangAuthPlugin {
	fn new(private: RSAPrivateKey) -> Self {
		let public: RSAPublicKey = private.clone().into();
		let public_der = public_key_to_der(
			&BigInt::from_biguint(Sign::Plus, public.n().clone()).to_signed_bytes_be(),
			&BigInt::from_biguint(Sign::Plus, public.e().clone()).to_signed_bytes_be(),
		);
		Self {
			private,
			// public,
			public_der,
			client: Client::new(),
		}
	}
	pub fn with_generated_keypair() -> Self {
		Self::new(RSAPrivateKey::new(&mut OsRng, 1024).unwrap())
	}
}

#[derive(Deserialize)]
pub struct HasJoinedResponse {
	pub id: Uuid,
	pub name: String,
}

pub struct AuthlibAuthData {
	verify_token: Vec<u8>,
	name: String,
}
#[async_trait]
impl AuthPlugin for MojangAuthPlugin {
	type AuthData = AuthlibAuthData;
	fn encryption_start(&self, name: String) -> EncryptionStartResult<Self::AuthData> {
		let verify_token: Vec<u8> = thread_rng().gen::<[u8; 4]>().into();
		EncryptionStartResult::BeginEncryption(
			EncryptionRequest {
				public: self.public_der.clone(),
				server_id: "".into(),
				verify_token: verify_token.clone(),
			},
			AuthlibAuthData {
				verify_token,
				name: name.clone(),
			},
		)
	}
	async fn encryption_response(
		&self,
		data: Self::AuthData,
		res: EncryptionResponse,
	) -> Result<AuthSucceeded, AuthError> {
		let verify_token = self
			.private
			.decrypt(PaddingScheme::PKCS1v15Encrypt, &res.verify_token)?;
		if verify_token != data.verify_token {
			return Err(AuthError::BadVerifyToken);
		}
		let shared_secret = self
			.private
			.decrypt(PaddingScheme::PKCS1v15Encrypt, &res.shared_secret)?;
		if shared_secret.len() != 16 {
			return Err(AuthError::BadSharedSecret);
		}
		let mut shared_secret_arr = [0; 16];
		shared_secret_arr.copy_from_slice(&shared_secret);

		let mut hash = sha1::Sha1::new();
		hash.update(b"");
		hash.update(&shared_secret);
		hash.update(&self.public_der);
		let hash_hex = hex::encode(hash.digest().bytes());

		let result = self
			.client
			.get("https://sessionserver.mojang.com/session/minecraft/hasJoined")
			.query(&[("username", &data.name), ("serverId", &hash_hex)])
			.query(&[("unsigned", false)])
			.send()
			.await
			.map_err(|e| AuthError::Other(e.into()))?
			.text()
			.await
			.map_err(|e| AuthError::Other(e.into()))?;
		dbg!(&result);
		let result: HasJoinedResponse =
			serde_json::from_str(&result).map_err(|e| AuthError::Other(e.into()))?;
		Ok(AuthSucceeded {
			username: result.name,
			uuid: result.id.to_string(),
		})
	}
}
