use minecraft_versions::gen::convert::{convert, index_of, Root};
use minecraft_versions::gen::convert::{v1_15_2, v1_16, v1_16_2, v1_17, v1_17_1};
use minecraft_versions::pds::{Input, Output, Result};
use minecraft_versions::registry;
use minecraft_versions::Session;
use nbt::gen::pds_Entry::Ty as Entry;
use nbt::gen::pds_List_0::Ty as Items;
use nbt::gen::pds_Payload::Ty as P;
use nbt::CompoundExt;

type C15 = v1_15_2::play_toClient_packet::Ty;
type C16 = v1_16::play_toClient_packet::Ty;
type C162 = v1_16_2::play_toClient_packet::Ty;
type C17 = v1_17::play_toClient_packet::Ty;
type C171 = v1_17_1::play_toClient_packet::Ty;

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
		&enc(p, v1_15_2::play_toClient_packet::serialize),
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
		&enc(p, v1_17::play_toClient_packet::serialize),
		cx,
	)
	.iter()
	.map(|b| dec(b, v1_16_2::play_toClient_packet::deserialize))
	.collect()
}

fn get<'a>(c: &'a [Entry], name: &str) -> Option<&'a P> {
	c.field(name)
}

fn compound(p: &P) -> &Vec<Entry> {
	match p {
		P::compound(c) => c,
		p => panic!("not a compound: {p:?}"),
	}
}

fn compounds(p: &P) -> &Vec<Vec<Entry>> {
	match p {
		P::list(l) => match &l.items {
			Items::compound(c) => c,
			i => panic!("not a compound list: {i:?}"),
		},
		p => panic!("not a list: {p:?}"),
	}
}

fn string(p: Option<&P>) -> &str {
	match p {
		Some(P::string(s)) => s,
		p => panic!("not a string: {p:?}"),
	}
}

fn int(p: Option<&P>) -> i32 {
	match p {
		Some(P::int(v)) => *v,
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
	C15::login(v1_15_2::play_toClient_packet_login::Ty {
		entityId: 7,
		gameMode: 1 | 8,
		dimension,
		hashedSeed: 42,
		maxPlayers: 20,
		levelType: "flat".to_owned(),
		viewDistance: 10,
		reducedDebugInfo: false,
		enableRespawnScreen: true,
	})
}

#[test]
fn join_game_builds_codecs() {
	let mut cx = Session::new();
	let C16::login(l) = one(up_15(
		&login_15(-1),
		"1.16",
		&mut cx,
		v1_16::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		(
			l.dimension.as_str(),
			l.worldName.as_str(),
			l.isFlat,
			l.previousGameMode
		),
		("minecraft:the_nether", "minecraft:the_nether", true, 255)
	);
	let dims = compounds(get(compound(&l.dimensionCodec.value), "dimension").unwrap());
	assert_eq!(dims.len(), 4);
	assert!(dims
		.iter()
		.any(|d| string(get(d, "name")) == "minecraft:the_nether"
			&& int(get(d, "logical_height")) == 128));
	assert_eq!(l.worldNames.len(), 3);

	let C162::login(l) = one(up_15(
		&login_15(-1),
		"1.16.2",
		&mut cx,
		v1_16_2::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert!(l.isHardcore);
	assert_eq!((l.gameMode, l.maxPlayers), (1, 20));
	let codec = compound(&l.dimensionCodec.value);
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

	let C17::login(l) = one(up_15(
		&login_15(0),
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	let dim = compound(&l.dimension.value);
	assert_eq!((int(get(dim, "min_y")), int(get(dim, "height"))), (0, 256));
	let types = compounds(
		get(
			compound(
				get(
					compound(&l.dimensionCodec.value),
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
	let p = C15::respawn(v1_15_2::play_toClient_packet_respawn::Ty {
		dimension: 1,
		hashedSeed: 5,
		gamemode: 0,
		levelType: "default".to_owned(),
	});
	let C16::respawn(r) = one(up_15(
		&p,
		"1.16",
		&mut cx,
		v1_16::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		(r.dimension.as_str(), r.isFlat, r.copyMetadata),
		("minecraft:the_end", false, true)
	);
	let C17::respawn(r) = one(up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
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
		&enc(&C17::respawn(r), v1_17::play_toClient_packet::serialize),
		&mut cx,
	);
	let C15::respawn(r) = one(back
		.iter()
		.map(|b| dec(b, v1_15_2::play_toClient_packet::deserialize))
		.collect::<Vec<_>>())
	else {
		panic!()
	};
	assert_eq!(r.dimension, 1);
}

#[test]
fn chat_and_login_success() {
	let mut cx = Session::new();
	let C16::chat(c) = one(up_15(
		&C15::chat(v1_15_2::play_toClient_packet_chat::Ty {
			message: "{\"text\":\"hi\"}".to_owned(),
			position: 1,
		}),
		"1.16",
		&mut cx,
		v1_16::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!((c.position, c.sender), (1, 0));
	let p =
		v1_15_2::login_toClient_packet::Ty::success(v1_15_2::login_toClient_packet_success::Ty {
			uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".to_owned(),
			username: "Notch".to_owned(),
		});
	let out = run(
		Root::LoginToClient,
		"1.15.2",
		"1.17.1",
		&enc(&p, v1_15_2::login_toClient_packet::serialize),
		&mut cx,
	);
	let v1_17_1::login_toClient_packet::Ty::success(s) =
		dec(&out[0], v1_17_1::login_toClient_packet::deserialize)
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
		P::compound(vec![Entry::new(
			"MOTION_BLOCKING",
			P::longArray(pack(&heights, 9, true)),
		)]),
	);
	let p = C15::map_chunk(v1_15_2::play_toClient_packet_map_chunk::Ty {
		x: 4,
		z: -9,
		bitMap: 0b101,
		heightmaps,
		biomes: if full {
			v1_15_2::play_toClient_packet_map_chunk__biomes::Ty::r#true(
				(0..1024).map(|i| i % 3).collect(),
			)
		} else {
			v1_15_2::play_toClient_packet_map_chunk__biomes::Ty::r#false
		},
		chunkData: data,
		blockEntities: vec![],
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
	let C16::map_chunk(c) = one(up_15(
		&p,
		"1.16",
		&mut cx,
		v1_16::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert!(c.ignoreOldData);
	let P::longArray(h) = &compound(&c.heightmaps.value)[0].value else {
		panic!()
	};
	assert_eq!(h.len(), 37);
	assert_eq!(
		unpack(h, 9, 256),
		(0..256).map(|i| (i * 3 % 257) as u32).collect::<Vec<_>>()
	);
	let C17::map_chunk(c) = one(up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		(c.x, c.z, c.bitMap.as_slice(), c.biomes.len()),
		(4, -9, &[0b101i64][..], 1024)
	);
	let mut at = 0;
	for (n, src) in sections.iter().enumerate() {
		let (count, palette, states) = read_section(&c.chunkData, &mut at, global_bits("1.17"));
		assert_eq!(palette.is_none(), n == 0, "section {n} palette kind");
		let air = state("1.17", "minecraft:air");
		assert_eq!(count as usize, states.iter().filter(|s| **s != air).count());
		for (i, (a, b)) in src.iter().zip(&states).enumerate() {
			if i % 37 == 0 || n == 1 {
				same_block("1.15.2", *a, "1.17", *b);
			}
		}
	}
	assert_eq!(at, c.chunkData.len());
	let stairs = state(
		"1.17",
		"minecraft:oak_stairs[facing=east,half=top,shape=outer_left,waterlogged=true]",
	);
	let mut at = 0;
	read_section(&c.chunkData, &mut at, global_bits("1.17"));
	assert!(read_section(&c.chunkData, &mut at, global_bits("1.17"))
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
		v1_17::play_toClient_packet::deserialize,
	);
	assert_eq!(out.len(), 2);
	for (packet, (y, src)) in out.iter().zip([(0u32, &sections[0]), (2, &sections[1])]) {
		let C17::multi_block_change(m) = packet else {
			panic!("{packet:?}")
		};
		assert_eq!(
			(
				m.chunkCoordinates.x,
				m.chunkCoordinates.z,
				m.chunkCoordinates.y
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
	let p = C15::update_light(v1_15_2::play_toClient_packet_update_light::Ty {
		chunkX: 1,
		chunkZ: 2,
		skyLightMask: 0b11,
		blockLightMask: 0b1000,
		emptySkyLightMask: 0b100,
		emptyBlockLightMask: 0,
		data,
	});
	let C16::update_light(l) = one(up_15(
		&p,
		"1.16",
		&mut cx,
		v1_16::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert!(l.trustEdges);
	let C17::update_light(l) = one(up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		(
			l.skyLightMask.as_slice(),
			l.blockLightMask.as_slice(),
			l.emptySkyLightMask.as_slice()
		),
		(&[3i64][..], &[8i64][..], &[4i64][..])
	);
	assert_eq!(
		l.skyLight
			.iter()
			.map(|a| (a.len(), a[0]))
			.collect::<Vec<_>>(),
		[(2048, 1), (2048, 2)]
	);
	assert_eq!(l.blockLight[0][100], 3);
}

#[test]
fn block_updates_follow_the_registries() {
	let mut cx = Session::new();
	let path = state("1.15.2", "minecraft:grass_path") as i32;
	let p = C15::block_change(v1_15_2::play_toClient_packet_block_change::Ty {
		location: v1_15_2::position::Ty { x: 1, y: 70, z: -3 },
		r#type: path,
	});
	let C17::block_change(b) = one(up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(describe("1.17", b.r#type as u32), "minecraft:dirt_path");
	assert_eq!(b.location.y, 70);
	let stone = state("1.15.2", "minecraft:stone") as i32;
	let p = C15::multi_block_change(v1_15_2::play_toClient_packet_multi_block_change::Ty {
		chunkX: -2,
		chunkZ: 3,
		records: vec![
			v1_15_2::play_toClient_packet_multi_block_change__records__items::Ty {
				horizontalPos: 0x5a,
				y: 3,
				blockId: stone,
			},
			v1_15_2::play_toClient_packet_multi_block_change__records__items::Ty {
				horizontalPos: 0x12,
				y: 40,
				blockId: path,
			},
		],
	});
	let out = up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	);
	let changes: Vec<(u32, i64)> = out
		.iter()
		.map(|p| match p {
			C17::multi_block_change(m) => {
				assert_eq!(
					(m.chunkCoordinates.x, m.chunkCoordinates.z, m.records.len()),
					(-2, 3, 1)
				);
				(m.chunkCoordinates.y, m.records[0])
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
	v1_15_2::slot::Ty::r#true(v1_15_2::slot__true::Ty {
		itemId: item(version, name),
		itemCount: count,
		nbtData: None,
	})
}

fn slot_name(version: &str, s: &v1_15_2::slot::Ty) -> String {
	match s {
		v1_15_2::slot::Ty::r#true(s) => registry::names(version, "items")
			.unwrap()
			.name(s.itemId as u32)
			.unwrap()
			.to_owned(),
		v1_15_2::slot::Ty::r#false => "empty".to_owned(),
	}
}

#[test]
fn entities_and_metadata() {
	let mut cx = Session::new();
	let spawn = C15::spawn_entity(v1_15_2::play_toClient_packet_spawn_entity::Ty {
		entityId: 50,
		objectUUID: 1,
		r#type: entity("1.15.2", "arrow"),
		x: 0.5,
		y: 65.0,
		z: 0.5,
		pitch: 0,
		yaw: 0,
		objectData: 0,
		velocity: v1_15_2::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let C17::spawn_entity(s) = one(up_15(
		&spawn,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(s.r#type, entity("1.17", "arrow"));
	use v1_15_2::entityMetadataItem::Ty as M;
	let entry =
		|key: u8, r#type: i32, value: M| v1_15_2::entityMetadata__items::Ty { key, r#type, value };
	let meta = C15::entity_metadata(v1_15_2::play_toClient_packet_entity_metadata::Ty {
		entityId: 50,
		metadata: vec![
			entry(0, 0, M::v0(0x20)),
			entry(8, 12, M::v12(Some(9))),
			entry(9, 0, M::v0(3)),
		],
	});
	let C16::entity_metadata(m) = one(up_15(
		&meta,
		"1.16",
		&mut cx,
		v1_16::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	let keys: Vec<(u8, i32)> = m.metadata.iter().map(|e| (e.key, e.r#type)).collect();
	assert_eq!(keys, [(0, 0), (8, 0)]);
	let C17::entity_metadata(m) = one(up_15(
		&meta,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(m.metadata.iter().map(|e| e.key).collect::<Vec<_>>(), [0, 9]);

	let pig = C15::spawn_entity_living(v1_15_2::play_toClient_packet_spawn_entity_living::Ty {
		entityId: 51,
		entityUUID: 2,
		r#type: entity("1.15.2", "zombie_pigman"),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		yaw: 0,
		pitch: 0,
		headPitch: 0,
		velocity: v1_15_2::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let C171::spawn_entity_living(s) = one(up_15(
		&pig,
		"1.17.1",
		&mut cx,
		v1_17_1::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(s.r#type, entity("1.17", "zombified_piglin"));

	let item_entity = C15::entity_metadata(v1_15_2::play_toClient_packet_entity_metadata::Ty {
		entityId: 52,
		metadata: vec![
			entry(7, 6, M::v6(slot("1.15.2", "diamond_sword", 1))),
			entry(6, 18, M::v18(6)),
		],
	});
	let C17::entity_metadata(m) = one(up_15(
		&item_entity,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	use v1_17::entityMetadataItem::Ty as M17;
	assert!(
		matches!(&m.metadata[0], e if e.key == 8 && matches!(&e.value, M17::v6(s) if slot_name("1.17", s) == "diamond_sword"))
	);
	assert!(matches!(&m.metadata[1], e if e.key == 6 && matches!(e.value, M17::v18(7))));

	let wolf = C15::spawn_entity_living(v1_15_2::play_toClient_packet_spawn_entity_living::Ty {
		entityId: 53,
		entityUUID: 3,
		r#type: entity("1.15.2", "wolf"),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		yaw: 0,
		pitch: 0,
		headPitch: 0,
		velocity: v1_15_2::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	up_15(
		&wolf,
		"1.16",
		&mut cx,
		v1_16::play_toClient_packet::deserialize,
	);
	let angry = C15::entity_metadata(v1_15_2::play_toClient_packet_entity_metadata::Ty {
		entityId: 53,
		metadata: vec![entry(16, 0, M::v0(2))],
	});
	let C16::entity_metadata(m) = one(up_15(
		&angry,
		"1.16",
		&mut cx,
		v1_16::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert!(m
		.metadata
		.iter()
		.any(|e| e.key == 20 && e.value == v1_16::entityMetadataItem::Ty::v1(i32::MAX)));
}

#[test]
fn equipment_becomes_a_list() {
	let mut cx = Session::new();
	let p = C15::entity_equipment(v1_15_2::play_toClient_packet_entity_equipment::Ty {
		entityId: 3,
		slot: 5,
		item: slot("1.15.2", "golden_helmet", 1),
	});
	let bytes = run(
		Root::PlayToClient,
		"1.15.2",
		"1.16",
		&enc(&p, v1_15_2::play_toClient_packet::serialize),
		&mut cx,
	);
	let C16::entity_equipment(e) = dec(&bytes[0], v1_16::play_toClient_packet::deserialize) else {
		panic!()
	};
	assert_eq!(
		(e.entityId, e.equipments.len(), e.equipments[0].slot),
		(3, 1, 5)
	);
	assert_eq!(slot_name("1.16", &e.equipments[0].item), "golden_helmet");
	let two = C16::entity_equipment(v1_16::play_toClient_packet_entity_equipment::Ty {
		entityId: 3,
		equipments: vec![
			v1_16::play_toClient_packet_entity_equipment__equipments__items::Ty {
				slot: 0,
				item: slot("1.16", "netherite_sword", 1),
			},
			v1_16::play_toClient_packet_entity_equipment__equipments__items::Ty {
				slot: 1,
				item: slot("1.16", "shield", 1),
			},
		],
	});
	let back = run(
		Root::PlayToClient,
		"1.16",
		"1.15.2",
		&enc(&two, v1_16::play_toClient_packet::serialize),
		&mut cx,
	);
	let back: Vec<C15> = back
		.iter()
		.map(|b| dec(b, v1_15_2::play_toClient_packet::deserialize))
		.collect();
	assert!(
		matches!(back.as_slice(), [C15::entity_equipment(a), C15::entity_equipment(b)]
		if a.slot == 0 && slot_name("1.15.2", &a.item) == "diamond_sword" && b.slot == 1 && slot_name("1.15.2", &b.item) == "shield")
	);
}

#[test]
fn pings_become_transactions_for_1_16_clients() {
	let mut cx = Session::new();
	let C162::transaction(t) = one(down_17(&C17::ping(77), &mut cx)) else {
		panic!()
	};
	assert_eq!((t.windowId, t.action, t.accepted), (0, 77, false));
	assert!(down_17(&C17::ping(1 << 20), &mut cx).is_empty());
	let reply = v1_16_2::play_toServer_packet::Ty::transaction(
		v1_16_2::play_toServer_packet_transaction::Ty {
			windowId: 0,
			action: 77,
			accepted: true,
		},
	);
	let out = run(
		Root::PlayToServer,
		"1.16.2",
		"1.17",
		&enc(&reply, v1_16_2::play_toServer_packet::serialize),
		&mut cx,
	);
	assert_eq!(
		dec(&out[0], v1_17::play_toServer_packet::deserialize),
		v1_17::play_toServer_packet::Ty::pong(77)
	);
}

#[test]
fn lightning_becomes_an_entity() {
	let mut cx = Session::new();
	let p = C15::spawn_entity_weather(v1_15_2::play_toClient_packet_spawn_entity_weather::Ty {
		entityId: 9,
		r#type: 1,
		x: 1.0,
		y: 2.0,
		z: 3.0,
	});
	let C16::spawn_entity(s) = one(up_15(
		&p,
		"1.16",
		&mut cx,
		v1_16::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!((s.r#type, s.y), (entity("1.16", "lightning_bolt"), 2.0));
}

#[test]
fn items_particles_and_sounds() {
	let mut cx = Session::new();
	let p = C15::set_slot(v1_15_2::play_toClient_packet_set_slot::Ty {
		windowId: 0,
		slot: 36,
		item: slot("1.15.2", "grass_path", 5),
	});
	let C171::set_slot(s) = one(up_15(
		&p,
		"1.17.1",
		&mut cx,
		v1_17_1::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		(slot_name("1.17", &s.item), s.stateId),
		("dirt_path".to_owned(), 0)
	);
	let p = C15::window_items(v1_15_2::play_toClient_packet_window_items::Ty {
		windowId: 0,
		items: vec![
			slot("1.15.2", "zombie_pigman_spawn_egg", 1),
			v1_15_2::slot::Ty::r#false,
			slot("1.15.2", "shield", 1),
		],
	});
	let C171::window_items(w) = one(up_15(
		&p,
		"1.17.1",
		&mut cx,
		v1_17_1::play_toClient_packet::deserialize,
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
	assert_eq!(w.carriedItem, v1_15_2::slot::Ty::r#false);

	let particle = C15::world_particles(v1_15_2::play_toClient_packet_world_particles::Ty {
		particleId: 32,
		longDistance: false,
		x: 0.0,
		y: 0.0,
		z: 0.0,
		offsetX: 0.0,
		offsetY: 0.0,
		offsetZ: 0.0,
		particleData: 0.0,
		particles: 1,
		data: v1_15_2::particleData::Ty::v32(slot("1.15.2", "apple", 1)),
	});
	let C17::world_particles(w) = one(up_15(
		&particle,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(w.particleId, 36);
	assert!(matches!(&w.data, v1_17::particleData::Ty::v36(s) if slot_name("1.17", s) == "apple"));

	let sounds = registry::names("1.16", "sounds").unwrap();
	let id = sounds.id("entity.player.levelup").unwrap() as i32;
	assert!(id > 631);
	let p = v1_16::play_toClient_packet::Ty::sound_effect(
		v1_16::play_toClient_packet_sound_effect::Ty {
			soundId: id,
			soundCategory: 0,
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
		&enc(&p, v1_16_1::play_toClient_packet::serialize),
		&mut cx,
	);
	let C162::sound_effect(s) = dec(&out[0], v1_16_2::play_toClient_packet::deserialize) else {
		panic!()
	};
	assert_eq!(s.soundId, id + 7);
	let out = run(
		Root::PlayToClient,
		"1.16.1",
		"1.17",
		&enc(&p, v1_16_1::play_toClient_packet::serialize),
		&mut cx,
	);
	let C17::sound_effect(s) = dec(&out[0], v1_17::play_toClient_packet::deserialize) else {
		panic!()
	};
	assert_eq!(
		registry::names("1.17", "sounds")
			.unwrap()
			.name(s.soundId as u32),
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
	let p = C15::declare_recipes(
		vec![v1_15_2::play_toClient_packet__declare_recipes__items::Ty {
		r#type: "minecraft:crafting_shaped".to_owned(),
		recipeId: "minecraft:test".to_owned(),
		data: v1_15_2::play_toClient_packet__declare_recipes__items__data::Ty::minecraft_crafting_shaped(
			v1_15_2::play_toClient_packet__declare_recipes__items__data__minecraft_crafting_shaped::Ty {
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
	let C162::declare_recipes(r) = one(up_15(
		&p,
		"1.16.2",
		&mut cx,
		v1_16_2::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	let v1_16_2::play_toClient_packet__declare_recipes__items__data::Ty::minecraft_crafting_shaped(
		s,
	) = &r[0].data
	else {
		panic!()
	};
	assert_eq!(flat(&s.ingredients, "1.16.2"), names);
	let C17::declare_recipes(r) = one(up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	let v1_17::play_toClient_packet__declare_recipes__items__data::Ty::minecraft_crafting_shaped(s) =
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
		|name: &str, entries: Vec<i32>| v1_15_2::play_toClient_packet_tags__blockTags__items::Ty {
			tagName: name.to_owned(),
			entries,
		};
	let p = C15::tags(v1_15_2::play_toClient_packet_tags::Ty {
		blockTags: vec![tag(
			"minecraft:logs",
			vec![registry::names("1.15.2", "blocks")
				.unwrap()
				.id("oak_log")
				.unwrap() as i32],
		)],
		itemTags: vec![tag("minecraft:furnace_materials", vec![])],
		fluidTags: vec![tag("minecraft:water", vec![1, 2])],
		entityTags: vec![],
	});
	let C16::tags(t) = one(up_15(
		&p,
		"1.16",
		&mut cx,
		v1_16::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	let climbable = t
		.blockTags
		.iter()
		.find(|t| t.tagName == "minecraft:climbable")
		.unwrap();
	let blocks = registry::names("1.16", "blocks").unwrap();
	assert!(climbable
		.entries
		.contains(&(blocks.id("ladder").unwrap() as i32)));
	assert_eq!(
		t.blockTags[0].entries,
		[blocks.id("oak_log").unwrap() as i32]
	);
	let C17::tags(groups) = one(up_15(
		&p,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	let types: Vec<&str> = groups.iter().map(|g| g.tagType.as_str()).collect();
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
		.all(|t| t.tagName != "minecraft:furnace_materials"));
	assert!(groups[4]
		.tags
		.iter()
		.any(|t| t.tagName == "minecraft:vibrations"));
	assert_eq!(groups[2].tags[0].entries, [1, 2]);
}

#[test]
fn split_packets_in_1_17() {
	let mut cx = Session::new();
	let title = C15::title(v1_15_2::play_toClient_packet_title::Ty::v2(
		"bar".to_owned(),
	));
	assert!(
		matches!(up_15(&title, "1.17", &mut cx, v1_17::play_toClient_packet::deserialize).as_slice(), [C17::action_bar(t)] if t == "bar")
	);
	let reset = C15::title(v1_15_2::play_toClient_packet_title::Ty::default(5));
	assert!(matches!(
		up_15(
			&reset,
			"1.17",
			&mut cx,
			v1_17::play_toClient_packet::deserialize
		)
		.as_slice(),
		[C17::clear_titles(true)]
	));
	let border = C15::world_border(v1_15_2::play_toClient_packet_world_border::Ty::v2(
		v1_15_2::play_toClient_packet_world_border__v2::Ty { x: 1.0, z: 2.0 },
	));
	assert!(
		matches!(up_15(&border, "1.17", &mut cx, v1_17::play_toClient_packet::deserialize).as_slice(), [C17::world_border_center(c)] if (c.x, c.z) == (1.0, 2.0))
	);
	let destroy = C15::entity_destroy(vec![4, 5]);
	assert!(matches!(
		up_15(
			&destroy,
			"1.17",
			&mut cx,
			v1_17::play_toClient_packet::deserialize
		)
		.as_slice(),
		[C17::destroy_entity(4), C17::destroy_entity(5)]
	));
	assert!(
		matches!(up_15(&destroy, "1.17.1", &mut cx, v1_17_1::play_toClient_packet::deserialize).as_slice(), [C171::entity_destroy(a), C171::entity_destroy(b)] if a == &[4] && b == &[5])
	);
	let rejected = C15::transaction(v1_15_2::play_toClient_packet_transaction::Ty {
		windowId: 3,
		action: 17,
		accepted: false,
	});
	let C17::ping(id) = one(up_15(
		&rejected,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	let out = run(
		Root::PlayToServer,
		"1.17",
		"1.15.2",
		&enc(
			&v1_17::play_toServer_packet::Ty::pong(id),
			v1_17::play_toServer_packet::serialize,
		),
		&mut cx,
	);
	let v1_15_2::play_toServer_packet::Ty::transaction(t) =
		dec(&out[0], v1_15_2::play_toServer_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!((t.windowId, t.action, t.accepted), (3, 17, true));
	let accepted = C15::transaction(v1_15_2::play_toClient_packet_transaction::Ty {
		windowId: 3,
		action: 18,
		accepted: true,
	});
	assert!(up_15(
		&accepted,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize
	)
	.is_empty());
}

#[test]
fn serverbound_reaches_1_15_2() {
	let mut cx = Session::new();
	let abilities = C15::abilities(v1_15_2::play_toClient_packet_abilities::Ty {
		flags: 4,
		flyingSpeed: 0.2,
		walkingSpeed: 0.3,
	});
	up_15(
		&abilities,
		"1.17.1",
		&mut cx,
		v1_17_1::play_toClient_packet::deserialize,
	);
	let server = |p: &v1_17_1::play_toServer_packet::Ty,
	              cx: &mut Session|
	 -> Vec<v1_15_2::play_toServer_packet::Ty> {
		run(
			Root::PlayToServer,
			"1.17.1",
			"1.15.2",
			&enc(p, v1_17_1::play_toServer_packet::serialize),
			cx,
		)
		.iter()
		.map(|b| dec(b, v1_15_2::play_toServer_packet::deserialize))
		.collect()
	};
	let v1_15_2::play_toServer_packet::Ty::abilities(a) = one(server(
		&v1_17_1::play_toServer_packet::Ty::abilities(2),
		&mut cx,
	)) else {
		panic!()
	};
	assert_eq!((a.flags, a.flyingSpeed, a.walkingSpeed), (2, 0.2, 0.3));
	let click = v1_17_1::play_toServer_packet::Ty::window_click(
		v1_17_1::play_toServer_packet_window_click::Ty {
			windowId: 1,
			stateId: 9,
			slot: 5,
			mouseButton: 0,
			mode: 0,
			changedSlots: vec![],
			cursorItem: slot("1.17", "dirt_path", 2),
		},
	);
	let v1_15_2::play_toServer_packet::Ty::window_click(c) = one(server(&click, &mut cx)) else {
		panic!()
	};
	assert_eq!((c.windowId, c.slot, c.action, c.mode), (1, 5, 0, 0));
	assert_eq!(slot_name("1.15.2", &c.item), "grass_path");
	let book =
		v1_17_1::play_toServer_packet::Ty::edit_book(v1_17_1::play_toServer_packet_edit_book::Ty {
			hand: 40,
			pages: vec!["one".to_owned(), "two".to_owned()],
			title: Some("T".to_owned()),
		});
	let v1_15_2::play_toServer_packet::Ty::edit_book(b) = one(server(&book, &mut cx)) else {
		panic!()
	};
	assert_eq!((b.hand, b.signing), (1, true));
	assert_eq!(slot_name("1.15.2", &b.new_book), "writable_book");
	let v1_15_2::slot::Ty::r#true(s) = &b.new_book else {
		panic!()
	};
	let tag = compound(&s.nbtData.as_ref().unwrap().value);
	assert_eq!(string(get(tag, "title")), "T");
	let recipe = v1_17_1::play_toServer_packet::Ty::recipe_book(
		v1_17_1::play_toServer_packet_recipe_book::Ty {
			bookId: 1,
			bookOpen: true,
			filterActive: false,
		},
	);
	let v1_15_2::play_toServer_packet::Ty::crafting_book_data(
		v1_15_2::play_toServer_packet_crafting_book_data::Ty::v1(d),
	) = one(server(&recipe, &mut cx))
	else {
		panic!()
	};
	assert!(d.smeltingBookOpen && !d.craftingBookOpen);
	let interact = v1_17_1::play_toServer_packet::Ty::use_entity(
		v1_17_1::play_toServer_packet_use_entity::Ty {
			target: 3,
			mouse: v1_17_1::play_toServer_packet_use_entity__mouse::Ty::v0(0),
			sneaking: true,
		},
	);
	assert!(
		matches!(server(&interact, &mut cx).as_slice(), [v1_15_2::play_toServer_packet::Ty::use_entity(u)] if u.target == 3)
	);
	assert!(server(&v1_17_1::play_toServer_packet::Ty::pong(5), &mut cx).is_empty());
}

fn login_17(min_y: i32, height: i32, cx: &mut Session) -> C17 {
	let C17::login(mut l) = one(up_15(
		&login_15(0),
		"1.17",
		cx,
		v1_17::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	let P::compound(dim) = &mut l.dimension.value else {
		panic!()
	};
	for e in dim.iter_mut() {
		match e.name.as_str() {
			"min_y" => e.value = P::int(min_y),
			"height" => e.value = P::int(height),
			_ => {}
		}
	}
	C17::login(l)
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
	C17::map_chunk(v1_17::play_toClient_packet_map_chunk::Ty {
		x,
		z,
		bitMap: mask,
		heightmaps: Entry::new(
			"",
			P::compound(vec![Entry::new(
				"WORLD_SURFACE",
				P::longArray(pack(&heights, 9, false)),
			)]),
		),
		biomes: (0..sections as i32 * 64).map(|i| i / 64).collect(),
		chunkData: data,
		blockEntities: [-60, 300]
			.iter()
			.map(|y| {
				Entry::new(
					"",
					P::compound(vec![
						Entry::new("id", P::string("minecraft:chest".to_owned())),
						Entry::new("x", P::int(x * 16)),
						Entry::new("y", P::int(*y)),
						Entry::new("z", P::int(z * 16)),
					]),
				)
			})
			.collect(),
	})
}

fn first_section_block(c: &v1_16_2::play_toClient_packet_map_chunk::Ty) -> String {
	let mut at = 0;
	let (_, _, states) = read_section(&c.chunkData, &mut at, global_bits("1.16.2"));
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
	C17::position(v1_17::play_toClient_packet_position::Ty {
		x: 8.0,
		y,
		z: 8.0,
		yaw: 0.0,
		pitch: 0.0,
		flags,
		teleportId: 1,
		dismountVehicle: false,
	})
}

fn light_17(x: i32, z: i32, sections: usize) -> C17 {
	let mask = bits(sections + 2);
	C17::update_light(v1_17::play_toClient_packet_update_light::Ty {
		chunkX: x,
		chunkZ: z,
		trustEdges: true,
		skyLightMask: mask.clone(),
		blockLightMask: vec![],
		emptySkyLightMask: vec![],
		emptyBlockLightMask: mask,
		skyLight: (0..sections + 2).map(|k| vec![k as u8; 2048]).collect(),
		blockLight: vec![],
	})
}

fn sky(l: &v1_16_2::play_toClient_packet_update_light::Ty) -> Vec<u8> {
	let mut at = 0;
	let mut out = Vec::new();
	for _ in 0..(l.skyLightMask as u32).count_ones() {
		let n = read_varint(&l.data, &mut at) as usize;
		out.push(l.data[at]);
		at += n;
	}
	out
}

#[test]
fn tall_worlds_start_in_the_identity_window() {
	let mut cx = Session::new();
	let C162::login(l) = one(down_17(&login_17(-64, 384, &mut cx), &mut cx)) else {
		panic!()
	};
	let dim = compound(&l.dimension.value);
	assert!(get(dim, "min_y").is_none() && get(dim, "height").is_none());
	assert_eq!(int(get(dim, "logical_height")), 256);
	let C162::update_light(light) = one(down_17(&light_17(0, 0, 24), &mut cx)) else {
		panic!()
	};
	assert_eq!(sky(&light), (4..22).collect::<Vec<u8>>());
	assert_eq!(light.emptyBlockLightMask, (1 << 18) - 1);
	let C162::map_chunk(c) = one(down_17(&tall_chunk(0, 0, 24), &mut cx)) else {
		panic!()
	};
	assert_eq!(c.bitMap, 0xffff);
	assert_eq!(first_section_block(&c), expected_block(4));
	let v1_16_2::play_toClient_packet_map_chunk__biomes::Ty::r#true(b) = &c.biomes else {
		panic!()
	};
	assert_eq!((b.len(), b[0], b[1023]), (1024, 4, 19));
	assert!(c.blockEntities.is_empty());
	let P::longArray(h) = &compound(&c.heightmaps.value)[0].value else {
		panic!()
	};
	assert_eq!(unpack(h, 9, 256)[0], 200);
	let C162::position(p) = one(down_17(&position_17(70.0, 0), &mut cx)) else {
		panic!()
	};
	assert_eq!(p.y, 70.0);
}

fn teleport(out: &[C162]) -> &v1_16_2::play_toClient_packet_position::Ty {
	out.iter()
		.find_map(|p| {
			if let C162::position(p) = p {
				Some(p)
			} else {
				None
			}
		})
		.expect("a teleport")
}

fn teleport_17(y: f64, flags: i8, id: i32) -> C17 {
	let C17::position(mut p) = position_17(y, flags) else {
		unreachable!()
	};
	p.teleportId = id;
	C17::position(p)
}

fn map_chunk(out: &[C162], x: i32, z: i32) -> Option<&v1_16_2::play_toClient_packet_map_chunk::Ty> {
	out.iter().find_map(|p| {
		if let C162::map_chunk(c) = p {
			(c.x == x && c.z == z).then_some(c)
		} else {
			None
		}
	})
}

fn order(out: &[C162], f: fn(&C162) -> bool) -> usize {
	out.iter().position(f).expect("a packet")
}

fn sections(c: &v1_16_2::play_toClient_packet_map_chunk::Ty) -> Vec<Vec<u32>> {
	let mut at = 0;
	(0..(c.bitMap as u32).count_ones())
		.map(|_| read_section(&c.chunkData, &mut at, global_bits("1.16.2")).2)
		.collect()
}

fn block_change_17(x: i32, y: i32, z: i32, text: &str) -> C17 {
	C17::block_change(v1_17::play_toClient_packet_block_change::Ty {
		location: v1_17::position::Ty { x, y: y as i16, z },
		r#type: state("1.17", text) as i32,
	})
}

fn mob_17(id: i32, y: f64) -> C17 {
	C17::spawn_entity_living(v1_17::play_toClient_packet_spawn_entity_living::Ty {
		entityId: id,
		entityUUID: id as u128,
		r#type: entity("1.17", "cow"),
		x: 1.0,
		y,
		z: 1.0,
		yaw: 0,
		pitch: 0,
		headPitch: 0,
		velocity: v1_17::vec3i16::Ty { x: 0, y: 0, z: 0 },
	})
}

fn confirm(id: i32, cx: &mut Session) -> Vec<S17> {
	up_16_2(&S162::teleport_confirm(id), cx).0
}

#[test]
fn tall_worlds_switch_windows_on_server_teleports() {
	let mut cx = Session::new();
	tall_world(&mut cx, -64, 384);
	let C162::spawn_entity_living(m) = one(down_17(&mob_17(70, 250.0), &mut cx)) else {
		panic!()
	};
	assert_eq!(m.y, 250.0);

	let out = down_17(&teleport_17(300.0, 0, 2), &mut cx);
	let p = teleport(&out);
	assert_eq!((p.y, p.flags), (236.0, 0));
	let chunk = map_chunk(&out, 0, 0).unwrap();
	assert_eq!(first_section_block(chunk), expected_block(8));
	let v1_16_2::play_toClient_packet_map_chunk__biomes::Ty::r#true(b) = &chunk.biomes else {
		panic!()
	};
	assert_eq!(b[0], 8);
	assert_eq!(int(get(compound(&chunk.blockEntities[0].value), "y")), 236);
	let light = out
		.iter()
		.find_map(|p| {
			if let C162::update_light(l) = p {
				Some(l)
			} else {
				None
			}
		})
		.unwrap();
	assert_eq!(sky(light), (8..26).collect::<Vec<u8>>());
	let light_at = order(&out, |p| matches!(p, C162::update_light(_)));
	let chunk_at = order(&out, |p| matches!(p, C162::map_chunk(_)));
	assert!(light_at < chunk_at && chunk_at < order(&out, |p| matches!(p, C162::position(_))));
	assert!(out
		.iter()
		.any(|p| matches!(p, C162::entity_teleport(t) if t.entityId == 70 && t.y == 186.0)));
	assert_eq!(confirm(2, &mut cx), [S17::teleport_confirm(2)]);

	let C162::block_change(b) = one(down_17(
		&block_change_17(3, -50, 4, "minecraft:stone"),
		&mut cx,
	)) else {
		panic!()
	};
	assert_eq!(
		(b.location.y, describe("1.16.2", b.r#type as u32)),
		(0, expected_block(8))
	);
	let C162::block_change(b) = one(down_17(
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
	assert_eq!(confirm(3, &mut cx), [S17::teleport_confirm(3)]);
	let (sent, back) = up_16_2(&walk(84.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (20.0, 0));
	let dig = S162::block_dig(v1_16_2::play_toServer_packet_block_dig::Ty {
		status: 0,
		location: v1_16_2::position::Ty { x: 1, y: 10, z: 1 },
		face: 1,
	});
	let (sent, _) = up_16_2(&dig, &mut cx);
	let [S17::block_dig(d)] = sent.as_slice() else {
		panic!("{sent:?}")
	};
	assert_eq!(d.location.y, -54);

	down_17(
		&C17::unload_chunk(v1_17::play_toClient_packet_unload_chunk::Ty {
			chunkX: 0,
			chunkZ: 0,
		}),
		&mut cx,
	);
	let again = down_17(&teleport_17(300.0, 0, 4), &mut cx);
	assert!(!again.iter().any(|p| matches!(p, C162::map_chunk(_))));
	assert_eq!(teleport(&again).y, 236.0);
}

type S162 = v1_16_2::play_toServer_packet::Ty;
type S17 = v1_17::play_toServer_packet::Ty;

fn up_16_2(p: &S162, cx: &mut Session) -> (Vec<S17>, Vec<C162>) {
	let sent = run(
		Root::PlayToServer,
		"1.16.2",
		"1.17",
		&enc(p, v1_16_2::play_toServer_packet::serialize),
		cx,
	)
	.iter()
	.map(|b| dec(b, v1_17::play_toServer_packet::deserialize))
	.collect();
	let back = cx
		.take_outgoing()
		.into_iter()
		.map(|o| {
			assert_eq!((o.root, o.version), (Root::PlayToClient, at("1.16.2")));
			dec(&o.data, v1_16_2::play_toClient_packet::deserialize)
		})
		.collect();
	(sent, back)
}

fn walk(y: f64) -> S162 {
	S162::position(v1_16_2::play_toServer_packet_position::Ty {
		x: 8.0,
		y,
		z: 8.0,
		onGround: true,
	})
}

fn server_y(sent: &[S17]) -> f64 {
	match sent {
		[S17::position(p)] => p.y,
		[S17::position_look(p)] => p.y,
		[S17::vehicle_move(p)] => p.y,
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
	assert_eq!(teleport(&out).teleportId, 2);
	assert_eq!(confirm(2, cx), [S17::teleport_confirm(2)]);
}

fn switched(back: &[C162], dy: f64) -> i32 {
	let t = teleport(back);
	assert_eq!((t.x, t.y, t.z, t.flags), (0.0, dy, 0.0, 0x1f));
	assert_eq!(t.teleportId & !0x00ff_ffff, 0x4e00_0000);
	t.teleportId
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
		order(&back, |p| matches!(p, C162::map_chunk(_)))
			< order(&back, |p| matches!(p, C162::position(_)))
	);
	assert!(back
		.iter()
		.any(|p| matches!(p, C162::entity_teleport(e) if e.entityId == 71 && e.y == 214.0)));

	let (sent, back) = up_16_2(&walk(30.0), &mut cx);
	assert_eq!(server_y(&sent), 30.0);
	assert_eq!(
		first_section_block(map_chunk(&back, 5, 5).unwrap()),
		expected_block(0)
	);
	let (sent, back) = up_16_2(&S162::teleport_confirm(id), &mut cx);
	assert!(sent.is_empty() && back.is_empty());
	let (sent, back) = up_16_2(&walk(94.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (30.0, 0));
	let (sent, back) = up_16_2(&walk(4.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (-60.0, 0));

	let flying = |y: f64| {
		S162::position_look(v1_16_2::play_toServer_packet_position_look::Ty {
			x: 8.0,
			y,
			z: 8.0,
			yaw: 0.0,
			pitch: 0.0,
			onGround: false,
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
	assert!(up_16_2(&S162::teleport_confirm(id), &mut cx).0.is_empty());

	let boat = |y: f64| {
		S162::vehicle_move(v1_16_2::play_toServer_packet_vehicle_move::Ty {
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
				.filter(|p| matches!(p, C162::position(_)))
				.count()
		),
		(224.0, 0)
	);
	let (sent, back) = up_16_2(&boat(225.0), &mut cx);
	assert_eq!(server_y(&sent), 225.0);
	switched(&back, -64.0);
	assert!(back
		.iter()
		.any(|p| matches!(p, C162::vehicle_move(v) if v.y == 161.0 && v.yaw == 90.0)));
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
	assert_eq!(confirm(3, &mut cx), [S17::teleport_confirm(3)]);
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
	up_16_2(&S162::teleport_confirm(id), &mut cx);
	let (sent, back) = up_16_2(&walk(225.0), &mut cx);
	assert_eq!(server_y(&sent), 353.0);
	let id = switched(&back, -128.0);
	up_16_2(&S162::teleport_confirm(id), &mut cx);
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
	let C162::map_chunk(c) = one(down_17(&filled_chunk(0, 0, &blocks), &mut cx)) else {
		panic!()
	};
	let floor = state("1.16.2", "minecraft:blackstone");
	let air = state("1.16.2", "minecraft:air");
	assert_eq!(c.bitMap, 0xffff);
	let s = sections(&c);
	assert!(s[0][..256].iter().all(|b| *b == floor) && s[0][256..].iter().all(|b| *b == air));
	let P::longArray(h) = &compound(&c.heightmaps.value)[0].value else {
		panic!()
	};
	assert_eq!(unpack(h, 9, 256)[0], 200);

	let C162::block_change(b) = one(down_17(
		&block_change_17(3, -1, 4, "minecraft:air"),
		&mut cx,
	)) else {
		panic!()
	};
	assert_eq!((b.location.y, b.r#type as u32), (0, floor));
	let C162::block_change(b) = one(down_17(
		&block_change_17(3, 0, 4, "minecraft:stone"),
		&mut cx,
	)) else {
		panic!()
	};
	assert_eq!(
		(b.location.y, b.r#type as u32),
		(0, state("1.16.2", "minecraft:stone"))
	);
	let C162::block_change(b) = one(down_17(&block_change_17(3, 0, 4, "minecraft:air"), &mut cx))
	else {
		panic!()
	};
	assert_eq!((b.location.y, b.r#type as u32), (0, floor));
	let C162::block_change(b) = one(down_17(
		&block_change_17(3, 5, 4, "minecraft:stone"),
		&mut cx,
	)) else {
		panic!()
	};
	assert_eq!(b.location.y, 5);

	let out = down_17(&teleport_17(-40.0, 0, 2), &mut cx);
	let c = map_chunk(&out, 0, 0).unwrap();
	assert_eq!(c.bitMap, 0xffff);
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
	assert_eq!(c.bitMap, 0x0f);
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
		&C17::update_light(v1_17::play_toClient_packet_update_light::Ty {
			chunkX: 0,
			chunkZ: 0,
			trustEdges: true,
			skyLightMask: mask,
			blockLightMask: vec![],
			emptySkyLightMask: vec![],
			emptyBlockLightMask: vec![],
			skyLight: (0..12).map(|k| vec![k as u8; 2048]).collect(),
			blockLight: vec![],
		}),
		&mut cx,
	);
	down_17(&tall_chunk(0, 0, 24), &mut cx);
	let out = down_17(&teleport_17(300.0, 0, 2), &mut cx);
	let light = out
		.iter()
		.find_map(|p| {
			if let C162::update_light(l) = p {
				Some(l)
			} else {
				None
			}
		})
		.unwrap();
	assert_eq!(
		(light.skyLightMask, light.blockLightMask),
		((1 << 18) - 1, (1 << 18) - 1)
	);
	assert_eq!((light.emptySkyLightMask, light.emptyBlockLightMask), (0, 0));
	let mut expected: Vec<u8> = (8..12).collect();
	expected.extend([0xff; 14]);
	assert_eq!(sky(light), expected);
	let out = down_17(&teleport_17(-40.0, 0, 3), &mut cx);
	let light = out
		.iter()
		.find_map(|p| {
			if let C162::update_light(l) = p {
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
	let C17::login(l) = login else { panic!() };
	let respawn = C17::respawn(v1_17::play_toClient_packet_respawn::Ty {
		dimension: l.dimension,
		worldName: l.worldName,
		hashedSeed: 0,
		gamemode: 0,
		previousGamemode: 255,
		isDebug: false,
		isFlat: false,
		copyMetadata: false,
	});
	let out = down_17(&respawn, &mut cx);
	let unloaded: Vec<(i32, i32)> = out
		.iter()
		.filter_map(|p| {
			if let C162::unload_chunk(u) = p {
				Some((u.chunkX, u.chunkZ))
			} else {
				None
			}
		})
		.collect();
	assert_eq!(unloaded, [(0, 0), (1, -2)]);
	assert!(matches!(out.last(), Some(C162::respawn(_))));
	let C162::map_chunk(c) = one(down_17(&tall_chunk(0, 0, 24), &mut cx)) else {
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
	let C162::map_chunk(c) = one(down_17(&tall_chunk(2, 3, 16), &mut cx)) else {
		panic!()
	};
	assert_eq!((c.x, c.z, c.bitMap), (2, 3, 0xffff));
	assert_eq!(first_section_block(&c), expected_block(0));
	assert_eq!(c.blockEntities.len(), 0);
	let C162::position(p) = one(down_17(&position_17(200.0, 0), &mut cx)) else {
		panic!()
	};
	assert_eq!(p.y, 200.0);
	let change = C17::block_change(v1_17::play_toClient_packet_block_change::Ty {
		location: v1_17::position::Ty { x: 3, y: 5, z: 4 },
		r#type: state("1.17", "minecraft:stone") as i32,
	});
	let C162::block_change(b) = one(down_17(&change, &mut cx)) else {
		panic!()
	};
	assert_eq!(b.location.y, 5);
}

fn down_16(p: &C16, cx: &mut Session) -> Vec<C15> {
	run(
		Root::PlayToClient,
		"1.16",
		"1.15.2",
		&enc(p, v1_16::play_toClient_packet::serialize),
		cx,
	)
	.iter()
	.map(|b| dec(b, v1_15_2::play_toClient_packet::deserialize))
	.collect()
}

fn block_change_15(text: &str) -> C15 {
	C15::block_change(v1_15_2::play_toClient_packet_block_change::Ty {
		location: v1_15_2::position::Ty { x: 0, y: 64, z: 0 },
		r#type: state("1.15.2", text) as i32,
	})
}

#[test]
fn changed_block_properties() {
	let mut cx = Session::new();
	let wall = block_change_15("minecraft:cobblestone_wall[east=true,north=false,south=true,up=true,waterlogged=false,west=false]");
	let C16::block_change(b) = one(up_15(
		&wall,
		"1.16",
		&mut cx,
		v1_16::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(describe("1.16", b.r#type as u32), "minecraft:cobblestone_wall[east=low,north=none,south=low,up=true,waterlogged=false,west=none]");
	let C15::block_change(b) = one(down_16(&C16::block_change(b), &mut cx)) else {
		panic!()
	};
	assert_eq!(describe("1.15.2", b.r#type as u32), "minecraft:cobblestone_wall[east=true,north=false,south=true,up=true,waterlogged=false,west=false]");
	let jigsaw = block_change_15("minecraft:jigsaw[facing=up]");
	let C16::block_change(b) = one(up_15(
		&jigsaw,
		"1.16",
		&mut cx,
		v1_16::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		describe("1.16", b.r#type as u32),
		"minecraft:jigsaw[orientation=up_north]"
	);
	let cauldron = block_change_15("minecraft:cauldron[level=2]");
	let C17::block_change(b) = one(up_15(
		&cauldron,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	assert_eq!(
		describe("1.17", b.r#type as u32),
		"minecraft:water_cauldron[level=2]"
	);
	let lava = C17::block_change(v1_17::play_toClient_packet_block_change::Ty {
		location: v1_17::position::Ty { x: 0, y: 64, z: 0 },
		r#type: state("1.17", "minecraft:lava_cauldron") as i32,
	});
	let C162::block_change(b) = one(down_17(&lava, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		describe("1.16.2", b.r#type as u32),
		"minecraft:cauldron[level=3]"
	);
	let deepslate = C17::block_change(v1_17::play_toClient_packet_block_change::Ty {
		location: v1_17::position::Ty { x: 0, y: 64, z: 0 },
		r#type: state("1.17", "minecraft:deepslate[axis=y]") as i32,
	});
	let C162::block_change(b) = one(down_17(&deepslate, &mut cx)) else {
		panic!()
	};
	assert_eq!(describe("1.16.2", b.r#type as u32), "minecraft:blackstone");
}

#[test]
fn newer_items_are_substituted_and_restored() {
	let mut cx = Session::new();
	let copper = C17::set_slot(v1_17::play_toClient_packet_set_slot::Ty {
		windowId: 0,
		slot: 36,
		item: slot("1.17", "copper_ingot", 3),
	});
	let C162::set_slot(s) = one(down_17(&copper, &mut cx)) else {
		panic!()
	};
	assert_eq!(slot_name("1.16.2", &s.item), "brick");
	let v1_15_2::slot::Ty::r#true(item) = &s.item else {
		panic!()
	};
	let display =
		compound(get(compound(&item.nbtData.as_ref().unwrap().value), "display").unwrap());
	assert!(string(get(display, "Name")).contains("Copper Ingot"));
	let click = S162::window_click(v1_16_2::play_toServer_packet_window_click::Ty {
		windowId: 0,
		slot: 36,
		mouseButton: 0,
		action: 1,
		mode: 0,
		item: s.item.clone(),
	});
	let (sent, _) = up_16_2(&click, &mut cx);
	let [S17::window_click(c)] = sent.as_slice() else {
		panic!()
	};
	assert_eq!(slot_name("1.17", &c.cursorItem), "copper_ingot");
	assert_eq!(c.cursorItem, slot("1.17", "copper_ingot", 3));
	let sword = C16::set_slot(v1_16::play_toClient_packet_set_slot::Ty {
		windowId: 0,
		slot: 36,
		item: slot("1.16", "netherite_sword", 1),
	});
	let C15::set_slot(s) = one(down_16(&sword, &mut cx)) else {
		panic!()
	};
	assert_eq!(slot_name("1.15.2", &s.item), "diamond_sword");
}

#[test]
fn newer_entities_are_substituted() {
	let mut cx = Session::new();
	let goat = C17::spawn_entity_living(v1_17::play_toClient_packet_spawn_entity_living::Ty {
		entityId: 80,
		entityUUID: 3,
		r#type: entity("1.17", "goat"),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		yaw: 0,
		pitch: 0,
		headPitch: 0,
		velocity: v1_17::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let C162::spawn_entity_living(g) = one(down_17(&goat, &mut cx)) else {
		panic!()
	};
	assert_eq!(g.r#type, entity("1.16.2", "sheep"));
	use v1_17::entityMetadataItem::Ty as M;
	let entry =
		|key: u8, r#type: i32, value: M| v1_17::entityMetadata__items::Ty { key, r#type, value };
	let meta = C17::entity_metadata(v1_17::play_toClient_packet_entity_metadata::Ty {
		entityId: 80,
		metadata: vec![
			entry(0, 0, M::v0(0)),
			entry(16, 7, M::v7(true)),
			entry(17, 7, M::v7(true)),
		],
	});
	let C162::entity_metadata(m) = one(down_17(&meta, &mut cx)) else {
		panic!()
	};
	assert_eq!(m.metadata.iter().map(|e| e.key).collect::<Vec<_>>(), [0]);
	let marker = C17::spawn_entity(v1_17::play_toClient_packet_spawn_entity::Ty {
		entityId: 81,
		objectUUID: 4,
		r#type: entity("1.17", "glow_item_frame"),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		pitch: 0,
		yaw: 0,
		objectData: 2,
		velocity: v1_17::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let C162::spawn_entity(f) = one(down_17(&marker, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		(f.r#type, f.objectData),
		(entity("1.16.2", "item_frame"), 2)
	);
	let frame = C15::spawn_entity(v1_15_2::play_toClient_packet_spawn_entity::Ty {
		entityId: 82,
		objectUUID: 5,
		r#type: entity("1.15.2", "item_frame"),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		pitch: 10,
		yaw: 20,
		objectData: 3,
		velocity: v1_15_2::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let out = up_15(
		&frame,
		"1.17",
		&mut cx,
		v1_17::play_toClient_packet::deserialize,
	);
	assert!(
		matches!(out.as_slice(), [C17::spawn_entity(_), C17::entity_look(l)] if (l.entityId, l.yaw, l.pitch) == (82, 20, 10))
	);
}

#[test]
fn texts_lose_newer_formatting_for_1_15_2() {
	let mut cx = Session::new();
	let message = r##"{"text":"hi","color":"#ff0000","hoverEvent":{"action":"show_item","contents":{"id":"minecraft:stone","count":2}}}"##;
	let chat = C16::chat(v1_16::play_toClient_packet_chat::Ty {
		message: message.to_owned(),
		position: 0,
		sender: 7,
	});
	let C15::chat(c) = one(down_16(&chat, &mut cx)) else {
		panic!()
	};
	let json: serde_json::Value = serde_json::from_str(&c.message).unwrap();
	assert_eq!(json["color"], "dark_red");
	assert_eq!(
		json["hoverEvent"]["value"],
		"{id:\"minecraft:stone\",Count:2b}"
	);
	let window = C16::open_window(v1_16::play_toClient_packet_open_window::Ty {
		windowId: 2,
		inventoryType: 20,
		windowTitle: "{\"text\":\"x\"}".to_owned(),
	});
	let C15::open_window(w) = one(down_16(&window, &mut cx)) else {
		panic!()
	};
	assert_eq!(w.inventoryType, 7);
	use v1_16::command_node__extraNodeData__v2__properties::Ty as Props;
	let node = |data: v1_16::command_node__extraNodeData::Ty| v1_16::command_node::Ty {
		flags: v1_16::command_node__flags::Ty {
			unused: 0,
			has_custom_suggestions: 0,
			has_redirect_node: 0,
			has_command: 0,
			command_node_type: 2,
		},
		children: vec![],
		redirectNode: v1_16::command_node__redirectNode::Ty::default(0),
		extraNodeData: data,
	};
	let commands = C16::declare_commands(v1_16::play_toClient_packet_declare_commands::Ty {
		nodes: vec![node(v1_16::command_node__extraNodeData::Ty::v2(
			v1_16::command_node__extraNodeData__v2::Ty {
				name: "target".to_owned(),
				parser: "minecraft:uuid".to_owned(),
				properties: Props::minecraft_uuid,
				suggestionType: v1_16::command_node__extraNodeData__v2__suggestionType::Ty::default(
					0,
				),
			},
		))],
		rootIndex: 0,
	});
	let C15::declare_commands(d) = one(down_16(&commands, &mut cx)) else {
		panic!()
	};
	let v1_15_2::command_node__extraNodeData::Ty::v2(arg) = &d.nodes[0].extraNodeData else {
		panic!()
	};
	assert_eq!(arg.parser, "brigadier:string");
}

#[test]
fn respawns_and_container_data_for_1_15_2() {
	let mut cx = Session::new();
	let C16::login(l) = one(up_15(
		&login_15(0),
		"1.16",
		&mut cx,
		v1_16::play_toClient_packet::deserialize,
	)) else {
		panic!()
	};
	down_16(&C16::login(l), &mut cx);
	let attributes =
		C16::entity_update_attributes(v1_16::play_toClient_packet_entity_update_attributes::Ty {
			entityId: 7,
			properties: vec![
				v1_16::play_toClient_packet_entity_update_attributes__properties__items::Ty {
					key: "minecraft:generic.movement_speed".to_owned(),
					value: 0.1,
					modifiers: vec![],
				},
			],
		});
	down_16(&attributes, &mut cx);
	let respawn = C16::respawn(v1_16::play_toClient_packet_respawn::Ty {
		dimension: "minecraft:overworld".to_owned(),
		worldName: "minecraft:other".to_owned(),
		hashedSeed: 0,
		gamemode: 0,
		previousGamemode: 255,
		isDebug: false,
		isFlat: false,
		copyMetadata: true,
	});
	let out = down_16(&respawn, &mut cx);
	assert!(
		matches!(out.as_slice(), [C15::respawn(a), C15::respawn(b), C15::entity_update_attributes(u)]
		if a.dimension == -1 && b.dimension == 0 && u.properties[0].key == "generic.movementSpeed")
	);
	let hint = |value: i16| {
		C16::craft_progress_bar(v1_16::play_toClient_packet_craft_progress_bar::Ty {
			windowId: 1,
			property: 4,
			value,
		})
	};
	assert!(
		matches!(down_16(&hint(11), &mut cx).as_slice(), [C15::craft_progress_bar(p)] if p.value == -1)
	);
	assert!(
		matches!(up_15(&C15::craft_progress_bar(v1_15_2::play_toClient_packet_craft_progress_bar::Ty { windowId: 1, property: 5, value: 11 }), "1.16", &mut cx, v1_16::play_toClient_packet::deserialize).as_slice(), [C16::craft_progress_bar(p)] if p.value == 12)
	);
	let stats = C16::statistics(vec![v1_16::play_toClient_packet__statistics__items::Ty {
		categoryId: 8,
		statisticId: 25,
		value: 3,
	}]);
	assert!(
		matches!(down_16(&stats, &mut cx).as_slice(), [C15::statistics(s)] if s[0].statisticId == 24)
	);
	let map = C16::map(v1_16::play_toClient_packet_map::Ty {
		itemDamage: 1,
		scale: 0,
		trackingPosition: true,
		locked: false,
		icons: vec![],
		columns: v1_16::play_toClient_packet_map__columns::Ty::default(
			1,
			v1_16::play_toClient_packet_map__columns__default::Ty {
				rows: 1,
				x: 0,
				y: 0,
				data: vec![208, 5],
			},
		),
	});
	let C15::map(m) = one(down_16(&map, &mut cx)) else {
		panic!()
	};
	let v1_15_2::play_toClient_packet_map__columns::Ty::default(_, c) = &m.columns else {
		panic!()
	};
	assert_eq!(c.data, [113, 5]);
}

#[test]
fn state_ids_follow_the_server() {
	let mut cx = Session::new();
	let items = C171::window_items(v1_17_1::play_toClient_packet_window_items::Ty {
		windowId: 3,
		stateId: 42,
		items: vec![],
		carriedItem: slot("1.17", "stone", 1),
	});
	let out: Vec<C17> = run(
		Root::PlayToClient,
		"1.17.1",
		"1.17",
		&enc(&items, v1_17_1::play_toClient_packet::serialize),
		&mut cx,
	)
	.iter()
	.map(|b| dec(b, v1_17::play_toClient_packet::deserialize))
	.collect();
	assert!(
		matches!(out.as_slice(), [C17::window_items(_), C17::set_slot(s)] if s.windowId == -1 && slot_name("1.17", &s.item) == "stone")
	);
	let click = v1_17::play_toServer_packet::Ty::window_click(
		v1_17::play_toServer_packet_window_click::Ty {
			windowId: 3,
			slot: 1,
			mouseButton: 0,
			mode: 0,
			changedSlots: vec![],
			cursorItem: v1_15_2::slot::Ty::r#false,
		},
	);
	let out = run(
		Root::PlayToServer,
		"1.17",
		"1.17.1",
		&enc(&click, v1_17::play_toServer_packet::serialize),
		&mut cx,
	);
	let v1_17_1::play_toServer_packet::Ty::window_click(c) =
		dec(&out[0], v1_17_1::play_toServer_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(c.stateId, 42);
}

#[test]
fn particles_fall_back_for_older_clients() {
	let mut cx = Session::new();
	let p = C17::world_particles(v1_17::play_toClient_packet_world_particles::Ty {
		particleId: registry::names("1.17", "particles")
			.unwrap()
			.id("dust_color_transition")
			.unwrap() as i32,
		longDistance: false,
		x: 0.0,
		y: 64.0,
		z: 0.0,
		offsetX: 0.0,
		offsetY: 0.0,
		offsetZ: 0.0,
		particleData: 0.0,
		particles: 1,
		data: v1_17::particleData::Ty::v16(v1_17::particleData__v16::Ty {
			fromRed: 1.0,
			fromGreen: 0.5,
			fromBlue: 0.0,
			scale: 2.0,
			toRed: 0.0,
			toGreen: 0.0,
			toBlue: 1.0,
		}),
	});
	let C162::world_particles(w) = one(down_17(&p, &mut cx)) else {
		panic!()
	};
	assert!(
		matches!(&w.data, v1_16_2::particleData::Ty::v14(d) if (d.red, d.green, d.scale) == (1.0, 0.5, 2.0))
	);
}
