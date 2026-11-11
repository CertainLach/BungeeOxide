use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use bungeeoxide_minecraft_v1_21_11 as gen;
use pds_bytes_runtime::{self as pds, Output};

const THRESHOLD: usize = 64;

fn ser<T>(v: &T, f: fn(&T, &mut Output) -> pds::Result<()>) -> Vec<u8> {
	let mut out = Output::new();
	f(v, &mut out).unwrap();
	out.into_inner()
}

fn varint(v: usize) -> Vec<u8> {
	let mut v = v as u32;
	let mut out = Vec::new();
	loop {
		let b = (v & 0x7f) as u8;
		v >>= 7;
		if v == 0 {
			out.push(b);
			return out;
		}
		out.push(b | 0x80);
	}
}

fn frame(data: &[u8], compressed: bool) -> Vec<u8> {
	let body = if !compressed {
		data.to_vec()
	} else if data.len() < THRESHOLD {
		[varint(0), data.to_vec()].concat()
	} else {
		let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
		z.write_all(data).unwrap();
		[varint(data.len()), z.finish().unwrap()].concat()
	};
	[varint(body.len()), body].concat()
}

fn skip_frame(r: &mut impl Read) {
	let mut len = 0usize;
	for i in 0.. {
		let mut b = [0u8];
		r.read_exact(&mut b).unwrap();
		len |= ((b[0] & 0x7f) as usize) << (7 * i);
		if b[0] & 0x80 == 0 {
			break;
		}
	}
	let mut body = vec![0u8; len];
	r.read_exact(&mut body).unwrap();
}

fn fake_server(listener: TcpListener) {
	let (mut s, _) = listener.accept().unwrap();
	let mut r = BufReader::new(s.try_clone().unwrap());
	skip_frame(&mut r);
	skip_frame(&mut r);
	use gen::login::to_client::packet as login;
	s.write_all(&frame(
		&ser(&login::Ty::Compress(THRESHOLD as i32), login::serialize),
		false,
	))
	.unwrap();
	let success = login::Ty::Success(gen::login::to_client::PacketSuccess {
		uuid: 0x1234,
		username: "pds".to_owned(),
		properties: vec![],
	});
	s.write_all(&frame(&ser(&success, login::serialize), true))
		.unwrap();
	skip_frame(&mut r);
	use gen::configuration::to_client::packet as config;
	let brand = config::Ty::CustomPayload(gen::configuration::to_client::PacketCustomPayload {
		channel: "minecraft:brand".to_owned(),
		data: vec![7; 300],
	});
	s.write_all(&frame(&ser(&brand, config::serialize), true))
		.unwrap();
	let finish = config::Ty::FinishConfiguration(
		gen::configuration::to_client::PacketFinishConfiguration {},
	);
	s.write_all(&frame(&ser(&finish, config::serialize), true))
		.unwrap();
	skip_frame(&mut r);
	use gen::play::to_client::packet as play;
	s.write_all(&frame(
		&ser(&play::Ty::KeepAlive(42), play::serialize),
		true,
	))
	.unwrap();
	let mut rest = Vec::new();
	let _ = r.read_to_end(&mut rest);
}

fn fake_client(port: u16) {
	let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
	let mut r = BufReader::new(c.try_clone().unwrap());
	use gen::handshaking::to_server::packet as hs;
	let handshake = hs::Ty::SetProtocol(gen::handshaking::to_server::PacketSetProtocol {
		protocol_version: 774,
		server_host: "localhost".to_owned(),
		server_port: 25565,
		next_state: 2,
	});
	c.write_all(&frame(&ser(&handshake, hs::serialize), false))
		.unwrap();
	use gen::login::to_server::packet as login;
	let start = login::Ty::LoginStart(gen::login::to_server::PacketLoginStart {
		username: "pds".to_owned(),
		player_uuid: 0x1234,
	});
	c.write_all(&frame(&ser(&start, login::serialize), false))
		.unwrap();
	skip_frame(&mut r);
	skip_frame(&mut r);
	let ack = login::Ty::LoginAcknowledged(gen::login::to_server::PacketLoginAcknowledged {});
	c.write_all(&frame(&ser(&ack, login::serialize), true))
		.unwrap();
	skip_frame(&mut r);
	skip_frame(&mut r);
	use gen::configuration::to_server::packet as config;
	let finish = config::Ty::FinishConfiguration(
		gen::configuration::to_server::PacketFinishConfiguration {},
	);
	c.write_all(&frame(&ser(&finish, config::serialize), true))
		.unwrap();
	skip_frame(&mut r);
	c.shutdown(std::net::Shutdown::Both).unwrap();
}

#[test]
fn proxies_login_through_compression_into_play() {
	let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
	let upstream_port = upstream.local_addr().unwrap().port();
	let listen_port = TcpListener::bind("127.0.0.1:0")
		.unwrap()
		.local_addr()
		.unwrap()
		.port();
	let mut proxy = Command::new(env!("CARGO_BIN_EXE_bungeeoxide-packet-dumper"))
		.arg(format!("127.0.0.1:{listen_port}"))
		.arg(format!("127.0.0.1:{upstream_port}"))
		.stdout(Stdio::piped())
		.spawn()
		.unwrap();
	let mut out = BufReader::new(proxy.stdout.take().unwrap());
	let mut line = String::new();
	out.read_line(&mut line).unwrap();
	assert!(line.starts_with("listening"), "{line}");

	let server = thread::spawn(move || fake_server(upstream));
	fake_client(listen_port);
	server.join().unwrap();
	thread::sleep(Duration::from_millis(300));
	proxy.kill().unwrap();
	let mut log = String::new();
	out.read_to_string(&mut log).unwrap();
	println!("{log}");

	for bad in ["MISMATCH", "ERROR", "PANIC", " error:", " ! "] {
		assert!(!log.contains(bad), "found {bad:?} in proxy log");
	}
	for (dir, state, packet) in [
		("c2s", "Handshaking", "SetProtocol"),
		("c2s", "Login", "LoginStart"),
		("s2c", "Login", "Compress"),
		("s2c", "Login", "Success"),
		("c2s", "Login", "LoginAcknowledged"),
		("s2c", "Configuration", "CustomPayload"),
		("s2c", "Configuration", "FinishConfiguration"),
		("c2s", "Configuration", "FinishConfiguration"),
		("s2c", "Play", "KeepAlive"),
	] {
		assert!(
			log.lines()
				.any(|l| l.contains(&format!("{dir} {state:<13} ok       {packet}"))),
			"missing ok {dir} {state} {packet}"
		);
	}
}
