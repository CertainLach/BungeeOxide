use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, ensure, Result};
use bungeeoxide::codec::Codec;
use bungeeoxide::users::User;
use bungeeoxide::{decode, encode, next, protocol_of, serve, Config, Conn};
use bungeeoxide_converter::gen::convert::packet_name;
use bungeeoxide_converter::gen::convert::v26_4_snapshot_10::{
	configuration_to_client_packet as config_c,
	configuration_to_client_packet_finish_configuration,
	configuration_to_server_packet as config_s,
	configuration_to_server_packet_finish_configuration, handshaking_to_server_packet as handshake,
	handshaking_to_server_packet_set_protocol, login_to_client_connection_options as options,
	login_to_client_packet as login_c, login_to_client_packet_key, login_to_client_packet_success,
	login_to_server_packet as login_s, login_to_server_packet_login_acknowledged,
	login_to_server_packet_login_start,
};
use bungeeoxide_converter::{Root, Version};
use futures::SinkExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::codec::Framed;

const WAIT: Duration = Duration::from_secs(10);
const SNAPSHOT: Version = Version::V26_4_snapshot_10;
const BACKEND_THRESHOLD: i32 = 48;
const PROXY_THRESHOLD: usize = 64;

async fn read(conn: &mut Conn) -> Result<Vec<u8>> {
	Ok(tokio::time::timeout(WAIT, next(conn)).await??)
}

async fn backend(online: bool) -> (SocketAddr, mpsc::UnboundedReceiver<String>) {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let (events, received) = mpsc::unbounded_channel();
	tokio::spawn(async move {
		loop {
			let (stream, _) = listener.accept().await.unwrap();
			let events = events.clone();
			tokio::spawn(async move {
				let conn = Framed::new(stream, Codec::default());
				if let Err(e) = serve_login(conn, online, &events).await {
					let _ = events.send(format!("error {e:#}"));
				}
			});
		}
	});
	(addr, received)
}

async fn serve_login(
	mut conn: Conn,
	online: bool,
	events: &mpsc::UnboundedSender<String>,
) -> Result<()> {
	let handshake::Ty::SetProtocol(hs) = decode(&read(&mut conn).await?, handshake::deserialize)?
	else {
		bail!("handshake");
	};
	ensure!(hs.protocol_version == protocol_of(SNAPSHOT).unwrap());
	let login_s::Ty::LoginStart(start) = decode(&read(&mut conn).await?, login_s::deserialize)?
	else {
		bail!("login start");
	};
	if online {
		let key = login_c::Ty::Key(login_to_client_packet_key::Ty {
			salt: 1,
			server_id: "server".to_owned(),
			public_key: vec![0; 44],
			should_authenticate: true,
		});
		conn.send(encode(&key, login_c::serialize)?).await?;
		read(&mut conn).await?;
		return Ok(());
	}
	conn.send(encode(
		&login_c::Ty::SetupCompression(BACKEND_THRESHOLD),
		login_c::serialize,
	)?)
	.await?;
	let success = login_c::Ty::Success(login_to_client_packet_success::Ty {
		uuid: start.player_uuid,
		username: start.username,
		properties: Vec::new(),
		session_id: 9,
		options: options::Ty {
			encryption: false,
			compression: true,
		},
	});
	conn.send(encode(&success, login_c::serialize)?).await?;
	let ack = read(&mut conn).await?;
	ensure!(
		packet_name(Root::LoginToServer, SNAPSHOT, &ack) == Some("login_acknowledged"),
		"the proxy acknowledged with {:?}",
		packet_name(Root::LoginToServer, SNAPSHOT, &ack)
	);
	let _ = events.send("acknowledged uncompressed".to_owned());
	conn.codec_mut().threshold = usize::try_from(BACKEND_THRESHOLD).ok();
	let finish = config_c::Ty::FinishConfiguration(
		configuration_to_client_packet_finish_configuration::Ty {},
	);
	conn.send(encode(&finish, config_c::serialize)?).await?;
	loop {
		let data = read(&mut conn).await?;
		if packet_name(Root::ConfigurationToServer, SNAPSHOT, &data) == Some("finish_configuration")
		{
			let _ = events.send("configured compressed".to_owned());
			return Ok(());
		}
	}
}

async fn proxy(target: SocketAddr) -> SocketAddr {
	proxy_for(target, User::guest("snap")).await
}

async fn proxy_for(target: SocketAddr, user: User) -> SocketAddr {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let config = Arc::new(Config {
		target: target.to_string(),
		motd: "snapshot".to_owned(),
		threshold: PROXY_THRESHOLD,
		server_version: SNAPSHOT,
		users: vec![user],
		..Config::default()
	});
	tokio::spawn(serve(listener, config));
	addr
}

async fn start_login(proxy: SocketAddr) -> Result<Conn> {
	let mut conn = Framed::new(TcpStream::connect(proxy).await?, Codec::default());
	let hs = handshake::Ty::SetProtocol(handshaking_to_server_packet_set_protocol::Ty {
		protocol_version: protocol_of(SNAPSHOT).unwrap(),
		server_host: "localhost".to_owned(),
		server_port: proxy.port(),
		next_state: 2,
	});
	conn.send(encode(&hs, handshake::serialize)?).await?;
	let start = login_s::Ty::LoginStart(login_to_server_packet_login_start::Ty {
		username: "snap".to_owned(),
		player_uuid: 5,
	});
	conn.send(encode(&start, login_s::serialize)?).await?;
	Ok(conn)
}

async fn event(rx: &mut mpsc::UnboundedReceiver<String>) -> Option<String> {
	tokio::time::timeout(WAIT, rx.recv()).await.ok().flatten()
}

#[tokio::test]
async fn compression_starts_after_the_login_acknowledgement() -> Result<()> {
	let (backend, mut events) = backend(false).await;
	let proxy = proxy(backend).await;
	let mut conn = start_login(proxy).await?;
	let login_c::Ty::SetupCompression(threshold) =
		decode(&read(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected setup_compression");
	};
	assert_eq!(threshold, PROXY_THRESHOLD as i32);
	let login_c::Ty::Success(success) = decode(&read(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected login_finished");
	};
	assert_eq!(success.username, "snap");
	assert!(success.options.compression);
	assert!(!success.options.encryption);
	let ack = login_s::Ty::LoginAcknowledged(login_to_server_packet_login_acknowledged::Ty {});
	conn.send(encode(&ack, login_s::serialize)?).await?;
	conn.codec_mut().threshold = Some(PROXY_THRESHOLD);
	assert_eq!(
		event(&mut events).await.as_deref(),
		Some("acknowledged uncompressed")
	);
	loop {
		let data = read(&mut conn).await?;
		if let config_c::Ty::FinishConfiguration(_) = decode(&data, config_c::deserialize)? {
			break;
		}
	}
	let finish = config_s::Ty::FinishConfiguration(
		configuration_to_server_packet_finish_configuration::Ty {},
	);
	conn.send(encode(&finish, config_s::serialize)?).await?;
	assert_eq!(
		event(&mut events).await.as_deref(),
		Some("configured compressed")
	);
	Ok(())
}

#[tokio::test]
async fn online_mode_snapshot_backends_are_refused_clearly() -> Result<()> {
	let (backend, _events) = backend(true).await;
	let proxy = proxy(backend).await;
	let mut conn = start_login(proxy).await?;
	let login_c::Ty::Disconnect(reason) = decode(&read(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected a disconnect");
	};
	assert!(
		reason.contains("key exchange is not implemented"),
		"{reason}"
	);
	Ok(())
}

#[tokio::test]
async fn online_mode_snapshot_clients_are_refused_clearly() -> Result<()> {
	let (backend, _events) = backend(false).await;
	let user = User {
		downstream: true,
		..User::guest("snap")
	};
	let proxy = proxy_for(backend, user).await;
	let mut conn = start_login(proxy).await?;
	let login_c::Ty::Disconnect(reason) = decode(&read(&mut conn).await?, login_c::deserialize)?
	else {
		bail!("expected a disconnect");
	};
	assert!(
		reason.contains("key exchange is not implemented"),
		"{reason}"
	);
	Ok(())
}
