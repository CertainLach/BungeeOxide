use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use minecraft_versions::custom::chunks::{
	decode, encode_chunk, Biomes, BlockEntity, Column, Light, Nibbles, Packet, Section,
};
use minecraft_versions::gen::convert::{convert, index_of, step, Root, VERSIONS};
use minecraft_versions::Session;

const ROOT: Root = Root::PlayToClient;

fn at(version: &str) -> usize {
	index_of(version).unwrap()
}

fn dump() -> Vec<Vec<u8>> {
	let bytes = std::fs::read(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/tests/data/1.8-toClient.bin"
	))
	.unwrap();
	let mut out = Vec::new();
	let mut i = 0;
	while i + 4 <= bytes.len() {
		let len = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
		out.push(bytes[i + 4..i + 4 + len].to_vec());
		i += 4 + len;
	}
	out
}

fn chain(from: usize, to: usize, data: &[u8], cx: &mut Session) -> Result<Vec<Vec<u8>>, String> {
	let mut current = vec![data.to_vec()];
	let mut v = from;
	while v != to {
		let next = if to > v { v + 1 } else { v - 1 };
		let mut out = Vec::new();
		for p in &current {
			out.extend(step(ROOT, v, next, p, cx).map_err(|e| e.to_string())?);
		}
		current = out;
		v = next;
	}
	Ok(current)
}

fn traffic(version: usize) -> Vec<Vec<u8>> {
	static CACHE: OnceLock<HashMap<usize, Vec<Vec<u8>>>> = OnceLock::new();
	let cache = CACHE.get_or_init(|| {
		let mut stages = HashMap::from([(at("1.8"), dump())]);
		for to in [VERSIONS.len() - 1, at("1.7")] {
			let mut cx = Session::new();
			let mut v = at("1.8");
			while v != to {
				let next = if to > v { v + 1 } else { v - 1 };
				let out = stages[&v]
					.iter()
					.flat_map(|p| step(ROOT, v, next, p, &mut cx).unwrap_or_default())
					.collect();
				stages.insert(next, out);
				v = next;
			}
		}
		stages
	});
	cache[&version].clone()
}

#[derive(Debug, PartialEq)]
enum Norm {
	Chunk(Box<Column>),
	Bulk(Vec<Column>, bool),
	Light(i32, i32, Light),
	Unload(i32, i32),
	Other(Vec<u8>),
}

fn normal_column(mut c: Column) -> Column {
	for s in c.sections.iter_mut() {
		match s {
			Some(sec) => sec.count = None,
			None if c.full => *s = Some(Section::filled(0, None)),
			None => {}
		}
	}
	while c.full
		&& c.sections
			.last()
			.is_some_and(|s| s.as_ref().is_some_and(|s| s.states.iter().all(|v| *v == 0)))
	{
		c.sections.pop();
	}
	c.at = 0;
	c.space = 0;
	c.biome_bits = None;
	c
}

fn normalize(version: usize, data: &[u8]) -> Norm {
	match decode(version, data) {
		Ok(Some(Packet::Chunk(c))) => Norm::Chunk(Box::new(normal_column(c))),
		Ok(Some(Packet::Bulk(c, sky))) => {
			Norm::Bulk(c.into_iter().map(normal_column).collect(), sky)
		}
		Ok(Some(Packet::Light(x, z, l))) => Norm::Light(x, z, l),
		Ok(Some(Packet::Unload(x, z))) => Norm::Unload(x, z),
		Ok(None) => Norm::Other(data.to_vec()),
		Err(e) => Norm::Other(
			format!("undecodable {} packet ({e}): {data:?}", VERSIONS[version]).into_bytes(),
		),
	}
}

fn describe(a: &Norm, b: &Norm) -> String {
	match (a, b) {
		(Norm::Chunk(a), Norm::Chunk(b)) => {
			let mut why = Vec::new();
			if (a.x, a.z, a.full) != (b.x, b.z, b.full) {
				why.push(format!(
					"position {:?} vs {:?}",
					(a.x, a.z, a.full),
					(b.x, b.z, b.full)
				));
			}
			if a.sections != b.sections {
				let n = a.sections.len().max(b.sections.len());
				for i in 0..n {
					let (x, y) = (
						a.sections.get(i).cloned().flatten(),
						b.sections.get(i).cloned().flatten(),
					);
					if x != y {
						let diff = match (&x, &y) {
							(Some(x), Some(y)) => {
								let k = (0..4096).find(|k| x.states[*k] != y.states[*k]).unwrap();
								format!("block {k}: {} vs {}", x.states[k], y.states[k])
							}
							_ => format!("presence {} vs {}", x.is_some(), y.is_some()),
						};
						why.push(format!("section {i} differs ({diff})"));
						break;
					}
				}
			}
			if a.light != b.light {
				why.push(format!(
					"light differs: {:?} vs {:?}",
					a.light.as_ref().map(summary),
					b.light.as_ref().map(summary)
				));
			}
			if a.biomes != b.biomes {
				why.push("biomes differ".into());
			}
			if a.heightmaps != b.heightmaps {
				why.push("heightmaps differ".into());
			}
			if a.entities != b.entities {
				why.push(format!(
					"block entities differ: {:?} vs {:?}",
					a.entities, b.entities
				));
			}
			why.join("; ")
		}
		_ => format!("{:?} vs {:?}", kind(a), kind(b)),
	}
}

fn summary(l: &Light) -> String {
	let s = |v: &[Nibbles]| {
		v.iter()
			.map(|n| match n {
				Nibbles::Missing => '.',
				Nibbles::Empty => '0',
				Nibbles::Data(_) => '#',
			})
			.collect::<String>()
	};
	format!(
		"sky {} block {} has_sky {} trust {}",
		s(&l.sky),
		s(&l.block),
		l.has_sky,
		l.trust
	)
}

fn kind(n: &Norm) -> String {
	match n {
		Norm::Chunk(c) => format!("chunk {} {}", c.x, c.z),
		Norm::Bulk(c, _) => format!("bulk of {}", c.len()),
		Norm::Light(x, z, _) => format!("light {x} {z}"),
		Norm::Unload(x, z) => format!("unload {x} {z}"),
		Norm::Other(b) => format!(
			"packet {:#04x} of {} bytes",
			b.first().copied().unwrap_or(0),
			b.len()
		),
	}
}

#[derive(Default, Debug)]
struct Report {
	packets: usize,
	chunks: usize,
	compared: usize,
	only_virtual: usize,
	mismatches: Vec<String>,
}

fn compare(from: usize, to: usize, packets: &[Vec<u8>]) -> Report {
	compare_in(from, to, packets, None)
}

fn compare_in(
	from: usize,
	to: usize,
	packets: &[Vec<u8>],
	versions: Option<(usize, usize)>,
) -> Report {
	let (mut a, mut b) = (Session::new(), Session::new());
	if let Some((server, client)) = versions {
		a.set_versions(server, client);
		b.set_versions(server, client);
	}
	let mut report = Report::default();
	for (i, p) in packets.iter().enumerate() {
		report.packets += 1;
		let expected = chain(from, to, p, &mut a);
		let got = convert(ROOT, from, to, p, &mut b).map_err(|e| e.to_string());
		match decode(from, p) {
			Ok(Some(_)) => report.chunks += 1,
			Ok(None) => {}
			Err(e) => report.mismatches.push(format!(
				"#{i} {}: source chunk does not decode: {e}",
				VERSIONS[from]
			)),
		}
		match (expected, got) {
			(Ok(e), Ok(g)) => {
				report.compared += 1;
				let (e, g): (Vec<Norm>, Vec<Norm>) = (
					e.iter().map(|x| normalize(to, x)).collect(),
					g.iter().map(|x| normalize(to, x)).collect(),
				);
				if e.len() != g.len() {
					report.mismatches.push(format!(
						"#{i} {} -> {}: {} packets vs {}: {:?} vs {:?}",
						VERSIONS[from],
						VERSIONS[to],
						e.len(),
						g.len(),
						e.iter().map(kind).collect::<Vec<_>>(),
						g.iter().map(kind).collect::<Vec<_>>()
					));
					continue;
				}
				for (x, y) in e.iter().zip(&g) {
					if x != y {
						report.mismatches.push(format!(
							"#{i} {} -> {}: {}",
							VERSIONS[from],
							VERSIONS[to],
							describe(x, y)
						));
					}
				}
			}
			(Err(_), Ok(_)) => report.only_virtual += 1,
			(Ok(_), Err(e)) => report.mismatches.push(format!(
				"#{i} {} -> {}: virtual path failed: {e}",
				VERSIONS[from], VERSIONS[to]
			)),
			(Err(_), Err(_)) => {}
		}
	}
	report
}

fn assert_equivalent(from: &str, to: &str, packets: &[Vec<u8>]) {
	let r = compare(at(from), at(to), packets);
	assert!(
		r.mismatches.is_empty(),
		"{from} -> {to}: {} mismatches:\n{}",
		r.mismatches.len(),
		r.mismatches
			.iter()
			.take(8)
			.cloned()
			.collect::<Vec<_>>()
			.join("\n")
	);
}

fn netty() -> Vec<usize> {
	(at("1.7")..VERSIONS.len()).collect()
}

#[test]
fn chunk_packets_are_recognized_in_every_version() {
	for v in netty() {
		let ids = minecraft_versions::custom::chunks::ids(v);
		assert!(ids.chunk.is_some(), "{} map_chunk", VERSIONS[v]);
		assert_eq!(
			ids.bulk.is_some(),
			v <= at("1.8"),
			"{} map_chunk_bulk",
			VERSIONS[v]
		);
		assert_eq!(
			ids.light.is_some(),
			v >= at("1.14"),
			"{} update_light",
			VERSIONS[v]
		);
		assert_eq!(
			ids.unload.is_some(),
			v >= at("1.9"),
			"{} unload_chunk",
			VERSIONS[v]
		);
	}
}

#[test]
fn probes_cover_every_native_step() {
	use minecraft_versions::custom::chunks::{probe, probe_column, step_table};
	for a in netty() {
		for b in [a.wrapping_sub(1), a + 1] {
			if b < at("1.7") || b >= VERSIONS.len() {
				continue;
			}
			let sections = probe_column(a).sections.len();
			if (a, b) != (at("1.9"), at("1.8")) {
				let probed = probe(a, b)
					.unwrap_or_else(|| panic!("{} -> {} probe", VERSIONS[a], VERSIONS[b]));
				assert_eq!(
					probed.len(),
					sections * 4096,
					"{} -> {}",
					VERSIONS[a],
					VERSIONS[b]
				);
			}
			let table = step_table(a, b);
			assert_eq!(
				table.len(),
				sections * 4096,
				"{} -> {}",
				VERSIONS[a],
				VERSIONS[b]
			);
			assert_eq!(table[0], 0, "{} -> {} keeps air", VERSIONS[a], VERSIONS[b]);
		}
	}
}

#[test]
fn recorded_traffic_up_from_1_8() {
	let packets = traffic(at("1.8"));
	for to in ["1.7", "26.1"] {
		assert_equivalent("1.8", to, &packets);
	}
}

#[test]
fn recorded_traffic_down_from_26_1() {
	assert_equivalent("26.1", "1.9", &traffic(at("26.1")));
}

#[test]
fn recorded_traffic_reaches_1_8() {
	let packets = traffic(at("26.1"));
	let r = compare(at("26.1"), at("1.8"), &packets);
	assert!(r.mismatches.is_empty(), "{:?}", r.mismatches.first());
	let mut cx = Session::new();
	let mut columns = 0;
	for p in &packets {
		for out in convert(ROOT, at("26.1"), at("1.8"), p, &mut cx).unwrap_or_default() {
			if let Ok(Some(Packet::Chunk(c))) = decode(at("1.8"), &out) {
				columns += 1;
				assert_eq!(
					(c.block(0, 0, 0), c.block(0, 3, 0), c.block(0, 4, 0)),
					(7 << 4, 2 << 4, 0)
				);
			}
		}
	}
	assert_eq!(columns, 81);
}

#[test]
fn chunks_reach_pre_netty_clients() {
	assert_equivalent("26.1", "1.0.1", &traffic(at("26.1")));
	assert_equivalent("1.8", "1.2.5", &traffic(at("1.8")));
	for (from, sections) in [("1.12.2", 16), ("26.1", 24)] {
		assert_equivalent(from, "1.0.1", &synthetic_packets(at(from), sections, true));
	}
}

#[test]
#[ignore]
fn recorded_traffic_every_pair() {
	every_pair(traffic);
}

#[test]
#[ignore]
fn synthetic_every_pair() {
	every_pair(|v| synthetic_packets(v, if v >= at("1.18") { 24 } else { 16 }, true));
}

fn every_pair(packets: impl Fn(usize) -> Vec<Vec<u8>> + Sync) {
	let pairs: Vec<(usize, usize)> = netty()
		.into_iter()
		.flat_map(|a| {
			netty()
				.into_iter()
				.filter(move |b| a != *b)
				.map(move |b| (a, b))
		})
		.collect();
	let next = std::sync::atomic::AtomicUsize::new(0);
	let failures = Mutex::new(Vec::new());
	let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
	std::thread::scope(|s| {
		for _ in 0..threads {
			s.spawn(|| loop {
				let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
				let Some(&(from, to)) = pairs.get(i) else {
					break;
				};
				let r = compare(from, to, &packets(from));
				if !r.mismatches.is_empty() {
					failures.lock().unwrap().push(format!(
						"{} -> {}: {} mismatches, first: {}",
						VERSIONS[from],
						VERSIONS[to],
						r.mismatches.len(),
						r.mismatches[0]
					));
				}
			});
		}
	});
	let failures = failures.into_inner().unwrap();
	assert!(
		failures.is_empty(),
		"{} pairs differ:\n{}",
		failures.len(),
		failures.join("\n")
	);
}

fn noise(seed: u64) -> impl FnMut() -> u64 {
	let mut s = seed;
	move || {
		s = s.wrapping_add(0x9e37_79b9_7f4a_7c15);
		let mut z = s;
		z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
		z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
		z ^ (z >> 31)
	}
}

fn states_of(version: usize) -> u32 {
	minecraft_versions::registry::blocks(VERSIONS[version]).map_or(4096, |b| b.states().max(1))
}

fn synthetic(version: usize, x: i32, z: i32, sections: usize, seed: u64) -> Column {
	let mut rng = noise(seed);
	let legacy = version < at("1.13");
	let pick = |r: u64| -> u32 {
		match legacy {
			true => (((r % 160) as u32 + 1) << 4) | ((r >> 32) as u32 % 16),
			false => (r % states_of(version) as u64) as u32,
		}
	};
	let mut col = Column::new(x, z, true, version);
	for s in 0..sections {
		let kinds = match s % 4 {
			0 => 1,
			1 => 3,
			2 => 40,
			_ => 400,
		};
		let palette: Vec<u32> = (0..kinds).map(|_| pick(rng())).collect();
		let states = (0..4096)
			.map(|_| palette[(rng() % kinds as u64) as usize])
			.collect();
		col.sections.push(if s == 5 {
			None
		} else {
			Some(Section::new(states, None))
		});
	}
	if version >= at("1.18") {
		for s in col.sections.iter_mut() {
			s.get_or_insert_with(|| Section::filled(0, Some(0)));
		}
	}
	let mut light = Light::new(sections, true);
	for (i, n) in light.sky.iter_mut().enumerate() {
		*n = match i % 3 {
			0 => Nibbles::Data((0..2048).map(|_| rng() as u8).collect()),
			1 => Nibbles::Empty,
			_ => Nibbles::Data(vec![0xff; 2048]),
		};
	}
	for (i, n) in light.block.iter_mut().enumerate() {
		*n = match i % 4 {
			0 => Nibbles::Missing,
			1 => Nibbles::Empty,
			_ => Nibbles::Data((0..2048).map(|_| rng() as u8 & 0x77).collect()),
		};
	}
	if version < at("1.14") {
		for (i, s) in col.sections.iter().enumerate() {
			if s.is_some() {
				light.block[i + 1] = Nibbles::Data((0..2048).map(|_| rng() as u8).collect());
				light.sky[i + 1] = Nibbles::Data((0..2048).map(|_| rng() as u8).collect());
			}
		}
	}
	col.light = Some(light);
	col.biomes = match version {
		v if v < at("1.15") => Biomes::Flat((0..256).map(|_| (rng() % 20) as i32).collect()),
		v if v < at("1.18") => Biomes::Cells(
			(0..sections.max(16) * 64)
				.map(|_| (rng() % 20) as i32)
				.collect(),
		),
		_ => Biomes::Cells(
			(0..sections * 64)
				.map(|i| (i as i32 / 70) % 5 + (rng() % 2) as i32)
				.collect(),
		),
	};
	if version >= at("1.14") {
		col.heightmaps = Some(vec![]);
	}
	col
}

fn tag(entries: Vec<nbt::Entry>) -> nbt::Tag {
	nbt::Tag::new(entries)
}

fn entry(name: &str, value: nbt::Payload) -> nbt::Entry {
	nbt::Entry::new(name, value)
}

fn string(s: &str) -> nbt::Payload {
	nbt::Payload::string(s.to_owned())
}

fn compound(entries: Vec<nbt::Entry>) -> nbt::Payload {
	nbt::Payload::compound(entries)
}

fn rich_fields() -> Vec<nbt::Entry> {
	let text = |s: &str| string(&format!("{{\"text\":\"{s}\"}}"));
	let lines = |s: &str| {
		nbt::Payload::list(nbt::List::new(nbt::ListItems::string(vec![
			format!("\"{s}\""),
			"\"\"".into(),
			"\"\"".into(),
			"\"\"".into(),
		])))
	};
	let pattern = vec![
		entry("Color", nbt::Payload::int(3)),
		entry("Pattern", string("bs")),
	];
	vec![
		entry("Text1", text("one")),
		entry("Text2", text("two")),
		entry("Text3", text("")),
		entry("Text4", text("")),
		entry(
			"front_text",
			compound(vec![
				entry("messages", lines("front")),
				entry("color", string("black")),
			]),
		),
		entry("CustomName", string("Chest of things")),
		entry("Base", nbt::Payload::int(4)),
		entry(
			"Patterns",
			nbt::Payload::list(nbt::List::new(nbt::ListItems::compound(vec![pattern]))),
		),
		entry(
			"Owner",
			compound(vec![
				entry("Id", nbt::Payload::intArray(vec![1, 2, 3, 4])),
				entry("Name", string("Notch")),
			]),
		),
		entry(
			"SkullOwner",
			compound(vec![
				entry("Id", string("069a79f4-44e9-4726-a5be-fca90e38aaf5")),
				entry("Name", string("Notch")),
			]),
		),
		entry(
			"SpawnData",
			compound(vec![entry("id", string("minecraft:zombie_pigman"))]),
		),
		entry(
			"target_uuid",
			string("069a79f4-44e9-4726-a5be-fca90e38aaf5"),
		),
		entry("Item", string("minecraft:red_flower")),
		entry("Data", nbt::Payload::int(2)),
		entry("color", nbt::Payload::int(14)),
		entry("SkullType", nbt::Payload::int(3)),
		entry("Rot", nbt::Payload::int(5)),
	]
}

const NAMED: &[&str] = &[
	"minecraft:sign",
	"minecraft:skull",
	"minecraft:mob_spawner",
	"minecraft:banner",
	"minecraft:conduit",
	"minecraft:chest",
	"minecraft:bed",
	"minecraft:flower_pot",
	"minecraft:noteblock",
	"Chest",
	"minecraft:beehive",
];
const TYPED: &[i32] = &[0, 1, 7, 8, 9, 14, 15, 18, 19, 23, 24, 33, 38, 40, 44];

fn block_entities(version: usize, cx: i32, cz: i32) -> Vec<BlockEntity> {
	let at_index = |i: usize| {
		(
			cx * 16 + (i as i32 * 5) % 16,
			3 + i as i32 * 11,
			cz * 16 + (i as i32 * 3) % 16,
		)
	};
	match version {
		v if v < at("1.18") => NAMED
			.iter()
			.enumerate()
			.map(|(i, id)| {
				let (x, y, z) = at_index(i);
				let mut fields = rich_fields();
				fields.extend([
					entry("id", string(id)),
					entry("x", nbt::Payload::int(x)),
					entry("y", nbt::Payload::int(y)),
					entry("z", nbt::Payload::int(z)),
				]);
				BlockEntity {
					x,
					y,
					z,
					kind: None,
					name: String::new(),
					nbt: Some(tag(fields)),
				}
			})
			.collect(),
		_ => TYPED
			.iter()
			.enumerate()
			.map(|(i, kind)| {
				let (x, y, z) = at_index(i);
				BlockEntity {
					x,
					y,
					z,
					kind: Some(*kind),
					name: String::new(),
					nbt: Some(tag(rich_fields())),
				}
			})
			.collect(),
	}
}

fn synthetic_packets(version: usize, sections: usize, entities: bool) -> Vec<Vec<u8>> {
	let mut out = Vec::new();
	for (i, (x, z)) in [(0, 0), (1, 0), (-1, 2), (5, -7)].into_iter().enumerate() {
		let mut col = synthetic(version, x, z, sections, 11 + i as u64);
		if entities && version >= at("1.9.4") {
			col.entities = block_entities(version, x, z);
		}
		out.extend(encode_chunk(version, &col).unwrap());
	}
	out
}

fn set_height(dimension: &mut nbt::Entry, min_y: i32, height: i32) {
	if let nbt::Payload::compound(c) = &mut dimension.value {
		c.retain(|e| e.name != "min_y" && e.name != "height");
		c.push(entry("min_y", nbt::Payload::int(min_y)));
		c.push(entry("height", nbt::Payload::int(height)));
	}
}

fn tall_login(version: usize, min_y: i32, height: i32) -> Vec<u8> {
	use minecraft_versions::gen::convert::{v1_17_1, v1_18};
	use minecraft_versions::pds::{Input, Output};
	let login = traffic(version)
		.into_iter()
		.find(|p| minecraft_versions::inspect(ROOT, version, p).is_ok_and(|n| n == "login"))
		.unwrap();
	let mut o = Output::new();
	match version {
		v if v == at("1.18") => {
			let v1_18::play_toClient_packet::Ty::login(mut p) =
				v1_18::play_toClient_packet::deserialize(&mut Input::new(&login)).unwrap()
			else {
				panic!()
			};
			set_height(&mut p.dimension, min_y, height);
			v1_18::play_toClient_packet::serialize(
				&v1_18::play_toClient_packet::Ty::login(p),
				&mut o,
			)
			.unwrap();
		}
		_ => {
			let v1_17_1::play_toClient_packet::Ty::login(mut p) =
				v1_17_1::play_toClient_packet::deserialize(&mut Input::new(&login)).unwrap()
			else {
				panic!()
			};
			set_height(&mut p.dimension, min_y, height);
			v1_17_1::play_toClient_packet::serialize(
				&v1_17_1::play_toClient_packet::Ty::login(p),
				&mut o,
			)
			.unwrap();
		}
	}
	o.into_inner()
}

#[test]
fn tall_worlds_with_block_entities() {
	for (from, targets) in [
		("1.18", ["1.16.2", "1.12.2"]),
		("1.17.1", ["1.20.2", "1.15.2"]),
	] {
		let mut packets = vec![tall_login(at(from), -64, 384)];
		packets.extend(synthetic_packets(at(from), 24, true));
		for to in targets {
			assert_equivalent(from, to, &packets);
		}
	}
}

macro_rules! switches {
	($m:ident, $version:expr, $section:ty) => {{
		use minecraft_versions::gen::convert::$m as m;
		let ser = |p: m::play_toClient_packet::Ty| {
			let mut o = minecraft_versions::pds::Output::new();
			m::play_toClient_packet::serialize(&p, &mut o).unwrap();
			o.into_inner()
		};
		let blocks = minecraft_versions::registry::blocks(VERSIONS[$version]).unwrap();
		let state = |name: &str| blocks.block(name).unwrap().default as i64;
		let position = |y: f64, flags: i8, id: i32| {
			ser(m::play_toClient_packet::Ty::position(
				m::play_toClient_packet_position::Ty {
					x: 8.5,
					y,
					z: 8.5,
					yaw: 0.0,
					pitch: 0.0,
					flags,
					teleportId: id,
					dismountVehicle: false,
				},
			))
		};
		let block = |x: i32, y: i16, z: i32, s: i64| {
			ser(m::play_toClient_packet::Ty::block_change(
				m::play_toClient_packet_block_change::Ty {
					location: m::position::Ty { x, z, y },
					r#type: s as i32,
				},
			))
		};
		let multi = |cx: i32, sy: i32, cz: i32, records: Vec<i64>| {
			ser(m::play_toClient_packet::Ty::multi_block_change(
				m::play_toClient_packet_multi_block_change::Ty {
					chunkCoordinates:
						m::play_toClient_packet_multi_block_change__chunkCoordinates::Ty {
							x: cx,
							z: cz,
							y: sy as $section,
						},
					notTrustEdges: false,
					records,
				},
			))
		};
		let layer = |y: i64, s: i64| {
			(0..256i64)
				.map(move |i| s << 12 | (i & 15) << 8 | (i >> 4) << 4 | y)
				.collect::<Vec<i64>>()
		};
		let (air, stone) = (state("air"), state("stone"));
		let mut packets = vec![tall_login($version, -64, 384)];
		packets.extend(synthetic_packets($version, 24, true));
		packets.extend([
			position(100.0, 0, 1),
			multi(0, 4, 0, layer(0, air)),
			block(3, 64, 4, stone),
			block(3, -50, 4, stone),
			position(300.0, 0, 2),
			block(5, 63, 5, air),
			block(5, 62, 5, air),
			multi(0, -1, 0, layer(15, stone)),
			multi(1, 4, 0, layer(0, air)),
			position(-40.0, 0, 3),
			block(6, -60, 6, air),
			position(100.0, 0, 4),
			position(200.0, 2, 5),
			position(-130.0, 2, 6),
		]);
		packets
	}};
}

fn switch_packets(version: usize) -> Vec<Vec<u8>> {
	match version {
		v if v == at("1.18") => switches!(v1_18, v, i32),
		v => switches!(v1_17_1, v, u32),
	}
}

#[test]
fn tall_world_switches_and_floors() {
	for (from, targets) in [
		("1.18", ["1.16.2", "1.13", "1.8", "1.0.1"]),
		("1.17.1", ["1.16.2", "1.12.2", "1.8", "1.2.5"]),
	] {
		let packets = switch_packets(at(from));
		for to in targets {
			assert_equivalent(from, to, &packets);
			let r = compare_in(at(from), at(to), &packets, Some((at(from), at("1.1"))));
			assert!(
				r.mismatches.is_empty(),
				"{from} -> {to} in 128-high windows: {:?}",
				r.mismatches.first()
			);
		}
	}
}

#[test]
fn switch_columns_skip_the_wire() {
	use minecraft_versions::custom::virtual_wire;
	use minecraft_versions::gen::convert::Carrier;
	let mut cx = Session::new();
	let mut columns = Vec::new();
	for p in &switch_packets(at("1.18")) {
		if !minecraft_versions::inspect(ROOT, at("1.18"), p).is_ok_and(|n| n == "position") {
			convert(ROOT, at("1.18"), at("1.8"), p, &mut cx).unwrap();
			continue;
		}
		let down = convert(ROOT, at("1.18"), at("1.17"), p, &mut cx).unwrap();
		let carried = virtual_wire(ROOT, at("1.17"), at("1.16.2"), &down[0], &mut cx)
			.unwrap()
			.unwrap();
		columns.push(
			carried
				.iter()
				.filter(|c| matches!(c, Carrier::Virtual(_)))
				.count(),
		);
	}
	assert_eq!(columns, [0, 2, 2, 2, 2, 2]);
}

#[test]
#[ignore]
fn tall_worlds_every_target() {
	for from in [at("1.17.1"), at("1.18")] {
		let mut packets = vec![tall_login(from, -64, 384)];
		packets.extend(synthetic_packets(from, 24, true));
		for to in netty().into_iter().filter(|to| *to != from) {
			let r = compare(from, to, &packets);
			assert!(
				r.mismatches.is_empty(),
				"{} -> {}: {:?}",
				VERSIONS[from],
				VERSIONS[to],
				r.mismatches.first()
			);
		}
	}
}

macro_rules! vanilla_login {
	($m:ident, $version:expr) => {{
		use minecraft_versions::gen::convert::$m as m;
		let v: serde_json::Value =
			serde_json::from_str(minecraft_versions::registry::login_packet($version).unwrap())
				.unwrap();
		let login = m::play_toClient_packet::Ty::login(m::play_toClient_packet_login::Ty {
			entityId: 1,
			isHardcore: false,
			gameMode: 0,
			previousGameMode: -1,
			worldNames: vec!["minecraft:overworld".into()],
			dimensionCodec: nbt::Entry::new(
				"",
				nbt::json::typed::read(&v["dimensionCodec"]).unwrap(),
			),
			worldType: "minecraft:overworld".into(),
			worldName: "minecraft:overworld".into(),
			hashedSeed: 0,
			maxPlayers: 20,
			viewDistance: 8,
			simulationDistance: 6,
			reducedDebugInfo: false,
			enableRespawnScreen: true,
			isDebug: false,
			isFlat: false,
			death: None,
		});
		let mut o = minecraft_versions::pds::Output::new();
		m::play_toClient_packet::serialize(&login, &mut o).unwrap();
		o.into_inner()
	}};
}

fn captured(name: &str) -> Vec<u8> {
	let gz = std::fs::read(format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap();
	nbt::gzip::decompress(&gz).unwrap()
}

#[test]
fn padded_server_chunks_stay_on_the_chunk_model() {
	for (from, login, chunk) in [
		(
			"1.19.2",
			vanilla_login!(v1_19_2, "1.19.2"),
			"1.19.2-map_chunk.bin.gz",
		),
		(
			"1.19.2",
			vanilla_login!(v1_19_2, "1.19.2"),
			"1.19.2-map_chunk-16.bin.gz",
		),
		(
			"1.19.4",
			vanilla_login!(v1_19_4, "1.19.4"),
			"1.19.4-map_chunk.bin.gz",
		),
	] {
		let chunk = captured(chunk);
		let Ok(Some(Packet::Chunk(c))) = decode(at(from), &chunk) else {
			panic!("{from}: chunk does not decode")
		};
		assert_eq!(c.sections.len(), 24, "{from}");
		let packets = vec![login, chunk];
		for to in [
			"1.8", "1.12.2", "1.16.2", "1.18.2", "1.19.3", "1.20.2", "26.1",
		] {
			let mut cx = Session::new();
			convert(ROOT, at(from), at(to), &packets[0], &mut cx).unwrap();
			let model = minecraft_versions::custom::virtual_start(
				ROOT,
				at(from),
				at(to),
				&packets[1],
				&mut cx,
			)
			.unwrap();
			assert!(model.is_some(), "{from} -> {to}: chunk left the model");
			let r = compare(at(from), at(to), &packets);
			assert!(
				r.mismatches.is_empty() && r.only_virtual == 0 && r.compared == 2,
				"{from} -> {to}: {r:?}"
			);
		}
	}
}

#[test]
fn synthetic_chunks_both_directions() {
	for (from, sections, targets) in [
		("1.8", 16, ["1.7", "1.13", "1.21.5"]),
		("1.12.2", 16, ["1.9", "1.14", "26.1"]),
		("1.15.2", 16, ["1.12.2", "1.17.1", "1.20"]),
		("1.18", 24, ["1.16.2", "1.13.2", "1.21.3"]),
		("26.1", 24, ["1.20.2", "1.17.1", "1.12.2"]),
	] {
		let packets = synthetic_packets(at(from), sections, true);
		for to in targets {
			assert_equivalent(from, to, &packets);
		}
	}
}
