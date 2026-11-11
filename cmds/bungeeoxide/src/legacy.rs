use minecraft_versions::Version;
use std::sync::OnceLock;
use std::time::Duration;

use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes128;
use anyhow::{anyhow, bail, Context as _, Result};
use bytes::BytesMut;
use futures::SinkExt;
use minecraft_versions::gen;
use minecraft_versions::pds::{self, Input};
use minecraft_versions::{legacy_version_of, reachable, Root, LEGACY_PROTOCOLS, STACK_BUDGET};
use rsa::pkcs8::{DecodePublicKey, EncodePublicKey};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use crate::codec::Codec;
use crate::status::{self, Upstream};
use crate::{join_upstream, next, relay, Config, Conn, Proxy};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
	Complete(usize),
	Need(usize),
}

macro_rules! lengths {
	($($v:ident => $ns:ident;)*) => {
		pub fn packet_len(root: Root, version: Version, data: &[u8]) -> std::result::Result<Frame, String> {
			let mut input = Input::new(data);
			let read = pds::stack::budget(STACK_BUDGET, || match (version, root) {
				$(
					(Version::$v, Root::PlayToClient) => Ok(gen::$ns::play::to_client::packet::deserialize(&mut input).map(drop)),
					(Version::$v, Root::PlayToServer) => Ok(gen::$ns::play::to_server::packet::deserialize(&mut input).map(drop)),
				)*
				(v, r) => Err(format!("{v} has no pre-netty {r:?} packets")),
			})?;
			match read {
				Ok(()) => Ok(Frame::Complete(input.offset())),
				Err(pds::Error::NotEnoughData) => Ok(Frame::Need(input.needed().max(data.len() + 1))),
				Err(e) => Err(format!("malformed {} packet {:#04x}: {e:?}", version, data[0])),
			}
		}
	};
}

lengths! {
	V1_0_1 => v1_0_1;
	V1_1 => v1_1;
	V1_2_3 => v1_2_3;
	V1_2_5 => v1_2_5;
	V1_3_2 => v1_3_2;
	V1_4_2 => v1_4_2;
	V1_4_5 => v1_4_5;
	V1_4_7 => v1_4_7;
	V1_5_1 => v1_5_1;
	V1_5_2 => v1_5_2;
	V1_6_1 => v1_6_1;
	V1_6_2 => v1_6_2;
	V1_6_4 => v1_6_4;
}

pub fn is_legacy(version: Version) -> bool {
	LEGACY_PROTOCOLS.iter().any(|(_, v)| *v == version)
}

pub fn handshake_len(data: &[u8]) -> std::result::Result<Frame, String> {
	match data {
		[] | [0x02] => Ok(Frame::Need(2)),
		[0x02, 0, ..] => packet_len(Root::PlayToServer, Version::V1_2_5, data),
		[0x02, ..] => packet_len(Root::PlayToServer, Version::V1_6_4, data),
		[0x01, a, b, c, d, ..] => {
			let protocol = i32::from_be_bytes([*a, *b, *c, *d]);
			let version = legacy_version_of(protocol)
				.ok_or_else(|| format!("unknown pre-netty protocol {protocol}"))?;
			packet_len(Root::PlayToServer, version, data)
		}
		[0x01, ..] => Ok(Frame::Need(5)),
		[id, ..] => Err(format!("unexpected pre-netty login packet {id:#04x}")),
	}
}

pub struct Cfb8 {
	aes: Aes128,
	register: [u8; 16],
}

impl Cfb8 {
	pub fn new(key: &[u8; 16]) -> Self {
		Self {
			aes: Aes128::new(GenericArray::from_slice(key)),
			register: *key,
		}
	}

	fn stream(&self) -> u8 {
		let mut block = GenericArray::clone_from_slice(&self.register);
		self.aes.encrypt_block(&mut block);
		block[0]
	}

	fn shift(&mut self, cipher: u8) {
		self.register.copy_within(1.., 0);
		self.register[15] = cipher;
	}

	pub fn encrypt(&mut self, data: &mut [u8]) {
		for b in data {
			*b ^= self.stream();
			self.shift(*b);
		}
	}

	pub fn decrypt(&mut self, data: &mut [u8]) {
		for b in data {
			let cipher = *b;
			*b ^= self.stream();
			self.shift(cipher);
		}
	}
}

pub struct Legacy {
	pub root: Root,
	pub version: Option<Version>,
	pub cipher: Option<(Cfb8, Cfb8)>,
	pub plain: BytesMut,
	pub need: usize,
}

impl Legacy {
	pub fn new(root: Root, version: Option<Version>) -> Self {
		Self {
			root,
			version,
			cipher: None,
			plain: BytesMut::new(),
			need: 0,
		}
	}
}

pub fn string16(out: &mut Vec<u8>, s: &str) {
	let units: Vec<u16> = s.encode_utf16().collect();
	out.extend_from_slice(&(units.len() as i16).to_be_bytes());
	for u in units {
		out.extend_from_slice(&u.to_be_bytes());
	}
}

pub(crate) fn read_string16(data: &[u8], at: usize) -> Option<(String, usize)> {
	let len = i16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?);
	let len = usize::try_from(len).ok()?;
	let units: Vec<u16> = data
		.get(at + 2..at + 2 + len * 2)?
		.chunks_exact(2)
		.map(|c| u16::from_be_bytes([c[0], c[1]]))
		.collect();
	Some((String::from_utf16(&units).ok()?, at + 2 + len * 2))
}

fn short_bytes(data: &[u8], at: usize) -> Option<(&[u8], usize)> {
	let len = usize::try_from(i16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?)).ok()?;
	Some((data.get(at + 2..at + 2 + len)?, at + 2 + len))
}

pub fn kick(reason: &str) -> Vec<u8> {
	let mut out = vec![0xff];
	let reason: String = reason.chars().take(256).collect();
	string16(&mut out, &reason);
	out
}

pub fn served(server: Version) -> Vec<Version> {
	let mut out: Vec<Version> = LEGACY_PROTOCOLS
		.iter()
		.map(|(_, v)| *v)
		.filter(|v| reachable(server, *v))
		.collect();
	out.sort();
	out.dedup();
	out
}

pub fn protocol_of(version: Version) -> Option<i32> {
	LEGACY_PROTOCOLS
		.iter()
		.filter(|(_, v)| *v == version)
		.map(|(p, _)| *p)
		.max()
}

pub fn range(server: Version) -> String {
	let served = served(server);
	match (served.first(), served.last()) {
		(Some(low), Some(high)) => format!("{}-{}", *low, *high),
		_ => "none".to_owned(),
	}
}

pub fn ping_reply(
	request: &[u8],
	config: &Config,
	server: Version,
	upstream: Option<&Upstream>,
) -> String {
	let (motd, online, max) = match upstream {
		Some(u) => (status::legacy_motd(&u.description), u.online, u.max),
		None => (config.motd.clone(), 0, 0),
	};
	if request.len() < 2 {
		return status::legacy_bare(&motd, online, max);
	}
	let served = served(server);
	let wanted = match request {
		[0xfe, 0x01, 0xfa, ..] => read_string16(request, 3)
			.and_then(|(_, at)| request.get(at + 2).copied())
			.map(i32::from),
		_ => None,
	};
	let protocol = wanted
		.filter(|p| legacy_version_of(*p).is_some_and(|v| served.contains(&v)))
		.or_else(|| served.last().and_then(|v| protocol_of(*v)))
		.unwrap_or(78);
	status::legacy_fields(
		protocol,
		&format!("pds2 bridge {}", range(server)),
		&motd,
		online,
		max,
	)
}

async fn ping(mut stream: TcpStream, config: &Config) -> Result<()> {
	use tokio::io::AsyncWriteExt;
	let server = config.server_version;
	let mut request = Vec::new();
	let mut buf = [0u8; 512];
	loop {
		let wait = if request.is_empty() { 5000 } else { 250 };
		match tokio::time::timeout(Duration::from_millis(wait), stream.read(&mut buf)).await {
			Ok(Ok(0)) | Err(_) => break,
			Ok(Ok(n)) => request.extend_from_slice(&buf[..n]),
			Ok(Err(e)) => return Err(e.into()),
		}
		if request.len() > 4096 {
			break;
		}
	}
	let upstream = status::upstream(config).await;
	stream
		.write_all(&kick(&ping_reply(
			&request,
			config,
			server,
			upstream.as_deref(),
		)))
		.await?;
	stream.shutdown().await?;
	Ok(())
}

fn rsa_key() -> &'static RsaPrivateKey {
	static KEY: OnceLock<RsaPrivateKey> = OnceLock::new();
	KEY.get_or_init(|| RsaPrivateKey::new(&mut rand::thread_rng(), 1024).expect("an RSA key"))
}

pub async fn warm_up() {
	let _ = tokio::task::spawn_blocking(rsa_key).await;
}

pub async fn handle(stream: TcpStream, config: &Config, first: u8) -> Result<()> {
	if first == 0xfe {
		return ping(stream, config).await;
	}
	let sv = config.server_version;
	let mut client = Framed::new(
		stream,
		Codec {
			threshold: None,
			legacy: Some(Legacy::new(Root::PlayToServer, None)),
			cipher: None,
		},
	);
	let hello = next(&mut client).await?;
	let (cv, username) = if hello[1] == 0 {
		let (who, _) = read_string16(&hello, 1).ok_or_else(|| anyhow!("bad handshake"))?;
		let mut hash = vec![0x02];
		string16(&mut hash, "-");
		client.send(hash).await?;
		let request = next(&mut client).await?;
		let protocol = i32::from_be_bytes(request[1..5].try_into()?);
		let cv =
			legacy_version_of(protocol).ok_or_else(|| anyhow!("unknown protocol {protocol}"))?;
		let (name, _) = read_string16(&request, 5).ok_or_else(|| anyhow!("bad login request"))?;
		let name = if name.is_empty() {
			who.split(';').next().unwrap_or_default().to_owned()
		} else {
			name
		};
		(Some(cv).filter(|v| reachable(sv, *v)), name)
	} else {
		let (name, _) = read_string16(&hello, 2).ok_or_else(|| anyhow!("bad handshake"))?;
		let cv = legacy_version_of(i32::from(hello[1])).filter(|v| reachable(sv, *v));
		(cv, name)
	};
	let Some(cv) = cv else {
		let reason = format!("this proxy speaks {} to pre-netty clients", range(sv));
		client.send(kick(&reason)).await?;
		bail!("{reason}");
	};
	if let Some(legacy) = client.codec_mut().legacy.as_mut() {
		legacy.version = Some(cv);
	}
	if hello[1] != 0 {
		encrypt(&mut client).await?;
		loop {
			let data = next(&mut client).await?;
			if data == [0xcd, 0] {
				break;
			}
		}
	}
	play(client, config, cv, sv, username).await
}

async fn encrypt(client: &mut Conn) -> Result<()> {
	let key = tokio::task::spawn_blocking(rsa_key).await?;
	let public = key
		.to_public_key()
		.to_public_key_der()
		.context("encoding the public key")?;
	let token: [u8; 4] = rand::random();
	let mut request = vec![0xfd];
	string16(&mut request, "-");
	request.extend_from_slice(&(public.as_bytes().len() as i16).to_be_bytes());
	request.extend_from_slice(public.as_bytes());
	request.extend_from_slice(&4i16.to_be_bytes());
	request.extend_from_slice(&token);
	client.send(request).await?;
	let response = next(client).await?;
	if response.first() != Some(&0xfc) {
		bail!("expected the encryption response, got {:#04x}", response[0]);
	}
	let (secret, at) =
		short_bytes(&response, 1).ok_or_else(|| anyhow!("bad encryption response"))?;
	let (verify, _) =
		short_bytes(&response, at).ok_or_else(|| anyhow!("bad encryption response"))?;
	let secret = key
		.decrypt(Pkcs1v15Encrypt, secret)
		.context("decrypting the shared secret")?;
	let verify = key
		.decrypt(Pkcs1v15Encrypt, verify)
		.context("decrypting the verify token")?;
	if verify != token {
		bail!("the client answered with the wrong verify token");
	}
	let secret: [u8; 16] = secret
		.try_into()
		.map_err(|_| anyhow!("the shared secret is not 16 bytes"))?;
	client.send(vec![0xfc, 0, 0, 0, 0]).await?;
	if let Some(legacy) = client.codec_mut().legacy.as_mut() {
		legacy.cipher = Some((Cfb8::new(&secret), Cfb8::new(&secret)));
	}
	Ok(())
}

pub fn login_request(protocol: i32, name: &str) -> Vec<u8> {
	let mut out = vec![0x01];
	out.extend_from_slice(&protocol.to_be_bytes());
	string16(&mut out, name);
	if protocol <= 23 {
		out.extend_from_slice(&0i64.to_be_bytes());
	}
	if protocol != 22 {
		string16(&mut out, "");
	}
	out.extend_from_slice(&0i32.to_be_bytes());
	if protocol >= 28 {
		out.extend_from_slice(&0i32.to_be_bytes());
	} else {
		out.push(0);
	}
	out.extend_from_slice(&[0, 0, 0]);
	out
}

pub async fn connect(
	target: &str,
	host: &str,
	port: u16,
	version: Version,
	username: &str,
) -> Result<Conn> {
	let protocol =
		protocol_of(version).ok_or_else(|| anyhow!("{} is not a pre-netty version", version))?;
	let stream = TcpStream::connect(target)
		.await
		.with_context(|| format!("connecting to {target}"))?;
	stream.set_nodelay(true)?;
	let mut conn = Framed::new(
		stream,
		Codec {
			threshold: None,
			legacy: Some(Legacy::new(Root::PlayToClient, Some(version))),
			cipher: None,
		},
	);
	let refused = |data: &[u8]| match data.first() {
		Some(0xff) => anyhow!(
			"server refused the login: {}",
			read_string16(data, 1).map(|(s, _)| s).unwrap_or_default()
		),
		_ => anyhow!(
			"unexpected login packet {:#04x}",
			data.first().copied().unwrap_or(0)
		),
	};
	if protocol < 39 {
		let mut hello = vec![0x02];
		string16(
			&mut hello,
			&if protocol >= 28 {
				format!("{username};{host}:{port}")
			} else {
				username.to_owned()
			},
		);
		conn.send(hello).await?;
		let hash = next(&mut conn).await?;
		match read_string16(&hash, 1) {
			Some((h, _)) if hash[0] == 0x02 && h == "-" => {}
			Some(_) if hash[0] == 0x02 => {
				bail!("the server is in online mode, set online-mode=false behind a proxy")
			}
			_ => return Err(refused(&hash)),
		}
		conn.send(login_request(protocol, username)).await?;
		return Ok(conn);
	}
	let mut hello = vec![0x02, protocol as u8];
	string16(&mut hello, username);
	string16(&mut hello, host);
	hello.extend_from_slice(&i32::from(port).to_be_bytes());
	conn.send(hello).await?;
	let request = next(&mut conn).await?;
	if request[0] != 0xfd {
		return Err(refused(&request));
	}
	let (server_id, at) =
		read_string16(&request, 1).ok_or_else(|| anyhow!("bad encryption request"))?;
	if server_id != "-" {
		bail!("the server is in online mode, set online-mode=false behind a proxy");
	}
	let (key, at) = short_bytes(&request, at).ok_or_else(|| anyhow!("bad encryption request"))?;
	let (token, _) = short_bytes(&request, at).ok_or_else(|| anyhow!("bad encryption request"))?;
	let key = RsaPublicKey::from_public_key_der(key).context("reading the server key")?;
	let secret: [u8; 16] = rand::random();
	let mut response = vec![0xfc];
	for part in [&secret[..], token] {
		let sealed = key.encrypt(&mut rand::thread_rng(), Pkcs1v15Encrypt, part)?;
		response.extend_from_slice(&(sealed.len() as i16).to_be_bytes());
		response.extend_from_slice(&sealed);
	}
	conn.send(response).await?;
	let done = next(&mut conn).await?;
	if done[0] != 0xfc {
		return Err(refused(&done));
	}
	if let Some(legacy) = conn.codec_mut().legacy.as_mut() {
		legacy.cipher = Some((Cfb8::new(&secret), Cfb8::new(&secret)));
	}
	conn.send(vec![0xcd, 0]).await?;
	Ok(conn)
}

async fn play(
	client: Conn,
	config: &Config,
	cv: Version,
	sv: Version,
	username: String,
) -> Result<()> {
	let online = crate::online::Online::new(config.accounts.find(&username, None), &username);
	let mut p = Proxy::new(client, cv, sv, &config.target, online);
	if let Err(e) = p.connect(&username, false).await {
		p.client
			.send(kick(&format!("can't join the server: {e:#}")))
			.await?;
		return Err(e);
	}
	let (join, early) = join_upstream(&mut p, config, &username, false).await?;
	p.enter(join, early, false).await?;
	relay(p, &username).await
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cfb8_round_trips_in_pieces() {
		let key = [7u8; 16];
		let (mut a, mut b) = (Cfb8::new(&key), Cfb8::new(&key));
		let plain: Vec<u8> = (0..100).collect();
		let mut data = plain.clone();
		a.encrypt(&mut data[..30]);
		a.encrypt(&mut data[30..]);
		assert_ne!(data, plain);
		b.decrypt(&mut data[..1]);
		b.decrypt(&mut data[1..]);
		assert_eq!(data, plain);
	}

	#[test]
	fn handshakes_are_told_apart() {
		let mut old = vec![0x02];
		string16(&mut old, "Notch;localhost:25565");
		assert_eq!(handshake_len(&old), Ok(Frame::Complete(old.len())));
		assert_eq!(handshake_len(&old[..5]), Ok(Frame::Need(old.len())));
		assert_eq!(handshake_len(&old[..1]), Ok(Frame::Need(2)));
		let mut new = vec![0x02, 78];
		string16(&mut new, "Notch");
		string16(&mut new, "localhost");
		new.extend_from_slice(&25565i32.to_be_bytes());
		assert_eq!(handshake_len(&new), Ok(Frame::Complete(new.len())));
		assert_eq!(handshake_len(&new[..8]), Ok(Frame::Need(14)));
		assert!(handshake_len(&[0x07]).is_err());
	}

	#[test]
	fn pings_get_the_format_their_client_reads() {
		let config = Config {
			motd: "\u{a7}ahi".to_owned(),
			..Config::default()
		};
		let server = Version::V1_8;
		assert_eq!(
			ping_reply(&[0xfe], &config, server, None),
			"hi\u{a7}0\u{a7}0"
		);
		let modern = ping_reply(&[0xfe, 0x01], &config, server, None);
		assert_eq!(
			modern,
			"\u{a7}1\x0078\x00pds2 bridge 1.0.1-1.6.4\x00\u{a7}ahi\x000\x000"
		);
		let mut host = vec![0xfe, 0x01, 0xfa];
		string16(&mut host, "MC|PingHost");
		host.extend_from_slice(&[0, 7, 73]);
		assert!(ping_reply(&host, &config, server, None).starts_with("\u{a7}1\x0073\x00"));
		let upstream = Upstream {
			description: serde_json::json!({"text": "Up", "color": "gold", "extra": [" there"]}),
			online: 4,
			max: 50,
			sample: Vec::new(),
			favicon: None,
			protocol: Some(47),
		};
		assert_eq!(
			ping_reply(&[0xfe], &config, server, Some(&upstream)),
			"Up there\u{a7}4\u{a7}50"
		);
		assert_eq!(
			ping_reply(&[0xfe, 0x01], &config, server, Some(&upstream)),
			"\u{a7}1\x0078\x00pds2 bridge 1.0.1-1.6.4\x00\u{a7}6Up there\x004\x0050"
		);
	}
}
