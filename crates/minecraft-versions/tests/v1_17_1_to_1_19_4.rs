use minecraft_versions::gen::convert::{convert, index_of, v1_17_1, v1_18, Root};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::Session;
use nbt::{Entry, Payload};

fn at(version: &str) -> usize {
	index_of(version).unwrap()
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

fn longs(out: &mut Vec<u8>, values: &[u32], bits: u32) {
	let per = (64 / bits) as usize;
	let mut l = vec![0u64; values.len().div_ceil(per)];
	for (i, v) in values.iter().enumerate() {
		l[i / per] |= (*v as u64) << ((i % per) as u32 * bits);
	}
	varint(out, l.len() as i32);
	for x in l {
		out.extend_from_slice(&x.to_be_bytes());
	}
}

fn read_container(data: &[u8], at: &mut usize, size: usize, min: u32, max: u32) -> Vec<u32> {
	let bits = data[*at] as u32;
	*at += 1;
	let palette: Option<Vec<u32>> = match bits {
		0 => {
			let v = read_varint(data, at) as u32;
			assert_eq!(read_varint(data, at), 0);
			return vec![v; size];
		}
		b if b <= max => {
			let n = read_varint(data, at);
			Some((0..n).map(|_| read_varint(data, at) as u32).collect())
		}
		_ => None,
	};
	let bits = bits.max(min);
	let n = read_varint(data, at) as usize;
	let words: Vec<u64> = (0..n)
		.map(|i| u64::from_be_bytes(data[*at + i * 8..*at + i * 8 + 8].try_into().unwrap()))
		.collect();
	*at += n * 8;
	let per = (64 / bits) as usize;
	assert_eq!(n, size.div_ceil(per));
	(0..size)
		.map(|i| {
			let v = ((words[i / per] >> ((i % per) as u32 * bits)) & ((1 << bits) - 1)) as u32;
			palette.as_ref().map_or(v, |p| p[v as usize])
		})
		.collect()
}

fn compound(entries: Vec<Entry>) -> Entry {
	Entry::new("", entries)
}

fn named(name: &str, value: Payload) -> Entry {
	Entry::new(name, value)
}

fn encode<T>(v: &T, ser: fn(&T, &mut Output) -> minecraft_versions::pds::Result<()>) -> Vec<u8> {
	let mut o = Output::new();
	ser(v, &mut o).unwrap();
	o.into_inner()
}

fn decode<T>(b: &[u8], de: fn(&mut Input) -> minecraft_versions::pds::Result<T>) -> T {
	let mut i = Input::new(b);
	let v = de(&mut i).unwrap();
	assert_eq!(i.remaining(), 0);
	v
}

mod s1_18 {
	use super::*;
	use v1_17_1::play_toClient_packet as c17;
	use v1_18::play_toClient_packet as c18;

	fn up(p: &c17::Ty, cx: &mut Session) -> Vec<c18::Ty> {
		convert(
			Root::PlayToClient,
			at("1.17.1"),
			at("1.18"),
			&encode(p, c17::serialize),
			cx,
		)
		.unwrap()
		.iter()
		.map(|b| decode(b, c18::deserialize))
		.collect()
	}

	fn light(x: i32, z: i32) -> v1_17_1::play_toClient_packet_update_light::Ty {
		v1_17_1::play_toClient_packet_update_light::Ty {
			chunkX: x,
			chunkZ: z,
			trustEdges: true,
			skyLightMask: vec![0b110],
			blockLightMask: vec![0b10],
			emptySkyLightMask: vec![0b1],
			emptyBlockLightMask: vec![0b101],
			skyLight: vec![vec![0xff; 2048], vec![0xee; 2048]],
			blockLight: vec![vec![0x11; 2048]],
		}
	}

	fn chunk(x: i32, z: i32) -> v1_17_1::play_toClient_packet_map_chunk::Ty {
		let stone = 1u32;
		let mut data = Vec::new();
		data.extend_from_slice(&4096i16.to_be_bytes());
		data.push(4);
		varint(&mut data, 2);
		varint(&mut data, 0);
		varint(&mut data, stone as i32);
		let blocks: Vec<u32> = (0..4096).map(|i| (i % 3 == 0) as u32).collect();
		longs(&mut data, &blocks, 4);
		v1_17_1::play_toClient_packet_map_chunk::Ty {
			x,
			z,
			bitMap: vec![0b10],
			heightmaps: compound(vec![]),
			biomes: (0..1024)
				.map(|i| if i < 128 { 1 } else { 2 + (i % 3) })
				.collect(),
			chunkData: data,
			blockEntities: vec![compound(vec![
				named("id", Payload::string("minecraft:chest".into())),
				named("x", Payload::int(x * 16 + 3)),
				named("y", Payload::int(20)),
				named("z", Payload::int(z * 16 + 5)),
			])],
		}
	}

	#[test]
	fn chunk_merges_buffered_light() {
		let mut cx = Session::new();
		assert!(up(&c17::Ty::update_light(light(4, -2)), &mut cx).is_empty());
		let out = up(&c17::Ty::map_chunk(chunk(4, -2)), &mut cx);
		let [c18::Ty::map_chunk(c)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!((c.x, c.z, c.trustEdges), (4, -2, true));
		assert_eq!(c.skyLightMask, vec![0b110]);
		assert_eq!(c.emptyBlockLightMask, vec![0b101]);
		assert_eq!(c.skyLight.len(), 2);
		assert_eq!(c.blockLight, vec![vec![0x11; 2048]]);
		let be = &c.blockEntities[0];
		assert_eq!((be.ANON.x, be.ANON.z, be.y, be.r#type), (3, 5, 20, 1));
		let d = &c.chunkData;
		let mut at = 0;
		for s in 0..16 {
			let count = i16::from_be_bytes([d[at], d[at + 1]]);
			at += 2;
			let blocks = read_container(d, &mut at, 4096, 4, 8);
			let biomes = read_container(d, &mut at, 64, 1, 3);
			if s == 1 {
				assert_eq!(count, 4096);
				assert!(blocks
					.iter()
					.enumerate()
					.all(|(i, b)| *b == (i % 3 == 0) as u32));
			} else {
				assert_eq!(count, 0);
				assert!(blocks.iter().all(|b| *b == 0));
			}
			let expected: Vec<u32> = (0..64)
				.map(|i| s * 64 + i)
				.map(|i| if i < 128 { 1 } else { 2 + (i % 3) as u32 })
				.collect();
			assert_eq!(biomes, expected, "section {s}");
		}
		assert_eq!(at, d.len());
		let later = up(&c17::Ty::update_light(light(4, -2)), &mut cx);
		assert!(matches!(later.as_slice(), [c18::Ty::update_light(_)]));
	}

	#[test]
	fn chunk_without_light_is_dark_until_updated() {
		let mut cx = Session::new();
		let out = up(&c17::Ty::map_chunk(chunk(0, 0)), &mut cx);
		let [c18::Ty::map_chunk(c)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(c.emptySkyLightMask, vec![(1 << 18) - 1]);
		assert!(c.skyLight.is_empty());
		assert!(matches!(
			up(&c17::Ty::update_light(light(0, 0)), &mut cx).as_slice(),
			[c18::Ty::update_light(_)]
		));
		up(
			&c17::Ty::unload_chunk(v1_17_1::play_toClient_packet_unload_chunk::Ty {
				chunkX: 0,
				chunkZ: 0,
			}),
			&mut cx,
		);
		assert!(up(&c17::Ty::update_light(light(0, 0)), &mut cx).is_empty());
	}

	#[test]
	fn chunk_round_trips_down() {
		let mut cx = Session::new();
		up(&c17::Ty::update_light(light(1, 1)), &mut cx);
		let out = up(&c17::Ty::map_chunk(chunk(1, 1)), &mut cx);
		let bytes = encode(&out[0], c18::serialize);
		let out: Vec<c17::Ty> = convert(
			Root::PlayToClient,
			at("1.18"),
			at("1.17.1"),
			&bytes,
			&mut cx,
		)
		.unwrap()
		.iter()
		.map(|b| decode(b, c17::deserialize))
		.collect();
		let [c17::Ty::update_light(l), c17::Ty::map_chunk(m)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(l, &light(1, 1));
		let original = chunk(1, 1);
		assert_eq!(
			(m.bitMap.clone(), m.biomes.clone()),
			(original.bitMap, original.biomes)
		);
		let Payload::compound(be) = &m.blockEntities[0].value else {
			panic!()
		};
		assert!(be
			.iter()
			.any(|e| e.name == "x" && e.value == Payload::int(19)));
		assert!(be
			.iter()
			.any(|e| e.name == "id" && e.value == Payload::string("minecraft:chest".into())));
	}
}

mod chain {
	use super::*;
	use minecraft_versions::gen::convert::{v1_19_2, v1_19_4, Version};
	use minecraft_versions::registry;
	use nbt::{List, ListItems};
	use v1_17_1::play_toClient_packet as old;
	use v1_17_1::play_toServer_packet as old_server;
	use v1_19_4::play_toClient_packet as new;
	use v1_19_4::play_toServer_packet as new_server;

	fn up(p: &old::Ty, cx: &mut Session) -> Vec<new::Ty> {
		convert(
			Root::PlayToClient,
			at("1.17.1"),
			at("1.19.4"),
			&encode(p, old::serialize),
			cx,
		)
		.unwrap()
		.iter()
		.map(|b| decode(b, new::deserialize))
		.collect()
	}

	fn down(p: &new_server::Ty, cx: &mut Session) -> Vec<old_server::Ty> {
		convert(
			Root::PlayToServer,
			at("1.19.4"),
			at("1.17.1"),
			&encode(p, new_server::serialize),
			cx,
		)
		.unwrap()
		.iter()
		.map(|b| decode(b, old_server::deserialize))
		.collect()
	}

	fn tag(name: &str, value: Payload) -> Entry {
		Entry::new(name, value)
	}

	fn find<'a>(c: &'a [Entry], name: &str) -> &'a Payload {
		&c.iter()
			.find(|e| e.name == name)
			.unwrap_or_else(|| panic!("no {name}"))
			.value
	}

	fn compound_of(p: &Payload) -> &Vec<Entry> {
		let Payload::compound(c) = p else {
			panic!("{p:?}")
		};
		c
	}

	fn overworld() -> Vec<Entry> {
		vec![
			tag("piglin_safe", Payload::byte(0)),
			tag("natural", Payload::byte(1)),
			tag("ambient_light", Payload::float(0.0)),
			tag(
				"infiniburn",
				Payload::string("minecraft:infiniburn_overworld".into()),
			),
			tag("respawn_anchor_works", Payload::byte(0)),
			tag("has_skylight", Payload::byte(1)),
			tag("bed_works", Payload::byte(1)),
			tag("effects", Payload::string("minecraft:overworld".into())),
			tag("has_raids", Payload::byte(1)),
			tag("logical_height", Payload::int(256)),
			tag("coordinate_scale", Payload::double(1.0)),
			tag("min_y", Payload::int(0)),
			tag("height", Payload::int(256)),
			tag("ultrawarm", Payload::byte(0)),
			tag("has_ceiling", Payload::byte(0)),
		]
	}

	fn registry(name: &str, entries: Vec<(&str, Vec<Entry>)>) -> Entry {
		let list = entries
			.into_iter()
			.enumerate()
			.map(|(i, (n, element))| {
				vec![
					tag("name", Payload::string(n.into())),
					tag("id", Payload::int(i as i32)),
					tag("element", Payload::compound(element)),
				]
			})
			.collect();
		tag(
			name,
			Payload::compound(vec![
				tag("type", Payload::string(name.into())),
				tag("value", Payload::list(List::new(ListItems::compound(list)))),
			]),
		)
	}

	fn login() -> old::Ty {
		let plains = vec![
			tag("precipitation", Payload::string("rain".into())),
			tag("temperature", Payload::float(0.8)),
			tag("downfall", Payload::float(0.4)),
			tag("category", Payload::string("plains".into())),
			tag(
				"effects",
				Payload::compound(vec![tag("sky_color", Payload::int(7907327))]),
			),
		];
		let codec = compound(vec![
			registry(
				"minecraft:dimension_type",
				vec![
					("minecraft:overworld", overworld()),
					("minecraft:the_nether", vec![]),
				],
			),
			registry(
				"minecraft:worldgen/biome",
				vec![("minecraft:plains", plains)],
			),
		]);
		old::Ty::login(v1_17_1::play_toClient_packet_login::Ty {
			entityId: 7,
			isHardcore: false,
			gameMode: 1,
			previousGameMode: -1,
			worldNames: vec!["minecraft:overworld".into()],
			dimensionCodec: codec,
			dimension: compound(overworld()),
			worldName: "minecraft:overworld".into(),
			hashedSeed: 0,
			maxPlayers: 20,
			viewDistance: 8,
			reducedDebugInfo: false,
			enableRespawnScreen: true,
			isDebug: false,
			isFlat: false,
		})
	}

	#[test]
	fn join_game_gets_every_registry() {
		let mut cx = Session::new();
		let out = up(&login(), &mut cx);
		let [new::Ty::login(l), new::Ty::feature_flags(flags)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(flags, &vec!["minecraft:vanilla".to_owned()]);
		assert_eq!(
			(l.worldType.as_str(), l.simulationDistance, l.entityId),
			("minecraft:overworld", 8, 7)
		);
		assert!(l.death.is_none());
		let codec = compound_of(&l.dimensionCodec.value);
		for name in [
			"minecraft:chat_type",
			"minecraft:damage_type",
			"minecraft:trim_material",
			"minecraft:trim_pattern",
		] {
			find(codec, name);
		}
		let dims = compound_of(find(codec, "minecraft:dimension_type"));
		let Payload::list(List {
			items: ListItems::compound(dims),
			..
		}) = find(dims, "value")
		else {
			panic!()
		};
		let overworld = compound_of(find(&dims[0], "element"));
		assert_eq!(
			find(overworld, "infiniburn"),
			&Payload::string("#minecraft:infiniburn_overworld".into())
		);
		find(overworld, "monster_spawn_light_level");
		let biomes = compound_of(find(codec, "minecraft:worldgen/biome"));
		let Payload::list(List {
			items: ListItems::compound(biomes),
			..
		}) = find(biomes, "value")
		else {
			panic!()
		};
		assert_eq!(
			find(
				compound_of(find(&biomes[0], "element")),
				"has_precipitation"
			),
			&Payload::byte(1)
		);
	}

	fn state(version: &str, text: &str) -> u32 {
		registry::blocks(version)
			.unwrap()
			.parse_state(text)
			.unwrap()
	}

	#[test]
	fn chunk_blocks_keep_their_names() {
		let mut cx = Session::new();
		up(&login(), &mut cx);
		let names = [
			"minecraft:air",
			"minecraft:stone",
			"minecraft:oak_sapling[stage=1]",
			"minecraft:chest[facing=east,type=single,waterlogged=false]",
		];
		let palette: Vec<u32> = names.iter().map(|n| state("1.17.1", n)).collect();
		let mut data = Vec::new();
		data.extend_from_slice(&3000i16.to_be_bytes());
		data.push(4);
		varint(&mut data, palette.len() as i32);
		for p in &palette {
			varint(&mut data, *p as i32);
		}
		let blocks: Vec<u32> = (0..4096).map(|i| (i % 4) as u32).collect();
		longs(&mut data, &blocks, 4);
		let chunk = v1_17_1::play_toClient_packet_map_chunk::Ty {
			x: 0,
			z: 0,
			bitMap: vec![1],
			heightmaps: compound(vec![]),
			biomes: vec![0; 1024],
			chunkData: data,
			blockEntities: vec![],
		};
		let out = up(&old::Ty::map_chunk(chunk), &mut cx);
		let [new::Ty::map_chunk(c)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let mut at = 0;
		at += 2;
		let states = read_container(&c.chunkData, &mut at, 4096, 4, 8);
		let b = registry::blocks("1.19.4").unwrap();
		for (i, s) in states.iter().enumerate().take(8) {
			assert_eq!(b.describe(*s).unwrap(), names[i % 4]);
		}
	}

	#[test]
	fn chat_becomes_system_chat() {
		let mut cx = Session::new();
		for (position, overlay) in [(0, false), (1, false), (2, true)] {
			let out = up(
				&old::Ty::chat(v1_17_1::play_toClient_packet_chat::Ty {
					message: "{\"text\":\"hi\"}".into(),
					position,
					sender: 0,
				}),
				&mut cx,
			);
			let [new::Ty::system_chat(c)] = out.as_slice() else {
				panic!("{out:?}")
			};
			assert_eq!(
				(c.content.as_str(), c.isActionBar),
				("{\"text\":\"hi\"}", overlay)
			);
		}
	}

	fn entity_id(version: &str, name: &str) -> i32 {
		registry::names(version, "entities")
			.unwrap()
			.id(name)
			.unwrap() as i32
	}

	#[test]
	fn entity_spawns_merge_into_spawn_entity() {
		let mut cx = Session::new();
		let zombie =
			old::Ty::spawn_entity_living(v1_17_1::play_toClient_packet_spawn_entity_living::Ty {
				entityId: 10,
				entityUUID: 5,
				r#type: entity_id("1.17.1", "zombie"),
				x: 1.0,
				y: 2.0,
				z: 3.0,
				yaw: 10,
				pitch: 20,
				headPitch: 30,
				velocity: v1_17_1::vec3i16::Ty { x: 1, y: 2, z: 3 },
			});
		let out = up(&zombie, &mut cx);
		let [new::Ty::spawn_entity(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			(s.r#type, s.yaw, s.pitch, s.headPitch, s.objectData),
			(entity_id("1.19.4", "zombie"), 10, 20, 30, 0)
		);
		let painting = old::Ty::spawn_entity_painting(
			v1_17_1::play_toClient_packet_spawn_entity_painting::Ty {
				entityId: 11,
				entityUUID: 6,
				title: 25,
				location: v1_17_1::position::Ty { x: 4, y: 70, z: -3 },
				direction: 1,
			},
		);
		let out = up(&painting, &mut cx);
		let [new::Ty::spawn_entity(s), new::Ty::entity_metadata(m)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			(s.r#type, s.objectData, s.x, s.y, s.z),
			(entity_id("1.19.4", "painting"), 4, 4.5, 70.5, -2.5)
		);
		assert_eq!(m.entityId, 11);
		let [item] = m.metadata.as_slice() else {
			panic!("{m:?}")
		};
		assert_eq!(item.key, 8);
		assert_eq!(
			item.value,
			v1_19_4::entityMetadataItem::Ty::painting_variant(29)
		);
		let sand = state("1.17.1", "minecraft:red_sand");
		let falling = old::Ty::spawn_entity(v1_17_1::play_toClient_packet_spawn_entity::Ty {
			entityId: 12,
			objectUUID: 7,
			r#type: entity_id("1.17.1", "falling_block"),
			x: 0.0,
			y: 0.0,
			z: 0.0,
			pitch: 0,
			yaw: 64,
			objectData: sand as i32,
			velocity: v1_17_1::vec3i16::Ty { x: 0, y: 0, z: 0 },
		});
		let out = up(&falling, &mut cx);
		let [new::Ty::spawn_entity(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(s.objectData as u32, state("1.19.4", "minecraft:red_sand"));
		assert_eq!(s.headPitch, 64);
	}

	#[test]
	fn item_metadata_follows_the_item_registry() {
		let mut cx = Session::new();
		let diamond = |v: &str| registry::names(v, "items").unwrap().id("diamond").unwrap() as i32;
		let metadata =
			old::Ty::entity_metadata(v1_17_1::play_toClient_packet_entity_metadata::Ty {
				entityId: 3,
				metadata: vec![v1_17_1::entityMetadata__items::Ty {
					key: 8,
					r#type: 6,
					value: v1_17_1::entityMetadataItem::Ty::v6(v1_17_1::slot::Ty::r#true(
						v1_17_1::slot__true::Ty {
							itemId: diamond("1.17.1"),
							itemCount: 3,
							nbtData: None,
						},
					)),
				}],
			});
		let out = up(&metadata, &mut cx);
		let [new::Ty::entity_metadata(m)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let v1_19_4::entityMetadataItem::Ty::item_stack(v1_19_4::slot::Ty::r#true(s)) =
			&m.metadata[0].value
		else {
			panic!("{m:?}")
		};
		assert_eq!((s.itemId, s.itemCount), (diamond("1.19.4"), 3));
		assert_ne!(diamond("1.17.1"), diamond("1.19.4"));
	}

	#[test]
	fn block_actions_are_acknowledged() {
		let mut cx = Session::new();
		let dig = new_server::Ty::block_dig(v1_19_4::play_toServer_packet_block_dig::Ty {
			status: 0,
			location: v1_19_4::position::Ty { x: 1, y: 64, z: 2 },
			face: 1,
			sequence: 42,
		});
		let out = down(&dig, &mut cx);
		assert!(
			matches!(out.as_slice(), [old_server::Ty::block_dig(d)] if d.status == 0 && d.location.y == 64)
		);
		let change = old::Ty::block_change(v1_17_1::play_toClient_packet_block_change::Ty {
			location: v1_17_1::position::Ty { x: 1, y: 64, z: 2 },
			r#type: 0,
		});
		let out = up(&change, &mut cx);
		assert!(
			matches!(
				out.as_slice(),
				[
					new::Ty::block_change(_),
					new::Ty::acknowledge_player_digging(42)
				]
			),
			"{out:?}"
		);
		assert_eq!(up(&change, &mut cx).len(), 1);
		assert!(cx.take_outgoing().is_empty());
		down(
			&new_server::Ty::use_item(v1_19_4::play_toServer_packet_use_item::Ty {
				hand: 0,
				sequence: 43,
			}),
			&mut cx,
		);
		let [sent] = cx.take_outgoing().try_into().unwrap();
		assert_eq!(sent.root, Root::PlayToClient);
		let acks: Vec<new::Ty> = convert(
			Root::PlayToClient,
			sent.version,
			at("1.19.4"),
			&sent.data,
			&mut cx,
		)
		.unwrap()
		.iter()
		.map(|b| decode(b, new::deserialize))
		.collect();
		assert_eq!(acks, vec![new::Ty::acknowledge_player_digging(43)]);
		down(
			&new_server::Ty::use_item(v1_19_4::play_toServer_packet_use_item::Ty {
				hand: 0,
				sequence: 44,
			}),
			&mut cx,
		);
		down(&dig, &mut cx);
		assert_eq!(cx.take_outgoing().len(), 1);
		let time =
			old::Ty::update_time(v1_17_1::play_toClient_packet_update_time::Ty { age: 1, time: 2 });
		assert_eq!(up(&time, &mut cx).len(), 1);
		let out = up(&time, &mut cx);
		assert!(
			matches!(
				out.as_slice(),
				[
					new::Ty::update_time(_),
					new::Ty::acknowledge_player_digging(42)
				]
			),
			"{out:?}"
		);
	}

	#[test]
	fn signed_chat_reaches_the_old_server_as_plain_chat() {
		let mut cx = Session::new();
		let message =
			new_server::Ty::chat_message(v1_19_4::play_toServer_packet_chat_message::Ty {
				message: "hello".into(),
				timestamp: 1,
				salt: 2,
				signature: None,
				offset: 0,
				acknowledged: vec![0; 3],
			});
		assert_eq!(
			down(&message, &mut cx),
			vec![old_server::Ty::chat("hello".into())]
		);
		let command =
			new_server::Ty::chat_command(v1_19_4::play_toServer_packet_chat_command::Ty {
				command: "gamemode 1".into(),
				timestamp: 1,
				salt: 2,
				argumentSignatures: vec![],
				messageCount: 0,
				acknowledged: vec![0; 3],
			});
		assert_eq!(
			down(&command, &mut cx),
			vec![old_server::Ty::chat("/gamemode 1".into())]
		);
	}

	#[test]
	fn client_packets_reach_the_old_server() {
		let mut cx = Session::new();
		let diamond = |v: &str| registry::names(v, "items").unwrap().id("diamond").unwrap() as i32;
		let settings = new_server::Ty::settings(v1_19_4::play_toServer_packet_settings::Ty {
			locale: "en_us".into(),
			viewDistance: 10,
			chatFlags: 0,
			chatColors: true,
			skinParts: 127,
			mainHand: 1,
			enableTextFiltering: true,
			enableServerListing: true,
		});
		let out = down(&settings, &mut cx);
		let [old_server::Ty::settings(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			(s.viewDistance, s.mainHand, s.disableTextFiltering),
			(10, 1, true)
		);
		let creative = new_server::Ty::set_creative_slot(
			v1_19_4::play_toServer_packet_set_creative_slot::Ty {
				slot: 36,
				item: v1_19_4::slot::Ty::r#true(v1_19_4::slot__true::Ty {
					itemId: diamond("1.19.4"),
					itemCount: 64,
					nbtData: None,
				}),
			},
		);
		let out = down(&creative, &mut cx);
		let [old_server::Ty::set_creative_slot(c)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert!(
			matches!(&c.item, v1_17_1::slot::Ty::r#true(s) if s.itemId == diamond("1.17.1") && s.itemCount == 64)
		);
		let session = new_server::Ty::chat_session_update(
			v1_19_4::play_toServer_packet_chat_session_update::Ty {
				sessionUUID: 1,
				expireTime: 2,
				publicKey: vec![1],
				signature: vec![2],
			},
		);
		assert!(down(&session, &mut cx).is_empty());
		assert!(down(&new_server::Ty::message_acknowledgement(3), &mut cx).is_empty());
		let place = new_server::Ty::block_place(v1_19_4::play_toServer_packet_block_place::Ty {
			hand: 0,
			location: v1_19_4::position::Ty { x: 3, y: 64, z: 3 },
			direction: 1,
			cursorX: 0.5,
			cursorY: 1.0,
			cursorZ: 0.5,
			insideBlock: false,
			sequence: 9,
		});
		assert!(
			matches!(down(&place, &mut cx).as_slice(), [old_server::Ty::block_place(p)] if p.location.x == 3 && p.direction == 1)
		);
		assert_eq!(
			down(&new_server::Ty::keep_alive(5), &mut cx),
			vec![old_server::Ty::keep_alive(5)]
		);
		let out = up(&old::Ty::keep_alive(6), &mut cx);
		assert_eq!(out, vec![new::Ty::keep_alive(6)]);
		let out = up(&old::Ty::keep_alive(7), &mut cx);
		assert_eq!(
			out,
			vec![
				new::Ty::keep_alive(7),
				new::Ty::acknowledge_player_digging(9)
			]
		);
	}

	#[test]
	fn player_list_uses_action_sets() {
		use v1_17_1::play_toClient_packet_player_info__data__items__ANON::Ty as Anon;
		let mut cx = Session::new();
		let add = old::Ty::player_info(v1_17_1::play_toClient_packet_player_info::Ty {
			action: v1_17_1::play_toClient_packet_player_info__action::Ty::add_player,
			data: vec![v1_17_1::play_toClient_packet_player_info__data__items::Ty {
				uuid: 99,
				ANON: Anon::add_player(
					v1_17_1::play_toClient_packet_player_info__data__items__ANON__add_player::Ty {
						name: "Steve".into(),
						properties: vec![],
						gamemode: 1,
						ping: 30,
						displayName: None,
					},
				),
			}],
		});
		let out = up(&add, &mut cx);
		let [new::Ty::player_info(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert!(p.action.add_player && p.action.update_listed && p.action.update_latency);
		let e = &p.data[0];
		assert_eq!(e.uuid, 99);
		assert!(
			matches!(&e.player, v1_19_4::play_toClient_packet_player_info__data__items__player::Ty::r#true(g) if g.name == "Steve")
		);
		assert!(matches!(
			e.latency,
			v1_19_4::play_toClient_packet_player_info__data__items__latency::Ty::r#true(30)
		));
		assert!(matches!(
			e.listed,
			v1_19_4::play_toClient_packet_player_info__data__items__listed::Ty::r#true(1)
		));
		let remove = old::Ty::player_info(v1_17_1::play_toClient_packet_player_info::Ty {
			action: v1_17_1::play_toClient_packet_player_info__action::Ty::remove_player,
			data: vec![v1_17_1::play_toClient_packet_player_info__data__items::Ty {
				uuid: 99,
				ANON: Anon::remove_player,
			}],
		});
		assert_eq!(up(&remove, &mut cx), vec![new::Ty::player_remove(vec![99])]);
	}

	#[test]
	fn login_state_round_trips() {
		use v1_17_1::login_toClient_packet as old_login;
		use v1_17_1::login_toServer_packet as old_login_server;
		use v1_19_4::login_toClient_packet as new_login;
		use v1_19_4::login_toServer_packet as new_login_server;
		let mut cx = Session::new();
		let start =
			new_login_server::Ty::login_start(v1_19_4::login_toServer_packet_login_start::Ty {
				username: "Alex".into(),
				playerUUID: Some(1),
			});
		let out = convert(
			Root::LoginToServer,
			at("1.19.4"),
			at("1.17.1"),
			&encode(&start, new_login_server::serialize),
			&mut cx,
		)
		.unwrap();
		assert_eq!(
			decode(&out[0], old_login_server::deserialize),
			old_login_server::Ty::login_start("Alex".into())
		);
		let success = old_login::Ty::success(v1_17_1::login_toClient_packet_success::Ty {
			uuid: 5,
			username: "Alex".into(),
		});
		let out = convert(
			Root::LoginToClient,
			at("1.17.1"),
			at("1.19.4"),
			&encode(&success, old_login::serialize),
			&mut cx,
		)
		.unwrap();
		let new_login::Ty::success(s) = decode(&out[0], new_login::deserialize) else {
			panic!()
		};
		assert_eq!(
			(s.uuid, s.username.as_str(), s.properties.len()),
			(5, "Alex", 0)
		);
	}

	#[test]
	fn sounds_follow_the_sound_registry() {
		let mut cx = Session::new();
		let id =
			|v: Version, name: &str| registry::names(v, "sounds").unwrap().id(name).unwrap() as i32;
		let ambient = id(Version::V1_17_1, "entity.zombie.ambient");
		let sound = old::Ty::sound_effect(v1_17_1::play_toClient_packet_sound_effect::Ty {
			soundId: ambient,
			soundCategory: 5,
			x: 8,
			y: 16,
			z: 24,
			volume: 1.0,
			pitch: 1.0,
		});
		let out = up(&sound, &mut cx);
		let [new::Ty::sound_effect(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			s.sound,
			v1_19_4::ItemSoundHolder::Ty::Id(id(Version::V1_19_4, "entity.zombie.ambient"))
		);
		assert_ne!(ambient, id(Version::V1_19_4, "entity.zombie.ambient"));
		assert!(matches!(s.soundCategory, v1_19_4::soundSource::Ty::hostile));
		let back = sound_down(&new::Ty::sound_effect(s.clone()), Version::V1_19_2, &mut cx);
		let [v1_19_2::play_toClient_packet::Ty::sound_effect(old)] = back.as_slice() else {
			panic!("{back:?}")
		};
		assert_eq!(old.soundId, id(Version::V1_19_2, "entity.zombie.ambient"));
		let mut custom = s.clone();
		custom.sound = v1_19_4::ItemSoundHolder::Ty::Inline(v1_19_4::ItemSoundEvent::Ty {
			soundName: "custom:thing".into(),
			fixedRange: None,
		});
		let back = sound_down(&new::Ty::sound_effect(custom), Version::V1_19_2, &mut cx);
		assert!(
			matches!(back.as_slice(), [v1_19_2::play_toClient_packet::Ty::named_sound_effect(n)] if n.soundName == "custom:thing")
		);
	}

	fn sound_down(
		p: &new::Ty,
		to: Version,
		cx: &mut Session,
	) -> Vec<v1_19_2::play_toClient_packet::Ty> {
		convert(
			Root::PlayToClient,
			at("1.19.4"),
			to.index(),
			&encode(p, new::serialize),
			cx,
		)
		.unwrap()
		.iter()
		.map(|b| decode(b, v1_19_2::play_toClient_packet::deserialize))
		.collect()
	}

	#[test]
	fn door_events_become_sounds_and_spawners_nest_their_entity() {
		let mut cx = Session::new();
		let door = old::Ty::world_event(v1_17_1::play_toClient_packet_world_event::Ty {
			effectId: 1006,
			location: v1_17_1::position::Ty { x: 2, y: 64, z: -1 },
			data: 0,
			global: false,
		});
		let out = up(&door, &mut cx);
		let [new::Ty::sound_effect(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let open = registry::names(Version::V1_19_4, "sounds")
			.unwrap()
			.id("block.wooden_door.open")
			.unwrap() as i32;
		assert_eq!(s.sound, v1_19_4::ItemSoundHolder::Ty::Id(open));
		assert_eq!((s.x, s.y, s.z), (20, 516, -4));
		let spawner =
			old::Ty::tile_entity_data(v1_17_1::play_toClient_packet_tile_entity_data::Ty {
				location: v1_17_1::position::Ty { x: 0, y: 10, z: 0 },
				action: 1,
				nbtData: Some(compound(vec![tag(
					"SpawnData",
					Payload::compound(vec![tag("id", Payload::string("minecraft:zombie".into()))]),
				)])),
			});
		let out = up(&spawner, &mut cx);
		let [new::Ty::tile_entity_data(t)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(t.action, 9);
		let data = compound_of(&t.nbtData.as_ref().unwrap().value);
		let entity = compound_of(find(compound_of(find(data, "SpawnData")), "entity"));
		assert_eq!(
			find(entity, "id"),
			&Payload::string("minecraft:zombie".into())
		);
	}

	#[test]
	fn hurt_status_becomes_damage_event() {
		let mut cx = Session::new();
		let out = up(
			&old::Ty::entity_status(v1_17_1::play_toClient_packet_entity_status::Ty {
				entityId: 4,
				entityStatus: 2,
			}),
			&mut cx,
		);
		let [new::Ty::damage_event(d)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!((d.entityId, d.sourceTypeId, d.sourceCauseId), (4, 16, 0));
		let out = up(
			&old::Ty::entity_status(v1_17_1::play_toClient_packet_entity_status::Ty {
				entityId: 4,
				entityStatus: 3,
			}),
			&mut cx,
		);
		assert!(matches!(out.as_slice(), [new::Ty::entity_status(_)]));
	}

	#[test]
	fn particles_keep_their_block() {
		let mut cx = Session::new();
		let id = |v: &str, n: &str| registry::names(v, "particles").unwrap().id(n).unwrap() as i32;
		let gold = state("1.17.1", "minecraft:gold_block");
		let particles =
			old::Ty::world_particles(v1_17_1::play_toClient_packet_world_particles::Ty {
				particleId: id("1.17.1", "block"),
				longDistance: false,
				x: 0.0,
				y: 0.0,
				z: 0.0,
				offsetX: 0.0,
				offsetY: 0.0,
				offsetZ: 0.0,
				particleData: 0.0,
				particles: 5,
				data: v1_17_1::particleData::Ty::v4(gold as i32),
			});
		let out = up(&particles, &mut cx);
		let [new::Ty::world_particles(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(p.particleId, id("1.19.4", "block"));
		assert_eq!(
			p.data,
			v1_19_4::particleData::Ty::v2(state("1.19.4", "minecraft:gold_block") as i32)
		);
	}

	fn vibration(destination: v1_17_1::particleData__v37__destination::Ty, kind: &str) -> old::Ty {
		old::Ty::world_particles(v1_17_1::play_toClient_packet_world_particles::Ty {
			particleId: registry::names(Version::V1_17_1, "particles")
				.unwrap()
				.id("vibration")
				.unwrap() as i32,
			longDistance: false,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			offsetX: 0.0,
			offsetY: 0.0,
			offsetZ: 0.0,
			particleData: 0.0,
			particles: 1,
			data: v1_17_1::particleData::Ty::v37(v1_17_1::particleData__v37::Ty {
				origin: v1_17_1::position::Ty { x: 1, y: 2, z: 3 },
				positionType: kind.into(),
				destination,
				ticks: 40,
			}),
		})
	}

	#[test]
	fn vibrations_follow_the_patched_layout() {
		use minecraft_versions::gen::convert::v1_19_3;
		use v1_17_1::particleData__v37__destination::Ty as Old;
		use v1_19_3::particleData__v40__destination::Ty as New;
		let mut cx = Session::new();
		let to_1_19_3 = |p: &old::Ty, cx: &mut Session| -> Vec<v1_19_3::play_toClient_packet::Ty> {
			convert(
				Root::PlayToClient,
				at("1.17.1"),
				at("1.19.3"),
				&encode(p, old::serialize),
				cx,
			)
			.unwrap()
			.iter()
			.map(|b| decode(b, v1_19_3::play_toClient_packet::deserialize))
			.collect()
		};
		let block = vibration(
			Old::minecraft_block(v1_17_1::position::Ty { x: 5, y: 6, z: 7 }),
			"minecraft:block",
		);
		let out = to_1_19_3(&block, &mut cx);
		let [v1_19_3::play_toClient_packet::Ty::world_particles(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let v1_19_3::particleData::Ty::v40(v) = &p.data else {
			panic!("{p:?}")
		};
		assert_eq!(
			(&v.destination, v.ticks),
			(
				&New::minecraft_block(v1_19_3::position::Ty { x: 5, y: 6, z: 7 }),
				40
			)
		);
		assert_eq!(up(&block, &mut cx).len(), 1);
		let entity = vibration(Old::minecraft_entity(9), "minecraft:entity");
		let out = to_1_19_3(&entity, &mut cx);
		let [v1_19_3::play_toClient_packet::Ty::world_particles(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let v1_19_3::particleData::Ty::v40(v) = &p.data else {
			panic!("{p:?}")
		};
		assert_eq!(
			v.destination,
			New::minecraft_entity(
				v1_19_3::particleData__v40__destination__minecraft_entity::Ty {
					entityId: 9,
					entityEyeHeight: 0.0
				}
			)
		);
		assert!(up(&entity, &mut cx).is_empty());
	}

	#[test]
	fn cats_get_their_variant_type_in_1_19() {
		use minecraft_versions::gen::convert::v1_19;
		let mut cx = Session::new();
		let cat =
			old::Ty::spawn_entity_living(v1_17_1::play_toClient_packet_spawn_entity_living::Ty {
				entityId: 20,
				entityUUID: 1,
				r#type: entity_id("1.17.1", "cat"),
				x: 0.0,
				y: 0.0,
				z: 0.0,
				yaw: 0,
				pitch: 0,
				headPitch: 0,
				velocity: v1_17_1::vec3i16::Ty { x: 0, y: 0, z: 0 },
			});
		let variant = old::Ty::entity_metadata(v1_17_1::play_toClient_packet_entity_metadata::Ty {
			entityId: 20,
			metadata: vec![v1_17_1::entityMetadata__items::Ty {
				key: 19,
				r#type: 1,
				value: v1_17_1::entityMetadataItem::Ty::v1(3),
			}],
		});
		let to_1_19 = |p: &old::Ty, cx: &mut Session| -> Vec<v1_19::play_toClient_packet::Ty> {
			convert(
				Root::PlayToClient,
				at("1.17.1"),
				at("1.19"),
				&encode(p, old::serialize),
				cx,
			)
			.unwrap()
			.iter()
			.map(|b| decode(b, v1_19::play_toClient_packet::deserialize))
			.collect()
		};
		to_1_19(&cat, &mut cx);
		let out = to_1_19(&variant, &mut cx);
		let [v1_19::play_toClient_packet::Ty::entity_metadata(m)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			(
				m.metadata[0].key,
				m.metadata[0].r#type,
				&m.metadata[0].value
			),
			(19, 19, &v1_19::entityMetadataItem::Ty::v19(3))
		);
		up(&cat, &mut cx);
		let out = up(&variant, &mut cx);
		let [new::Ty::entity_metadata(m)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			m.metadata[0].value,
			v1_19_4::entityMetadataItem::Ty::cat_variant(3)
		);
	}

	fn down_client(p: &new::Ty, cx: &mut Session) -> Vec<old::Ty> {
		convert(
			Root::PlayToClient,
			at("1.19.4"),
			at("1.17.1"),
			&encode(p, new::serialize),
			cx,
		)
		.unwrap()
		.iter()
		.map(|b| decode(b, old::deserialize))
		.collect()
	}

	fn spawn_new(id: i32, ty: &str, data: i32) -> new::Ty {
		new::Ty::spawn_entity(v1_19_4::play_toClient_packet_spawn_entity::Ty {
			entityId: id,
			objectUUID: id as u128,
			r#type: entity_id("1.19.4", ty),
			x: 4.5,
			y: 70.5,
			z: -2.5,
			pitch: 0,
			yaw: 0,
			headPitch: 0,
			objectData: data,
			velocity: v1_19_4::vec3i16::Ty { x: 0, y: 0, z: 0 },
		})
	}

	fn metadata_new(
		id: i32,
		key: u8,
		ty: v1_19_4::entityMetadata__items__type::Ty,
		value: v1_19_4::entityMetadataItem::Ty,
	) -> new::Ty {
		new::Ty::entity_metadata(v1_19_4::play_toClient_packet_entity_metadata::Ty {
			entityId: id,
			metadata: vec![v1_19_4::entityMetadata__items::Ty {
				key,
				r#type: ty,
				value,
			}],
		})
	}

	#[test]
	fn old_clients_see_paintings_and_new_mobs() {
		use v1_19_4::entityMetadataItem::Ty as Item;
		use v1_19_4::entityMetadata__items__type::Ty as Type;
		let mut cx = Session::new();
		assert!(down_client(&spawn_new(30, "painting", 4), &mut cx).is_empty());
		let out = down_client(
			&metadata_new(30, 8, Type::painting_variant, Item::painting_variant(29)),
			&mut cx,
		);
		let [old::Ty::spawn_entity_painting(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			(
				p.entityId,
				p.title,
				p.direction,
				p.location.x,
				p.location.y,
				p.location.z
			),
			(30, 25, 1, 4, 70, -3)
		);
		let out = down_client(&spawn_new(31, "frog", 0), &mut cx);
		let [old::Ty::spawn_entity_living(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(s.r#type, entity_id("1.17.1", "rabbit"));
		let out = down_client(
			&metadata_new(31, 17, Type::frog_variant, Item::frog_variant(1)),
			&mut cx,
		);
		let [old::Ty::entity_metadata(m)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert!(m.metadata.is_empty());
		assert!(down_client(&spawn_new(32, "text_display", 0), &mut cx).is_empty());
	}

	#[test]
	fn old_clients_get_substitutes_for_new_content() {
		let mut cx = Session::new();
		let mangrove = registry::names(Version::V1_19_4, "items")
			.unwrap()
			.id("mangrove_planks")
			.unwrap() as i32;
		let window = new::Ty::window_items(v1_19_4::play_toClient_packet_window_items::Ty {
			windowId: 0,
			stateId: 0,
			items: vec![v1_19_4::slot::Ty::r#true(v1_19_4::slot__true::Ty {
				itemId: mangrove,
				itemCount: 2,
				nbtData: None,
			})],
			carriedItem: v1_19_4::slot::Ty::r#false,
		});
		let out = down_client(&window, &mut cx);
		let [old::Ty::window_items(w)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let v1_17_1::slot::Ty::r#true(s) = &w.items[0] else {
			panic!("{w:?}")
		};
		assert_eq!(
			registry::names(Version::V1_17_1, "items")
				.unwrap()
				.name(s.itemId as u32),
			Some("acacia_planks")
		);
		let cherry = state("1.19.4", "minecraft:cherry_planks");
		let change = new::Ty::block_change(v1_19_4::play_toClient_packet_block_change::Ty {
			location: v1_19_4::position::Ty { x: 0, y: 0, z: 0 },
			r#type: cherry as i32,
		});
		let out = down_client(&change, &mut cx);
		let [old::Ty::block_change(b)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_ne!(
			registry::blocks(Version::V1_17_1)
				.unwrap()
				.describe(b.r#type as u32)
				.unwrap(),
			"minecraft:air"
		);
		let marker = registry::names(Version::V1_19_4, "particles")
			.unwrap()
			.id("block_marker")
			.unwrap() as i32;
		let particles =
			new::Ty::world_particles(v1_19_4::play_toClient_packet_world_particles::Ty {
				particleId: marker,
				longDistance: false,
				x: 0.0,
				y: 0.0,
				z: 0.0,
				offsetX: 0.0,
				offsetY: 0.0,
				offsetZ: 0.0,
				particleData: 0.0,
				particles: 1,
				data: v1_19_4::particleData::Ty::v3(state(
					"1.19.4",
					"minecraft:light[level=15,waterlogged=false]",
				) as i32),
			});
		let out = down_client(&particles, &mut cx);
		let [old::Ty::world_particles(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			registry::names(Version::V1_17_1, "particles")
				.unwrap()
				.name(p.particleId as u32),
			Some("light")
		);
	}

	#[test]
	fn old_clients_send_sequenced_actions() {
		let mut cx = Session::new();
		let dig = old_server::Ty::block_dig(v1_17_1::play_toServer_packet_block_dig::Ty {
			status: 0,
			location: v1_17_1::position::Ty { x: 1, y: 2, z: 3 },
			face: 1,
		});
		let sequences: Vec<i32> = (0..2)
			.map(|_| {
				let out = convert(
					Root::PlayToServer,
					at("1.17.1"),
					at("1.19.4"),
					&encode(&dig, old_server::serialize),
					&mut cx,
				)
				.unwrap();
				let new_server::Ty::block_dig(d) = decode(&out[0], new_server::deserialize) else {
					panic!("{out:?}")
				};
				d.sequence
			})
			.collect();
		assert!(sequences[1] > sequences[0] && sequences[0] > 0);
	}
}

mod padded_chunks {
	use super::*;
	use minecraft_versions::gen::convert::{v1_12_2, v1_18_2, v1_19_2, v1_8};
	use minecraft_versions::registry;
	use serde_json::Value;

	fn login() -> Vec<u8> {
		let v: Value = serde_json::from_str(registry::login_packet("1.19.2").unwrap()).unwrap();
		let login =
			v1_19_2::play_toClient_packet::Ty::login(v1_19_2::play_toClient_packet_login::Ty {
				entityId: 1,
				isHardcore: false,
				gameMode: 0,
				previousGameMode: -1,
				worldNames: vec!["minecraft:overworld".into()],
				dimensionCodec: named("", nbt::json::typed::read(&v["dimensionCodec"]).unwrap()),
				worldType: "minecraft:overworld".into(),
				worldName: "minecraft:overworld".into(),
				hashedSeed: 0,
				maxPlayers: 20,
				viewDistance: 8,
				simulationDistance: 6,
				reducedDebugInfo: false,
				enableRespawnScreen: true,
				isDebug: false,
				isFlat: false,
				death: None,
			});
		encode(&login, v1_19_2::play_toClient_packet::serialize)
	}

	fn captured() -> Vec<u8> {
		let gz = std::fs::read(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/tests/data/1.19.2-map_chunk.bin.gz"
		))
		.unwrap();
		nbt::gzip::decompress(&gz).unwrap()
	}

	fn down(to: &str) -> Vec<Vec<u8>> {
		let mut cx = Session::new();
		convert(Root::PlayToClient, at("1.19.2"), at(to), &login(), &mut cx).unwrap();
		convert(
			Root::PlayToClient,
			at("1.19.2"),
			at(to),
			&captured(),
			&mut cx,
		)
		.unwrap()
	}

	#[test]
	fn vanilla_padding_after_the_last_section_is_left_out() {
		let out = down("1.18.2");
		let [bytes] = out.as_slice() else {
			panic!("{} packets", out.len())
		};
		let v1_18_2::play_toClient_packet::Ty::map_chunk(c) =
			decode(bytes, v1_18_2::play_toClient_packet::deserialize)
		else {
			panic!()
		};
		assert_eq!((c.x, c.z), (-8, 1));
		let mut at = 0;
		for _ in 0..24 {
			at += 2;
			read_container(&c.chunkData, &mut at, 4096, 4, 8);
			read_container(&c.chunkData, &mut at, 64, 1, 3);
		}
		assert_eq!(at, c.chunkData.len());
		let chunks = |packets: Vec<Vec<u8>>, de: fn(&[u8]) -> Option<(i32, i32)>| {
			packets.iter().filter_map(|b| de(b)).collect::<Vec<_>>()
		};
		let at_1_12_2 = chunks(down("1.12.2"), |b| {
			match decode(b, v1_12_2::play_toClient_packet::deserialize) {
				v1_12_2::play_toClient_packet::Ty::map_chunk(c) if c.bitMap != 0 => {
					Some((c.x, c.z))
				}
				_ => None,
			}
		});
		let at_1_8 = chunks(down("1.8"), |b| {
			match decode(b, v1_8::play_toClient_packet::deserialize) {
				v1_8::play_toClient_packet::Ty::map_chunk(c) if c.bitMap != 0 => Some((c.x, c.z)),
				_ => None,
			}
		});
		assert_eq!((at_1_12_2, at_1_8), (vec![(-8, 1)], vec![(-8, 1)]));
	}
}
