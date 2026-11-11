use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use bungeeoxide::codec::Codec;
use bungeeoxide::{decode, encode, next};
use futures::SinkExt;
use minecraft_versions::gen::convert::v1_12_2::{
	play_to_client_packet as play_c, play_to_server_packet as play_s,
	play_to_server_packet_position_look as position_look,
	play_to_server_packet_settings as settings,
};
use minecraft_versions::gen::convert::v1_8::{
	handshaking_to_server_packet as handshake, handshaking_to_server_packet_set_protocol,
	login_to_client_packet as login_c, login_to_server_packet as login_s,
};
use minecraft_versions::pds::Input;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

const PROTOCOL: i32 = 340;

fn kind(packet: &play_c::Ty) -> String {
	let debug = format!("{packet:?}");
	debug.split('(').next().unwrap_or(&debug).to_owned()
}

#[tokio::main]
async fn main() -> Result<()> {
	let mut args = std::env::args().skip(1);
	let addr = args.next().unwrap_or_else(|| "127.0.0.1:25577".to_owned());
	let seconds: u64 = args.next().map_or(Ok(10), |s| s.parse())?;
	let (host, port) = addr.rsplit_once(':').expect("host:port");
	let mut conn = Framed::new(TcpStream::connect(&addr).await?, Codec::default());
	let hs = handshake::Ty::SetProtocol(handshaking_to_server_packet_set_protocol::Ty {
		protocol_version: PROTOCOL,
		server_host: host.to_owned(),
		server_port: port.parse()?,
		next_state: 2,
	});
	conn.send(encode(&hs, handshake::serialize)?).await?;
	conn.send(encode(
		&login_s::Ty::LoginStart("probe12".to_owned()),
		login_s::serialize,
	)?)
	.await?;
	loop {
		match decode(&next(&mut conn).await?, login_c::deserialize)? {
			login_c::Ty::Compress(t) => conn.codec_mut().threshold = usize::try_from(t).ok(),
			login_c::Ty::Success(s) => {
				println!("logged in as {} {}", s.username, s.uuid);
				break;
			}
			other => bail!("login failed: {other:?}"),
		}
	}
	let client_settings = play_s::Ty::Settings(settings::Ty {
		locale: "en_US".to_owned(),
		view_distance: 4,
		chat_flags: 0,
		chat_colors: true,
		skin_parts: 0x7f,
		main_hand: 1,
	});
	let mut counts: BTreeMap<String, usize> = BTreeMap::new();
	let mut failed: BTreeMap<String, String> = BTreeMap::new();
	let mut greeted = false;
	let until = Instant::now() + Duration::from_secs(seconds);
	while let Ok(data) = tokio::time::timeout_at(until.into(), next(&mut conn)).await {
		let data = data?;
		let mut input = Input::new(&data);
		let packet = match play_c::deserialize(&mut input) {
			Ok(p) if input.remaining() == 0 => p,
			Ok(p) => {
				failed.insert(kind(&p), format!("{} bytes left over", input.remaining()));
				continue;
			}
			Err(e) => {
				let id = bungeeoxide::packet_id(&data).unwrap_or(-1);
				failed.insert(format!("{id:#04x}"), format!("{e:?}"));
				continue;
			}
		};
		match &packet {
			play_c::Ty::Login(_) => {
				conn.send(encode(&client_settings, play_s::serialize)?)
					.await?
			}
			play_c::Ty::KeepAlive(id) => {
				conn.send(encode(&play_s::Ty::KeepAlive(*id), play_s::serialize)?)
					.await?
			}
			play_c::Ty::Chat(c) => println!("chat: {}", c.message),
			play_c::Ty::Position(p) => {
				println!(
					"teleported to {:.2} {:.2} {:.2} (id {})",
					p.x, p.y, p.z, p.teleport_id
				);
				conn.send(encode(
					&play_s::Ty::TeleportConfirm(p.teleport_id),
					play_s::serialize,
				)?)
				.await?;
				let echo = play_s::Ty::PositionLook(position_look::Ty {
					x: p.x,
					y: p.y,
					z: p.z,
					yaw: p.yaw,
					pitch: p.pitch,
					on_ground: false,
				});
				conn.send(encode(&echo, play_s::serialize)?).await?;
				if !greeted {
					greeted = true;
					for message in ["/proxy-ping", "hello from 1.12.2"] {
						conn.send(encode(
							&play_s::Ty::Chat(message.to_owned()),
							play_s::serialize,
						)?)
						.await?;
					}
				}
			}
			_ => {}
		}
		*counts.entry(kind(&packet)).or_default() += 1;
	}
	for (k, n) in &counts {
		println!("{n:>6} {k}");
	}
	for (id, e) in &failed {
		println!("undecodable {id}: {e}");
	}
	Ok(())
}
