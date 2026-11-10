use super::util::{bits_for, read_varint, write_varint};
use crate::gen::convert::Version;
use crate::pds::convert::{Error, Result};

#[derive(Clone, Copy)]
pub struct Format {
	pub lengths: bool,
	pub fluids: bool,
	pub block_states: u32,
	pub biomes: u32,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
	Blocks,
	Biomes,
}

impl Kind {
	fn entries(self) -> usize {
		match self {
			Kind::Blocks => 4096,
			Kind::Biomes => 64,
		}
	}

	fn storage_bits(self, bits: u8, global: u8) -> u8 {
		match (self, bits) {
			(_, 0) => 0,
			(Kind::Blocks, 1..=4) => 4,
			(Kind::Blocks, 5..=8) | (Kind::Biomes, 1..=3) => bits,
			_ => global,
		}
	}

	fn indirect_limit(self) -> u8 {
		match self {
			Kind::Blocks => 8,
			Kind::Biomes => 3,
		}
	}
}

pub struct Section {
	pub count: i16,
	pub fluids: i16,
	pub blocks: Vec<u32>,
	pub biomes: Vec<u32>,
}

fn long_count(entries: usize, bits: u8) -> usize {
	match bits {
		0 => 0,
		b => entries.div_ceil(64 / b as usize),
	}
}

fn bad(what: &str) -> Error {
	Error::Encoding(format!("chunk data: {what}"))
}

fn read_container(data: &[u8], at: &mut usize, kind: Kind, f: Format) -> Result<Vec<u32>> {
	let bits = *data.get(*at).ok_or_else(|| bad("truncated"))?;
	*at += 1;
	let global = bits_for(match kind {
		Kind::Blocks => f.block_states,
		Kind::Biomes => f.biomes,
	});
	let palette: Option<Vec<u32>> = if bits == 0 {
		Some(vec![read_varint(data, at)? as u32])
	} else if bits <= kind.indirect_limit() {
		let n = read_varint(data, at)? as usize;
		Some(
			(0..n)
				.map(|_| read_varint(data, at).map(|v| v as u32))
				.collect::<Result<_>>()?,
		)
	} else {
		None
	};
	let mut storage = kind.storage_bits(bits, if f.lengths { bits } else { global });
	let longs = if f.lengths {
		let n = read_varint(data, at)? as usize;
		if n != long_count(kind.entries(), storage) && bits != 0 {
			if n == long_count(kind.entries(), bits) {
				storage = bits;
			} else {
				return Err(bad("unexpected long array length"));
			}
		}
		n
	} else {
		long_count(kind.entries(), storage)
	};
	let end = *at + longs * 8;
	let raw = data
		.get(*at..end)
		.ok_or_else(|| bad("truncated long array"))?;
	*at = end;
	if bits == 0 {
		return Ok(vec![
			palette.expect("single value palette")[0];
			kind.entries()
		]);
	}
	let per = 64 / storage as usize;
	let mask = (1u64 << storage) - 1;
	let mut out = Vec::with_capacity(kind.entries());
	for i in 0..kind.entries() {
		let at = i / per * 8;
		let word = u64::from_be_bytes(raw[at..at + 8].try_into().expect("8 bytes"));
		let v = ((word >> ((i % per) * storage as usize)) & mask) as u32;
		out.push(match &palette {
			Some(p) => *p
				.get(v as usize)
				.ok_or_else(|| bad("palette index out of range"))?,
			None => v,
		});
	}
	Ok(out)
}

fn write_container(out: &mut Vec<u8>, values: &[u32], kind: Kind, f: Format) {
	let mut palette: Vec<u32> = Vec::new();
	for v in values {
		if !palette.contains(v) {
			palette.push(*v);
			if palette.len() > 1 << kind.indirect_limit() {
				break;
			}
		}
	}
	let global = bits_for(match kind {
		Kind::Blocks => f.block_states,
		Kind::Biomes => f.biomes,
	})
	.max(kind.indirect_limit() + 1);
	if palette.len() == 1 {
		out.push(0);
		write_varint(out, palette[0] as i32);
		if f.lengths {
			write_varint(out, 0);
		}
		return;
	}
	let indirect = palette.len() <= 1 << kind.indirect_limit();
	let bits = if indirect {
		kind.storage_bits(bits_for(palette.len() as u32).max(1), global)
	} else {
		global
	};
	out.push(bits);
	if indirect {
		write_varint(out, palette.len() as i32);
		for p in &palette {
			write_varint(out, *p as i32);
		}
	}
	let per = 64 / bits as usize;
	let longs = long_count(values.len(), bits);
	if f.lengths {
		write_varint(out, longs as i32);
	}
	let mut words = vec![0u64; longs];
	for (i, v) in values.iter().enumerate() {
		let v = if indirect {
			palette
				.iter()
				.position(|p| p == v)
				.expect("value in palette") as u64
		} else {
			*v as u64
		};
		words[i / per] |= v << ((i % per) * bits as usize);
	}
	for w in words {
		out.extend_from_slice(&w.to_be_bytes());
	}
}

fn read_i16(data: &[u8], at: &mut usize) -> Result<i16> {
	let v = i16::from_be_bytes(
		data.get(*at..*at + 2)
			.ok_or_else(|| bad("truncated section"))?
			.try_into()
			.expect("2 bytes"),
	);
	*at += 2;
	Ok(v)
}

pub fn read_sections(data: &[u8], f: Format) -> Result<Vec<Section>> {
	let mut at = 0;
	let mut out = Vec::new();
	while at < data.len() {
		let count = read_i16(data, &mut at)?;
		let fluids = if f.fluids {
			read_i16(data, &mut at)?
		} else {
			0
		};
		let blocks = read_container(data, &mut at, Kind::Blocks, f)?;
		let biomes = read_container(data, &mut at, Kind::Biomes, f)?;
		out.push(Section {
			count,
			fluids,
			blocks,
			biomes,
		});
	}
	Ok(out)
}

pub fn write_sections(sections: &[Section], f: Format) -> Vec<u8> {
	let mut out = Vec::new();
	for s in sections {
		out.extend_from_slice(&s.count.to_be_bytes());
		if f.fluids {
			out.extend_from_slice(&s.fluids.to_be_bytes());
		}
		write_container(&mut out, &s.blocks, Kind::Blocks, f);
		write_container(&mut out, &s.biomes, Kind::Biomes, f);
	}
	out
}

pub fn read_biomes(data: &[u8], f: Format) -> Result<Vec<Vec<u32>>> {
	let mut at = 0;
	let mut out = Vec::new();
	while at < data.len() {
		out.push(read_container(data, &mut at, Kind::Biomes, f)?);
	}
	Ok(out)
}

pub fn write_biomes(sections: &[Vec<u32>], f: Format) -> Vec<u8> {
	let mut out = Vec::new();
	for s in sections {
		write_container(&mut out, s, Kind::Biomes, f);
	}
	out
}

pub fn format(version: Version, biomes: u32) -> Format {
	Format {
		lengths: version == Version::V1_21_4,
		fluids: version == Version::V26_1,
		block_states: crate::registry::blocks(version).map_or(1, |b| b.states()),
		biomes,
	}
}

const FLUID_BLOCKS: &[&str] = &[
	"water",
	"lava",
	"seagrass",
	"tall_seagrass",
	"kelp",
	"kelp_plant",
	"bubble_column",
];

fn fluid_states(version: Version) -> &'static [bool] {
	static CACHE: std::sync::OnceLock<Vec<bool>> = std::sync::OnceLock::new();
	CACHE.get_or_init(|| {
		let Some(blocks) = crate::registry::blocks(version) else {
			return Vec::new();
		};
		(0..blocks.states())
			.map(|s| {
				blocks.owner(s).is_some_and(|b| {
					FLUID_BLOCKS.contains(&b.name.as_str())
						|| b.values(s)
							.iter()
							.any(|(k, v)| *k == "waterlogged" && *v == "true")
				})
			})
			.collect()
	})
}

fn fluid_count(count: i16, blocks: &[u32], fluid: &dyn Fn(u32) -> bool) -> i16 {
	if count == 0 {
		0
	} else {
		blocks.iter().filter(|b| fluid(**b)).count() as i16
	}
}

pub fn fluid_count_26_1(count: i16, blocks: &[u32]) -> i16 {
	fluid_count(count, blocks, &|s| {
		fluid_states(Version::V26_1)
			.get(s as usize)
			.copied()
			.unwrap_or(false)
	})
}

pub fn step(from: Version, to: Version, data: &[u8], biomes: u32) -> Result<Vec<u8>> {
	let fluid = |s: u32| {
		fluid_states(Version::V26_1)
			.get(s as usize)
			.copied()
			.unwrap_or(false)
	};
	let table = crate::registry::state_table(from, to).unwrap_or_default();
	convert(
		data,
		format(from, biomes),
		format(to, biomes),
		|s| table.get(s as usize).copied().unwrap_or(0),
		(to == Version::V26_1).then_some(&fluid as &dyn Fn(u32) -> bool),
	)
}

pub fn biomes_step(from: Version, to: Version, data: &[u8], biomes: u32) -> Result<Vec<u8>> {
	Ok(write_biomes(
		&read_biomes(data, format(from, biomes))?,
		format(to, biomes),
	))
}

pub fn convert(
	data: &[u8],
	from: Format,
	to: Format,
	remap: impl Fn(u32) -> u32,
	fluid: Option<&dyn Fn(u32) -> bool>,
) -> Result<Vec<u8>> {
	let mut sections = read_sections(data, from)?;
	for s in &mut sections {
		for b in &mut s.blocks {
			*b = remap(*b);
		}
		if let Some(fluid) = fluid {
			s.fluids = fluid_count(s.count, &s.blocks, fluid);
		}
	}
	Ok(write_sections(&sections, to))
}
