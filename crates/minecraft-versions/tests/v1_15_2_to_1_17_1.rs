use minecraft_versions::gen::convert::{convert, index_of, Root};
use minecraft_versions::gen::convert::{v1_15_2, v1_16, v1_16_2, v1_17, v1_17_1};
use minecraft_versions::pds::{Input, Output, Result};
use minecraft_versions::registry;
use minecraft_versions::Session;
use nbt::gen::entry::Ty as Entry;
use nbt::gen::list_0::Ty as Items;
use nbt::gen::payload::Ty as P;
use nbt::CompoundExt;

type C15 = v1_15_2::play_to_client_packet::Ty;
type C16 = v1_16::play_to_client_packet::Ty;
type C162 = v1_16_2::play_to_client_packet::Ty;
type C17 = v1_17::play_to_client_packet::Ty;
type C171 = v1_17_1::play_to_client_packet::Ty;

fn one<T: std::fmt::Debug>(v: Vec<T>) -> T {
	let [x]: [T; 1] = v.try_into().unwrap();
	x
}

fn at(version: &str) -> usize {
	index_of(version).unwrap()
}

fn enc<T>(v: &T, f: fn(&T, &mut Output) -> Result<()>) -> Vec<u8> {
	let mut o = Output::new();
	f(v, &mut o).unwrap();
	o.into_inner()
}

fn dec<T>(b: &[u8], f: fn(&mut Input) -> Result<T>) -> T {
	let mut i = Input::new(b);
	let v = f(&mut i).unwrap();
	assert_eq!(i.remaining(), 0, "leftover bytes");
	v
}

fn run(root: Root, from: &str, to: &str, data: &[u8], cx: &mut Session) -> Vec<Vec<u8>> {
	convert(root, at(from), at(to), data, cx).unwrap()
}

fn up_15<T>(p: &C15, to: &str, cx: &mut Session, f: fn(&mut Input) -> Result<T>) -> Vec<T> {
	run(
		Root::PlayToClient,
		"1.15.2",
		to,
		&enc(p, v1_15_2::play_to_client_packet::serialize),
		cx,
	)
	.iter()
	.map(|b| dec(b, f))
	.collect()
}

fn down_17(p: &C17, cx: &mut Session) -> Vec<C162> {
	run(
		Root::PlayToClient,
		"1.17",
		"1.16.2",
		&enc(p, v1_17::play_to_client_packet::serialize),
		cx,
	)
	.iter()
	.map(|b| dec(b, v1_16_2::play_to_client_packet::deserialize))
	.collect()
}

fn get<'a>(c: &'a [Entry], name: &str) -> Option<&'a P> {
	c.field(name)
}

fn compound(p: &P) -> &Vec<Entry> {
	match p {
		P::Compound(c) => c,
		p => panic!("not a compound: {p:?}"),
	}
}

fn compounds(p: &P) -> &Vec<Vec<Entry>> {
	match p {
		P::List(l) => match &l.items {
			Items::Compound(c) => c,
			i => panic!("not a compound list: {i:?}"),
		},
		p => panic!("not a list: {p:?}"),
	}
}

fn string(p: Option<&P>) -> &str {
	match p {
		Some(P::String(s)) => s,
		p => panic!("not a string: {p:?}"),
	}
}

fn int(p: Option<&P>) -> i32 {
	match p {
		Some(P::Int(v)) => *v,
		p => panic!("not an int: {p:?}"),
	}
}

fn state(version: &str, text: &str) -> u32 {
	registry::blocks(version)
		.unwrap()
		.parse_state(text)
		.unwrap()
}

fn describe(version: &str, s: u32) -> String {
	registry::blocks(version).unwrap().describe(s).unwrap()
}

fn item(version: &str, name: &str) -> i32 {
	registry::names(version, "items").unwrap().id(name).unwrap() as i32
}

fn entity(version: &str, name: &str) -> i32 {
	registry::names(version, "entities")
		.unwrap()
		.id(name)
		.unwrap() as i32
}

fn varint(out: &mut Vec<u8>, v: i32) {
	let mut v = v as u32;
	loop {
		if v & !0x7f == 0 {
			out.push(v as u8);
			return;
		}
		out.push((v as u8 & 0x7f) | 0x80);
		v >>= 7;
	}
}

fn read_varint(d: &[u8], at: &mut usize) -> i32 {
	let mut v = 0u32;
	for shift in (0..35).step_by(7) {
		let b = d[*at];
		*at += 1;
		v |= ((b & 0x7f) as u32) << shift;
		if b & 0x80 == 0 {
			break;
		}
	}
	v as i32
}

fn pack(values: &[u32], bits: usize, spanning: bool) -> Vec<i64> {
	let mut out = vec![
		0u64;
		if spanning {
			(values.len() * bits).div_ceil(64)
		} else {
			values.len().div_ceil(64 / bits)
		}
	];
	for (i, v) in values.iter().enumerate() {
		let v = *v as u64;
		if spanning {
			let (w, off) = (i * bits / 64, i * bits % 64);
			out[w] |= v << off;
			if off + bits > 64 {
				out[w + 1] |= v >> (64 - off);
			}
		} else {
			let per = 64 / bits;
			out[i / per] |= v << ((i % per) * bits);
		}
	}
	out.into_iter().map(|v| v as i64).collect()
}

fn unpack(longs: &[i64], bits: usize, count: usize) -> Vec<u32> {
	let per = 64 / bits;
	(0..count)
		.map(|i| ((longs[i / per] as u64 >> ((i % per) * bits)) & ((1 << bits) - 1)) as u32)
		.collect()
}

fn write_section(
	out: &mut Vec<u8>,
	states: &[u32],
	palette: Option<&[u32]>,
	bits: usize,
	spanning: bool,
) {
	out.extend_from_slice(&(states.iter().filter(|s| **s != 0).count() as i16).to_be_bytes());
	out.push(bits as u8);
	let values: Vec<u32> = match palette {
		Some(p) => {
			varint(out, p.len() as i32);
			for s in p {
				varint(out, *s as i32);
			}
			states
				.iter()
				.map(|s| p.iter().position(|x| x == s).unwrap() as u32)
				.collect()
		}
		None => states.to_vec(),
	};
	let longs = pack(&values, bits, spanning);
	varint(out, longs.len() as i32);
	for l in longs {
		out.extend_from_slice(&l.to_be_bytes());
	}
}

fn read_section(d: &[u8], at: &mut usize, global_bits: usize) -> (i16, Option<Vec<u32>>, Vec<u32>) {
	let count = i16::from_be_bytes([d[*at], d[*at + 1]]);
	let bits = d[*at + 2] as usize;
	*at += 3;
	let palette = (bits <= 8).then(|| {
		let n = read_varint(d, at);
		(0..n)
			.map(|_| read_varint(d, at) as u32)
			.collect::<Vec<_>>()
	});
	let n = read_varint(d, at) as usize;
	let longs: Vec<i64> = (0..n)
		.map(|i| i64::from_be_bytes(d[*at + i * 8..*at + i * 8 + 8].try_into().unwrap()))
		.collect();
	*at += n * 8;
	let bits = if palette.is_some() { bits } else { global_bits };
	assert_eq!(
		n,
		4096usize.div_ceil(64 / bits),
		"long count for {bits} bits"
	);
	let values = unpack(&longs, bits, 4096);
	let states = match &palette {
		Some(p) => values.iter().map(|v| p[*v as usize]).collect(),
		None => values,
	};
	(count, palette, states)
}

fn global_bits(version: &str) -> usize {
	let n = registry::blocks(version).unwrap().states() as usize;
	(usize::BITS - (n - 1).leading_zeros()) as usize
}

fn login_15(dimension: i32) -> C15 {
	C15::Login(v1_15_2::play_to_client_packet_login::Ty {
		entity_id: 7,
		game_mode: 1 | 8,
		dimension,
		hashed_seed: 42,
		max_players: 20,
		level_type: "flat".to_owned(),
		view_distance: 10,
		reduced_debug_info: false,
		enable_respawn_screen: true,
	})
}

#[test]
fn join_game_builds_codecs() {
	let mut cx = Session::new();
	let C16::Login(l) = one(up_15(
		&login_15(-1),
		"1.16",
		&mut cx,
		v1_16::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		(
			l.dimension.as_str(),
			l.world_name.as_str(),
			l.is_flat,
			l.previous_game_mode
		),
		("minecraft:the_nether", "minecraft:the_nether", true, 255)
	);
	let dims = compounds(get(compound(&l.dimension_codec.value), "dimension").unwrap());
	assert_eq!(dims.len(), 4);
	assert!(dims
		.iter()
		.any(|d| string(get(d, "name")) == "minecraft:the_nether"
			&& int(get(d, "logical_height")) == 128));
	assert_eq!(l.world_names.len(), 3);

	let C162::Login(l) = one(up_15(
		&login_15(-1),
		"1.16.2",
		&mut cx,
		v1_16_2::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert!(l.is_hardcore);
	assert_eq!((l.game_mode, l.max_players), (1, 20));
	let codec = compound(&l.dimension_codec.value);
	assert!(get(codec, "minecraft:worldgen/biome").is_some());
	let types = compounds(
		get(
			compound(get(codec, "minecraft:dimension_type").unwrap()),
			"value",
		)
		.unwrap(),
	);
	let nether = types
		.iter()
		.find(|t| string(get(t, "name")) == "minecraft:the_nether")
		.unwrap();
	assert_eq!(&l.dimension.value, get(nether, "element").unwrap());
	assert_eq!(
		string(get(compound(&l.dimension.value), "effects")),
		"minecraft:the_nether"
	);

	let C17::Login(l) = one(up_15(
		&login_15(0),
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	let dim = compound(&l.dimension.value);
	assert_eq!((int(get(dim, "min_y")), int(get(dim, "height"))), (0, 256));
	let types = compounds(
		get(
			compound(
				get(
					compound(&l.dimension_codec.value),
					"minecraft:dimension_type",
				)
				.unwrap(),
			),
			"value",
		)
		.unwrap(),
	);
	assert!(types
		.iter()
		.all(|t| int(get(compound(get(t, "element").unwrap()), "height")) == 256));
}

#[test]
fn respawn_reaches_every_version() {
	let mut cx = Session::new();
	let p = C15::Respawn(v1_15_2::play_to_client_packet_respawn::Ty {
		dimension: 1,
		hashed_seed: 5,
		gamemode: 0,
		level_type: "default".to_owned(),
	});
	let C16::Respawn(r) = one(up_15(
		&p,
		"1.16",
		&mut cx,
		v1_16::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		(r.dimension.as_str(), r.is_flat, r.copy_metadata),
		("minecraft:the_end", false, true)
	);
	let C17::Respawn(r) = one(up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	let dim = compound(&r.dimension.value);
	assert_eq!(string(get(dim, "effects")), "minecraft:the_end");
	assert_eq!(int(get(dim, "height")), 256);
	let back = run(
		Root::PlayToClient,
		"1.17",
		"1.15.2",
		&enc(&C17::Respawn(r), v1_17::play_to_client_packet::serialize),
		&mut cx,
	);
	let C15::Respawn(r) = one(back
		.iter()
		.map(|b| dec(b, v1_15_2::play_to_client_packet::deserialize))
		.collect::<Vec<_>>())
	else {
		panic!()
	};
	assert_eq!(r.dimension, 1);
}

#[test]
fn chat_and_login_success() {
	let mut cx = Session::new();
	let C16::Chat(c) = one(up_15(
		&C15::Chat(v1_15_2::play_to_client_packet_chat::Ty {
			message: "{\"text\":\"hi\"}".to_owned(),
			position: 1,
		}),
		"1.16",
		&mut cx,
		v1_16::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!((c.position, c.sender), (1, 0));
	let p =
		v1_15_2::login_to_client_packet::Ty::Success(v1_15_2::login_to_client_packet_success::Ty {
			uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".to_owned(),
			username: "Notch".to_owned(),
		});
	let out = run(
		Root::LoginToClient,
		"1.15.2",
		"1.17.1",
		&enc(&p, v1_15_2::login_to_client_packet::serialize),
		&mut cx,
	);
	let v1_17_1::login_to_client_packet::Ty::Success(s) =
		dec(&out[0], v1_17_1::login_to_client_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(s.uuid, 0x069a79f444e94726a5befca90e38aaf5);
}

fn chunk_15(full: bool) -> (C15, Vec<Vec<u32>>) {
	let many: Vec<u32> = (0..4096).map(|i| (i % 700) as u32).collect();
	let names = [
		"minecraft:air",
		"minecraft:stone",
		"minecraft:oak_log[axis=z]",
		"minecraft:grass_path",
		"minecraft:oak_stairs[facing=east,half=top,shape=outer_left,waterlogged=true]",
	];
	let palette: Vec<u32> = names.iter().map(|n| state("1.15.2", n)).collect();
	let few: Vec<u32> = (0..4096)
		.map(|i| palette[(i * 7 / 3) % palette.len()])
		.collect();
	let mut data = Vec::new();
	write_section(&mut data, &many, None, global_bits("1.15.2"), true);
	write_section(&mut data, &few, Some(&palette), 4, true);
	let heights: Vec<u32> = (0..256).map(|i| (i * 3 % 257) as u32).collect();
	let heightmaps = Entry::new(
		"",
		P::Compound(vec![Entry::new(
			"MOTION_BLOCKING",
			P::LongArray(pack(&heights, 9, true)),
		)]),
	);
	let p = C15::MapChunk(v1_15_2::play_to_client_packet_map_chunk::Ty {
		x: 4,
		z: -9,
		bit_map: 0b101,
		heightmaps,
		biomes: if full {
			v1_15_2::play_to_client_packet_map_chunk_biomes::Ty::True(
				(0..1024).map(|i| i % 3).collect(),
			)
		} else {
			v1_15_2::play_to_client_packet_map_chunk_biomes::Ty::False
		},
		chunk_data: data,
		block_entities: vec![],
	});
	(p, vec![many, few])
}

fn same_block(from: &str, a: u32, to: &str, b: u32) {
	let (a, b) = (describe(from, a), describe(to, b));
	let name = |s: &str| {
		s.split('[')
			.next()
			.unwrap()
			.replace("grass_path", "dirt_path")
	};
	assert_eq!(name(&a), name(&b), "{a} -> {b}");
}

#[test]
fn chunks_keep_blocks_through_1_17() {
	let mut cx = Session::new();
	let (p, sections) = chunk_15(true);
	let C16::MapChunk(c) = one(up_15(
		&p,
		"1.16",
		&mut cx,
		v1_16::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert!(c.ignore_old_data);
	let P::LongArray(h) = &compound(&c.heightmaps.value)[0].value else {
		panic!()
	};
	assert_eq!(h.len(), 37);
	assert_eq!(
		unpack(h, 9, 256),
		(0..256).map(|i| (i * 3 % 257) as u32).collect::<Vec<_>>()
	);
	let C17::MapChunk(c) = one(up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		(c.x, c.z, c.bit_map.as_slice(), c.biomes.len()),
		(4, -9, &[0b101i64][..], 1024)
	);
	let mut at = 0;
	for (n, src) in sections.iter().enumerate() {
		let (count, palette, states) = read_section(&c.chunk_data, &mut at, global_bits("1.17"));
		assert_eq!(palette.is_none(), n == 0, "section {n} palette kind");
		let air = state("1.17", "minecraft:air");
		assert_eq!(count as usize, states.iter().filter(|s| **s != air).count());
		for (i, (a, b)) in src.iter().zip(&states).enumerate() {
			if i % 37 == 0 || n == 1 {
				same_block("1.15.2", *a, "1.17", *b);
			}
		}
	}
	assert_eq!(at, c.chunk_data.len());
	let stairs = state(
		"1.17",
		"minecraft:oak_stairs[facing=east,half=top,shape=outer_left,waterlogged=true]",
	);
	let mut at = 0;
	read_section(&c.chunk_data, &mut at, global_bits("1.17"));
	assert!(read_section(&c.chunk_data, &mut at, global_bits("1.17"))
		.2
		.contains(&stairs));
}

#[test]
fn partial_chunk_becomes_block_changes_in_1_17() {
	let mut cx = Session::new();
	let (p, sections) = chunk_15(false);
	let out = up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	);
	assert_eq!(out.len(), 2);
	for (packet, (y, src)) in out.iter().zip([(0u32, &sections[0]), (2, &sections[1])]) {
		let C17::MultiBlockChange(m) = packet else {
			panic!("{packet:?}")
		};
		assert_eq!(
			(
				m.chunk_coordinates.x,
				m.chunk_coordinates.z,
				m.chunk_coordinates.y
			),
			(4, -9, y)
		);
		assert_eq!(m.records.len(), 4096);
		for r in m.records.iter().step_by(101) {
			let (x, z, ly) = ((r >> 8) & 15, (r >> 4) & 15, r & 15);
			same_block(
				"1.15.2",
				src[(ly << 8 | z << 4 | x) as usize],
				"1.17",
				(r >> 12) as u32,
			);
		}
	}
}

#[test]
fn light_reaches_1_17() {
	let mut cx = Session::new();
	let mut data = Vec::new();
	for fill in [1u8, 2, 3] {
		varint(&mut data, 2048);
		data.extend(std::iter::repeat(fill).take(2048));
	}
	let p = C15::UpdateLight(v1_15_2::play_to_client_packet_update_light::Ty {
		chunk_x: 1,
		chunk_z: 2,
		sky_light_mask: 0b11,
		block_light_mask: 0b1000,
		empty_sky_light_mask: 0b100,
		empty_block_light_mask: 0,
		data,
	});
	let C16::UpdateLight(l) = one(up_15(
		&p,
		"1.16",
		&mut cx,
		v1_16::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert!(l.trust_edges);
	let C17::UpdateLight(l) = one(up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		(
			l.sky_light_mask.as_slice(),
			l.block_light_mask.as_slice(),
			l.empty_sky_light_mask.as_slice()
		),
		(&[3i64][..], &[8i64][..], &[4i64][..])
	);
	assert_eq!(
		l.sky_light
			.iter()
			.map(|a| (a.len(), a[0]))
			.collect::<Vec<_>>(),
		[(2048, 1), (2048, 2)]
	);
	assert_eq!(l.block_light[0][100], 3);
}

#[test]
fn block_updates_follow_the_registries() {
	let mut cx = Session::new();
	let path = state("1.15.2", "minecraft:grass_path") as i32;
	let p = C15::BlockChange(v1_15_2::play_to_client_packet_block_change::Ty {
		location: v1_15_2::position::Ty { x: 1, y: 70, z: -3 },
		r#type: path,
	});
	let C17::BlockChange(b) = one(up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(describe("1.17", b.r#type as u32), "minecraft:dirt_path");
	assert_eq!(b.location.y, 70);
	let stone = state("1.15.2", "minecraft:stone") as i32;
	let p = C15::MultiBlockChange(v1_15_2::play_to_client_packet_multi_block_change::Ty {
		chunk_x: -2,
		chunk_z: 3,
		records: vec![
			v1_15_2::play_to_client_packet_multi_block_change_records_items::Ty {
				horizontal_pos: 0x5a,
				y: 3,
				block_id: stone,
			},
			v1_15_2::play_to_client_packet_multi_block_change_records_items::Ty {
				horizontal_pos: 0x12,
				y: 40,
				block_id: path,
			},
		],
	});
	let out = up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	);
	let changes: Vec<(u32, i64)> = out
		.iter()
		.map(|p| match p {
			C17::MultiBlockChange(m) => {
				assert_eq!(
					(
						m.chunk_coordinates.x,
						m.chunk_coordinates.z,
						m.records.len()
					),
					(-2, 3, 1)
				);
				(m.chunk_coordinates.y, m.records[0])
			}
			p => panic!("{p:?}"),
		})
		.collect();
	assert_eq!(changes[0].0, 0);
	assert_eq!(changes[0].1 & 0xfff, 0x5a3);
	assert_eq!(
		describe("1.17", (changes[0].1 >> 12) as u32),
		"minecraft:stone"
	);
	assert_eq!((changes[1].0, changes[1].1 & 0xfff), (2, 0x128));
	assert_eq!(
		describe("1.17", (changes[1].1 >> 12) as u32),
		"minecraft:dirt_path"
	);
}

fn slot(version: &str, name: &str, count: i8) -> v1_15_2::slot::Ty {
	v1_15_2::slot::Ty::True(v1_15_2::slot_true::Ty {
		item_id: item(version, name),
		item_count: count,
		nbt_data: None,
	})
}

fn slot_name(version: &str, s: &v1_15_2::slot::Ty) -> String {
	match s {
		v1_15_2::slot::Ty::True(s) => registry::names(version, "items")
			.unwrap()
			.name(s.item_id as u32)
			.unwrap()
			.to_owned(),
		v1_15_2::slot::Ty::False => "empty".to_owned(),
	}
}

#[test]
fn entities_and_metadata() {
	let mut cx = Session::new();
	let spawn = C15::SpawnEntity(v1_15_2::play_to_client_packet_spawn_entity::Ty {
		entity_id: 50,
		object_uuid: 1,
		r#type: entity("1.15.2", "arrow"),
		x: 0.5,
		y: 65.0,
		z: 0.5,
		pitch: 0,
		yaw: 0,
		object_data: 0,
		velocity: v1_15_2::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let C17::SpawnEntity(s) = one(up_15(
		&spawn,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(s.r#type, entity("1.17", "arrow"));
	use v1_15_2::entity_metadata_item::Ty as M;
	let entry =
		|key: u8, r#type: i32, value: M| v1_15_2::entity_metadata_items::Ty { key, r#type, value };
	let meta = C15::EntityMetadata(v1_15_2::play_to_client_packet_entity_metadata::Ty {
		entity_id: 50,
		metadata: vec![
			entry(0, 0, M::V0(0x20)),
			entry(8, 12, M::V12(Some(9))),
			entry(9, 0, M::V0(3)),
		],
	});
	let C16::EntityMetadata(m) = one(up_15(
		&meta,
		"1.16",
		&mut cx,
		v1_16::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	let keys: Vec<(u8, i32)> = m.metadata.iter().map(|e| (e.key, e.r#type)).collect();
	assert_eq!(keys, [(0, 0), (8, 0)]);
	let C17::EntityMetadata(m) = one(up_15(
		&meta,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(m.metadata.iter().map(|e| e.key).collect::<Vec<_>>(), [0, 9]);

	let pig = C15::SpawnEntityLiving(v1_15_2::play_to_client_packet_spawn_entity_living::Ty {
		entity_id: 51,
		entity_uuid: 2,
		r#type: entity("1.15.2", "zombie_pigman"),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		yaw: 0,
		pitch: 0,
		head_pitch: 0,
		velocity: v1_15_2::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let C171::SpawnEntityLiving(s) = one(up_15(
		&pig,
		"1.17.1",
		&mut cx,
		v1_17_1::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(s.r#type, entity("1.17", "zombified_piglin"));

	let item_entity = C15::EntityMetadata(v1_15_2::play_to_client_packet_entity_metadata::Ty {
		entity_id: 52,
		metadata: vec![
			entry(7, 6, M::V6(slot("1.15.2", "diamond_sword", 1))),
			entry(6, 18, M::V18(6)),
		],
	});
	let C17::EntityMetadata(m) = one(up_15(
		&item_entity,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	use v1_17::entity_metadata_item::Ty as M17;
	assert!(
		matches!(&m.metadata[0], e if e.key == 8 && matches!(&e.value, M17::V6(s) if slot_name("1.17", s) == "diamond_sword"))
	);
	assert!(matches!(&m.metadata[1], e if e.key == 6 && matches!(e.value, M17::V18(7))));

	let wolf = C15::SpawnEntityLiving(v1_15_2::play_to_client_packet_spawn_entity_living::Ty {
		entity_id: 53,
		entity_uuid: 3,
		r#type: entity("1.15.2", "wolf"),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		yaw: 0,
		pitch: 0,
		head_pitch: 0,
		velocity: v1_15_2::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	up_15(
		&wolf,
		"1.16",
		&mut cx,
		v1_16::play_to_client_packet::deserialize,
	);
	let angry = C15::EntityMetadata(v1_15_2::play_to_client_packet_entity_metadata::Ty {
		entity_id: 53,
		metadata: vec![entry(16, 0, M::V0(2))],
	});
	let C16::EntityMetadata(m) = one(up_15(
		&angry,
		"1.16",
		&mut cx,
		v1_16::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert!(m
		.metadata
		.iter()
		.any(|e| e.key == 20 && e.value == v1_16::entity_metadata_item::Ty::V1(i32::MAX)));
}

#[test]
fn equipment_becomes_a_list() {
	let mut cx = Session::new();
	let p = C15::EntityEquipment(v1_15_2::play_to_client_packet_entity_equipment::Ty {
		entity_id: 3,
		slot: 5,
		item: slot("1.15.2", "golden_helmet", 1),
	});
	let bytes = run(
		Root::PlayToClient,
		"1.15.2",
		"1.16",
		&enc(&p, v1_15_2::play_to_client_packet::serialize),
		&mut cx,
	);
	let C16::EntityEquipment(e) = dec(&bytes[0], v1_16::play_to_client_packet::deserialize) else {
		panic!()
	};
	assert_eq!(
		(e.entity_id, e.equipments.len(), e.equipments[0].slot),
		(3, 1, 5)
	);
	assert_eq!(slot_name("1.16", &e.equipments[0].item), "golden_helmet");
	let two = C16::EntityEquipment(v1_16::play_to_client_packet_entity_equipment::Ty {
		entity_id: 3,
		equipments: vec![
			v1_16::play_to_client_packet_entity_equipment_equipments_items::Ty {
				slot: 0,
				item: slot("1.16", "netherite_sword", 1),
			},
			v1_16::play_to_client_packet_entity_equipment_equipments_items::Ty {
				slot: 1,
				item: slot("1.16", "shield", 1),
			},
		],
	});
	let back = run(
		Root::PlayToClient,
		"1.16",
		"1.15.2",
		&enc(&two, v1_16::play_to_client_packet::serialize),
		&mut cx,
	);
	let back: Vec<C15> = back
		.iter()
		.map(|b| dec(b, v1_15_2::play_to_client_packet::deserialize))
		.collect();
	assert!(
		matches!(back.as_slice(), [C15::EntityEquipment(a), C15::EntityEquipment(b)]
		if a.slot == 0 && slot_name("1.15.2", &a.item) == "diamond_sword" && b.slot == 1 && slot_name("1.15.2", &b.item) == "shield")
	);
}

#[test]
fn pings_become_transactions_for_1_16_clients() {
	let mut cx = Session::new();
	let C162::Transaction(t) = one(down_17(&C17::Ping(77), &mut cx)) else {
		panic!()
	};
	assert_eq!((t.window_id, t.action, t.accepted), (0, 77, false));
	assert!(down_17(&C17::Ping(1 << 20), &mut cx).is_empty());
	let reply = v1_16_2::play_to_server_packet::Ty::Transaction(
		v1_16_2::play_to_server_packet_transaction::Ty {
			window_id: 0,
			action: 77,
			accepted: true,
		},
	);
	let out = run(
		Root::PlayToServer,
		"1.16.2",
		"1.17",
		&enc(&reply, v1_16_2::play_to_server_packet::serialize),
		&mut cx,
	);
	assert_eq!(
		dec(&out[0], v1_17::play_to_server_packet::deserialize),
		v1_17::play_to_server_packet::Ty::Pong(77)
	);
}

#[test]
fn lightning_becomes_an_entity() {
	let mut cx = Session::new();
	let p = C15::SpawnEntityWeather(v1_15_2::play_to_client_packet_spawn_entity_weather::Ty {
		entity_id: 9,
		r#type: 1,
		x: 1.0,
		y: 2.0,
		z: 3.0,
	});
	let C16::SpawnEntity(s) = one(up_15(
		&p,
		"1.16",
		&mut cx,
		v1_16::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!((s.r#type, s.y), (entity("1.16", "lightning_bolt"), 2.0));
}

#[test]
fn items_particles_and_sounds() {
	let mut cx = Session::new();
	let p = C15::SetSlot(v1_15_2::play_to_client_packet_set_slot::Ty {
		window_id: 0,
		slot: 36,
		item: slot("1.15.2", "grass_path", 5),
	});
	let C171::SetSlot(s) = one(up_15(
		&p,
		"1.17.1",
		&mut cx,
		v1_17_1::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		(slot_name("1.17", &s.item), s.state_id),
		("dirt_path".to_owned(), 0)
	);
	let p = C15::WindowItems(v1_15_2::play_to_client_packet_window_items::Ty {
		window_id: 0,
		items: vec![
			slot("1.15.2", "zombie_pigman_spawn_egg", 1),
			v1_15_2::slot::Ty::False,
			slot("1.15.2", "shield", 1),
		],
	});
	let C171::WindowItems(w) = one(up_15(
		&p,
		"1.17.1",
		&mut cx,
		v1_17_1::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		w.items
			.iter()
			.map(|s| slot_name("1.17", s))
			.collect::<Vec<_>>(),
		["zombified_piglin_spawn_egg", "empty", "shield"]
	);
	assert_eq!(w.carried_item, v1_15_2::slot::Ty::False);

	let particle = C15::WorldParticles(v1_15_2::play_to_client_packet_world_particles::Ty {
		particle_id: 32,
		long_distance: false,
		x: 0.0,
		y: 0.0,
		z: 0.0,
		offset_x: 0.0,
		offset_y: 0.0,
		offset_z: 0.0,
		particle_data: 0.0,
		particles: 1,
		data: v1_15_2::particle_data::Ty::V32(slot("1.15.2", "apple", 1)),
	});
	let C17::WorldParticles(w) = one(up_15(
		&particle,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(w.particle_id, 36);
	assert!(matches!(&w.data, v1_17::particle_data::Ty::V36(s) if slot_name("1.17", s) == "apple"));

	let sounds = registry::names("1.16", "sounds").unwrap();
	let id = sounds.id("entity.player.levelup").unwrap() as i32;
	assert!(id > 631);
	let p = v1_16::play_to_client_packet::Ty::SoundEffect(
		v1_16::play_to_client_packet_sound_effect::Ty {
			sound_id: id,
			sound_category: 0,
			x: 8,
			y: 512,
			z: 8,
			volume: 1.0,
			pitch: 1.0,
		},
	);
	let out = run(
		Root::PlayToClient,
		"1.16.1",
		"1.16.2",
		&enc(&p, v1_16_1::play_to_client_packet::serialize),
		&mut cx,
	);
	let C162::SoundEffect(s) = dec(&out[0], v1_16_2::play_to_client_packet::deserialize) else {
		panic!()
	};
	assert_eq!(s.sound_id, id + 7);
	let out = run(
		Root::PlayToClient,
		"1.16.1",
		"1.17",
		&enc(&p, v1_16_1::play_to_client_packet::serialize),
		&mut cx,
	);
	let C17::SoundEffect(s) = dec(&out[0], v1_17::play_to_client_packet::deserialize) else {
		panic!()
	};
	assert_eq!(
		registry::names("1.17", "sounds")
			.unwrap()
			.name(s.sound_id as u32),
		Some("entity.player.levelup")
	);
}

use minecraft_versions::gen::convert::v1_16_1;

#[test]
fn shaped_recipes_keep_ingredient_order() {
	let mut cx = Session::new();
	let names = [
		"oak_planks",
		"stick",
		"cobblestone",
		"iron_ingot",
		"gold_ingot",
		"diamond",
	];
	let ingredients: Vec<Vec<Vec<v1_15_2::slot::Ty>>> = (0..2)
		.map(|x| {
			(0..3)
				.map(|y| vec![slot("1.15.2", names[x * 3 + y], 1)])
				.collect()
		})
		.collect();
	let p = C15::DeclareRecipes(
		vec![v1_15_2::play_to_client_packet_declare_recipes_items::Ty {
		r#type: "minecraft:crafting_shaped".to_owned(),
		recipe_id: "minecraft:test".to_owned(),
		data: v1_15_2::play_to_client_packet_declare_recipes_items_data::Ty::MinecraftCraftingShaped(
			v1_15_2::play_to_client_packet_declare_recipes_items_data_minecraft_crafting_shaped::Ty {
				width: 2,
				height: 3,
				group: String::new(),
				ingredients,
				result: slot("1.15.2", "grass_path", 1),
			},
		),
	}],
	);
	let flat = |i: &Vec<Vec<Vec<v1_15_2::slot::Ty>>>, v: &str| {
		i.iter()
			.flatten()
			.flatten()
			.map(|s| slot_name(v, s))
			.collect::<Vec<_>>()
	};
	let C162::DeclareRecipes(r) = one(up_15(
		&p,
		"1.16.2",
		&mut cx,
		v1_16_2::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	let v1_16_2::play_to_client_packet_declare_recipes_items_data::Ty::MinecraftCraftingShaped(s) =
		&r[0].data
	else {
		panic!()
	};
	assert_eq!(flat(&s.ingredients, "1.16.2"), names);
	let C17::DeclareRecipes(r) = one(up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	let v1_17::play_to_client_packet_declare_recipes_items_data::Ty::MinecraftCraftingShaped(s) =
		&r[0].data
	else {
		panic!()
	};
	assert_eq!(flat(&s.ingredients, "1.17"), names);
	assert_eq!(slot_name("1.17", &s.result), "dirt_path");
}

#[test]
fn tags_gain_required_entries() {
	let mut cx = Session::new();
	let tag =
		|name: &str, entries: Vec<i32>| v1_15_2::play_to_client_packet_tags_block_tags_items::Ty {
			tag_name: name.to_owned(),
			entries,
		};
	let p = C15::Tags(v1_15_2::play_to_client_packet_tags::Ty {
		block_tags: vec![tag(
			"minecraft:logs",
			vec![registry::names("1.15.2", "blocks")
				.unwrap()
				.id("oak_log")
				.unwrap() as i32],
		)],
		item_tags: vec![tag("minecraft:furnace_materials", vec![])],
		fluid_tags: vec![tag("minecraft:water", vec![1, 2])],
		entity_tags: vec![],
	});
	let C16::Tags(t) = one(up_15(
		&p,
		"1.16",
		&mut cx,
		v1_16::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	let climbable = t
		.block_tags
		.iter()
		.find(|t| t.tag_name == "minecraft:climbable")
		.unwrap();
	let blocks = registry::names("1.16", "blocks").unwrap();
	assert!(climbable
		.entries
		.contains(&(blocks.id("ladder").unwrap() as i32)));
	assert_eq!(
		t.block_tags[0].entries,
		[blocks.id("oak_log").unwrap() as i32]
	);
	let C17::Tags(groups) = one(up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	let types: Vec<&str> = groups.iter().map(|g| g.tag_type.as_str()).collect();
	assert_eq!(
		types,
		[
			"minecraft:block",
			"minecraft:item",
			"minecraft:fluid",
			"minecraft:entity_type",
			"minecraft:game_event"
		]
	);
	assert!(groups[1]
		.tags
		.iter()
		.all(|t| t.tag_name != "minecraft:furnace_materials"));
	assert!(groups[4]
		.tags
		.iter()
		.any(|t| t.tag_name == "minecraft:vibrations"));
	assert_eq!(groups[2].tags[0].entries, [1, 2]);
}

#[test]
fn split_packets_in_1_17() {
	let mut cx = Session::new();
	let title = C15::Title(v1_15_2::play_to_client_packet_title::Ty::V2(
		"bar".to_owned(),
	));
	assert!(
		matches!(up_15(&title, "1.17", &mut cx, v1_17::play_to_client_packet::deserialize).as_slice(), [C17::ActionBar(t)] if t == "bar")
	);
	let reset = C15::Title(v1_15_2::play_to_client_packet_title::Ty::Default(5));
	assert!(matches!(
		up_15(
			&reset,
			"1.17",
			&mut cx,
			v1_17::play_to_client_packet::deserialize
		)
		.as_slice(),
		[C17::ClearTitles(true)]
	));
	let border = C15::WorldBorder(v1_15_2::play_to_client_packet_world_border::Ty::V2(
		v1_15_2::play_to_client_packet_world_border_v2::Ty { x: 1.0, z: 2.0 },
	));
	assert!(
		matches!(up_15(&border, "1.17", &mut cx, v1_17::play_to_client_packet::deserialize).as_slice(), [C17::WorldBorderCenter(c)] if (c.x, c.z) == (1.0, 2.0))
	);
	let destroy = C15::EntityDestroy(vec![4, 5]);
	assert!(matches!(
		up_15(
			&destroy,
			"1.17",
			&mut cx,
			v1_17::play_to_client_packet::deserialize
		)
		.as_slice(),
		[C17::DestroyEntity(4), C17::DestroyEntity(5)]
	));
	assert!(
		matches!(up_15(&destroy, "1.17.1", &mut cx, v1_17_1::play_to_client_packet::deserialize).as_slice(), [C171::EntityDestroy(a), C171::EntityDestroy(b)] if a == &[4] && b == &[5])
	);
	let rejected = C15::Transaction(v1_15_2::play_to_client_packet_transaction::Ty {
		window_id: 3,
		action: 17,
		accepted: false,
	});
	let C17::Ping(id) = one(up_15(
		&rejected,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	let out = run(
		Root::PlayToServer,
		"1.17",
		"1.15.2",
		&enc(
			&v1_17::play_to_server_packet::Ty::Pong(id),
			v1_17::play_to_server_packet::serialize,
		),
		&mut cx,
	);
	let v1_15_2::play_to_server_packet::Ty::Transaction(t) =
		dec(&out[0], v1_15_2::play_to_server_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!((t.window_id, t.action, t.accepted), (3, 17, true));
	let accepted = C15::Transaction(v1_15_2::play_to_client_packet_transaction::Ty {
		window_id: 3,
		action: 18,
		accepted: true,
	});
	assert!(up_15(
		&accepted,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize
	)
	.is_empty());
}

#[test]
fn serverbound_reaches_1_15_2() {
	let mut cx = Session::new();
	let abilities = C15::Abilities(v1_15_2::play_to_client_packet_abilities::Ty {
		flags: 4,
		flying_speed: 0.2,
		walking_speed: 0.3,
	});
	up_15(
		&abilities,
		"1.17.1",
		&mut cx,
		v1_17_1::play_to_client_packet::deserialize,
	);
	let server = |p: &v1_17_1::play_to_server_packet::Ty,
	              cx: &mut Session|
	 -> Vec<v1_15_2::play_to_server_packet::Ty> {
		run(
			Root::PlayToServer,
			"1.17.1",
			"1.15.2",
			&enc(p, v1_17_1::play_to_server_packet::serialize),
			cx,
		)
		.iter()
		.map(|b| dec(b, v1_15_2::play_to_server_packet::deserialize))
		.collect()
	};
	let v1_15_2::play_to_server_packet::Ty::Abilities(a) = one(server(
		&v1_17_1::play_to_server_packet::Ty::Abilities(2),
		&mut cx,
	)) else {
		panic!()
	};
	assert_eq!((a.flags, a.flying_speed, a.walking_speed), (2, 0.2, 0.3));
	let click = v1_17_1::play_to_server_packet::Ty::WindowClick(
		v1_17_1::play_to_server_packet_window_click::Ty {
			window_id: 1,
			state_id: 9,
			slot: 5,
			mouse_button: 0,
			mode: 0,
			changed_slots: vec![],
			cursor_item: slot("1.17", "dirt_path", 2),
		},
	);
	let v1_15_2::play_to_server_packet::Ty::WindowClick(c) = one(server(&click, &mut cx)) else {
		panic!()
	};
	assert_eq!((c.window_id, c.slot, c.action, c.mode), (1, 5, 0, 0));
	assert_eq!(slot_name("1.15.2", &c.item), "grass_path");
	let book = v1_17_1::play_to_server_packet::Ty::EditBook(
		v1_17_1::play_to_server_packet_edit_book::Ty {
			hand: 40,
			pages: vec!["one".to_owned(), "two".to_owned()],
			title: Some("T".to_owned()),
		},
	);
	let v1_15_2::play_to_server_packet::Ty::EditBook(b) = one(server(&book, &mut cx)) else {
		panic!()
	};
	assert_eq!((b.hand, b.signing), (1, true));
	assert_eq!(slot_name("1.15.2", &b.new_book), "writable_book");
	let v1_15_2::slot::Ty::True(s) = &b.new_book else {
		panic!()
	};
	let tag = compound(&s.nbt_data.as_ref().unwrap().value);
	assert_eq!(string(get(tag, "title")), "T");
	let recipe = v1_17_1::play_to_server_packet::Ty::RecipeBook(
		v1_17_1::play_to_server_packet_recipe_book::Ty {
			book_id: 1,
			book_open: true,
			filter_active: false,
		},
	);
	let v1_15_2::play_to_server_packet::Ty::CraftingBookData(
		v1_15_2::play_to_server_packet_crafting_book_data::Ty::V1(d),
	) = one(server(&recipe, &mut cx))
	else {
		panic!()
	};
	assert!(d.smelting_book_open && !d.crafting_book_open);
	let interact = v1_17_1::play_to_server_packet::Ty::UseEntity(
		v1_17_1::play_to_server_packet_use_entity::Ty {
			target: 3,
			mouse: v1_17_1::play_to_server_packet_use_entity_mouse::Ty::V0(0),
			sneaking: true,
		},
	);
	assert!(
		matches!(server(&interact, &mut cx).as_slice(), [v1_15_2::play_to_server_packet::Ty::UseEntity(u)] if u.target == 3)
	);
	assert!(server(&v1_17_1::play_to_server_packet::Ty::Pong(5), &mut cx).is_empty());
}

fn login_17(min_y: i32, height: i32, cx: &mut Session) -> C17 {
	let C17::Login(mut l) = one(up_15(
		&login_15(0),
		"1.17",
		cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	let P::Compound(dim) = &mut l.dimension.value else {
		panic!()
	};
	for e in dim.iter_mut() {
		match e.name.as_str() {
			"min_y" => e.value = P::Int(min_y),
			"height" => e.value = P::Int(height),
			_ => {}
		}
	}
	C17::Login(l)
}

fn bits(n: usize) -> Vec<i64> {
	(0..n.div_ceil(64))
		.map(|w| {
			if n - w * 64 >= 64 {
				-1
			} else {
				(1i64 << (n - w * 64)) - 1
			}
		})
		.collect()
}

fn tall_chunk(x: i32, z: i32, sections: usize) -> C17 {
	let names = registry::names("1.17", "blocks").unwrap();
	let blocks = (0..sections)
		.map(|k| {
			Some(
				registry::blocks("1.17")
					.unwrap()
					.block(names.name(1 + k as u32).unwrap())
					.unwrap()
					.default,
			)
		})
		.collect::<Vec<_>>();
	filled_chunk(x, z, &blocks)
}

fn filled_chunk(x: i32, z: i32, blocks: &[Option<u32>]) -> C17 {
	let sections = blocks.len();
	let mut data = Vec::new();
	let mut mask = vec![0i64; sections.div_ceil(64)];
	for (k, block) in blocks.iter().enumerate() {
		if let Some(block) = block {
			write_section(&mut data, &vec![*block; 4096], Some(&[*block]), 4, false);
			mask[k / 64] |= 1 << (k % 64);
		}
	}
	let heights = vec![200 + 64; 256];
	C17::MapChunk(v1_17::play_to_client_packet_map_chunk::Ty {
		x,
		z,
		bit_map: mask,
		heightmaps: Entry::new(
			"",
			P::Compound(vec![Entry::new(
				"WORLD_SURFACE",
				P::LongArray(pack(&heights, 9, false)),
			)]),
		),
		biomes: (0..sections as i32 * 64).map(|i| i / 64).collect(),
		chunk_data: data,
		block_entities: [-60, 300]
			.iter()
			.map(|y| {
				Entry::new(
					"",
					P::Compound(vec![
						Entry::new("id", P::String("minecraft:chest".to_owned())),
						Entry::new("x", P::Int(x * 16)),
						Entry::new("y", P::Int(*y)),
						Entry::new("z", P::Int(z * 16)),
					]),
				)
			})
			.collect(),
	})
}

fn first_section_block(c: &v1_16_2::play_to_client_packet_map_chunk::Ty) -> String {
	let mut at = 0;
	let (_, _, states) = read_section(&c.chunk_data, &mut at, global_bits("1.16.2"));
	describe("1.16.2", states[0])
}

fn expected_block(k: u32) -> String {
	let name = registry::names("1.17", "blocks")
		.unwrap()
		.name(1 + k)
		.unwrap();
	describe("1.16.2", state("1.16.2", &format!("minecraft:{name}")))
}

fn position_17(y: f64, flags: i8) -> C17 {
	C17::Position(v1_17::play_to_client_packet_position::Ty {
		x: 8.0,
		y,
		z: 8.0,
		yaw: 0.0,
		pitch: 0.0,
		flags,
		teleport_id: 1,
		dismount_vehicle: false,
	})
}

fn light_17(x: i32, z: i32, sections: usize) -> C17 {
	let mask = bits(sections + 2);
	C17::UpdateLight(v1_17::play_to_client_packet_update_light::Ty {
		chunk_x: x,
		chunk_z: z,
		trust_edges: true,
		sky_light_mask: mask.clone(),
		block_light_mask: vec![],
		empty_sky_light_mask: vec![],
		empty_block_light_mask: mask,
		sky_light: (0..sections + 2).map(|k| vec![k as u8; 2048]).collect(),
		block_light: vec![],
	})
}

fn sky(l: &v1_16_2::play_to_client_packet_update_light::Ty) -> Vec<u8> {
	let mut at = 0;
	let mut out = Vec::new();
	for _ in 0..(l.sky_light_mask as u32).count_ones() {
		let n = read_varint(&l.data, &mut at) as usize;
		out.push(l.data[at]);
		at += n;
	}
	out
}

#[test]
fn tall_worlds_start_in_the_identity_window() {
	let mut cx = Session::new();
	let C162::Login(l) = one(down_17(&login_17(-64, 384, &mut cx), &mut cx)) else {
		panic!()
	};
	let dim = compound(&l.dimension.value);
	assert!(get(dim, "min_y").is_none() && get(dim, "height").is_none());
	assert_eq!(int(get(dim, "logical_height")), 256);
	let C162::UpdateLight(light) = one(down_17(&light_17(0, 0, 24), &mut cx)) else {
		panic!()
	};
	assert_eq!(sky(&light), (4..22).collect::<Vec<u8>>());
	assert_eq!(light.empty_block_light_mask, (1 << 18) - 1);
	let C162::MapChunk(c) = one(down_17(&tall_chunk(0, 0, 24), &mut cx)) else {
		panic!()
	};
	assert_eq!(c.bit_map, 0xffff);
	assert_eq!(first_section_block(&c), expected_block(4));
	let v1_16_2::play_to_client_packet_map_chunk_biomes::Ty::True(b) = &c.biomes else {
		panic!()
	};
	assert_eq!((b.len(), b[0], b[1023]), (1024, 4, 19));
	assert!(c.block_entities.is_empty());
	let P::LongArray(h) = &compound(&c.heightmaps.value)[0].value else {
		panic!()
	};
	assert_eq!(unpack(h, 9, 256)[0], 200);
	let C162::Position(p) = one(down_17(&position_17(70.0, 0), &mut cx)) else {
		panic!()
	};
	assert_eq!(p.y, 70.0);
}

fn teleport(out: &[C162]) -> &v1_16_2::play_to_client_packet_position::Ty {
	out.iter()
		.find_map(|p| {
			if let C162::Position(p) = p {
				Some(p)
			} else {
				None
			}
		})
		.expect("a teleport")
}

fn teleport_17(y: f64, flags: i8, id: i32) -> C17 {
	let C17::Position(mut p) = position_17(y, flags) else {
		unreachable!()
	};
	p.teleport_id = id;
	C17::Position(p)
}

fn map_chunk(
	out: &[C162],
	x: i32,
	z: i32,
) -> Option<&v1_16_2::play_to_client_packet_map_chunk::Ty> {
	out.iter().find_map(|p| {
		if let C162::MapChunk(c) = p {
			(c.x == x && c.z == z).then_some(c)
		} else {
			None
		}
	})
}

fn order(out: &[C162], f: fn(&C162) -> bool) -> usize {
	out.iter().position(f).expect("a packet")
}

fn sections(c: &v1_16_2::play_to_client_packet_map_chunk::Ty) -> Vec<Vec<u32>> {
	let mut at = 0;
	(0..(c.bit_map as u32).count_ones())
		.map(|_| read_section(&c.chunk_data, &mut at, global_bits("1.16.2")).2)
		.collect()
}

fn block_change_17(x: i32, y: i32, z: i32, text: &str) -> C17 {
	C17::BlockChange(v1_17::play_to_client_packet_block_change::Ty {
		location: v1_17::position::Ty { x, y: y as i16, z },
		r#type: state("1.17", text) as i32,
	})
}

fn mob_17(id: i32, y: f64) -> C17 {
	C17::SpawnEntityLiving(v1_17::play_to_client_packet_spawn_entity_living::Ty {
		entity_id: id,
		entity_uuid: id as u128,
		r#type: entity("1.17", "cow"),
		x: 1.0,
		y,
		z: 1.0,
		yaw: 0,
		pitch: 0,
		head_pitch: 0,
		velocity: v1_17::vec3i16::Ty { x: 0, y: 0, z: 0 },
	})
}

fn confirm(id: i32, cx: &mut Session) -> Vec<S17> {
	up_16_2(&S162::TeleportConfirm(id), cx).0
}

#[test]
fn tall_worlds_switch_windows_on_server_teleports() {
	let mut cx = Session::new();
	tall_world(&mut cx, -64, 384);
	let C162::SpawnEntityLiving(m) = one(down_17(&mob_17(70, 250.0), &mut cx)) else {
		panic!()
	};
	assert_eq!(m.y, 250.0);

	let out = down_17(&teleport_17(300.0, 0, 2), &mut cx);
	let p = teleport(&out);
	assert_eq!((p.y, p.flags), (236.0, 0));
	let chunk = map_chunk(&out, 0, 0).unwrap();
	assert_eq!(first_section_block(chunk), expected_block(8));
	let v1_16_2::play_to_client_packet_map_chunk_biomes::Ty::True(b) = &chunk.biomes else {
		panic!()
	};
	assert_eq!(b[0], 8);
	assert_eq!(int(get(compound(&chunk.block_entities[0].value), "y")), 236);
	let light = out
		.iter()
		.find_map(|p| {
			if let C162::UpdateLight(l) = p {
				Some(l)
			} else {
				None
			}
		})
		.unwrap();
	assert_eq!(sky(light), (8..26).collect::<Vec<u8>>());
	let light_at = order(&out, |p| matches!(p, C162::UpdateLight(_)));
	let chunk_at = order(&out, |p| matches!(p, C162::MapChunk(_)));
	assert!(light_at < chunk_at && chunk_at < order(&out, |p| matches!(p, C162::Position(_))));
	assert!(out
		.iter()
		.any(|p| matches!(p, C162::EntityTeleport(t) if t.entity_id == 70 && t.y == 186.0)));
	assert_eq!(confirm(2, &mut cx), [S17::TeleportConfirm(2)]);

	let C162::BlockChange(b) = one(down_17(
		&block_change_17(3, -50, 4, "minecraft:stone"),
		&mut cx,
	)) else {
		panic!()
	};
	assert_eq!(
		(b.location.y, describe("1.16.2", b.r#type as u32)),
		(0, expected_block(8))
	);
	let C162::BlockChange(b) = one(down_17(
		&block_change_17(3, 100, 4, "minecraft:stone"),
		&mut cx,
	)) else {
		panic!()
	};
	assert_eq!(
		(b.location.y, b.r#type as u32),
		(36, state("1.16.2", "minecraft:stone"))
	);

	let rel = down_17(&teleport_17(-280.0, 2, 3), &mut cx);
	let p = teleport(&rel);
	assert_eq!((p.y, p.flags), (84.0, 0));
	let chunk = map_chunk(&rel, 0, 0).unwrap();
	assert_eq!(
		sections(chunk)[0][(14 << 8) | (4 << 4) | 3],
		state("1.16.2", "minecraft:stone")
	);

	let (sent, back) = up_16_2(&walk(236.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (300.0, 0));
	assert_eq!(confirm(3, &mut cx), [S17::TeleportConfirm(3)]);
	let (sent, back) = up_16_2(&walk(84.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (20.0, 0));
	let dig = S162::BlockDig(v1_16_2::play_to_server_packet_block_dig::Ty {
		status: 0,
		location: v1_16_2::position::Ty { x: 1, y: 10, z: 1 },
		face: 1,
	});
	let (sent, _) = up_16_2(&dig, &mut cx);
	let [S17::BlockDig(d)] = sent.as_slice() else {
		panic!("{sent:?}")
	};
	assert_eq!(d.location.y, -54);

	down_17(
		&C17::UnloadChunk(v1_17::play_to_client_packet_unload_chunk::Ty {
			chunk_x: 0,
			chunk_z: 0,
		}),
		&mut cx,
	);
	let again = down_17(&teleport_17(300.0, 0, 4), &mut cx);
	assert!(!again.iter().any(|p| matches!(p, C162::MapChunk(_))));
	assert_eq!(teleport(&again).y, 236.0);
}

type S162 = v1_16_2::play_to_server_packet::Ty;
type S17 = v1_17::play_to_server_packet::Ty;

fn up_16_2(p: &S162, cx: &mut Session) -> (Vec<S17>, Vec<C162>) {
	let sent = run(
		Root::PlayToServer,
		"1.16.2",
		"1.17",
		&enc(p, v1_16_2::play_to_server_packet::serialize),
		cx,
	)
	.iter()
	.map(|b| dec(b, v1_17::play_to_server_packet::deserialize))
	.collect();
	let back = cx
		.take_outgoing()
		.into_iter()
		.map(|o| {
			assert_eq!((o.root, o.version), (Root::PlayToClient, at("1.16.2")));
			dec(&o.data, v1_16_2::play_to_client_packet::deserialize)
		})
		.collect();
	(sent, back)
}

fn walk(y: f64) -> S162 {
	S162::Position(v1_16_2::play_to_server_packet_position::Ty {
		x: 8.0,
		y,
		z: 8.0,
		on_ground: true,
	})
}

fn server_y(sent: &[S17]) -> f64 {
	match sent {
		[S17::Position(p)] => p.y,
		[S17::PositionLook(p)] => p.y,
		[S17::VehicleMove(p)] => p.y,
		s => panic!("{s:?}"),
	}
}

fn tall_world(cx: &mut Session, min_y: i32, height: i32) {
	down_17(&login_17(min_y, height, cx), cx);
	down_17(&light_17(0, 0, height as usize / 16), cx);
	down_17(&tall_chunk(0, 0, height as usize / 16), cx);
}

fn arrive(y: f64, cx: &mut Session) {
	let out = down_17(&teleport_17(y, 0, 2), cx);
	assert_eq!(teleport(&out).teleport_id, 2);
	assert_eq!(confirm(2, cx), [S17::TeleportConfirm(2)]);
}

fn switched(back: &[C162], dy: f64) -> i32 {
	let t = teleport(back);
	assert_eq!((t.x, t.y, t.z, t.flags), (0.0, dy, 0.0, 0x1f));
	assert_eq!(t.teleport_id & !0x00ff_ffff, 0x4e00_0000);
	t.teleport_id
}

#[test]
fn tall_worlds_follow_walking_digging_and_flying() {
	let mut cx = Session::new();
	tall_world(&mut cx, -64, 384);
	down_17(&tall_chunk(5, 5, 24), &mut cx);
	let (sent, back) = up_16_2(&walk(20.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (20.0, 0));
	arrive(100.0, &mut cx);
	down_17(&mob_17(71, 150.0), &mut cx);

	let (sent, back) = up_16_2(&walk(33.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (33.0, 0));
	let (sent, back) = up_16_2(&walk(31.0), &mut cx);
	assert_eq!(server_y(&sent), 31.0);
	let id = switched(&back, 64.0);
	assert_eq!(
		first_section_block(map_chunk(&back, 0, 0).unwrap()),
		expected_block(0)
	);
	assert!(map_chunk(&back, 5, 5).is_none());
	assert!(
		order(&back, |p| matches!(p, C162::MapChunk(_)))
			< order(&back, |p| matches!(p, C162::Position(_)))
	);
	assert!(back
		.iter()
		.any(|p| matches!(p, C162::EntityTeleport(e) if e.entity_id == 71 && e.y == 214.0)));

	let (sent, back) = up_16_2(&walk(30.0), &mut cx);
	assert_eq!(server_y(&sent), 30.0);
	assert_eq!(
		first_section_block(map_chunk(&back, 5, 5).unwrap()),
		expected_block(0)
	);
	let (sent, back) = up_16_2(&S162::TeleportConfirm(id), &mut cx);
	assert!(sent.is_empty() && back.is_empty());
	let (sent, back) = up_16_2(&walk(94.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (30.0, 0));
	let (sent, back) = up_16_2(&walk(4.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (-60.0, 0));

	let flying = |y: f64| {
		S162::PositionLook(v1_16_2::play_to_server_packet_position_look::Ty {
			x: 8.0,
			y,
			z: 8.0,
			yaw: 0.0,
			pitch: 0.0,
			on_ground: false,
		})
	};
	let (sent, back) = up_16_2(&flying(127.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (63.0, 0));
	let (sent, back) = up_16_2(&flying(128.0), &mut cx);
	assert_eq!(server_y(&sent), 64.0);
	let id = switched(&back, -64.0);
	assert_eq!(
		first_section_block(map_chunk(&back, 0, 0).unwrap()),
		expected_block(4)
	);
	assert!(up_16_2(&S162::TeleportConfirm(id), &mut cx).0.is_empty());

	let boat = |y: f64| {
		S162::VehicleMove(v1_16_2::play_to_server_packet_vehicle_move::Ty {
			x: 8.0,
			y,
			z: 8.0,
			yaw: 90.0,
			pitch: 0.0,
		})
	};
	let (sent, back) = up_16_2(&boat(224.0), &mut cx);
	assert_eq!(
		(
			server_y(&sent),
			back.iter()
				.filter(|p| matches!(p, C162::Position(_)))
				.count()
		),
		(224.0, 0)
	);
	let (sent, back) = up_16_2(&boat(225.0), &mut cx);
	assert_eq!(server_y(&sent), 225.0);
	switched(&back, -64.0);
	assert!(back
		.iter()
		.any(|p| matches!(p, C162::VehicleMove(v) if v.y == 161.0 && v.yaw == 90.0)));
	assert_eq!(
		first_section_block(map_chunk(&back, 0, 0).unwrap()),
		expected_block(8)
	);
}

#[test]
fn tall_worlds_hold_back_client_moves_until_teleports_land() {
	let mut cx = Session::new();
	tall_world(&mut cx, -64, 384);
	arrive(100.0, &mut cx);
	let out = down_17(&teleport_17(-40.0, 0, 3), &mut cx);
	assert_eq!(teleport(&out).y, 24.0);
	let (sent, back) = up_16_2(&walk(100.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (100.0, 0));
	assert_eq!(confirm(3, &mut cx), [S17::TeleportConfirm(3)]);
	let (sent, back) = up_16_2(&walk(24.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (-40.0, 0));
	let near = down_17(&teleport_17(4.0, 2, 4), &mut cx);
	assert_eq!((teleport(&near).y, teleport(&near).flags), (4.0, 2));
}

#[test]
fn very_tall_worlds_use_more_windows() {
	let mut cx = Session::new();
	tall_world(&mut cx, 0, 1024);
	arrive(100.0, &mut cx);
	let (sent, back) = up_16_2(&walk(224.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (224.0, 0));
	let (_, back) = up_16_2(&walk(225.0), &mut cx);
	let id = switched(&back, -128.0);
	assert_eq!(
		first_section_block(map_chunk(&back, 0, 0).unwrap()),
		expected_block(8)
	);
	up_16_2(&S162::TeleportConfirm(id), &mut cx);
	let (sent, back) = up_16_2(&walk(225.0), &mut cx);
	assert_eq!(server_y(&sent), 353.0);
	let id = switched(&back, -128.0);
	up_16_2(&S162::TeleportConfirm(id), &mut cx);
	let (sent, back) = up_16_2(&walk(32.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (288.0, 0));
	let (_, back) = up_16_2(&walk(31.0), &mut cx);
	switched(&back, 128.0);
	let out = down_17(&teleport_17(1000.0, 0, 3), &mut cx);
	assert_eq!(teleport(&out).y, 232.0);

	let mut cx = Session::new();
	down_17(&login_17(-2032, 4064, &mut cx), &mut cx);
	for (y, client) in [
		(2000.0, 224.0),
		(-2000.0, 32.0),
		(70.0, 70.0),
		(400.0, 144.0),
	] {
		let out = down_17(&teleport_17(y, 0, 2), &mut cx);
		assert_eq!(teleport(&out).y, client, "server y {y}");
	}
}

#[test]
fn window_floors_show_the_ground_below() {
	let mut cx = Session::new();
	down_17(&login_17(-64, 384, &mut cx), &mut cx);
	let deepslate = state("1.17", "minecraft:deepslate[axis=y]");
	let stone = state("1.17", "minecraft:stone");
	let blocks: Vec<Option<u32>> = (0..24)
		.map(|k| match k {
			0..4 => Some(deepslate),
			4 => None,
			_ => Some(stone),
		})
		.collect();
	let C162::MapChunk(c) = one(down_17(&filled_chunk(0, 0, &blocks), &mut cx)) else {
		panic!()
	};
	let floor = state("1.16.2", "minecraft:blackstone");
	let air = state("1.16.2", "minecraft:air");
	assert_eq!(c.bit_map, 0xffff);
	let s = sections(&c);
	assert!(s[0][..256].iter().all(|b| *b == floor) && s[0][256..].iter().all(|b| *b == air));
	let P::LongArray(h) = &compound(&c.heightmaps.value)[0].value else {
		panic!()
	};
	assert_eq!(unpack(h, 9, 256)[0], 200);

	let C162::BlockChange(b) = one(down_17(
		&block_change_17(3, -1, 4, "minecraft:air"),
		&mut cx,
	)) else {
		panic!()
	};
	assert_eq!((b.location.y, b.r#type as u32), (0, floor));
	let C162::BlockChange(b) = one(down_17(
		&block_change_17(3, 0, 4, "minecraft:stone"),
		&mut cx,
	)) else {
		panic!()
	};
	assert_eq!(
		(b.location.y, b.r#type as u32),
		(0, state("1.16.2", "minecraft:stone"))
	);
	let C162::BlockChange(b) = one(down_17(&block_change_17(3, 0, 4, "minecraft:air"), &mut cx))
	else {
		panic!()
	};
	assert_eq!((b.location.y, b.r#type as u32), (0, floor));
	let C162::BlockChange(b) = one(down_17(
		&block_change_17(3, 5, 4, "minecraft:stone"),
		&mut cx,
	)) else {
		panic!()
	};
	assert_eq!(b.location.y, 5);

	let out = down_17(&teleport_17(-40.0, 0, 2), &mut cx);
	let c = map_chunk(&out, 0, 0).unwrap();
	assert_eq!(c.bit_map, 0xffff);
	let s = sections(&c);
	assert!(s[0].iter().all(|b| *b == floor));
	assert_eq!(s[3][(15 << 8) | (4 << 4) | 3], air);
	assert_eq!(
		s[4][(5 << 8) | (4 << 4) | 3],
		state("1.16.2", "minecraft:stone")
	);
	assert_eq!(s[4].iter().filter(|b| **b != air).count(), 1);
	let out = down_17(&teleport_17(300.0, 0, 3), &mut cx);
	let c = map_chunk(&out, 0, 0).unwrap();
	assert!(sections(c)
		.iter()
		.all(|s| s.iter().all(|b| *b == state("1.16.2", "minecraft:stone"))));
	let mut cx = Session::new();
	down_17(&login_17(-64, 384, &mut cx), &mut cx);
	let low: Vec<Option<u32>> = (0..24).map(|k| (k < 8).then_some(stone)).collect();
	down_17(&filled_chunk(0, 0, &low), &mut cx);
	let out = down_17(&teleport_17(300.0, 0, 2), &mut cx);
	let c = map_chunk(&out, 0, 0).unwrap();
	let s = sections(c);
	assert_eq!(c.bit_map, 0x0f);
	assert!(
		s[0][..256]
			.iter()
			.all(|b| *b == state("1.16.2", "minecraft:stone"))
			&& s[0][256..].iter().all(|b| *b == air)
	);
	assert!(s[1..].iter().all(|s| s.iter().all(|b| *b == air)));
}

#[test]
fn relocated_light_fills_unknown_sky() {
	let mut cx = Session::new();
	down_17(&login_17(-64, 384, &mut cx), &mut cx);
	let mask = bits(12);
	down_17(
		&C17::UpdateLight(v1_17::play_to_client_packet_update_light::Ty {
			chunk_x: 0,
			chunk_z: 0,
			trust_edges: true,
			sky_light_mask: mask,
			block_light_mask: vec![],
			empty_sky_light_mask: vec![],
			empty_block_light_mask: vec![],
			sky_light: (0..12).map(|k| vec![k as u8; 2048]).collect(),
			block_light: vec![],
		}),
		&mut cx,
	);
	down_17(&tall_chunk(0, 0, 24), &mut cx);
	let out = down_17(&teleport_17(300.0, 0, 2), &mut cx);
	let light = out
		.iter()
		.find_map(|p| {
			if let C162::UpdateLight(l) = p {
				Some(l)
			} else {
				None
			}
		})
		.unwrap();
	assert_eq!(
		(light.sky_light_mask, light.block_light_mask),
		((1 << 18) - 1, (1 << 18) - 1)
	);
	assert_eq!(
		(light.empty_sky_light_mask, light.empty_block_light_mask),
		(0, 0)
	);
	let mut expected: Vec<u8> = (8..12).collect();
	expected.extend([0xff; 14]);
	assert_eq!(sky(light), expected);
	let out = down_17(&teleport_17(-40.0, 0, 3), &mut cx);
	let light = out
		.iter()
		.find_map(|p| {
			if let C162::UpdateLight(l) = p {
				Some(l)
			} else {
				None
			}
		})
		.unwrap();
	let mut expected: Vec<u8> = (0..12).collect();
	expected.extend([0xff; 6]);
	assert_eq!(sky(light), expected);
}

#[test]
fn respawns_unload_the_old_world() {
	let mut cx = Session::new();
	let login = login_17(-64, 384, &mut cx);
	down_17(&login, &mut cx);
	down_17(&tall_chunk(0, 0, 24), &mut cx);
	down_17(&tall_chunk(1, -2, 24), &mut cx);
	down_17(&teleport_17(-40.0, 0, 2), &mut cx);
	let C17::Login(l) = login else { panic!() };
	let respawn = C17::Respawn(v1_17::play_to_client_packet_respawn::Ty {
		dimension: l.dimension,
		world_name: l.world_name,
		hashed_seed: 0,
		gamemode: 0,
		previous_gamemode: 255,
		is_debug: false,
		is_flat: false,
		copy_metadata: false,
	});
	let out = down_17(&respawn, &mut cx);
	let unloaded: Vec<(i32, i32)> = out
		.iter()
		.filter_map(|p| {
			if let C162::UnloadChunk(u) = p {
				Some((u.chunk_x, u.chunk_z))
			} else {
				None
			}
		})
		.collect();
	assert_eq!(unloaded, [(0, 0), (1, -2)]);
	assert!(matches!(out.last(), Some(C162::Respawn(_))));
	let C162::MapChunk(c) = one(down_17(&tall_chunk(0, 0, 24), &mut cx)) else {
		panic!()
	};
	assert_eq!(first_section_block(&c), expected_block(4));
	let out = down_17(&teleport_17(70.0, 0, 3), &mut cx);
	assert_eq!(teleport(&out).y, 70.0);
	assert!(map_chunk(&out, 0, 0).is_none());
}

#[test]
fn short_worlds_are_not_shifted() {
	let mut cx = Session::new();
	down_17(&login_17(0, 256, &mut cx), &mut cx);
	let C162::MapChunk(c) = one(down_17(&tall_chunk(2, 3, 16), &mut cx)) else {
		panic!()
	};
	assert_eq!((c.x, c.z, c.bit_map), (2, 3, 0xffff));
	assert_eq!(first_section_block(&c), expected_block(0));
	assert_eq!(c.block_entities.len(), 0);
	let C162::Position(p) = one(down_17(&position_17(200.0, 0), &mut cx)) else {
		panic!()
	};
	assert_eq!(p.y, 200.0);
	let change = C17::BlockChange(v1_17::play_to_client_packet_block_change::Ty {
		location: v1_17::position::Ty { x: 3, y: 5, z: 4 },
		r#type: state("1.17", "minecraft:stone") as i32,
	});
	let C162::BlockChange(b) = one(down_17(&change, &mut cx)) else {
		panic!()
	};
	assert_eq!(b.location.y, 5);
}

fn down_16(p: &C16, cx: &mut Session) -> Vec<C15> {
	run(
		Root::PlayToClient,
		"1.16",
		"1.15.2",
		&enc(p, v1_16::play_to_client_packet::serialize),
		cx,
	)
	.iter()
	.map(|b| dec(b, v1_15_2::play_to_client_packet::deserialize))
	.collect()
}

fn block_change_15(text: &str) -> C15 {
	C15::BlockChange(v1_15_2::play_to_client_packet_block_change::Ty {
		location: v1_15_2::position::Ty { x: 0, y: 64, z: 0 },
		r#type: state("1.15.2", text) as i32,
	})
}

#[test]
fn changed_block_properties() {
	let mut cx = Session::new();
	let wall = block_change_15("minecraft:cobblestone_wall[east=true,north=false,south=true,up=true,waterlogged=false,west=false]");
	let C16::BlockChange(b) = one(up_15(
		&wall,
		"1.16",
		&mut cx,
		v1_16::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(describe("1.16", b.r#type as u32), "minecraft:cobblestone_wall[east=low,north=none,south=low,up=true,waterlogged=false,west=none]");
	let C15::BlockChange(b) = one(down_16(&C16::BlockChange(b), &mut cx)) else {
		panic!()
	};
	assert_eq!(describe("1.15.2", b.r#type as u32), "minecraft:cobblestone_wall[east=true,north=false,south=true,up=true,waterlogged=false,west=false]");
	let jigsaw = block_change_15("minecraft:jigsaw[facing=up]");
	let C16::BlockChange(b) = one(up_15(
		&jigsaw,
		"1.16",
		&mut cx,
		v1_16::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		describe("1.16", b.r#type as u32),
		"minecraft:jigsaw[orientation=up_north]"
	);
	let cauldron = block_change_15("minecraft:cauldron[level=2]");
	let C17::BlockChange(b) = one(up_15(
		&cauldron,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		describe("1.17", b.r#type as u32),
		"minecraft:water_cauldron[level=2]"
	);
	let lava = C17::BlockChange(v1_17::play_to_client_packet_block_change::Ty {
		location: v1_17::position::Ty { x: 0, y: 64, z: 0 },
		r#type: state("1.17", "minecraft:lava_cauldron") as i32,
	});
	let C162::BlockChange(b) = one(down_17(&lava, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		describe("1.16.2", b.r#type as u32),
		"minecraft:cauldron[level=3]"
	);
	let deepslate = C17::BlockChange(v1_17::play_to_client_packet_block_change::Ty {
		location: v1_17::position::Ty { x: 0, y: 64, z: 0 },
		r#type: state("1.17", "minecraft:deepslate[axis=y]") as i32,
	});
	let C162::BlockChange(b) = one(down_17(&deepslate, &mut cx)) else {
		panic!()
	};
	assert_eq!(describe("1.16.2", b.r#type as u32), "minecraft:blackstone");
}

#[test]
fn newer_items_are_substituted_and_restored() {
	let mut cx = Session::new();
	let copper = C17::SetSlot(v1_17::play_to_client_packet_set_slot::Ty {
		window_id: 0,
		slot: 36,
		item: slot("1.17", "copper_ingot", 3),
	});
	let C162::SetSlot(s) = one(down_17(&copper, &mut cx)) else {
		panic!()
	};
	assert_eq!(slot_name("1.16.2", &s.item), "brick");
	let v1_15_2::slot::Ty::True(item) = &s.item else {
		panic!()
	};
	let display =
		compound(get(compound(&item.nbt_data.as_ref().unwrap().value), "display").unwrap());
	assert!(string(get(display, "Name")).contains("Copper Ingot"));
	let click = S162::WindowClick(v1_16_2::play_to_server_packet_window_click::Ty {
		window_id: 0,
		slot: 36,
		mouse_button: 0,
		action: 1,
		mode: 0,
		item: s.item.clone(),
	});
	let (sent, _) = up_16_2(&click, &mut cx);
	let [S17::WindowClick(c)] = sent.as_slice() else {
		panic!()
	};
	assert_eq!(slot_name("1.17", &c.cursor_item), "copper_ingot");
	assert_eq!(c.cursor_item, slot("1.17", "copper_ingot", 3));
	let sword = C16::SetSlot(v1_16::play_to_client_packet_set_slot::Ty {
		window_id: 0,
		slot: 36,
		item: slot("1.16", "netherite_sword", 1),
	});
	let C15::SetSlot(s) = one(down_16(&sword, &mut cx)) else {
		panic!()
	};
	assert_eq!(slot_name("1.15.2", &s.item), "diamond_sword");
}

#[test]
fn newer_entities_are_substituted() {
	let mut cx = Session::new();
	let goat = C17::SpawnEntityLiving(v1_17::play_to_client_packet_spawn_entity_living::Ty {
		entity_id: 80,
		entity_uuid: 3,
		r#type: entity("1.17", "goat"),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		yaw: 0,
		pitch: 0,
		head_pitch: 0,
		velocity: v1_17::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let C162::SpawnEntityLiving(g) = one(down_17(&goat, &mut cx)) else {
		panic!()
	};
	assert_eq!(g.r#type, entity("1.16.2", "sheep"));
	use v1_17::entity_metadata_item::Ty as M;
	let entry =
		|key: u8, r#type: i32, value: M| v1_17::entity_metadata_items::Ty { key, r#type, value };
	let meta = C17::EntityMetadata(v1_17::play_to_client_packet_entity_metadata::Ty {
		entity_id: 80,
		metadata: vec![
			entry(0, 0, M::V0(0)),
			entry(16, 7, M::V7(true)),
			entry(17, 7, M::V7(true)),
		],
	});
	let C162::EntityMetadata(m) = one(down_17(&meta, &mut cx)) else {
		panic!()
	};
	assert_eq!(m.metadata.iter().map(|e| e.key).collect::<Vec<_>>(), [0]);
	let marker = C17::SpawnEntity(v1_17::play_to_client_packet_spawn_entity::Ty {
		entity_id: 81,
		object_uuid: 4,
		r#type: entity("1.17", "glow_item_frame"),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		pitch: 0,
		yaw: 0,
		object_data: 2,
		velocity: v1_17::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let C162::SpawnEntity(f) = one(down_17(&marker, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		(f.r#type, f.object_data),
		(entity("1.16.2", "item_frame"), 2)
	);
	let frame = C15::SpawnEntity(v1_15_2::play_to_client_packet_spawn_entity::Ty {
		entity_id: 82,
		object_uuid: 5,
		r#type: entity("1.15.2", "item_frame"),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		pitch: 10,
		yaw: 20,
		object_data: 3,
		velocity: v1_15_2::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let out = up_15(
		&frame,
		"1.17",
		&mut cx,
		v1_17::play_to_client_packet::deserialize,
	);
	assert!(
		matches!(out.as_slice(), [C17::SpawnEntity(_), C17::EntityLook(l)] if (l.entity_id, l.yaw, l.pitch) == (82, 20, 10))
	);
}

#[test]
fn texts_lose_newer_formatting_for_1_15_2() {
	let mut cx = Session::new();
	let message = r##"{"text":"hi","color":"#ff0000","hoverEvent":{"action":"show_item","contents":{"id":"minecraft:stone","count":2}}}"##;
	let chat = C16::Chat(v1_16::play_to_client_packet_chat::Ty {
		message: message.to_owned(),
		position: 0,
		sender: 7,
	});
	let C15::Chat(c) = one(down_16(&chat, &mut cx)) else {
		panic!()
	};
	let json: serde_json::Value = serde_json::from_str(&c.message).unwrap();
	assert_eq!(json["color"], "dark_red");
	assert_eq!(
		json["hoverEvent"]["value"],
		"{id:\"minecraft:stone\",Count:2b}"
	);
	let window = C16::OpenWindow(v1_16::play_to_client_packet_open_window::Ty {
		window_id: 2,
		inventory_type: 20,
		window_title: "{\"text\":\"x\"}".to_owned(),
	});
	let C15::OpenWindow(w) = one(down_16(&window, &mut cx)) else {
		panic!()
	};
	assert_eq!(w.inventory_type, 7);
	use v1_16::command_node_extra_node_data_v2_properties::Ty as Props;
	let node = |data: v1_16::command_node_extra_node_data::Ty| v1_16::command_node::Ty {
		flags: v1_16::command_node_flags::Ty {
			unused: 0,
			has_custom_suggestions: 0,
			has_redirect_node: 0,
			has_command: 0,
			command_node_type: 2,
		},
		children: vec![],
		redirect_node: v1_16::command_node_redirect_node::Ty::Default(0),
		extra_node_data: data,
	};
	let commands = C16::DeclareCommands(v1_16::play_to_client_packet_declare_commands::Ty {
		nodes: vec![node(v1_16::command_node_extra_node_data::Ty::V2(
			v1_16::command_node_extra_node_data_v2::Ty {
				name: "target".to_owned(),
				parser: "minecraft:uuid".to_owned(),
				properties: Props::MinecraftUuid,
				suggestion_type:
					v1_16::command_node_extra_node_data_v2_suggestion_type::Ty::Default(0),
			},
		))],
		root_index: 0,
	});
	let C15::DeclareCommands(d) = one(down_16(&commands, &mut cx)) else {
		panic!()
	};
	let v1_15_2::command_node_extra_node_data::Ty::V2(arg) = &d.nodes[0].extra_node_data else {
		panic!()
	};
	assert_eq!(arg.parser, "brigadier:string");
}

#[test]
fn respawns_and_container_data_for_1_15_2() {
	let mut cx = Session::new();
	let C16::Login(l) = one(up_15(
		&login_15(0),
		"1.16",
		&mut cx,
		v1_16::play_to_client_packet::deserialize,
	)) else {
		panic!()
	};
	down_16(&C16::Login(l), &mut cx);
	let attributes =
		C16::EntityUpdateAttributes(v1_16::play_to_client_packet_entity_update_attributes::Ty {
			entity_id: 7,
			properties: vec![
				v1_16::play_to_client_packet_entity_update_attributes_properties_items::Ty {
					key: "minecraft:generic.movement_speed".to_owned(),
					value: 0.1,
					modifiers: vec![],
				},
			],
		});
	down_16(&attributes, &mut cx);
	let respawn = C16::Respawn(v1_16::play_to_client_packet_respawn::Ty {
		dimension: "minecraft:overworld".to_owned(),
		world_name: "minecraft:other".to_owned(),
		hashed_seed: 0,
		gamemode: 0,
		previous_gamemode: 255,
		is_debug: false,
		is_flat: false,
		copy_metadata: true,
	});
	let out = down_16(&respawn, &mut cx);
	assert!(
		matches!(out.as_slice(), [C15::Respawn(a), C15::Respawn(b), C15::EntityUpdateAttributes(u)]
		if a.dimension == -1 && b.dimension == 0 && u.properties[0].key == "generic.movementSpeed")
	);
	let hint = |value: i16| {
		C16::CraftProgressBar(v1_16::play_to_client_packet_craft_progress_bar::Ty {
			window_id: 1,
			property: 4,
			value,
		})
	};
	assert!(
		matches!(down_16(&hint(11), &mut cx).as_slice(), [C15::CraftProgressBar(p)] if p.value == -1)
	);
	assert!(
		matches!(up_15(&C15::CraftProgressBar(v1_15_2::play_to_client_packet_craft_progress_bar::Ty { window_id: 1, property: 5, value: 11 }), "1.16", &mut cx, v1_16::play_to_client_packet::deserialize).as_slice(), [C16::CraftProgressBar(p)] if p.value == 12)
	);
	let stats = C16::Statistics(vec![v1_16::play_to_client_packet_statistics_items::Ty {
		category_id: 8,
		statistic_id: 25,
		value: 3,
	}]);
	assert!(
		matches!(down_16(&stats, &mut cx).as_slice(), [C15::Statistics(s)] if s[0].statistic_id == 24)
	);
	let map = C16::Map(v1_16::play_to_client_packet_map::Ty {
		item_damage: 1,
		scale: 0,
		tracking_position: true,
		locked: false,
		icons: vec![],
		columns: v1_16::play_to_client_packet_map_columns::Ty::Default(
			1,
			v1_16::play_to_client_packet_map_columns_default::Ty {
				rows: 1,
				x: 0,
				y: 0,
				data: vec![208, 5],
			},
		),
	});
	let C15::Map(m) = one(down_16(&map, &mut cx)) else {
		panic!()
	};
	let v1_15_2::play_to_client_packet_map_columns::Ty::Default(_, c) = &m.columns else {
		panic!()
	};
	assert_eq!(c.data, [113, 5]);
}

#[test]
fn state_ids_follow_the_server() {
	let mut cx = Session::new();
	let items = C171::WindowItems(v1_17_1::play_to_client_packet_window_items::Ty {
		window_id: 3,
		state_id: 42,
		items: vec![],
		carried_item: slot("1.17", "stone", 1),
	});
	let out: Vec<C17> = run(
		Root::PlayToClient,
		"1.17.1",
		"1.17",
		&enc(&items, v1_17_1::play_to_client_packet::serialize),
		&mut cx,
	)
	.iter()
	.map(|b| dec(b, v1_17::play_to_client_packet::deserialize))
	.collect();
	assert!(
		matches!(out.as_slice(), [C17::WindowItems(_), C17::SetSlot(s)] if s.window_id == -1 && slot_name("1.17", &s.item) == "stone")
	);
	let click = v1_17::play_to_server_packet::Ty::WindowClick(
		v1_17::play_to_server_packet_window_click::Ty {
			window_id: 3,
			slot: 1,
			mouse_button: 0,
			mode: 0,
			changed_slots: vec![],
			cursor_item: v1_15_2::slot::Ty::False,
		},
	);
	let out = run(
		Root::PlayToServer,
		"1.17",
		"1.17.1",
		&enc(&click, v1_17::play_to_server_packet::serialize),
		&mut cx,
	);
	let v1_17_1::play_to_server_packet::Ty::WindowClick(c) =
		dec(&out[0], v1_17_1::play_to_server_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(c.state_id, 42);
}

#[test]
fn particles_fall_back_for_older_clients() {
	let mut cx = Session::new();
	let p = C17::WorldParticles(v1_17::play_to_client_packet_world_particles::Ty {
		particle_id: registry::names("1.17", "particles")
			.unwrap()
			.id("dust_color_transition")
			.unwrap() as i32,
		long_distance: false,
		x: 0.0,
		y: 64.0,
		z: 0.0,
		offset_x: 0.0,
		offset_y: 0.0,
		offset_z: 0.0,
		particle_data: 0.0,
		particles: 1,
		data: v1_17::particle_data::Ty::V16(v1_17::particle_data_v16::Ty {
			from_red: 1.0,
			from_green: 0.5,
			from_blue: 0.0,
			scale: 2.0,
			to_red: 0.0,
			to_green: 0.0,
			to_blue: 1.0,
		}),
	});
	let C162::WorldParticles(w) = one(down_17(&p, &mut cx)) else {
		panic!()
	};
	assert!(
		matches!(&w.data, v1_16_2::particle_data::Ty::V14(d) if (d.red, d.green, d.scale) == (1.0, 0.5, 2.0))
	);
}
