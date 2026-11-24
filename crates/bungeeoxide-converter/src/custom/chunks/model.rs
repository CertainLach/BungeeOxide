pub const BLOCKS: usize = 4096;
pub const NIBBLES: usize = 2048;
pub const CELLS: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
	pub states: Vec<u32>,
	pub count: Option<i16>,
}

impl Section {
	pub fn new(states: Vec<u32>, count: Option<i16>) -> Self {
		Self { states, count }
	}

	pub fn filled(state: u32, count: Option<i16>) -> Self {
		Self {
			states: vec![state; BLOCKS],
			count,
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum Nibbles {
	Missing,
	Empty,
	Data(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Light {
	pub sky: Vec<Nibbles>,
	pub block: Vec<Nibbles>,
	pub has_sky: bool,
	pub trust: bool,
}

impl Light {
	pub fn new(sections: usize, has_sky: bool) -> Self {
		Self {
			sky: vec![Nibbles::Missing; sections + 2],
			block: vec![Nibbles::Missing; sections + 2],
			has_sky,
			trust: false,
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum Biomes {
	Absent,
	Flat(Vec<i32>),
	Cells(Vec<i32>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeightKind {
	Named(String),
	Id(i32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Heightmap {
	pub kind: HeightKind,
	pub data: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockEntity {
	pub x: i32,
	pub y: i32,
	pub z: i32,
	pub kind: Option<i32>,
	pub name: String,
	pub nbt: Option<bungeeoxide_nbt::Tag>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
	pub x: i32,
	pub z: i32,
	pub full: bool,
	pub at: Version,
	pub space: Version,
	pub sections: Vec<Option<Section>>,
	pub light: Option<Light>,
	pub biomes: Biomes,
	pub heightmaps: Option<Vec<Heightmap>>,
	pub entities: Vec<BlockEntity>,
	pub biome_bits: Option<u32>,
}

impl Column {
	pub fn new(x: i32, z: i32, full: bool, version: Version) -> Self {
		Self {
			x,
			z,
			full,
			at: version,
			space: version,
			sections: Vec::new(),
			light: None,
			biomes: Biomes::Absent,
			heightmaps: None,
			entities: Vec::new(),
			biome_bits: None,
		}
	}

	pub fn block(&self, x: usize, y: usize, z: usize) -> u32 {
		match self.sections.get(y >> 4) {
			Some(Some(s)) => s.states[(y & 15) << 8 | z << 4 | x],
			_ => 0,
		}
	}
}

fn cell_block(cell: usize, offset: usize) -> usize {
	(cell >> 4 << 2 | offset >> 4) << 8
		| (cell >> 2 & 3) << 6
		| (offset >> 2 & 3) << 4
		| (cell & 3) << 2
		| (offset & 3)
}

pub fn expand_cells<T: Copy>(cells: &[T]) -> Vec<T> {
	(0..BLOCKS)
		.map(|i| cells[(i >> 10) << 4 | (i >> 6 & 3) << 2 | (i >> 2 & 3)])
		.collect()
}

pub fn sample_cells<T: Copy + PartialEq>(blocks: &[T]) -> Vec<T> {
	(0..CELLS)
		.map(|c| {
			let center = blocks[cell_block(c, 0b10_10_10)];
			let mut counts: Vec<(T, usize)> = Vec::with_capacity(4);
			for k in 0..CELLS {
				let v = blocks[cell_block(c, k)];
				match counts.iter_mut().find(|(x, _)| *x == v) {
					Some((_, n)) => *n += 1,
					None => counts.push((v, 1)),
				}
			}
			let start = counts.iter().copied().find(|(x, _)| *x == center);
			counts
				.iter()
				.copied()
				.fold(start.unwrap_or((center, 0)), |best, c| {
					if c.1 > best.1 { c } else { best }
				})
				.0
		})
		.collect()
}
use crate::generated::convert::Version;
