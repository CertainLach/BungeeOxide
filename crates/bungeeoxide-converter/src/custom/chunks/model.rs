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
use crate::gen::convert::Version;
