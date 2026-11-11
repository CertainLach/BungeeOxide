use bungeeoxide_converter::Version;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Result};
use bungeeoxide::codec::{Cipher, Codec};
use bungeeoxide::{decode, encode, next, serve, users, Config, Conn};
use bungeeoxide_authlib::{hyphenated, server_hash};
use bungeeoxide_converter::gen::convert::v1_8::{
	handshaking_to_server_packet as handshake, handshaking_to_server_packet_set_protocol,
	login_to_client_packet as login_c, login_to_client_packet_encryption_begin,
	login_to_client_packet_success, login_to_server_packet as login_s,
	login_to_server_packet_encryption_begin,
	play_to_client_packet as play_c, play_to_client_packet_chat, play_to_client_packet_login,
	play_to_server_packet as play_s,
};
use futures::SinkExt;
use rsa::pkcs8::{DecodePublicKey, EncodePublicKey};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::codec::Framed;

const PROFILE: u128 = 0x069a79f444e94726a5befca90e38aaf5;

const PROTOCOL_1_8: i32 = 47;

type Verified = Arc<Mutex<BTreeMap<String, (String, u128)>>>;

struct Session {
	addr: SocketAddr,
	joins: mpsc::UnboundedReceiver<serde_json::Value>,
	verified: Verified,
}

async fn session_server() -> Session {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let (joins, received) = mpsc::unbounded_channel();
	let verified = Verified::default();
	let known = verified.clone();
	tokio::spawn(async move {
		loop {
			let (mut stream, _) = listener.accept().await.unwrap();
			let joins = joins.clone();
			let known = known.clone();
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
					if let Some(query) = head
						.lines()
						.next()
						.and_then(|l| l.strip_prefix("GET /session/minecraft/hasJoined?"))
					{
						let query = query.split(' ').next().unwrap_or_default();
						let param = |key: &str| {
							query
								.split('&')
								.find_map(|p| p.strip_prefix(&format!("{key}=")))
								.unwrap_or_default()
								.to_owned()
						};
						let found = known.lock().unwrap().get(&param("serverId")).cloned();
						let reply = match found.filter(|(name, _)| *name == param("username")) {
							Some((name, id)) => {
								let body = serde_json::json!({ "id": format!("{id:032x}"), "name": name }).to_string();
								format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
							}
							None => "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
						};
						stream.write_all(reply.as_bytes()).await.unwrap();
						return;
					}
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
	Session {
		addr,
		joins: received,
		verified,
	}
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
	let handshake::Ty::SetProtocol(_) = decode(&next(&mut conn).await?, handshake::deserialize)?
	else {
		bail!("handshake");
	};
	let login_s::Ty::LoginStart(username) = decode(&next(&mut conn).await?, login_s::deserialize)?
	else {
		bail!("login start");
	};
	let public = key.to_public_key().to_public_key_der()?.into_vec();
	let request = login_to_client_packet_encryption_begin::Ty {
		server_id: String::new(),
		public_key: public.clone(),
		verify_token: vec![1, 2, 3, 4],
	};
	conn.send(encode(
		&login_c::Ty::EncryptionBegin(request),
		login_c::serialize,
	)?)
	.await?;
	let login_s::Ty::EncryptionBegin(response) =
		decode(&next(&mut conn).await?, login_s::deserialize)?
	else {
		bail!("encryption response");
	};
	assert_eq!(
		key.decrypt(Pkcs1v15Encrypt, &response.verify_token)?,
		[1, 2, 3, 4]
	);
	let secret: [u8; 16] = key
		.decrypt(Pkcs1v15Encrypt, &response.shared_secret)?
		.try_into()
		.unwrap();
	hashes.send(server_hash("", &secret, &public))?;
	conn.codec_mut().cipher = Some(Cipher::new(&secret));
	conn.send(encode(&login_c::Ty::Compress(64), login_c::serialize)?)
		.await?;
	conn.codec_mut().threshold = Some(64);
	let success = login_to_client_packet_success::Ty {
		uuid: hyphenated(PROFILE),
		username,
	};
	conn.send(encode(&login_c::Ty::Success(success), login_c::serialize)?)
		.await?;
	let join = play_c::Ty::Login(play_to_client_packet_login::Ty {
		entity_id: 7,
		game_mode: 1,
		dimension: 0,
		difficulty: 2,
		max_players: 20,
		level_type: "flat".to_owned(),
		reduced_debug_info: false,
	});
	conn.send(encode(&join, play_c::serialize)?).await?;
	loop {
		if let play_s::Ty::Chat(message) = decode(&next(&mut conn).await?, play_s::deserialize)? {
			let echo = play_c::Ty::Chat(play_to_client_packet_chat::Ty {
				message: serde_json::json!({ "text": format!("echo: {message}") }).to_string(),
				position: 0,
			});
			conn.send(encode(&echo, play_c::serialize)?).await?;
		}
	}
}

async fn start_proxy(target: SocketAddr, session: SocketAddr, version: Version) -> SocketAddr {
	let upstream = format!(
		"{{ source = \"token\", access_token = \"fake-token\", uuid = \"{}\", name = \"Notch\" }}",
		hyphenated(PROFILE)
	);
	let config = format!(
		"[endpoints]\nsession = \"http://{session}\"\nservices = \"http://127.0.0.1:1\"\n\n[[user]]\nusername = \"Steve\"\ndownstream = false\nallow_goto = false\nupstream = {upstream}\n\n[[user]]\nusername = \"Notch\"\nuuid = \"{}\"\ndownstream = true\nallow_goto = false\nupstream = {upstream}\n",
		hyphenated(PROFILE)
	);
	let users = users::parse(&config, Path::new("/nonexistent/users.toml")).unwrap();
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let config = Arc::new(Config {
		target: target.to_string(),
		motd: "online bridge".to_owned(),
		threshold: 32,
		server_version: version.to_owned(),
		endpoints: users.endpoints,
		users: users.users,
		..Config::default()
	});
	tokio::spawn(serve(listener, config));
	addr
}

async fn login(proxy: SocketAddr, name: &str) -> Result<Conn> {
	let mut conn = Framed::new(TcpStream::connect(proxy).await?, Codec::default());
	let hs = handshake::Ty::SetProtocol(handshaking_to_server_packet_set_protocol::Ty {
		protocol_version: PROTOCOL_1_8,
		server_host: "localhost".to_owned(),
		server_port: proxy.port(),
		next_state: 2,
	});
	conn.send(encode(&hs, handshake::serialize)?).await?;
	conn.send(encode(
		&login_s::Ty::LoginStart(name.to_owned()),
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
	let Session {
		addr: session,
		mut joins,
		..
	} = session_server().await;
	let (backend, mut hashes) = online_backend().await;
	let proxy = start_proxy(backend, session, Version::V1_8).await;
	let mut conn = login(proxy, "Steve").await?;
	let login_c::Ty::Compress(threshold) =
		decode(&receive(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected compression");
	};
	conn.codec_mut().threshold = Some(usize::try_from(threshold)?);
	let login_c::Ty::Success(success) = decode(&receive(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected login success");
	};
	assert_eq!(success.username, "Notch");
	assert_eq!(success.uuid, hyphenated(PROFILE));
	let join = joins.recv().await.unwrap();
	assert_eq!(join["accessToken"], "fake-token");
	assert_eq!(join["selectedProfile"], format!("{PROFILE:032x}"));
	assert_eq!(join["serverId"], hashes.recv().await.unwrap());
	let play_c::Ty::Login(_) = decode(&receive(&mut conn).await?, play_c::deserialize)? else {
		bail!("expected join game");
	};
	conn.send(encode(
		&play_s::Ty::Chat("through the cipher".to_owned()),
		play_s::serialize,
	)?)
	.await?;
	let play_c::Ty::Chat(chat) = decode(&receive(&mut conn).await?, play_c::deserialize)? else {
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
	let Session { addr: session, .. } = session_server().await;
	let (backend, _hashes) = online_backend().await;
	let proxy = start_proxy(backend, session, Version::V1_8).await;
	let mut conn = login(proxy, "Alex").await?;
	let login_c::Ty::Disconnect(reason) = decode(&receive(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected a disconnect");
	};
	assert!(reason.contains("--users"), "{reason}");
	Ok(())
}

#[tokio::test]
async fn pre_netty_servers_are_refused_for_mapped_players() -> Result<()> {
	let Session { addr: session, .. } = session_server().await;
	let (backend, _hashes) = online_backend().await;
	let proxy = start_proxy(backend, session, Version::V1_6_4).await;
	let mut conn = login(proxy, "Steve").await?;
	let login_c::Ty::Disconnect(reason) = decode(&receive(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected a disconnect");
	};
	assert!(reason.contains("legacy session service"), "{reason}");
	Ok(())
}

async fn verify(conn: &mut Conn, name: &str, session: &Session, known: bool) -> Result<()> {
	let login_c::Ty::EncryptionBegin(request) = decode(&receive(conn).await?, login_c::deserialize)?
	else {
		bail!("expected the proxy to ask for encryption");
	};
	assert_eq!(request.server_id, "");
	let key = RsaPublicKey::from_public_key_der(&request.public_key)?;
	let secret: [u8; 16] = rand::random();
	if known {
		session.verified.lock().unwrap().insert(
			server_hash("", &secret, &request.public_key),
			(name.to_owned(), PROFILE),
		);
	}
	let seal = |data: &[u8]| key.encrypt(&mut rand::thread_rng(), Pkcs1v15Encrypt, data);
	let response = login_to_server_packet_encryption_begin::Ty {
		shared_secret: seal(&secret)?,
		verify_token: seal(&request.verify_token)?,
	};
	conn.send(encode(
		&login_s::Ty::EncryptionBegin(response),
		login_s::serialize,
	)?)
	.await?;
	conn.codec_mut().cipher = Some(Cipher::new(&secret));
	Ok(())
}

#[tokio::test]
async fn downstream_players_prove_their_account_before_joining() -> Result<()> {
	let session = session_server().await;
	let (backend, _hashes) = online_backend().await;
	let proxy = start_proxy(backend, session.addr, Version::V1_8).await;
	let mut conn = login(proxy, "Notch").await?;
	verify(&mut conn, "Notch", &session, true).await?;
	let login_c::Ty::Compress(threshold) =
		decode(&receive(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected compression");
	};
	conn.codec_mut().threshold = Some(usize::try_from(threshold)?);
	let login_c::Ty::Success(success) = decode(&receive(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected login success");
	};
	assert_eq!(success.uuid, hyphenated(PROFILE));
	let play_c::Ty::Login(_) = decode(&receive(&mut conn).await?, play_c::deserialize)? else {
		bail!("expected join game");
	};
	Ok(())
}

#[tokio::test]
async fn downstream_players_the_session_server_did_not_see_are_kicked() -> Result<()> {
	let session = session_server().await;
	let (backend, _hashes) = online_backend().await;
	let proxy = start_proxy(backend, session.addr, Version::V1_8).await;
	let mut conn = login(proxy, "Notch").await?;
	verify(&mut conn, "Notch", &session, false).await?;
	let login_c::Ty::Disconnect(reason) = decode(&receive(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected a disconnect");
	};
	assert!(reason.contains("did not see Notch join"), "{reason}");
	Ok(())
}
