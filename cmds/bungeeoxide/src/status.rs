use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context as _, Result};
use futures::SinkExt;
use minecraft_versions::gen::convert::v26_1::{
	handshaking_to_server_packet as handshake, handshaking_to_server_packet_set_protocol,
	status_to_client_packet as status_c, status_to_server_packet as status_s,
	status_to_server_packet_ping_start,
};
use minecraft_versions::{legacy_version_of, version_of, Session, VERSIONS};
use serde_json::{json, Map, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::codec::Framed;
use tracing::{info, warn};

use crate::codec::Codec;
use crate::{
	at, client_version, decode, encode, legacy, next, pivot, protocol_of, reach, split_target,
	Config,
};

const TIMEOUT: Duration = Duration::from_secs(3);
const LEGACY_REPLY: usize = 256;
const SECTION: char = '\u{a7}';

#[derive(Clone, Debug, PartialEq)]
pub struct Upstream {
	pub description: Value,
	pub online: i64,
	pub max: i64,
	pub sample: Vec<Value>,
	pub favicon: Option<String>,
	pub protocol: Option<i32>,
}

#[derive(Default)]
pub struct Cache {
	entry: Mutex<Option<(Instant, Option<Arc<Upstream>>)>>,
	fetches: std::sync::atomic::AtomicUsize,
}

impl Cache {
	pub fn fetches(&self) -> usize {
		self.fetches.load(std::sync::atomic::Ordering::Relaxed)
	}
}

pub async fn upstream(config: &Config) -> Option<Arc<Upstream>> {
	let mut entry = config.status.entry.lock().await;
	if let Some((when, value)) = entry.as_ref() {
		if when.elapsed() < config.status_cache {
			return value.clone();
		}
	}
	config
		.status
		.fetches
		.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	let fetched = match config.server() {
		Ok(sv) => ping(&config.target, sv).await,
		Err(e) => Err(e),
	};
	let value = match fetched {
		Ok(u) => Some(Arc::new(u)),
		Err(e) => {
			info!(
				"server list shows the bridge's own reply, {} did not answer: {e:#}",
				config.target
			);
			None
		}
	};
	*entry = Some((Instant::now(), value.clone()));
	value
}

pub async fn ping(target: &str, version: usize) -> Result<Upstream> {
	let work = async {
		if legacy::is_legacy(version) {
			legacy_ping(target, version).await
		} else {
			let protocol = protocol_of(version)
				.ok_or_else(|| anyhow!("no protocol number for {}", VERSIONS[version]))?;
			netty_ping(target, protocol).await
		}
	};
	tokio::time::timeout(TIMEOUT, work)
		.await
		.map_err(|_| anyhow!("{target} did not answer the status ping in time"))?
}

async fn connect(target: &str) -> Result<TcpStream> {
	let stream = TcpStream::connect(target)
		.await
		.with_context(|| format!("connecting to {target}"))?;
	stream.set_nodelay(true)?;
	Ok(stream)
}

async fn netty_ping(target: &str, protocol: i32) -> Result<Upstream> {
	let (host, port) = split_target(target)?;
	let mut conn = Framed::new(connect(target).await?, Codec::default());
	let hs = handshake::Ty::SetProtocol(handshaking_to_server_packet_set_protocol::Ty {
		protocol_version: protocol,
		server_host: host,
		server_port: port,
		next_state: 1,
	});
	conn.send(encode(&hs, handshake::serialize)?).await?;
	let request = status_s::Ty::PingStart(status_to_server_packet_ping_start::Ty {});
	conn.send(encode(&request, status_s::serialize)?).await?;
	match decode(&next(&mut conn).await?, status_c::deserialize)? {
		status_c::Ty::ServerInfo(info) => parse(&info),
		_ => bail!("expected the server info"),
	}
}

pub fn parse(info: &str) -> Result<Upstream> {
	let v: Value = serde_json::from_str(info).context("reading the server info")?;
	let players = &v["players"];
	Ok(Upstream {
		description: v
			.get("description")
			.cloned()
			.unwrap_or_else(|| Value::String(String::new())),
		online: players["online"].as_i64().unwrap_or(0),
		max: players["max"].as_i64().unwrap_or(0),
		sample: players["sample"].as_array().cloned().unwrap_or_default(),
		favicon: v["favicon"].as_str().map(|f| f.replace(['\n', '\r'], "")),
		protocol: v["version"]["protocol"]
			.as_i64()
			.and_then(|p| i32::try_from(p).ok()),
	})
}

pub fn legacy_request(version: usize, host: &str, port: u16) -> Vec<u8> {
	if version >= at("1.6.1") {
		let mut out = vec![0xfe, 0x01, 0xfa];
		legacy::string16(&mut out, "MC|PingHost");
		let mut data = vec![legacy::protocol_of(version).unwrap_or(78) as u8];
		legacy::string16(&mut data, host);
		data.extend_from_slice(&i32::from(port).to_be_bytes());
		out.extend_from_slice(&(data.len() as i16).to_be_bytes());
		out.extend_from_slice(&data);
		out
	} else if version >= at("1.4.2") {
		vec![0xfe, 0x01]
	} else {
		vec![0xfe]
	}
}

async fn legacy_ping(target: &str, version: usize) -> Result<Upstream> {
	let (host, port) = split_target(target)?;
	let mut stream = connect(target).await?;
	stream
		.write_all(&legacy_request(version, &host, port))
		.await?;
	let mut reply = Vec::new();
	let mut buf = [0u8; 4096];
	while !complete(&reply) && reply.len() < 1 << 16 {
		match stream.read(&mut buf).await {
			Ok(0) => break,
			Ok(n) => reply.extend_from_slice(&buf[..n]),
			Err(e) if reply.is_empty() => return Err(e.into()),
			Err(_) => break,
		}
	}
	parse_legacy(&reply)
}

fn complete(reply: &[u8]) -> bool {
	match reply {
		[0xff, a, b, rest @ ..] => rest.len() >= usize::from(u16::from_be_bytes([*a, *b])) * 2,
		_ => false,
	}
}

pub fn parse_legacy(reply: &[u8]) -> Result<Upstream> {
	if reply.first() != Some(&0xff) {
		bail!("not a pre-netty ping reply");
	}
	let (text, _) = legacy::read_string16(reply, 1)
		.ok_or_else(|| anyhow!("unreadable pre-netty ping reply"))?;
	let number = |s: &str| s.trim().parse::<i64>().unwrap_or(0);
	let (motd, online, max, protocol) = match text.strip_prefix("\u{a7}1\0") {
		Some(rest) => {
			let fields: Vec<&str> = rest.split('\0').collect();
			let field = |i: usize| fields.get(i).copied().unwrap_or_default();
			(
				field(2).to_owned(),
				number(field(3)),
				number(field(4)),
				field(0).parse().ok(),
			)
		}
		None => {
			let mut parts = text.rsplitn(3, SECTION);
			let max = number(parts.next().unwrap_or_default());
			let online = number(parts.next().unwrap_or_default());
			(
				parts.next().unwrap_or_default().to_owned(),
				online,
				max,
				None,
			)
		}
	};
	Ok(Upstream {
		description: Value::String(motd),
		online,
		max,
		sample: Vec::new(),
		favicon: None,
		protocol,
	})
}

pub async fn probe(target: &str, current: usize) -> Result<usize> {
	let hint = if legacy::is_legacy(current) {
		pivot()
	} else {
		current
	};
	let netty =
		tokio::time::timeout(TIMEOUT, netty_ping(target, protocol_of(hint).unwrap_or(-1))).await;
	let refused = match netty {
		Ok(Ok(status)) => {
			let protocol = status
				.protocol
				.ok_or_else(|| anyhow!("{target} does not name its protocol"))?;
			return version_of(protocol).ok_or_else(|| {
				anyhow!("{target} speaks protocol {protocol}, which the bridge does not know")
			});
		}
		Ok(Err(e)) => e.chain().any(|c| {
			c.downcast_ref::<std::io::Error>()
				.is_some_and(|e| e.kind() == std::io::ErrorKind::ConnectionRefused)
		}),
		Err(_) => false,
	};
	if refused {
		bail!("{target} refuses connections");
	}
	let reply = ping(target, at("1.6.4")).await?;
	match reply.protocol {
		Some(p) => legacy_version_of(p)
			.ok_or_else(|| anyhow!("{target} answered the pre-netty ping with protocol {p}")),
		None => bail!("{target} does not tell its version, name it after the address"),
	}
}

pub fn netty_reply(config: &Config, upstream: Option<&Upstream>, protocol: i32) -> Result<String> {
	let server = config.server()?;
	let (low, high) = reach(server);
	let served = client_version(protocol, server);
	let shown = served.or_else(|| version_of(protocol)).unwrap_or(high);
	let protocol = match served {
		Some(_) => protocol,
		None => protocol_of(high).unwrap_or(protocol),
	};
	let own = json!({ "text": config.motd });
	let (description, online, max) = match upstream {
		Some(u) => (&u.description, u.online, u.max),
		None => (&own, 0, 0),
	};
	let mut players = json!({ "max": max, "online": online });
	if let Some(sample) = upstream.map(|u| &u.sample).filter(|s| !s.is_empty()) {
		players["sample"] = Value::Array(sample.clone());
	}
	let mut reply = json!({
		"version": { "name": format!("pds2 bridge {}-{}", VERSIONS[low], VERSIONS[high]), "protocol": protocol },
		"players": players,
		"description": downgrade(description, shown),
	});
	if let Some(favicon) = upstream.and_then(|u| u.favicon.as_ref()) {
		reply["favicon"] = Value::String(favicon.clone());
	}
	let mut text = reply.to_string();
	if let Err(e) =
		minecraft_versions::custom::localize_status(&mut text, pivot(), shown, &mut Session::new())
	{
		warn!("can't translate the server list text: {e}");
	}
	Ok(text)
}

pub fn legacy_motd(description: &Value) -> String {
	let flat = downgrade(description, at("1.7")).to_string();
	minecraft_versions::custom::localize::legacy(&flat)
		.chars()
		.map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
		.filter(|c| *c != '\0')
		.collect()
}

pub fn strip_codes(text: &str) -> String {
	let mut out = String::with_capacity(text.len());
	let mut chars = text.chars();
	while let Some(c) = chars.next() {
		if c == SECTION {
			chars.next();
		} else {
			out.push(c);
		}
	}
	out
}

pub fn fit(motd: &str, room: usize) -> String {
	let mut out: String = motd.chars().take(room).collect();
	if out.ends_with(SECTION) {
		out.pop();
	}
	out
}

pub fn legacy_bare(motd: &str, online: i64, max: i64) -> String {
	let tail = format!("{SECTION}{online}{SECTION}{max}");
	let room = LEGACY_REPLY.saturating_sub(tail.chars().count());
	format!(
		"{}{tail}",
		fit(&strip_codes(motd).replace(SECTION, ""), room)
	)
}

pub fn legacy_fields(protocol: i32, version: &str, motd: &str, online: i64, max: i64) -> String {
	let head = format!("{SECTION}1\0{protocol}\0{version}\0");
	let tail = format!("\0{online}\0{max}");
	let room = LEGACY_REPLY.saturating_sub(head.chars().count() + tail.chars().count());
	format!("{head}{}{tail}", fit(motd, room))
}

pub fn downgrade(value: &Value, version: usize) -> Value {
	text(value, version, 0)
}

fn text(value: &Value, v: usize, depth: usize) -> Value {
	if depth > 32 {
		return Value::String(String::new());
	}
	match value {
		Value::String(_) => value.clone(),
		Value::Null => Value::String(String::new()),
		Value::Bool(_) | Value::Number(_) => Value::String(value.to_string()),
		Value::Array(items) if items.is_empty() => Value::String(String::new()),
		Value::Array(items) => Value::Array(items.iter().map(|i| text(i, v, depth + 1)).collect()),
		Value::Object(map) => Value::Object(component(map, v, depth)),
	}
}

fn plain(value: &Value) -> String {
	match value {
		Value::String(s) => s.clone(),
		Value::Null => String::new(),
		v => v.to_string(),
	}
}

fn components(value: &Value, v: usize, depth: usize) -> Vec<Value> {
	match value {
		Value::Array(items) => items.iter().map(|i| text(i, v, depth + 1)).collect(),
		Value::Null => Vec::new(),
		other => vec![text(other, v, depth + 1)],
	}
}

fn key_name(key: &str) -> String {
	let last = key.rsplit('.').next().unwrap_or(key);
	let mut c = last.chars();
	c.next()
		.map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
		.unwrap_or_default()
}

fn component(map: &Map<String, Value>, v: usize, depth: usize) -> Map<String, Value> {
	let mut out = Map::new();
	if let Some(t) = map.get("text") {
		out.insert("text".to_owned(), Value::String(plain(t)));
	} else if let Some(key) = map.get("translate") {
		out.insert("translate".to_owned(), Value::String(plain(key)));
		if let Some(with) = map.get("with") {
			let with = components(with, v, depth);
			if !with.is_empty() {
				out.insert("with".to_owned(), Value::Array(with));
			}
		}
		if let Some(fallback) = map.get("fallback").filter(|_| v >= at("1.19.4")) {
			out.insert("fallback".to_owned(), Value::String(plain(fallback)));
		}
	} else if let Some(key) = map.get("keybind") {
		match v >= at("1.12") {
			true => out.insert("keybind".to_owned(), Value::String(plain(key))),
			false => out.insert("text".to_owned(), Value::String(key_name(&plain(key)))),
		};
	} else if map.contains_key("object") && v >= at("1.21.9") {
		for (k, value) in map {
			if ["object", "atlas", "sprite", "player", "hat"].contains(&k.as_str()) {
				out.insert(k.clone(), value.clone());
			}
		}
	} else {
		let shown = match (map.get("score"), map.get("selector")) {
			(Some(score), _) => score.get("value").map(plain).unwrap_or_default(),
			(None, Some(selector)) => plain(selector),
			_ => String::new(),
		};
		out.insert("text".to_owned(), Value::String(shown));
	}
	for (k, value) in map {
		match k.as_str() {
			"color" => {
				if let Some(c) = color(value, v) {
					out.insert(k.clone(), Value::String(c));
				}
			}
			"bold" | "italic" | "underlined" | "strikethrough" | "obfuscated" => {
				let on = match value {
					Value::Bool(b) => *b,
					Value::String(s) => s == "true",
					Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
					_ => false,
				};
				out.insert(k.clone(), Value::Bool(on));
			}
			"font" if v >= at("1.16") => {
				out.insert(k.clone(), value.clone());
			}
			"shadow_color" if v >= at("1.21.4") => {
				out.insert(k.clone(), value.clone());
			}
			"extra" => {
				let extra = components(value, v, depth);
				if !extra.is_empty() {
					out.insert(k.clone(), Value::Array(extra));
				}
			}
			_ => {}
		}
	}
	out
}

const COLORS: [(&str, u32); 16] = [
	("black", 0x000000),
	("dark_blue", 0x0000aa),
	("dark_green", 0x00aa00),
	("dark_aqua", 0x00aaaa),
	("dark_red", 0xaa0000),
	("dark_purple", 0xaa00aa),
	("gold", 0xffaa00),
	("gray", 0xaaaaaa),
	("dark_gray", 0x555555),
	("blue", 0x5555ff),
	("green", 0x55ff55),
	("aqua", 0x55ffff),
	("red", 0xff5555),
	("light_purple", 0xff55ff),
	("yellow", 0xffff55),
	("white", 0xffffff),
];

fn color(value: &Value, v: usize) -> Option<String> {
	let name = value.as_str()?;
	let Some(hex) = name.strip_prefix('#') else {
		return (COLORS.iter().any(|(n, _)| *n == name) || name == "reset")
			.then(|| name.to_owned());
	};
	let rgb = u32::from_str_radix(hex, 16)
		.ok()
		.filter(|_| hex.len() == 6)?;
	if v >= at("1.16") {
		return Some(name.to_owned());
	}
	let channel = |c: u32, shift: u32| ((c >> shift) & 0xff) as i64;
	COLORS
		.iter()
		.min_by_key(|(_, c)| {
			[16, 8, 0]
				.iter()
				.map(|s| (channel(*c, *s) - channel(rgb, *s)).pow(2))
				.sum::<i64>()
		})
		.map(|(n, _)| (*n).to_owned())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn descriptions_keep_what_each_client_reads() {
		let modern = json!({
			"text": "",
			"extra": [
				{"text": "Hex", "color": "#ff5050", "shadow_color": -1, "font": "minecraft:uniform", "bold": 1},
				{"keybind": "key.jump"},
				{"score": {"name": "a", "objective": "b"}},
				{"text": "click", "click_event": {"action": "open_url", "url": "https://x"}, "hoverEvent": {"action": "show_text", "value": "x"}, "insertion": "i"},
				{"translate": "k", "with": [], "fallback": "F"},
				{"text": 5, "extra": []}
			]
		});
		let old = downgrade(&modern, at("1.8"));
		assert_eq!(
			old,
			json!({"text": "", "extra": [
				{"text": "Hex", "color": "red", "bold": true},
				{"text": "Jump"},
				{"text": ""},
				{"text": "click"},
				{"translate": "k"},
				{"text": "5"}
			]})
		);
		let new = downgrade(&modern, at("26.1"));
		assert_eq!(
			new["extra"][0],
			json!({"text": "Hex", "color": "#ff5050", "shadow_color": -1, "font": "minecraft:uniform", "bold": true})
		);
		assert_eq!(new["extra"][1], json!({"keybind": "key.jump"}));
		assert_eq!(new["extra"][4], json!({"translate": "k", "fallback": "F"}));
		assert_eq!(downgrade(&json!("plain"), at("1.7")), json!("plain"));
		assert_eq!(downgrade(&json!([]), at("1.7")), json!(""));
	}

	#[test]
	fn legacy_motds_are_section_text() {
		let motd = legacy_motd(
			&json!({"text": "A ", "extra": [{"text": "B", "color": "#40ff40"}, "\nC"]}),
		);
		assert_eq!(motd, "A \u{a7}aB\u{a7}r C");
		assert_eq!(legacy_bare(&motd, 3, 20), "A B C\u{a7}3\u{a7}20");
		assert_eq!(
			legacy_fields(78, "v", &motd, 3, 20),
			"\u{a7}1\x0078\x00v\x00A \u{a7}aB\u{a7}r C\x003\x0020"
		);
		let long = "\u{a7}a".repeat(200);
		let fields = legacy_fields(78, "v", &long, 1, 2);
		assert_eq!(fields.chars().count(), LEGACY_REPLY);
		assert!(fields.ends_with("\u{a7}a\x001\x002"));
		assert_eq!(legacy_bare(&long, 1, 2), "\u{a7}1\u{a7}2");
	}

	#[test]
	fn pings_parse_in_both_formats() {
		let mut reply = vec![0xff];
		legacy::string16(&mut reply, "\u{a7}1\x0078\x001.6.4\x00\u{a7}cHi\x005\x0010");
		let u = parse_legacy(&reply).unwrap();
		assert_eq!(
			(u.description.clone(), u.online, u.max, u.protocol),
			(json!("\u{a7}cHi"), 5, 10, Some(78))
		);
		let mut reply = vec![0xff];
		legacy::string16(&mut reply, "Old server\u{a7}1\u{a7}8");
		let u = parse_legacy(&reply).unwrap();
		assert_eq!(
			(u.description.clone(), u.online, u.max, u.protocol),
			(json!("Old server"), 1, 8, None)
		);
		let u = parse(r#"{"version":{"name":"x","protocol":47},"players":{"max":9,"online":2,"sample":[{"name":"a","id":"00000000-0000-0000-0000-000000000000"}]},"description":"d","favicon":"data:x\nyz"}"#).unwrap();
		assert_eq!(
			(
				u.online,
				u.max,
				u.sample.len(),
				u.favicon.as_deref(),
				u.protocol
			),
			(2, 9, 1, Some("data:xyz"), Some(47))
		);
	}

	#[test]
	fn requests_match_the_server_era() {
		assert_eq!(legacy_request(at("1.3.2"), "h", 1), [0xfe]);
		assert_eq!(legacy_request(at("1.5.2"), "h", 1), [0xfe, 0x01]);
		let host = legacy_request(at("1.6.4"), "h", 1);
		assert_eq!(&host[..3], [0xfe, 0x01, 0xfa]);
		assert_eq!(host[host.len() - 9], 78);
	}
}
