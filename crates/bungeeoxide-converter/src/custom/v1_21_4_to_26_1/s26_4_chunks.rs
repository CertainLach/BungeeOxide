use super::{chunk, world};
use crate::Session;
use crate::generated::convert::{Version, v26_3 as a, v26_4_snapshot_10 as b};
use crate::pds::convert::Result;

pub use super::chunk::fluid_count_in;

const OLD: Version = Version::V26_3;
const NEW: Version = Version::V26_4_snapshot_10;

pub fn late_chunk_biomes_up_26_4(
	v: &a::play_to_client_packet_chunk_biomes_items::Ty,
	cx: &mut Session,
) -> Result<Vec<u8>> {
	chunk::biomes_step(OLD, NEW, &v.data, world::biomes(cx))
}

pub fn late_chunk_biomes_down_26_3(
	v: &b::play_to_client_packet_chunk_biomes_items::Ty,
	cx: &mut Session,
) -> Result<Vec<u8>> {
	chunk::biomes_step(NEW, OLD, &v.data, world::biomes(cx))
}

pub fn session_biome_bits(cx: &mut Session) -> Option<u32> {
	world::world(cx)
		.biomes
		.map(|n| super::util::bits_for(n) as u32)
}

#[cfg(test)]
mod tests {
	use super::super::util::{decode, read_varint};
	use super::chunk::{Section, format, read_biomes, read_sections, write_biomes, write_sections};
	use super::*;
	use crate::custom::chunks::expand_cells;

	const REGISTRY: u32 = 68;

	fn cell(x: usize, y: usize, z: usize) -> usize {
		y << 4 | z << 2 | x
	}

	fn block(x: usize, y: usize, z: usize) -> usize {
		y << 8 | z << 4 | x
	}

	fn old_section(names: &[&str], biomes: Vec<u32>) -> Section {
		let table = crate::registry::blocks(OLD).unwrap();
		let palette: Vec<u32> = names
			.iter()
			.map(|n| table.parse_state(n).unwrap())
			.collect();
		let blocks: Vec<u32> = (0..4096).map(|i| palette[i * 7 % palette.len()]).collect();
		Section {
			count: 4096,
			fluids: fluid_count_in(OLD, 4096, &blocks),
			blocks,
			biomes,
		}
	}

	fn position() -> a::packed_chunk_pos::Ty {
		a::packed_chunk_pos::Ty { x: 3, z: -4 }
	}

	#[test]
	fn sections_round_trip_through_block_biomes() {
		let names = [
			"minecraft:stone",
			"minecraft:water[level=0]",
			"minecraft:kelp[age=3]",
			"minecraft:deepslate[axis=x]",
			"minecraft:firefly_bush",
		];
		let sections = vec![
			old_section(&names, (0..64).map(|c| c as u32).collect()),
			old_section(&names[..2], (0..64).map(|c| [4, 9, 4, 2][c % 4]).collect()),
		];
		let data = write_sections(&sections, format(OLD, REGISTRY));
		let up = chunk::step(OLD, NEW, &data, REGISTRY).unwrap();
		let read = read_sections(&up, format(NEW, REGISTRY)).unwrap();
		let (old_table, new_table) = (
			crate::registry::blocks(OLD).unwrap(),
			crate::registry::blocks(NEW).unwrap(),
		);
		assert_eq!(read.len(), 2);
		for (s, r) in sections.iter().zip(&read) {
			assert_eq!(r.biomes.len(), 4096);
			for i in 0..4096 {
				let (x, y, z) = (i & 15, i >> 8, i >> 4 & 15);
				assert_eq!(
					r.biomes[block(x, y, z)],
					s.biomes[cell(x >> 2, y >> 2, z >> 2)]
				);
			}
			for (o, n) in s.blocks.iter().zip(&r.blocks) {
				assert_eq!(old_table.describe(*o), new_table.describe(*n));
			}
			assert_eq!(r.count, 4096);
			assert_eq!(r.fluids, fluid_count_in(NEW, 4096, &r.blocks));
			assert_eq!(r.fluids, s.fluids);
		}
		assert_eq!(read[0].biomes[block(5, 9, 14)], cell(1, 2, 3) as u32);
		assert_ne!(sections[0].blocks, read[0].blocks);
		assert!(read[0].fluids > 0);
		assert_eq!(chunk::step(NEW, OLD, &up, REGISTRY).unwrap(), data);
	}

	#[test]
	fn block_biome_palettes_use_26_4_sizes() {
		let cases: [(usize, u8, usize); 11] = [
			(1, 0, 0),
			(2, 1, 64),
			(3, 2, 128),
			(5, 3, 196),
			(16, 4, 256),
			(17, 5, 342),
			(33, 6, 410),
			(65, 7, 456),
			(129, 8, 512),
			(256, 8, 512),
			(257, 9, 586),
		];
		let f = format(NEW, 300);
		for (distinct, bits, longs) in cases {
			let values: Vec<u32> = (0..4096).map(|i| (i % distinct) as u32).collect();
			let data = write_biomes(std::slice::from_ref(&values), f);
			assert_eq!(data[0], bits, "{distinct} distinct biomes");
			let mut at = 1;
			let palette = match bits {
				0 => 1,
				1..=8 => read_varint(&data, &mut at).unwrap() as usize,
				_ => 0,
			};
			if bits <= 8 {
				assert_eq!(palette, distinct);
			}
			for _ in 0..palette {
				read_varint(&data, &mut at).unwrap();
			}
			assert_eq!(data.len() - at, longs * 8, "{distinct} distinct biomes");
			assert_eq!(read_biomes(&data, f).unwrap(), vec![values]);
		}
	}

	#[test]
	fn block_biomes_are_indexed_y_z_x() {
		let mut values = vec![4u32; 4096];
		values[block(1, 2, 3)] = 9;
		let data = write_biomes(&[values], format(NEW, REGISTRY));
		assert_eq!(data[..4], [1, 2, 4, 9]);
		assert_eq!(data.len(), 4 + 64 * 8);
		let long = |k: usize| u64::from_be_bytes(data[4 + k * 8..12 + k * 8].try_into().unwrap());
		assert_eq!(long(561 / 64), 1 << (561 % 64));
		assert!((0..64).filter(|k| *k != 561 / 64).all(|k| long(k) == 0));
	}

	#[test]
	fn chunk_biomes_convert_both_ways() {
		let mut cx = Session::new();
		let cells: Vec<Vec<u32>> = vec![
			(0..64).map(|c| (c * 5 % 64) as u32).collect(),
			(0..64).map(|c| (c % 3) as u32).collect(),
			vec![12; 64],
		];
		let old = a::play_to_client_packet_chunk_biomes_items::Ty {
			position: position(),
			data: write_biomes(&cells, format(OLD, world::biomes(&mut cx))),
		};
		let up = late_chunk_biomes_up_26_4(&old, &mut cx).unwrap();
		let fine = read_biomes(&up, format(NEW, world::biomes(&mut cx))).unwrap();
		assert_eq!(
			fine,
			cells.iter().map(|c| expand_cells(c)).collect::<Vec<_>>()
		);
		assert_eq!(fine[0][block(5, 9, 14)], cells[0][cell(1, 2, 3)]);
		assert_eq!(up[0], 6);
		assert!(up.ends_with(&[0, 12]));
		let new = b::play_to_client_packet_chunk_biomes_items::Ty {
			position: position(),
			data: up,
		};
		assert_eq!(
			late_chunk_biomes_down_26_3(&new, &mut cx).unwrap(),
			old.data
		);
	}

	#[test]
	fn chunk_biomes_down_take_the_cell_majority() {
		let mut cx = Session::new();
		let mut values = vec![1u32; 4096];
		for k in 0..64 {
			let (x, y, z) = (k & 3, k >> 4, k >> 2 & 3);
			values[block(4 + x, 8 + y, 12 + z)] = if x >= 2 && y >= 2 { 6 } else { 5 };
			values[block(x, y, z)] = if x >= 2 { 8 } else { 7 };
		}
		let new = b::play_to_client_packet_chunk_biomes_items::Ty {
			position: position(),
			data: write_biomes(&[values], format(NEW, world::biomes(&mut cx))),
		};
		let down = late_chunk_biomes_down_26_3(&new, &mut cx).unwrap();
		let cells = read_biomes(&down, format(OLD, world::biomes(&mut cx))).unwrap();
		assert_eq!(cells.len(), 1);
		assert_eq!(cells[0][cell(1, 2, 3)], 5);
		assert_eq!(cells[0][cell(0, 0, 0)], 8);
		assert!(
			(0..64)
				.filter(|c| *c != cell(1, 2, 3) && *c != cell(0, 0, 0))
				.all(|c| cells[0][c] == 1)
		);
	}

	#[test]
	fn virtual_columns_write_block_biomes() {
		use crate::custom::chunks::{self, Biomes, Column, Packet};
		let stone = crate::registry::blocks(NEW)
			.unwrap()
			.parse_state("minecraft:stone")
			.unwrap();
		let mut col = Column::new(1, 2, true, NEW);
		col.sections = vec![Some(chunks::Section::filled(stone, None)), None];
		col.biomes = Biomes::Cells((0..128).map(|c| (c % 7) as i32).collect());
		col.heightmaps = Some(Vec::new());
		let packet = chunks::encode_chunk(NEW, &col).unwrap().pop().unwrap();
		let mut at = 0;
		read_varint(&packet, &mut at).unwrap();
		let p = decode(
			&packet[at..],
			b::play_to_client_packet_map_chunk::deserialize,
		)
		.unwrap();
		let read = read_sections(&p.chunk_data, format(NEW, REGISTRY)).unwrap();
		assert_eq!(read.len(), 2);
		for (k, s) in read.iter().enumerate() {
			let cells: Vec<u32> = (0..64).map(|c| ((k * 64 + c) % 7) as u32).collect();
			assert_eq!(s.biomes, expand_cells(&cells));
		}
		assert_eq!(read[0].blocks, vec![stone; 4096]);
		assert_eq!((read[1].count, read[1].fluids), (0, 0));
		match chunks::decode(NEW, &packet) {
			Ok(Some(Packet::Chunk(c))) => assert_eq!(c.biomes, col.biomes),
			_ => panic!("26.4 chunk does not decode"),
		}
	}

	#[test]
	fn virtual_chunks_down_use_the_session_biome_registry() {
		use crate::custom::chunks::{self, Biomes, Column};
		use crate::generated::convert::{Root, convert};
		let mut cx = Session::new();
		world::world(&mut cx).biomes = Some(200);
		let stone = crate::registry::blocks(NEW)
			.unwrap()
			.parse_state("minecraft:stone")
			.unwrap();
		let cells: Vec<u32> = (0..64).map(|c| (c % 12 + 150) as u32).collect();
		let mut col = Column::new(0, 0, true, NEW);
		col.sections = vec![Some(chunks::Section::filled(stone, None))];
		col.biomes = Biomes::Cells(cells.iter().map(|c| *c as i32).collect());
		col.heightmaps = Some(Vec::new());
		let packet = chunks::encode_chunk(NEW, &col).unwrap().pop().unwrap();
		let out = convert(Root::PlayToClient, NEW, OLD, &packet, &mut cx).unwrap();
		assert_eq!(out.len(), 1);
		let mut at = 0;
		read_varint(&out[0], &mut at).unwrap();
		let p = decode(
			&out[0][at..],
			a::play_to_client_packet_map_chunk::deserialize,
		)
		.unwrap();
		assert_eq!(p.chunk_data[6], 8);
		let read = read_sections(&p.chunk_data, format(OLD, 200)).unwrap();
		assert_eq!(read.len(), 1);
		assert_eq!(read[0].biomes, cells);
	}
}
