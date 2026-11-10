use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::{anyhow, bail, Context as _, Result};
use futures::{SinkExt, StreamExt};
use md5::{Digest, Md5};
use minecraft_versions::gen::convert::v26_1::{
	configuration_toClient_packet as config_c, configuration_toServer_packet as config_s,
	handshaking_toServer_packet as handshake, handshaking_toServer_packet_set_protocol,
	login_toClient_packet as login_c, login_toServer_packet as login_s,
	login_toServer_packet_login_acknowledged, login_toServer_packet_login_plugin_response,
	packet_common_cookie_response, packet_common_store_cookie, packet_common_transfer,
	play_toClient_SpawnInfo, play_toClient_packet as play_c, play_toClient_packet_login,
	play_toClient_packet_respawn, play_toClient_packet_start_configuration,
	play_toClient_packet_system_chat, play_toServer_packet as play_s,
	status_toClient_packet as status_c, status_toServer_packet as status_s,
};
use minecraft_versions::gen::convert::{packet_name, v1_20_2};
use minecraft_versions::pds::{self, Input, Output};
use minecraft_versions::{
	convert, index_of, reachable, version_of, Root, Session, CONFIGURATION, PROTOCOLS, VERSIONS,
};
use serde_json::Value;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;
use tracing::{info, warn};

pub mod accounts;
pub mod auth;
pub mod codec;
pub mod cookies;
pub mod legacy;
pub mod online;
pub mod signing;
pub mod status;

use codec::{read_varint, Codec};

pub type Conn = Framed<TcpStream, Codec>;

pub const PIVOT: &str = "26.1";

pub const TRANSFERS: &str = "1.20.5";

const MAX_HOPS: usize = 4;

const TRANSFERS_DISABLED: &str = "multiplayer.disconnect.transfers_disabled";

pub struct Config {
	pub target: String,
	pub motd: String,
	pub threshold: usize,
	pub server_version: String,
	pub accounts: accounts::Accounts,
	pub status_cache: Duration,
	pub status: status::Cache,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			target: "127.0.0.1:25565".to_owned(),
			motd: "pds2 bridge".to_owned(),
			threshold: 256,
			server_version: "1.8".to_owned(),
			accounts: accounts::Accounts::default(),
			status_cache: Duration::from_secs(5),
			status: status::Cache::default(),
		}
	}
}

impl Config {
	pub fn server(&self) -> Result<usize> {
		index_of(&self.server_version)
			.ok_or_else(|| anyhow!("unknown server version {}", self.server_version))
	}
}

pub fn decode<T>(data: &[u8], de: fn(&mut Input<'_>) -> pds::Result<T>) -> Result<T> {
	de(&mut Input::new(data)).map_err(|e| anyhow!("malformed packet: {e:?}"))
}

pub fn encode<T>(value: &T, ser: fn(&T, &mut Output) -> pds::Result<()>) -> Result<Vec<u8>> {
	let mut out = Output::new();
	ser(value, &mut out).map_err(|e| anyhow!("can't encode packet: {e:?}"))?;
	Ok(out.into_inner())
}

pub fn packet_id(data: &[u8]) -> Option<i32> {
	read_varint(data).map(|(id, _)| id)
}

pub async fn next(conn: &mut Conn) -> Result<Vec<u8>> {
	conn.next()
		.await
		.ok_or_else(|| anyhow!("connection closed"))?
		.context("reading a packet")
}

pub fn pivot() -> usize {
	index_of(PIVOT).expect("the pivot version is in the merged schema")
}

pub(crate) fn at(version: &str) -> usize {
	index_of(version).expect("a known version")
}

pub fn protocol_of(version: usize) -> Option<i32> {
	PROTOCOLS
		.iter()
		.filter(|(_, v)| *v == version)
		.map(|(p, _)| *p)
		.max()
}

pub fn client_version(protocol: i32, server: usize) -> Option<usize> {
	version_of(protocol).filter(|v| reachable(server, *v))
}

pub fn reach(server: usize) -> (usize, usize) {
	let reach = |v: &usize| reachable(server, *v);
	let low = (0..VERSIONS.len()).find(reach).unwrap_or(server);
	let high = (0..VERSIONS.len()).rev().find(reach).unwrap_or(server);
	(low, high)
}

pub fn client_range(server: usize) -> (&'static str, &'static str) {
	let (low, high) = reach(server);
	(VERSIONS[low], VERSIONS[high])
}

pub fn text_tag(text: &str) -> nbt::Tag {
	nbt::Tag::new(nbt::json::text_to_nbt(&serde_json::json!({ "text": text })))
}

pub fn chat_text(text: &str) -> Result<Vec<u8>> {
	encode(
		&play_c::Ty::system_chat(play_toClient_packet_system_chat::Ty {
			content: text_tag(text),
			isActionBar: false,
		}),
		play_c::serialize,
	)
}

pub fn login_kick(reason: &str) -> Result<Vec<u8>> {
	let reason = serde_json::json!({ "text": reason }).to_string();
	encode(&login_c::Ty::disconnect(reason), login_c::serialize)
}

pub fn offline_uuid_bits(name: &str) -> u128 {
	let mut b: [u8; 16] = Md5::digest(format!("OfflinePlayer:{name}")).into();
	b[6] = (b[6] & 0x0f) | 0x30;
	b[8] = (b[8] & 0x3f) | 0x80;
	u128::from_be_bytes(b)
}

pub fn offline_uuid(name: &str) -> String {
	auth::hyphenated(offline_uuid_bits(name))
}

pub(crate) fn split_target(target: &str) -> Result<(String, u16)> {
	let (host, port) = target
		.rsplit_once(':')
		.ok_or_else(|| anyhow!("{target:?} should be host:port"))?;
	let host = host
		.strip_prefix('[')
		.and_then(|h| h.strip_suffix(']'))
		.unwrap_or(host);
	Ok((host.to_owned(), port.parse().context("bad port")?))
}

fn join_target(host: &str, port: i32) -> String {
	match host.contains(':') {
		true => format!("[{host}]:{port}"),
		false => format!("{host}:{port}"),
	}
}

fn dimension_types() -> &'static [String] {
	static TYPES: OnceLock<Vec<String>> = OnceLock::new();
	TYPES.get_or_init(|| {
		minecraft_versions::registry::login_packet(PIVOT)
			.and_then(|text| serde_json::from_str::<Value>(text).ok())
			.and_then(|login| {
				let entries =
					login["dimensionCodec"]["minecraft:dimension_type"]["entries"].as_array()?;
				Some(
					entries
						.iter()
						.filter_map(|e| e["key"].as_str().map(str::to_owned))
						.collect::<Vec<_>>(),
				)
			})
			.filter(|types| !types.is_empty())
			.unwrap_or_else(|| {
				[
					"minecraft:overworld",
					"minecraft:overworld_caves",
					"minecraft:the_end",
					"minecraft:the_nether",
				]
				.map(str::to_owned)
				.to_vec()
			})
	})
}

fn elsewhere(world: &play_toClient_SpawnInfo::Ty) -> (i32, String) {
	let types = dimension_types();
	let current = usize::try_from(world.dimension)
		.ok()
		.and_then(|i| types.get(i));
	[
		"minecraft:the_nether",
		"minecraft:overworld",
		"minecraft:the_end",
	]
	.iter()
	.filter(|name| current.map(String::as_str) != Some(**name) && world.name != **name)
	.find_map(|name| {
		Some((
			types.iter().position(|t| t == name)? as i32,
			(*name).to_owned(),
		))
	})
	.unwrap_or_else(|| (world.dimension, "pds2:elsewhere".to_owned()))
}

pub fn trick(login: &play_toClient_packet_login::Ty, cv: usize) -> Result<Vec<Vec<u8>>> {
	let mut cx = Session::new();
	cx.set_versions(pivot(), cv);
	let mut down = |data: Vec<u8>| -> Result<Vec<Vec<u8>>> {
		match cv == pivot() {
			true => Ok(vec![data]),
			false => convert(Root::PlayToClient, pivot(), cv, &data, &mut cx)
				.map_err(|e| anyhow!("{PIVOT} -> {}: {e}", VERSIONS[cv])),
		}
	};
	down(encode(
		&play_c::Ty::login(login.clone()),
		play_c::serialize,
	)?)?;
	let mut out = Vec::new();
	for respawn in respawns(&login.worldState) {
		out.extend(down(encode(
			&play_c::Ty::respawn(respawn),
			play_c::serialize,
		)?)?);
	}
	Ok(out)
}

pub fn respawns(world: &play_toClient_SpawnInfo::Ty) -> [play_toClient_packet_respawn::Ty; 2] {
	let (dimension, name) = elsewhere(world);
	let bounce = play_toClient_SpawnInfo::Ty {
		dimension,
		name,
		death: None,
		..world.clone()
	};
	[bounce, world.clone()].map(|w| play_toClient_packet_respawn::Ty {
		worldState: w,
		copyMetadata: 0,
	})
}

enum Caught {
	Pass(Vec<u8>),
	Done,
	Transfer(String),
}

enum Common {
	Store(packet_common_store_cookie::Ty),
	Request(String),
	Transfer(packet_common_transfer::Ty),
}

fn common(root: Root, packet: &[u8]) -> Result<Option<Common>> {
	Ok(match root {
		Root::ConfigurationToClient => match decode(packet, config_c::deserialize)? {
			config_c::Ty::store_cookie(c) => Some(Common::Store(c)),
			config_c::Ty::cookie_request(key) => Some(Common::Request(key)),
			config_c::Ty::transfer(t) => Some(Common::Transfer(t)),
			_ => None,
		},
		Root::PlayToClient => match decode(packet, play_c::deserialize)? {
			play_c::Ty::store_cookie(c) => Some(Common::Store(c)),
			play_c::Ty::cookie_request(key) => Some(Common::Request(key)),
			play_c::Ty::transfer(t) => Some(Common::Transfer(t)),
			_ => None,
		},
		_ => None,
	})
}

pub struct Proxy {
	pub client: Conn,
	pub server: Option<Conn>,
	pub cv: usize,
	pub sv: usize,
	pub target: String,
	pub cookies: cookies::Jar,
	cx: Session,
	online: online::Online,
	trail: Vec<String>,
}

impl Proxy {
	pub(crate) fn new(
		client: Conn,
		cv: usize,
		sv: usize,
		target: &str,
		online: online::Online,
	) -> Self {
		let mut cx = Session::new();
		cx.set_versions(sv, cv);
		Self {
			client,
			server: None,
			cv,
			sv,
			target: target.to_owned(),
			cookies: cookies::Jar::default(),
			cx,
			online,
			trail: Vec::new(),
		}
	}

	async fn read_server(&mut self, root: Root) -> Result<Vec<u8>> {
		let sv = self.sv;
		let read = next(self.server()?).await;
		let data = read.map_err(|e| {
			anyhow!(
				"{e:#} after {}",
				if self.trail.is_empty() {
					"nothing".to_owned()
				} else {
					self.trail.join(", ")
				}
			)
		})?;
		let name = packet_name(root, sv, &data).map_or_else(
			|| format!("{root:?} {:#04x}", packet_id(&data).unwrap_or(-1)),
			|n| format!("{root:?} {n}"),
		);
		if self.trail.len() >= 16 {
			self.trail.remove(0);
		}
		self.trail.push(name);
		Ok(data)
	}

	fn convert(&mut self, root: Root, from: usize, to: usize, data: Vec<u8>) -> Vec<Vec<u8>> {
		if from == to {
			return vec![data];
		}
		match convert(root, from, to, &data, &mut self.cx) {
			Ok(packets) => packets,
			Err(e) => {
				warn!(
					"{root:?} packet {:#04x} {} -> {}: {e}",
					packet_id(&data).unwrap_or(-1),
					VERSIONS[from],
					VERSIONS[to]
				);
				vec![]
			}
		}
	}

	async fn deliver(&mut self, root: Root, from: usize, data: Vec<u8>) -> Result<()> {
		if from == self.sv {
			self.online.observe(root, &data);
		}
		let to = if root.to_client() { self.cv } else { self.sv };
		if !root.to_client() && self.server.is_none() {
			return Ok(());
		}
		let packets = self.convert(root, from, to, data);
		let packets = self.online.sign(root, packets);
		let conn = if root.to_client() {
			&mut self.client
		} else {
			self.server.as_mut().expect("checked above")
		};
		for p in packets {
			conn.feed(p).await?;
		}
		conn.flush().await?;
		Ok(())
	}

	pub async fn send(&mut self, root: Root, from: usize, data: Vec<u8>) -> Result<()> {
		self.deliver(root, from, data).await?;
		self.drain().await
	}

	async fn drain(&mut self) -> Result<()> {
		loop {
			let outgoing = self.cx.take_outgoing();
			if outgoing.is_empty() {
				return Ok(());
			}
			for o in outgoing {
				self.deliver(o.root, o.version, o.data).await?;
			}
		}
	}

	pub async fn to_client(&mut self, from: usize, data: Vec<u8>) -> Result<()> {
		self.send(Root::PlayToClient, from, data).await
	}

	fn server(&mut self) -> Result<&mut Conn> {
		self.server
			.as_mut()
			.ok_or_else(|| anyhow!("no server connection"))
	}

	fn at_pivot(&self, root: Root, from: usize, data: &[u8]) -> Vec<Vec<u8>> {
		if from == pivot() {
			return vec![data.to_vec()];
		}
		convert(root, from, pivot(), data, &mut Session::new()).unwrap_or_default()
	}

	async fn expect_client(&mut self, root: Root, name: &str) -> Result<()> {
		loop {
			let data = next(&mut self.client).await?;
			if packet_name(root, self.cv, &data) == Some(name) {
				return Ok(());
			}
		}
	}

	async fn answer_cookie(&mut self, root: Root, key: String) -> Result<()> {
		let value = self.cookies.get(&key);
		info!(
			"{} asks for cookie {key}, {}",
			self.target,
			if value.is_some() {
				"sending it"
			} else {
				"none stored"
			}
		);
		let response = packet_common_cookie_response::Ty { key, value };
		let data = match root {
			Root::LoginToServer => {
				encode(&login_s::Ty::cookie_response(response), login_s::serialize)?
			}
			Root::ConfigurationToServer => encode(
				&config_s::Ty::cookie_response(response),
				config_s::serialize,
			)?,
			_ => encode(&play_s::Ty::cookie_response(response), play_s::serialize)?,
		};
		self.send(root, pivot(), data).await
	}

	async fn catch(&mut self, root: Root, data: Vec<u8>) -> Result<Caught> {
		if self.sv < at(TRANSFERS) {
			return Ok(Caught::Pass(data));
		}
		let Some("store_cookie" | "cookie_request" | "transfer") =
			packet_name(root, self.sv, &data)
		else {
			return Ok(Caught::Pass(data));
		};
		let reply = match root {
			Root::ConfigurationToClient => Root::ConfigurationToServer,
			_ => Root::PlayToServer,
		};
		for packet in self.at_pivot(root, self.sv, &data) {
			match common(root, &packet)? {
				Some(Common::Store(cookie)) => {
					match self.cookies.store(&cookie.key, cookie.value) {
						Ok(()) => info!("{} stores cookie {}", self.target, cookie.key),
						Err(e) => warn!("ignored a cookie from {}: {e}", self.target),
					}
				}
				Some(Common::Request(key)) => self.answer_cookie(reply, key).await?,
				Some(Common::Transfer(t)) => {
					return Ok(Caught::Transfer(join_target(&t.host, t.port)))
				}
				None => {}
			}
		}
		Ok(Caught::Done)
	}

	async fn connect(&mut self, username: &str, transfer: bool) -> Result<()> {
		let target = self.target.clone();
		let (host, port) = split_target(&target)?;
		self.online.prepare(self.sv).await?;
		if legacy::is_legacy(self.sv) {
			self.server = Some(legacy::connect(&target, &host, port, self.sv, username).await?);
			return Ok(());
		}
		let stream = TcpStream::connect(&target)
			.await
			.with_context(|| format!("connecting to {target}"))?;
		stream.set_nodelay(true)?;
		self.server = Some(Framed::new(stream, Codec::default()));
		let protocol = protocol_of(self.sv)
			.ok_or_else(|| anyhow!("no protocol number for {}", VERSIONS[self.sv]))?;
		let hs = handshake::Ty::set_protocol(handshaking_toServer_packet_set_protocol::Ty {
			protocolVersion: protocol,
			serverHost: host,
			serverPort: port,
			nextState: if transfer && self.sv >= at(TRANSFERS) {
				3
			} else {
				2
			},
		});
		self.server()?
			.send(encode(&hs, handshake::serialize)?)
			.await?;
		let (from, start) = self.online.login_start(self.sv, username)?;
		self.send(Root::LoginToServer, from, start).await?;
		loop {
			let data = self.read_server(Root::LoginToClient).await?;
			let packets = self.convert(Root::LoginToClient, self.sv, pivot(), data);
			self.drain().await?;
			for p in packets {
				match decode(&p, login_c::deserialize)? {
					login_c::Ty::compress(t) => {
						self.server()?.codec_mut().threshold = usize::try_from(t).ok()
					}
					login_c::Ty::success(profile) => {
						self.online.joined(self.sv, &profile);
						if self.sv >= at(CONFIGURATION) {
							let ack = login_s::Ty::login_acknowledged(
								login_toServer_packet_login_acknowledged::Ty {},
							);
							self.send(
								Root::LoginToServer,
								pivot(),
								encode(&ack, login_s::serialize)?,
							)
							.await?;
						}
						return Ok(());
					}
					login_c::Ty::disconnect(reason) if reason.contains(TRANSFERS_DISABLED) => {
						bail!("{target} does not accept transfers (accepts-transfers=false)")
					}
					login_c::Ty::disconnect(reason) => {
						bail!(
							"server refused the login: {}",
							minecraft_versions::custom::localize::legacy(&reason)
						)
					}
					login_c::Ty::encryption_begin(request) => {
						online::encrypt(self, request, username).await?
					}
					login_c::Ty::login_plugin_request(request) => {
						let response = login_s::Ty::login_plugin_response(
							login_toServer_packet_login_plugin_response::Ty {
								messageId: request.messageId,
								data: None,
							},
						);
						self.send(
							Root::LoginToServer,
							pivot(),
							encode(&response, login_s::serialize)?,
						)
						.await?;
					}
					login_c::Ty::cookie_request(key) => {
						self.answer_cookie(Root::LoginToServer, key).await?
					}
					login_c::Ty::default(id) => bail!("unexpected login packet {id}"),
				}
			}
		}
	}

	async fn reconnect(
		&mut self,
		target: &str,
		version: Option<usize>,
		username: &str,
		transfer: bool,
	) -> Result<()> {
		let sv = match version {
			Some(v) => v,
			None => match status::probe(target, self.sv).await {
				Ok(v) => v,
				Err(e) => {
					warn!(
						"can't tell which version {target} runs, trying {}: {e:#}",
						VERSIONS[self.sv]
					);
					self.sv
				}
			},
		};
		if !reachable(sv, self.cv) {
			bail!(
				"{target} runs {}, which {} clients can't reach",
				VERSIONS[sv],
				VERSIONS[self.cv]
			);
		}
		self.server = None;
		self.sv = sv;
		self.target = target.to_owned();
		self.cx = Session::new();
		self.cx.set_versions(sv, self.cv);
		self.trail.clear();
		self.connect(username, transfer).await
	}

	async fn answer_configuration(&mut self) -> Result<Option<String>> {
		let base = at(CONFIGURATION);
		use v1_20_2::configuration_toServer_packet::Ty as S;
		let settings = S::settings(v1_20_2::configuration_toServer_packet_settings::Ty {
			locale: "en_us".to_owned(),
			viewDistance: 8,
			chatFlags: 0,
			chatColors: true,
			skinParts: 0x7f,
			mainHand: 1,
			enableTextFiltering: false,
			enableServerListing: true,
		});
		let mut brand = vec![7];
		brand.extend_from_slice(b"vanilla");
		let brand = S::custom_payload(v1_20_2::configuration_toServer_packet_custom_payload::Ty {
			channel: "minecraft:brand".to_owned(),
			data: brand,
		});
		for p in [brand, settings] {
			self.send(
				Root::ConfigurationToServer,
				base,
				encode(&p, v1_20_2::configuration_toServer_packet::serialize)?,
			)
			.await?;
		}
		loop {
			let data = self.read_server(Root::ConfigurationToClient).await?;
			let data = match self.catch(Root::ConfigurationToClient, data).await? {
				Caught::Pass(data) => data,
				Caught::Done => continue,
				Caught::Transfer(target) => return Ok(Some(target)),
			};
			let name = packet_name(Root::ConfigurationToClient, self.sv, &data);
			if name == Some("disconnect") {
				for p in self.at_pivot(Root::ConfigurationToClient, self.sv, &data) {
					if let config_c::Ty::disconnect(reason) = decode(&p, config_c::deserialize)? {
						let reason = nbt::json::text_from_nbt(&reason.value).to_string();
						bail!(
							"the server disconnected: {}",
							minecraft_versions::custom::localize::legacy(&reason)
						);
					}
				}
			}
			for p in self.convert(Root::ConfigurationToClient, self.sv, base, data) {
				let replies =
					minecraft_versions::custom::configuration_capture_1_20_2(&mut self.cx, &p)
						.map_err(|e| anyhow!("configuration from the server: {e}"))?;
				for reply in replies {
					self.send(Root::ConfigurationToServer, base, reply).await?;
				}
			}
			self.drain().await?;
			if name == Some("finish_configuration") {
				return Ok(None);
			}
		}
	}

	async fn relay_configuration(&mut self) -> Result<Option<String>> {
		let (mut server_done, mut client_done) = (false, false);
		while !(server_done && client_done) {
			tokio::select! {
				data = next(self.server.as_mut().expect("connected")), if !server_done => {
					let data = match self.catch(Root::ConfigurationToClient, data?).await? {
						Caught::Pass(data) => data,
						Caught::Done => continue,
						Caught::Transfer(target) => return Ok(Some(target)),
					};
					let name = packet_name(Root::ConfigurationToClient, self.sv, &data);
					server_done = name == Some("finish_configuration");
					self.send(Root::ConfigurationToClient, self.sv, data).await?;
				}
				data = next(&mut self.client), if !client_done => {
					let data = data?;
					client_done = packet_name(Root::ConfigurationToServer, self.cv, &data) == Some("finish_configuration");
					self.send(Root::ConfigurationToServer, self.cv, data).await?;
				}
			}
		}
		Ok(None)
	}

	async fn configure(&mut self, modern_client: bool, username: &str) -> Result<()> {
		let mut hops = 0;
		loop {
			let next = match (modern_client, self.sv >= at(CONFIGURATION)) {
				(true, true) => self.relay_configuration().await?,
				(false, true) => self.answer_configuration().await?,
				_ => None,
			};
			let Some(target) = next else { return Ok(()) };
			hops += 1;
			if hops > MAX_HOPS {
				bail!("{target} is transfer number {hops} in a row");
			}
			info!("{username} is transferred to {target} while configuring");
			self.reconnect(&target, None, username, true).await?;
		}
	}

	async fn configure_client(&mut self) -> Result<()> {
		let base = at(CONFIGURATION);
		let packets = minecraft_versions::configuration_packets(&mut self.cx)
			.map_err(|e| anyhow!("configuring {}: {e}", VERSIONS[self.cv]))?;
		for p in packets {
			self.send(Root::ConfigurationToClient, base, p).await?;
		}
		self.expect_client(Root::ConfigurationToServer, "finish_configuration")
			.await
	}

	async fn await_join(&mut self) -> Result<(Vec<u8>, Vec<Vec<u8>>)> {
		let mut early = Vec::new();
		loop {
			let data = self.read_server(Root::PlayToClient).await?;
			if packet_name(Root::PlayToClient, self.sv, &data) == Some("login") {
				return Ok((data, early));
			}
			early.push(data);
		}
	}

	async fn say(&mut self, text: &str) -> Result<()> {
		self.to_client(pivot(), chat_text(text)?).await
	}

	fn join_at_pivot(&self, join: &[u8]) -> Result<play_toClient_packet_login::Ty> {
		for p in self.at_pivot(Root::PlayToClient, self.sv, join) {
			if let play_c::Ty::login(login) = decode(&p, play_c::deserialize)? {
				return Ok(login);
			}
		}
		bail!("the new server's join game does not convert to {PIVOT}")
	}

	async fn enter(&mut self, join: Vec<u8>, early: Vec<Vec<u8>>, bounce: bool) -> Result<()> {
		let respawns = match bounce {
			true => trick(&self.join_at_pivot(&join)?, self.cv)?,
			false => Vec::new(),
		};
		if self.cv >= at(CONFIGURATION) && self.sv < at(CONFIGURATION) {
			let held = self.convert(Root::PlayToClient, self.sv, self.cv, join);
			self.configure_client().await?;
			for packet in held {
				self.client.feed(packet).await?;
			}
			self.client.flush().await?;
		} else {
			self.to_client(self.sv, join).await?;
		}
		for packet in respawns {
			self.client.feed(packet).await?;
		}
		self.client.flush().await?;
		for data in early {
			self.to_client(self.sv, data).await?;
		}
		Ok(())
	}

	async fn reconfigure_client(&mut self) -> Result<()> {
		let start =
			play_c::Ty::start_configuration(play_toClient_packet_start_configuration::Ty {});
		self.to_client(pivot(), encode(&start, play_c::serialize)?)
			.await?;
		self.expect_client(Root::PlayToServer, "configuration_acknowledged")
			.await
	}

	async fn switch_server(
		&mut self,
		target: &str,
		version: Option<usize>,
		username: &str,
		transfer: bool,
	) -> Result<(), Switch> {
		let server = self.server.take();
		let chat = self.online.chat.take();
		let (sv, previous) = (self.sv, self.target.clone());
		let cx = std::mem::take(&mut self.cx);
		let modern_client = self.cv >= at(CONFIGURATION);
		let ready = async {
			self.reconnect(target, version, username, transfer).await?;
			if modern_client {
				return Ok(None);
			}
			self.configure(false, username).await?;
			self.await_join().await.map(Some)
		}
		.await;
		let joined = match ready {
			Ok(joined) => joined,
			Err(e) => {
				self.server = server;
				self.online.chat = chat;
				self.sv = sv;
				self.target = previous;
				self.cx = cx;
				self.trail.clear();
				return Err(Switch::Stayed(e));
			}
		};
		drop(server);
		let entered = async {
			let (join, early) = match joined {
				Some(joined) => joined,
				None => {
					self.reconfigure_client().await?;
					self.configure(true, username).await?;
					self.await_join().await?
				}
			};
			self.enter(join, early, true).await
		}
		.await;
		if let Err(e) = entered {
			if modern_client {
				let reason = format!("can't move to {target}: {e:#}");
				let kick = encode(
					&config_c::Ty::disconnect(text_tag(&reason)),
					config_c::serialize,
				)
				.map_err(Switch::Lost)?;
				let _ = self.send(Root::ConfigurationToClient, pivot(), kick).await;
			}
			return Err(Switch::Lost(e));
		}
		Ok(())
	}
}

enum Switch {
	Stayed(anyhow::Error),
	Lost(anyhow::Error),
}

#[derive(Debug, PartialEq)]
enum Command {
	Ping,
	Goto(String, Option<usize>),
	Unknown(String),
}

fn command_of(cv: usize, data: &[u8]) -> Result<Option<Command>> {
	let Some("chat" | "chat_message" | "chat_command" | "chat_command_signed") =
		packet_name(Root::PlayToServer, cv, data)
	else {
		return Ok(None);
	};
	let packets = match cv == pivot() {
		true => vec![data.to_vec()],
		false => {
			convert(Root::PlayToServer, cv, pivot(), data, &mut Session::new()).unwrap_or_default()
		}
	};
	for p in packets {
		let line = match decode(&p, play_s::deserialize)? {
			play_s::Ty::chat_command(command) => command,
			play_s::Ty::chat_command_signed(signed) => signed.command,
			play_s::Ty::chat_message(message) => match message.message.strip_prefix('/') {
				Some(command) => command.to_owned(),
				None => continue,
			},
			_ => continue,
		};
		let Some(rest) = line.strip_prefix("proxy-") else {
			continue;
		};
		let words: Vec<&str> = rest.split_whitespace().collect();
		return Ok(Some(match words.as_slice() {
			["ping", ..] => Command::Ping,
			["goto", target] => Command::Goto((*target).to_owned(), None),
			["goto", target, version] => match index_of(version) {
				Some(v) => Command::Goto((*target).to_owned(), Some(v)),
				None => Command::Unknown(format!("/{line} (unknown version {version})")),
			},
			_ => Command::Unknown(format!("/{line}")),
		}));
	}
	Ok(None)
}

pub async fn serve(listener: TcpListener, config: Arc<Config>) -> Result<()> {
	config.server()?;
	legacy::warm_up().await;
	loop {
		let (stream, peer) = listener.accept().await?;
		let config = config.clone();
		tokio::spawn(async move {
			if let Err(e) = handle(stream, &config).await {
				warn!("{peer}: {e:#}");
			}
		});
	}
}

async fn handle(stream: TcpStream, config: &Config) -> Result<()> {
	stream.set_nodelay(true)?;
	let mut first = [0u8; 1];
	if stream.peek(&mut first).await? == 1 && matches!(first[0], 0xfe | 0x02) {
		return legacy::handle(stream, config, first[0]).await;
	}
	let mut client = Framed::new(stream, Codec::default());
	let handshake::Ty::set_protocol(hs) =
		decode(&next(&mut client).await?, handshake::deserialize)?
	else {
		bail!("expected a handshake");
	};
	match hs.nextState {
		1 => status(client, config, hs.protocolVersion).await,
		2 | 3 => login(client, config, hs).await,
		n => bail!("unknown next state {n}"),
	}
}

async fn status(mut client: Conn, config: &Config, protocol: i32) -> Result<()> {
	loop {
		match decode(&next(&mut client).await?, status_s::deserialize)? {
			status_s::Ty::ping_start(_) => {
				let upstream = status::upstream(config).await;
				let info = status::netty_reply(config, upstream.as_deref(), protocol)?;
				client
					.send(encode(
						&status_c::Ty::server_info(info),
						status_c::serialize,
					)?)
					.await?;
			}
			status_s::Ty::ping(time) => {
				client
					.send(encode(&status_c::Ty::ping(time), status_c::serialize)?)
					.await?;
				return Ok(());
			}
			status_s::Ty::default(id) => bail!("unexpected status packet {id}"),
		}
	}
}

pub(crate) async fn join_upstream(
	p: &mut Proxy,
	config: &Config,
	username: &str,
	modern_client: bool,
) -> Result<(Vec<u8>, Vec<Vec<u8>>)> {
	p.configure(modern_client, username).await?;
	let (join, early) = p.await_join().await?;
	info!(
		"{username} joined {} ({}) with {}",
		p.target, VERSIONS[p.sv], VERSIONS[p.cv]
	);
	if p.target != config.target {
		info!(
			"{username} was transferred from {} on the way",
			config.target
		);
	}
	Ok((join, early))
}

async fn login(
	mut client: Conn,
	config: &Config,
	hs: handshaking_toServer_packet_set_protocol::Ty,
) -> Result<()> {
	let sv = config.server()?;
	let start = next(&mut client).await?;
	let Some(cv) = client_version(hs.protocolVersion, sv) else {
		let (low, high) = client_range(sv);
		let reason = format!(
			"this proxy speaks {low} to {high}, not protocol {}",
			hs.protocolVersion
		);
		client.send(login_kick(&reason)?).await?;
		bail!("{reason}");
	};
	let mut p = Proxy::new(client, cv, sv, &config.target, online::Online::default());
	let mut player = None;
	for data in p.convert(Root::LoginToServer, cv, pivot(), start) {
		if let login_s::Ty::login_start(start) = decode(&data, login_s::deserialize)? {
			player = Some(start);
		}
	}
	let player = player.ok_or_else(|| anyhow!("expected login start"))?;
	let username = player.username;
	let claimed = Some(player.playerUUID).filter(|uuid| *uuid != 0);
	p.online = online::Online::new(config.accounts.find(&username, claimed), &username);
	if let Err(e) = p.connect(&username, false).await {
		p.send(
			Root::LoginToClient,
			pivot(),
			login_kick(&format!("can't join the server: {e:#}"))?,
		)
		.await?;
		return Err(e);
	}
	if cv >= at("1.8") {
		let threshold = config.threshold;
		let compress = encode(&login_c::Ty::compress(threshold as i32), login_c::serialize)?;
		p.send(Root::LoginToClient, pivot(), compress).await?;
		p.client.codec_mut().threshold = Some(threshold);
	}
	let success = p.online.success(&username);
	p.send(
		Root::LoginToClient,
		pivot(),
		encode(&login_c::Ty::success(success), login_c::serialize)?,
	)
	.await?;
	let modern_client = cv >= at(CONFIGURATION);
	if modern_client {
		p.expect_client(Root::LoginToServer, "login_acknowledged")
			.await?;
	}
	let (join, early) = join_upstream(&mut p, config, &username, modern_client).await?;
	p.enter(join, early, false).await?;
	relay(p, &username).await
}

async fn moved(
	p: &mut Proxy,
	outcome: Result<(), Switch>,
	username: &str,
	target: &str,
	transfer: bool,
) -> Result<()> {
	let how = if transfer { "transfer" } else { "move" };
	match outcome {
		Ok(()) => {
			info!("{username} made the {how} to {target} ({})", VERSIONS[p.sv]);
			Ok(())
		}
		Err(Switch::Stayed(e)) => {
			warn!(
				"{username} stays on {}, the {how} to {target} failed: {e:#}",
				p.target
			);
			p.say(&format!("can't {how} to {target}: {e:#}")).await
		}
		Err(Switch::Lost(e)) => {
			Err(e.context(format!("{username} was lost on the {how} to {target}")))
		}
	}
}

async fn relay(mut p: Proxy, username: &str) -> Result<()> {
	loop {
		tokio::select! {
			from_client = p.client.next() => {
				let Some(data) = from_client else {
					info!("{username} left");
					return Ok(());
				};
				let data = data?;
				match command_of(p.cv, &data)? {
					None => {
						let cv = p.cv;
						p.send(Root::PlayToServer, cv, data).await?
					}
					Some(Command::Ping) => p.say("Pong").await?,
					Some(Command::Unknown(message)) => p.say(&format!("unknown proxy command: {message}")).await?,
					Some(Command::Goto(target, version)) => {
						let outcome = p.switch_server(&target, version, username, false).await;
						moved(&mut p, outcome, username, &target, false).await?
					}
				}
			}
			from_server = p.server.as_mut().expect("connected").next() => {
				let Some(data) = from_server else {
					let kick = play_c::Ty::kick_disconnect(text_tag("the server closed the connection"));
					p.to_client(pivot(), encode(&kick, play_c::serialize)?).await?;
					return Ok(());
				};
				match p.catch(Root::PlayToClient, data?).await? {
					Caught::Pass(data) => {
						let sv = p.sv;
						p.to_client(sv, data).await?
					}
					Caught::Done => {}
					Caught::Transfer(target) => {
						let outcome = p.switch_server(&target, None, username, true).await;
						moved(&mut p, outcome, username, &target, true).await?
					}
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn world(dimension: i32, name: &str) -> play_toClient_SpawnInfo::Ty {
		play_toClient_SpawnInfo::Ty {
			dimension,
			name: name.to_owned(),
			hashedSeed: 5,
			gamemode: minecraft_versions::gen::convert::v26_1::play_toClient_SpawnInfo__gamemode::Ty::creative,
			previousGamemode: 255,
			isDebug: false,
			isFlat: true,
			death: None,
			portalCooldown: 0,
			seaLevel: 63,
		}
	}

	#[test]
	fn the_bounce_goes_to_another_dimension_type_and_world() {
		assert_eq!(
			&dimension_types()[..4],
			[
				"minecraft:overworld",
				"minecraft:overworld_caves",
				"minecraft:the_end",
				"minecraft:the_nether"
			]
		);
		let [bounce, back] = respawns(&world(0, "minecraft:overworld"));
		assert_eq!(
			(bounce.worldState.dimension, bounce.worldState.name.as_str()),
			(3, "minecraft:the_nether")
		);
		assert_eq!(back.worldState, world(0, "minecraft:overworld"));
		let [bounce, _] = respawns(&world(3, "minecraft:the_nether"));
		assert_eq!(
			(bounce.worldState.dimension, bounce.worldState.name.as_str()),
			(0, "minecraft:overworld")
		);
		let [bounce, _] = respawns(&world(0, "minecraft:the_nether"));
		assert_eq!(
			(bounce.worldState.dimension, bounce.worldState.name.as_str()),
			(2, "minecraft:the_end")
		);
		let [bounce, _] = respawns(&world(2, "custom:end"));
		assert_eq!(
			(bounce.worldState.dimension, bounce.worldState.name.as_str()),
			(3, "minecraft:the_nether")
		);
	}

	fn login(world: play_toClient_SpawnInfo::Ty) -> play_toClient_packet_login::Ty {
		play_toClient_packet_login::Ty {
			entityId: 7,
			isHardcore: false,
			worldNames: [
				"minecraft:overworld",
				"minecraft:the_nether",
				"minecraft:the_end",
			]
			.map(str::to_owned)
			.to_vec(),
			maxPlayers: 20,
			viewDistance: 8,
			simulationDistance: 8,
			reducedDebugInfo: false,
			enableRespawnScreen: true,
			doLimitedCrafting: false,
			worldState: world,
			enforcesSecureChat: false,
		}
	}

	fn clients() -> impl Iterator<Item = usize> {
		(0..VERSIONS.len()).filter(|v| reachable(pivot(), *v))
	}

	fn names(root: Root, version: usize, packets: &[Vec<u8>]) -> Vec<&'static str> {
		packets
			.iter()
			.map(|p| packet_name(root, version, p).unwrap_or("?"))
			.collect()
	}

	#[test]
	fn the_trick_reaches_every_client_as_two_respawns() {
		assert!(clients().count() > 50);
		for cv in clients() {
			for w in [
				world(0, "minecraft:overworld"),
				world(3, "minecraft:the_nether"),
			] {
				let out =
					trick(&login(w), cv).unwrap_or_else(|e| panic!("{}: {e:#}", VERSIONS[cv]));
				assert_eq!(
					names(Root::PlayToClient, cv, &out),
					["respawn", "respawn"],
					"{}",
					VERSIONS[cv]
				);
			}
		}
	}

	#[test]
	fn old_clients_see_the_dimension_change_twice() {
		use minecraft_versions::gen::convert::{v1_0_1, v1_12_2, v1_16, v1_20_3, v1_8};
		let overworld = login(world(0, "minecraft:overworld"));
		let nether = login(world(3, "minecraft:the_nether"));
		let dims = |l: &play_toClient_packet_login::Ty, v: &str| -> Vec<String> {
			trick(l, at(v))
				.unwrap()
				.iter()
				.map(|p| match v {
					"1.0.1" => {
						match decode(p, v1_0_1::play_toClient_packet::deserialize).unwrap() {
							v1_0_1::play_toClient_packet::Ty::respawn(r) => r.dimension.to_string(),
							other => panic!("{other:?}"),
						}
					}
					"1.8" => match decode(p, v1_8::play_toClient_packet::deserialize).unwrap() {
						v1_8::play_toClient_packet::Ty::respawn(r) => r.dimension.to_string(),
						other => panic!("{other:?}"),
					},
					"1.12.2" => {
						match decode(p, v1_12_2::play_toClient_packet::deserialize).unwrap() {
							v1_12_2::play_toClient_packet::Ty::respawn(r) => {
								r.dimension.to_string()
							}
							other => panic!("{other:?}"),
						}
					}
					"1.16" => match decode(p, v1_16::play_toClient_packet::deserialize).unwrap() {
						v1_16::play_toClient_packet::Ty::respawn(r) => r.worldName,
						other => panic!("{other:?}"),
					},
					_ => match decode(p, v1_20_3::play_toClient_packet::deserialize).unwrap() {
						v1_20_3::play_toClient_packet::Ty::respawn(r) => {
							format!("{} {}", r.dimension, r.worldName)
						}
						other => panic!("{other:?}"),
					},
				})
				.collect()
		};
		for v in ["1.0.1", "1.8", "1.12.2"] {
			assert_eq!(dims(&overworld, v), ["-1", "0"], "{v}");
			assert_eq!(dims(&nether, v), ["0", "-1"], "{v}");
		}
		assert_eq!(
			dims(&overworld, "1.16"),
			["minecraft:the_nether", "minecraft:overworld"]
		);
		assert_eq!(
			dims(&nether, "1.20.3"),
			[
				"minecraft:overworld minecraft:overworld",
				"minecraft:the_nether minecraft:the_nether"
			]
		);
	}

	#[test]
	fn proxy_commands_are_read_from_every_client() {
		let line = encode(
			&play_s::Ty::chat_command("proxy-goto example.org:25565 1.8".to_owned()),
			play_s::serialize,
		)
		.unwrap();
		let plain = encode(
			&play_s::Ty::chat_message(
				minecraft_versions::gen::convert::v26_1::play_toServer_packet_chat_message::Ty {
					message: "/proxy-goto".to_owned(),
					timestamp: 0,
					salt: 0,
					signature: None,
					offset: 0,
					acknowledged: vec![0; 3],
					checksum: 0,
				},
			),
			play_s::serialize,
		)
		.unwrap();
		for cv in clients() {
			let sent =
				convert(Root::PlayToServer, pivot(), cv, &line, &mut Session::new()).unwrap();
			let [sent] = sent.as_slice() else {
				panic!("{}: {sent:?}", VERSIONS[cv])
			};
			let command = command_of(cv, sent).unwrap();
			assert_eq!(
				command,
				Some(Command::Goto(
					"example.org:25565".to_owned(),
					Some(at("1.8"))
				)),
				"{}",
				VERSIONS[cv]
			);
		}
		assert_eq!(
			command_of(pivot(), &plain).unwrap(),
			Some(Command::Unknown("/proxy-goto".to_owned()))
		);
	}

	#[test]
	fn replies_and_kicks_reach_every_client() {
		let chat = chat_text("Pong").unwrap();
		let kick = encode(
			&play_c::Ty::kick_disconnect(text_tag("closed")),
			play_c::serialize,
		)
		.unwrap();
		for cv in clients() {
			let out = convert(Root::PlayToClient, pivot(), cv, &chat, &mut Session::new()).unwrap();
			assert!(
				matches!(
					names(Root::PlayToClient, cv, &out)[..],
					["chat" | "system_chat"]
				),
				"{}",
				VERSIONS[cv]
			);
			let out = convert(Root::PlayToClient, pivot(), cv, &kick, &mut Session::new()).unwrap();
			assert_eq!(
				names(Root::PlayToClient, cv, &out),
				["kick_disconnect"],
				"{}",
				VERSIONS[cv]
			);
		}
		let refusal = login_kick("no").unwrap();
		for cv in clients().filter(|v| !legacy::is_legacy(*v)) {
			let out = convert(
				Root::LoginToClient,
				pivot(),
				cv,
				&refusal,
				&mut Session::new(),
			)
			.unwrap();
			assert_eq!(
				names(Root::LoginToClient, cv, &out),
				["disconnect"],
				"{}",
				VERSIONS[cv]
			);
		}
	}

	#[test]
	fn targets_round_trip() {
		assert_eq!(
			split_target("[::1]:25565").unwrap(),
			("::1".to_owned(), 25565)
		);
		assert_eq!(join_target("::1", 25565), "[::1]:25565");
		assert_eq!(join_target("example.org", 1), "example.org:1");
	}
}
