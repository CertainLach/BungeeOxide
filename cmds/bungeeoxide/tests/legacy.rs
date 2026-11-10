use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use bungeeoxide::codec::Codec;
use bungeeoxide::legacy::{string16, Cfb8, Legacy};
use bungeeoxide::{decode, encode, next, serve, Config, Conn};
use futures::SinkExt;
use minecraft_versions::gen::convert::packet_name;
use minecraft_versions::gen::convert::v1_8::{
	handshaking_toServer_packet as handshake, login_toClient_packet as login_c,
	login_toClient_packet_success, login_toServer_packet as login_s,
	play_toClient_packet as play_c, play_toClient_packet_chat, play_toClient_packet_login,
	play_toServer_packet as play_s, status_toClient_packet as status_c,
	status_toServer_packet as status_s,
};
use minecraft_versions::{index_of, Root};
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;

async fn mock_server(name: &'static str, dimension: i8) -> SocketAddr {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	tokio::spawn(async move {
		loop {
			let (stream, _) = listener.accept().await.unwrap();
			tokio::spawn(async move {
				let _ = backend(Framed::new(stream, Codec::default()), name, dimension).await;
			});
		}
	});
	addr
}

async fn answer_status(mut conn: Conn, name: &str) -> Result<()> {
	let status_s::Ty::ping_start(_) = decode(&next(&mut conn).await?, status_s::deserialize)?
	else {
		bail!("status request");
	};
	let info = serde_json::json!({
		"version": { "name": "1.8.8", "protocol": 47 },
		"players": { "max": 30, "online": 2 },
		"description": { "text": "", "extra": [{ "text": "Mock ", "color": "#ff0000" }, { "text": name, "bold": true }] },
	});
	conn.send(encode(
		&status_c::Ty::server_info(info.to_string()),
		status_c::serialize,
	)?)
	.await?;
	Ok(())
}

async fn backend(mut conn: Conn, name: &str, dimension: i8) -> Result<()> {
	let handshake::Ty::set_protocol(hs) = decode(&next(&mut conn).await?, handshake::deserialize)?
	else {
		bail!("handshake");
	};
	if hs.nextState == 1 {
		return answer_status(conn, name).await;
	}
	let login_s::Ty::login_start(username) = decode(&next(&mut conn).await?, login_s::deserialize)?
	else {
		bail!("login start");
	};
	let success = login_toClient_packet_success::Ty {
		uuid: bungeeoxide::offline_uuid(&username),
		username,
	};
	conn.send(encode(&login_c::Ty::success(success), login_c::serialize)?)
		.await?;
	let join = play_c::Ty::login(play_toClient_packet_login::Ty {
		entityId: 7,
		gameMode: 1,
		dimension,
		difficulty: 2,
		maxPlayers: 20,
		levelType: "flat".to_owned(),
		reducedDebugInfo: false,
	});
	conn.send(encode(&join, play_c::serialize)?).await?;
	conn.send(encode(&play_c::Ty::keep_alive(1234), play_c::serialize)?)
		.await?;
	loop {
		match decode(&next(&mut conn).await?, play_s::deserialize)? {
			play_s::Ty::chat(message) if message == "bye" => return Ok(()),
			play_s::Ty::chat(message) => {
				let echo = play_c::Ty::chat(play_toClient_packet_chat::Ty {
					message: serde_json::json!({ "text": format!("{name}: {message}") })
						.to_string(),
					position: 0,
				});
				conn.send(encode(&echo, play_c::serialize)?).await?;
			}
			play_s::Ty::keep_alive(id) => {
				let echo = play_c::Ty::chat(play_toClient_packet_chat::Ty {
					message: serde_json::json!({ "text": format!("{name}: keep alive {id}") })
						.to_string(),
					position: 0,
				});
				conn.send(encode(&echo, play_c::serialize)?).await?;
			}
			_ => {}
		}
	}
}

async fn start_proxy(target: SocketAddr) -> SocketAddr {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let config = Arc::new(Config {
		target: target.to_string(),
		motd: "test bridge".to_owned(),
		threshold: 32,
		server_version: "1.8".to_owned(),
		..Config::default()
	});
	tokio::spawn(serve(listener, config));
	addr
}

struct Client {
	conn: Conn,
	version: usize,
}

fn read_string16(data: &[u8], at: usize) -> String {
	let len = i16::from_be_bytes([data[at], data[at + 1]]) as usize;
	let units: Vec<u16> = data[at + 2..at + 2 + len * 2]
		.chunks_exact(2)
		.map(|c| u16::from_be_bytes([c[0], c[1]]))
		.collect();
	String::from_utf16(&units).unwrap()
}

fn short_bytes(data: &[u8], at: usize) -> (Vec<u8>, usize) {
	let len = i16::from_be_bytes([data[at], data[at + 1]]) as usize;
	(data[at + 2..at + 2 + len].to_vec(), at + 2 + len)
}

impl Client {
	async fn join(proxy: SocketAddr, version: &str, protocol: i32, name: &str) -> Result<Client> {
		let v = index_of(version).unwrap();
		let stream = TcpStream::connect(proxy).await?;
		let mut conn = Framed::new(
			stream,
			Codec {
				threshold: None,
				legacy: Some(Legacy::new(Root::PlayToClient, Some(v))),
				cipher: None,
			},
		);
		if protocol < 39 {
			let mut hello = vec![0x02];
			string16(
				&mut hello,
				&if protocol >= 28 {
					format!("{name};localhost:{}", proxy.port())
				} else {
					name.to_owned()
				},
			);
			conn.send(hello).await?;
			let hash = next(&mut conn).await?;
			assert_eq!((hash[0], read_string16(&hash, 1).as_str()), (0x02, "-"));
			let mut login = vec![0x01];
			login.extend_from_slice(&protocol.to_be_bytes());
			string16(&mut login, name);
			if protocol <= 23 {
				login.extend_from_slice(&0i64.to_be_bytes());
			}
			if protocol != 22 {
				string16(&mut login, "");
			}
			login.extend_from_slice(&0i32.to_be_bytes());
			if protocol >= 28 {
				login.extend_from_slice(&0i32.to_be_bytes());
			} else {
				login.push(0);
			}
			login.extend_from_slice(&[0, 0, 0]);
			conn.send(login).await?;
		} else {
			let mut hello = vec![0x02, protocol as u8];
			string16(&mut hello, name);
			string16(&mut hello, "localhost");
			hello.extend_from_slice(&i32::from(proxy.port()).to_be_bytes());
			conn.send(hello).await?;
			let request = next(&mut conn).await?;
			assert_eq!(request[0], 0xfd);
			assert_eq!(read_string16(&request, 1), "-");
			let (key, at) = short_bytes(&request, 5);
			let (token, _) = short_bytes(&request, at);
			let key = RsaPublicKey::from_public_key_der(&key)?;
			let secret = [42u8; 16];
			let mut rng = rand::thread_rng();
			let mut response = vec![0xfc];
			for part in [
				key.encrypt(&mut rng, Pkcs1v15Encrypt, &secret)?,
				key.encrypt(&mut rng, Pkcs1v15Encrypt, &token)?,
			] {
				response.extend_from_slice(&(part.len() as i16).to_be_bytes());
				response.extend_from_slice(&part);
			}
			conn.send(response).await?;
			assert_eq!(next(&mut conn).await?, [0xfc, 0, 0, 0, 0]);
			conn.codec_mut().legacy.as_mut().unwrap().cipher =
				Some((Cfb8::new(&secret), Cfb8::new(&secret)));
			conn.send(vec![0xcd, 0]).await?;
		}
		Ok(Client { conn, version: v })
	}

	async fn packet(&mut self) -> Result<(&'static str, Vec<u8>)> {
		let data = tokio::time::timeout(Duration::from_secs(5), next(&mut self.conn)).await??;
		let name = packet_name(Root::PlayToClient, self.version, &data).unwrap_or("unknown");
		Ok((name, data))
	}

	async fn expect(&mut self, wanted: &str) -> Result<Vec<u8>> {
		loop {
			let (name, data) = self.packet().await?;
			if name == wanted {
				return Ok(data);
			}
		}
	}

	async fn say(&mut self, message: &str) -> Result<()> {
		let mut chat = vec![0x03];
		string16(&mut chat, message);
		self.conn.send(chat).await?;
		Ok(())
	}

	async fn chat(&mut self) -> Result<String> {
		let data = self.expect("chat").await?;
		Ok(read_string16(&data, 1))
	}

	async fn live(&mut self, wanted: &str) -> Result<Vec<u8>> {
		loop {
			let (name, data) = self.packet().await?;
			match name {
				n if n == wanted => return Ok(data),
				"keep_alive" => self.conn.send(data).await?,
				"kick_disconnect" => bail!("kicked: {}", read_string16(&data, 1)),
				_ => {}
			}
		}
	}

	async fn live_chat(&mut self, text: &str) -> Result<()> {
		loop {
			let data = self.live("chat").await?;
			if read_string16(&data, 1).contains(text) {
				return Ok(());
			}
		}
	}
}

#[tokio::test]
#[ignore = "needs LIVE_BRIDGE=host:port and LIVE_TARGETS=host:port,... of running servers"]
async fn live_bridge_moves_pre_netty_clients() -> Result<()> {
	let (Ok(bridge), Ok(targets)) = (std::env::var("LIVE_BRIDGE"), std::env::var("LIVE_TARGETS"))
	else {
		bail!("set LIVE_BRIDGE and LIVE_TARGETS");
	};
	let bridge: SocketAddr = bridge.parse()?;
	for (version, protocol) in [("1.0.1", 22), ("1.2.5", 29), ("1.4.7", 51), ("1.6.4", 78)] {
		let name = format!("pre_{}", version.replace('.', "_"));
		let mut client = Client::join(bridge, version, protocol, &name).await?;
		client.live("login").await?;
		client.say("/proxy-ping").await?;
		client.live_chat("Pong").await?;
		for target in targets.split(',') {
			client.say(&format!("/proxy-goto {target}")).await?;
			let mut dimensions = Vec::new();
			for wanted in ["login", "respawn", "respawn"] {
				let data = client.live(wanted).await?;
				if wanted == "respawn" {
					dimensions.push(match version {
						"1.0.1" => i32::from(data[1] as i8),
						_ => i32::from_be_bytes(data[1..5].try_into()?),
					});
				}
			}
			client.say(&format!("hello on {target}")).await?;
			client.live_chat(&format!("hello on {target}")).await?;
			println!("{version} moved to {target}, respawns {dimensions:?}");
		}
	}
	Ok(())
}

async fn session(version: &str, protocol: i32) -> Result<()> {
	let a = mock_server("a", 0).await;
	let b = mock_server("b", -1).await;
	let proxy = start_proxy(a).await;
	let mut client = Client::join(proxy, version, protocol, "tester").await?;
	let login = client.expect("login").await?;
	assert_eq!(i32::from_be_bytes(login[1..5].try_into()?), 7, "{version}");

	let keep_alive = client.expect("keep_alive").await?;
	assert_eq!(i32::from_be_bytes(keep_alive[1..5].try_into()?), 1234);
	client.conn.send(keep_alive).await?;
	assert!(
		client.chat().await?.contains("a: keep alive 1234"),
		"{version}"
	);

	client.say("hello").await?;
	assert!(client.chat().await?.contains("a: hello"), "{version}");
	client.say("/proxy-ping").await?;
	assert!(client.chat().await?.contains("Pong"), "{version}");

	client.say(&format!("/proxy-goto {b}")).await?;
	let login = client.expect("login").await?;
	assert_eq!(i32::from_be_bytes(login[1..5].try_into()?), 7);
	client.expect("respawn").await?;
	client.expect("respawn").await?;
	client.say("hello").await?;
	assert!(client.chat().await?.contains("b: hello"), "{version}");

	client.say("/proxy-goto 127.0.0.1:1").await?;
	assert!(client.chat().await?.contains("can't move"), "{version}");

	client.say("bye").await?;
	let kick = client.expect("kick_disconnect").await?;
	assert!(read_string16(&kick, 1).contains("closed"), "{version}");
	Ok(())
}

#[tokio::test]
async fn release_1_0_joins_through_the_bridge() -> Result<()> {
	session("1.0.1", 22).await
}

#[tokio::test]
async fn release_1_2_5_joins_through_the_bridge() -> Result<()> {
	session("1.2.5", 29).await
}

#[tokio::test]
async fn release_1_4_7_joins_with_encryption() -> Result<()> {
	session("1.4.7", 51).await
}

#[tokio::test]
async fn release_1_6_4_joins_with_encryption() -> Result<()> {
	session("1.6.4", 78).await
}

async fn ping(proxy: SocketAddr, request: &[u8]) -> Result<String> {
	let mut stream = TcpStream::connect(proxy).await?;
	stream.write_all(request).await?;
	let mut reply = Vec::new();
	tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut reply)).await??;
	assert_eq!(reply[0], 0xff);
	Ok(read_string16(&reply, 1))
}

fn ping_host(protocol: u8) -> Vec<u8> {
	let mut host = vec![0xfe, 0x01, 0xfa];
	string16(&mut host, "MC|PingHost");
	let mut data = vec![protocol];
	string16(&mut data, "localhost");
	data.extend_from_slice(&25565i32.to_be_bytes());
	host.extend_from_slice(&(data.len() as i16).to_be_bytes());
	host.extend_from_slice(&data);
	host
}

#[tokio::test]
async fn server_list_pings_show_the_backend_in_each_era() -> Result<()> {
	let proxy = start_proxy(mock_server("a", 0).await).await;
	assert_eq!(ping(proxy, &[0xfe]).await?, "Mock a\u{a7}2\u{a7}30");
	assert_eq!(
		ping(proxy, &[0xfe, 0x01]).await?,
		"\u{a7}1\x0078\x00pds2 bridge 1.0.1-1.6.4\x00\u{a7}4Mock \u{a7}r\u{a7}la\x002\x0030"
	);
	assert_eq!(
		ping(proxy, &ping_host(74)).await?,
		"\u{a7}1\x0074\x00pds2 bridge 1.0.1-1.6.4\x00\u{a7}4Mock \u{a7}r\u{a7}la\x002\x0030"
	);
	Ok(())
}

async fn pre_netty_server() -> SocketAddr {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	tokio::spawn(async move {
		loop {
			let (mut stream, _) = listener.accept().await.unwrap();
			tokio::spawn(async move {
				let mut first = [0u8; 1];
				if stream.read_exact(&mut first).await.is_ok() && first[0] == 0xfe {
					let mut rest = [0u8; 512];
					let _ =
						tokio::time::timeout(Duration::from_millis(200), stream.read(&mut rest))
							.await;
					let mut reply = vec![0xff];
					string16(&mut reply, "\u{a7}1\x0078\x001.6.4\x00old\x001\x0020");
					let _ = stream.write_all(&reply).await;
				}
			});
		}
	});
	addr
}

#[tokio::test]
async fn probes_tell_netty_and_pre_netty_servers_apart() -> Result<()> {
	let modern = index_of("26.1").unwrap();
	let netty = mock_server("a", 0).await.to_string();
	assert_eq!(
		bungeeoxide::status::probe(&netty, modern).await?,
		index_of("1.8").unwrap()
	);
	let old = pre_netty_server().await.to_string();
	assert_eq!(
		bungeeoxide::status::probe(&old, modern).await?,
		index_of("1.6.4").unwrap()
	);
	assert!(bungeeoxide::status::probe("127.0.0.1:1", modern)
		.await
		.is_err());
	Ok(())
}

#[tokio::test]
async fn server_list_pings_fall_back_to_the_motd() -> Result<()> {
	let proxy = start_proxy("127.0.0.1:1".parse()?).await;
	assert_eq!(ping(proxy, &[0xfe]).await?, "test bridge\u{a7}0\u{a7}0");
	assert_eq!(
		ping(proxy, &[0xfe, 0x01]).await?,
		"\u{a7}1\x0078\x00pds2 bridge 1.0.1-1.6.4\x00test bridge\x000\x000"
	);
	assert!(ping(proxy, &ping_host(74))
		.await?
		.starts_with("\u{a7}1\x0074\x00"));
	Ok(())
}

#[tokio::test]
async fn unknown_pre_netty_protocols_are_kicked() -> Result<()> {
	let proxy = start_proxy(mock_server("a", 0).await).await;
	let mut stream = TcpStream::connect(proxy).await?;
	let mut hello = vec![0x02, 62];
	string16(&mut hello, "tester");
	string16(&mut hello, "localhost");
	hello.extend_from_slice(&25565i32.to_be_bytes());
	stream.write_all(&hello).await?;
	let mut reply = Vec::new();
	tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut reply)).await??;
	assert_eq!(reply[0], 0xff);
	assert!(read_string16(&reply, 1).contains("1.0.1-1.6.4"));
	Ok(())
}
