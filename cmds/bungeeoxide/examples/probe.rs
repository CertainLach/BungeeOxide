use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use bungeeoxide::codec::Codec;
use bungeeoxide::{decode, encode, next};
use futures::SinkExt;
use minecraft_versions::gen::convert::v1_8::{
	handshaking_toServer_packet as handshake, handshaking_toServer_packet_set_protocol,
	login_toClient_packet as login_c, login_toServer_packet as login_s,
	play_toClient_packet as play_c, play_toServer_packet as play_s,
};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

const PROTOCOL: i32 = 47;

fn kind(packet: &play_c::Ty) -> String {
	let debug = format!("{packet:?}");
	debug.split('(').next().unwrap_or(&debug).to_owned()
}

#[tokio::main]
async fn main() -> Result<()> {
	let mut args = std::env::args().skip(1);
	let addr = args.next().unwrap_or_else(|| "127.0.0.1:25577".to_owned());
	let seconds: u64 = args.next().map_or(Ok(10), |s| s.parse())?;
	let mut dump = match args.next() {
		Some(path) => Some(std::fs::File::create(path)?),
		None => None,
	};
	let (host, port) = addr.rsplit_once(':').expect("host:port");
	let mut conn = Framed::new(TcpStream::connect(&addr).await?, Codec::default());
	let hs = handshake::Ty::set_protocol(handshaking_toServer_packet_set_protocol::Ty {
		protocolVersion: PROTOCOL,
		serverHost: host.to_owned(),
		serverPort: port.parse()?,
		nextState: 2,
	});
	conn.send(encode(&hs, handshake::serialize)?).await?;
	conn.send(encode(
		&login_s::Ty::login_start("probe".to_owned()),
		login_s::serialize,
	)?)
	.await?;
	loop {
		match decode(&next(&mut conn).await?, login_c::deserialize)? {
			login_c::Ty::compress(t) => conn.codec_mut().threshold = usize::try_from(t).ok(),
			login_c::Ty::success(s) => {
				println!("logged in as {} {}", s.username, s.uuid);
				break;
			}
			other => bail!("login failed: {other:?}"),
		}
	}
	let mut counts: BTreeMap<String, usize> = BTreeMap::new();
	let mut failed: BTreeMap<String, String> = BTreeMap::new();
	let mut greeted = false;
	let until = Instant::now() + Duration::from_secs(seconds);
	while let Ok(data) = tokio::time::timeout_at(until.into(), next(&mut conn)).await {
		let data = data?;
		if let Some(file) = &mut dump {
			use std::io::Write;
			file.write_all(&(data.len() as u32).to_be_bytes())?;
			file.write_all(&data)?;
		}
		match decode(&data, play_c::deserialize) {
			Ok(packet) => {
				if let play_c::Ty::keep_alive(id) = packet {
					conn.send(encode(&play_s::Ty::keep_alive(id), play_s::serialize)?)
						.await?;
				}
				if let play_c::Ty::chat(c) = &packet {
					println!("chat: {}", c.message);
				}
				if !greeted && matches!(packet, play_c::Ty::position(_)) {
					greeted = true;
					for message in ["/proxy-ping", "hello from the probe"] {
						conn.send(encode(
							&play_s::Ty::chat(message.to_owned()),
							play_s::serialize,
						)?)
						.await?;
					}
				}
				*counts.entry(kind(&packet)).or_default() += 1;
			}
			Err(e) => {
				let id = bungeeoxide::packet_id(&data).unwrap_or(-1);
				failed.insert(format!("{id:#04x}"), format!("{e:#}"));
			}
		}
	}
	for (k, n) in &counts {
		println!("{n:>6} {k}");
	}
	for (id, e) in &failed {
		println!("undecodable {id}: {e}");
	}
	Ok(())
}
