use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::io::Read;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context as _, Result};
use bungeeoxide::codec::Codec;
use bungeeoxide::legacy::{login_request, string16, Legacy};
use bungeeoxide::{decode, encode, next};
use flate2::read::ZlibDecoder;
use futures::{SinkExt, StreamExt};
use minecraft_versions::gen::convert::{packet_name, v1_0_1};
use minecraft_versions::{index_of, Root};
use serde_json::{json, Map, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Notify};
use tokio::time::{sleep, MissedTickBehavior};
use tokio_util::codec::Framed;

use v1_0_1::play_to_client_packet::Ty as C;
use v1_0_1::play_to_server_packet::Ty as S;

const NAME: &str = "legacy_bot";
const PROTOCOL: i32 = 22;
const HEIGHT: i32 = 128;
const EYES: f64 = 1.6200000047683716;
const HOME: i32 = 2000;

const BLOCKS: [(u8, &str, &str); 24] = [
	(41, "gold_block", "gold_block"),
	(57, "diamond_block", "diamond_block"),
	(42, "iron_block", "iron_block"),
	(22, "lapis_block", "lapis_block"),
	(45, "bricks", "brick_block"),
	(49, "obsidian", "obsidian"),
	(47, "bookshelf", "bookshelf"),
	(19, "sponge", "sponge"),
	(89, "glowstone", "glowstone"),
	(87, "netherrack", "netherrack"),
	(88, "soul_sand", "soul_sand"),
	(24, "sandstone", "sandstone"),
	(48, "mossy_cobblestone", "mossy_cobblestone"),
	(20, "glass", "glass"),
	(86, "pumpkin", "pumpkin"),
	(35, "white_wool", "wool"),
	(84, "jukebox", "jukebox"),
	(25, "note_block", "noteblock"),
	(46, "tnt", "tnt"),
	(80, "snow_block", "snow"),
	(58, "crafting_table", "crafting_table"),
	(112, "nether_bricks", "nether_brick"),
	(121, "end_stone", "end_stone"),
	(103, "melon", "melon_block"),
];

const PASSABLE: [u8; 40] = [
	0, 6, 8, 9, 10, 11, 27, 28, 30, 31, 32, 37, 38, 39, 40, 50, 51, 55, 59, 63, 65, 66, 68, 69, 70,
	72, 75, 76, 77, 78, 83, 90, 93, 94, 104, 105, 106, 111, 115, 119,
];

const OPAQUE: [u8; 45] = [
	1, 2, 3, 4, 5, 7, 12, 13, 14, 15, 16, 17, 19, 21, 22, 23, 24, 25, 35, 41, 42, 43, 45, 46, 47,
	48, 49, 56, 57, 58, 61, 62, 73, 74, 80, 82, 84, 86, 87, 91, 97, 98, 103, 110, 112,
];

fn solid(id: u8) -> bool {
	!PASSABLE.contains(&id)
}

fn opaque(id: u8) -> bool {
	OPAQUE.contains(&id) || id == 121
}

fn liquid(id: u8) -> bool {
	(8..=11).contains(&id)
}

fn round2(v: f64) -> f64 {
	(v * 100.0).round() / 100.0
}

struct Flavor {
	modern: bool,
	bottom: i32,
	top: i32,
	fly_to: f64,
	deep: i32,
	high: i32,
	middle: i32,
	markers: Vec<(i32, &'static str, u8)>,
}

impl Flavor {
	fn new(version: &str) -> Flavor {
		let modern = !version.starts_with("1.8");
		let ys: &[i32] = if modern {
			&[
				-63, -48, -17, -1, 0, 1, 15, 16, 63, 64, 100, 127, 128, 191, 192, 200, 240, 255,
				256, 290, 300, 319,
			]
		} else {
			&[
				1, 5, 15, 16, 31, 32, 63, 64, 100, 127, 128, 129, 160, 191, 192, 200, 224, 240, 255,
			]
		};
		let markers = ys
			.iter()
			.zip(BLOCKS)
			.map(|(y, (id, new, old))| (*y, if modern { new } else { old }, id))
			.collect();
		Flavor {
			modern,
			bottom: if modern { -64 } else { 0 },
			top: if modern { 320 } else { 256 },
			fly_to: if modern { 300.0 } else { 250.0 },
			deep: if modern { -42 } else { 10 },
			high: if modern { 290 } else { 240 },
			middle: 120,
			markers,
		}
	}
}

struct Column {
	blocks: Vec<u8>,
	sky: Vec<u8>,
}

impl Column {
	fn new() -> Column {
		let n = (16 * 16 * HEIGHT) as usize;
		Column {
			blocks: vec![0; n],
			sky: vec![15; n],
		}
	}
}

fn slot(x: i32, y: i32, z: i32) -> usize {
	(((x & 15) * 16 + (z & 15)) * HEIGHT + y) as usize
}

fn nibble(data: &[u8], i: usize) -> u8 {
	data.get(i / 2).map_or(0, |b| b >> ((i & 1) * 4) & 15)
}

#[derive(Clone, Copy)]
struct Join {
	seed: i64,
	mode: i32,
	dimension: i8,
	difficulty: u8,
	height: u8,
}

#[derive(Default)]
struct State {
	pos: [f64; 3],
	ground: bool,
	flying: bool,
	vy: f64,
	fly: f64,
	spawned: bool,
	dead: bool,
	teleports: Vec<f64>,
	reads: usize,
	respawns: usize,
	respawn_requests: usize,
	respawn_tries: usize,
	respawn_sent: Option<Instant>,
	join: Option<Join>,
	chunks: HashMap<(i32, i32), Column>,
	kicked: Option<String>,
	closed: Option<String>,
	packets: BTreeMap<&'static str, usize>,
	issues: BTreeMap<String, usize>,
}

impl State {
	fn block(&self, x: i32, y: i32, z: i32) -> Option<u8> {
		let column = self.chunks.get(&(x >> 4, z >> 4))?;
		if !(0..HEIGHT).contains(&y) {
			return Some(0);
		}
		Some(column.blocks[slot(x, y, z)])
	}

	fn sky(&self, x: i32, y: i32, z: i32) -> Option<u8> {
		let column = self.chunks.get(&(x >> 4, z >> 4))?;
		if !(0..HEIGHT).contains(&y) {
			return Some(15);
		}
		Some(column.sky[slot(x, y, z)])
	}

	fn set_block(&mut self, x: i32, y: i32, z: i32, id: u8) -> bool {
		match self.chunks.get_mut(&(x >> 4, z >> 4)) {
			Some(column) if (0..HEIGHT).contains(&y) => {
				column.blocks[slot(x, y, z)] = id;
				true
			}
			_ => false,
		}
	}

	fn moved(&mut self, pos: [f64; 3]) {
		self.pos = pos;
		self.vy = 0.0;
		self.spawned = true;
		self.teleports.push(pos[1]);
	}
}

struct Link {
	state: Mutex<State>,
	out: mpsc::UnboundedSender<Vec<u8>>,
	tag: String,
	version: usize,
	ended: Notify,
}

impl Link {
	fn st(&self) -> MutexGuard<'_, State> {
		self.state.lock().unwrap_or_else(|e| e.into_inner())
	}

	fn send(&self, packet: S) {
		match encode(&packet, v1_0_1::play_to_server_packet::serialize) {
			Ok(data) => {
				let _ = self.out.send(data);
			}
			Err(e) => self.log(format!("can't encode {packet:?}: {e:#}")),
		}
	}

	fn log(&self, text: impl Display) {
		eprintln!("{} {text}", self.tag);
	}

	fn issue(&self, st: &mut State, kind: &str, detail: impl Display) {
		let count = st.issues.entry(kind.to_owned()).or_default();
		*count += 1;
		if *count == 1 {
			self.log(format!("PROBLEM {kind}: {detail}"));
		}
	}
}

fn map_chunk(
	st: &mut State,
	m: &v1_0_1::play_to_client_packet_map_chunk::Ty,
) -> std::result::Result<(), (&'static str, String)> {
	let (sx, sy, sz) = (
		usize::from(m.size_x) + 1,
		usize::from(m.size_y) + 1,
		usize::from(m.size_z) + 1,
	);
	let volume = sx * sy * sz;
	let half = volume / 2;
	let mut raw = Vec::new();
	ZlibDecoder::new(&m.compressed_chunk_data[..])
		.read_to_end(&mut raw)
		.map_err(|e| ("map_chunk data does not inflate", e.to_string()))?;
	if raw.len() < volume + 3 * half {
		return Err((
			"map_chunk data is too short",
			format!(
				"{sx}x{sy}x{sz} at {},{},{} has {} bytes, expected {}",
				m.x,
				m.y,
				m.z,
				raw.len(),
				volume + 3 * half
			),
		));
	}
	let ids = &raw[..volume];
	let sky = &raw[volume + 2 * half..volume + 3 * half];
	let y0 = i32::from(m.y);
	let mut missing = None;
	let mut i = 0;
	for dx in 0..sx {
		for dz in 0..sz {
			let (x, z) = (m.x + dx as i32, m.z + dz as i32);
			let Some(column) = st.chunks.get_mut(&(x >> 4, z >> 4)) else {
				missing = Some((x >> 4, z >> 4));
				i += sy;
				continue;
			};
			for dy in 0..sy {
				let y = y0 + dy as i32;
				if (0..HEIGHT).contains(&y) {
					let at = slot(x, y, z);
					column.blocks[at] = ids[i];
					column.sky[at] = nibble(sky, i);
				}
				i += 1;
			}
		}
	}
	if y0 < 0 || y0 + sy as i32 > HEIGHT {
		return Err((
			"map_chunk cuboid leaves client y 0..128",
			format!("y {}..{} at {},{}", y0, y0 + sy as i32, m.x, m.z),
		));
	}
	if let Some((cx, cz)) = missing {
		return Err((
			"map_chunk for a chunk that was never pre-loaded (vanilla 1.0 drops it)",
			format!("chunk {cx},{cz}"),
		));
	}
	Ok(())
}

fn on_packet(link: &Link, st: &mut State, data: &[u8]) {
	let name = packet_name(Root::PlayToClient, link.version, data).unwrap_or("unknown");
	*st.packets.entry(name).or_default() += 1;
	let packet = match decode(data, v1_0_1::play_to_client_packet::deserialize) {
		Ok(p) => p,
		Err(e) => {
			link.issue(st, "undecodable packet", format!("{name}: {e:#}"));
			return;
		}
	};
	match packet {
		C::KeepAlive(id) => link.send(S::KeepAlive(id)),
		C::Login(l) => {
			st.join = Some(Join {
				seed: l.seed,
				mode: l.game_mode,
				dimension: l.dimension,
				difficulty: l.difficulty,
				height: l.world_height,
			});
			link.log(format!(
				"login {}",
				json!({ "entity": l.entity_id, "mode": l.game_mode, "dimension": l.dimension, "worldHeight": l.world_height })
			));
		}
		C::Position(p) => {
			let feet = p.y - EYES;
			if (p.stance - feet).abs() > 0.01 {
				link.issue(
					st,
					"position packet stance is not y - 1.62",
					format!("y {} stance {}", p.y, p.stance),
				);
			}
			st.moved([p.x, feet, p.z]);
			link.send(S::PositionLook(
				v1_0_1::play_to_server_packet_position_look::Ty {
					x: p.x,
					y: feet,
					stance: p.y,
					z: p.z,
					yaw: p.yaw,
					pitch: p.pitch,
					on_ground: p.on_ground,
				},
			));
		}
		C::PositionOnly(p) => {
			let feet = p.y - EYES;
			st.moved([p.x, feet, p.z]);
			link.send(S::Position(v1_0_1::play_to_server_packet_position::Ty {
				x: p.x,
				y: feet,
				stance: p.y,
				z: p.z,
				on_ground: p.on_ground,
			}));
		}
		C::Look(p) => link.send(S::Look(v1_0_1::play_to_server_packet_look::Ty {
			yaw: p.yaw,
			pitch: p.pitch,
			on_ground: p.on_ground,
		})),
		C::MapChunk(m) => {
			if let Err((kind, detail)) = map_chunk(st, &m) {
				link.issue(st, kind, detail);
			}
		}
		C::PreChunk(p) => {
			if p.mode {
				st.chunks.insert((p.x, p.z), Column::new());
			} else {
				st.chunks.remove(&(p.x, p.z));
			}
		}
		C::BlockChange(b) => {
			let (x, y, z) = (b.location.x, i32::from(b.location.y), b.location.z);
			if y >= HEIGHT {
				link.issue(
					st,
					"block_change above client y 127",
					format!("{x},{y},{z}"),
				);
			} else if !st.set_block(x, y, z, b.r#type) {
				*st.packets
					.entry("block_change outside loaded chunks")
					.or_default() += 1;
			}
		}
		C::MultiBlockChange(m) => {
			for (i, c) in m.coordinates.iter().enumerate() {
				let (x, y, z) = (
					m.chunk_x * 16 + i32::from(c.x),
					i32::from(c.y),
					m.chunk_z * 16 + i32::from(c.z),
				);
				if y >= HEIGHT {
					link.issue(
						st,
						"multi_block_change above client y 127",
						format!("{x},{y},{z}"),
					);
					continue;
				}
				let id = m.block_ids.get(i).copied().unwrap_or(0);
				if !st.set_block(x, y, z, id) {
					*st.packets
						.entry("multi_block_change outside loaded chunks")
						.or_default() += 1;
				}
			}
		}
		C::UpdateHealth(h) => {
			if h.health <= 0 && !st.dead {
				st.dead = true;
				st.respawn_tries = 0;
				link.log(format!("health {}, sending respawn", h.health));
				link.send(respawn_request(st));
			}
		}
		C::Respawn(r) => {
			let previous = st.join.map(|j| j.dimension);
			if previous != Some(r.dimension) {
				st.chunks.clear();
			}
			if let Some(join) = st.join.as_mut() {
				join.dimension = r.dimension;
				join.difficulty = r.difficulty;
				join.mode = i32::from(r.gamemode);
				join.seed = r.seed;
			}
			st.dead = false;
			st.spawned = false;
			st.fly = 0.0;
			st.vy = 0.0;
			st.respawns += 1;
			link.log(format!(
				"respawn {}",
				json!({ "dimension": r.dimension, "gamemode": r.gamemode, "worldHeight": r.world_height })
			));
		}
		C::GameStateChange(g) => {
			use v1_0_1::play_to_client_packet_game_state_change_reason::Ty as Reason;
			if matches!(g.reason, Reason::ChangeGameMode) {
				if let Some(join) = st.join.as_mut() {
					join.mode = i32::from(g.game_mode);
				}
				link.log(format!("game mode {}", g.game_mode));
			}
		}
		C::KickDisconnect(reason) => st.kicked = Some(reason),
		C::Chat(text) => link.log(format!("chat {text:?}")),
		_ => {}
	}
}

fn fall(st: &mut State) {
	let [x, y, z] = st.pos;
	let (bx, bz) = (x.floor() as i32, z.floor() as i32);
	if !st.chunks.contains_key(&(bx >> 4, bz >> 4)) || y < -32.0 {
		return;
	}
	let vy = ((st.vy - 0.08) * 0.98).max(-1.0);
	let target = y + vy;
	let high = (y + 1e-9).floor() as i32 - 1;
	let low = (target - 1e-9).ceil() as i32 - 1;
	for by in (low..=high).rev() {
		if st.block(bx, by, bz).is_some_and(solid) {
			st.pos[1] = f64::from(by + 1);
			st.vy = 0.0;
			st.ground = true;
			return;
		}
	}
	st.pos[1] = target;
	st.vy = vy;
	st.ground = false;
}

fn respawn_request(st: &mut State) -> S {
	let join = st.join.unwrap_or(Join {
		seed: 0,
		mode: 0,
		dimension: 0,
		difficulty: 0,
		height: 128,
	});
	st.respawn_requests += 1;
	st.respawn_tries += 1;
	st.respawn_sent = Some(Instant::now());
	S::Respawn(v1_0_1::play_to_server_packet_respawn::Ty {
		dimension: join.dimension,
		difficulty: join.difficulty,
		gamemode: join.mode as u8,
		world_height: i16::from(join.height),
		seed: join.seed,
	})
}

fn tick(link: &Link, st: &mut State) -> Option<S> {
	if st.kicked.is_some() {
		return None;
	}
	if st.dead {
		let stale = st
			.respawn_sent
			.is_some_and(|t| t.elapsed() > Duration::from_secs(5));
		if stale && st.respawn_tries < 4 {
			link.log("respawn not answered, sending it again");
			return Some(respawn_request(st));
		}
		return None;
	}
	if !st.spawned {
		return None;
	}
	if st.fly != 0.0 {
		let step = st.fly.clamp(-0.5, 0.5);
		st.pos[1] += step;
		st.fly -= step;
		if st.fly.abs() < 1e-9 {
			st.fly = 0.0;
		}
		st.ground = false;
		st.vy = 0.0;
	} else if !st.flying {
		fall(st);
	}
	let [x, y, z] = st.pos;
	Some(S::Position(v1_0_1::play_to_server_packet_position::Ty {
		x,
		y,
		stance: y + EYES,
		z,
		on_ground: st.ground,
	}))
}

async fn connect(address: &str, tag: String) -> Result<Arc<Link>> {
	let version = index_of("1.0.1").ok_or_else(|| anyhow!("1.0.1 is not a known version"))?;
	let stream = TcpStream::connect(address)
		.await
		.with_context(|| format!("connecting to {address}"))?;
	stream.set_nodelay(true)?;
	let mut conn = Framed::new(
		stream,
		Codec {
			threshold: None,
			legacy: Some(Legacy::new(Root::PlayToClient, Some(version))),
			cipher: None,
		},
	);
	let mut hello = vec![0x02];
	string16(&mut hello, NAME);
	conn.send(hello).await?;
	let reply = next(&mut conn).await?;
	match decode(&reply, v1_0_1::play_to_client_packet::deserialize)? {
		C::Handshake(_) => {}
		C::KickDisconnect(reason) => bail!("the bridge refused the handshake: {reason}"),
		_ => bail!("unexpected reply {:#04x} to the handshake", reply[0]),
	}
	conn.send(login_request(PROTOCOL, NAME)).await?;
	let (mut sink, mut stream) = conn.split();
	let (out, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
	let link = Arc::new(Link {
		state: Mutex::new(State::default()),
		out,
		tag,
		version,
		ended: Notify::new(),
	});
	tokio::spawn(async move {
		while let Some(data) = rx.recv().await {
			if sink.send(data).await.is_err() {
				break;
			}
		}
	});
	let reader = link.clone();
	tokio::spawn(async move {
		let reason = loop {
			match stream.next().await {
				None => break "the bridge closed the connection".to_owned(),
				Some(Err(e)) => break format!("reading a packet: {e}"),
				Some(Ok(data)) => {
					let mut st = reader.st();
					on_packet(&reader, &mut st, &data);
					if st.kicked.is_some() {
						break String::new();
					}
				}
			}
		};
		{
			let mut st = reader.st();
			if st.kicked.is_none() {
				st.closed = Some(reason);
			}
		}
		reader.ended.notify_one();
	});
	let ticker = link.clone();
	tokio::spawn(async move {
		let mut every = tokio::time::interval(Duration::from_millis(50));
		every.set_missed_tick_behavior(MissedTickBehavior::Delay);
		while !ticker.out.is_closed() {
			every.tick().await;
			let packet = tick(&ticker, &mut ticker.st());
			if let Some(p) = packet {
				ticker.send(p);
			}
		}
	});
	Ok(link)
}

struct Rcon {
	stream: TcpStream,
	id: i32,
}

impl Rcon {
	async fn connect(address: &str, password: &str) -> Result<Rcon> {
		let stream = TcpStream::connect(address)
			.await
			.with_context(|| format!("connecting to RCON at {address}"))?;
		let mut rcon = Rcon { stream, id: 0 };
		rcon.request(3, password).await?;
		Ok(rcon)
	}

	async fn request(&mut self, kind: i32, body: &str) -> Result<String> {
		self.id += 1;
		let mut packet = Vec::with_capacity(14 + body.len());
		packet.extend_from_slice(&(10 + body.len() as i32).to_le_bytes());
		packet.extend_from_slice(&self.id.to_le_bytes());
		packet.extend_from_slice(&kind.to_le_bytes());
		packet.extend_from_slice(body.as_bytes());
		packet.extend_from_slice(&[0, 0]);
		self.stream.write_all(&packet).await?;
		loop {
			let len = usize::try_from(self.stream.read_i32_le().await?)?;
			let mut rest = vec![0u8; len];
			self.stream.read_exact(&mut rest).await?;
			if len < 10 {
				bail!("RCON packet of {len} bytes is too short");
			}
			let id = i32::from_le_bytes(rest[0..4].try_into()?);
			if id == -1 {
				bail!("RCON login failed");
			}
			if id == self.id {
				return Ok(String::from_utf8_lossy(&rest[8..len - 2]).into_owned());
			}
		}
	}

	async fn command(&mut self, text: &str) -> Result<String> {
		tokio::time::timeout(Duration::from_secs(15), self.request(2, text))
			.await
			.with_context(|| format!("RCON command {text:?} timed out"))?
	}
}

fn numbers(text: &str) -> Option<[f64; 3]> {
	let found: Vec<f64> = text
		.split(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | 'E')))
		.filter_map(|s| s.parse().ok())
		.collect();
	match found[..] {
		[.., x, y, z] => Some([x, y, z]),
		_ => None,
	}
}

#[derive(Clone, Copy)]
struct Around {
	loaded: usize,
	solid: usize,
	opaque: usize,
	lit: usize,
}

impl Around {
	fn json(&self) -> Value {
		json!({ "loaded": self.loaded, "solid": self.solid, "opaque": self.opaque, "lit": self.lit })
	}
}

struct Bot {
	link: Arc<Link>,
	rcon: Rcon,
	flavor: Flavor,
	report: Map<String, Value>,
	phases: Map<String, Value>,
	markers: Map<String, Value>,
	wheres: Vec<Value>,
	problems: Vec<String>,
	column: (i32, i32),
	home: [i32; 3],
	surface: Option<f64>,
	only: Vec<String>,
}

impl Bot {
	fn log(&self, text: impl Display) {
		self.link.log(text);
	}

	fn fail(&mut self, text: String) {
		self.log(format!("PROBLEM {text}"));
		self.problems.push(text);
	}

	fn pos(&self) -> [f64; 3] {
		self.link.st().pos
	}

	fn set(&self, f: impl FnOnce(&mut State)) {
		f(&mut self.link.st());
	}

	fn teleports(&self) -> usize {
		self.link.st().teleports.len()
	}

	async fn cmd(&mut self, text: &str) -> Result<String> {
		self.rcon.command(text).await
	}

	async fn wait_until(&self, ms: u64, f: impl Fn(&State) -> bool) -> bool {
		let deadline = Instant::now() + Duration::from_millis(ms);
		loop {
			if f(&self.link.st()) {
				return true;
			}
			if Instant::now() >= deadline {
				return false;
			}
			sleep(Duration::from_millis(50)).await;
		}
	}

	async fn teleport(&mut self, x: f64, y: f64, z: f64) -> Result<bool> {
		let before = self.teleports();
		let out = self.cmd(&format!("tp {NAME} {x} {y} {z}")).await?;
		let arrived = self
			.wait_until(5000, |st| st.teleports.len() > before)
			.await;
		if !arrived {
			self.log(format!("no position packet after tp to {x} {y} {z}: {out}"));
		}
		Ok(arrived)
	}

	async fn settle(&self) {
		let mut last = self.pos();
		let mut still = 0;
		for _ in 0..200 {
			if still >= 1500 {
				break;
			}
			sleep(Duration::from_millis(100)).await;
			let now = self.pos();
			let moved = (0..3).any(|i| (now[i] - last[i]).abs() > 1e-3);
			still = if moved { 0 } else { still + 100 };
			last = now;
		}
	}

	async fn server_pos(&mut self) -> Result<[f64; 3]> {
		let text = if self.flavor.modern {
			self.cmd(&format!("data get entity {NAME} Pos")).await?
		} else {
			let before = self.teleports();
			let text = self.cmd(&format!("tp {NAME} ~ ~ ~")).await?;
			self.wait_until(3000, |st| st.teleports.len() > before)
				.await;
			sleep(Duration::from_millis(200)).await;
			self.set(|st| {
				if st.teleports.len() == before + 1 {
					st.teleports.pop();
					st.reads += 1;
				}
			});
			text
		};
		numbers(&text).ok_or_else(|| anyhow!("no position in {text:?}"))
	}

	async fn where_(&mut self, label: &str) -> Result<([f64; 3], i32)> {
		self.settle().await;
		let c = self.pos();
		let s = self.server_pos().await?;
		let after = self.pos();
		let offset = s[1] - c[1];
		let base = offset.round() as i32;
		let entry = json!({
			"label": label,
			"server": s.map(round2),
			"client": c.map(round2),
			"base": base,
		});
		self.log(format!("where {entry}"));
		self.wheres.push(entry.clone());
		if (s[0] - c[0]).abs() > 0.01 || (s[2] - c[2]).abs() > 0.01 {
			self.fail(format!("{label}: x/z differ {entry}"));
		}
		if (offset - f64::from(base)).abs() > 0.01 {
			self.fail(format!("{label}: y offset {offset} is not a whole number"));
		}
		if (0..3).any(|i| (after[i] - c[i]).abs() > 0.01) {
			self.fail(format!(
				"{label}: reading the server position moved the client from {:?} to {:?}",
				c.map(round2),
				after.map(round2)
			));
		}
		let inside = s[1] >= f64::from(self.flavor.bottom) && s[1] < f64::from(self.flavor.top);
		if inside && !(0.0..f64::from(HEIGHT)).contains(&c[1]) {
			self.fail(format!("{label}: client y {} outside 0..128", round2(c[1])));
		}
		self.check_light(label, s[1]);
		Ok((s, base))
	}

	fn around(&self, radius: i32) -> Around {
		let st = self.link.st();
		let [x, y, z] = st.pos.map(|v| v.floor() as i32);
		let mut out = Around {
			loaded: 0,
			solid: 0,
			opaque: 0,
			lit: 0,
		};
		for dx in -radius..=radius {
			for dy in -radius..=radius {
				for dz in -radius..=radius {
					let (bx, by, bz) = (x + dx, y + dy, z + dz);
					let Some(id) = st.block(bx, by, bz) else {
						continue;
					};
					out.loaded += 1;
					if solid(id) {
						out.solid += 1;
					}
					if opaque(id) {
						out.opaque += 1;
						if st.sky(bx, by, bz) == Some(15) {
							out.lit += 1;
						}
					}
				}
			}
		}
		out
	}

	fn check_light(&mut self, label: &str, server_y: f64) {
		let around = self.around(3);
		let head = {
			let st = self.link.st();
			let [x, y, z] = st.pos.map(|v| v.floor() as i32);
			st.sky(x, y + 1, z)
		};
		let mut entry = around.json();
		entry["head"] = json!(head);
		self.log(format!("{label} light {entry}"));
		let underground = self.surface.is_some_and(|s| server_y < s - 16.0);
		if underground && around.opaque > 20 && around.lit * 4 > around.opaque {
			self.fail(format!(
				"{label}: {} of {} opaque blocks around are sky lit underground",
				around.lit, around.opaque
			));
		}
	}

	async fn check_markers(&mut self, label: &str, base: i32) {
		sleep(Duration::from_millis(500)).await;
		let (cx, cz) = self.column;
		let (loaded, pos, got) = {
			let st = self.link.st();
			let got: Vec<(i32, &'static str, u8, i32, Option<u8>)> = self
				.flavor
				.markers
				.iter()
				.filter_map(|&(y, name, id)| {
					let cy = y - base;
					(0..HEIGHT)
						.contains(&cy)
						.then(|| (y, name, id, cy, st.block(cx, cy, cz)))
				})
				.collect();
			(st.chunks.contains_key(&(cx >> 4, cz >> 4)), st.pos, got)
		};
		if !loaded {
			let near = (f64::from(cx) - pos[0])
				.abs()
				.max((f64::from(cz) - pos[2]).abs())
				< 64.0;
			if near {
				self.fail(format!(
					"{label}: the marker column next to the bot is not loaded"
				));
			}
			self.log(format!("{label} marker column not loaded"));
			self.markers
				.insert(label.to_owned(), json!({ "loaded": false }));
			return;
		}
		let mut seen = 0;
		for &(y, name, id, cy, block) in &got {
			if block == Some(id) {
				seen += 1;
			} else {
				self.fail(format!(
					"{label}: marker {name} ({id}) at server y {y} shows {} at client y {cy}",
					block.map_or("nothing".to_owned(), |b| b.to_string())
				));
			}
		}
		self.log(format!("{label} markers seen {seen} of {}", got.len()));
		self.markers.insert(
			label.to_owned(),
			json!({ "seen": seen, "inWindow": got.len(), "base": base }),
		);
	}

	async fn place_markers(&mut self, x: i32, z: i32) -> Result<()> {
		self.column = (x, z);
		let markers = self.flavor.markers.clone();
		for &(y, _, _) in &markers {
			self.cmd(&format!("setblock {x} {y} {z} minecraft:glass"))
				.await?;
		}
		sleep(Duration::from_millis(300)).await;
		for &(y, name, _) in &markers {
			let out = self
				.cmd(&format!("setblock {x} {y} {z} minecraft:{name}"))
				.await?;
			if !(out.starts_with("Changed") || out.contains("placed") || name == "glass") {
				self.log(format!("setblock {y} {name}: {out}"));
			}
		}
		Ok(())
	}

	async fn platform(&mut self, x: i32, z: i32) -> Result<()> {
		let d = self.flavor.deep;
		let mode = if self.flavor.modern {
			"hollow"
		} else {
			"0 hollow"
		};
		let out = self
			.cmd(&format!(
				"fill {} {} {} {} {} {} minecraft:stone {mode}",
				x - 2,
				d - 1,
				z - 2,
				x + 2,
				d + 5,
				z + 2
			))
			.await?;
		self.log(format!("deep box: {out}"));
		Ok(())
	}

	async fn setup(&mut self) -> Result<()> {
		if !self.wait_until(30_000, |st| st.spawned).await {
			bail!("no position packet after joining");
		}
		self.cmd(&format!("op {NAME}")).await?;
		let out = self.cmd(&format!("gamemode creative {NAME}")).await?;
		self.log(format!("gamemode: {out}"));
		sleep(Duration::from_secs(2)).await;
		let mut landed = false;
		for attempt in 0..6 {
			let spread = if attempt == 0 { 96 } else { 400 };
			let shift = || (rand::random::<u32>() % spread) as i32 - spread as i32 / 2;
			let (hx, hz) = (HOME + shift(), HOME + shift());
			self.set(|st| {
				st.flying = true;
				st.fly = 0.0;
			});
			self.teleport(f64::from(hx) + 0.5, 120.0, f64::from(hz) + 0.5)
				.await?;
			sleep(Duration::from_secs(3)).await;
			let before = self.teleports();
			let surface = if self.flavor.modern {
				self.cmd(&format!(
					"execute positioned {hx}.5 0 {hz}.5 positioned over motion_blocking_no_leaves run tp {NAME} ~ ~ ~"
				))
				.await?
			} else {
				self.cmd(&format!("spreadplayers {hx} {hz} 0 1 false {NAME}"))
					.await?
			};
			self.log(format!("surface {surface}"));
			self.wait_until(5000, |st| st.teleports.len() > before)
				.await;
			sleep(Duration::from_secs(1)).await;
			let arrived = self.pos()[1];
			self.set(|st| st.flying = false);
			sleep(Duration::from_secs(2)).await;
			let (below, feet, ground, y) = {
				let st = self.link.st();
				let [x, y, z] = st.pos.map(|v| v.floor() as i32);
				let below = st.block(x, (st.pos[1] - 0.5).floor() as i32, z);
				(below, st.block(x, y, z), st.ground, st.pos[1])
			};
			let dry = ground
				&& below.is_some_and(|b| solid(b) && !liquid(b))
				&& feet.is_some_and(|b| !liquid(b))
				&& (arrived - y).abs() < 1.5;
			if dry {
				landed = true;
				break;
			}
			self.log(format!(
				"home {hx},{hz} is not dry land (below {below:?}, feet {feet:?}, fell from {} to {}), trying another",
				round2(arrived),
				round2(y)
			));
		}
		if !landed {
			self.fail("setup: found no dry land for the work area".to_owned());
		}
		sleep(Duration::from_secs(2)).await;
		let (s, base) = self.where_("spawn").await?;
		self.surface = Some(s[1]);
		self.home = s.map(|v| v.floor() as i32);
		let [sx, _, sz] = self.home;
		self.place_markers(sx + 3, sz + 3).await?;
		self.check_markers("spawn", base).await;
		self.report
			.insert("spawnAround".into(), self.around(2).json());
		Ok(())
	}

	async fn dig(&mut self) -> Result<()> {
		self.set(|st| {
			st.flying = false;
			st.fly = 0.0;
		});
		self.where_("dig start").await?;
		let goal = f64::from(self.flavor.bottom + 5);
		let (mut digs, mut forced, mut repeats) = (0, 0, 0);
		let mut last = None;
		let mut stop = "iteration limit";
		for i in 0..1500 {
			let (pos, ground) = {
				let st = self.link.st();
				(st.pos, st.ground)
			};
			if !ground {
				sleep(Duration::from_millis(50)).await;
				continue;
			}
			if self.flavor.modern && i % 25 == 0 {
				let s = self.server_pos().await?;
				if s[1] <= goal {
					stop = "goal";
					break;
				}
			}
			let (bx, by, bz) = (
				pos[0].floor() as i32,
				(pos[1] - 1e-3).floor() as i32,
				pos[2].floor() as i32,
			);
			let below = self.link.st().block(bx, by, bz);
			let Some(id) = below else {
				sleep(Duration::from_millis(200)).await;
				continue;
			};
			if id == 7 {
				stop = "bedrock";
				break;
			}
			if !solid(id) {
				sleep(Duration::from_millis(50)).await;
				continue;
			}
			if last == Some((bx, by, bz)) {
				repeats += 1;
			} else {
				repeats = 0;
				last = Some((bx, by, bz));
			}
			if repeats >= 4 {
				let command = if self.flavor.modern {
					format!("execute at {NAME} run setblock ~ ~-1 ~ minecraft:air")
				} else {
					format!("execute {NAME} ~ ~ ~ setblock ~ ~-1 ~ minecraft:air")
				};
				let out = self.cmd(&command).await?;
				forced += 1;
				if forced == 1 {
					self.log(format!(
						"dig falls back to RCON at client y {by} (block {id}): {out}"
					));
				}
			} else {
				self.link
					.send(S::BlockDig(v1_0_1::play_to_server_packet_block_dig::Ty {
						status: 0,
						location: v1_0_1::position_ibi::Ty {
							x: bx,
							y: by as u8,
							z: bz,
						},
						face: 1,
					}));
				self.link.st().set_block(bx, by, bz, 0);
				digs += 1;
			}
			sleep(Duration::from_millis(150)).await;
		}
		self.log(format!(
			"dig stopped ({stop}) after {digs} dig packets and {forced} RCON removals"
		));
		self.report.insert(
			"dig".into(),
			json!({ "digPackets": digs, "rconRemovals": forced, "stop": stop }),
		);
		let (s, base) = self.where_("dig end").await?;
		if s[1] > f64::from(self.flavor.bottom + 16) {
			self.fail(format!("dig: only reached server y {}", s[1]));
		}
		self.check_markers("dig", base).await;
		let around = self.around(2);
		if around.solid < 20 {
			self.fail(format!(
				"dig: only {} solid blocks around the bot",
				around.solid
			));
		}
		self.report.insert("digAround".into(), around.json());
		Ok(())
	}

	async fn fly_by(&mut self, dy: f64) {
		self.set(|st| {
			st.flying = true;
			st.fly = dy;
		});
		let ms = (dy.abs() / 0.5 * 50.0) as u64 * 2 + 10_000;
		if !self.wait_until(ms, |st| st.fly == 0.0).await {
			self.log("flying did not finish in time");
		}
		sleep(Duration::from_millis(500)).await;
	}

	async fn fly(&mut self) -> Result<()> {
		let [fx, fy, fz] = self.server_pos().await?.map(|v| v.floor() as i32);
		if fy + 2 < self.flavor.top {
			self.cmd(&format!(
				"fill {fx} {} {fz} {fx} {} {fz} minecraft:air",
				fy + 2,
				self.flavor.top - 1
			))
			.await?;
		}
		self.set(|st| {
			st.flying = true;
			st.fly = 0.0;
		});
		self.teleport(f64::from(fx) + 0.5, f64::from(fy), f64::from(fz) + 0.5)
			.await?;
		sleep(Duration::from_secs(1)).await;
		let start = self.server_pos().await?[1];
		self.fly_by(self.flavor.fly_to - start).await;
		let (s, base) = self.where_("fly").await?;
		if s[1] < self.flavor.fly_to - 10.0 {
			self.fail(format!("fly: only reached server y {}", s[1]));
		}
		self.check_markers("fly", base).await;
		Ok(())
	}

	async fn fall(&mut self) -> Result<()> {
		self.set(|st| {
			st.flying = false;
			st.fly = 0.0;
			st.vy = 0.0;
			st.ground = false;
		});
		sleep(Duration::from_millis(200)).await;
		if !self.wait_until(60_000, |st| st.ground).await {
			self.fail("fall: never landed".to_owned());
		}
		let (_, base) = self.where_("fall").await?;
		self.check_markers("fall", base).await;
		let below = {
			let st = self.link.st();
			let [x, y, z] = st.pos;
			st.block(x.floor() as i32, (y - 0.5).floor() as i32, z.floor() as i32)
		};
		if !below.is_some_and(solid) {
			self.fail(format!("fall: standing on {below:?}"));
		}
		Ok(())
	}

	async fn tp(&mut self) -> Result<()> {
		let [x, _, z] = self.home;
		let (cx, cz) = (f64::from(x) + 0.5, f64::from(z) + 0.5);
		self.platform(x, z).await?;
		self.set(|st| {
			st.flying = false;
			st.fly = 0.0;
		});
		self.teleport(cx, f64::from(self.flavor.deep), cz).await?;
		let (_, base) = self.where_("tp deep").await?;
		self.check_markers("tp deep", base).await;
		self.set(|st| st.flying = true);
		self.teleport(cx, f64::from(self.flavor.high), cz).await?;
		let (_, base) = self.where_("tp high").await?;
		self.check_markers("tp high", base).await;
		self.teleport(cx, f64::from(self.flavor.middle), cz).await?;
		let (_, base) = self.where_("tp middle").await?;
		self.check_markers("tp middle", base).await;
		Ok(())
	}

	async fn die(&mut self, label: &str) -> Result<bool> {
		self.cmd(&format!("gamemode survival {NAME}")).await?;
		let before = self.link.st().respawns;
		let out = self.cmd(&format!("kill {NAME}")).await?;
		self.log(format!("kill: {out}"));
		if !self.wait_until(15_000, |st| st.respawns > before).await {
			self.fail(format!("{label}: no respawn packet after the kill"));
			return Ok(false);
		}
		if !self.wait_until(15_000, |st| st.spawned).await {
			self.fail(format!("{label}: no position packet after the respawn"));
			return Ok(false);
		}
		sleep(Duration::from_secs(3)).await;
		Ok(true)
	}

	async fn respawn(&mut self) -> Result<()> {
		let [x, _, z] = self.home;
		let (cx, cz) = (f64::from(x) + 0.5, f64::from(z) + 0.5);
		self.platform(x, z).await?;
		self.set(|st| {
			st.flying = false;
			st.fly = 0.0;
		});
		let [hx, hy, hz] = self.home;
		self.cmd(&format!(
			"fill {} {} {} {} {} {} minecraft:stone",
			hx - 3,
			hy - 1,
			hz - 3,
			hx - 1,
			hy - 1,
			hz - 1
		))
		.await?;
		self.cmd(&format!(
			"fill {} {hy} {} {} {} {} minecraft:air",
			hx - 3,
			hz - 3,
			hx - 1,
			hy + 2,
			hz - 1
		))
		.await?;
		let out = self
			.cmd(&format!("spawnpoint {NAME} {} {hy} {}", hx - 2, hz - 2))
			.await?;
		self.log(format!("spawnpoint: {out}"));
		self.teleport(cx, f64::from(self.flavor.deep), cz).await?;
		self.where_("before death").await?;
		if self.die("respawn").await? {
			let (_, base) = self.where_("respawn").await?;
			self.check_markers("respawn", base).await;
			self.report
				.insert("respawnAround".into(), self.around(2).json());
		}
		let deep = self.flavor.deep;
		let out = self
			.cmd(&format!("spawnpoint {NAME} {x} {deep} {z}"))
			.await?;
		self.log(format!("spawnpoint: {out}"));
		if self.die("respawn deep").await? {
			let (_, base) = self.where_("respawn deep").await?;
			self.check_markers("respawn deep", base).await;
			self.report
				.insert("respawnDeepAround".into(), self.around(2).json());
		}
		self.cmd(&format!("gamemode creative {NAME}")).await?;
		Ok(())
	}

	async fn run(&mut self) -> Result<()> {
		self.setup().await?;
		self.report.insert("spawn".into(), json!(true));
		for label in ["dig", "fly", "fall", "tp", "respawn"] {
			if !self.only.is_empty() && !self.only.iter().any(|o| o == label) {
				continue;
			}
			self.log(format!("phase {label}"));
			let before = self.teleports();
			let result = match label {
				"dig" => self.dig().await,
				"fly" => self.fly().await,
				"fall" => self.fall().await,
				"tp" => self.tp().await,
				_ => self.respawn().await,
			};
			if let Err(e) = result {
				self.fail(format!("{label}: {e:#}"));
			}
			let teleports: Vec<f64> = self.link.st().teleports[before..]
				.iter()
				.map(|y| round2(*y))
				.collect();
			self.phases
				.insert(label.to_owned(), json!({ "teleports": teleports }));
		}
		Ok(())
	}

	async fn finish(&mut self) -> i32 {
		let (issues, packets) = {
			let st = self.link.st();
			self.report.insert(
				"respawns".into(),
				json!({ "requested": st.respawn_requests, "received": st.respawns }),
			);
			(st.issues.clone(), st.packets.clone())
		};
		for (kind, count) in issues {
			self.problems.push(format!("{kind} (x{count})"));
		}
		self.report
			.insert("phases".into(), Value::Object(self.phases.clone()));
		self.report
			.insert("markers".into(), Value::Object(self.markers.clone()));
		self.report.insert("where".into(), json!(self.wheres));
		self.report.insert("packets".into(), json!(packets));
		self.report.insert("problems".into(), json!(self.problems));
		println!("{}", Value::Object(self.report.clone()));
		let _ = self.cmd(&format!("gamemode creative {NAME}")).await;
		self.link.send(S::KickDisconnect("Quitting".to_owned()));
		sleep(Duration::from_millis(300)).await;
		i32::from(!self.problems.is_empty() || !self.report.contains_key("done"))
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	let args: Vec<String> = std::env::args().skip(1).collect();
	let arg = |i: usize, default: &str| args.get(i).cloned().unwrap_or_else(|| default.to_owned());
	let address = arg(0, "127.0.0.1:25634");
	let version = arg(1, "26.1");
	let rcon_address = arg(2, "127.0.0.1:25632");
	let password = arg(3, "tall");
	let only = arg(4, "");
	let tag = format!("[1.0.1 on {version}]");
	let rcon = Rcon::connect(&rcon_address, &password).await?;
	let link = connect(&address, tag).await?;
	let mut report = Map::new();
	report.insert("version".into(), json!(version));
	report.insert("client".into(), json!("1.0.1"));
	report.insert("name".into(), json!(NAME));
	let mut bot = Bot {
		link: link.clone(),
		rcon,
		flavor: Flavor::new(&version),
		report,
		phases: Map::new(),
		markers: Map::new(),
		wheres: Vec::new(),
		problems: Vec::new(),
		column: (HOME, HOME),
		home: [HOME, 64, HOME],
		surface: None,
		only: only
			.split(',')
			.filter(|s| !s.is_empty())
			.map(str::to_owned)
			.collect(),
	};
	let outcome: std::result::Result<(), String> = tokio::select! {
		r = bot.run() => r.map_err(|e| format!("{e:#}")),
		_ = link.ended.notified() => Err("disconnected".to_owned()),
		_ = sleep(Duration::from_secs(600)) => Err("timeout".to_owned()),
	};
	match outcome {
		Ok(()) => {
			bot.report.insert("done".into(), json!(true));
		}
		Err(e) if e == "timeout" => {
			bot.report.insert("timeout".into(), json!(true));
		}
		Err(e) if e == "disconnected" => {}
		Err(e) => {
			bot.report.insert("error".into(), json!(e));
		}
	}
	let (kicked, closed) = {
		let st = link.st();
		(st.kicked.clone(), st.closed.clone())
	};
	if let Some(reason) = kicked {
		bot.report.insert("kicked".into(), json!(reason));
	} else if let Some(reason) = closed {
		bot.report.insert("error".into(), json!(reason));
	}
	let code = bot.finish().await;
	std::process::exit(code);
}
