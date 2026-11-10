use minecraft_versions::gen::convert::{convert, Root, Version};
use minecraft_versions::gen::convert::{v1_12_2 as a, v1_13 as b, v1_13_2 as d};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::registry;
use minecraft_versions::Session;
use nbt::{Entry, List, ListItems, Payload};

type Client12 = a::play_toClient_packet::Ty;
type Client13 = b::play_toClient_packet::Ty;
type Client132 = d::play_toClient_packet::Ty;

fn encode<T>(v: &T, f: fn(&T, &mut Output) -> minecraft_versions::pds::Result<()>) -> Vec<u8> {
	let mut o = Output::new();
	f(v, &mut o).unwrap();
	o.into_inner()
}

fn decode<T>(bytes: &[u8], f: fn(&mut Input) -> minecraft_versions::pds::Result<T>) -> T {
	let mut input = Input::new(bytes);
	let v = f(&mut input).unwrap();
	assert_eq!(input.remaining(), 0);
	v
}

fn to_1_13(p: Client12, cx: &mut Session) -> Vec<Client13> {
	let out = convert(
		Root::PlayToClient,
		Version::V1_12_2.index(),
		Version::V1_13.index(),
		&encode(&p, a::play_toClient_packet::serialize),
		cx,
	)
	.unwrap();
	out.iter()
		.map(|b| decode(b, b::play_toClient_packet::deserialize))
		.collect()
}

fn to_1_13_2(p: Client12, cx: &mut Session) -> Vec<Client132> {
	let out = convert(
		Root::PlayToClient,
		Version::V1_12_2.index(),
		Version::V1_13_2.index(),
		&encode(&p, a::play_toClient_packet::serialize),
		cx,
	)
	.unwrap();
	out.iter()
		.map(|b| decode(b, d::play_toClient_packet::deserialize))
		.collect()
}

fn put_varint(out: &mut Vec<u8>, v: u32) {
	let mut v = v;
	loop {
		if v & !0x7f == 0 {
			out.push(v as u8);
			return;
		}
		out.push((v as u8 & 0x7f) | 0x80);
		v >>= 7;
	}
}

fn varint(data: &[u8], at: &mut usize) -> u32 {
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

fn pack(values: &[u32], bits: usize) -> Vec<u64> {
	let mut longs = vec![0u64; (4096 * bits).div_ceil(64)];
	for (i, v) in values.iter().enumerate() {
		let bit = i * bits;
		let (w, off) = (bit / 64, bit % 64);
		longs[w] |= (*v as u64) << off;
		if off + bits > 64 {
			longs[w + 1] |= (*v as u64) >> (64 - off);
		}
	}
	longs
}

fn legacy_section(out: &mut Vec<u8>, states: &[u32], palette: Option<&[u32]>) {
	let (bits, values) = match palette {
		Some(p) => (
			4,
			states
				.iter()
				.map(|s| p.iter().position(|x| x == s).unwrap() as u32)
				.collect::<Vec<_>>(),
		),
		None => (13, states.to_vec()),
	};
	out.push(bits as u8);
	match palette {
		Some(p) => {
			put_varint(out, p.len() as u32);
			for s in p {
				put_varint(out, *s);
			}
		}
		None => put_varint(out, 0),
	}
	let longs = pack(&values, bits);
	put_varint(out, longs.len() as u32);
	for l in longs {
		out.extend(l.to_be_bytes());
	}
	out.extend(std::iter::repeat(0x0f).take(2048));
	out.extend(std::iter::repeat(0xf0).take(2048));
}

struct Flat {
	sections: Vec<Vec<u32>>,
	biomes: Vec<i32>,
}

fn read_flat(data: &[u8], count: usize) -> Flat {
	let mut at = 0;
	let mut sections = Vec::new();
	for _ in 0..count {
		let bits = data[at] as usize;
		at += 1;
		let palette = (bits <= 8).then(|| {
			let n = varint(data, &mut at);
			(0..n).map(|_| varint(data, &mut at)).collect::<Vec<_>>()
		});
		let n = varint(data, &mut at) as usize;
		let longs: Vec<u64> = (0..n)
			.map(|i| u64::from_be_bytes(data[at + i * 8..at + i * 8 + 8].try_into().unwrap()))
			.collect();
		at += n * 8;
		let states = (0..4096)
			.map(|i| {
				let bit = i * bits;
				let (w, off) = (bit / 64, bit % 64);
				let mut v = longs[w] >> off;
				if off + bits > 64 {
					v |= longs[w + 1] << (64 - off);
				}
				let v = (v & ((1 << bits) - 1)) as u32;
				palette.as_ref().map_or(v, |p| p[v as usize])
			})
			.collect();
		assert!(data[at..at + 2048].iter().all(|b| *b == 0x0f));
		assert!(data[at + 2048..at + 4096].iter().all(|b| *b == 0xf0));
		at += 4096;
		sections.push(states);
	}
	let biomes = (0..256)
		.map(|i| i32::from_be_bytes(data[at + i * 4..at + i * 4 + 4].try_into().unwrap()))
		.collect();
	assert_eq!(at + 1024, data.len());
	Flat { sections, biomes }
}

fn index(x: usize, y: usize, z: usize) -> usize {
	(y & 15) << 8 | z << 4 | x
}

fn legacy_chunk(blocks: &[(usize, usize, usize, u32)], top: &[u32]) -> Client12 {
	let palette = [
		0,
		7 << 4,
		1 << 4,
		1 << 4 | 1,
		35 << 4 | 14,
		46 << 4,
		144 << 4 | 1,
	];
	let mut low = vec![0u32; 4096];
	for (x, y, z, s) in blocks {
		low[index(*x, *y, *z)] = *s;
	}
	let mut data = Vec::new();
	legacy_section(&mut data, &low, Some(&palette));
	legacy_section(&mut data, top, None);
	data.extend((0..=255u8).map(|b| if b == 7 { 200 } else { b % 40 }));
	Client12::map_chunk(a::play_toClient_packet_map_chunk::Ty {
		x: 2,
		z: -3,
		groundUp: true,
		bitMap: 0b1001,
		chunkData: data,
		blockEntities: vec![],
	})
}

fn describe(version: Version, state: u32) -> String {
	registry::blocks(version).unwrap().describe(state).unwrap()
}

#[test]
fn chunks_flatten_palettes_and_direct_sections() {
	let top: Vec<u32> = (0..4096u32)
		.map(|i| (((i % 200) + 1) << 4) | (i % 3))
		.collect();
	let blocks = [
		(0, 0, 0, 7 << 4),
		(1, 1, 0, 1 << 4 | 1),
		(2, 2, 3, 35 << 4 | 14),
		(4, 5, 6, 46 << 4),
	];
	let out = to_1_13(legacy_chunk(&blocks, &top), &mut Session::new());
	let [Client13::map_chunk(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((c.x, c.z, c.bitMap), (2, -3, 0b1001));
	let flat = read_flat(&c.chunkData, 2);
	let low = &flat.sections[0];
	assert_eq!(
		describe(Version::V1_13, low[index(0, 0, 0)]),
		"minecraft:bedrock"
	);
	assert_eq!(
		describe(Version::V1_13, low[index(1, 1, 0)]),
		"minecraft:granite"
	);
	assert_eq!(
		describe(Version::V1_13, low[index(2, 2, 3)]),
		"minecraft:red_wool"
	);
	assert_eq!(
		describe(Version::V1_13, low[index(4, 5, 6)]),
		"minecraft:tnt"
	);
	assert_eq!(
		describe(Version::V1_13, low[index(9, 9, 9)]),
		"minecraft:air"
	);
	let blocks13 = registry::blocks(Version::V1_13).unwrap();
	for (i, s) in flat.sections[1].iter().enumerate() {
		let id = (i as u32 % 200) + 1;
		let expected = registry::legacy_block(id as u16, (i % 3) as u8)
			.or_else(|| registry::legacy_block(id as u16, 0))
			.and_then(|n| blocks13.parse_state(n))
			.unwrap_or(0);
		if ![
			18, 53, 69, 77, 96, 132, 139, 140, 143, 144, 149, 150, 161, 175,
		]
		.contains(&id)
		{
			let name = |s: u32| blocks13.owner(s).map(|b| b.name.as_str());
			assert_eq!(name(*s), name(expected), "block {id}:{}", i % 3);
		}
	}
	assert_eq!(flat.biomes[7], 1);
	assert_eq!(flat.biomes[41], 1);
}

#[test]
fn chunks_and_block_changes_reach_1_13_2() {
	let mut cx = Session::new();
	let out = to_1_13_2(
		legacy_chunk(&[(4, 5, 6, 46 << 4)], &vec![1 << 4; 4096]),
		&mut cx,
	);
	let [Client132::map_chunk(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let flat = read_flat(&c.chunkData, 2);
	assert_eq!(
		describe(Version::V1_13_2, flat.sections[0][index(4, 5, 6)]),
		"minecraft:tnt[unstable=false]"
	);
	assert_eq!(
		describe(Version::V1_13_2, flat.sections[1][0]),
		"minecraft:stone"
	);
	let change = Client12::block_change(a::play_toClient_packet_block_change::Ty {
		location: a::position::Ty { x: 1, y: 64, z: -1 },
		r#type: 64 << 4 | 2,
	});
	let out = to_1_13_2(change, &mut cx);
	let [Client132::block_change(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert!(describe(Version::V1_13_2, c.r#type as u32)
		.starts_with("minecraft:oak_door[facing=west,half=lower"));
}

fn column(x: i32, z: i32, blocks: &[(usize, usize, usize, u32)]) -> Client12 {
	let mut states = vec![0u32; 4096];
	for (bx, by, bz, s) in blocks {
		states[index(*bx, *by, *bz)] = *s;
	}
	let mut data = Vec::new();
	legacy_section(&mut data, &states, None);
	data.extend([1u8; 256]);
	Client12::map_chunk(a::play_toClient_packet_map_chunk::Ty {
		x,
		z,
		groundUp: true,
		bitMap: 1,
		chunkData: data,
		blockEntities: vec![],
	})
}

#[test]
fn connections_are_computed_for_flattened_blocks() {
	let mut cx = Session::new();
	let fence = 85 << 4;
	let blocks = [
		(5, 4, 5, fence),
		(6, 4, 5, fence),
		(5, 4, 4, 1 << 4),
		(8, 4, 8, 64 << 4 | 5),
		(8, 5, 8, 64 << 4 | 9),
		(10, 4, 10, 175 << 4 | 4),
		(10, 5, 10, 175 << 4 | 8),
		(12, 4, 12, 102 << 4),
		(15, 4, 0, fence),
		(3, 4, 3, 2 << 4),
		(3, 5, 3, 78 << 4),
	];
	let out = to_1_13(column(0, 0, &blocks), &mut cx);
	let [Client13::map_chunk(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let flat = read_flat(&c.chunkData, 1);
	let at = |x, y, z| describe(Version::V1_13, flat.sections[0][index(x, y, z)]);
	assert_eq!(
		at(5, 4, 5),
		"minecraft:oak_fence[east=true,north=true,south=false,waterlogged=false,west=false]"
	);
	assert_eq!(
		at(6, 4, 5),
		"minecraft:oak_fence[east=false,north=false,south=false,waterlogged=false,west=true]"
	);
	assert_eq!(
		at(8, 4, 8),
		"minecraft:oak_door[facing=south,half=lower,hinge=right,open=true,powered=false]"
	);
	assert_eq!(
		at(8, 5, 8),
		"minecraft:oak_door[facing=south,half=upper,hinge=right,open=true,powered=false]"
	);
	assert_eq!(at(10, 5, 10), "minecraft:rose_bush[half=upper]");
	assert_eq!(
		at(12, 4, 12),
		"minecraft:glass_pane[east=true,north=true,south=true,waterlogged=false,west=true]"
	);
	assert_eq!(at(3, 4, 3), "minecraft:grass_block[snowy=true]");
	let out = to_1_13(column(1, 0, &[(0, 4, 0, fence)]), &mut cx);
	let [Client13::map_chunk(_), Client13::multi_block_change(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((m.chunkX, m.chunkZ, m.records.len()), (0, 0, 1));
	assert_eq!(
		describe(Version::V1_13, m.records[0].blockId as u32),
		"minecraft:oak_fence[east=true,north=false,south=false,waterlogged=false,west=false]"
	);
	let change = Client12::block_change(a::play_toClient_packet_block_change::Ty {
		location: a::position::Ty { x: 5, y: 4, z: 6 },
		r#type: fence as i32,
	});
	let out = to_1_13(change, &mut cx);
	let [Client13::block_change(placed), Client13::multi_block_change(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		describe(Version::V1_13, placed.r#type as u32),
		"minecraft:oak_fence[east=false,north=true,south=false,waterlogged=false,west=false]"
	);
	assert_eq!(
		describe(Version::V1_13, m.records[0].blockId as u32),
		"minecraft:oak_fence[east=true,north=true,south=true,waterlogged=false,west=false]"
	);
}

fn compound(entries: Vec<Entry>) -> Payload {
	Payload::compound(entries)
}

fn named(name: &str, value: Payload) -> Entry {
	Entry::new(name, value)
}

fn get<'a>(c: &'a Payload, name: &str) -> Option<&'a Payload> {
	c.field(name)
}

#[test]
fn skulls_take_their_type_and_rotation_from_block_entities() {
	let mut cx = Session::new();
	to_1_13(
		legacy_chunk(&[(1, 6, 1, 144 << 4 | 1)], &vec![0; 4096]),
		&mut cx,
	);
	let te = Client12::tile_entity_data(a::play_toClient_packet_tile_entity_data::Ty {
		location: a::position::Ty {
			x: 33,
			y: 6,
			z: -47,
		},
		action: 4,
		nbtData: Some(named(
			"",
			compound(vec![
				named("id", Payload::string("Skull".to_owned())),
				named("SkullType", Payload::byte(3)),
				named("Rot", Payload::byte(4)),
			]),
		)),
	});
	let out = to_1_13(te, &mut cx);
	let [Client13::block_change(c), Client13::tile_entity_data(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		describe(Version::V1_13, c.r#type as u32),
		"minecraft:player_head[rotation=4]"
	);
	assert_eq!(t.action, 4);
}

fn sword() -> a::slot::Ty {
	let ench = Payload::list(List::new(ListItems::compound(vec![vec![
		named("id", Payload::short(16)),
		named("lvl", Payload::short(3)),
	]])));
	let display = compound(vec![named(
		"Name",
		Payload::string("\u{a7}cBlade".to_owned()),
	)]);
	a::slot::Ty::default(
		276,
		a::slot__default::Ty {
			itemCount: 1,
			itemDamage: 17,
			nbtData: Some(named(
				"",
				compound(vec![named("display", display), named("ench", ench)]),
			)),
		},
	)
}

#[test]
fn items_flatten_and_come_back() {
	let mut cx = Session::new();
	let items = Client12::window_items(a::play_toClient_packet_window_items::Ty {
		windowId: 0,
		items: vec![
			sword(),
			a::slot::Ty::_1,
			a::slot::Ty::default(
				35,
				a::slot__default::Ty {
					itemCount: 5,
					itemDamage: 14,
					nbtData: None,
				},
			),
		],
	});
	let out = to_1_13_2(items, &mut cx);
	let [Client132::window_items(w)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let names = registry::names(Version::V1_13_2, "items").unwrap();
	let [d::slot::Ty::r#true(blade), d::slot::Ty::r#false, d::slot::Ty::r#true(wool)] =
		w.items.as_slice()
	else {
		panic!("{w:?}")
	};
	assert_eq!(names.name(blade.itemId as u32), Some("diamond_sword"));
	assert_eq!(
		(names.name(wool.itemId as u32), wool.itemCount),
		(Some("red_wool"), 5)
	);
	let tag = &blade.nbtData.as_ref().unwrap().value;
	assert_eq!(get(tag, "Damage"), Some(&Payload::int(17)));
	let Some(Payload::list(List {
		items: ListItems::compound(ench),
		..
	})) = get(tag, "Enchantments")
	else {
		panic!("{tag:?}")
	};
	assert_eq!(
		get(&Payload::compound(ench[0].clone()), "id"),
		Some(&Payload::string("minecraft:sharpness".to_owned()))
	);
	let name = get(get(tag, "display").unwrap(), "Name").unwrap();
	assert_eq!(
		name,
		&Payload::string("{\"text\":\"Blade\",\"color\":\"red\",\"italic\":false}".to_owned())
	);

	let click = d::play_toServer_packet::Ty::set_creative_slot(
		d::play_toServer_packet_set_creative_slot::Ty {
			slot: 36,
			item: w.items[0].clone(),
		},
	);
	let back = convert(
		Root::PlayToServer,
		Version::V1_13_2.index(),
		Version::V1_12_2.index(),
		&encode(&click, d::play_toServer_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let [bytes] = back.as_slice() else {
		panic!("{back:?}")
	};
	let a::play_toServer_packet::Ty::set_creative_slot(s) =
		decode(bytes, a::play_toServer_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(s.item, sword());
}

#[test]
fn mobs_get_new_ids_and_metadata() {
	let mut cx = Session::new();
	let meta = vec![
		a::entityMetadata__items::Ty {
			key: 0,
			r#type: 0,
			value: a::entityMetadataItem::Ty::v0(0x10 | 0x02),
		},
		a::entityMetadata__items::Ty {
			key: 2,
			r#type: 3,
			value: a::entityMetadataItem::Ty::v3("\u{a7}aBob".to_owned()),
		},
		a::entityMetadata__items::Ty {
			key: 12,
			r#type: 6,
			value: a::entityMetadataItem::Ty::v6(true),
		},
		a::entityMetadata__items::Ty {
			key: 16,
			r#type: 1,
			value: a::entityMetadataItem::Ty::v1(3),
		},
	];
	let spawn = Client12::spawn_entity_living(a::play_toClient_packet_spawn_entity_living::Ty {
		entityId: 7,
		entityUUID: 1,
		r#type: 27,
		x: 0.0,
		y: 64.0,
		z: 0.0,
		yaw: 0,
		pitch: 0,
		headPitch: 0,
		velocity: a::vec3i16::Ty { x: 0, y: 0, z: 0 },
		metadata: meta,
	});
	let out = to_1_13(spawn, &mut cx);
	let [Client13::spawn_entity_living(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		registry::names(Version::V1_13, "entities")
			.unwrap()
			.name(s.r#type as u32),
		Some("zombie_villager")
	);
	use b::entityMetadataItem::Ty as M;
	let got: Vec<(u8, i32, M)> = s
		.metadata
		.iter()
		.map(|e| (e.key, e.r#type, e.value.clone()))
		.collect();
	assert_eq!(
		got,
		vec![
			(0, 0, M::v0(0x02)),
			(
				2,
				5,
				M::v5(Some("{\"text\":\"Bob\",\"color\":\"green\"}".to_owned()))
			),
			(12, 7, M::v7(true)),
			(17, 1, M::v1(3)),
		]
	);
	let update = Client12::entity_metadata(a::play_toClient_packet_entity_metadata::Ty {
		entityId: 7,
		metadata: vec![a::entityMetadata__items::Ty {
			key: 15,
			r#type: 6,
			value: a::entityMetadataItem::Ty::v6(true),
		}],
	});
	let out = to_1_13(update, &mut cx);
	let [Client13::entity_metadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(m.metadata[0].key, 16);
}

#[test]
fn falling_blocks_carry_flattened_states() {
	let spawn = Client12::spawn_entity(a::play_toClient_packet_spawn_entity::Ty {
		entityId: 3,
		objectUUID: 9,
		r#type: 70,
		x: 0.0,
		y: 0.0,
		z: 0.0,
		pitch: 0,
		yaw: 0,
		objectData: 12 | 1 << 12,
		velocity: a::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let out = to_1_13_2(spawn, &mut Session::new());
	let [Client132::spawn_entity(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		describe(Version::V1_13_2, s.objectData as u32),
		"minecraft:red_sand"
	);
}

#[test]
fn login_sends_commands_and_fluid_tags() {
	let login = Client12::login(a::play_toClient_packet_login::Ty {
		entityId: 1,
		gameMode: 0,
		dimension: 0,
		difficulty: 1,
		maxPlayers: 20,
		levelType: "flat".to_owned(),
		reducedDebugInfo: false,
	});
	let out = to_1_13_2(login, &mut Session::new());
	let [Client132::login(_), Client132::declare_commands(c), Client132::tags(t)] = out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!(c.nodes.len(), 2);
	let water = t
		.fluidTags
		.iter()
		.find(|t| t.tagName == "minecraft:water")
		.unwrap();
	assert_eq!(water.entries, vec![1, 2]);
	let blocks = registry::blocks(Version::V1_13_2).unwrap();
	let ice = t
		.blockTags
		.iter()
		.find(|t| t.tagName == "minecraft:ice")
		.unwrap();
	let names: Vec<&str> = ice
		.entries
		.iter()
		.map(|i| blocks.list[*i as usize].name.as_str())
		.collect();
	assert_eq!(names, ["ice", "packed_ice", "frosted_ice", "blue_ice"]);
}

#[test]
fn tab_completion_round_trip() {
	let mut cx = Session::new();
	let ask = d::play_toServer_packet::Ty::tab_complete(d::play_toServer_packet_tab_complete::Ty {
		transactionId: 42,
		text: "/gamemode cr".to_owned(),
	});
	let down = convert(
		Root::PlayToServer,
		Version::V1_13_2.index(),
		Version::V1_12_2.index(),
		&encode(&ask, d::play_toServer_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let a::play_toServer_packet::Ty::tab_complete(t) =
		decode(&down[0], a::play_toServer_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(t.text, "/gamemode cr");
	let out = to_1_13_2(Client12::tab_complete(vec!["creative".to_owned()]), &mut cx);
	let [Client132::tab_complete(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((t.transactionId, t.start, t.length), (42, 10, 2));
	assert_eq!(t.matches[0].r#match, "creative");
}

#[test]
fn plugin_channels_are_renamed_and_books_become_payloads() {
	let mut cx = Session::new();
	let brand = Client12::custom_payload(a::play_toClient_packet_custom_payload::Ty {
		channel: "MC|Brand".to_owned(),
		data: b"\x07vanilla".to_vec(),
	});
	let out = to_1_13_2(brand, &mut cx);
	let [Client132::custom_payload(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(p.channel, "minecraft:brand");
	let book = d::play_toServer_packet::Ty::edit_book(d::play_toServer_packet_edit_book::Ty {
		new_book: d::slot::Ty::r#false,
		signing: true,
		hand: 0,
	});
	let down = convert(
		Root::PlayToServer,
		Version::V1_13_2.index(),
		Version::V1_12_2.index(),
		&encode(&book, d::play_toServer_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let a::play_toServer_packet::Ty::custom_payload(p) =
		decode(&down[0], a::play_toServer_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(
		(p.channel.as_str(), p.data.as_slice()),
		("MC|BSign", &[0xff, 0xff][..])
	);
	let offhand = d::play_toServer_packet::Ty::edit_book(d::play_toServer_packet_edit_book::Ty {
		new_book: d::slot::Ty::r#false,
		signing: false,
		hand: 1,
	});
	let down = convert(
		Root::PlayToServer,
		Version::V1_13_2.index(),
		Version::V1_12_2.index(),
		&encode(&offhand, d::play_toServer_packet::serialize),
		&mut cx,
	)
	.unwrap();
	assert!(down.is_empty());
}

#[test]
fn status_favicons_lose_line_breaks() {
	let json = "{\"description\":\"a\\nb\",\"favicon\":\"data:image/png;base64,AAAA\\nBBBB\\n\"}"
		.to_owned();
	let status = a::status_toClient_packet::Ty::server_info(json);
	let out = convert(
		Root::StatusToClient,
		Version::V1_12_2.index(),
		Version::V1_13_2.index(),
		&encode(&status, a::status_toClient_packet::serialize),
		&mut Session::new(),
	)
	.unwrap();
	let d::status_toClient_packet::Ty::server_info(s) =
		decode(&out[0], d::status_toClient_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(
		s,
		"{\"description\":\"a\\nb\",\"favicon\":\"data:image/png;base64,AAAABBBB\"}"
	);
}

#[test]
fn teams_and_objectives_become_components() {
	let mut cx = Session::new();
	let team = Client12::teams(a::play_toClient_packet_teams::Ty {
		team: "red".to_owned(),
		mode: a::play_toClient_packet_teams__mode::Ty::v0(
			a::play_toClient_packet_teams__mode__v0::Ty {
				name: "Red".to_owned(),
				prefix: "\u{a7}c".to_owned(),
				suffix: String::new(),
				friendlyFire: 1,
				nameTagVisibility: "always".to_owned(),
				collisionRule: "always".to_owned(),
				color: 12,
				players: vec!["Steve".to_owned(), "\u{a7}a\u{a7}r".to_owned()],
			},
		),
	});
	let out = to_1_13_2(team, &mut cx);
	let [Client132::teams(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let d::play_toClient_packet_teams__mode::Ty::v0(m) = &t.mode else {
		panic!("{t:?}")
	};
	assert_eq!((m.name.as_str(), m.formatting), ("{\"text\":\"Red\"}", 12));
	assert_eq!(m.suffix, "{\"text\":\"\",\"color\":\"red\"}");
	assert_eq!(m.players, ["Steve", "\u{a7}w\u{a7}/"]);
	let objective =
		Client12::scoreboard_objective(a::play_toClient_packet_scoreboard_objective::Ty {
			name: "hp".to_owned(),
			action: a::play_toClient_packet_scoreboard_objective__action::Ty::v0(
				a::play_toClient_packet_scoreboard_objective__action__v0::Ty {
					displayText: "Health".to_owned(),
					r#type: "hearts".to_owned(),
				},
			),
		});
	let out = to_1_13_2(objective, &mut cx);
	let [Client132::scoreboard_objective(o)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let d::play_toClient_packet_scoreboard_objective__action::Ty::v0(v) = &o.action else {
		panic!("{o:?}")
	};
	assert_eq!(
		(v.displayText.as_str(), v.r#type),
		("{\"text\":\"Health\"}", 1)
	);
}

#[test]
fn block_particles_use_states() {
	let p = Client12::world_particles(a::play_toClient_packet_world_particles::Ty {
		longDistance: false,
		x: 0.0,
		y: 0.0,
		z: 0.0,
		offsetX: 0.0,
		offsetY: 0.0,
		offsetZ: 0.0,
		particleData: 0.0,
		particles: 4,
		data: a::play_toClient_packet_world_particles__data::Ty::v37(vec![1 | 3 << 12]),
	});
	let out = to_1_13_2(p, &mut Session::new());
	let [Client132::world_particles(w)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let d::particleData::Ty::v3(state) = w.data else {
		panic!("{w:?}")
	};
	assert_eq!(
		(
			w.particleId,
			describe(Version::V1_13_2, state as u32).as_str()
		),
		(3, "minecraft:diorite")
	);
}

fn to_1_12_2(p: Client132, cx: &mut Session) -> Vec<Client12> {
	let out = convert(
		Root::PlayToClient,
		Version::V1_13_2.index(),
		Version::V1_12_2.index(),
		&encode(&p, d::play_toClient_packet::serialize),
		cx,
	)
	.unwrap();
	out.iter()
		.map(|b| decode(b, a::play_toClient_packet::deserialize))
		.collect()
}

fn from_1_12_2(
	p: a::play_toServer_packet::Ty,
	cx: &mut Session,
) -> Vec<d::play_toServer_packet::Ty> {
	let out = convert(
		Root::PlayToServer,
		Version::V1_12_2.index(),
		Version::V1_13_2.index(),
		&encode(&p, a::play_toServer_packet::serialize),
		cx,
	)
	.unwrap();
	out.iter()
		.map(|b| decode(b, d::play_toServer_packet::deserialize))
		.collect()
}

fn state(text: &str) -> i32 {
	registry::blocks(Version::V1_13_2)
		.unwrap()
		.parse_state(text)
		.unwrap() as i32
}

fn spot() -> d::position::Ty {
	d::position::Ty { x: 5, y: 70, z: -3 }
}

fn place(text: &str, cx: &mut Session) -> Vec<Client12> {
	to_1_12_2(
		Client132::block_change(d::play_toClient_packet_block_change::Ty {
			location: spot(),
			r#type: state(text),
		}),
		cx,
	)
}

fn block_entity(action: u8, entries: Vec<Entry>) -> Client132 {
	Client132::tile_entity_data(d::play_toClient_packet_tile_entity_data::Ty {
		location: spot(),
		action,
		nbtData: Some(named("", compound(entries))),
	})
}

#[test]
fn flower_pots_get_block_entities_on_1_12_2() {
	let mut cx = Session::new();
	let out = place("minecraft:potted_blue_orchid", &mut cx);
	let [Client12::block_change(clear), Client12::block_change(pot), Client12::tile_entity_data(te)] =
		out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!((clear.r#type, pot.r#type >> 4, te.action), (0, 140, 5));
	let tag = &te.nbtData.as_ref().unwrap().value;
	assert_eq!(
		get(tag, "Item"),
		Some(&Payload::string("minecraft:red_flower".to_owned()))
	);
	assert_eq!(get(tag, "Data"), Some(&Payload::int(1)));
	assert_eq!(get(tag, "x"), Some(&Payload::int(5)));
}

#[test]
fn note_blocks_and_block_entities_use_stored_states() {
	let mut cx = Session::new();
	place(
		"minecraft:note_block[instrument=bell,note=7,powered=false]",
		&mut cx,
	);
	let note = registry::blocks(Version::V1_13_2)
		.unwrap()
		.block("note_block")
		.unwrap()
		.id as i32;
	let action = Client132::block_action(d::play_toClient_packet_block_action::Ty {
		location: spot(),
		byte1: 0,
		byte2: 0,
		blockId: note,
	});
	let out = to_1_12_2(action, &mut cx);
	let [Client12::block_action(b)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((b.byte1, b.byte2, b.blockId), (6, 7, 25));

	place(
		"minecraft:red_bed[facing=north,occupied=false,part=head]",
		&mut cx,
	);
	let out = to_1_12_2(
		block_entity(
			11,
			vec![named("id", Payload::string("minecraft:bed".to_owned()))],
		),
		&mut cx,
	);
	let [Client12::tile_entity_data(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		get(&t.nbtData.as_ref().unwrap().value, "color"),
		Some(&Payload::int(14))
	);

	place("minecraft:lime_banner[rotation=0]", &mut cx);
	let patterns = Payload::list(List::new(ListItems::compound(vec![vec![
		named("Pattern", Payload::string("bo".to_owned())),
		named("Color", Payload::int(0)),
	]])));
	let banner = block_entity(
		6,
		vec![
			named("id", Payload::string("minecraft:banner".to_owned())),
			named("Patterns", patterns),
			named(
				"CustomName",
				Payload::string("{\"text\":\"Flag\",\"color\":\"green\"}".to_owned()),
			),
		],
	);
	let out = to_1_12_2(banner, &mut cx);
	let [Client12::tile_entity_data(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let tag = &t.nbtData.as_ref().unwrap().value;
	assert_eq!(get(tag, "Base"), Some(&Payload::int(10)));
	assert_eq!(
		get(tag, "CustomName"),
		Some(&Payload::string("\u{a7}aFlag".to_owned()))
	);
	let Some(Payload::list(List {
		items: ListItems::compound(p),
		..
	})) = get(tag, "Patterns")
	else {
		panic!("{tag:?}")
	};
	assert_eq!(
		get(&Payload::compound(p[0].clone()), "Color"),
		Some(&Payload::int(15))
	);

	place("minecraft:creeper_head[rotation=9]", &mut cx);
	let out = to_1_12_2(
		block_entity(
			4,
			vec![named("id", Payload::string("minecraft:skull".to_owned()))],
		),
		&mut cx,
	);
	let [Client12::tile_entity_data(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let tag = &t.nbtData.as_ref().unwrap().value;
	assert_eq!(
		(get(tag, "SkullType"), get(tag, "Rot")),
		(Some(&Payload::byte(4)), Some(&Payload::byte(9)))
	);

	let spawner = block_entity(
		1,
		vec![
			named("id", Payload::string("minecraft:mob_spawner".to_owned())),
			named(
				"SpawnData",
				compound(vec![named(
					"id",
					Payload::string("minecraft:iron_golem".to_owned()),
				)]),
			),
		],
	);
	let out = to_1_12_2(spawner, &mut cx);
	let [Client12::tile_entity_data(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let spawn = get(&t.nbtData.as_ref().unwrap().value, "SpawnData").unwrap();
	assert_eq!(
		get(spawn, "id"),
		Some(&Payload::string("minecraft:villager_golem".to_owned()))
	);
	assert!(to_1_12_2(
		block_entity(
			5,
			vec![named("id", Payload::string("minecraft:conduit".to_owned()))]
		),
		&mut cx
	)
	.is_empty());
}

#[test]
fn items_reach_1_12_2_clients() {
	let names = registry::names(Version::V1_13_2, "items").unwrap();
	let enchantments = Payload::list(List::new(ListItems::compound(vec![vec![
		named("id", Payload::string("minecraft:sharpness".to_owned())),
		named("lvl", Payload::short(3)),
	]])));
	let tag = compound(vec![
		named("Damage", Payload::int(17)),
		named("Enchantments", enchantments),
		named(
			"display",
			compound(vec![named(
				"Name",
				Payload::string("{\"text\":\"Blade\",\"color\":\"red\"}".to_owned()),
			)]),
		),
	]);
	let item = |name: &str, count: i8, tag: Option<Entry>| {
		d::slot::Ty::r#true(d::slot__true::Ty {
			itemId: names.id(name).unwrap() as i32,
			itemCount: count,
			nbtData: tag,
		})
	};
	let items = Client132::window_items(d::play_toClient_packet_window_items::Ty {
		windowId: 0,
		items: vec![
			item("diamond_sword", 1, Some(named("", tag))),
			item("red_wool", 5, None),
			d::slot::Ty::r#false,
		],
	});
	let out = to_1_12_2(items, &mut Session::new());
	let [Client12::window_items(w)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let [a::slot::Ty::default(276, sword), a::slot::Ty::default(35, wool), a::slot::Ty::_1] =
		w.items.as_slice()
	else {
		panic!("{w:?}")
	};
	assert_eq!(
		(sword.itemDamage, wool.itemDamage, wool.itemCount),
		(17, 14, 5)
	);
	let tag = &sword.nbtData.as_ref().unwrap().value;
	assert_eq!(
		get(get(tag, "display").unwrap(), "Name"),
		Some(&Payload::string("\u{a7}cBlade".to_owned()))
	);
	let Some(Payload::list(List {
		items: ListItems::compound(ench),
		..
	})) = get(tag, "ench")
	else {
		panic!("{tag:?}")
	};
	assert_eq!(
		get(&Payload::compound(ench[0].clone()), "id"),
		Some(&Payload::short(16))
	);
}

fn particle(id: i32, data: d::particleData::Ty) -> Client132 {
	Client132::world_particles(d::play_toClient_packet_world_particles::Ty {
		particleId: id,
		longDistance: false,
		x: 1.0,
		y: 2.0,
		z: 3.0,
		offsetX: 0.5,
		offsetY: 0.5,
		offsetZ: 0.5,
		particleData: 0.1,
		particles: 8,
		data,
	})
}

#[test]
fn particles_reach_1_12_2_clients() {
	use a::play_toClient_packet_world_particles__data::Ty as Old;
	let mut cx = Session::new();
	let dust = d::particleData::Ty::v11(d::particleData__v11::Ty {
		red: 0.0,
		green: 0.5,
		blue: 1.0,
		scale: 2.0,
	});
	let out = to_1_12_2(particle(11, dust), &mut cx);
	let [Client12::world_particles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(p.data, Old::default(30));
	assert_eq!(
		(p.offsetX, p.offsetY, p.offsetZ, p.particleData, p.particles),
		(f32::MIN_POSITIVE, 0.5, 1.0, 2.0, 0)
	);
	assert!(to_1_12_2(particle(3, d::particleData::Ty::v3(0)), &mut cx).is_empty());
	let out = to_1_12_2(
		particle(20, d::particleData::Ty::v20(state("minecraft:sand"))),
		&mut cx,
	);
	let [Client12::world_particles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(p.data, Old::v46(vec![12]));
	let sword = registry::names(Version::V1_13_2, "items")
		.unwrap()
		.id("diamond_sword")
		.unwrap() as i32;
	let item = d::slot::Ty::r#true(d::slot__true::Ty {
		itemId: sword,
		itemCount: 1,
		nbtData: None,
	});
	let out = to_1_12_2(particle(27, d::particleData::Ty::v27(item)), &mut cx);
	let [Client12::world_particles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(p.data, Old::v36(vec![276, 0]));
	let out = to_1_12_2(particle(39, d::particleData::Ty::default(39)), &mut cx);
	let [Client12::world_particles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(p.data, Old::default(12));
}

#[test]
fn sounds_fall_back_for_1_12_2_clients() {
	let mut cx = Session::new();
	let sound = |name: &str| {
		Client132::named_sound_effect(d::play_toClient_packet_named_sound_effect::Ty {
			soundName: name.to_owned(),
			soundCategory: 0,
			x: 0,
			y: 0,
			z: 0,
			volume: 1.0,
			pitch: 1.0,
		})
	};
	for (new, old) in [
		("minecraft:block.note_block.harp", "block.note.harp"),
		("entity.dolphin.ambient", "entity.bat.ambient"),
	] {
		let out = to_1_12_2(sound(new), &mut cx);
		let [Client12::named_sound_effect(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(s.soundName, old);
	}
	let stop = Client132::stop_sound(d::play_toClient_packet_stop_sound::Ty::v2(
		"block.note_block.harp".to_owned(),
	));
	let out = to_1_12_2(stop, &mut cx);
	let [Client12::custom_payload(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(p.channel.as_str(), p.data.as_slice()),
		("MC|StopSound", &b"\x00\x0fblock.note.harp"[..])
	);
}

fn node(
	kind: u8,
	children: Vec<i32>,
	extra: d::command_node__extraNodeData::Ty,
) -> d::command_node::Ty {
	d::command_node::Ty {
		flags: d::command_node__flags::Ty {
			unused: 0,
			has_custom_suggestions: 0,
			has_redirect_node: 0,
			has_command: u8::from(kind != 0),
			command_node_type: kind,
		},
		children,
		redirectNode: d::command_node__redirectNode::Ty::default(0),
		extraNodeData: extra,
	}
}

fn literal(name: &str) -> d::command_node::Ty {
	node(
		1,
		vec![],
		d::command_node__extraNodeData::Ty::v1(name.to_owned()),
	)
}

fn argument(
	name: &str,
	parser: &str,
	properties: d::command_node__extraNodeData__v2__properties::Ty,
) -> d::command_node::Ty {
	node(
		2,
		vec![],
		d::command_node__extraNodeData::Ty::v2(d::command_node__extraNodeData__v2::Ty {
			name: name.to_owned(),
			parser: parser.to_owned(),
			properties,
			suggestionType: d::command_node__extraNodeData__v2__suggestionType::Ty::default(0),
		}),
	)
}

fn root(children: Vec<i32>) -> d::command_node::Ty {
	node(0, children, d::command_node__extraNodeData::Ty::v0)
}

#[test]
fn legacy_clients_complete_commands_and_names() {
	let mut cx = Session::new();
	let commands = Client132::declare_commands(d::play_toClient_packet_declare_commands::Ty {
		nodes: vec![root(vec![1, 2]), literal("gamemode"), literal("give")],
		rootIndex: 0,
	});
	assert!(to_1_12_2(commands, &mut cx).is_empty());
	let players = Client132::player_info(d::play_toClient_packet_player_info::Ty {
		action: d::play_toClient_packet_player_info__action::Ty::add_player,
		data: vec![d::play_toClient_packet_player_info__data__items::Ty {
			uuid: 5,
			ANON: d::play_toClient_packet_player_info__data__items__ANON::Ty::add_player(
				d::play_toClient_packet_player_info__data__items__ANON__add_player::Ty {
					name: "Notch".to_owned(),
					properties: vec![],
					gamemode: 0,
					ping: 0,
					displayName: None,
				},
			),
		}],
	});
	assert_eq!(to_1_12_2(players, &mut cx).len(), 1);
	let ask = |text: &str| {
		a::play_toServer_packet::Ty::tab_complete(a::play_toServer_packet_tab_complete::Ty {
			text: text.to_owned(),
			assumeCommand: false,
			lookedAtBlock: None,
		})
	};
	for (text, expected) in [("/g", vec!["/gamemode", "/give"]), ("hi no", vec!["Notch"])] {
		assert!(from_1_12_2(ask(text), &mut cx).is_empty());
		let sent = cx.take_outgoing();
		let [answer] = sent.as_slice() else {
			panic!("{sent:?}")
		};
		assert_eq!(
			(answer.root, answer.version),
			(Root::PlayToClient, Version::V1_12_2.index())
		);
		assert_eq!(
			decode(&answer.data, a::play_toClient_packet::deserialize),
			Client12::tab_complete(expected.iter().map(|s| s.to_string()).collect())
		);
	}
	let out = from_1_12_2(ask("/gamemode cr"), &mut cx);
	let [d::play_toServer_packet::Ty::tab_complete(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(t.text, "/gamemode cr");
	let reply = |id: i32| {
		Client132::tab_complete(d::play_toClient_packet_tab_complete::Ty {
			transactionId: id,
			start: 10,
			length: 2,
			matches: vec![d::play_toClient_packet_tab_complete__matches__items::Ty {
				r#match: "creative".to_owned(),
				tooltip: None,
			}],
		})
	};
	assert!(to_1_12_2(reply(t.transactionId + 1), &mut cx).is_empty());
	assert_eq!(
		to_1_12_2(reply(t.transactionId), &mut cx),
		vec![Client12::tab_complete(vec!["creative".to_owned()])]
	);
}

#[test]
fn legacy_payloads_become_1_13_packets() {
	let mut cx = Session::new();
	let payload = |channel: &str, data: Vec<u8>| {
		a::play_toServer_packet::Ty::custom_payload(a::play_toServer_packet_custom_payload::Ty {
			channel: channel.to_owned(),
			data,
		})
	};
	let out = from_1_12_2(payload("MC|BSign", vec![0xff, 0xff]), &mut cx);
	let [d::play_toServer_packet::Ty::edit_book(e)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(&e.new_book, e.signing, e.hand),
		(&d::slot::Ty::r#false, true, 0)
	);
	assert_eq!(
		from_1_12_2(payload("MC|ItemName", b"\x05Sword".to_vec()), &mut cx),
		vec![d::play_toServer_packet::Ty::name_item("Sword".to_owned())]
	);
	assert_eq!(
		from_1_12_2(payload("MC|TrSel", vec![0, 0, 0, 3]), &mut cx),
		vec![d::play_toServer_packet::Ty::select_trade(3)]
	);
	let mut auto = Vec::new();
	for v in [5i32, 70, -3] {
		auto.extend(v.to_be_bytes());
	}
	auto.extend(b"\x06say hi\x01\x04AUTO\x00\x01");
	let out = from_1_12_2(payload("MC|AutoCmd", auto), &mut cx);
	let [d::play_toServer_packet::Ty::update_command_block(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(&c.location, c.command.as_str(), c.mode, c.flags),
		(&spot(), "say hi", 1, 5)
	);
	let out = from_1_12_2(
		payload("REGISTER", b"MC|Brand\0BungeeCord\0Bad Name".to_vec()),
		&mut cx,
	);
	let [d::play_toServer_packet::Ty::custom_payload(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(p.channel.as_str(), p.data.as_slice()),
		(
			"minecraft:register",
			&b"minecraft:brand\0bungeecord:main"[..]
		)
	);
	assert!(from_1_12_2(payload("Bad Name", vec![]), &mut cx).is_empty());
}

#[test]
fn login_plugin_requests_are_answered_for_legacy_clients() {
	let mut cx = Session::new();
	let request = d::login_toClient_packet::Ty::login_plugin_request(
		d::login_toClient_packet_login_plugin_request::Ty {
			messageId: 7,
			channel: "velocity:player_info".to_owned(),
			data: vec![1],
		},
	);
	let out = convert(
		Root::LoginToClient,
		Version::V1_13_2.index(),
		Version::V1_12_2.index(),
		&encode(&request, d::login_toClient_packet::serialize),
		&mut cx,
	)
	.unwrap();
	assert!(out.is_empty());
	let sent = cx.take_outgoing();
	let [answer] = sent.as_slice() else {
		panic!("{sent:?}")
	};
	assert_eq!(
		(answer.root, answer.version),
		(Root::LoginToServer, Version::V1_13.index())
	);
	let b::login_toServer_packet::Ty::login_plugin_response(r) =
		decode(&answer.data, b::login_toServer_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!((r.messageId, r.data), (7, None));
}

#[test]
fn face_player_turns_into_a_relative_teleport() {
	let mut cx = Session::new();
	let moved = a::play_toServer_packet::Ty::position(a::play_toServer_packet_position::Ty {
		x: 10.0,
		y: 64.0,
		z: 10.0,
		onGround: true,
	});
	assert_eq!(from_1_12_2(moved, &mut cx).len(), 1);
	let face = Client132::face_player(d::play_toClient_packet_face_player::Ty {
		feet_eyes: 0,
		x: 0.0,
		y: 64.0,
		z: 10.0,
		isEntity: d::play_toClient_packet_face_player__isEntity::Ty::default(false),
	});
	let out = to_1_12_2(face, &mut cx);
	let [Client12::position(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(p.x, p.y, p.z, p.yaw, p.pitch, p.flags, p.teleportId),
		(0.0, 0.0, 0.0, 90.0, 0.0, 7, -1)
	);
}

#[test]
fn teams_are_truncated_for_1_12_2_clients() {
	let team = Client132::teams(d::play_toClient_packet_teams::Ty {
		team: "staff".to_owned(),
		mode: d::play_toClient_packet_teams__mode::Ty::v0(
			d::play_toClient_packet_teams__mode__v0::Ty {
				name: "{\"text\":\"A very long team display name here\"}".to_owned(),
				friendlyFire: 0,
				nameTagVisibility: "always".to_owned(),
				collisionRule: "always".to_owned(),
				formatting: 12,
				prefix: "{\"text\":\"[Admin] \",\"color\":\"gold\"}".to_owned(),
				suffix: "{\"text\":\"\"}".to_owned(),
				players: vec!["Steve".to_owned()],
			},
		),
	});
	let out = to_1_12_2(team, &mut Session::new());
	let [Client12::teams(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let a::play_toClient_packet_teams__mode::Ty::v0(m) = &t.mode else {
		panic!("{t:?}")
	};
	assert_eq!(m.name, "A very long team display name he");
	assert_eq!(
		(m.prefix.as_str(), m.suffix.as_str(), m.color),
		("\u{a7}6[Admin] \u{a7}c", "", 12)
	);
}

#[test]
fn new_entities_fall_back_for_1_12_2_clients() {
	let mut cx = Session::new();
	let ids = registry::names(Version::V1_13_2, "entities").unwrap();
	let spawn = Client132::spawn_entity_living(d::play_toClient_packet_spawn_entity_living::Ty {
		entityId: 9,
		entityUUID: 2,
		r#type: ids.id("drowned").unwrap() as i32,
		x: 0.0,
		y: 64.0,
		z: 0.0,
		yaw: 0,
		pitch: 0,
		headPitch: 0,
		velocity: d::vec3i16::Ty { x: 0, y: 0, z: 0 },
		metadata: vec![
			d::entityMetadata__items::Ty {
				key: 12,
				r#type: 7,
				value: d::entityMetadataItem::Ty::v7(true),
			},
			d::entityMetadata__items::Ty {
				key: 15,
				r#type: 7,
				value: d::entityMetadataItem::Ty::v7(true),
			},
		],
	});
	let out = to_1_12_2(spawn, &mut cx);
	let [Client12::spawn_entity_living(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.r#type, 27);
	let keys: Vec<u8> = s.metadata.iter().map(|e| e.key).collect();
	assert_eq!(keys, [12]);
	let trident = Client132::spawn_entity(d::play_toClient_packet_spawn_entity::Ty {
		entityId: 10,
		objectUUID: 3,
		r#type: 94,
		x: 0.0,
		y: 0.0,
		z: 0.0,
		pitch: 0,
		yaw: 0,
		objectData: 0,
		velocity: d::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let out = to_1_12_2(trident, &mut cx);
	let [Client12::spawn_entity(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.r#type, 60);
	let icon = |r#type: i32| d::play_toClient_packet_map__icons__items::Ty {
		r#type,
		x: 1,
		z: 2,
		direction: 3,
		displayName: None,
	};
	let map = Client132::map(d::play_toClient_packet_map::Ty {
		itemDamage: 1,
		scale: 0,
		trackingPosition: true,
		icons: vec![icon(1), icon(12)],
		columns: d::play_toClient_packet_map__columns::Ty::v0,
	});
	let out = to_1_12_2(map, &mut cx);
	let [Client12::map(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let icons: Vec<i8> = m.icons.iter().map(|i| i.directionAndType).collect();
	assert_eq!(icons, [1 << 4 | 3]);
}

#[test]
fn step_1_13_1_down_rewrites_commands_titles_and_effects() {
	let mut cx = Session::new();
	let down = |p: Client132, cx: &mut Session| -> Vec<Client13> {
		let out = convert(
			Root::PlayToClient,
			Version::V1_13_2.index(),
			Version::V1_13.index(),
			&encode(&p, d::play_toClient_packet::serialize),
			cx,
		)
		.unwrap();
		out.iter()
			.map(|b| decode(b, b::play_toClient_packet::deserialize))
			.collect()
	};
	use d::command_node__extraNodeData__v2__properties::Ty as P;
	let commands = Client132::declare_commands(d::play_toClient_packet_declare_commands::Ty {
		nodes: vec![
			root(vec![1, 2]),
			argument("pos", "minecraft:column_pos", P::minecraft_column_pos),
			argument("dim", "minecraft:dimension", P::minecraft_dimension),
		],
		rootIndex: 0,
	});
	let out = down(commands, &mut cx);
	let [Client13::declare_commands(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let parsers: Vec<(String, b::command_node__extraNodeData__v2__properties::Ty)> = c.nodes[1..]
		.iter()
		.map(|n| match &n.extraNodeData {
			b::command_node__extraNodeData::Ty::v2(v) => (v.parser.clone(), v.properties.clone()),
			e => panic!("{e:?}"),
		})
		.collect();
	use b::command_node__extraNodeData__v2__properties::Ty as Q;
	assert_eq!(
		parsers,
		[
			("minecraft:vec2".to_owned(), Q::minecraft_vec2),
			("brigadier:string".to_owned(), Q::brigadier_string(b::command_node__extraNodeData__v2__properties__brigadier_string::Ty::SINGLE_WORD)),
		]
	);
	let window = |title: &str| {
		Client132::open_window(d::play_toClient_packet_open_window::Ty {
			windowId: 1,
			inventoryType: "minecraft:chest".to_owned(),
			windowTitle: title.to_owned(),
			slotCount: 27,
			entityId: d::play_toClient_packet_open_window__entityId::Ty::default(
				"minecraft:chest".to_owned(),
			),
		})
	};
	let titles: Vec<String> = [
		"{\"text\":\"Loot\",\"color\":\"red\"}",
		"{\"translate\":\"container.chest\"}",
	]
	.into_iter()
	.map(|t| match down(window(t), &mut cx).as_slice() {
		[Client13::open_window(w)] => w.windowTitle.clone(),
		out => panic!("{out:?}"),
	})
	.collect();
	assert_eq!(
		titles,
		[
			"{\"text\":\"\u{a7}cLoot\"}",
			"{\"translate\":\"container.chest\"}"
		]
	);
	let boss = Client132::boss_bar(d::play_toClient_packet_boss_bar::Ty {
		entityUUID: 1,
		action: d::play_toClient_packet_boss_bar__action::Ty::v0(
			d::play_toClient_packet_boss_bar__action__v0::Ty {
				title: "{\"text\":\"Boss\"}".to_owned(),
				health: 1.0,
				color: 0,
				dividers: 0,
				flags: 4,
			},
		),
	});
	let out = down(boss, &mut cx);
	let [Client13::boss_bar(bar)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let b::play_toClient_packet_boss_bar__action::Ty::v0(v) = &bar.action else {
		panic!("{bar:?}")
	};
	assert_eq!(v.flags, 6);
	let smoke = Client132::world_event(d::play_toClient_packet_world_event::Ty {
		effectId: 2000,
		location: spot(),
		data: 0,
		global: false,
	});
	let out = down(smoke, &mut cx);
	let [Client13::world_event(e)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((e.location.y, e.data), (69, 4));
}
