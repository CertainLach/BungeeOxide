use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use bungeeoxide_nbt::gen::entry;
use bungeeoxide_nbt::{CompoundExt, CompoundVecExt, Entry, List, Payload};
use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::custom::chunks::bits::{bits_for, pack, unpack};
use crate::custom::chunks::composed;
use crate::gen::convert::Version;
use crate::pds::{stack, Input, Output};
use crate::registry::{self, Blocks};
use crate::STACK_BUDGET;

const SECTOR: usize = 4096;
const SLOTS: usize = 1024;
const MAX_SECTORS: usize = 255;
const MAX_CHUNK: u64 = 64 << 20;
const ANVIL: i32 = 19133;
const BLOCKS: usize = 4096;
const CELLS: usize = 64;
const PLAINS: &str = "minecraft:plains";

// https://minecraft.wiki/w/Java_Edition_1.18#World_generation
const BIOME_RENAMES: &[(&str, &str)] = &[
	("badlands_plateau", "badlands"),
	("bamboo_jungle_hills", "bamboo_jungle"),
	("birch_forest_hills", "birch_forest"),
	("dark_forest_hills", "dark_forest"),
	("deep_warm_ocean", "warm_ocean"),
	("desert_hills", "desert"),
	("desert_lakes", "desert"),
	("giant_spruce_taiga", "old_growth_spruce_taiga"),
	("giant_spruce_taiga_hills", "old_growth_spruce_taiga"),
	("giant_tree_taiga", "old_growth_pine_taiga"),
	("giant_tree_taiga_hills", "old_growth_pine_taiga"),
	("gravelly_mountains", "windswept_gravelly_hills"),
	("jungle_edge", "sparse_jungle"),
	("jungle_hills", "jungle"),
	("modified_badlands_plateau", "badlands"),
	("modified_gravelly_mountains", "windswept_gravelly_hills"),
	("modified_jungle", "jungle"),
	("modified_jungle_edge", "sparse_jungle"),
	("modified_wooded_badlands_plateau", "wooded_badlands"),
	("mountain_edge", "windswept_hills"),
	("mountains", "windswept_hills"),
	("mushroom_field_shore", "mushroom_fields"),
	("nether", "nether_wastes"),
	("shattered_savanna", "windswept_savanna"),
	("shattered_savanna_plateau", "windswept_savanna"),
	("snowy_mountains", "snowy_plains"),
	("snowy_taiga_hills", "snowy_taiga"),
	("snowy_taiga_mountains", "snowy_taiga"),
	("snowy_tundra", "snowy_plains"),
	("stone_shore", "stony_shore"),
	("swamp_hills", "swamp"),
	("taiga_hills", "taiga"),
	("taiga_mountains", "taiga"),
	("tall_birch_forest", "old_growth_birch_forest"),
	("tall_birch_hills", "old_growth_birch_forest"),
	("wooded_badlands_plateau", "wooded_badlands"),
	("wooded_hills", "forest"),
	("wooded_mountains", "windswept_forest"),
];

#[derive(Debug)]
pub enum Error {
	Io(PathBuf, io::Error),
	Data(String),
	Target(PathBuf),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::Io(path, e) => write!(f, "{}: {e}", path.display()),
			Error::Data(why) => f.write_str(why),
			Error::Target(path) => write!(f, "{} exists and is not empty", path.display()),
		}
	}
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
	pub regions: usize,
	pub chunks: usize,
	pub sources: BTreeMap<String, usize>,
	pub skipped: Vec<String>,
}

impl Report {
	fn add(&mut self, other: Report) {
		self.regions += other.regions;
		self.chunks += other.chunks;
		for (source, n) in other.sources {
			*self.sources.entry(source).or_default() += n;
		}
		self.skipped.extend(other.skipped);
	}
}

fn io<T>(path: &Path, result: io::Result<T>) -> Result<T> {
	result.map_err(|e| Error::Io(path.to_owned(), e))
}

fn bad<T>(why: impl Into<String>) -> Result<T> {
	Err(Error::Data(why.into()))
}

pub fn convert_world(src: &Path, dst: &Path) -> Result<Report> {
	let occupied = match fs::read_dir(dst) {
		Ok(mut entries) => entries.next().is_some(),
		Err(e) if e.kind() == io::ErrorKind::NotFound => false,
		Err(e) => return Err(Error::Io(dst.to_owned(), e)),
	};
	if occupied {
		return Err(Error::Target(dst.to_owned()));
	}
	let jobs = regions(src, dst)?;
	io(dst, fs::create_dir_all(dst))?;
	level(src, dst)?;
	let next = AtomicUsize::new(0);
	let total = Mutex::new(Report::default());
	let failure = Mutex::new(None);
	let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
	std::thread::scope(|s| {
		for _ in 0..threads.min(jobs.len()) {
			s.spawn(|| loop {
				let i = next.fetch_add(1, Ordering::Relaxed);
				let Some((source, target)) = jobs.get(i) else {
					break;
				};
				match region(source, target) {
					Ok(r) => total.lock().expect("report lock").add(r),
					Err(e) => {
						failure.lock().expect("failure lock").get_or_insert(e);
						next.store(jobs.len(), Ordering::Relaxed);
						break;
					}
				}
			});
		}
	});
	match failure.into_inner().expect("failure lock") {
		Some(e) => Err(e),
		None => Ok(total.into_inner().expect("report lock")),
	}
}

fn regions(src: &Path, dst: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
	let mut found: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
	let mut pending = vec![PathBuf::new()];
	while let Some(rel) = pending.pop() {
		let dir = src.join(&rel);
		for e in io(&dir, fs::read_dir(&dir))? {
			let e = io(&dir, e)?;
			let kind = io(&dir, e.file_type())?;
			let name = e.file_name();
			let Some(name) = name.to_str() else {
				continue;
			};
			if kind.is_dir() {
				pending.push(rel.join(name));
				continue;
			}
			let in_region = rel.file_name().is_some_and(|n| n == "region");
			let stem = name
				.strip_suffix(".mca")
				.or_else(|| name.strip_suffix(".mcr"))
				.filter(|s| s.starts_with("r."));
			let (true, true, Some(stem)) = (kind.is_file(), in_region, stem) else {
				continue;
			};
			let target = rel.join(format!("{stem}.mca"));
			if name.ends_with(".mca") || !found.contains_key(&target) {
				found.insert(target, e.path());
			}
		}
	}
	Ok(found
		.into_iter()
		.map(|(rel, source)| (source, dst.join(rel)))
		.collect())
}

fn level(src: &Path, dst: &Path) -> Result<()> {
	let path = src.join("level.dat");
	let data = match fs::read(&path) {
		Ok(d) => d,
		Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
		Err(e) => return Err(Error::Io(path, e)),
	};
	let mut root = stack::budget(STACK_BUDGET, || bungeeoxide_nbt::gzip::read(&data))
		.map_err(|e| Error::Data(format!("{}: {e}", path.display())))?;
	if let Some(d) = root
		.value
		.as_compound_mut()
		.and_then(|c| c.compound_mut("Data"))
	{
		d.put("version", ANVIL);
	}
	let out = bungeeoxide_nbt::gzip::write(&root)
		.map_err(|e| Error::Data(format!("{}: {e}", path.display())))?;
	let target = dst.join("level.dat");
	io(&target, fs::write(&target, out))
}

fn region(source: &Path, target: &Path) -> Result<Report> {
	let data = io(source, fs::read(source))?;
	let (out, mut report) = convert_region(&data);
	report.skipped = report
		.skipped
		.into_iter()
		.map(|why| format!("{}: {why}", source.display()))
		.collect();
	if report.chunks > 0 {
		if let Some(dir) = target.parent() {
			io(dir, fs::create_dir_all(dir))?;
		}
		io(target, fs::write(target, out))?;
		report.regions = 1;
	}
	Ok(report)
}

pub fn convert_region(data: &[u8]) -> (Vec<u8>, Report) {
	let mut report = Report::default();
	let mut chunks = Vec::new();
	for slot in 0..SLOTS {
		let converted = read_slot(data, slot).and_then(|raw| {
			raw.map(|raw| {
				let (chunk, source) = convert_chunk_from(&parse(&raw)?)?;
				let bytes = encode(&chunk)?;
				match (bytes.len() + 5).div_ceil(SECTOR) <= MAX_SECTORS {
					true => Ok((bytes, source)),
					false => bad("converted chunk needs more than 255 sectors"),
				}
			})
			.transpose()
		});
		match converted {
			Ok(None) => {}
			Ok(Some((bytes, source))) => {
				chunks.push((slot, word(data, SECTOR + slot * 4).unwrap_or(0), bytes));
				report.chunks += 1;
				*report.sources.entry(source).or_default() += 1;
			}
			Err(e) => report
				.skipped
				.push(format!("chunk {} {}: {e}", slot % 32, slot / 32)),
		}
	}
	(write_region(&chunks), report)
}

fn word(data: &[u8], at: usize) -> Option<u32> {
	Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn read_slot(data: &[u8], slot: usize) -> Result<Option<Vec<u8>>> {
	let Some(location) = word(data, slot * 4).filter(|l| *l != 0) else {
		return Ok(None);
	};
	let start = (location >> 8) as usize * SECTOR;
	let (Some(length), Some(&kind)) = (word(data, start), data.get(start + 4)) else {
		return bad("points past the end of the region file");
	};
	let Some(body) = (length as usize)
		.checked_sub(1)
		.and_then(|n| data.get(start + 5..start + 5 + n))
	else {
		return bad("is truncated");
	};
	let mut out = Vec::new();
	let read = match kind {
		1 => GzDecoder::new(body).take(MAX_CHUNK).read_to_end(&mut out),
		2 => ZlibDecoder::new(body).take(MAX_CHUNK).read_to_end(&mut out),
		3 => {
			out.extend_from_slice(body);
			Ok(body.len())
		}
		k if k & 128 != 0 => return bad("is stored outside the region file"),
		k => return bad(format!("uses unsupported compression {k}")),
	};
	read.map_err(|e| Error::Data(e.to_string()))?;
	Ok(Some(out))
}

fn parse(raw: &[u8]) -> Result<Entry> {
	stack::budget(STACK_BUDGET, || entry::deserialize(&mut Input::new(raw)))
		.map_err(|e| Error::Data(format!("unreadable NBT: {e:?}")))
}

fn encode(chunk: &Entry) -> Result<Vec<u8>> {
	let mut o = Output::new();
	stack::budget(STACK_BUDGET, || entry::serialize(chunk, &mut o))
		.map_err(|e| Error::Data(format!("can't write NBT: {e:?}")))?;
	let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
	z.write_all(&o.into_inner())
		.and_then(|()| z.finish())
		.map_err(|e| Error::Data(e.to_string()))
}

fn write_region(chunks: &[(usize, u32, Vec<u8>)]) -> Vec<u8> {
	let mut out = vec![0u8; 2 * SECTOR];
	for (slot, stamp, bytes) in chunks {
		let offset = out.len() / SECTOR;
		let length = bytes.len() + 1;
		let sectors = (length + 4).div_ceil(SECTOR);
		out.extend_from_slice(&(length as u32).to_be_bytes());
		out.push(2);
		out.extend_from_slice(bytes);
		out.resize(out.len().div_ceil(SECTOR) * SECTOR, 0);
		let location = (offset as u32) << 8 | sectors as u32;
		out[slot * 4..slot * 4 + 4].copy_from_slice(&location.to_be_bytes());
		out[SECTOR + slot * 4..SECTOR + slot * 4 + 4].copy_from_slice(&stamp.to_be_bytes());
	}
	out
}

struct Section {
	y: i8,
	states: Vec<u32>,
	biomes: Vec<String>,
}

struct Source {
	x: i32,
	z: i32,
	space: Version,
	sections: Vec<Section>,
}

pub fn convert_chunk(chunk: &Entry) -> Result<Entry> {
	convert_chunk_from(chunk).map(|(chunk, _)| chunk)
}

fn convert_chunk_from(chunk: &Entry) -> Result<(Entry, String)> {
	let Some(root) = chunk.value.as_compound() else {
		return bad("chunk NBT is not a compound");
	};
	let dated = dated(root)?;
	let source = match root.field("Level").and_then(Payload::as_compound) {
		Some(level) if level.field("Blocks").is_some() => mcregion(level, dated)?,
		Some(level) => anvil(level, dated)?,
		None => modern(root, dated)?,
	};
	let label = match dated {
		Some(v) => v.to_string(),
		None => format!("{} or older", Version::V1_8),
	};
	Ok((write(&source)?, label))
}

fn dated(root: &[Entry]) -> Result<Option<Version>> {
	let Some(data) = root.field("DataVersion").and_then(Payload::to_i32) else {
		return Ok(None);
	};
	match registry::data_version(Version::LATEST) {
		Some(latest) if data > latest => bad(format!(
			"chunk data version {data} is newer than {}",
			Version::LATEST
		)),
		_ => Ok(registry::version_at_data(data)),
	}
}

fn named_version(dated: Option<Version>) -> Result<Version> {
	match dated {
		Some(v) if v >= Version::V1_13 => Ok(v),
		_ => bad("block palette in a chunk without a 1.13+ data version"),
	}
}

fn coords(c: &[Entry]) -> Result<(i32, i32)> {
	match (
		c.field("xPos").and_then(Payload::to_i32),
		c.field("zPos").and_then(Payload::to_i32),
	) {
		(Some(x), Some(z)) => Ok((x, z)),
		_ => bad("chunk has no xPos/zPos"),
	}
}

fn bytes<'a>(c: &'a [Entry], name: &str) -> Option<&'a [i8]> {
	match c.field(name)? {
		Payload::ByteArray(b) => Some(b),
		_ => None,
	}
}

fn longs<'a>(c: &'a [Entry], name: &str) -> &'a [i64] {
	match c.field(name) {
		Some(Payload::LongArray(l)) => l,
		_ => &[],
	}
}

fn nibble(a: &[i8], i: usize) -> u32 {
	a.get(i >> 1)
		.map_or(0, |b| ((*b as u8 >> ((i & 1) * 4)) & 15) as u32)
}

fn numeric_space(dated: Option<Version>) -> Version {
	dated.map_or(Version::V1_8, |v| v.clamp(Version::V1_8, Version::V1_12_2))
}

fn named_blocks(version: Version) -> Result<&'static Blocks> {
	registry::blocks(version)
		.filter(|b| b.flattened)
		.ok_or_else(|| Error::Data(format!("no block registry for {version}")))
}

fn mcregion(level: &[Entry], dated: Option<Version>) -> Result<Source> {
	let (x, z) = coords(level)?;
	let Some(ids) = bytes(level, "Blocks").filter(|b| b.len() == 8 * BLOCKS) else {
		return bad("McRegion chunk has no 32768 block ids");
	};
	let data = bytes(level, "Data").unwrap_or(&[]);
	let sections = (0..8)
		.map(|s| Section {
			y: s as i8,
			states: (0..BLOCKS)
				.map(|i| {
					let at = (i & 15) << 11 | (i >> 4 & 15) << 7 | (s * 16 + (i >> 8));
					(ids[at] as u8 as u32) << 4 | nibble(data, at)
				})
				.collect(),
			biomes: vec![PLAINS.to_owned(); CELLS],
		})
		.collect();
	Ok(Source {
		x,
		z,
		space: numeric_space(dated),
		sections,
	})
}

fn anvil(level: &[Entry], dated: Option<Version>) -> Result<Source> {
	let (x, z) = coords(level)?;
	let listed = level.field("Sections").map_or(&[][..], Payload::compounds);
	let numeric = listed.iter().any(|s| s.field("Blocks").is_some());
	let space = match numeric {
		true => numeric_space(dated),
		false => named_version(dated)?,
	};
	let biomes = legacy_biomes(level, space);
	let mut sections = Vec::new();
	let mut air = 0;
	if !numeric {
		let blocks = named_blocks(space)?;
		air = blocks.block("air").map_or(0, |b| b.default);
		for s in listed {
			let (Some(y), Some(palette)) = (
				s.field("Y").and_then(Payload::to_i32),
				s.field("Palette").map(Payload::compounds),
			) else {
				continue;
			};
			let palette: Vec<u32> = palette
				.iter()
				.map(|e| named_state(blocks, air, e))
				.collect();
			sections.push(Section {
				y: y as i8,
				states: indexed(&palette, longs(s, "BlockStates"))?,
				biomes: biomes(y),
			});
		}
	} else {
		for s in listed {
			let (Some(y), Some(ids)) = (
				s.field("Y").and_then(Payload::to_i32),
				bytes(s, "Blocks").filter(|b| b.len() == BLOCKS),
			) else {
				continue;
			};
			let data = bytes(s, "Data").unwrap_or(&[]);
			let add = bytes(s, "Add").unwrap_or(&[]);
			sections.push(Section {
				y: y as i8,
				states: (0..BLOCKS)
					.map(|i| (nibble(add, i) << 8 | ids[i] as u8 as u32) << 4 | nibble(data, i))
					.collect(),
				biomes: biomes(y),
			});
		}
	}
	for y in 0..16 {
		if !sections.iter().any(|s| s.y as i32 == y) {
			sections.push(Section {
				y: y as i8,
				states: vec![air; BLOCKS],
				biomes: biomes(y),
			});
		}
	}
	Ok(Source {
		x,
		z,
		space,
		sections,
	})
}

fn legacy_biomes(level: &[Entry], space: Version) -> impl Fn(i32) -> Vec<String> {
	let names = registry::names(space, "biomes");
	let mut known: HashMap<i32, String> = HashMap::new();
	let mut name = |id: i32| {
		known
			.entry(id)
			.or_insert_with(|| {
				names
					.and_then(|n| n.name(u32::try_from(id).ok()?))
					.map_or_else(|| PLAINS.to_owned(), latest_biome)
			})
			.clone()
	};
	let ids: Vec<i32> = match level.field("Biomes") {
		Some(Payload::ByteArray(b)) => b.iter().map(|v| *v as u8 as i32).collect(),
		Some(Payload::IntArray(v)) => v.clone(),
		_ => Vec::new(),
	};
	let cells: Vec<String> = match ids.len() {
		256 => (0..CELLS)
			.map(|c| name(ids[(c >> 2 & 3) * 64 + (c & 3) * 4]))
			.collect(),
		1024 => ids.iter().map(|id| name(*id)).collect(),
		_ => Vec::new(),
	};
	move |y: i32| match cells.len() {
		CELLS => cells.clone(),
		1024 => {
			let base = (y.clamp(0, 15) as usize) * CELLS;
			cells[base..base + CELLS].to_vec()
		}
		_ => vec![PLAINS.to_owned(); CELLS],
	}
}

fn latest_biome(name: &str) -> String {
	let name = name.strip_prefix("minecraft:").unwrap_or(name);
	let name = BIOME_RENAMES
		.iter()
		.find(|(old, _)| *old == name)
		.map_or(name, |(_, new)| new);
	match registry::names(Version::LATEST, "biomes").and_then(|n| n.id(name)) {
		Some(_) => format!("minecraft:{name}"),
		None => PLAINS.to_owned(),
	}
}

fn modern(root: &[Entry], dated: Option<Version>) -> Result<Source> {
	let (x, z) = coords(root)?;
	let space = named_version(dated)?;
	let blocks = named_blocks(space)?;
	let air = blocks.block("air").map_or(0, |b| b.default);
	let mut sections = Vec::new();
	for s in root.field("sections").map_or(&[][..], Payload::compounds) {
		let (Some(y), Some(states)) = (
			s.field("Y").and_then(Payload::to_i32),
			s.field("block_states").and_then(Payload::as_compound),
		) else {
			continue;
		};
		let palette: Vec<u32> = states
			.field("palette")
			.map_or(&[][..], Payload::compounds)
			.iter()
			.map(|e| named_state(blocks, air, e))
			.collect();
		if palette.is_empty() {
			continue;
		}
		sections.push(Section {
			y: y as i8,
			states: indexed(&palette, longs(states, "data"))?,
			biomes: modern_biomes(s)?,
		});
	}
	Ok(Source {
		x,
		z,
		space,
		sections,
	})
}

fn modern_biomes(s: &[Entry]) -> Result<Vec<String>> {
	let Some(b) = s.field("biomes").and_then(Payload::as_compound) else {
		return Ok(vec![PLAINS.to_owned(); CELLS]);
	};
	let names: Vec<String> = b
		.field("palette")
		.and_then(Payload::as_list)
		.and_then(List::strings)
		.unwrap_or(&[])
		.iter()
		.map(|n| latest_biome(n))
		.collect();
	match names.len() {
		0 => Ok(vec![PLAINS.to_owned(); CELLS]),
		1 => Ok(vec![names[0].clone(); CELLS]),
		n => {
			let bits = bits_for(n);
			let data: Vec<u64> = longs(b, "data").iter().map(|l| *l as u64).collect();
			if data.len() != CELLS.div_ceil(64 / bits as usize) {
				return bad(format!("biomes have {} longs for {n} entries", data.len()));
			}
			Ok(unpack(&data, bits, CELLS, false)
				.into_iter()
				.map(|i| names.get(i as usize).unwrap_or(&names[0]).clone())
				.collect())
		}
	}
}

fn named_state(blocks: &Blocks, air: u32, e: &[Entry]) -> u32 {
	let Some(block) = e
		.field("Name")
		.and_then(Payload::as_str)
		.and_then(|n| blocks.block(n))
	else {
		return air;
	};
	let props: Vec<(&str, &str)> = e
		.field("Properties")
		.and_then(Payload::as_compound)
		.map_or_else(Vec::new, |p| {
			p.iter()
				.filter_map(|x| Some((x.name.as_str(), x.value.as_str()?)))
				.collect()
		});
	block.state(&props)
}

fn indexed(palette: &[u32], data: &[i64]) -> Result<Vec<u32>> {
	if palette.len() == 1 && data.is_empty() {
		return Ok(vec![palette[0]; BLOCKS]);
	}
	let bits = bits_for(palette.len()).max(4);
	let spanning = data.len() == 64 * bits as usize;
	if !spanning && data.len() != BLOCKS.div_ceil(64 / bits as usize) {
		return bad(format!(
			"block states have {} longs for {} palette entries",
			data.len(),
			palette.len()
		));
	}
	let data: Vec<u64> = data.iter().map(|l| *l as u64).collect();
	Ok(unpack(&data, bits, BLOCKS, spanning)
		.into_iter()
		.map(|i| palette.get(i as usize).copied().unwrap_or(palette[0]))
		.collect())
}

fn write(source: &Source) -> Result<Entry> {
	let latest = named_blocks(Version::LATEST)?;
	let Some(data_version) = registry::data_version(Version::LATEST) else {
		return bad(format!("no data version for {}", Version::LATEST));
	};
	let air = latest.block("air").map_or(0, |b| b.default);
	let table = (source.space != Version::LATEST).then(|| composed(source.space, Version::LATEST));
	let mut ordered: Vec<&Section> = source.sections.iter().collect();
	ordered.sort_by_key(|s| s.y);
	let sections: Vec<Payload> = ordered
		.iter()
		.filter_map(|s| {
			let states: Vec<u32> = s
				.states
				.iter()
				.map(|state| match table {
					Some(t) => t.get(*state as usize).copied().unwrap_or(air),
					None => *state,
				})
				.collect();
			if states.iter().all(|state| *state == air) && s.biomes.iter().all(|b| b == PLAINS) {
				return None;
			}
			Some(Payload::Compound(vec![
				Entry::new("Y", s.y),
				Entry::new("block_states", block_states(latest, &states)),
				Entry::new("biomes", biome_container(&s.biomes)),
			]))
		})
		.collect();
	Ok(Entry::new(
		"",
		vec![
			Entry::new("DataVersion", data_version),
			Entry::new("xPos", source.x),
			Entry::new("yPos", ordered.first().map_or(0, |s| s.y as i32)),
			Entry::new("zPos", source.z),
			Entry::new("Status", "minecraft:full"),
			Entry::new("LastUpdate", 0i64),
			Entry::new("InhabitedTime", 0i64),
			Entry::new("isLightOn", 0i8),
			Entry::new("sections", List::of(sections)),
			Entry::new("block_entities", List::of(Vec::new())),
			Entry::new("Heightmaps", Vec::<Entry>::new()),
		],
	))
}

fn palette_of<T: Clone + Eq + std::hash::Hash>(values: &[T]) -> (Vec<T>, Vec<u32>) {
	let mut palette = Vec::new();
	let mut index: HashMap<T, u32> = HashMap::new();
	let indices = values
		.iter()
		.map(|v| {
			*index.entry(v.clone()).or_insert_with(|| {
				palette.push(v.clone());
				palette.len() as u32 - 1
			})
		})
		.collect();
	(palette, indices)
}

fn packed(indices: Vec<u32>, bits: u32) -> Payload {
	Payload::LongArray(
		pack(indices.into_iter(), bits, false)
			.into_iter()
			.map(|l| l as i64)
			.collect(),
	)
}

fn block_states(blocks: &Blocks, states: &[u32]) -> Vec<Entry> {
	let (palette, indices) = palette_of(states);
	let entries = palette
		.iter()
		.map(|state| Payload::Compound(state_entry(blocks, *state)))
		.collect();
	let mut out = vec![Entry::new("palette", List::of(entries))];
	if palette.len() > 1 {
		out.push(Entry::new(
			"data",
			packed(indices, bits_for(palette.len()).max(4)),
		));
	}
	out
}

fn state_entry(blocks: &Blocks, state: u32) -> Vec<Entry> {
	let Some(block) = blocks.owner(state) else {
		return vec![Entry::new("Name", "minecraft:air")];
	};
	let mut out = vec![Entry::new("Name", format!("minecraft:{}", block.name))];
	let values = block.values(state);
	if !values.is_empty() {
		let props: Vec<Entry> = values.iter().map(|(k, v)| Entry::new(*k, *v)).collect();
		out.push(Entry::new("Properties", props));
	}
	out
}

fn biome_container(biomes: &[String]) -> Vec<Entry> {
	let (palette, indices) = palette_of(biomes);
	let names = palette.iter().map(|n| Payload::from(n.as_str())).collect();
	let mut out = vec![Entry::new("palette", List::of(names))];
	if palette.len() > 1 {
		out.push(Entry::new("data", packed(indices, bits_for(palette.len()))));
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn biome_renames_reach_latest_biomes() {
		let latest = registry::names(Version::LATEST, "biomes").unwrap();
		for (_, new) in BIOME_RENAMES {
			assert!(latest.id(new).is_some(), "{new}");
		}
	}
}
