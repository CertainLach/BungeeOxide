use bungeeoxide_converter::Version;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use bungeeoxide::codec::Codec;
use bungeeoxide::{decode, encode, next, serve, Config, Conn};
use bungeeoxide_converter::gen::convert::v1_8::{
	handshaking_to_server_packet as handshake, handshaking_to_server_packet_set_protocol,
	login_to_client_packet as login_c, login_to_client_packet_success,
	login_to_server_packet as login_s, play_to_client_packet as play_c, play_to_client_packet_chat,
	play_to_client_packet_login, play_to_server_packet as play_s,
	status_to_client_packet as status_c, status_to_server_packet as status_s,
	status_to_server_packet_ping_start,
};
use futures::SinkExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;

const PROTOCOL_1_8: i32 = 47;

async fn answer_status(mut conn: Conn, name: &str) -> Result<()> {
	loop {
		match decode(&next(&mut conn).await?, status_s::deserialize)? {
			status_s::Ty::PingStart(_) => {
				let info = serde_json::json!({
					"version": { "name": "1.8.8", "protocol": PROTOCOL_1_8 },
					"players": { "max": 20, "online": 1, "sample": [{ "name": "Alex", "id": "ec561538-f3fd-461d-aff5-086b22154bce" }] },
					"description": { "text": format!("mock {name}"), "color": "gold" },
					"favicon": "data:image/png;base64,AAAA\nBBBB",
				});
				conn.send(encode(
					&status_c::Ty::ServerInfo(info.to_string()),
					status_c::serialize,
				)?)
				.await?;
			}
			status_s::Ty::Ping(time) => {
				conn.send(encode(&status_c::Ty::Ping(time), status_c::serialize)?)
					.await?;
				return Ok(());
			}
			status_s::Ty::Default(id) => bail!("status packet {id}"),
		}
	}
}

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

async fn backend(mut conn: Conn, name: &str, dimension: i8) -> Result<()> {
	let handshake::Ty::SetProtocol(hs) = decode(&next(&mut conn).await?, handshake::deserialize)?
	else {
		bail!("handshake");
	};
	if hs.next_state == 1 {
		return answer_status(conn, name).await;
	}
	assert_eq!(hs.protocol_version, PROTOCOL_1_8);
	let login_s::Ty::LoginStart(username) = decode(&next(&mut conn).await?, login_s::deserialize)?
	else {
		bail!("login start");
	};
	conn.send(encode(&login_c::Ty::Compress(64), login_c::serialize)?)
		.await?;
	conn.codec_mut().threshold = Some(64);
	let success = login_to_client_packet_success::Ty {
		uuid: bungeeoxide_authlib::offline_uuid(&username),
		username,
	};
	conn.send(encode(&login_c::Ty::Success(success), login_c::serialize)?)
		.await?;
	let join = play_c::Ty::Login(play_to_client_packet_login::Ty {
		entity_id: 7,
		game_mode: 1,
		dimension,
		difficulty: 2,
		max_players: 20,
		level_type: "flat".to_owned(),
		reduced_debug_info: false,
	});
	conn.send(encode(&join, play_c::serialize)?).await?;
	loop {
		if let play_s::Ty::Chat(message) = decode(&next(&mut conn).await?, play_s::deserialize)? {
			let echo = play_c::Ty::Chat(play_to_client_packet_chat::Ty {
				message: serde_json::json!({ "text": format!("{name}: {message}") }).to_string(),
				position: 0,
			});
			conn.send(encode(&echo, play_c::serialize)?).await?;
		}
	}
}

async fn start_proxy(target: SocketAddr) -> SocketAddr {
	start_proxy_with(target).await.0
}

async fn start_proxy_with(target: SocketAddr) -> (SocketAddr, Arc<Config>) {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let config = Arc::new(Config {
		target: target.to_string(),
		motd: "test bridge".to_owned(),
		threshold: 32,
		server_version: Version::V1_8,
		..Config::default()
	});
	tokio::spawn(serve(listener, config.clone()));
	(addr, config)
}

async fn connect(proxy: SocketAddr, next_state: i32) -> Result<Conn> {
	connect_as(proxy, next_state, PROTOCOL_1_8).await
}

async fn connect_as(proxy: SocketAddr, next_state: i32, protocol: i32) -> Result<Conn> {
	let mut conn = Framed::new(TcpStream::connect(proxy).await?, Codec::default());
	let hs = handshake::Ty::SetProtocol(handshaking_to_server_packet_set_protocol::Ty {
		protocol_version: protocol,
		server_host: "localhost".to_owned(),
		server_port: proxy.port(),
		next_state: next_state,
	});
	conn.send(encode(&hs, handshake::serialize)?).await?;
	Ok(conn)
}

async fn play(conn: &mut Conn) -> Result<play_c::Ty> {
	let data = tokio::time::timeout(Duration::from_secs(5), next(conn)).await??;
	decode(&data, play_c::deserialize)
}

async fn say(conn: &mut Conn, message: &str) -> Result<()> {
	conn.send(encode(
		&play_s::Ty::Chat(message.to_owned()),
		play_s::serialize,
	)?)
	.await?;
	Ok(())
}

async fn chat(conn: &mut Conn) -> Result<String> {
	match play(conn).await? {
		play_c::Ty::Chat(c) => Ok(c.message),
		other => bail!("expected chat, got {other:?}"),
	}
}

async fn server_info(proxy: SocketAddr, protocol: i32) -> Result<serde_json::Value> {
	let mut conn = connect_as(proxy, 1, protocol).await?;
	conn.send(encode(
		&status_s::Ty::PingStart(status_to_server_packet_ping_start::Ty {}),
		status_s::serialize,
	)?)
	.await?;
	let status_c::Ty::ServerInfo(info) = decode(&next(&mut conn).await?, status_c::deserialize)?
	else {
		bail!("expected server info");
	};
	conn.send(encode(&status_s::Ty::Ping(42), status_s::serialize)?)
		.await?;
	let pong = decode(&next(&mut conn).await?, status_c::deserialize)?;
	assert_eq!(pong, status_c::Ty::Ping(42));
	Ok(serde_json::from_str(&info)?)
}

#[tokio::test]
async fn status_relays_the_backend() -> Result<()> {
	let (proxy, config) = start_proxy_with(mock_server("a", 0).await).await;
	let info = server_info(proxy, PROTOCOL_1_8).await?;
	assert_eq!(
		info["description"],
		serde_json::json!({ "text": "mock a", "color": "gold" })
	);
	assert_eq!(info["version"]["protocol"], PROTOCOL_1_8);
	assert!(
		info["version"]["name"]
			.as_str()
			.unwrap()
			.starts_with("pds2 bridge 1.0.1-"),
		"{info}"
	);
	assert_eq!(
		(
			info["players"]["online"].as_i64(),
			info["players"]["max"].as_i64()
		),
		(Some(1), Some(20))
	);
	assert_eq!(info["players"]["sample"][0]["name"], "Alex");
	assert_eq!(info["favicon"], "data:image/png;base64,AAAABBBB");
	let server = Version::V1_8;
	let newest = bungeeoxide::protocol_of(bungeeoxide::reach(server).1).unwrap();
	let stranger = server_info(proxy, 9999).await?;
	assert_eq!(stranger["version"]["protocol"], newest);
	assert_eq!(stranger["description"]["text"], "mock a");
	assert_eq!(config.status.fetches(), 1);
	Ok(())
}

#[tokio::test]
async fn status_falls_back_to_the_motd() -> Result<()> {
	let proxy = start_proxy("127.0.0.1:1".parse()?).await;
	let info = server_info(proxy, PROTOCOL_1_8).await?;
	assert_eq!(info["description"]["text"], "test bridge");
	assert_eq!(info["players"]["online"], 0);
	assert_eq!(info["version"]["protocol"], PROTOCOL_1_8);
	Ok(())
}

#[tokio::test]
async fn players_relay_chat_and_move_between_servers() -> Result<()> {
	let a = mock_server("a", 0).await;
	let b = mock_server("b", -1).await;
	let proxy = start_proxy(a).await;
	let mut conn = connect(proxy, 2).await?;
	conn.send(encode(
		&login_s::Ty::LoginStart("tester".to_owned()),
		login_s::serialize,
	)?)
	.await?;

	let login_c::Ty::Compress(threshold) = decode(&next(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected compression");
	};
	assert_eq!(threshold, 32);
	conn.codec_mut().threshold = Some(32);
	let login_c::Ty::Success(success) = decode(&next(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected login success");
	};
	assert_eq!(success.username, "tester");
	assert_eq!(success.uuid, bungeeoxide_authlib::offline_uuid("tester"));
	let play_c::Ty::Login(join) = play(&mut conn).await? else {
		bail!("expected join game");
	};
	assert_eq!((join.entity_id, join.dimension), (7, 0));

	say(&mut conn, "hello").await?;
	assert!(chat(&mut conn).await?.contains("a: hello"));
	say(&mut conn, &"long ".repeat(20)).await?;
	assert!(chat(&mut conn).await?.contains("a: long long"));
	say(&mut conn, "/proxy-ping").await?;
	assert!(chat(&mut conn).await?.contains("Pong"));

	say(&mut conn, &format!("/proxy-goto {b}")).await?;
	let play_c::Ty::Login(join) = play(&mut conn).await? else {
		bail!("expected join game from the new server");
	};
	assert_eq!(join.dimension, -1);
	let mut dimensions = Vec::new();
	for _ in 0..2 {
		let play_c::Ty::Respawn(r) = play(&mut conn).await? else {
			bail!("expected respawn");
		};
		dimensions.push(r.dimension);
	}
	assert_eq!(dimensions, [0, -1]);
	say(&mut conn, "hello").await?;
	assert!(chat(&mut conn).await?.contains("b: hello"));

	say(&mut conn, "/proxy-goto 127.0.0.1:1").await?;
	assert!(chat(&mut conn).await?.contains("can't move"));
	say(&mut conn, "still here").await?;
	assert!(chat(&mut conn).await?.contains("b: still here"));
	Ok(())
}

#[tokio::test]
async fn other_versions_are_turned_away() -> Result<()> {
	let proxy = start_proxy(mock_server("a", 0).await).await;
	let mut conn = Framed::new(TcpStream::connect(proxy).await?, Codec::default());
	let hs = handshake::Ty::SetProtocol(handshaking_to_server_packet_set_protocol::Ty {
		protocol_version: 3,
		server_host: "localhost".to_owned(),
		server_port: proxy.port(),
		next_state: 2,
	});
	conn.send(encode(&hs, handshake::serialize)?).await?;
	conn.send(encode(
		&login_s::Ty::LoginStart("old".to_owned()),
		login_s::serialize,
	)?)
	.await?;
	let login_c::Ty::Disconnect(reason) = decode(&next(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected a disconnect");
	};
	assert!(reason.contains("not protocol 3"), "{reason}");
	Ok(())
}
