use anyhow::{Result, anyhow, bail, ensure};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::codec::Cipher;
use crate::legacy::Cfb8;

pub const SERVER_NAME: &str = "server";

const SALT: [i64; 3] = [
	3195254534265565831,
	8302438179253405948,
	6794457759077931664,
];

const DER_PREFIX: [u8; 12] = [
	0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x6e, 0x03, 0x21, 0x00,
];

const SEPARATOR: u8 = b'+';

const BASE32_HEX: &[u8; 32] = b"0123456789ABCDEFGHIJKLMNOPQRSTUV";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
	Server,
	Client,
}

pub struct Params {
	pub protocol: i32,
	pub salt: i64,
	pub authenticate: bool,
}

pub struct Peer<'a> {
	pub name: &'a str,
	pub key: &'a [u8],
}

pub fn secret() -> StaticSecret {
	StaticSecret::random_from_rng(rand::thread_rng())
}

pub fn der(secret: &StaticSecret) -> Vec<u8> {
	let mut out = DER_PREFIX.to_vec();
	out.extend_from_slice(PublicKey::from(secret).as_bytes());
	out
}

fn parse(der: &[u8]) -> Result<PublicKey> {
	let Some(raw) = der.strip_prefix(&DER_PREFIX) else {
		bail!("not an X.509 X25519 public key");
	};
	let raw: [u8; 32] = raw
		.try_into()
		.map_err(|_| anyhow!("X25519 public key of {} bytes", raw.len()))?;
	Ok(PublicKey::from(raw))
}

fn hmac(key: &[u8]) -> Hmac<Sha256> {
	Hmac::new_from_slice(key).expect("hmac takes keys of any size")
}

fn base32_hex(data: &[u8]) -> String {
	let mut out = String::new();
	let (mut buffer, mut bits) = (0u32, 0);
	for b in data {
		buffer = (buffer << 8) | u32::from(*b);
		bits += 8;
		while bits >= 5 {
			bits -= 5;
			out.push(BASE32_HEX[((buffer >> bits) & 31) as usize] as char);
		}
	}
	if bits > 0 {
		out.push(BASE32_HEX[((buffer << (5 - bits)) & 31) as usize] as char);
	}
	out
}

fn java_hash(data: &[u8]) -> i32 {
	data.iter().fold(1i32, |h, b| {
		h.wrapping_mul(31).wrapping_add(i32::from(*b as i8))
	})
}

struct JavaRandom(u64);

impl JavaRandom {
	const MULTIPLIER: u64 = 0x5_DEEC_E66D;
	const MASK: u64 = (1 << 48) - 1;

	fn new(seed: i64) -> Self {
		Self((seed as u64 ^ Self::MULTIPLIER) & Self::MASK)
	}

	fn next_int(&mut self) -> i32 {
		self.0 = self.0.wrapping_mul(Self::MULTIPLIER).wrapping_add(0xB) & Self::MASK;
		(self.0 >> 16) as u32 as i32
	}

	fn bytes<const N: usize>(&mut self) -> [u8; N] {
		let mut out = [0; N];
		for chunk in out.chunks_mut(4) {
			let mut rnd = self.next_int();
			for b in chunk {
				*b = rnd as u8;
				rnd >>= 8;
			}
		}
		out
	}
}

pub struct Keys {
	a_to_b: [u8; 16],
	b_to_a: [u8; 16],
	integrity: Hmac<Sha256>,
	pub authentication: String,
}

impl Keys {
	pub fn derive(
		params: &Params,
		secret: &StaticSecret,
		side: Side,
		name: &str,
		peer: &Peer<'_>,
	) -> Result<Self> {
		let shared = secret.diffie_hellman(&parse(peer.key)?);
		ensure!(
			shared.was_contributory(),
			"the peer's X25519 key has a small order"
		);
		let key = der(secret);
		let own = Peer { name, key: &key };
		let (a, b) = match side {
			Side::Server => (&own, peer),
			Side::Client => (peer, &own),
		};
		let mut salt = params.salt.to_be_bytes().to_vec();
		for part in SALT {
			salt.extend_from_slice(&part.to_be_bytes());
		}
		let hkdf = Hkdf::<Sha256>::new(Some(&salt), shared.as_bytes());
		let names = format!("a={};b={}", a.name, b.name);
		let connection = format!("{names};protocol={}", params.protocol);
		let expand = |info: String, out: &mut [u8]| {
			hkdf.expand(info.as_bytes(), out)
				.map_err(|e| anyhow!("deriving session keys: {e}"))
		};
		let mut a_to_b = [0; 16];
		let mut b_to_a = [0; 16];
		let mut auth = [0; 32];
		let mut verify = [0; 32];
		expand(format!("a_to_b;{connection}"), &mut a_to_b)?;
		expand(format!("b_to_a;{connection}"), &mut b_to_a)?;
		expand(format!("authenticate;{names}"), &mut auth)?;
		expand(format!("verify;{names}"), &mut verify)?;

		let mut digest = hmac(&auth);
		digest.update(a.key);
		digest.update(&[SEPARATOR]);
		digest.update(b.key);
		let authentication = base32_hex(&digest.finalize().into_bytes()[..25]);

		let mut integrity = hmac(&verify);
		integrity.update(&params.protocol.to_be_bytes());
		integrity.update(&[SEPARATOR]);
		integrity.update(&params.salt.to_be_bytes());
		integrity.update(&[SEPARATOR]);
		integrity.update(&[if params.authenticate { 1 } else { 2 }]);
		for peer in [a, b] {
			integrity.update(&[SEPARATOR]);
			integrity.update(peer.name.as_bytes());
			integrity.update(&[SEPARATOR]);
			integrity.update(peer.key);
		}

		Ok(Self {
			a_to_b,
			b_to_a,
			integrity,
			authentication,
		})
	}

	pub fn signature(&self) -> [u8; 32] {
		self.integrity.clone().finalize().into_bytes().into()
	}

	pub fn verify(&self, signature: &[u8]) -> bool {
		self.integrity.clone().verify_slice(signature).is_ok()
	}

	fn ivs(&self) -> ([u8; 16], [u8; 16]) {
		let mut random = JavaRandom::new(i64::from(java_hash(&self.signature())));
		let server_to_client = random.bytes();
		let client_to_server = random.bytes();
		(server_to_client, client_to_server)
	}

	pub fn cipher(&self, side: Side) -> Cipher {
		let (server_to_client, client_to_server) = self.ivs();
		let clientbound = (&self.a_to_b, &server_to_client);
		let serverbound = (&self.b_to_a, &client_to_server);
		let (inbound, outbound) = match side {
			Side::Server => (serverbound, clientbound),
			Side::Client => (clientbound, serverbound),
		};
		Cipher::split(
			Cfb8::with_iv(inbound.0, inbound.1),
			Cfb8::with_iv(outbound.0, outbound.1),
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::codec::Codec;
	use bytes::BytesMut;
	use tokio_util::codec::{Decoder, Encoder};

	struct Vector {
		protocol: i32,
		salt: i64,
		authenticate: bool,
		server_name: &'static str,
		client_name: &'static str,
		server_seed: &'static str,
		client_seed: &'static str,
		server_public: &'static str,
		client_public: &'static str,
		a_to_b: &'static str,
		b_to_a: &'static str,
		integrity: &'static str,
		hash: i32,
		authentication: &'static str,
		server_to_client: &'static str,
		client_to_server: &'static str,
		clientbound: &'static str,
		serverbound: &'static str,
	}

	const VECTORS: [Vector; 2] = [
		Vector {
			protocol: 1073742172,
			salt: -5047293836101632921,
			authenticate: true,
			server_name: "server",
			client_name: "Notch",
			server_seed: "01080f161d242b323940474e555c636a71787f868d949ba2a9b0b7bec5ccd3da",
			client_seed: "40474e555c636a71787f868d949ba2a9b0b7bec5ccd3dae1e8eff6fd040b1219",
			server_public: "302a300506032b656e032100c8feca81be196cdf2cadeabf13c4903d7632dce4955aa68b6e5d9adef54e2616",
			client_public: "302a300506032b656e032100e7a0ed1ad488a5ff884c273c2a17b3d2cee3195f22308326f3845f70a85b5a2b",
			a_to_b: "e2662a9cce27090e02a3bd6388233922",
			b_to_a: "47b7204337d1896d706dd295be2a0861",
			integrity: "05f2b5a41911d28b8066cb8a962883e80e5015e284b72832b02795b7db1d5fee",
			hash: -307477888,
			authentication: "A60GK4BNBBF9HE1FAVJ4G0P061GRH1L5CRHTO7PG",
			server_to_client: "2f77d24e029db42c8a79cd80ff5b3091",
			client_to_server: "4afb9e547469152e42a893a6a00f3f24",
			clientbound: "8e28f8a3fd38ab07a8db08d38059a24e5fc69759cad27a6788ede69a88d2701cd8d2",
			serverbound: "688aea7154e301124f217b54949d6c2bf8764510d6d97ccd742f9a7a49f4ca449542",
		},
		Vector {
			protocol: 777,
			salt: 7154011228417052183,
			authenticate: false,
			server_name: "proxy",
			client_name: "Steve_2",
			server_seed: "939aa1a8afb6bdc4cbd2d9e0e7eef5fc030a11181f262d343b424950575e656c",
			client_seed: "c5ccd3dae1e8eff6fd040b121920272e353c434a51585f666d747b828990979e",
			server_public: "302a300506032b656e032100b581212de1db954ddb11ad97d2f3f7255644971db23e091494f140bced3c5f1c",
			client_public: "302a300506032b656e0321009c60c13bcf521d810b3c761859048098006c5ef65faa11034e0643b56746e970",
			a_to_b: "872268ecfe6e874815bc4ae139c47af9",
			b_to_a: "007a5eeb4b1893d0d3e1e1d35d29272b",
			integrity: "0f1d4d301d1f18cdd608f457e55e4a8a29ebfd8969f88448b744155c4600643a",
			hash: 986344604,
			authentication: "6TEMJJ0VLE4581BVCRSHL4GAUO5MJ15CQ18028DO",
			server_to_client: "69d936c4ca37da4e5980ad024fd0b71b",
			client_to_server: "79381d3241e52ff10ac04f63b361d648",
			clientbound: "48537889c0359fe1d5042e43185dff5266b64746ce6e4405f6f2e663ca034fb84bbd",
			serverbound: "112fe3177df17efc012a8836e10bb25b94942e64bcdf5e7cb3a7c4f2915584f23e63",
		},
	];

	fn hex(s: &str) -> Vec<u8> {
		(0..s.len())
			.step_by(2)
			.map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
			.collect()
	}

	fn seeded(s: &str) -> StaticSecret {
		StaticSecret::from(<[u8; 32]>::try_from(hex(s)).unwrap())
	}

	fn encrypt(cipher: Cipher, payload: &str) -> Vec<u8> {
		let mut codec = Codec {
			cipher: Some(cipher),
			..Codec::default()
		};
		let mut out = BytesMut::new();
		codec.encode(payload.as_bytes().to_vec(), &mut out).unwrap();
		out.to_vec()
	}

	fn decrypt(cipher: Cipher, wire: &[u8]) -> Vec<u8> {
		let mut codec = Codec {
			cipher: Some(cipher),
			..Codec::default()
		};
		codec
			.decode(&mut BytesMut::from(wire))
			.unwrap()
			.expect("a whole frame")
	}

	#[test]
	fn sessions_match_the_vanilla_key_exchange() {
		for v in VECTORS {
			let params = Params {
				protocol: v.protocol,
				salt: v.salt,
				authenticate: v.authenticate,
			};
			let (server, client) = (seeded(v.server_seed), seeded(v.client_seed));
			assert_eq!(der(&server), hex(v.server_public), "{}", v.client_name);
			assert_eq!(der(&client), hex(v.client_public), "{}", v.client_name);
			let server_keys = Keys::derive(
				&params,
				&server,
				Side::Server,
				v.server_name,
				&Peer {
					name: v.client_name,
					key: &der(&client),
				},
			)
			.unwrap();
			let client_keys = Keys::derive(
				&params,
				&client,
				Side::Client,
				v.client_name,
				&Peer {
					name: v.server_name,
					key: &der(&server),
				},
			)
			.unwrap();
			for keys in [&server_keys, &client_keys] {
				assert_eq!(keys.a_to_b.to_vec(), hex(v.a_to_b), "{}", v.client_name);
				assert_eq!(keys.b_to_a.to_vec(), hex(v.b_to_a), "{}", v.client_name);
				assert_eq!(keys.signature().to_vec(), hex(v.integrity));
				assert!(keys.verify(&hex(v.integrity)));
				assert_eq!(java_hash(&keys.signature()), v.hash);
				assert_eq!(keys.authentication, v.authentication);
				let (server_to_client, client_to_server) = keys.ivs();
				assert_eq!(server_to_client.to_vec(), hex(v.server_to_client));
				assert_eq!(client_to_server.to_vec(), hex(v.client_to_server));
			}
			let mut tampered = hex(v.integrity);
			tampered[31] ^= 1;
			assert!(!server_keys.verify(&tampered));
			let clientbound = "clientbound bytes from the server";
			let serverbound = "serverbound bytes from the client";
			assert_eq!(
				encrypt(server_keys.cipher(Side::Server), clientbound),
				hex(v.clientbound)
			);
			assert_eq!(
				encrypt(client_keys.cipher(Side::Client), serverbound),
				hex(v.serverbound)
			);
			assert_eq!(
				decrypt(client_keys.cipher(Side::Client), &hex(v.clientbound)),
				clientbound.as_bytes()
			);
			assert_eq!(
				decrypt(server_keys.cipher(Side::Server), &hex(v.serverbound)),
				serverbound.as_bytes()
			);
		}
	}

	#[test]
	fn malformed_and_small_order_keys_are_refused() {
		let params = Params {
			protocol: 1,
			salt: 0,
			authenticate: true,
		};
		let mut zero = DER_PREFIX.to_vec();
		zero.extend_from_slice(&[0; 32]);
		for key in [&zero[..], &zero[..43], &zero[1..]] {
			let peer = Peer { name: "x", key };
			assert!(Keys::derive(&params, &secret(), Side::Server, SERVER_NAME, &peer).is_err());
		}
	}
}
