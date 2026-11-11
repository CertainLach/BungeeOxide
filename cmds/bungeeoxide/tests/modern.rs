use bungeeoxide_converter::Version;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use bungeeoxide::codec::Codec;
use bungeeoxide::legacy::string16;
use bungeeoxide::{decode, encode, next, protocol_of, serve, text_tag, Config, Conn};
use bungeeoxide_converter::gen::convert::packet_name;
use bungeeoxide_converter::gen::convert::v1_8;
use bungeeoxide_converter::gen::convert::v26_1::{
	configuration_to_client_packet as config_c,
	configuration_to_client_packet_finish_configuration,
	configuration_to_server_packet as config_s,
	configuration_to_server_packet_finish_configuration, handshaking_to_server_packet as handshake,
	handshaking_to_server_packet_set_protocol, login_to_client_packet as login_c,
	login_to_client_packet_success, login_to_server_packet as login_s,
	login_to_server_packet_login_acknowledged, login_to_server_packet_login_start,
	packet_common_store_cookie, packet_common_transfer, play_to_client_packet as play_c,
	play_to_client_packet_login, play_to_client_packet_system_chat, play_to_client_spawn_info,
	play_to_client_spawn_info_gamemode, play_to_server_packet as play_s,
	play_to_server_packet_chat_message, play_to_server_packet_configuration_acknowledged,
	status_to_client_packet as status_c, status_to_server_packet as status_s,
	status_to_server_packet_ping_start,
};
use bungeeoxide_converter::Root;
use futures::SinkExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::codec::Framed;

const WAIT: Duration = Duration::from_secs(10);

fn modern() -> Version {
	Version::V26_1
}

fn protocol(version: Version) -> i32 {
	protocol_of(version).unwrap()
}

#[derive(Clone)]
struct Mock {
	name: &'static str,
	dimension: i32,
	world: &'static str,
	transfers: bool,
	forward: Option<u16>,
	events: mpsc::UnboundedSender<String>,
}

async fn mock(
	name: &'static str,
	dimension: i32,
	world: &'static str,
	transfers: bool,
) -> (SocketAddr, mpsc::UnboundedReceiver<String>) {
	start(name, dimension, world, transfers, None).await
}

async fn start(
	name: &'static str,
	dimension: i32,
	world: &'static str,
	transfers: bool,
	forward: Option<u16>,
) -> (SocketAddr, mpsc::UnboundedReceiver<String>) {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let (events, received) = mpsc::unbounded_channel();
	let mock = Mock {
		name,
		dimension,
		world,
		transfers,
		forward,
		events,
	};
	tokio::spawn(async move {
		loop {
			let (stream, _) = listener.accept().await.unwrap();
			let mock = mock.clone();
			tokio::spawn(async move {
				match mock.serve(Framed::new(stream, Codec::default())).await {
					Err(e) if !format!("{e:#}").contains("connection closed") => {
						mock.event(format!("error {e:#}"))
					}
					_ => {}
				}
			});
		}
	});
	(addr, received)
}

fn sys(text: &str) -> Result<Vec<u8>> {
	encode(
		&play_c::Ty::SystemChat(play_to_client_packet_system_chat::Ty {
			content: text_tag(text),
			is_action_bar: false,
		}),
		play_c::serialize,
	)
}

fn cookie_text(value: &Option<Vec<u8>>) -> String {
	value.as_ref().map_or("none".to_owned(), |v| {
		String::from_utf8_lossy(v).into_owned()
	})
}

async fn until(conn: &mut Conn, root: Root, name: &str) -> Result<Vec<u8>> {
	loop {
		let data = tokio::time::timeout(WAIT, next(conn)).await??;
		if packet_name(root, Version::V26_1, &data) == Some(name) {
			return Ok(data);
		}
	}
}

impl Mock {
	fn event(&self, text: String) {
		let _ = self.events.send(format!("{} {text}", self.name));
	}

	async fn serve(&self, mut conn: Conn) -> Result<()> {
		let handshake::Ty::SetProtocol(hs) =
			decode(&next(&mut conn).await?, handshake::deserialize)?
		else {
			bail!("handshake");
		};
		if hs.next_state == 1 {
			return self.status(conn).await;
		}
		self.event(format!("intent {}", hs.next_state));
		let login_s::Ty::LoginStart(start) = decode(&next(&mut conn).await?, login_s::deserialize)?
		else {
			bail!("login start");
		};
		if hs.next_state == 3 && !self.transfers {
			let refusal =
				serde_json::json!({ "translate": "multiplayer.disconnect.transfers_disabled" })
					.to_string();
			conn.send(encode(
				&login_c::Ty::Disconnect(refusal),
				login_c::serialize,
			)?)
			.await?;
			return Ok(());
		}
		conn.send(encode(
			&login_c::Ty::CookieRequest("test:k".to_owned()),
			login_c::serialize,
		)?)
		.await?;
		let login_s::Ty::CookieResponse(r) = decode(
			&until(&mut conn, Root::LoginToServer, "cookie_response").await?,
			login_s::deserialize,
		)?
		else {
			bail!("cookie response");
		};
		self.event(format!(
			"login cookie {} = {}",
			r.key,
			cookie_text(&r.value)
		));
		let success = login_to_client_packet_success::Ty {
			uuid: start.player_uuid,
			username: start.username,
			properties: Vec::new(),
		};
		conn.send(encode(&login_c::Ty::Success(success), login_c::serialize)?)
			.await?;
		until(&mut conn, Root::LoginToServer, "login_acknowledged").await?;
		let store = config_c::Ty::StoreCookie(packet_common_store_cookie::Ty {
			key: "test:config".to_owned(),
			value: self.name.as_bytes().to_vec(),
		});
		conn.send(encode(&store, config_c::serialize)?).await?;
		conn.send(encode(
			&config_c::Ty::CookieRequest("test:config".to_owned()),
			config_c::serialize,
		)?)
		.await?;
		let config_s::Ty::CookieResponse(r) = decode(
			&until(&mut conn, Root::ConfigurationToServer, "cookie_response").await?,
			config_s::deserialize,
		)?
		else {
			bail!("cookie response");
		};
		self.event(format!(
			"config cookie {} = {}",
			r.key,
			cookie_text(&r.value)
		));
		if let Some(port) = self.forward {
			let transfer = config_c::Ty::Transfer(packet_common_transfer::Ty {
				host: "127.0.0.1".to_owned(),
				port: i32::from(port),
			});
			conn.send(encode(&transfer, config_c::serialize)?).await?;
			loop {
				next(&mut conn).await?;
			}
		}
		let finish = config_c::Ty::FinishConfiguration(
			configuration_to_client_packet_finish_configuration::Ty {},
		);
		conn.send(encode(&finish, config_c::serialize)?).await?;
		until(
			&mut conn,
			Root::ConfigurationToServer,
			"finish_configuration",
		)
		.await?;
		let join = play_c::Ty::Login(play_to_client_packet_login::Ty {
			entity_id: 7,
			is_hardcore: false,
			world_names: [
				"minecraft:overworld",
				"minecraft:the_nether",
				"minecraft:the_end",
			]
			.map(str::to_owned)
			.to_vec(),
			max_players: 20,
			view_distance: 8,
			simulation_distance: 8,
			reduced_debug_info: false,
			enable_respawn_screen: true,
			do_limited_crafting: false,
			world_state: play_to_client_spawn_info::Ty {
				dimension: self.dimension,
				name: self.world.to_owned(),
				hashed_seed: 1,
				gamemode: play_to_client_spawn_info_gamemode::Ty::Creative,
				previous_gamemode: 255,
				is_debug: false,
				is_flat: true,
				death: None,
				portal_cooldown: 0,
				sea_level: 63,
			},
			enforces_secure_chat: false,
		});
		conn.send(encode(&join, play_c::serialize)?).await?;
		self.event("joined".to_owned());
		loop {
			let text = match decode(&next(&mut conn).await?, play_s::deserialize)? {
				play_s::Ty::ChatMessage(m) => m.message,
				play_s::Ty::ChatCommand(c) => format!("/{c}"),
				play_s::Ty::CookieResponse(r) => {
					let text = format!("play cookie {} = {}", r.key, cookie_text(&r.value));
					self.event(text.clone());
					conn.send(sys(&format!("{}: {text}", self.name))?).await?;
					continue;
				}
				_ => continue,
			};
			let words: Vec<&str> = text.split_whitespace().collect();
			let reply = match words.as_slice() {
				["store", key, value] => play_c::Ty::StoreCookie(packet_common_store_cookie::Ty {
					key: (*key).to_owned(),
					value: value.as_bytes().to_vec(),
				}),
				["ask", key] => play_c::Ty::CookieRequest((*key).to_owned()),
				["transfer", host, port] => play_c::Ty::Transfer(packet_common_transfer::Ty {
					host: (*host).to_owned(),
					port: port.parse()?,
				}),
				_ => {
					conn.send(sys(&format!("{}: {text}", self.name))?).await?;
					continue;
				}
			};
			conn.send(encode(&reply, play_c::serialize)?).await?;
		}
	}

	async fn status(&self, mut conn: Conn) -> Result<()> {
		let status_s::Ty::PingStart(_) = decode(&next(&mut conn).await?, status_s::deserialize)?
		else {
			bail!("status request");
		};
		let info = serde_json::json!({
			"version": { "name": Version::V26_1.name(), "protocol": protocol(Version::V26_1) },
			"players": { "max": 100, "online": 3, "sample": [{ "name": "Steve", "id": "8667ba71-b85a-4004-af54-457a9734eed7" }] },
			"description": { "text": "", "extra": [
				{ "text": "Modern ", "color": "#ff5555", "shadow_color": -16777216 },
				{ "text": self.name, "bold": true, "font": "minecraft:uniform" },
			] },
			"favicon": "data:image/png;base64,iVBORw0KGgo=",
		});
		conn.send(encode(
			&status_c::Ty::ServerInfo(info.to_string()),
			status_c::serialize,
		)?)
		.await?;
		Ok(())
	}
}

async fn bridge(target: SocketAddr) -> SocketAddr {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let config = Arc::new(Config {
		target: target.to_string(),
		motd: "modern bridge".to_owned(),
		threshold: 64,
		server_version: Version::V26_1,
		..Config::default()
	});
	tokio::spawn(serve(listener, config));
	addr
}

async fn events(rx: &mut mpsc::UnboundedReceiver<String>, n: usize) -> Vec<String> {
	let mut out = Vec::new();
	for _ in 0..n {
		match tokio::time::timeout(WAIT, rx.recv()).await {
			Ok(Some(e)) => out.push(e),
			_ => break,
		}
	}
	out
}

struct Client {
	conn: Conn,
	seen: Vec<&'static str>,
	configured: usize,
}

impl Client {
	async fn join(proxy: SocketAddr) -> Result<Client> {
		let mut conn = Framed::new(TcpStream::connect(proxy).await?, Codec::default());
		let hs = handshake::Ty::SetProtocol(handshaking_to_server_packet_set_protocol::Ty {
			protocol_version: protocol(Version::V26_1),
			server_host: "localhost".to_owned(),
			server_port: proxy.port(),
			next_state: 2,
		});
		conn.send(encode(&hs, handshake::serialize)?).await?;
		let start = login_s::Ty::LoginStart(login_to_server_packet_login_start::Ty {
			username: "modern".to_owned(),
			player_uuid: 5,
		});
		conn.send(encode(&start, login_s::serialize)?).await?;
		loop {
			match decode(
				&tokio::time::timeout(WAIT, next(&mut conn)).await??,
				login_c::deserialize,
			)? {
				login_c::Ty::Compress(t) => conn.codec_mut().threshold = usize::try_from(t).ok(),
				login_c::Ty::Success(s) => {
					assert_eq!(s.username, "modern");
					break;
				}
				other => bail!("login: {other:?}"),
			}
		}
		let ack = login_s::Ty::LoginAcknowledged(login_to_server_packet_login_acknowledged::Ty {});
		conn.send(encode(&ack, login_s::serialize)?).await?;
		let mut client = Client {
			conn,
			seen: Vec::new(),
			configured: 0,
		};
		client.configure().await?;
		Ok(client)
	}

	async fn configure(&mut self) -> Result<()> {
		self.configured += 1;
		loop {
			let data = tokio::time::timeout(WAIT, next(&mut self.conn)).await??;
			let name = packet_name(Root::ConfigurationToClient, modern(), &data).unwrap_or("?");
			self.seen.push(name);
			if name == "finish_configuration" {
				break;
			}
		}
		let finish = config_s::Ty::FinishConfiguration(
			configuration_to_server_packet_finish_configuration::Ty {},
		);
		self.conn
			.send(encode(&finish, config_s::serialize)?)
			.await?;
		Ok(())
	}

	async fn packet(&mut self) -> Result<play_c::Ty> {
		loop {
			let data = tokio::time::timeout(WAIT, next(&mut self.conn)).await??;
			self.seen
				.push(packet_name(Root::PlayToClient, modern(), &data).unwrap_or("?"));
			let packet = decode(&data, play_c::deserialize)?;
			if let play_c::Ty::StartConfiguration(_) = packet {
				let ack = play_s::Ty::ConfigurationAcknowledged(
					play_to_server_packet_configuration_acknowledged::Ty {},
				);
				self.conn.send(encode(&ack, play_s::serialize)?).await?;
				self.configure().await?;
				continue;
			}
			return Ok(packet);
		}
	}

	async fn world(&mut self) -> Result<(i32, String)> {
		loop {
			match self.packet().await? {
				play_c::Ty::Login(l) => return Ok((l.world_state.dimension, l.world_state.name)),
				play_c::Ty::Respawn(r) => return Ok((r.world_state.dimension, r.world_state.name)),
				_ => {}
			}
		}
	}

	async fn chat(&mut self) -> Result<String> {
		loop {
			if let play_c::Ty::SystemChat(c) = self.packet().await? {
				return Ok(bungeeoxide_nbt::json::text_from_nbt(&c.content.value).to_string());
			}
		}
	}

	async fn say(&mut self, text: &str) -> Result<()> {
		let packet = match text.strip_prefix('/') {
			Some(command) => play_s::Ty::ChatCommand(command.to_owned()),
			None => play_s::Ty::ChatMessage(play_to_server_packet_chat_message::Ty {
				message: text.to_owned(),
				timestamp: 0,
				salt: 0,
				signature: None,
				offset: 0,
				acknowledged: vec![0; 3],
				checksum: 0,
			}),
		};
		self.conn.send(encode(&packet, play_s::serialize)?).await?;
		Ok(())
	}
}

#[tokio::test]
async fn modern_clients_transfer_and_keep_their_cookies() -> Result<()> {
	let (a, mut a_events) = mock("a", 0, "minecraft:overworld", false).await;
	let (b, mut b_events) = mock("b", 3, "minecraft:the_nether", true).await;
	let (c, mut c_events) = mock("c", 0, "minecraft:overworld", false).await;
	let proxy = bridge(a).await;
	let mut client = Client::join(proxy).await?;
	assert_eq!(
		events(&mut a_events, 4).await,
		[
			"a intent 2",
			"a login cookie test:k = none",
			"a config cookie test:config = a",
			"a joined"
		]
	);
	assert_eq!(client.world().await?, (0, "minecraft:overworld".to_owned()));

	client.say("store test:k v1").await?;
	client.say("ask test:k").await?;
	assert!(client.chat().await?.contains("a: play cookie test:k = v1"));
	assert_eq!(
		events(&mut a_events, 1).await,
		["a play cookie test:k = v1"]
	);

	client
		.say(&format!("transfer 127.0.0.1 {}", b.port()))
		.await?;
	assert_eq!(
		client.world().await?,
		(3, "minecraft:the_nether".to_owned())
	);
	assert_eq!(client.world().await?, (0, "minecraft:overworld".to_owned()));
	assert_eq!(
		client.world().await?,
		(3, "minecraft:the_nether".to_owned())
	);
	assert_eq!(
		events(&mut b_events, 4).await,
		[
			"b intent 3",
			"b login cookie test:k = v1",
			"b config cookie test:config = b",
			"b joined"
		]
	);
	client.say("hello").await?;
	assert!(client.chat().await?.contains("b: hello"));

	client
		.say(&format!("transfer 127.0.0.1 {}", c.port()))
		.await?;
	assert_eq!(events(&mut c_events, 1).await, ["c intent 3"]);
	let refusal = client.chat().await?;
	assert!(refusal.contains("does not accept transfers"), "{refusal}");
	client.say("still here").await?;
	assert!(client.chat().await?.contains("b: still here"));

	client.say(&format!("/proxy-goto {a}")).await?;
	assert_eq!(client.world().await?, (0, "minecraft:overworld".to_owned()));
	assert_eq!(
		client.world().await?,
		(3, "minecraft:the_nether".to_owned())
	);
	assert_eq!(client.world().await?, (0, "minecraft:overworld".to_owned()));
	assert_eq!(
		events(&mut a_events, 4).await,
		[
			"a intent 2",
			"a login cookie test:k = v1",
			"a config cookie test:config = a",
			"a joined"
		]
	);
	client.say("back").await?;
	assert!(client.chat().await?.contains("a: back"));
	assert_eq!(client.configured, 3);
	for hidden in ["store_cookie", "cookie_request", "transfer"] {
		assert!(
			!client.seen.contains(&hidden),
			"{hidden} reached the client: {:?}",
			client.seen
		);
	}
	Ok(())
}

async fn join_1_8(proxy: SocketAddr) -> Result<Conn> {
	use v1_8::{
		handshaking_to_server_packet as hs8, login_to_client_packet as lc8,
		login_to_server_packet as ls8,
	};
	let mut conn = Framed::new(TcpStream::connect(proxy).await?, Codec::default());
	let hs = hs8::Ty::SetProtocol(v1_8::handshaking_to_server_packet_set_protocol::Ty {
		protocol_version: 47,
		server_host: "localhost".to_owned(),
		server_port: proxy.port(),
		next_state: 2,
	});
	conn.send(encode(&hs, hs8::serialize)?).await?;
	conn.send(encode(
		&ls8::Ty::LoginStart("old".to_owned()),
		ls8::serialize,
	)?)
	.await?;
	loop {
		match decode(
			&tokio::time::timeout(WAIT, next(&mut conn)).await??,
			lc8::deserialize,
		)? {
			lc8::Ty::Compress(t) => conn.codec_mut().threshold = usize::try_from(t).ok(),
			lc8::Ty::Success(_) => return Ok(conn),
			other => bail!("login: {other:?}"),
		}
	}
}

async fn play_1_8(conn: &mut Conn) -> Result<v1_8::play_to_client_packet::Ty> {
	decode(
		&tokio::time::timeout(WAIT, next(conn)).await??,
		v1_8::play_to_client_packet::deserialize,
	)
}

async fn chat_1_8(conn: &mut Conn) -> Result<String> {
	loop {
		if let v1_8::play_to_client_packet::Ty::Chat(c) = play_1_8(conn).await? {
			return Ok(c.message);
		}
	}
}

async fn dimensions_1_8(conn: &mut Conn, n: usize) -> Result<Vec<String>> {
	let mut out = Vec::new();
	while out.len() < n {
		match play_1_8(conn).await? {
			v1_8::play_to_client_packet::Ty::Login(l) => out.push(format!("login {}", l.dimension)),
			v1_8::play_to_client_packet::Ty::Respawn(r) => {
				out.push(format!("respawn {}", r.dimension))
			}
			_ => {}
		}
	}
	Ok(out)
}

async fn say_1_8(conn: &mut Conn, text: &str) -> Result<()> {
	conn.send(encode(
		&v1_8::play_to_server_packet::Ty::Chat(text.to_owned()),
		v1_8::play_to_server_packet::serialize,
	)?)
	.await?;
	Ok(())
}

#[tokio::test]
async fn old_clients_switch_between_modern_servers() -> Result<()> {
	let (a, mut a_events) = mock("a", 0, "minecraft:overworld", false).await;
	let (b, mut b_events) = mock("b", 3, "minecraft:the_nether", true).await;
	let proxy = bridge(a).await;
	let mut conn = join_1_8(proxy).await?;
	assert_eq!(
		events(&mut a_events, 4).await,
		[
			"a intent 2",
			"a login cookie test:k = none",
			"a config cookie test:config = a",
			"a joined"
		]
	);
	assert_eq!(dimensions_1_8(&mut conn, 1).await?, ["login 0"]);
	say_1_8(&mut conn, "store test:k old").await?;
	say_1_8(&mut conn, "/proxy-ping").await?;
	assert!(chat_1_8(&mut conn).await?.contains("Pong"));
	say_1_8(&mut conn, &format!("transfer 127.0.0.1 {}", b.port())).await?;
	assert_eq!(
		events(&mut b_events, 4).await,
		[
			"b intent 3",
			"b login cookie test:k = old",
			"b config cookie test:config = b",
			"b joined"
		]
	);
	assert_eq!(
		dimensions_1_8(&mut conn, 3).await?,
		["login -1", "respawn 0", "respawn -1"]
	);
	say_1_8(&mut conn, "ask test:config").await?;
	assert!(chat_1_8(&mut conn)
		.await?
		.contains("b: play cookie test:config = b"));
	say_1_8(&mut conn, &format!("/proxy-goto {a} 26.1")).await?;
	assert_eq!(
		events(&mut a_events, 4).await,
		[
			"a intent 2",
			"a login cookie test:k = old",
			"a config cookie test:config = a",
			"a joined"
		]
	);
	assert_eq!(
		dimensions_1_8(&mut conn, 3).await?,
		["login 0", "respawn -1", "respawn 0"]
	);
	say_1_8(&mut conn, "hi").await?;
	assert!(chat_1_8(&mut conn).await?.contains("a: hi"));
	Ok(())
}

#[tokio::test]
async fn transfers_during_configuration_are_followed() -> Result<()> {
	let (b, mut b_events) = mock("b", 3, "minecraft:the_nether", true).await;
	let (t, mut t_events) = start("t", 0, "minecraft:overworld", false, Some(b.port())).await;
	let proxy = bridge(t).await;
	let mut client = Client::join(proxy).await?;
	assert_eq!(
		events(&mut t_events, 3).await,
		[
			"t intent 2",
			"t login cookie test:k = none",
			"t config cookie test:config = t"
		]
	);
	assert_eq!(
		events(&mut b_events, 4).await,
		[
			"b intent 3",
			"b login cookie test:k = none",
			"b config cookie test:config = b",
			"b joined"
		]
	);
	assert_eq!(
		client.world().await?,
		(3, "minecraft:the_nether".to_owned())
	);
	client.say("hi").await?;
	assert!(client.chat().await?.contains("b: hi"));
	let mut old = join_1_8(proxy).await?;
	assert_eq!(
		events(&mut t_events, 3).await,
		[
			"t intent 2",
			"t login cookie test:k = none",
			"t config cookie test:config = t"
		]
	);
	assert_eq!(
		events(&mut b_events, 4).await,
		[
			"b intent 3",
			"b login cookie test:k = none",
			"b config cookie test:config = b",
			"b joined"
		]
	);
	assert_eq!(dimensions_1_8(&mut old, 1).await?, ["login -1"]);
	say_1_8(&mut old, "hi").await?;
	assert!(chat_1_8(&mut old).await?.contains("b: hi"));
	Ok(())
}

async fn server_info(proxy: SocketAddr, protocol: i32) -> Result<serde_json::Value> {
	let mut conn = Framed::new(TcpStream::connect(proxy).await?, Codec::default());
	let hs = handshake::Ty::SetProtocol(handshaking_to_server_packet_set_protocol::Ty {
		protocol_version: protocol,
		server_host: "localhost".to_owned(),
		server_port: proxy.port(),
		next_state: 1,
	});
	conn.send(encode(&hs, handshake::serialize)?).await?;
	conn.send(encode(
		&status_s::Ty::PingStart(status_to_server_packet_ping_start::Ty {}),
		status_s::serialize,
	)?)
	.await?;
	let status_c::Ty::ServerInfo(info) = decode(
		&tokio::time::timeout(WAIT, next(&mut conn)).await??,
		status_c::deserialize,
	)?
	else {
		bail!("server info");
	};
	Ok(serde_json::from_str(&info)?)
}

async fn legacy_ping(proxy: SocketAddr, request: &[u8]) -> Result<String> {
	let mut stream = TcpStream::connect(proxy).await?;
	stream.write_all(request).await?;
	let mut reply = Vec::new();
	tokio::time::timeout(WAIT, stream.read_to_end(&mut reply)).await??;
	assert_eq!(reply[0], 0xff);
	let len = i16::from_be_bytes([reply[1], reply[2]]) as usize;
	let units: Vec<u16> = reply[3..3 + len * 2]
		.as_chunks::<2>()
		.0
		.iter()
		.map(|c| u16::from_be_bytes(*c))
		.collect();
	Ok(String::from_utf16(&units)?)
}

#[tokio::test]
async fn the_server_list_shows_a_modern_backend_to_every_era() -> Result<()> {
	let (a, _events) = mock("a", 0, "minecraft:overworld", false).await;
	let proxy = bridge(a).await;
	let new = server_info(proxy, protocol(Version::V26_1)).await?;
	assert_eq!(new["version"]["protocol"], protocol(Version::V26_1));
	assert!(
		new["version"]["name"]
			.as_str()
			.unwrap()
			.contains("1.0.1-26.1"),
		"{new}"
	);
	assert_eq!(
		new["description"]["extra"][0],
		serde_json::json!({ "text": "Modern ", "color": "#ff5555", "shadow_color": -16777216 })
	);
	assert_eq!(new["description"]["extra"][1]["font"], "minecraft:uniform");
	assert_eq!(
		(
			new["players"]["online"].as_i64(),
			new["players"]["max"].as_i64()
		),
		(Some(3), Some(100))
	);
	assert_eq!(new["players"]["sample"][0]["name"], "Steve");
	assert_eq!(new["favicon"], "data:image/png;base64,iVBORw0KGgo=");
	let nether_update = server_info(proxy, protocol(Version::V1_16)).await?;
	assert_eq!(
		nether_update["description"]["extra"][0],
		serde_json::json!({ "text": "Modern ", "color": "#ff5555" })
	);
	for old in [
		Version::V1_7,
		Version::V1_8,
		Version::V1_12_2,
		Version::V1_15_2,
	] {
		let info = server_info(proxy, protocol(old)).await?;
		assert_eq!(info["version"]["protocol"], protocol(old), "{old}");
		assert_eq!(
			info["description"],
			serde_json::json!({ "text": "", "extra": [{ "text": "Modern ", "color": "red" }, { "text": "a", "bold": true }] }),
			"{old}"
		);
		assert_eq!(info["players"]["online"], 3, "{old}");
	}
	assert_eq!(
		legacy_ping(proxy, &[0xfe]).await?,
		"Modern a\u{a7}3\u{a7}100"
	);
	assert_eq!(
		legacy_ping(proxy, &[0xfe, 0x01]).await?,
		"\u{a7}1\x0078\x00pds2 bridge 1.0.1-1.6.4\x00\u{a7}cModern \u{a7}r\u{a7}la\x003\x00100"
	);
	let mut host = vec![0xfe, 0x01, 0xfa];
	string16(&mut host, "MC|PingHost");
	let mut data = vec![61];
	string16(&mut data, "localhost");
	data.extend_from_slice(&25565i32.to_be_bytes());
	host.extend_from_slice(&(data.len() as i16).to_be_bytes());
	host.extend_from_slice(&data);
	assert_eq!(
		legacy_ping(proxy, &host).await?,
		"\u{a7}1\x0061\x00pds2 bridge 1.0.1-1.6.4\x00\u{a7}cModern \u{a7}r\u{a7}la\x003\x00100"
	);
	Ok(())
}
