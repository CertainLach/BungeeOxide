use std::io::{Read, Write};
use std::path::PathBuf;

use bungeeoxide_converter::pds::{Input, Output};
use bungeeoxide_converter::{registry, world, Version};
use bungeeoxide_nbt::gen::entry;
use bungeeoxide_nbt::{CompoundExt, Entry, List, Payload};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

fn compound(entries: Vec<Entry>) -> Payload {
	Payload::Compound(entries)
}

fn data_version(version: Version) -> Entry {
	Entry::new("DataVersion", registry::data_version(version).unwrap())
}

fn bits_for(n: usize) -> u32 {
	match n {
		0 | 1 => 0,
		n => usize::BITS - (n - 1).leading_zeros(),
	}
}

fn pack(values: &[u32], bits: u32, spanning: bool) -> Vec<i64> {
	let bits = bits as usize;
	if spanning {
		let mut out = vec![0u64; (values.len() * bits).div_ceil(64)];
		for (i, v) in values.iter().enumerate() {
			let (w, off) = (i * bits / 64, i * bits % 64);
			out[w] |= (*v as u64) << off;
			if off + bits > 64 {
				out[w + 1] |= (*v as u64) >> (64 - off);
			}
		}
		out.into_iter().map(|l| l as i64).collect()
	} else {
		let per = 64 / bits;
		let mut out = vec![0u64; values.len().div_ceil(per)];
		for (i, v) in values.iter().enumerate() {
			out[i / per] |= (*v as u64) << (i % per * bits);
		}
		out.into_iter().map(|l| l as i64).collect()
	}
}

fn unpack(data: &[i64], bits: u32, count: usize) -> Vec<usize> {
	let bits = bits as usize;
	let per = 64 / bits;
	(0..count)
		.map(|i| ((data[i / per] as u64 >> (i % per * bits)) & ((1 << bits) - 1)) as usize)
		.collect()
}

fn section(chunk: &Entry, y: i32) -> Option<&Vec<Entry>> {
	chunk
		.value
		.field("sections")?
		.compounds()
		.iter()
		.find(|s| s.field("Y").and_then(Payload::to_i32) == Some(y >> 4))
}

fn container(c: &[Entry], min_bits: u32, cells: usize, at: usize) -> Payload {
	let palette = c.field("palette").unwrap().as_list().unwrap().values();
	if palette.len() == 1 {
		return palette[0].clone();
	}
	let Some(Payload::LongArray(data)) = c.field("data") else {
		panic!("no data for {} entries", palette.len())
	};
	let bits = bits_for(palette.len()).max(min_bits);
	palette[unpack(data, bits, cells)[at]].clone()
}

fn block(chunk: &Entry, x: usize, y: i32, z: usize) -> String {
	let Some(s) = section(chunk, y) else {
		return "minecraft:air".to_owned();
	};
	let states = s.field("block_states").unwrap().as_compound().unwrap();
	let at = ((y & 15) as usize) << 8 | z << 4 | x;
	let state = container(states, 4, 4096, at);
	let name = state.field("Name").unwrap().as_str().unwrap().to_owned();
	match state.field("Properties").and_then(Payload::as_compound) {
		Some(p) => {
			let props: Vec<String> = p
				.iter()
				.map(|e| format!("{}={}", e.name, e.value.as_str().unwrap()))
				.collect();
			format!("{name}[{}]", props.join(","))
		}
		None => name,
	}
}

fn biome(chunk: &Entry, x: usize, y: i32, z: usize) -> String {
	let s = section(chunk, y).unwrap();
	let biomes = s.field("biomes").unwrap().as_compound().unwrap();
	let at = ((y & 15) as usize >> 2) << 4 | (z >> 2) << 2 | x >> 2;
	container(biomes, 0, 64, at).as_str().unwrap().to_owned()
}

fn int(chunk: &Entry, name: &str) -> i32 {
	chunk.value.field(name).and_then(Payload::to_i32).unwrap()
}

fn set_nibble(a: &mut [i8], i: usize, v: u8) {
	a[i >> 1] = (a[i >> 1] as u8 | (v & 15) << ((i & 1) * 4)) as i8;
}

fn mcregion_chunk(x: i32, z: i32) -> Entry {
	let mut ids = vec![0i8; 32768];
	let mut data = vec![0i8; 16384];
	let at = |x: usize, y: usize, z: usize| x << 11 | z << 7 | y;
	for x in 0..16 {
		for z in 0..16 {
			for y in 0..10 {
				ids[at(x, y, z)] = 1;
			}
		}
	}
	ids[at(3, 20, 5)] = 35;
	set_nibble(&mut data, at(3, 20, 5), 14);
	ids[at(15, 127, 15)] = 20;
	Entry::new(
		"",
		vec![Entry::new(
			"Level",
			vec![
				Entry::new("xPos", x),
				Entry::new("zPos", z),
				Entry::new("Blocks", Payload::ByteArray(ids)),
				Entry::new("Data", Payload::ByteArray(data)),
			],
		)],
	)
}

#[test]
fn mcregion_chunks_from_1_0_reach_latest() {
	let out = world::convert_chunk(&mcregion_chunk(2, -3)).unwrap();
	assert_eq!(
		int(&out, "DataVersion"),
		registry::data_version(Version::LATEST).unwrap()
	);
	assert_eq!((int(&out, "xPos"), int(&out, "zPos")), (2, -3));
	assert_eq!(block(&out, 0, 5, 0), "minecraft:stone");
	assert_eq!(block(&out, 3, 20, 5), "minecraft:red_wool");
	assert_eq!(block(&out, 15, 127, 15), "minecraft:glass");
	assert_eq!(block(&out, 4, 20, 5), "minecraft:air");
	assert_eq!(biome(&out, 0, 0, 0), "minecraft:plains");
}

#[test]
fn numeric_anvil_chunks_keep_states_and_biomes() {
	let mut ids = vec![1i8; 4096];
	let mut data = vec![0i8; 2048];
	let stairs = 4 << 8 | 7 << 4 | 2;
	ids[stairs] = 53;
	set_nibble(&mut data, stairs, 1);
	let mut biomes = vec![2i8; 256];
	for z in 0..4 {
		for x in 0..4 {
			biomes[z * 16 + x] = 3;
		}
	}
	let chunk = Entry::new(
		"",
		vec![
			data_version(Version::V1_12_2),
			Entry::new(
				"Level",
				vec![
					Entry::new("xPos", 0),
					Entry::new("zPos", 0),
					Entry::new("Biomes", Payload::ByteArray(biomes)),
					Entry::new(
						"Sections",
						List::of(vec![compound(vec![
							Entry::new("Y", 4i8),
							Entry::new("Blocks", Payload::ByteArray(ids)),
							Entry::new("Data", Payload::ByteArray(data)),
						])]),
					),
				],
			),
		],
	);
	let out = world::convert_chunk(&chunk).unwrap();
	assert_eq!(block(&out, 0, 64, 0), "minecraft:stone");
	let stairs = block(&out, 2, 68, 7);
	assert!(
		stairs.starts_with("minecraft:oak_stairs[") && stairs.contains("facing=west"),
		"{stairs}"
	);
	assert_eq!(block(&out, 0, 10, 0), "minecraft:air");
	assert_eq!(biome(&out, 0, 64, 0), "minecraft:windswept_hills");
	assert_eq!(biome(&out, 8, 64, 8), "minecraft:desert");
	assert_eq!(biome(&out, 8, 200, 8), "minecraft:desert");
}

fn wools() -> Vec<String> {
	[
		"white",
		"orange",
		"magenta",
		"light_blue",
		"yellow",
		"lime",
		"pink",
		"gray",
		"light_gray",
		"cyan",
		"purple",
		"blue",
		"brown",
		"green",
		"red",
		"black",
	]
	.iter()
	.map(|c| format!("minecraft:{c}_wool"))
	.collect()
}

fn palette_chunk(version: Version, spanning: bool) -> Entry {
	let mut names = vec!["minecraft:air".to_owned()];
	names.extend(wools());
	let palette: Vec<Payload> = names
		.iter()
		.map(|n| compound(vec![Entry::new("Name", n.as_str())]))
		.collect();
	let indices: Vec<u32> = (0..4096).map(|i| (i % 17) as u32).collect();
	Entry::new(
		"",
		vec![
			data_version(version),
			Entry::new(
				"Level",
				vec![
					Entry::new("xPos", 1),
					Entry::new("zPos", 1),
					Entry::new(
						"Sections",
						List::of(vec![compound(vec![
							Entry::new("Y", 1i8),
							Entry::new("Palette", List::of(palette)),
							Entry::new(
								"BlockStates",
								Payload::LongArray(pack(&indices, 5, spanning)),
							),
						])]),
					),
				],
			),
		],
	)
}

#[test]
fn palette_sections_read_both_packings() {
	for (from, spanning) in [(Version::V1_14, true), (Version::V1_16_2, false)] {
		let out = world::convert_chunk(&palette_chunk(from, spanning)).unwrap();
		let wools = wools();
		for i in [1usize, 16, 17, 100, 4095] {
			let (x, y, z) = (i & 15, 16 + (i >> 8) as i32, i >> 4 & 15);
			let want = match i % 17 {
				0 => "minecraft:air",
				n => &wools[n - 1],
			};
			assert_eq!(block(&out, x, y, z), want, "{from} block {i}");
		}
	}
}

#[test]
fn modern_chunks_follow_renames() {
	let chunk = Entry::new(
		"",
		vec![
			data_version(Version::V1_18),
			Entry::new("xPos", 5),
			Entry::new("zPos", 6),
			Entry::new(
				"sections",
				List::of(vec![compound(vec![
					Entry::new("Y", -4i8),
					Entry::new(
						"block_states",
						vec![Entry::new(
							"palette",
							List::of(vec![compound(vec![Entry::new("Name", "minecraft:grass")])]),
						)],
					),
					Entry::new(
						"biomes",
						vec![Entry::new(
							"palette",
							List::of(vec![Payload::from("minecraft:snowy_plains")]),
						)],
					),
				])]),
			),
		],
	);
	let out = world::convert_chunk(&chunk).unwrap();
	assert_eq!(block(&out, 3, -60, 3), "minecraft:short_grass");
	assert_eq!(biome(&out, 3, -60, 3), "minecraft:snowy_plains");
}

fn nbt(chunk: &Entry) -> Vec<u8> {
	let mut o = Output::new();
	entry::serialize(chunk, &mut o).unwrap();
	o.into_inner()
}

fn region(chunks: &[(usize, u8, Vec<u8>)]) -> Vec<u8> {
	let mut out = vec![0u8; 8192];
	for (slot, kind, body) in chunks {
		let offset = out.len() / 4096;
		out.extend_from_slice(&(body.len() as u32 + 1).to_be_bytes());
		out.push(*kind);
		out.extend_from_slice(body);
		out.resize(out.len().div_ceil(4096) * 4096, 0);
		let sectors = (out.len() / 4096 - offset) as u32;
		out[slot * 4..slot * 4 + 4]
			.copy_from_slice(&((offset as u32) << 8 | sectors).to_be_bytes());
	}
	out
}

fn zlib(data: &[u8]) -> Vec<u8> {
	let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
	z.write_all(data).unwrap();
	z.finish().unwrap()
}

fn slot(region: &[u8], slot: usize) -> Option<Entry> {
	let location = u32::from_be_bytes(region[slot * 4..slot * 4 + 4].try_into().unwrap());
	if location == 0 {
		return None;
	}
	let start = (location >> 8) as usize * 4096;
	let length = u32::from_be_bytes(region[start..start + 4].try_into().unwrap()) as usize;
	assert_eq!(region[start + 4], 2);
	let mut raw = Vec::new();
	ZlibDecoder::new(&region[start + 5..start + 4 + length])
		.read_to_end(&mut raw)
		.unwrap();
	Some(entry::deserialize(&mut Input::new(&raw)).unwrap())
}

#[test]
fn regions_convert_readable_chunks_and_report_the_rest() {
	let data = region(&[
		(0, 2, zlib(&nbt(&mcregion_chunk(0, 0)))),
		(33, 3, nbt(&mcregion_chunk(1, 1))),
		(64, 9, vec![1, 2, 3]),
		(65, 2, zlib(b"not nbt")),
	]);
	let (out, report) = world::convert_region(&data);
	assert_eq!(report.chunks, 2);
	assert_eq!(report.skipped.len(), 2, "{:?}", report.skipped);
	assert_eq!(int(&slot(&out, 0).unwrap(), "xPos"), 0);
	let second = slot(&out, 33).unwrap();
	assert_eq!(int(&second, "zPos"), 1);
	assert_eq!(block(&second, 3, 20, 5), "minecraft:red_wool");
	assert!(slot(&out, 64).is_none());
}

struct Scratch(PathBuf);

impl Drop for Scratch {
	fn drop(&mut self) {
		let _ = std::fs::remove_dir_all(&self.0);
	}
}

#[test]
fn worlds_get_anvil_regions_and_level_dat() {
	let root = Scratch(std::env::temp_dir().join(format!("convert-world-{}", std::process::id())));
	let (src, dst) = (root.0.join("old"), root.0.join("new"));
	std::fs::create_dir_all(src.join("region")).unwrap();
	std::fs::create_dir_all(src.join("DIM-1/region")).unwrap();
	let data = region(&[(0, 2, zlib(&nbt(&mcregion_chunk(0, 0))))]);
	std::fs::write(src.join("region/r.0.0.mcr"), &data).unwrap();
	std::fs::write(src.join("DIM-1/region/r.0.0.mcr"), &data).unwrap();
	let level = Entry::new(
		"",
		vec![Entry::new(
			"Data",
			vec![Entry::new("version", 19132), Entry::new("LevelName", "old")],
		)],
	);
	std::fs::write(
		src.join("level.dat"),
		bungeeoxide_nbt::gzip::write(&level).unwrap(),
	)
	.unwrap();
	let report = world::convert_world(&src, &dst).unwrap();
	assert_eq!((report.regions, report.chunks), (2, 2));
	assert_eq!(report.sources.get("1.8 or older"), Some(&2));
	for path in ["region/r.0.0.mca", "DIM-1/region/r.0.0.mca"] {
		let out = std::fs::read(dst.join(path)).unwrap();
		assert_eq!(
			block(&slot(&out, 0).unwrap(), 3, 20, 5),
			"minecraft:red_wool"
		);
	}
	let level =
		bungeeoxide_nbt::gzip::read(&std::fs::read(dst.join("level.dat")).unwrap()).unwrap();
	let data = level.value.field("Data").unwrap();
	assert_eq!(data.field("version").and_then(Payload::to_i32), Some(19133));
	assert_eq!(
		data.field("LevelName").and_then(Payload::as_str),
		Some("old")
	);
	assert!(matches!(
		world::convert_world(&src, &dst),
		Err(world::Error::Target(_))
	));
}

#[test]
fn chunk_versions_come_from_their_data_version() {
	let mut newer = palette_chunk(Version::V1_16_2, false);
	newer.value.as_compound_mut().unwrap()[0] = Entry::new("DataVersion", i32::MAX);
	assert!(world::convert_chunk(&newer).is_err());
	let mut undated = palette_chunk(Version::V1_16_2, false);
	undated.value.as_compound_mut().unwrap().remove(0);
	assert!(world::convert_chunk(&undated).is_err());
	let mut snapshot = palette_chunk(Version::V1_16_2, false);
	let between = registry::data_version(Version::V1_16_2).unwrap() + 3;
	snapshot.value.as_compound_mut().unwrap()[0] = Entry::new("DataVersion", between);
	assert_eq!(
		block(&world::convert_chunk(&snapshot).unwrap(), 1, 16, 0),
		"minecraft:white_wool"
	);
}
