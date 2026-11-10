use minecraft_versions::gen::convert::Version::{self, *};
use minecraft_versions::gen::convert::{convert, v1_13_2, v1_14, v1_15_2, Root};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::registry;
use minecraft_versions::Session;
use nbt::gen::pds_Entry::Ty as Nbt;
use nbt::gen::pds_List::Ty as NbtList;
use nbt::gen::pds_List_0::Ty as NbtItems;
use nbt::gen::pds_Payload::Ty as Tag;
use v1_13_2::play_toClient_packet::Ty as C13;
use v1_14::play_toClient_packet::Ty as C14;
use v1_15_2::play_toClient_packet::Ty as C15;

type M13 = v1_13_2::entityMetadataItem::Ty;
type M14 = v1_14::entityMetadataItem::Ty;
type E13 = v1_13_2::entityMetadata__items::Ty;
type Slot = v1_13_2::slot::Ty;

const ZOMBIE_13: i32 = 87;
const VILLAGER_13: i32 = 79;
const HORSE_13: i32 = 29;

fn at(version: Version) -> usize {
	version as usize
}

fn up<T>(
	p: &C13,
	to: Version,
	cx: &mut Session,
	de: fn(&mut Input) -> minecraft_versions::pds::Result<T>,
) -> Vec<T> {
	let mut o = Output::new();
	v1_13_2::play_toClient_packet::serialize(p, &mut o).unwrap();
	convert(Root::PlayToClient, at(V1_13_2), at(to), &o.into_inner(), cx)
		.unwrap()
		.iter()
		.map(|b| {
			let mut input = Input::new(b);
			let p = de(&mut input).unwrap();
			assert_eq!(input.remaining(), 0, "leftover bytes at {to:?}");
			p
		})
		.collect()
}

fn to14(p: &C13, cx: &mut Session) -> Vec<C14> {
	up(p, V1_14, cx, v1_14::play_toClient_packet::deserialize)
}

fn to15(p: &C13, cx: &mut Session) -> Vec<C15> {
	up(p, V1_15_2, cx, v1_15_2::play_toClient_packet::deserialize)
}

fn login(cx: &mut Session) {
	to15(
		&C13::login(v1_13_2::play_toClient_packet_login::Ty {
			entityId: 1,
			gameMode: 0,
			dimension: 0,
			difficulty: 2,
			maxPlayers: 20,
			levelType: "default".into(),
			reducedDebugInfo: false,
		}),
		cx,
	);
}

fn state(version: Version, text: &str) -> u32 {
	registry::blocks(version)
		.unwrap()
		.parse_state(text)
		.unwrap()
}

fn describe(version: Version, state: u32) -> String {
	registry::blocks(version).unwrap().describe(state).unwrap()
}

fn item(version: Version, name: &str) -> i32 {
	registry::names(version, "items").unwrap().id(name).unwrap() as i32
}

fn entity(version: Version, name: &str) -> i32 {
	registry::names(version, "entities")
		.unwrap()
		.id(name)
		.unwrap() as i32
}

fn varint(out: &mut Vec<u8>, v: i32) {
	let mut v = v as u32;
	while v >= 0x80 {
		out.push(v as u8 | 0x80);
		v >>= 7;
	}
	out.push(v as u8);
}

fn read_varint(data: &[u8], at: &mut usize) -> i32 {
	let mut v = 0u32;
	for shift in (0..35).step_by(7) {
		let b = data[*at];
		*at += 1;
		v |= ((b & 0x7f) as u32) << shift;
		if b & 0x80 == 0 {
			break;
		}
	}
	v as i32
}

fn pack(values: &[u64], bits: usize) -> Vec<u64> {
	let mut longs = vec![0u64; values.len() * bits / 64];
	for (i, v) in values.iter().enumerate() {
		let (w, off) = (i * bits / 64, i * bits % 64);
		longs[w] |= v << off;
		if off + bits > 64 {
			longs[w + 1] |= v >> (64 - off);
		}
	}
	longs
}

fn unpack(longs: &[u64], bits: usize, n: usize) -> Vec<u64> {
	(0..n)
		.map(|i| {
			let (w, off) = (i * bits / 64, i * bits % 64);
			let mut v = longs[w] >> off;
			if off + bits > 64 {
				v |= longs[w + 1] << (64 - off);
			}
			v & ((1 << bits) - 1)
		})
		.collect()
}

fn write_section(out: &mut Vec<u8>, states: &[u32], global: bool) {
	let (bits, values): (usize, Vec<u64>) = if global {
		(14, states.iter().map(|s| *s as u64).collect())
	} else {
		let mut palette: Vec<u32> = Vec::new();
		for s in states {
			if !palette.contains(s) {
				palette.push(*s);
			}
		}
		out.push(8);
		varint(out, palette.len() as i32);
		for p in &palette {
			varint(out, *p as i32);
		}
		(
			8,
			states
				.iter()
				.map(|s| palette.iter().position(|p| p == s).unwrap() as u64)
				.collect(),
		)
	};
	if global {
		out.push(14);
	}
	let longs = pack(&values, bits);
	varint(out, longs.len() as i32);
	for l in longs {
		out.extend(l.to_be_bytes());
	}
}

fn read_section(data: &[u8], at: &mut usize, count: bool) -> (Option<i16>, Vec<u32>) {
	let (count, _, states) = read_section_bits(data, at, count);
	(count, states)
}

fn read_section_bits(data: &[u8], at: &mut usize, count: bool) -> (Option<i16>, usize, Vec<u32>) {
	let count = count.then(|| {
		let c = i16::from_be_bytes([data[*at], data[*at + 1]]);
		*at += 2;
		c
	});
	let bits = data[*at] as usize;
	*at += 1;
	let palette: Vec<u32> = match bits <= 8 {
		true => (0..read_varint(data, at))
			.map(|_| read_varint(data, at) as u32)
			.collect(),
		false => Vec::new(),
	};
	let n = read_varint(data, at) as usize;
	let longs: Vec<u64> = (0..n)
		.map(|i| u64::from_be_bytes(data[*at + i * 8..*at + i * 8 + 8].try_into().unwrap()))
		.collect();
	*at += n * 8;
	let values = unpack(&longs, if bits > 8 { 14 } else { bits }, 4096);
	let states = values
		.iter()
		.map(|v| {
			if palette.is_empty() {
				*v as u32
			} else {
				palette[*v as usize]
			}
		})
		.collect();
	(count, bits, states)
}

fn nibble(a: &[u8], i: usize) -> u8 {
	(a[i >> 1] >> ((i & 1) * 4)) & 15
}

fn idx(x: usize, y: usize, z: usize) -> usize {
	y << 8 | z << 4 | x
}

struct Column {
	sections: Vec<(usize, Vec<u32>, Vec<u8>, Vec<u8>)>,
	biomes: Vec<i32>,
}

fn column_1_13_2() -> Column {
	let v = V1_13_2;
	let air = state(v, "minecraft:air");
	let mut low = vec![air; 4096];
	for x in 0..16 {
		for z in 0..16 {
			for y in 0..4 {
				low[idx(x, y, z)] = state(v, "minecraft:stone");
			}
			low[idx(x, 4, z)] = state(v, "minecraft:grass_block[snowy=false]");
		}
	}
	low[idx(1, 5, 1)] = state(
		v,
		"minecraft:oak_stairs[facing=east,half=bottom,shape=straight,waterlogged=false]",
	);
	low[idx(2, 5, 2)] = state(v, "minecraft:sign[rotation=4,waterlogged=false]");
	low[idx(3, 5, 3)] = state(v, "minecraft:poppy");
	let mut low_block = vec![0u8; 2048];
	let i = idx(1, 5, 0);
	low_block[i >> 1] |= 12 << ((i & 1) * 4);
	let mut low_sky = vec![0u8; 2048];
	for i in 0..4096 {
		if i >> 8 >= 5 {
			low_sky[i >> 1] |= 15 << ((i & 1) * 4);
		}
	}
	let mut high = vec![air; 4096];
	high[idx(0, 8, 0)] = state(v, "minecraft:diamond_block");
	Column {
		sections: vec![
			(0, low, low_block, low_sky),
			(2, high, vec![0x11; 2048], vec![0xff; 2048]),
		],
		biomes: (0..256)
			.map(|i| ((i >> 6) * 4 + (i & 15) / 4) % 7)
			.collect(),
	}
}

fn chunk_1_13_2(c: &Column) -> C13 {
	let mut data = Vec::new();
	for (n, (_, states, block, sky)) in c.sections.iter().enumerate() {
		write_section(&mut data, states, n == 1);
		data.extend(block);
		data.extend(sky);
	}
	for b in &c.biomes {
		data.extend(b.to_be_bytes());
	}
	C13::map_chunk(v1_13_2::play_toClient_packet_map_chunk::Ty {
		x: 2,
		z: -3,
		groundUp: true,
		bitMap: 0b101,
		chunkData: data,
		blockEntities: Vec::new(),
	})
}

fn light_arrays(data: &[u8], count: u32) -> Vec<Vec<u8>> {
	let mut at = 0;
	let out = (0..count)
		.map(|_| {
			let n = read_varint(data, &mut at) as usize;
			at += n;
			data[at - n..at].to_vec()
		})
		.collect();
	assert_eq!(at, data.len());
	out
}

fn heights(nbt: &Nbt, name: &str) -> Vec<u64> {
	let Tag::compound(maps) = &nbt.value else {
		panic!("{nbt:?}")
	};
	let Tag::longArray(longs) = &maps.iter().find(|e| e.name == name).unwrap().value else {
		panic!()
	};
	unpack(&longs.iter().map(|l| *l as u64).collect::<Vec<_>>(), 9, 256)
}

#[test]
fn login_adds_view_distance_and_difficulty() {
	let packet = C13::login(v1_13_2::play_toClient_packet_login::Ty {
		entityId: 7,
		gameMode: 1,
		dimension: -1,
		difficulty: 3,
		maxPlayers: 20,
		levelType: "flat".into(),
		reducedDebugInfo: true,
	});
	let out = to14(&packet, &mut Session::new());
	let [C14::login(l), C14::update_view_distance(64), C14::difficulty(d)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(
			l.entityId,
			l.gameMode,
			l.dimension,
			l.viewDistance,
			l.reducedDebugInfo
		),
		(7, 1, -1, 64, true)
	);
	assert!(
		matches!(
			d.difficulty,
			v1_14::play_toClient_packet_difficulty__difficulty::Ty::hard
		) && !d.difficultyLocked
	);
	let out = to15(&packet, &mut Session::new());
	let [C15::login(l), C15::update_view_distance(64), C15::difficulty(_)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(
			l.hashedSeed,
			l.enableRespawnScreen,
			l.viewDistance,
			l.levelType.as_str()
		),
		(0, true, 64, "flat")
	);
}

#[test]
fn chunk_moves_light_out_and_computes_heightmaps() {
	let mut cx = Session::new();
	login(&mut cx);
	let column = column_1_13_2();
	let out = to14(&chunk_1_13_2(&column), &mut cx);
	let [C14::update_view_position(v), C14::update_light(light), C14::map_chunk(chunk)] =
		out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!((v.chunkX, v.chunkZ), (2, -3));
	assert_eq!(
		(
			light.chunkX,
			light.chunkZ,
			light.skyLightMask,
			light.blockLightMask
		),
		(2, -3, 0x3ffff, 0b1010)
	);
	let arrays = light_arrays(&light.data, 20);
	assert!(arrays[0].iter().all(|b| *b == 0xff) && arrays[17].iter().all(|b| *b == 0xff));
	assert_eq!(nibble(&arrays[1], idx(5, 6, 5)), 15);
	assert_eq!(nibble(&arrays[1], idx(5, 3, 5)), 0);
	assert_eq!(
		nibble(&arrays[1], idx(1, 5, 1)),
		15,
		"stairs take the sky light above them"
	);
	assert_eq!(
		nibble(&arrays[2], idx(0, 0, 0)),
		0,
		"empty section under the diamond block is dark"
	);
	assert_eq!(
		nibble(&arrays[2], idx(5, 0, 5)),
		15,
		"empty section above the ground is lit"
	);
	assert!(arrays[3].iter().all(|b| *b == 0xff));
	assert_eq!(
		nibble(&arrays[18], idx(1, 5, 1)),
		11,
		"stairs take the block light of their neighbour"
	);
	assert_eq!(nibble(&arrays[18], idx(1, 5, 0)), 12);
	assert!(arrays[19].iter().all(|b| *b == 0x11));
	assert_eq!(
		(chunk.x, chunk.z, chunk.groundUp, chunk.bitMap),
		(2, -3, true, 0b101)
	);
	let surface = heights(&chunk.heightmaps, "WORLD_SURFACE");
	let motion = heights(&chunk.heightmaps, "MOTION_BLOCKING");
	assert_eq!(
		(
			surface[0],
			surface[1 + 16],
			surface[3 + 3 * 16],
			surface[5 + 5 * 16]
		),
		(41, 6, 6, 5)
	);
	assert_eq!(
		(
			motion[0],
			motion[1 + 16],
			motion[3 + 3 * 16],
			motion[5 + 5 * 16]
		),
		(41, 6, 5, 5)
	);
	let d = &chunk.chunkData;
	let mut at = 0;
	let (count, bits, low) = read_section_bits(d, &mut at, true);
	assert_eq!((count, bits), (Some(16 * 16 * 5 + 3), 4));
	assert_eq!(
		describe(V1_14, low[idx(2, 5, 2)]),
		"minecraft:oak_sign[rotation=4,waterlogged=false]"
	);
	assert_eq!(
		describe(V1_14, low[idx(1, 5, 1)]),
		"minecraft:oak_stairs[facing=east,half=bottom,shape=straight,waterlogged=false]"
	);
	assert_eq!(describe(V1_14, low[idx(0, 0, 0)]), "minecraft:stone");
	let (count, bits, high) = read_section_bits(d, &mut at, true);
	assert_eq!(
		(count, bits),
		(Some(1), 4),
		"a global palette section with few states gets a local palette"
	);
	assert_eq!(
		describe(V1_14, high[idx(0, 8, 0)]),
		"minecraft:diamond_block"
	);
	let biomes: Vec<i32> = d[at..]
		.chunks(4)
		.map(|c| i32::from_be_bytes(c.try_into().unwrap()))
		.collect();
	assert_eq!(biomes, column.biomes);
}

#[test]
fn chunk_reaches_1_15_2_with_biome_volume() {
	let mut cx = Session::new();
	login(&mut cx);
	let column = column_1_13_2();
	let out = to15(&chunk_1_13_2(&column), &mut cx);
	let [C15::update_view_position(_), C15::update_light(_), C15::map_chunk(chunk)] =
		out.as_slice()
	else {
		panic!("{out:?}")
	};
	let v1_15_2::play_toClient_packet_map_chunk__biomes::Ty::r#true(biomes) = &chunk.biomes else {
		panic!()
	};
	assert_eq!(biomes.len(), 1024);
	for i in 0..1024 {
		assert_eq!(
			biomes[i],
			column.biomes[((i >> 2 & 3) * 4 + 2) * 16 + (i & 3) * 4 + 2]
		);
	}
	let mut at = 0;
	let (_, low) = read_section(&chunk.chunkData, &mut at, true);
	assert_eq!(
		describe(V1_15_2, low[idx(2, 5, 2)]),
		"minecraft:oak_sign[rotation=4,waterlogged=false]"
	);
	let (_, high) = read_section(&chunk.chunkData, &mut at, true);
	assert_eq!(
		describe(V1_15_2, high[idx(0, 8, 0)]),
		"minecraft:diamond_block"
	);
	assert_eq!(at, chunk.chunkData.len());
}

#[test]
fn chunk_round_trips_back_to_1_13_2() {
	let mut cx = Session::new();
	login(&mut cx);
	let mut column = column_1_13_2();
	column.sections[0].1[idx(1, 5, 1)] = state(V1_13_2, "minecraft:air");
	let mut o = Output::new();
	v1_13_2::play_toClient_packet::serialize(&chunk_1_13_2(&column), &mut o).unwrap();
	let newest = convert(
		Root::PlayToClient,
		at(V1_13_2),
		at(V1_15_2),
		&o.into_inner(),
		&mut cx,
	)
	.unwrap();
	let mut back = Session::new();
	let mut result = Vec::new();
	for p in &newest {
		result.extend(convert(Root::PlayToClient, at(V1_15_2), at(V1_13_2), p, &mut back).unwrap());
	}
	let [bytes] = result.as_slice() else {
		panic!("{result:?}")
	};
	let C13::map_chunk(chunk) =
		v1_13_2::play_toClient_packet::deserialize(&mut Input::new(bytes)).unwrap()
	else {
		panic!()
	};
	let mut at = 0;
	for (_, states, block, sky) in &column.sections {
		let (_, read) = read_section(&chunk.chunkData, &mut at, false);
		assert_eq!(&read, states);
		assert_eq!(&chunk.chunkData[at..at + 2048], block.as_slice());
		assert_eq!(&chunk.chunkData[at + 2048..at + 4096], sky.as_slice());
		at += 4096;
	}
	let biomes: Vec<i32> = chunk.chunkData[at..]
		.chunks(4)
		.map(|c| i32::from_be_bytes(c.try_into().unwrap()))
		.collect();
	assert_eq!(biomes, column.biomes);
}

#[test]
fn signs_are_renamed_in_block_changes() {
	let sign = state(V1_13_2, "minecraft:sign[rotation=9,waterlogged=true]");
	let packet = C13::block_change(v1_13_2::play_toClient_packet_block_change::Ty {
		location: v1_13_2::position::Ty {
			x: -5,
			y: 70,
			z: 12,
		},
		r#type: sign as i32,
	});
	let out = to15(&packet, &mut Session::new());
	let [C15::block_change(b)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((b.location.x, b.location.y, b.location.z), (-5, 70, 12));
	assert_eq!(
		describe(V1_15_2, b.r#type as u32),
		"minecraft:oak_sign[rotation=9,waterlogged=true]"
	);
}

fn meta(key: u8, value: M13) -> E13 {
	let ty = match &value {
		M13::v0(_) => 0,
		M13::v1(_) => 1,
		M13::v2(_) => 2,
		M13::v7(_) => 7,
		_ => panic!(),
	};
	E13 {
		key,
		r#type: ty,
		value,
	}
}

fn living(id: i32, ty: i32, metadata: Vec<E13>) -> C13 {
	C13::spawn_entity_living(v1_13_2::play_toClient_packet_spawn_entity_living::Ty {
		entityId: id,
		entityUUID: id as u128,
		r#type: ty,
		x: 1.0,
		y: 2.0,
		z: 3.0,
		yaw: 0,
		pitch: 0,
		headPitch: 0,
		velocity: v1_13_2::vec3i16::Ty { x: 0, y: 0, z: 0 },
		metadata,
	})
}

fn keys(m: &[v1_14::entityMetadata__items::Ty]) -> Vec<(u8, M14)> {
	m.iter().map(|e| (e.key, e.value.clone())).collect()
}

#[test]
fn zombie_metadata_moves_hands_into_mob_flags() {
	let mut cx = Session::new();
	login(&mut cx);
	let zombie = living(
		10,
		ZOMBIE_13,
		vec![
			meta(0, M13::v0(0)),
			meta(7, M13::v2(20.0)),
			meta(11, M13::v0(1)),
			meta(12, M13::v7(true)),
			meta(14, M13::v7(true)),
		],
	);
	let out = to14(&zombie, &mut cx);
	let [C14::spawn_entity_living(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.r#type, entity(V1_14, "zombie"));
	assert_eq!(
		keys(&s.metadata),
		vec![
			(0, M14::v0(0)),
			(8, M14::v2(20.0)),
			(14, M14::v7(true)),
			(13, M14::v0(5))
		]
	);
	let mut cx = Session::new();
	login(&mut cx);
	let out = to15(&zombie, &mut cx);
	let [C15::spawn_entity_living(s), C15::entity_metadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((s.r#type, m.entityId), (entity(V1_15_2, "zombie"), 10));
	assert_eq!(
		keys(&m.metadata),
		vec![
			(0, M14::v0(0)),
			(8, M14::v2(20.0)),
			(15, M14::v7(true)),
			(14, M14::v0(5))
		]
	);
	let update = C13::entity_metadata(v1_13_2::play_toClient_packet_entity_metadata::Ty {
		entityId: 10,
		metadata: vec![meta(14, M13::v7(false))],
	});
	let out = to15(&update, &mut cx);
	let [C15::entity_metadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(keys(&m.metadata), vec![(14, M14::v0(1))]);
}

#[test]
fn villagers_get_villager_data_at_every_version() {
	let mut cx = Session::new();
	login(&mut cx);
	let villager = living(
		11,
		VILLAGER_13,
		vec![meta(12, M13::v7(false)), meta(13, M13::v1(1))],
	);
	let data = M14::v16(v1_14::entityMetadataItem__v16::Ty {
		villagerType: 2,
		villagerProfession: 9,
		level: 1,
	});
	let out = to14(&villager, &mut cx);
	let [C14::spawn_entity_living(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		keys(&s.metadata),
		vec![(14, M14::v7(false)), (15, data.clone())]
	);
	let mut cx = Session::new();
	login(&mut cx);
	let out = to15(&villager, &mut cx);
	let [C15::spawn_entity_living(_), C15::entity_metadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(keys(&m.metadata), vec![(15, M14::v7(false)), (17, data)]);
}

#[test]
fn players_get_a_pose_and_horses_armor_equipment() {
	let mut cx = Session::new();
	login(&mut cx);
	let player = C13::named_entity_spawn(v1_13_2::play_toClient_packet_named_entity_spawn::Ty {
		entityId: 12,
		playerUUID: 5,
		x: 0.0,
		y: 0.0,
		z: 0.0,
		yaw: 0,
		pitch: 0,
		metadata: vec![meta(0, M13::v0(0x02)), meta(13, M13::v0(0x7f))],
	});
	let out = to14(&player, &mut cx);
	let [C14::named_entity_spawn(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		keys(&s.metadata),
		vec![(0, M14::v0(2)), (15, M14::v0(0x7f)), (6, M14::v18(5))]
	);
	let horse = living(
		13,
		HORSE_13,
		vec![meta(15, M13::v1(3)), meta(16, M13::v1(3))],
	);
	let out = to14(&horse, &mut cx);
	let [C14::spawn_entity_living(s), C14::entity_equipment(e)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(keys(&s.metadata), vec![(17, M14::v1(3))]);
	let Slot::r#true(armor) = &e.item else {
		panic!()
	};
	assert_eq!(
		(e.entityId, e.slot, armor.itemId),
		(13, 4, item(V1_14, "diamond_horse_armor"))
	);
}

#[test]
fn every_mob_and_object_keeps_its_type() {
	let names15 = registry::names(V1_15_2, "entities").unwrap();
	let mut cx = Session::new();
	login(&mut cx);
	for (id, name) in &registry::names(V1_13_2, "entities").unwrap().names {
		let out = to15(&living(100 + *id as i32, *id as i32, Vec::new()), &mut cx);
		let [C15::spawn_entity_living(s)] = out.as_slice() else {
			panic!("{name}: {out:?}")
		};
		assert_eq!(names15.name(s.r#type as u32), Some(name.as_str()));
	}
	for (id, name) in &registry::names(V1_13_2, "objects").unwrap().names {
		let object = C13::spawn_entity(v1_13_2::play_toClient_packet_spawn_entity::Ty {
			entityId: 300 + *id as i32,
			objectUUID: 1,
			r#type: *id as i8,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			objectData: 1,
			velocity: v1_13_2::vec3i16::Ty { x: 0, y: 0, z: 0 },
		});
		let out = to15(&object, &mut cx);
		let C15::spawn_entity(s) = &out[0] else {
			panic!("{name}: {out:?}")
		};
		let expected = if name == "minecart" {
			"chest_minecart"
		} else {
			name.as_str()
		};
		assert_eq!(names15.name(s.r#type as u32), Some(expected));
	}
}

#[test]
fn objects_become_entity_types() {
	let mut cx = Session::new();
	login(&mut cx);
	let sand = state(V1_13_2, "minecraft:sand");
	let object = |id: i32, ty: i8, data: i32| {
		C13::spawn_entity(v1_13_2::play_toClient_packet_spawn_entity::Ty {
			entityId: id,
			objectUUID: 1,
			r#type: ty,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			objectData: data,
			velocity: v1_13_2::vec3i16::Ty { x: 1, y: 2, z: 3 },
		})
	};
	let out = to15(&object(20, 70, sand as i32), &mut cx);
	let [C15::spawn_entity(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.r#type, entity(V1_15_2, "falling_block"));
	assert_eq!(describe(V1_15_2, s.objectData as u32), "minecraft:sand");
	let out = to14(&object(21, 60, 6), &mut cx);
	let [C14::spawn_entity(s), C14::entity_velocity(v)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(s.r#type, s.objectData, v.entityId, v.velocity.z),
		(entity(V1_14, "arrow"), 5, 21, 3)
	);
	let out = to14(&object(22, 10, 1), &mut cx);
	let [C14::spawn_entity(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.r#type, entity(V1_14, "chest_minecart"));
}

fn lore_slot(id: i32, lines: &[&str]) -> Slot {
	let lore = Nbt::new(
		"Lore",
		Tag::list(NbtList::new(NbtItems::string(
			lines.iter().map(|l| l.to_string()).collect(),
		))),
	);
	let display = Nbt::new("display", Tag::compound(vec![lore]));
	Slot::r#true(v1_13_2::slot__true::Ty {
		itemId: id,
		itemCount: 3,
		nbtData: Some(Nbt::new("", Tag::compound(vec![display]))),
	})
}

#[test]
fn items_are_renamed_and_lore_becomes_json() {
	let packet = C13::set_slot(v1_13_2::play_toClient_packet_set_slot::Ty {
		windowId: 0,
		slot: 36,
		item: lore_slot(item(V1_13_2, "sign"), &["\u{a7}cHello \"world\""]),
	});
	let out = to15(&packet, &mut Session::new());
	let [C15::set_slot(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		s.item,
		lore_slot(
			item(V1_15_2, "oak_sign"),
			&["{\"text\":\"\u{a7}cHello \\\"world\\\"\"}"]
		)
	);
	let mut o = Output::new();
	v1_15_2::play_toServer_packet::serialize(
		&v1_15_2::play_toServer_packet::Ty::set_creative_slot(
			v1_15_2::play_toServer_packet_set_creative_slot::Ty {
				slot: 36,
				item: s.item.clone(),
			},
		),
		&mut o,
	)
	.unwrap();
	let back = convert(
		Root::PlayToServer,
		at(V1_15_2),
		at(V1_13_2),
		&o.into_inner(),
		&mut Session::new(),
	)
	.unwrap();
	let [bytes] = back.as_slice() else { panic!() };
	let v1_13_2::play_toServer_packet::Ty::set_creative_slot(s) =
		v1_13_2::play_toServer_packet::deserialize(&mut Input::new(bytes)).unwrap()
	else {
		panic!()
	};
	assert_eq!(
		s.item,
		lore_slot(item(V1_13_2, "sign"), &["\u{a7}cHello \"world\""])
	);
}

#[test]
fn trader_list_becomes_trade_list() {
	let mut data = Vec::new();
	data.extend(5i32.to_be_bytes());
	data.push(1);
	let mut o = Output::new();
	let emerald = Slot::r#true(v1_13_2::slot__true::Ty {
		itemId: item(V1_13_2, "emerald"),
		itemCount: 2,
		nbtData: None,
	});
	let dye = Slot::r#true(v1_13_2::slot__true::Ty {
		itemId: item(V1_13_2, "rose_red"),
		itemCount: 1,
		nbtData: None,
	});
	v1_13_2::slot::serialize(&emerald, &mut o).unwrap();
	v1_13_2::slot::serialize(&dye, &mut o).unwrap();
	data.extend(o.into_inner());
	data.push(0);
	data.push(0);
	data.extend(3i32.to_be_bytes());
	data.extend(7i32.to_be_bytes());
	let packet = C13::custom_payload(v1_13_2::play_toClient_packet_custom_payload::Ty {
		channel: "minecraft:trader_list".into(),
		data,
	});
	let out = to15(&packet, &mut Session::new());
	let [C15::trade_list(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(
			t.windowId,
			t.trades.len(),
			t.isRegularVillager,
			t.canRestock
		),
		(5, 1, false, false)
	);
	let trade = &t.trades[0];
	let Slot::r#true(out_item) = &trade.outputItem else {
		panic!()
	};
	assert_eq!(out_item.itemId, item(V1_15_2, "red_dye"));
	assert_eq!(
		(
			trade.inputItem2.is_none(),
			trade.nbTradeUses,
			trade.maximumNbTradeUses,
			trade.demand
		),
		(true, 3, 7, 0)
	);
}

#[test]
fn windows_get_registry_types() {
	let window = |kind: &str, slots: u8, horse: Option<i32>| {
		C13::open_window(v1_13_2::play_toClient_packet_open_window::Ty {
			windowId: 4,
			inventoryType: kind.into(),
			windowTitle: "{\"text\":\"x\"}".into(),
			slotCount: slots,
			entityId: match horse {
				Some(id) => {
					v1_13_2::play_toClient_packet_open_window__entityId::Ty::EntityHorse(id)
				}
				None => {
					v1_13_2::play_toClient_packet_open_window__entityId::Ty::default(kind.into())
				}
			},
		})
	};
	let mut cx = Session::new();
	let out = to14(&window("minecraft:chest", 27, None), &mut cx);
	assert!(
		matches!(out.as_slice(), [C14::open_window(w)] if (w.windowId, w.inventoryType) == (4, 2))
	);
	let out = to14(&window("minecraft:villager", 3, None), &mut cx);
	assert!(matches!(out.as_slice(), [C14::open_window(w)] if w.inventoryType == 18));
	let out = to14(&window("EntityHorse", 17, Some(33)), &mut cx);
	assert!(
		matches!(out.as_slice(), [C14::open_horse_window(w)] if (w.windowId, w.nbSlots, w.entityId) == (4, 17, 33))
	);
}

#[test]
fn view_position_follows_the_player() {
	let mut cx = Session::new();
	login(&mut cx);
	let teleport = C13::position(v1_13_2::play_toClient_packet_position::Ty {
		x: 100.5,
		y: 64.0,
		z: -200.5,
		yaw: 0.0,
		pitch: 0.0,
		flags: 0,
		teleportId: 1,
	});
	let out = to15(&teleport, &mut cx);
	let [C15::update_view_position(v), C15::position(_)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((v.chunkX, v.chunkZ), (6, -13));
	assert!(matches!(
		to15(&teleport, &mut cx).as_slice(),
		[C15::position(_)]
	));
	let mut o = Output::new();
	v1_15_2::play_toServer_packet::serialize(
		&v1_15_2::play_toServer_packet::Ty::position_look(
			v1_15_2::play_toServer_packet_position_look::Ty {
				x: 130.0,
				y: 64.0,
				z: -200.0,
				yaw: 0.0,
				pitch: 0.0,
				onGround: true,
			},
		),
		&mut o,
	)
	.unwrap();
	let sent = convert(
		Root::PlayToServer,
		at(V1_15_2),
		at(V1_13_2),
		&o.into_inner(),
		&mut cx,
	)
	.unwrap();
	assert_eq!(sent.len(), 1);
	let out = to15(&chunk_1_13_2(&column_1_13_2()), &mut cx);
	let [C15::update_view_position(v), C15::update_light(_), C15::map_chunk(_)] = out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!((v.chunkX, v.chunkZ), (8, -13));
	let out = to15(&chunk_1_13_2(&column_1_13_2()), &mut cx);
	assert!(matches!(
		out.as_slice(),
		[C15::update_light(_), C15::map_chunk(_)]
	));
	let C13::map_chunk(mut far) = chunk_1_13_2(&column_1_13_2()) else {
		panic!()
	};
	far.x = 300;
	let out = to15(&C13::map_chunk(far), &mut cx);
	let [C15::update_view_position(v), C15::update_light(_), C15::map_chunk(_)] = out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!(
		(v.chunkX, v.chunkZ),
		(300, -3),
		"chunks far from the player move the view instead of being dropped"
	);
}

#[test]
fn particles_are_renumbered() {
	let packet = C13::world_particles(v1_13_2::play_toClient_packet_world_particles::Ty {
		particleId: 11,
		longDistance: false,
		x: 1.5,
		y: 2.5,
		z: 3.5,
		offsetX: 0.0,
		offsetY: 0.0,
		offsetZ: 0.0,
		particleData: 0.0,
		particles: 4,
		data: v1_13_2::particleData::Ty::v11(v1_13_2::particleData__v11::Ty {
			red: 1.0,
			green: 0.5,
			blue: 0.0,
			scale: 1.0,
		}),
	});
	let out = to15(&packet, &mut Session::new());
	let [C15::world_particles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((p.particleId, p.x, p.particles), (14, 1.5, 4));
	assert!(matches!(&p.data, v1_15_2::particleData::Ty::v14(d) if d.green == 0.5));
}

#[test]
fn recipes_and_tags_use_new_ids() {
	let stick = Slot::r#true(v1_13_2::slot__true::Ty {
		itemId: item(V1_13_2, "stick"),
		itemCount: 1,
		nbtData: None,
	});
	let sign = Slot::r#true(v1_13_2::slot__true::Ty {
		itemId: item(V1_13_2, "sign"),
		itemCount: 3,
		nbtData: None,
	});
	let recipes = C13::declare_recipes(vec![
		v1_13_2::play_toClient_packet__declare_recipes__items::Ty {
			recipeId: "minecraft:sign".into(),
			r#type: "crafting_shaped".into(),
			data: v1_13_2::play_toClient_packet__declare_recipes__items__data::Ty::crafting_shaped(
				v1_13_2::play_toClient_packet__declare_recipes__items__data__crafting_shaped::Ty {
					width: 1,
					height: 1,
					group: String::new(),
					ingredients: vec![vec![vec![stick]]],
					result: sign,
				},
			),
		},
		v1_13_2::play_toClient_packet__declare_recipes__items::Ty {
			recipeId: "minecraft:repair_item".into(),
			r#type: "crafting_special_repairitem".into(),
			data: v1_13_2::play_toClient_packet__declare_recipes__items__data::Ty::crafting_special_repairitem,
		},
	]);
	let out = to15(&recipes, &mut Session::new());
	let [C15::declare_recipes(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(r.len(), 1);
	assert_eq!(r[0].r#type, "minecraft:crafting_shaped");
	let v1_15_2::play_toClient_packet__declare_recipes__items__data::Ty::minecraft_crafting_shaped(
		s,
	) = &r[0].data
	else {
		panic!()
	};
	assert!(matches!(&s.result, Slot::r#true(i) if i.itemId == item(V1_15_2, "oak_sign")));
	assert!(
		matches!(&s.ingredients[0][0][0], Slot::r#true(i) if i.itemId == item(V1_15_2, "stick"))
	);
	let sign_block = registry::blocks(V1_13_2).unwrap().block("sign").unwrap().id as i32;
	let tags = C13::tags(v1_13_2::play_toClient_packet_tags::Ty {
		blockTags: vec![v1_13_2::play_toClient_packet_tags__blockTags__items::Ty {
			tagName: "minecraft:signs".into(),
			entries: vec![sign_block],
		}],
		itemTags: vec![v1_13_2::play_toClient_packet_tags__blockTags__items::Ty {
			tagName: "minecraft:x".into(),
			entries: vec![item(V1_13_2, "dandelion_yellow")],
		}],
		fluidTags: Vec::new(),
	});
	let out = to15(&tags, &mut Session::new());
	let [C15::tags(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let oak_sign = registry::blocks(V1_15_2)
		.unwrap()
		.block("oak_sign")
		.unwrap()
		.id as i32;
	assert_eq!(
		(
			t.blockTags[0].entries.as_slice(),
			t.itemTags[0].entries.as_slice()
		),
		(&[oak_sign][..], &[item(V1_15_2, "yellow_dye")][..])
	);
	assert!(t.entityTags.is_empty());
}

#[test]
fn nbt_arguments_and_explosions_follow_1_14() {
	use v1_13_2::command_node__extraNodeData__v2__properties::Ty as Parser;
	let node = |parser: &str, properties: Parser| v1_13_2::command_node::Ty {
		flags: v1_13_2::command_node__flags::Ty {
			unused: 0,
			has_custom_suggestions: 0,
			has_redirect_node: 0,
			has_command: 1,
			command_node_type: 2,
		},
		children: Vec::new(),
		redirectNode: v1_13_2::command_node__redirectNode::Ty::default(0),
		extraNodeData: v1_13_2::command_node__extraNodeData::Ty::v2(
			v1_13_2::command_node__extraNodeData__v2::Ty {
				name: "tag".into(),
				parser: parser.into(),
				properties,
				suggestionType:
					v1_13_2::command_node__extraNodeData__v2__suggestionType::Ty::default(0),
			},
		),
	};
	let packet = C13::declare_commands(v1_13_2::play_toClient_packet_declare_commands::Ty {
		nodes: vec![node("minecraft:nbt", Parser::minecraft_nbt)],
		rootIndex: 0,
	});
	let out = to15(&packet, &mut Session::new());
	let [C15::declare_commands(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		c.nodes,
		vec![node(
			"minecraft:nbt_compound_tag",
			Parser::minecraft_nbt_compound_tag
		)]
	);
	let explosion = C13::explosion(v1_13_2::play_toClient_packet_explosion::Ty {
		x: -10.5,
		y: 64.5,
		z: 3.25,
		radius: 4.0,
		affectedBlockOffsets: Vec::new(),
		playerMotionX: 0.0,
		playerMotionY: 0.0,
		playerMotionZ: 0.0,
	});
	let out = to15(&explosion, &mut Session::new());
	let [C15::explosion(e)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((e.x, e.y, e.z), (-10.0, 64.5, 3.25));
}

#[test]
fn select_trade_forces_an_inventory_resync() {
	let mut cx = Session::new();
	let mut o = Output::new();
	v1_15_2::play_toServer_packet::serialize(
		&v1_15_2::play_toServer_packet::Ty::select_trade(2),
		&mut o,
	)
	.unwrap();
	let out = convert(
		Root::PlayToServer,
		at(V1_15_2),
		at(V1_13_2),
		&o.into_inner(),
		&mut cx,
	)
	.unwrap();
	let packets: Vec<_> = out
		.iter()
		.map(|b| v1_13_2::play_toServer_packet::deserialize(&mut Input::new(b)).unwrap())
		.collect();
	let [v1_13_2::play_toServer_packet::Ty::select_trade(2), v1_13_2::play_toServer_packet::Ty::window_click(c)] =
		packets.as_slice()
	else {
		panic!("{packets:?}")
	};
	assert_eq!((c.slot, c.mouseButton, c.mode), (-999, 2, 5));
}

type M15 = v1_15_2::entityMetadataItem::Ty;
type E15 = v1_15_2::entityMetadata__items::Ty;

fn down(p: &C15, cx: &mut Session) -> Vec<C13> {
	let mut o = Output::new();
	v1_15_2::play_toClient_packet::serialize(p, &mut o).unwrap();
	convert(
		Root::PlayToClient,
		at(V1_15_2),
		at(V1_13_2),
		&o.into_inner(),
		cx,
	)
	.unwrap()
	.iter()
	.map(|b| {
		let mut input = Input::new(b);
		let p = v1_13_2::play_toClient_packet::deserialize(&mut input).unwrap();
		assert_eq!(input.remaining(), 0, "leftover bytes in {p:?}");
		p
	})
	.collect()
}

fn login15(cx: &mut Session, respawn_screen: bool) {
	down(
		&C15::login(v1_15_2::play_toClient_packet_login::Ty {
			entityId: 1,
			gameMode: 0,
			dimension: 0,
			hashedSeed: 0,
			maxPlayers: 20,
			levelType: "default".into(),
			viewDistance: 10,
			reducedDebugInfo: false,
			enableRespawnScreen: respawn_screen,
		}),
		cx,
	);
}

fn meta15(key: u8, value: M15) -> E15 {
	let r#type = match &value {
		M15::v0(_) => 0,
		M15::v1(_) => 1,
		M15::v2(_) => 2,
		M15::v7(_) => 7,
		M15::v10(_) => 10,
		_ => panic!(),
	};
	E15 { key, r#type, value }
}

fn mob15(id: i32, name: &str) -> C15 {
	C15::spawn_entity_living(v1_15_2::play_toClient_packet_spawn_entity_living::Ty {
		entityId: id,
		entityUUID: id as u128,
		r#type: entity(V1_15_2, name),
		x: 10.0,
		y: 64.0,
		z: 20.0,
		yaw: 0,
		pitch: 0,
		headPitch: 0,
		velocity: v1_15_2::vec3i16::Ty { x: 0, y: 0, z: 0 },
	})
}

fn metadata15(id: i32, metadata: Vec<E15>) -> C15 {
	C15::entity_metadata(v1_15_2::play_toClient_packet_entity_metadata::Ty {
		entityId: id,
		metadata,
	})
}

fn keys13(m: &[E13]) -> Vec<(u8, M13)> {
	m.iter().map(|e| (e.key, e.value.clone())).collect()
}

#[test]
fn newer_mobs_get_older_stand_ins() {
	let mut cx = Session::new();
	login15(&mut cx, true);
	let out = down(&mob15(30, "fox"), &mut cx);
	let [C13::spawn_entity_living(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.r#type, entity(V1_13_2, "wolf"));
	assert_eq!(
		keys13(&s.metadata),
		vec![(2, M13::v5(Some("{\"text\":\"Fox\"}".into())))]
	);
	let out = down(
		&metadata15(
			30,
			vec![
				meta15(15, M15::v7(true)),
				meta15(16, M15::v1(1)),
				meta15(17, M15::v0(4)),
			],
		),
		&mut cx,
	);
	let [C13::entity_metadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(keys13(&m.metadata), vec![(12, M13::v7(true))]);
	down(&mob15(31, "cat"), &mut cx);
	let out = down(
		&metadata15(
			31,
			vec![
				meta15(16, M15::v0(4)),
				meta15(18, M15::v1(10)),
				meta15(19, M15::v7(true)),
			],
		),
		&mut cx,
	);
	let [C13::entity_metadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		keys13(&m.metadata),
		vec![(13, M13::v0(4)), (15, M13::v1(1))]
	);
	let out = down(&mob15(32, "bee"), &mut cx);
	let [C13::spawn_entity_living(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.r#type, entity(V1_13_2, "pufferfish"));
	assert_eq!(
		keys13(&s.metadata),
		vec![
			(12, M13::v7(false)),
			(13, M13::v1(2)),
			(2, M13::v5(Some("{\"text\":\"Bee\"}".into())))
		]
	);
	let out = down(&mob15(33, "zombie"), &mut cx);
	let [C13::spawn_entity_living(_)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let out = down(
		&metadata15(33, vec![meta15(14, M15::v0(5)), meta15(15, M15::v7(true))]),
		&mut cx,
	);
	let [C13::entity_metadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		keys13(&m.metadata),
		vec![(11, M13::v0(1)), (14, M13::v7(true)), (12, M13::v7(true))]
	);
}

#[test]
fn equipment_beds_and_sounds_reach_old_clients() {
	let mut cx = Session::new();
	login15(&mut cx, true);
	down(&mob15(40, "horse"), &mut cx);
	let armor = Slot::r#true(v1_13_2::slot__true::Ty {
		itemId: item(V1_15_2, "diamond_horse_armor"),
		itemCount: 1,
		nbtData: None,
	});
	let out = down(
		&C15::entity_equipment(v1_15_2::play_toClient_packet_entity_equipment::Ty {
			entityId: 40,
			slot: 4,
			item: armor,
		}),
		&mut cx,
	);
	let [C13::entity_metadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(keys13(&m.metadata), vec![(16, M13::v1(3))]);
	down(
		&C15::named_entity_spawn(v1_15_2::play_toClient_packet_named_entity_spawn::Ty {
			entityId: 41,
			playerUUID: 41,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			yaw: 0,
			pitch: 0,
		}),
		&mut cx,
	);
	let bed = v1_15_2::position::Ty { x: 3, y: 70, z: -4 };
	let out = down(
		&metadata15(41, vec![meta15(13, M15::v10(Some(bed)))]),
		&mut cx,
	);
	let [C13::entity_metadata(m), C13::bed(b)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert!(m.metadata.is_empty());
	assert_eq!(
		(b.entityId, b.location.x, b.location.y, b.location.z),
		(41, 3, 70, -4)
	);
	down(&mob15(42, "zombie"), &mut cx);
	down(
		&C15::rel_entity_move(v1_15_2::play_toClient_packet_rel_entity_move::Ty {
			entityId: 42,
			dX: 4096,
			dY: 0,
			dZ: -2048,
			onGround: true,
		}),
		&mut cx,
	);
	let sound = |version| {
		registry::names(version, "sounds")
			.unwrap()
			.id("entity.zombie.ambient")
			.unwrap() as i32
	};
	let out = down(
		&C15::entity_sound_effect(v1_15_2::play_toClient_packet_entity_sound_effect::Ty {
			soundId: sound(V1_15_2),
			soundCategory: 5,
			entityId: 42,
			volume: 1.0,
			pitch: 1.0,
		}),
		&mut cx,
	);
	let [C13::sound_effect(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(s.soundId, s.soundCategory, s.x, s.y, s.z),
		(sound(V1_13_2), 5, 88, 512, 156)
	);
}

#[test]
fn json_lore_becomes_legacy_text() {
	let lore = "{\"text\":\"Hi\",\"color\":\"red\",\"bold\":true,\"extra\":[{\"text\":\" there\",\"color\":\"blue\"},\"!\"]}";
	let packet = C15::set_slot(v1_15_2::play_toClient_packet_set_slot::Ty {
		windowId: 0,
		slot: 36,
		item: lore_slot(item(V1_15_2, "andesite_wall"), &[lore]),
	});
	let out = down(&packet, &mut Session::new());
	let [C13::set_slot(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		s.item,
		lore_slot(
			item(V1_13_2, "cobblestone_wall"),
			&["\u{a7}c\u{a7}lHi\u{a7}9\u{a7}l there\u{a7}c\u{a7}l!"]
		)
	);
}

#[test]
fn newer_blocks_windows_and_commands_degrade() {
	let mut cx = Session::new();
	login15(&mut cx, true);
	let wall = state(V1_15_2, "minecraft:andesite_wall[east=true,north=false,south=false,up=true,waterlogged=false,west=true]");
	let out = down(
		&C15::block_change(v1_15_2::play_toClient_packet_block_change::Ty {
			location: v1_15_2::position::Ty { x: 1, y: 2, z: 3 },
			r#type: wall as i32,
		}),
		&mut cx,
	);
	let [C13::block_change(b)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		describe(V1_13_2, b.r#type as u32),
		"minecraft:cobblestone_wall[east=true,north=false,south=false,up=true,waterlogged=false,west=true]"
	);
	let window = |ty: i32| {
		C15::open_window(v1_15_2::play_toClient_packet_open_window::Ty {
			windowId: 7,
			inventoryType: ty,
			windowTitle: "{\"translate\":\"container.grindstone_title\"}".into(),
		})
	};
	let out = down(&window(14), &mut cx);
	let [C13::open_window(w)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(
			w.inventoryType.as_str(),
			w.slotCount,
			w.windowTitle.as_str()
		),
		("minecraft:furnace", 3, "{\"text\":\"Grindstone\"}")
	);
	assert!(down(&window(16), &mut cx).is_empty());
	use v1_13_2::command_node__extraNodeData__v2__properties::Ty as Parser;
	let node = |parser: &str, properties: Parser| v1_13_2::command_node::Ty {
		flags: v1_13_2::command_node__flags::Ty {
			unused: 0,
			has_custom_suggestions: 0,
			has_redirect_node: 0,
			has_command: 1,
			command_node_type: 2,
		},
		children: Vec::new(),
		redirectNode: v1_13_2::command_node__redirectNode::Ty::default(0),
		extraNodeData: v1_13_2::command_node__extraNodeData::Ty::v2(
			v1_13_2::command_node__extraNodeData__v2::Ty {
				name: "arg".into(),
				parser: parser.into(),
				properties,
				suggestionType:
					v1_13_2::command_node__extraNodeData__v2__suggestionType::Ty::default(0),
			},
		),
	};
	let out = down(
		&C15::declare_commands(v1_15_2::play_toClient_packet_declare_commands::Ty {
			nodes: vec![
				node("minecraft:time", Parser::minecraft_time),
				node(
					"minecraft:nbt_tag",
					Parser::default("minecraft:nbt_tag".into()),
				),
			],
			rootIndex: 0,
		}),
		&mut cx,
	);
	let [C13::declare_commands(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let parsers: Vec<String> = c
		.nodes
		.iter()
		.map(|n| match &n.extraNodeData {
			v1_13_2::command_node__extraNodeData::Ty::v2(a) => a.parser.clone(),
			_ => String::new(),
		})
		.collect();
	assert_eq!(parsers, ["brigadier:integer", "brigadier:string"]);
}

#[test]
fn statistics_and_digging_follow_old_registries() {
	let mut cx = Session::new();
	login15(&mut cx, true);
	let stat = |category, id| v1_15_2::play_toClient_packet__statistics__items::Ty {
		categoryId: category,
		statisticId: id,
		value: 3,
	};
	let out = down(
		&C15::statistics(vec![
			stat(8, 10),
			stat(8, 60),
			stat(8, 70),
			stat(1, item(V1_15_2, "red_dye")),
		]),
		&mut cx,
	);
	let [C13::statistics(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let got: Vec<(i32, i32)> = s.iter().map(|s| (s.categoryId, s.statisticId)).collect();
	assert_eq!(got, [(8, 10), (1, item(V1_13_2, "rose_red"))]);
	let stone = state(V1_15_2, "minecraft:stone") as i32;
	let ack = |successful| {
		C15::acknowledge_player_digging(
			v1_15_2::play_toClient_packet_acknowledge_player_digging::Ty {
				location: v1_15_2::position::Ty { x: 5, y: 6, z: 7 },
				block: stone,
				status: 0,
				successful,
			},
		)
	};
	assert!(down(&ack(true), &mut cx).is_empty());
	let out = down(&ack(false), &mut cx);
	let [C13::block_change(b)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(describe(V1_13_2, b.r#type as u32), "minecraft:stone");
}

#[test]
fn immediate_respawn_respawns_for_old_clients() {
	let mut cx = Session::new();
	login15(&mut cx, false);
	let health = |health| {
		C15::update_health(v1_15_2::play_toClient_packet_update_health::Ty {
			health,
			food: 20,
			foodSaturation: 5.0,
		})
	};
	down(&health(10.0), &mut cx);
	assert!(cx.take_outgoing().is_empty());
	let out = down(&health(0.0), &mut cx);
	assert!(matches!(out.as_slice(), [C13::update_health(_)]));
	let sent = cx.take_outgoing();
	let [packet] = sent.as_slice() else {
		panic!("{sent:?}")
	};
	assert_eq!(
		(packet.root, packet.version),
		(Root::PlayToServer, V1_15 as usize)
	);
	assert_eq!(
		v1_15_2::play_toServer_packet::deserialize(&mut Input::new(&packet.data)).unwrap(),
		v1_15_2::play_toServer_packet::Ty::client_command(
			v1_15_2::play_toServer_packet_client_command::Ty::perform_respawn
		)
	);
}

#[test]
fn old_clients_reach_new_servers() {
	let mut cx = Session::new();
	let send = |p: v1_13_2::play_toServer_packet::Ty, cx: &mut Session| {
		let mut o = Output::new();
		v1_13_2::play_toServer_packet::serialize(&p, &mut o).unwrap();
		convert(
			Root::PlayToServer,
			at(V1_13_2),
			at(V1_15_2),
			&o.into_inner(),
			cx,
		)
		.unwrap()
		.iter()
		.map(|b| v1_15_2::play_toServer_packet::deserialize(&mut Input::new(b)).unwrap())
		.collect::<Vec<_>>()
	};
	let out = send(
		v1_13_2::play_toServer_packet::Ty::block_place(
			v1_13_2::play_toServer_packet_block_place::Ty {
				location: v1_13_2::position::Ty { x: -1, y: 64, z: 9 },
				direction: 1,
				hand: 0,
				cursorX: 0.5,
				cursorY: 1.0,
				cursorZ: 0.5,
			},
		),
		&mut cx,
	);
	let [v1_15_2::play_toServer_packet::Ty::block_place(b)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(
			b.location.x,
			b.location.y,
			b.location.z,
			b.direction,
			b.insideBlock
		),
		(-1, 64, 9, 1, false)
	);
	let out = send(
		v1_13_2::play_toServer_packet::Ty::set_creative_slot(
			v1_13_2::play_toServer_packet_set_creative_slot::Ty {
				slot: 36,
				item: lore_slot(item(V1_13_2, "rose_red"), &["\u{a7}aGreen"]),
			},
		),
		&mut cx,
	);
	let [v1_15_2::play_toServer_packet::Ty::set_creative_slot(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		s.item,
		lore_slot(item(V1_15_2, "red_dye"), &["{\"text\":\"\u{a7}aGreen\"}"])
	);
}
