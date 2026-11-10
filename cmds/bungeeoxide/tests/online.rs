use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use bungeeoxide::accounts::Accounts;
use bungeeoxide::auth::{hyphenated, server_hash};
use bungeeoxide::codec::{Cipher, Codec};
use bungeeoxide::{decode, encode, next, serve, Config, Conn};
use futures::SinkExt;
use minecraft_versions::gen::convert::v1_8::{
	handshaking_toServer_packet as handshake, handshaking_toServer_packet_set_protocol,
	login_toClient_packet as login_c, login_toClient_packet_encryption_begin,
	login_toClient_packet_success, login_toServer_packet as login_s,
	play_toClient_packet as play_c, play_toClient_packet_chat, play_toClient_packet_login,
	play_toServer_packet as play_s,
};
use rsa::pkcs8::EncodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::codec::Framed;

const PROFILE: u128 = 0x069a79f444e94726a5befca90e38aaf5;

const PROTOCOL_1_8: i32 = 47;

async fn session_server() -> (SocketAddr, mpsc::UnboundedReceiver<serde_json::Value>) {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let (joins, received) = mpsc::unbounded_channel();
	tokio::spawn(async move {
		loop {
			let (mut stream, _) = listener.accept().await.unwrap();
			let joins = joins.clone();
			tokio::spawn(async move {
				let mut request = Vec::new();
				let mut buf = [0u8; 4096];
				let body = loop {
					let n = stream.read(&mut buf).await.unwrap();
					request.extend_from_slice(&buf[..n]);
					let text = String::from_utf8_lossy(&request).into_owned();
					let Some((head, body)) = text.split_once("\r\n\r\n") else {
						continue;
					};
					let length = head
						.lines()
						.find_map(|l| {
							l.to_ascii_lowercase()
								.strip_prefix("content-length:")
								.map(|v| v.trim().parse::<usize>().unwrap())
						})
						.unwrap_or(0);
					if body.len() >= length || n == 0 {
						assert!(head.starts_with("POST /session/minecraft/join "), "{head}");
						break body.to_owned();
					}
				};
				joins.send(serde_json::from_str(&body).unwrap()).unwrap();
				stream
					.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
					.await
					.unwrap();
			});
		}
	});
	(addr, received)
}

async fn online_backend() -> (SocketAddr, mpsc::UnboundedReceiver<String>) {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let key = RsaPrivateKey::new(&mut rand::thread_rng(), 1024).unwrap();
	let (hashes, received) = mpsc::unbounded_channel();
	tokio::spawn(async move {
		loop {
			let (stream, _) = listener.accept().await.unwrap();
			let key = key.clone();
			let hashes = hashes.clone();
			tokio::spawn(async move {
				let _ = backend(Framed::new(stream, Codec::default()), key, hashes).await;
			});
		}
	});
	(addr, received)
}

async fn backend(
	mut conn: Conn,
	key: RsaPrivateKey,
	hashes: mpsc::UnboundedSender<String>,
) -> Result<()> {
	let handshake::Ty::set_protocol(_) = decode(&next(&mut conn).await?, handshake::deserialize)?
	else {
		bail!("handshake");
	};
	let login_s::Ty::login_start(username) = decode(&next(&mut conn).await?, login_s::deserialize)?
	else {
		bail!("login start");
	};
	let public = key.to_public_key().to_public_key_der()?.into_vec();
	let request = login_toClient_packet_encryption_begin::Ty {
		serverId: String::new(),
		publicKey: public.clone(),
		verifyToken: vec![1, 2, 3, 4],
	};
	conn.send(encode(
		&login_c::Ty::encryption_begin(request),
		login_c::serialize,
	)?)
	.await?;
	let login_s::Ty::encryption_begin(response) =
		decode(&next(&mut conn).await?, login_s::deserialize)?
	else {
		bail!("encryption response");
	};
	assert_eq!(
		key.decrypt(Pkcs1v15Encrypt, &response.verifyToken)?,
		[1, 2, 3, 4]
	);
	let secret: [u8; 16] = key
		.decrypt(Pkcs1v15Encrypt, &response.sharedSecret)?
		.try_into()
		.unwrap();
	hashes.send(server_hash("", &secret, &public))?;
	conn.codec_mut().cipher = Some(Cipher::new(&secret));
	conn.send(encode(&login_c::Ty::compress(64), login_c::serialize)?)
		.await?;
	conn.codec_mut().threshold = Some(64);
	let success = login_toClient_packet_success::Ty {
		uuid: hyphenated(PROFILE),
		username,
	};
	conn.send(encode(&login_c::Ty::success(success), login_c::serialize)?)
		.await?;
	let join = play_c::Ty::login(play_toClient_packet_login::Ty {
		entityId: 7,
		gameMode: 1,
		dimension: 0,
		difficulty: 2,
		maxPlayers: 20,
		levelType: "flat".to_owned(),
		reducedDebugInfo: false,
	});
	conn.send(encode(&join, play_c::serialize)?).await?;
	loop {
		if let play_s::Ty::chat(message) = decode(&next(&mut conn).await?, play_s::deserialize)? {
			let echo = play_c::Ty::chat(play_toClient_packet_chat::Ty {
				message: serde_json::json!({ "text": format!("echo: {message}") }).to_string(),
				position: 0,
			});
			conn.send(encode(&echo, play_c::serialize)?).await?;
		}
	}
}

async fn start_proxy(target: SocketAddr, session: SocketAddr, version: &str) -> SocketAddr {
	let accounts = format!(
		"[endpoints]\nsession = \"http://{session}\"\nservices = \"http://127.0.0.1:1\"\n\n[[account]]\nplayers = [\"Steve\"]\nsource = \"token\"\naccess_token = \"fake-token\"\nuuid = \"{}\"\nname = \"Notch\"\n",
		hyphenated(PROFILE)
	);
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let config = Arc::new(Config {
		target: target.to_string(),
		motd: "online bridge".to_owned(),
		threshold: 32,
		server_version: version.to_owned(),
		accounts: Accounts::parse(&accounts, Path::new("/nonexistent/accounts.toml")).unwrap(),
		..Config::default()
	});
	tokio::spawn(serve(listener, config));
	addr
}

async fn login(proxy: SocketAddr, name: &str) -> Result<Conn> {
	let mut conn = Framed::new(TcpStream::connect(proxy).await?, Codec::default());
	let hs = handshake::Ty::set_protocol(handshaking_toServer_packet_set_protocol::Ty {
		protocolVersion: PROTOCOL_1_8,
		serverHost: "localhost".to_owned(),
		serverPort: proxy.port(),
		nextState: 2,
	});
	conn.send(encode(&hs, handshake::serialize)?).await?;
	conn.send(encode(
		&login_s::Ty::login_start(name.to_owned()),
		login_s::serialize,
	)?)
	.await?;
	Ok(conn)
}

async fn receive(conn: &mut Conn) -> Result<Vec<u8>> {
	tokio::time::timeout(Duration::from_secs(10), next(conn)).await?
}

#[tokio::test]
async fn mapped_players_log_in_upstream_with_their_account() -> Result<()> {
	let (session, mut joins) = session_server().await;
	let (backend, mut hashes) = online_backend().await;
	let proxy = start_proxy(backend, session, "1.8").await;
	let mut conn = login(proxy, "Steve").await?;
	let login_c::Ty::compress(threshold) =
		decode(&receive(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected compression");
	};
	conn.codec_mut().threshold = Some(usize::try_from(threshold)?);
	let login_c::Ty::success(success) = decode(&receive(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected login success");
	};
	assert_eq!(success.username, "Notch");
	assert_eq!(success.uuid, hyphenated(PROFILE));
	let join = joins.recv().await.unwrap();
	assert_eq!(join["accessToken"], "fake-token");
	assert_eq!(join["selectedProfile"], format!("{PROFILE:032x}"));
	assert_eq!(join["serverId"], hashes.recv().await.unwrap());
	let play_c::Ty::login(_) = decode(&receive(&mut conn).await?, play_c::deserialize)? else {
		bail!("expected join game");
	};
	conn.send(encode(
		&play_s::Ty::chat("through the cipher".to_owned()),
		play_s::serialize,
	)?)
	.await?;
	let play_c::Ty::chat(chat) = decode(&receive(&mut conn).await?, play_c::deserialize)? else {
		bail!("expected chat");
	};
	assert!(
		chat.message.contains("echo: through the cipher"),
		"{}",
		chat.message
	);
	Ok(())
}

#[tokio::test]
async fn unmapped_players_are_told_how_to_join_online_servers() -> Result<()> {
	let (session, _joins) = session_server().await;
	let (backend, _hashes) = online_backend().await;
	let proxy = start_proxy(backend, session, "1.8").await;
	let mut conn = login(proxy, "Alex").await?;
	let login_c::Ty::disconnect(reason) = decode(&receive(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected a disconnect");
	};
	assert!(reason.contains("--accounts"), "{reason}");
	Ok(())
}

#[tokio::test]
async fn pre_netty_servers_are_refused_for_mapped_players() -> Result<()> {
	let (session, _joins) = session_server().await;
	let (backend, _hashes) = online_backend().await;
	let proxy = start_proxy(backend, session, "1.6.4").await;
	let mut conn = login(proxy, "Steve").await?;
	let login_c::Ty::disconnect(reason) = decode(&receive(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected a disconnect");
	};
	assert!(reason.contains("legacy session service"), "{reason}");
	Ok(())
}
