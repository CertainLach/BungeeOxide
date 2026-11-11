use minecraft_versions::gen::convert::{convert, Root, Version};
use minecraft_versions::gen::convert::{v1_12_2 as a, v1_13 as b, v1_13_2 as d};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::registry;
use minecraft_versions::Session;
use nbt::{Entry, List, ListItems, Payload};

type Client12 = a::play_to_client_packet::Ty;
type Client13 = b::play_to_client_packet::Ty;
type Client132 = d::play_to_client_packet::Ty;

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
		Version::V1_12_2,
		Version::V1_13,
		&encode(&p, a::play_to_client_packet::serialize),
		cx,
	)
	.unwrap();
	out.iter()
		.map(|b| decode(b, b::play_to_client_packet::deserialize))
		.collect()
}

fn to_1_13_2(p: Client12, cx: &mut Session) -> Vec<Client132> {
	let out = convert(
		Root::PlayToClient,
		Version::V1_12_2,
		Version::V1_13_2,
		&encode(&p, a::play_to_client_packet::serialize),
		cx,
	)
	.unwrap();
	out.iter()
		.map(|b| decode(b, d::play_to_client_packet::deserialize))
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
	Client12::MapChunk(a::play_to_client_packet_map_chunk::Ty {
		x: 2,
		z: -3,
		ground_up: true,
		bit_map: 0b1001,
		chunk_data: data,
		block_entities: vec![],
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
	let [Client13::MapChunk(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((c.x, c.z, c.bit_map), (2, -3, 0b1001));
	let flat = read_flat(&c.chunk_data, 2);
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
	let [Client132::MapChunk(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let flat = read_flat(&c.chunk_data, 2);
	assert_eq!(
		describe(Version::V1_13_2, flat.sections[0][index(4, 5, 6)]),
		"minecraft:tnt[unstable=false]"
	);
	assert_eq!(
		describe(Version::V1_13_2, flat.sections[1][0]),
		"minecraft:stone"
	);
	let change = Client12::BlockChange(a::play_to_client_packet_block_change::Ty {
		location: a::position::Ty { x: 1, y: 64, z: -1 },
		r#type: 64 << 4 | 2,
	});
	let out = to_1_13_2(change, &mut cx);
	let [Client132::BlockChange(c)] = out.as_slice() else {
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
	Client12::MapChunk(a::play_to_client_packet_map_chunk::Ty {
		x,
		z,
		ground_up: true,
		bit_map: 1,
		chunk_data: data,
		block_entities: vec![],
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
	let [Client13::MapChunk(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let flat = read_flat(&c.chunk_data, 1);
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
	let [Client13::MapChunk(_), Client13::MultiBlockChange(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((m.chunk_x, m.chunk_z, m.records.len()), (0, 0, 1));
	assert_eq!(
		describe(Version::V1_13, m.records[0].block_id as u32),
		"minecraft:oak_fence[east=true,north=false,south=false,waterlogged=false,west=false]"
	);
	let change = Client12::BlockChange(a::play_to_client_packet_block_change::Ty {
		location: a::position::Ty { x: 5, y: 4, z: 6 },
		r#type: fence as i32,
	});
	let out = to_1_13(change, &mut cx);
	let [Client13::BlockChange(placed), Client13::MultiBlockChange(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		describe(Version::V1_13, placed.r#type as u32),
		"minecraft:oak_fence[east=false,north=true,south=false,waterlogged=false,west=false]"
	);
	assert_eq!(
		describe(Version::V1_13, m.records[0].block_id as u32),
		"minecraft:oak_fence[east=true,north=true,south=true,waterlogged=false,west=false]"
	);
}

fn compound(entries: Vec<Entry>) -> Payload {
	Payload::Compound(entries)
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
	let te = Client12::TileEntityData(a::play_to_client_packet_tile_entity_data::Ty {
		location: a::position::Ty {
			x: 33,
			y: 6,
			z: -47,
		},
		action: 4,
		nbt_data: Some(named(
			"",
			compound(vec![
				named("id", Payload::String("Skull".to_owned())),
				named("SkullType", Payload::Byte(3)),
				named("Rot", Payload::Byte(4)),
			]),
		)),
	});
	let out = to_1_13(te, &mut cx);
	let [Client13::BlockChange(c), Client13::TileEntityData(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		describe(Version::V1_13, c.r#type as u32),
		"minecraft:player_head[rotation=4]"
	);
	assert_eq!(t.action, 4);
}

fn sword() -> a::slot::Ty {
	let ench = Payload::List(List::new(ListItems::Compound(vec![vec![
		named("id", Payload::Short(16)),
		named("lvl", Payload::Short(3)),
	]])));
	let display = compound(vec![named(
		"Name",
		Payload::String("\u{a7}cBlade".to_owned()),
	)]);
	a::slot::Ty::Default(
		276,
		a::slot_default::Ty {
			item_count: 1,
			item_damage: 17,
			nbt_data: Some(named(
				"",
				compound(vec![named("display", display), named("ench", ench)]),
			)),
		},
	)
}

#[test]
fn items_flatten_and_come_back() {
	let mut cx = Session::new();
	let items = Client12::WindowItems(a::play_to_client_packet_window_items::Ty {
		window_id: 0,
		items: vec![
			sword(),
			a::slot::Ty::_1,
			a::slot::Ty::Default(
				35,
				a::slot_default::Ty {
					item_count: 5,
					item_damage: 14,
					nbt_data: None,
				},
			),
		],
	});
	let out = to_1_13_2(items, &mut cx);
	let [Client132::WindowItems(w)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let names = registry::names(Version::V1_13_2, "items").unwrap();
	let [d::slot::Ty::True(blade), d::slot::Ty::False, d::slot::Ty::True(wool)] =
		w.items.as_slice()
	else {
		panic!("{w:?}")
	};
	assert_eq!(names.name(blade.item_id as u32), Some("diamond_sword"));
	assert_eq!(
		(names.name(wool.item_id as u32), wool.item_count),
		(Some("red_wool"), 5)
	);
	let tag = &blade.nbt_data.as_ref().unwrap().value;
	assert_eq!(get(tag, "Damage"), Some(&Payload::Int(17)));
	let Some(Payload::List(List {
		items: ListItems::Compound(ench),
		..
	})) = get(tag, "Enchantments")
	else {
		panic!("{tag:?}")
	};
	assert_eq!(
		get(&Payload::Compound(ench[0].clone()), "id"),
		Some(&Payload::String("minecraft:sharpness".to_owned()))
	);
	let name = get(get(tag, "display").unwrap(), "Name").unwrap();
	assert_eq!(
		name,
		&Payload::String("{\"text\":\"Blade\",\"color\":\"red\",\"italic\":false}".to_owned())
	);

	let click = d::play_to_server_packet::Ty::SetCreativeSlot(
		d::play_to_server_packet_set_creative_slot::Ty {
			slot: 36,
			item: w.items[0].clone(),
		},
	);
	let back = convert(
		Root::PlayToServer,
		Version::V1_13_2,
		Version::V1_12_2,
		&encode(&click, d::play_to_server_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let [bytes] = back.as_slice() else {
		panic!("{back:?}")
	};
	let a::play_to_server_packet::Ty::SetCreativeSlot(s) =
		decode(bytes, a::play_to_server_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(s.item, sword());
}

#[test]
fn mobs_get_new_ids_and_metadata() {
	let mut cx = Session::new();
	let meta = vec![
		a::entity_metadata_items::Ty {
			key: 0,
			r#type: 0,
			value: a::entity_metadata_item::Ty::V0(0x10 | 0x02),
		},
		a::entity_metadata_items::Ty {
			key: 2,
			r#type: 3,
			value: a::entity_metadata_item::Ty::V3("\u{a7}aBob".to_owned()),
		},
		a::entity_metadata_items::Ty {
			key: 12,
			r#type: 6,
			value: a::entity_metadata_item::Ty::V6(true),
		},
		a::entity_metadata_items::Ty {
			key: 16,
			r#type: 1,
			value: a::entity_metadata_item::Ty::V1(3),
		},
	];
	let spawn = Client12::SpawnEntityLiving(a::play_to_client_packet_spawn_entity_living::Ty {
		entity_id: 7,
		entity_uuid: 1,
		r#type: 27,
		x: 0.0,
		y: 64.0,
		z: 0.0,
		yaw: 0,
		pitch: 0,
		head_pitch: 0,
		velocity: a::vec3i16::Ty { x: 0, y: 0, z: 0 },
		metadata: meta,
	});
	let out = to_1_13(spawn, &mut cx);
	let [Client13::SpawnEntityLiving(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		registry::names(Version::V1_13, "entities")
			.unwrap()
			.name(s.r#type as u32),
		Some("zombie_villager")
	);
	use b::entity_metadata_item::Ty as M;
	let got: Vec<(u8, i32, M)> = s
		.metadata
		.iter()
		.map(|e| (e.key, e.r#type, e.value.clone()))
		.collect();
	assert_eq!(
		got,
		vec![
			(0, 0, M::V0(0x02)),
			(
				2,
				5,
				M::V5(Some("{\"text\":\"Bob\",\"color\":\"green\"}".to_owned()))
			),
			(12, 7, M::V7(true)),
			(17, 1, M::V1(3)),
		]
	);
	let update = Client12::EntityMetadata(a::play_to_client_packet_entity_metadata::Ty {
		entity_id: 7,
		metadata: vec![a::entity_metadata_items::Ty {
			key: 15,
			r#type: 6,
			value: a::entity_metadata_item::Ty::V6(true),
		}],
	});
	let out = to_1_13(update, &mut cx);
	let [Client13::EntityMetadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(m.metadata[0].key, 16);
}

#[test]
fn falling_blocks_carry_flattened_states() {
	let spawn = Client12::SpawnEntity(a::play_to_client_packet_spawn_entity::Ty {
		entity_id: 3,
		object_uuid: 9,
		r#type: 70,
		x: 0.0,
		y: 0.0,
		z: 0.0,
		pitch: 0,
		yaw: 0,
		object_data: 12 | 1 << 12,
		velocity: a::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let out = to_1_13_2(spawn, &mut Session::new());
	let [Client132::SpawnEntity(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		describe(Version::V1_13_2, s.object_data as u32),
		"minecraft:red_sand"
	);
}

#[test]
fn login_sends_commands_and_fluid_tags() {
	let login = Client12::Login(a::play_to_client_packet_login::Ty {
		entity_id: 1,
		game_mode: 0,
		dimension: 0,
		difficulty: 1,
		max_players: 20,
		level_type: "flat".to_owned(),
		reduced_debug_info: false,
	});
	let out = to_1_13_2(login, &mut Session::new());
	let [Client132::Login(_), Client132::DeclareCommands(c), Client132::Tags(t)] = out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!(c.nodes.len(), 2);
	let water = t
		.fluid_tags
		.iter()
		.find(|t| t.tag_name == "minecraft:water")
		.unwrap();
	assert_eq!(water.entries, vec![1, 2]);
	let blocks = registry::blocks(Version::V1_13_2).unwrap();
	let ice = t
		.block_tags
		.iter()
		.find(|t| t.tag_name == "minecraft:ice")
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
	let ask =
		d::play_to_server_packet::Ty::TabComplete(d::play_to_server_packet_tab_complete::Ty {
			transaction_id: 42,
			text: "/gamemode cr".to_owned(),
		});
	let down = convert(
		Root::PlayToServer,
		Version::V1_13_2,
		Version::V1_12_2,
		&encode(&ask, d::play_to_server_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let a::play_to_server_packet::Ty::TabComplete(t) =
		decode(&down[0], a::play_to_server_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(t.text, "/gamemode cr");
	let out = to_1_13_2(Client12::TabComplete(vec!["creative".to_owned()]), &mut cx);
	let [Client132::TabComplete(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((t.transaction_id, t.start, t.length), (42, 10, 2));
	assert_eq!(t.matches[0].r#match, "creative");
}

#[test]
fn plugin_channels_are_renamed_and_books_become_payloads() {
	let mut cx = Session::new();
	let brand = Client12::CustomPayload(a::play_to_client_packet_custom_payload::Ty {
		channel: "MC|Brand".to_owned(),
		data: b"\x07vanilla".to_vec(),
	});
	let out = to_1_13_2(brand, &mut cx);
	let [Client132::CustomPayload(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(p.channel, "minecraft:brand");
	let book = d::play_to_server_packet::Ty::EditBook(d::play_to_server_packet_edit_book::Ty {
		new_book: d::slot::Ty::False,
		signing: true,
		hand: 0,
	});
	let down = convert(
		Root::PlayToServer,
		Version::V1_13_2,
		Version::V1_12_2,
		&encode(&book, d::play_to_server_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let a::play_to_server_packet::Ty::CustomPayload(p) =
		decode(&down[0], a::play_to_server_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(
		(p.channel.as_str(), p.data.as_slice()),
		("MC|BSign", &[0xff, 0xff][..])
	);
	let offhand = d::play_to_server_packet::Ty::EditBook(d::play_to_server_packet_edit_book::Ty {
		new_book: d::slot::Ty::False,
		signing: false,
		hand: 1,
	});
	let down = convert(
		Root::PlayToServer,
		Version::V1_13_2,
		Version::V1_12_2,
		&encode(&offhand, d::play_to_server_packet::serialize),
		&mut cx,
	)
	.unwrap();
	assert!(down.is_empty());
}

#[test]
fn status_favicons_lose_line_breaks() {
	let json = "{\"description\":\"a\\nb\",\"favicon\":\"data:image/png;base64,AAAA\\nBBBB\\n\"}"
		.to_owned();
	let status = a::status_to_client_packet::Ty::ServerInfo(json);
	let out = convert(
		Root::StatusToClient,
		Version::V1_12_2,
		Version::V1_13_2,
		&encode(&status, a::status_to_client_packet::serialize),
		&mut Session::new(),
	)
	.unwrap();
	let d::status_to_client_packet::Ty::ServerInfo(s) =
		decode(&out[0], d::status_to_client_packet::deserialize)
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
	let team = Client12::Teams(a::play_to_client_packet_teams::Ty {
		team: "red".to_owned(),
		mode: a::play_to_client_packet_teams_mode::Ty::V0(
			a::play_to_client_packet_teams_mode_v0::Ty {
				name: "Red".to_owned(),
				prefix: "\u{a7}c".to_owned(),
				suffix: String::new(),
				friendly_fire: 1,
				name_tag_visibility: "always".to_owned(),
				collision_rule: "always".to_owned(),
				color: 12,
				players: vec!["Steve".to_owned(), "\u{a7}a\u{a7}r".to_owned()],
			},
		),
	});
	let out = to_1_13_2(team, &mut cx);
	let [Client132::Teams(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let d::play_to_client_packet_teams_mode::Ty::V0(m) = &t.mode else {
		panic!("{t:?}")
	};
	assert_eq!((m.name.as_str(), m.formatting), ("{\"text\":\"Red\"}", 12));
	assert_eq!(m.suffix, "{\"text\":\"\",\"color\":\"red\"}");
	assert_eq!(m.players, ["Steve", "\u{a7}w\u{a7}/"]);
	let objective =
		Client12::ScoreboardObjective(a::play_to_client_packet_scoreboard_objective::Ty {
			name: "hp".to_owned(),
			action: a::play_to_client_packet_scoreboard_objective_action::Ty::V0(
				a::play_to_client_packet_scoreboard_objective_action_v0::Ty {
					display_text: "Health".to_owned(),
					r#type: "hearts".to_owned(),
				},
			),
		});
	let out = to_1_13_2(objective, &mut cx);
	let [Client132::ScoreboardObjective(o)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let d::play_to_client_packet_scoreboard_objective_action::Ty::V0(v) = &o.action else {
		panic!("{o:?}")
	};
	assert_eq!(
		(v.display_text.as_str(), v.r#type),
		("{\"text\":\"Health\"}", 1)
	);
}

#[test]
fn block_particles_use_states() {
	let p = Client12::WorldParticles(a::play_to_client_packet_world_particles::Ty {
		long_distance: false,
		x: 0.0,
		y: 0.0,
		z: 0.0,
		offset_x: 0.0,
		offset_y: 0.0,
		offset_z: 0.0,
		particle_data: 0.0,
		particles: 4,
		data: a::play_to_client_packet_world_particles_data::Ty::V37(vec![1 | 3 << 12]),
	});
	let out = to_1_13_2(p, &mut Session::new());
	let [Client132::WorldParticles(w)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let d::particle_data::Ty::V3(state) = w.data else {
		panic!("{w:?}")
	};
	assert_eq!(
		(
			w.particle_id,
			describe(Version::V1_13_2, state as u32).as_str()
		),
		(3, "minecraft:diorite")
	);
}

fn to_1_12_2(p: Client132, cx: &mut Session) -> Vec<Client12> {
	let out = convert(
		Root::PlayToClient,
		Version::V1_13_2,
		Version::V1_12_2,
		&encode(&p, d::play_to_client_packet::serialize),
		cx,
	)
	.unwrap();
	out.iter()
		.map(|b| decode(b, a::play_to_client_packet::deserialize))
		.collect()
}

fn from_1_12_2(
	p: a::play_to_server_packet::Ty,
	cx: &mut Session,
) -> Vec<d::play_to_server_packet::Ty> {
	let out = convert(
		Root::PlayToServer,
		Version::V1_12_2,
		Version::V1_13_2,
		&encode(&p, a::play_to_server_packet::serialize),
		cx,
	)
	.unwrap();
	out.iter()
		.map(|b| decode(b, d::play_to_server_packet::deserialize))
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
		Client132::BlockChange(d::play_to_client_packet_block_change::Ty {
			location: spot(),
			r#type: state(text),
		}),
		cx,
	)
}

fn block_entity(action: u8, entries: Vec<Entry>) -> Client132 {
	Client132::TileEntityData(d::play_to_client_packet_tile_entity_data::Ty {
		location: spot(),
		action,
		nbt_data: Some(named("", compound(entries))),
	})
}

#[test]
fn flower_pots_get_block_entities_on_1_12_2() {
	let mut cx = Session::new();
	let out = place("minecraft:potted_blue_orchid", &mut cx);
	let [Client12::BlockChange(clear), Client12::BlockChange(pot), Client12::TileEntityData(te)] =
		out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!((clear.r#type, pot.r#type >> 4, te.action), (0, 140, 5));
	let tag = &te.nbt_data.as_ref().unwrap().value;
	assert_eq!(
		get(tag, "Item"),
		Some(&Payload::String("minecraft:red_flower".to_owned()))
	);
	assert_eq!(get(tag, "Data"), Some(&Payload::Int(1)));
	assert_eq!(get(tag, "x"), Some(&Payload::Int(5)));
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
	let action = Client132::BlockAction(d::play_to_client_packet_block_action::Ty {
		location: spot(),
		byte1: 0,
		byte2: 0,
		block_id: note,
	});
	let out = to_1_12_2(action, &mut cx);
	let [Client12::BlockAction(b)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((b.byte1, b.byte2, b.block_id), (6, 7, 25));

	place(
		"minecraft:red_bed[facing=north,occupied=false,part=head]",
		&mut cx,
	);
	let out = to_1_12_2(
		block_entity(
			11,
			vec![named("id", Payload::String("minecraft:bed".to_owned()))],
		),
		&mut cx,
	);
	let [Client12::TileEntityData(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		get(&t.nbt_data.as_ref().unwrap().value, "color"),
		Some(&Payload::Int(14))
	);

	place("minecraft:lime_banner[rotation=0]", &mut cx);
	let patterns = Payload::List(List::new(ListItems::Compound(vec![vec![
		named("Pattern", Payload::String("bo".to_owned())),
		named("Color", Payload::Int(0)),
	]])));
	let banner = block_entity(
		6,
		vec![
			named("id", Payload::String("minecraft:banner".to_owned())),
			named("Patterns", patterns),
			named(
				"CustomName",
				Payload::String("{\"text\":\"Flag\",\"color\":\"green\"}".to_owned()),
			),
		],
	);
	let out = to_1_12_2(banner, &mut cx);
	let [Client12::TileEntityData(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let tag = &t.nbt_data.as_ref().unwrap().value;
	assert_eq!(get(tag, "Base"), Some(&Payload::Int(10)));
	assert_eq!(
		get(tag, "CustomName"),
		Some(&Payload::String("\u{a7}aFlag".to_owned()))
	);
	let Some(Payload::List(List {
		items: ListItems::Compound(p),
		..
	})) = get(tag, "Patterns")
	else {
		panic!("{tag:?}")
	};
	assert_eq!(
		get(&Payload::Compound(p[0].clone()), "Color"),
		Some(&Payload::Int(15))
	);

	place("minecraft:creeper_head[rotation=9]", &mut cx);
	let out = to_1_12_2(
		block_entity(
			4,
			vec![named("id", Payload::String("minecraft:skull".to_owned()))],
		),
		&mut cx,
	);
	let [Client12::TileEntityData(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let tag = &t.nbt_data.as_ref().unwrap().value;
	assert_eq!(
		(get(tag, "SkullType"), get(tag, "Rot")),
		(Some(&Payload::Byte(4)), Some(&Payload::Byte(9)))
	);

	let spawner = block_entity(
		1,
		vec![
			named("id", Payload::String("minecraft:mob_spawner".to_owned())),
			named(
				"SpawnData",
				compound(vec![named(
					"id",
					Payload::String("minecraft:iron_golem".to_owned()),
				)]),
			),
		],
	);
	let out = to_1_12_2(spawner, &mut cx);
	let [Client12::TileEntityData(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let spawn = get(&t.nbt_data.as_ref().unwrap().value, "SpawnData").unwrap();
	assert_eq!(
		get(spawn, "id"),
		Some(&Payload::String("minecraft:villager_golem".to_owned()))
	);
	assert!(to_1_12_2(
		block_entity(
			5,
			vec![named("id", Payload::String("minecraft:conduit".to_owned()))]
		),
		&mut cx
	)
	.is_empty());
}

#[test]
fn items_reach_1_12_2_clients() {
	let names = registry::names(Version::V1_13_2, "items").unwrap();
	let enchantments = Payload::List(List::new(ListItems::Compound(vec![vec![
		named("id", Payload::String("minecraft:sharpness".to_owned())),
		named("lvl", Payload::Short(3)),
	]])));
	let tag = compound(vec![
		named("Damage", Payload::Int(17)),
		named("Enchantments", enchantments),
		named(
			"display",
			compound(vec![named(
				"Name",
				Payload::String("{\"text\":\"Blade\",\"color\":\"red\"}".to_owned()),
			)]),
		),
	]);
	let item = |name: &str, count: i8, tag: Option<Entry>| {
		d::slot::Ty::True(d::slot_true::Ty {
			item_id: names.id(name).unwrap() as i32,
			item_count: count,
			nbt_data: tag,
		})
	};
	let items = Client132::WindowItems(d::play_to_client_packet_window_items::Ty {
		window_id: 0,
		items: vec![
			item("diamond_sword", 1, Some(named("", tag))),
			item("red_wool", 5, None),
			d::slot::Ty::False,
		],
	});
	let out = to_1_12_2(items, &mut Session::new());
	let [Client12::WindowItems(w)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let [a::slot::Ty::Default(276, sword), a::slot::Ty::Default(35, wool), a::slot::Ty::_1] =
		w.items.as_slice()
	else {
		panic!("{w:?}")
	};
	assert_eq!(
		(sword.item_damage, wool.item_damage, wool.item_count),
		(17, 14, 5)
	);
	let tag = &sword.nbt_data.as_ref().unwrap().value;
	assert_eq!(
		get(get(tag, "display").unwrap(), "Name"),
		Some(&Payload::String("\u{a7}cBlade".to_owned()))
	);
	let Some(Payload::List(List {
		items: ListItems::Compound(ench),
		..
	})) = get(tag, "ench")
	else {
		panic!("{tag:?}")
	};
	assert_eq!(
		get(&Payload::Compound(ench[0].clone()), "id"),
		Some(&Payload::Short(16))
	);
}

fn particle(id: i32, data: d::particle_data::Ty) -> Client132 {
	Client132::WorldParticles(d::play_to_client_packet_world_particles::Ty {
		particle_id: id,
		long_distance: false,
		x: 1.0,
		y: 2.0,
		z: 3.0,
		offset_x: 0.5,
		offset_y: 0.5,
		offset_z: 0.5,
		particle_data: 0.1,
		particles: 8,
		data,
	})
}

#[test]
fn particles_reach_1_12_2_clients() {
	use a::play_to_client_packet_world_particles_data::Ty as Old;
	let mut cx = Session::new();
	let dust = d::particle_data::Ty::V11(d::particle_data_v11::Ty {
		red: 0.0,
		green: 0.5,
		blue: 1.0,
		scale: 2.0,
	});
	let out = to_1_12_2(particle(11, dust), &mut cx);
	let [Client12::WorldParticles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(p.data, Old::Default(30));
	assert_eq!(
		(
			p.offset_x,
			p.offset_y,
			p.offset_z,
			p.particle_data,
			p.particles
		),
		(f32::MIN_POSITIVE, 0.5, 1.0, 2.0, 0)
	);
	assert!(to_1_12_2(particle(3, d::particle_data::Ty::V3(0)), &mut cx).is_empty());
	let out = to_1_12_2(
		particle(20, d::particle_data::Ty::V20(state("minecraft:sand"))),
		&mut cx,
	);
	let [Client12::WorldParticles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(p.data, Old::V46(vec![12]));
	let sword = registry::names(Version::V1_13_2, "items")
		.unwrap()
		.id("diamond_sword")
		.unwrap() as i32;
	let item = d::slot::Ty::True(d::slot_true::Ty {
		item_id: sword,
		item_count: 1,
		nbt_data: None,
	});
	let out = to_1_12_2(particle(27, d::particle_data::Ty::V27(item)), &mut cx);
	let [Client12::WorldParticles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(p.data, Old::V36(vec![276, 0]));
	let out = to_1_12_2(particle(39, d::particle_data::Ty::Default(39)), &mut cx);
	let [Client12::WorldParticles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(p.data, Old::Default(12));
}

#[test]
fn sounds_fall_back_for_1_12_2_clients() {
	let mut cx = Session::new();
	let sound = |name: &str| {
		Client132::NamedSoundEffect(d::play_to_client_packet_named_sound_effect::Ty {
			sound_name: name.to_owned(),
			sound_category: 0,
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
		let [Client12::NamedSoundEffect(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(s.sound_name, old);
	}
	let stop = Client132::StopSound(d::play_to_client_packet_stop_sound::Ty::V2(
		"block.note_block.harp".to_owned(),
	));
	let out = to_1_12_2(stop, &mut cx);
	let [Client12::CustomPayload(p)] = out.as_slice() else {
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
	extra: d::command_node_extra_node_data::Ty,
) -> d::command_node::Ty {
	d::command_node::Ty {
		flags: d::command_node_flags::Ty {
			unused: 0,
			has_custom_suggestions: 0,
			has_redirect_node: 0,
			has_command: u8::from(kind != 0),
			command_node_type: kind,
		},
		children,
		redirect_node: d::command_node_redirect_node::Ty::Default(0),
		extra_node_data: extra,
	}
}

fn literal(name: &str) -> d::command_node::Ty {
	node(
		1,
		vec![],
		d::command_node_extra_node_data::Ty::V1(name.to_owned()),
	)
}

fn argument(
	name: &str,
	parser: &str,
	properties: d::command_node_extra_node_data_v2_properties::Ty,
) -> d::command_node::Ty {
	node(
		2,
		vec![],
		d::command_node_extra_node_data::Ty::V2(d::command_node_extra_node_data_v2::Ty {
			name: name.to_owned(),
			parser: parser.to_owned(),
			properties,
			suggestion_type: d::command_node_extra_node_data_v2_suggestion_type::Ty::Default(0),
		}),
	)
}

fn root(children: Vec<i32>) -> d::command_node::Ty {
	node(0, children, d::command_node_extra_node_data::Ty::V0)
}

#[test]
fn legacy_clients_complete_commands_and_names() {
	let mut cx = Session::new();
	let commands = Client132::DeclareCommands(d::play_to_client_packet_declare_commands::Ty {
		nodes: vec![root(vec![1, 2]), literal("gamemode"), literal("give")],
		root_index: 0,
	});
	assert!(to_1_12_2(commands, &mut cx).is_empty());
	let players = Client132::PlayerInfo(d::play_to_client_packet_player_info::Ty {
		action: d::play_to_client_packet_player_info_action::Ty::AddPlayer,
		data: vec![d::play_to_client_packet_player_info_data_items::Ty {
			uuid: 5,
			anon: d::play_to_client_packet_player_info_data_items_anon::Ty::AddPlayer(
				d::play_to_client_packet_player_info_data_items_anon_add_player::Ty {
					name: "Notch".to_owned(),
					properties: vec![],
					gamemode: 0,
					ping: 0,
					display_name: None,
				},
			),
		}],
	});
	assert_eq!(to_1_12_2(players, &mut cx).len(), 1);
	let ask = |text: &str| {
		a::play_to_server_packet::Ty::TabComplete(a::play_to_server_packet_tab_complete::Ty {
			text: text.to_owned(),
			assume_command: false,
			looked_at_block: None,
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
			(Root::PlayToClient, Version::V1_12_2)
		);
		assert_eq!(
			decode(&answer.data, a::play_to_client_packet::deserialize),
			Client12::TabComplete(expected.iter().map(|s| s.to_string()).collect())
		);
	}
	let out = from_1_12_2(ask("/gamemode cr"), &mut cx);
	let [d::play_to_server_packet::Ty::TabComplete(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(t.text, "/gamemode cr");
	let reply = |id: i32| {
		Client132::TabComplete(d::play_to_client_packet_tab_complete::Ty {
			transaction_id: id,
			start: 10,
			length: 2,
			matches: vec![d::play_to_client_packet_tab_complete_matches_items::Ty {
				r#match: "creative".to_owned(),
				tooltip: None,
			}],
		})
	};
	assert!(to_1_12_2(reply(t.transaction_id + 1), &mut cx).is_empty());
	assert_eq!(
		to_1_12_2(reply(t.transaction_id), &mut cx),
		vec![Client12::TabComplete(vec!["creative".to_owned()])]
	);
}

#[test]
fn legacy_payloads_become_1_13_packets() {
	let mut cx = Session::new();
	let payload = |channel: &str, data: Vec<u8>| {
		a::play_to_server_packet::Ty::CustomPayload(a::play_to_server_packet_custom_payload::Ty {
			channel: channel.to_owned(),
			data,
		})
	};
	let out = from_1_12_2(payload("MC|BSign", vec![0xff, 0xff]), &mut cx);
	let [d::play_to_server_packet::Ty::EditBook(e)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(&e.new_book, e.signing, e.hand),
		(&d::slot::Ty::False, true, 0)
	);
	assert_eq!(
		from_1_12_2(payload("MC|ItemName", b"\x05Sword".to_vec()), &mut cx),
		vec![d::play_to_server_packet::Ty::NameItem("Sword".to_owned())]
	);
	assert_eq!(
		from_1_12_2(payload("MC|TrSel", vec![0, 0, 0, 3]), &mut cx),
		vec![d::play_to_server_packet::Ty::SelectTrade(3)]
	);
	let mut auto = Vec::new();
	for v in [5i32, 70, -3] {
		auto.extend(v.to_be_bytes());
	}
	auto.extend(b"\x06say hi\x01\x04AUTO\x00\x01");
	let out = from_1_12_2(payload("MC|AutoCmd", auto), &mut cx);
	let [d::play_to_server_packet::Ty::UpdateCommandBlock(c)] = out.as_slice() else {
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
	let [d::play_to_server_packet::Ty::CustomPayload(p)] = out.as_slice() else {
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
	let request = d::login_to_client_packet::Ty::LoginPluginRequest(
		d::login_to_client_packet_login_plugin_request::Ty {
			message_id: 7,
			channel: "velocity:player_info".to_owned(),
			data: vec![1],
		},
	);
	let out = convert(
		Root::LoginToClient,
		Version::V1_13_2,
		Version::V1_12_2,
		&encode(&request, d::login_to_client_packet::serialize),
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
		(Root::LoginToServer, Version::V1_13)
	);
	let b::login_to_server_packet::Ty::LoginPluginResponse(r) =
		decode(&answer.data, b::login_to_server_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!((r.message_id, r.data), (7, None));
}

#[test]
fn face_player_turns_into_a_relative_teleport() {
	let mut cx = Session::new();
	let moved = a::play_to_server_packet::Ty::Position(a::play_to_server_packet_position::Ty {
		x: 10.0,
		y: 64.0,
		z: 10.0,
		on_ground: true,
	});
	assert_eq!(from_1_12_2(moved, &mut cx).len(), 1);
	let face = Client132::FacePlayer(d::play_to_client_packet_face_player::Ty {
		feet_eyes: 0,
		x: 0.0,
		y: 64.0,
		z: 10.0,
		is_entity: d::play_to_client_packet_face_player_is_entity::Ty::Default(false),
	});
	let out = to_1_12_2(face, &mut cx);
	let [Client12::Position(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(p.x, p.y, p.z, p.yaw, p.pitch, p.flags, p.teleport_id),
		(0.0, 0.0, 0.0, 90.0, 0.0, 7, -1)
	);
}

#[test]
fn teams_are_truncated_for_1_12_2_clients() {
	let team = Client132::Teams(d::play_to_client_packet_teams::Ty {
		team: "staff".to_owned(),
		mode: d::play_to_client_packet_teams_mode::Ty::V0(
			d::play_to_client_packet_teams_mode_v0::Ty {
				name: "{\"text\":\"A very long team display name here\"}".to_owned(),
				friendly_fire: 0,
				name_tag_visibility: "always".to_owned(),
				collision_rule: "always".to_owned(),
				formatting: 12,
				prefix: "{\"text\":\"[Admin] \",\"color\":\"gold\"}".to_owned(),
				suffix: "{\"text\":\"\"}".to_owned(),
				players: vec!["Steve".to_owned()],
			},
		),
	});
	let out = to_1_12_2(team, &mut Session::new());
	let [Client12::Teams(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let a::play_to_client_packet_teams_mode::Ty::V0(m) = &t.mode else {
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
	let spawn = Client132::SpawnEntityLiving(d::play_to_client_packet_spawn_entity_living::Ty {
		entity_id: 9,
		entity_uuid: 2,
		r#type: ids.id("drowned").unwrap() as i32,
		x: 0.0,
		y: 64.0,
		z: 0.0,
		yaw: 0,
		pitch: 0,
		head_pitch: 0,
		velocity: d::vec3i16::Ty { x: 0, y: 0, z: 0 },
		metadata: vec![
			d::entity_metadata_items::Ty {
				key: 12,
				r#type: 7,
				value: d::entity_metadata_item::Ty::V7(true),
			},
			d::entity_metadata_items::Ty {
				key: 15,
				r#type: 7,
				value: d::entity_metadata_item::Ty::V7(true),
			},
		],
	});
	let out = to_1_12_2(spawn, &mut cx);
	let [Client12::SpawnEntityLiving(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.r#type, 27);
	let keys: Vec<u8> = s.metadata.iter().map(|e| e.key).collect();
	assert_eq!(keys, [12]);
	let trident = Client132::SpawnEntity(d::play_to_client_packet_spawn_entity::Ty {
		entity_id: 10,
		object_uuid: 3,
		r#type: 94,
		x: 0.0,
		y: 0.0,
		z: 0.0,
		pitch: 0,
		yaw: 0,
		object_data: 0,
		velocity: d::vec3i16::Ty { x: 0, y: 0, z: 0 },
	});
	let out = to_1_12_2(trident, &mut cx);
	let [Client12::SpawnEntity(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.r#type, 60);
	let icon = |r#type: i32| d::play_to_client_packet_map_icons_items::Ty {
		r#type,
		x: 1,
		z: 2,
		direction: 3,
		display_name: None,
	};
	let map = Client132::Map(d::play_to_client_packet_map::Ty {
		item_damage: 1,
		scale: 0,
		tracking_position: true,
		icons: vec![icon(1), icon(12)],
		columns: d::play_to_client_packet_map_columns::Ty::V0,
	});
	let out = to_1_12_2(map, &mut cx);
	let [Client12::Map(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let icons: Vec<i8> = m.icons.iter().map(|i| i.direction_and_type).collect();
	assert_eq!(icons, [1 << 4 | 3]);
}

#[test]
fn step_1_13_1_down_rewrites_commands_titles_and_effects() {
	let mut cx = Session::new();
	let down = |p: Client132, cx: &mut Session| -> Vec<Client13> {
		let out = convert(
			Root::PlayToClient,
			Version::V1_13_2,
			Version::V1_13,
			&encode(&p, d::play_to_client_packet::serialize),
			cx,
		)
		.unwrap();
		out.iter()
			.map(|b| decode(b, b::play_to_client_packet::deserialize))
			.collect()
	};
	use d::command_node_extra_node_data_v2_properties::Ty as P;
	let commands = Client132::DeclareCommands(d::play_to_client_packet_declare_commands::Ty {
		nodes: vec![
			root(vec![1, 2]),
			argument("pos", "minecraft:column_pos", P::MinecraftColumnPos),
			argument("dim", "minecraft:dimension", P::MinecraftDimension),
		],
		root_index: 0,
	});
	let out = down(commands, &mut cx);
	let [Client13::DeclareCommands(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let parsers: Vec<(String, b::command_node_extra_node_data_v2_properties::Ty)> = c.nodes[1..]
		.iter()
		.map(|n| match &n.extra_node_data {
			b::command_node_extra_node_data::Ty::V2(v) => (v.parser.clone(), v.properties.clone()),
			e => panic!("{e:?}"),
		})
		.collect();
	use b::command_node_extra_node_data_v2_properties::Ty as Q;
	assert_eq!(
		parsers,
		[
			("minecraft:vec2".to_owned(), Q::MinecraftVec2),
			(
				"brigadier:string".to_owned(),
				Q::BrigadierString(
					b::command_node_extra_node_data_v2_properties_brigadier_string::Ty::SingleWord
				)
			),
		]
	);
	let window = |title: &str| {
		Client132::OpenWindow(d::play_to_client_packet_open_window::Ty {
			window_id: 1,
			inventory_type: "minecraft:chest".to_owned(),
			window_title: title.to_owned(),
			slot_count: 27,
			entity_id: d::play_to_client_packet_open_window_entity_id::Ty::Default(
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
		[Client13::OpenWindow(w)] => w.window_title.clone(),
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
	let boss = Client132::BossBar(d::play_to_client_packet_boss_bar::Ty {
		entity_uuid: 1,
		action: d::play_to_client_packet_boss_bar_action::Ty::V0(
			d::play_to_client_packet_boss_bar_action_v0::Ty {
				title: "{\"text\":\"Boss\"}".to_owned(),
				health: 1.0,
				color: 0,
				dividers: 0,
				flags: 4,
			},
		),
	});
	let out = down(boss, &mut cx);
	let [Client13::BossBar(bar)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let b::play_to_client_packet_boss_bar_action::Ty::V0(v) = &bar.action else {
		panic!("{bar:?}")
	};
	assert_eq!(v.flags, 6);
	let smoke = Client132::WorldEvent(d::play_to_client_packet_world_event::Ty {
		effect_id: 2000,
		location: spot(),
		data: 0,
		global: false,
	});
	let out = down(smoke, &mut cx);
	let [Client13::WorldEvent(e)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((e.location.y, e.data), (69, 4));
}
