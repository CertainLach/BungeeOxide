use bungeeoxide_converter::gen::convert::{convert, Root};
use bungeeoxide_converter::gen::convert::{v1_12_2, v1_8};
use bungeeoxide_converter::pds::{Input, Output};
use bungeeoxide_converter::Session;
use bungeeoxide_converter::Version;
use v1_12_2::{play_to_client_packet as newest, play_to_server_packet as newest_server};
use v1_8::{
	play_to_client_packet as oldest, play_to_client_packet_map_chunk as chunk_1_8,
	play_to_server_packet as oldest_server,
};

fn encode_1_8(p: &oldest::Ty) -> Vec<u8> {
	let mut o = Output::new();
	oldest::serialize(p, &mut o).unwrap();
	o.into_inner()
}

fn up(p: &oldest::Ty, cx: &mut Session) -> Vec<newest::Ty> {
	convert(
		Root::PlayToClient,
		Version::V1_8,
		Version::V1_12_2,
		&encode_1_8(p),
		cx,
	)
	.unwrap()
	.iter()
	.map(|b| {
		let mut input = Input::new(b);
		let p = newest::deserialize(&mut input).unwrap();
		assert_eq!(input.remaining(), 0);
		p
	})
	.collect()
}

fn read_varint(data: &[u8], at: &mut usize) -> u32 {
	let mut v = 0;
	for shift in (0..35).step_by(7) {
		let b = data[*at];
		*at += 1;
		v |= ((b & 0x7f) as u32) << shift;
		if b & 0x80 == 0 {
			break;
		}
	}
	v
}

#[test]
fn chunk_sections_keep_block_states() {
	let state = |s: usize, i: usize| ((s * 7 + i * 13) % 0x1000) as u16 | 0x10;
	let bitmap: u16 = 0b1000_0000_0010_0001;
	let sections: Vec<usize> = (0..16).filter(|s| bitmap & (1 << s) != 0).collect();
	let mut data = Vec::new();
	for &s in &sections {
		for i in 0..4096 {
			data.extend_from_slice(&state(s, i).to_le_bytes());
		}
	}
	for &s in &sections {
		data.extend(std::iter::repeat(s as u8).take(2048));
	}
	for &s in &sections {
		data.extend(std::iter::repeat(0xf0 | s as u8).take(2048));
	}
	data.extend((0..256).map(|b| b as u8));
	let packet = oldest::Ty::MapChunk(chunk_1_8::Ty {
		x: 3,
		z: -7,
		ground_up: true,
		bit_map: bitmap,
		chunk_data: data,
	});
	let out = up(&packet, &mut Session::new());
	let [newest::Ty::MapChunk(c)] = out.as_slice() else {
		panic!("expected one chunk, got {out:?}");
	};
	assert_eq!(
		(c.x, c.z, c.ground_up, c.bit_map),
		(3, -7, true, bitmap as i32)
	);
	assert!(c.block_entities.is_empty());
	let d = &c.chunk_data;
	let mut at = 0;
	for &s in &sections {
		assert_eq!(d[at], 13);
		at += 1;
		assert_eq!(read_varint(d, &mut at), 0);
		let longs = read_varint(d, &mut at) as usize;
		let words: Vec<u64> = (0..longs)
			.map(|i| u64::from_be_bytes(d[at + i * 8..at + i * 8 + 8].try_into().unwrap()))
			.collect();
		at += longs * 8;
		for i in 0..4096 {
			let bit = i * 13;
			let (w, off) = (bit / 64, bit % 64);
			let mut v = words[w] >> off;
			if off + 13 > 64 {
				v |= words[w + 1] << (64 - off);
			}
			assert_eq!((v & 0x1fff) as u16, state(s, i), "section {s} block {i}");
		}
		assert!(d[at..at + 2048].iter().all(|b| *b == s as u8));
		at += 2048;
		assert!(d[at..at + 2048].iter().all(|b| *b == 0xf0 | s as u8));
		at += 2048;
	}
	assert_eq!(
		&d[at..],
		(0..256).map(|b| b as u8).collect::<Vec<_>>().as_slice()
	);
}

#[test]
fn empty_full_chunk_unloads() {
	let packet = oldest::Ty::MapChunk(chunk_1_8::Ty {
		x: 1,
		z: 2,
		ground_up: true,
		bit_map: 0,
		chunk_data: vec![0; 256],
	});
	let out = up(&packet, &mut Session::new());
	assert!(
		matches!(out.as_slice(), [newest::Ty::UnloadChunk(u)] if (u.chunk_x, u.chunk_z) == (1, 2))
	);
}

#[test]
fn positions_and_teleports() {
	use v1_8::play_to_client_packet_position as pos;
	use v1_8::play_to_client_packet_rel_entity_move as rel;
	let mut cx = Session::new();
	let p = oldest::Ty::Position(pos::Ty {
		x: 1.5,
		y: 64.0,
		z: -2.25,
		yaw: 90.0,
		pitch: 0.0,
		flags: 0,
	});
	let first = up(&p, &mut cx);
	let second = up(&p, &mut cx);
	let id = |o: &[newest::Ty]| match o {
		[newest::Ty::Position(p)] => p.teleport_id,
		o => panic!("{o:?}"),
	};
	assert_ne!(id(&first), id(&second));
	let moved = up(
		&oldest::Ty::RelEntityMove(rel::Ty {
			entity_id: 5,
			d_x: 32,
			d_y: -4,
			d_z: 0,
			on_ground: true,
		}),
		&mut cx,
	);
	assert!(
		matches!(moved.as_slice(), [newest::Ty::RelEntityMove(m)] if (m.d_x, m.d_y, m.d_z) == (4096, -512, 0))
	);
}

#[test]
fn keep_alive_round_trips_through_every_step() {
	let mut cx = Session::new();
	let mut o = Output::new();
	newest_server::serialize(&newest_server::Ty::KeepAlive(1234), &mut o).unwrap();
	let down = convert(
		Root::PlayToServer,
		Version::V1_12_2,
		Version::V1_8,
		&o.into_inner(),
		&mut cx,
	)
	.unwrap();
	let [bytes] = down.as_slice() else {
		panic!("{down:?}")
	};
	assert_eq!(
		oldest_server::deserialize(&mut Input::new(bytes)).unwrap(),
		oldest_server::Ty::KeepAlive(1234)
	);
}
