use minecraft_versions::gen::convert::{convert, v1_17_1, v1_18, Root, Version};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::Session;
use nbt::{Entry, Payload};

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
	use v1_17_1::play_to_client_packet as c17;
	use v1_18::play_to_client_packet as c18;

	fn up(p: &c17::Ty, cx: &mut Session) -> Vec<c18::Ty> {
		convert(
			Root::PlayToClient,
			Version::V1_17_1,
			Version::V1_18,
			&encode(p, c17::serialize),
			cx,
		)
		.unwrap()
		.iter()
		.map(|b| decode(b, c18::deserialize))
		.collect()
	}

	fn light(x: i32, z: i32) -> v1_17_1::play_to_client_packet_update_light::Ty {
		v1_17_1::play_to_client_packet_update_light::Ty {
			chunk_x: x,
			chunk_z: z,
			trust_edges: true,
			sky_light_mask: vec![0b110],
			block_light_mask: vec![0b10],
			empty_sky_light_mask: vec![0b1],
			empty_block_light_mask: vec![0b101],
			sky_light: vec![vec![0xff; 2048], vec![0xee; 2048]],
			block_light: vec![vec![0x11; 2048]],
		}
	}

	fn chunk(x: i32, z: i32) -> v1_17_1::play_to_client_packet_map_chunk::Ty {
		let stone = 1u32;
		let mut data = Vec::new();
		data.extend_from_slice(&4096i16.to_be_bytes());
		data.push(4);
		varint(&mut data, 2);
		varint(&mut data, 0);
		varint(&mut data, stone as i32);
		let blocks: Vec<u32> = (0..4096).map(|i| (i % 3 == 0) as u32).collect();
		longs(&mut data, &blocks, 4);
		v1_17_1::play_to_client_packet_map_chunk::Ty {
			x,
			z,
			bit_map: vec![0b10],
			heightmaps: compound(vec![]),
			biomes: (0..1024)
				.map(|i| if i < 128 { 1 } else { 2 + (i % 3) })
				.collect(),
			chunk_data: data,
			block_entities: vec![compound(vec![
				named("id", Payload::String("minecraft:chest".into())),
				named("x", Payload::Int(x * 16 + 3)),
				named("y", Payload::Int(20)),
				named("z", Payload::Int(z * 16 + 5)),
			])],
		}
	}

	#[test]
	fn chunk_merges_buffered_light() {
		let mut cx = Session::new();
		assert!(up(&c17::Ty::UpdateLight(light(4, -2)), &mut cx).is_empty());
		let out = up(&c17::Ty::MapChunk(chunk(4, -2)), &mut cx);
		let [c18::Ty::MapChunk(c)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!((c.x, c.z, c.trust_edges), (4, -2, true));
		assert_eq!(c.sky_light_mask, vec![0b110]);
		assert_eq!(c.empty_block_light_mask, vec![0b101]);
		assert_eq!(c.sky_light.len(), 2);
		assert_eq!(c.block_light, vec![vec![0x11; 2048]]);
		let be = &c.block_entities[0];
		assert_eq!((be.anon.x, be.anon.z, be.y, be.r#type), (3, 5, 20, 1));
		let d = &c.chunk_data;
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
		let later = up(&c17::Ty::UpdateLight(light(4, -2)), &mut cx);
		assert!(matches!(later.as_slice(), [c18::Ty::UpdateLight(_)]));
	}

	#[test]
	fn chunk_without_light_is_dark_until_updated() {
		let mut cx = Session::new();
		let out = up(&c17::Ty::MapChunk(chunk(0, 0)), &mut cx);
		let [c18::Ty::MapChunk(c)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(c.empty_sky_light_mask, vec![(1 << 18) - 1]);
		assert!(c.sky_light.is_empty());
		assert!(matches!(
			up(&c17::Ty::UpdateLight(light(0, 0)), &mut cx).as_slice(),
			[c18::Ty::UpdateLight(_)]
		));
		up(
			&c17::Ty::UnloadChunk(v1_17_1::play_to_client_packet_unload_chunk::Ty {
				chunk_x: 0,
				chunk_z: 0,
			}),
			&mut cx,
		);
		assert!(up(&c17::Ty::UpdateLight(light(0, 0)), &mut cx).is_empty());
	}

	#[test]
	fn chunk_round_trips_down() {
		let mut cx = Session::new();
		up(&c17::Ty::UpdateLight(light(1, 1)), &mut cx);
		let out = up(&c17::Ty::MapChunk(chunk(1, 1)), &mut cx);
		let bytes = encode(&out[0], c18::serialize);
		let out: Vec<c17::Ty> = convert(
			Root::PlayToClient,
			Version::V1_18,
			Version::V1_17_1,
			&bytes,
			&mut cx,
		)
		.unwrap()
		.iter()
		.map(|b| decode(b, c17::deserialize))
		.collect();
		let [c17::Ty::UpdateLight(l), c17::Ty::MapChunk(m)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(l, &light(1, 1));
		let original = chunk(1, 1);
		assert_eq!(
			(m.bit_map.clone(), m.biomes.clone()),
			(original.bit_map, original.biomes)
		);
		let Payload::Compound(be) = &m.block_entities[0].value else {
			panic!()
		};
		assert!(be
			.iter()
			.any(|e| e.name == "x" && e.value == Payload::Int(19)));
		assert!(be
			.iter()
			.any(|e| e.name == "id" && e.value == Payload::String("minecraft:chest".into())));
	}
}

mod chain {
	use super::*;
	use minecraft_versions::gen::convert::{v1_19_2, v1_19_4, Version};
	use minecraft_versions::registry;
	use nbt::{List, ListItems};
	use v1_17_1::play_to_client_packet as old;
	use v1_17_1::play_to_server_packet as old_server;
	use v1_19_4::play_to_client_packet as new;
	use v1_19_4::play_to_server_packet as new_server;

	fn up(p: &old::Ty, cx: &mut Session) -> Vec<new::Ty> {
		convert(
			Root::PlayToClient,
			Version::V1_17_1,
			Version::V1_19_4,
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
			Version::V1_19_4,
			Version::V1_17_1,
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
		let Payload::Compound(c) = p else {
			panic!("{p:?}")
		};
		c
	}

	fn overworld() -> Vec<Entry> {
		vec![
			tag("piglin_safe", Payload::Byte(0)),
			tag("natural", Payload::Byte(1)),
			tag("ambient_light", Payload::Float(0.0)),
			tag(
				"infiniburn",
				Payload::String("minecraft:infiniburn_overworld".into()),
			),
			tag("respawn_anchor_works", Payload::Byte(0)),
			tag("has_skylight", Payload::Byte(1)),
			tag("bed_works", Payload::Byte(1)),
			tag("effects", Payload::String("minecraft:overworld".into())),
			tag("has_raids", Payload::Byte(1)),
			tag("logical_height", Payload::Int(256)),
			tag("coordinate_scale", Payload::Double(1.0)),
			tag("min_y", Payload::Int(0)),
			tag("height", Payload::Int(256)),
			tag("ultrawarm", Payload::Byte(0)),
			tag("has_ceiling", Payload::Byte(0)),
		]
	}

	fn registry(name: &str, entries: Vec<(&str, Vec<Entry>)>) -> Entry {
		let list = entries
			.into_iter()
			.enumerate()
			.map(|(i, (n, element))| {
				vec![
					tag("name", Payload::String(n.into())),
					tag("id", Payload::Int(i as i32)),
					tag("element", Payload::Compound(element)),
				]
			})
			.collect();
		tag(
			name,
			Payload::Compound(vec![
				tag("type", Payload::String(name.into())),
				tag("value", Payload::List(List::new(ListItems::Compound(list)))),
			]),
		)
	}

	fn login() -> old::Ty {
		let plains = vec![
			tag("precipitation", Payload::String("rain".into())),
			tag("temperature", Payload::Float(0.8)),
			tag("downfall", Payload::Float(0.4)),
			tag("category", Payload::String("plains".into())),
			tag(
				"effects",
				Payload::Compound(vec![tag("sky_color", Payload::Int(7907327))]),
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
		old::Ty::Login(v1_17_1::play_to_client_packet_login::Ty {
			entity_id: 7,
			is_hardcore: false,
			game_mode: 1,
			previous_game_mode: -1,
			world_names: vec!["minecraft:overworld".into()],
			dimension_codec: codec,
			dimension: compound(overworld()),
			world_name: "minecraft:overworld".into(),
			hashed_seed: 0,
			max_players: 20,
			view_distance: 8,
			reduced_debug_info: false,
			enable_respawn_screen: true,
			is_debug: false,
			is_flat: false,
		})
	}

	#[test]
	fn join_game_gets_every_registry() {
		let mut cx = Session::new();
		let out = up(&login(), &mut cx);
		let [new::Ty::Login(l), new::Ty::FeatureFlags(flags)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(flags, &vec!["minecraft:vanilla".to_owned()]);
		assert_eq!(
			(l.world_type.as_str(), l.simulation_distance, l.entity_id),
			("minecraft:overworld", 8, 7)
		);
		assert!(l.death.is_none());
		let codec = compound_of(&l.dimension_codec.value);
		for name in [
			"minecraft:chat_type",
			"minecraft:damage_type",
			"minecraft:trim_material",
			"minecraft:trim_pattern",
		] {
			find(codec, name);
		}
		let dims = compound_of(find(codec, "minecraft:dimension_type"));
		let Payload::List(List {
			items: ListItems::Compound(dims),
			..
		}) = find(dims, "value")
		else {
			panic!()
		};
		let overworld = compound_of(find(&dims[0], "element"));
		assert_eq!(
			find(overworld, "infiniburn"),
			&Payload::String("#minecraft:infiniburn_overworld".into())
		);
		find(overworld, "monster_spawn_light_level");
		let biomes = compound_of(find(codec, "minecraft:worldgen/biome"));
		let Payload::List(List {
			items: ListItems::Compound(biomes),
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
			&Payload::Byte(1)
		);
	}

	fn state(version: Version, text: &str) -> u32 {
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
		let palette: Vec<u32> = names.iter().map(|n| state(Version::V1_17_1, n)).collect();
		let mut data = Vec::new();
		data.extend_from_slice(&3000i16.to_be_bytes());
		data.push(4);
		varint(&mut data, palette.len() as i32);
		for p in &palette {
			varint(&mut data, *p as i32);
		}
		let blocks: Vec<u32> = (0..4096).map(|i| (i % 4) as u32).collect();
		longs(&mut data, &blocks, 4);
		let chunk = v1_17_1::play_to_client_packet_map_chunk::Ty {
			x: 0,
			z: 0,
			bit_map: vec![1],
			heightmaps: compound(vec![]),
			biomes: vec![0; 1024],
			chunk_data: data,
			block_entities: vec![],
		};
		let out = up(&old::Ty::MapChunk(chunk), &mut cx);
		let [new::Ty::MapChunk(c)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let mut at = 0;
		at += 2;
		let states = read_container(&c.chunk_data, &mut at, 4096, 4, 8);
		let b = registry::blocks(Version::V1_19_4).unwrap();
		for (i, s) in states.iter().enumerate().take(8) {
			assert_eq!(b.describe(*s).unwrap(), names[i % 4]);
		}
	}

	#[test]
	fn chat_becomes_system_chat() {
		let mut cx = Session::new();
		for (position, overlay) in [(0, false), (1, false), (2, true)] {
			let out = up(
				&old::Ty::Chat(v1_17_1::play_to_client_packet_chat::Ty {
					message: "{\"text\":\"hi\"}".into(),
					position,
					sender: 0,
				}),
				&mut cx,
			);
			let [new::Ty::SystemChat(c)] = out.as_slice() else {
				panic!("{out:?}")
			};
			assert_eq!(
				(c.content.as_str(), c.is_action_bar),
				("{\"text\":\"hi\"}", overlay)
			);
		}
	}

	fn entity_id(version: Version, name: &str) -> i32 {
		registry::names(version, "entities")
			.unwrap()
			.id(name)
			.unwrap() as i32
	}

	#[test]
	fn entity_spawns_merge_into_spawn_entity() {
		let mut cx = Session::new();
		let zombie =
			old::Ty::SpawnEntityLiving(v1_17_1::play_to_client_packet_spawn_entity_living::Ty {
				entity_id: 10,
				entity_uuid: 5,
				r#type: entity_id(Version::V1_17_1, "zombie"),
				x: 1.0,
				y: 2.0,
				z: 3.0,
				yaw: 10,
				pitch: 20,
				head_pitch: 30,
				velocity: v1_17_1::vec3i16::Ty { x: 1, y: 2, z: 3 },
			});
		let out = up(&zombie, &mut cx);
		let [new::Ty::SpawnEntity(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			(s.r#type, s.yaw, s.pitch, s.head_pitch, s.object_data),
			(entity_id(Version::V1_19_4, "zombie"), 10, 20, 30, 0)
		);
		let painting = old::Ty::SpawnEntityPainting(
			v1_17_1::play_to_client_packet_spawn_entity_painting::Ty {
				entity_id: 11,
				entity_uuid: 6,
				title: 25,
				location: v1_17_1::position::Ty { x: 4, y: 70, z: -3 },
				direction: 1,
			},
		);
		let out = up(&painting, &mut cx);
		let [new::Ty::SpawnEntity(s), new::Ty::EntityMetadata(m)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			(s.r#type, s.object_data, s.x, s.y, s.z),
			(entity_id(Version::V1_19_4, "painting"), 4, 4.5, 70.5, -2.5)
		);
		assert_eq!(m.entity_id, 11);
		let [item] = m.metadata.as_slice() else {
			panic!("{m:?}")
		};
		assert_eq!(item.key, 8);
		assert_eq!(
			item.value,
			v1_19_4::entity_metadata_item::Ty::PaintingVariant(29)
		);
		let sand = state(Version::V1_17_1, "minecraft:red_sand");
		let falling = old::Ty::SpawnEntity(v1_17_1::play_to_client_packet_spawn_entity::Ty {
			entity_id: 12,
			object_uuid: 7,
			r#type: entity_id(Version::V1_17_1, "falling_block"),
			x: 0.0,
			y: 0.0,
			z: 0.0,
			pitch: 0,
			yaw: 64,
			object_data: sand as i32,
			velocity: v1_17_1::vec3i16::Ty { x: 0, y: 0, z: 0 },
		});
		let out = up(&falling, &mut cx);
		let [new::Ty::SpawnEntity(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			s.object_data as u32,
			state(Version::V1_19_4, "minecraft:red_sand")
		);
		assert_eq!(s.head_pitch, 64);
	}

	#[test]
	fn item_metadata_follows_the_item_registry() {
		let mut cx = Session::new();
		let diamond =
			|v: Version| registry::names(v, "items").unwrap().id("diamond").unwrap() as i32;
		let metadata =
			old::Ty::EntityMetadata(v1_17_1::play_to_client_packet_entity_metadata::Ty {
				entity_id: 3,
				metadata: vec![v1_17_1::entity_metadata_items::Ty {
					key: 8,
					r#type: 6,
					value: v1_17_1::entity_metadata_item::Ty::V6(v1_17_1::slot::Ty::True(
						v1_17_1::slot_true::Ty {
							item_id: diamond(Version::V1_17_1),
							item_count: 3,
							nbt_data: None,
						},
					)),
				}],
			});
		let out = up(&metadata, &mut cx);
		let [new::Ty::EntityMetadata(m)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let v1_19_4::entity_metadata_item::Ty::ItemStack(v1_19_4::slot::Ty::True(s)) =
			&m.metadata[0].value
		else {
			panic!("{m:?}")
		};
		assert_eq!((s.item_id, s.item_count), (diamond(Version::V1_19_4), 3));
		assert_ne!(diamond(Version::V1_17_1), diamond(Version::V1_19_4));
	}

	#[test]
	fn block_actions_are_acknowledged() {
		let mut cx = Session::new();
		let dig = new_server::Ty::BlockDig(v1_19_4::play_to_server_packet_block_dig::Ty {
			status: 0,
			location: v1_19_4::position::Ty { x: 1, y: 64, z: 2 },
			face: 1,
			sequence: 42,
		});
		let out = down(&dig, &mut cx);
		assert!(
			matches!(out.as_slice(), [old_server::Ty::BlockDig(d)] if d.status == 0 && d.location.y == 64)
		);
		let change = old::Ty::BlockChange(v1_17_1::play_to_client_packet_block_change::Ty {
			location: v1_17_1::position::Ty { x: 1, y: 64, z: 2 },
			r#type: 0,
		});
		let out = up(&change, &mut cx);
		assert!(
			matches!(
				out.as_slice(),
				[
					new::Ty::BlockChange(_),
					new::Ty::AcknowledgePlayerDigging(42)
				]
			),
			"{out:?}"
		);
		assert_eq!(up(&change, &mut cx).len(), 1);
		assert!(cx.take_outgoing().is_empty());
		down(
			&new_server::Ty::UseItem(v1_19_4::play_to_server_packet_use_item::Ty {
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
			Version::V1_19_4,
			&sent.data,
			&mut cx,
		)
		.unwrap()
		.iter()
		.map(|b| decode(b, new::deserialize))
		.collect();
		assert_eq!(acks, vec![new::Ty::AcknowledgePlayerDigging(43)]);
		down(
			&new_server::Ty::UseItem(v1_19_4::play_to_server_packet_use_item::Ty {
				hand: 0,
				sequence: 44,
			}),
			&mut cx,
		);
		down(&dig, &mut cx);
		assert_eq!(cx.take_outgoing().len(), 1);
		let time =
			old::Ty::UpdateTime(v1_17_1::play_to_client_packet_update_time::Ty { age: 1, time: 2 });
		assert_eq!(up(&time, &mut cx).len(), 1);
		let out = up(&time, &mut cx);
		assert!(
			matches!(
				out.as_slice(),
				[
					new::Ty::UpdateTime(_),
					new::Ty::AcknowledgePlayerDigging(42)
				]
			),
			"{out:?}"
		);
	}

	#[test]
	fn signed_chat_reaches_the_old_server_as_plain_chat() {
		let mut cx = Session::new();
		let message =
			new_server::Ty::ChatMessage(v1_19_4::play_to_server_packet_chat_message::Ty {
				message: "hello".into(),
				timestamp: 1,
				salt: 2,
				signature: None,
				offset: 0,
				acknowledged: vec![0; 3],
			});
		assert_eq!(
			down(&message, &mut cx),
			vec![old_server::Ty::Chat("hello".into())]
		);
		let command =
			new_server::Ty::ChatCommand(v1_19_4::play_to_server_packet_chat_command::Ty {
				command: "gamemode 1".into(),
				timestamp: 1,
				salt: 2,
				argument_signatures: vec![],
				message_count: 0,
				acknowledged: vec![0; 3],
			});
		assert_eq!(
			down(&command, &mut cx),
			vec![old_server::Ty::Chat("/gamemode 1".into())]
		);
	}

	#[test]
	fn client_packets_reach_the_old_server() {
		let mut cx = Session::new();
		let diamond =
			|v: Version| registry::names(v, "items").unwrap().id("diamond").unwrap() as i32;
		let settings = new_server::Ty::Settings(v1_19_4::play_to_server_packet_settings::Ty {
			locale: "en_us".into(),
			view_distance: 10,
			chat_flags: 0,
			chat_colors: true,
			skin_parts: 127,
			main_hand: 1,
			enable_text_filtering: true,
			enable_server_listing: true,
		});
		let out = down(&settings, &mut cx);
		let [old_server::Ty::Settings(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			(s.view_distance, s.main_hand, s.disable_text_filtering),
			(10, 1, true)
		);
		let creative =
			new_server::Ty::SetCreativeSlot(v1_19_4::play_to_server_packet_set_creative_slot::Ty {
				slot: 36,
				item: v1_19_4::slot::Ty::True(v1_19_4::slot_true::Ty {
					item_id: diamond(Version::V1_19_4),
					item_count: 64,
					nbt_data: None,
				}),
			});
		let out = down(&creative, &mut cx);
		let [old_server::Ty::SetCreativeSlot(c)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert!(
			matches!(&c.item, v1_17_1::slot::Ty::True(s) if s.item_id == diamond(Version::V1_17_1) && s.item_count == 64)
		);
		let session = new_server::Ty::ChatSessionUpdate(
			v1_19_4::play_to_server_packet_chat_session_update::Ty {
				session_uuid: 1,
				expire_time: 2,
				public_key: vec![1],
				signature: vec![2],
			},
		);
		assert!(down(&session, &mut cx).is_empty());
		assert!(down(&new_server::Ty::MessageAcknowledgement(3), &mut cx).is_empty());
		let place = new_server::Ty::BlockPlace(v1_19_4::play_to_server_packet_block_place::Ty {
			hand: 0,
			location: v1_19_4::position::Ty { x: 3, y: 64, z: 3 },
			direction: 1,
			cursor_x: 0.5,
			cursor_y: 1.0,
			cursor_z: 0.5,
			inside_block: false,
			sequence: 9,
		});
		assert!(
			matches!(down(&place, &mut cx).as_slice(), [old_server::Ty::BlockPlace(p)] if p.location.x == 3 && p.direction == 1)
		);
		assert_eq!(
			down(&new_server::Ty::KeepAlive(5), &mut cx),
			vec![old_server::Ty::KeepAlive(5)]
		);
		let out = up(&old::Ty::KeepAlive(6), &mut cx);
		assert_eq!(out, vec![new::Ty::KeepAlive(6)]);
		let out = up(&old::Ty::KeepAlive(7), &mut cx);
		assert_eq!(
			out,
			vec![new::Ty::KeepAlive(7), new::Ty::AcknowledgePlayerDigging(9)]
		);
	}

	#[test]
	fn player_list_uses_action_sets() {
		use v1_17_1::play_to_client_packet_player_info_data_items_anon::Ty as Anon;
		let mut cx = Session::new();
		let add = old::Ty::PlayerInfo(v1_17_1::play_to_client_packet_player_info::Ty {
			action: v1_17_1::play_to_client_packet_player_info_action::Ty::AddPlayer,
			data: vec![v1_17_1::play_to_client_packet_player_info_data_items::Ty {
				uuid: 99,
				anon: Anon::AddPlayer(
					v1_17_1::play_to_client_packet_player_info_data_items_anon_add_player::Ty {
						name: "Steve".into(),
						properties: vec![],
						gamemode: 1,
						ping: 30,
						display_name: None,
					},
				),
			}],
		});
		let out = up(&add, &mut cx);
		let [new::Ty::PlayerInfo(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert!(p.action.add_player && p.action.update_listed && p.action.update_latency);
		let e = &p.data[0];
		assert_eq!(e.uuid, 99);
		assert!(
			matches!(&e.player, v1_19_4::play_to_client_packet_player_info_data_items_player::Ty::True(g) if g.name == "Steve")
		);
		assert!(matches!(
			e.latency,
			v1_19_4::play_to_client_packet_player_info_data_items_latency::Ty::True(30)
		));
		assert!(matches!(
			e.listed,
			v1_19_4::play_to_client_packet_player_info_data_items_listed::Ty::True(1)
		));
		let remove = old::Ty::PlayerInfo(v1_17_1::play_to_client_packet_player_info::Ty {
			action: v1_17_1::play_to_client_packet_player_info_action::Ty::RemovePlayer,
			data: vec![v1_17_1::play_to_client_packet_player_info_data_items::Ty {
				uuid: 99,
				anon: Anon::RemovePlayer,
			}],
		});
		assert_eq!(up(&remove, &mut cx), vec![new::Ty::PlayerRemove(vec![99])]);
	}

	#[test]
	fn login_state_round_trips() {
		use v1_17_1::login_to_client_packet as old_login;
		use v1_17_1::login_to_server_packet as old_login_server;
		use v1_19_4::login_to_client_packet as new_login;
		use v1_19_4::login_to_server_packet as new_login_server;
		let mut cx = Session::new();
		let start =
			new_login_server::Ty::LoginStart(v1_19_4::login_to_server_packet_login_start::Ty {
				username: "Alex".into(),
				player_uuid: Some(1),
			});
		let out = convert(
			Root::LoginToServer,
			Version::V1_19_4,
			Version::V1_17_1,
			&encode(&start, new_login_server::serialize),
			&mut cx,
		)
		.unwrap();
		assert_eq!(
			decode(&out[0], old_login_server::deserialize),
			old_login_server::Ty::LoginStart("Alex".into())
		);
		let success = old_login::Ty::Success(v1_17_1::login_to_client_packet_success::Ty {
			uuid: 5,
			username: "Alex".into(),
		});
		let out = convert(
			Root::LoginToClient,
			Version::V1_17_1,
			Version::V1_19_4,
			&encode(&success, old_login::serialize),
			&mut cx,
		)
		.unwrap();
		let new_login::Ty::Success(s) = decode(&out[0], new_login::deserialize) else {
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
		let sound = old::Ty::SoundEffect(v1_17_1::play_to_client_packet_sound_effect::Ty {
			sound_id: ambient,
			sound_category: 5,
			x: 8,
			y: 16,
			z: 24,
			volume: 1.0,
			pitch: 1.0,
		});
		let out = up(&sound, &mut cx);
		let [new::Ty::SoundEffect(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			s.sound,
			v1_19_4::item_sound_holder::Ty::Id(id(Version::V1_19_4, "entity.zombie.ambient"))
		);
		assert_ne!(ambient, id(Version::V1_19_4, "entity.zombie.ambient"));
		assert!(matches!(
			s.sound_category,
			v1_19_4::sound_source::Ty::Hostile
		));
		let back = sound_down(&new::Ty::SoundEffect(s.clone()), Version::V1_19_2, &mut cx);
		let [v1_19_2::play_to_client_packet::Ty::SoundEffect(old)] = back.as_slice() else {
			panic!("{back:?}")
		};
		assert_eq!(old.sound_id, id(Version::V1_19_2, "entity.zombie.ambient"));
		let mut custom = s.clone();
		custom.sound = v1_19_4::item_sound_holder::Ty::Inline(v1_19_4::item_sound_event::Ty {
			sound_name: "custom:thing".into(),
			fixed_range: None,
		});
		let back = sound_down(&new::Ty::SoundEffect(custom), Version::V1_19_2, &mut cx);
		assert!(
			matches!(back.as_slice(), [v1_19_2::play_to_client_packet::Ty::NamedSoundEffect(n)] if n.sound_name == "custom:thing")
		);
	}

	fn sound_down(
		p: &new::Ty,
		to: Version,
		cx: &mut Session,
	) -> Vec<v1_19_2::play_to_client_packet::Ty> {
		convert(
			Root::PlayToClient,
			Version::V1_19_4,
			to,
			&encode(p, new::serialize),
			cx,
		)
		.unwrap()
		.iter()
		.map(|b| decode(b, v1_19_2::play_to_client_packet::deserialize))
		.collect()
	}

	#[test]
	fn door_events_become_sounds_and_spawners_nest_their_entity() {
		let mut cx = Session::new();
		let door = old::Ty::WorldEvent(v1_17_1::play_to_client_packet_world_event::Ty {
			effect_id: 1006,
			location: v1_17_1::position::Ty { x: 2, y: 64, z: -1 },
			data: 0,
			global: false,
		});
		let out = up(&door, &mut cx);
		let [new::Ty::SoundEffect(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let open = registry::names(Version::V1_19_4, "sounds")
			.unwrap()
			.id("block.wooden_door.open")
			.unwrap() as i32;
		assert_eq!(s.sound, v1_19_4::item_sound_holder::Ty::Id(open));
		assert_eq!((s.x, s.y, s.z), (20, 516, -4));
		let spawner =
			old::Ty::TileEntityData(v1_17_1::play_to_client_packet_tile_entity_data::Ty {
				location: v1_17_1::position::Ty { x: 0, y: 10, z: 0 },
				action: 1,
				nbt_data: Some(compound(vec![tag(
					"SpawnData",
					Payload::Compound(vec![tag("id", Payload::String("minecraft:zombie".into()))]),
				)])),
			});
		let out = up(&spawner, &mut cx);
		let [new::Ty::TileEntityData(t)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(t.action, 9);
		let data = compound_of(&t.nbt_data.as_ref().unwrap().value);
		let entity = compound_of(find(compound_of(find(data, "SpawnData")), "entity"));
		assert_eq!(
			find(entity, "id"),
			&Payload::String("minecraft:zombie".into())
		);
	}

	#[test]
	fn hurt_status_becomes_damage_event() {
		let mut cx = Session::new();
		let out = up(
			&old::Ty::EntityStatus(v1_17_1::play_to_client_packet_entity_status::Ty {
				entity_id: 4,
				entity_status: 2,
			}),
			&mut cx,
		);
		let [new::Ty::DamageEvent(d)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			(d.entity_id, d.source_type_id, d.source_cause_id),
			(4, 16, 0)
		);
		let out = up(
			&old::Ty::EntityStatus(v1_17_1::play_to_client_packet_entity_status::Ty {
				entity_id: 4,
				entity_status: 3,
			}),
			&mut cx,
		);
		assert!(matches!(out.as_slice(), [new::Ty::EntityStatus(_)]));
	}

	#[test]
	fn particles_keep_their_block() {
		let mut cx = Session::new();
		let id =
			|v: Version, n: &str| registry::names(v, "particles").unwrap().id(n).unwrap() as i32;
		let gold = state(Version::V1_17_1, "minecraft:gold_block");
		let particles =
			old::Ty::WorldParticles(v1_17_1::play_to_client_packet_world_particles::Ty {
				particle_id: id(Version::V1_17_1, "block"),
				long_distance: false,
				x: 0.0,
				y: 0.0,
				z: 0.0,
				offset_x: 0.0,
				offset_y: 0.0,
				offset_z: 0.0,
				particle_data: 0.0,
				particles: 5,
				data: v1_17_1::particle_data::Ty::V4(gold as i32),
			});
		let out = up(&particles, &mut cx);
		let [new::Ty::WorldParticles(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(p.particle_id, id(Version::V1_19_4, "block"));
		assert_eq!(
			p.data,
			v1_19_4::particle_data::Ty::V2(state(Version::V1_19_4, "minecraft:gold_block") as i32)
		);
	}

	fn vibration(destination: v1_17_1::particle_data_v37_destination::Ty, kind: &str) -> old::Ty {
		old::Ty::WorldParticles(v1_17_1::play_to_client_packet_world_particles::Ty {
			particle_id: registry::names(Version::V1_17_1, "particles")
				.unwrap()
				.id("vibration")
				.unwrap() as i32,
			long_distance: false,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			offset_x: 0.0,
			offset_y: 0.0,
			offset_z: 0.0,
			particle_data: 0.0,
			particles: 1,
			data: v1_17_1::particle_data::Ty::V37(v1_17_1::particle_data_v37::Ty {
				origin: v1_17_1::position::Ty { x: 1, y: 2, z: 3 },
				position_type: kind.into(),
				destination,
				ticks: 40,
			}),
		})
	}

	#[test]
	fn vibrations_follow_the_patched_layout() {
		use minecraft_versions::gen::convert::v1_19_3;
		use v1_17_1::particle_data_v37_destination::Ty as Old;
		use v1_19_3::particle_data_v40_destination::Ty as New;
		let mut cx = Session::new();
		let to_1_19_3 =
			|p: &old::Ty, cx: &mut Session| -> Vec<v1_19_3::play_to_client_packet::Ty> {
				convert(
					Root::PlayToClient,
					Version::V1_17_1,
					Version::V1_19_3,
					&encode(p, old::serialize),
					cx,
				)
				.unwrap()
				.iter()
				.map(|b| decode(b, v1_19_3::play_to_client_packet::deserialize))
				.collect()
			};
		let block = vibration(
			Old::MinecraftBlock(v1_17_1::position::Ty { x: 5, y: 6, z: 7 }),
			"minecraft:block",
		);
		let out = to_1_19_3(&block, &mut cx);
		let [v1_19_3::play_to_client_packet::Ty::WorldParticles(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let v1_19_3::particle_data::Ty::V40(v) = &p.data else {
			panic!("{p:?}")
		};
		assert_eq!(
			(&v.destination, v.ticks),
			(
				&New::MinecraftBlock(v1_19_3::position::Ty { x: 5, y: 6, z: 7 }),
				40
			)
		);
		assert_eq!(up(&block, &mut cx).len(), 1);
		let entity = vibration(Old::MinecraftEntity(9), "minecraft:entity");
		let out = to_1_19_3(&entity, &mut cx);
		let [v1_19_3::play_to_client_packet::Ty::WorldParticles(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let v1_19_3::particle_data::Ty::V40(v) = &p.data else {
			panic!("{p:?}")
		};
		assert_eq!(
			v.destination,
			New::MinecraftEntity(
				v1_19_3::particle_data_v40_destination_minecraft_entity::Ty {
					entity_id: 9,
					entity_eye_height: 0.0
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
			old::Ty::SpawnEntityLiving(v1_17_1::play_to_client_packet_spawn_entity_living::Ty {
				entity_id: 20,
				entity_uuid: 1,
				r#type: entity_id(Version::V1_17_1, "cat"),
				x: 0.0,
				y: 0.0,
				z: 0.0,
				yaw: 0,
				pitch: 0,
				head_pitch: 0,
				velocity: v1_17_1::vec3i16::Ty { x: 0, y: 0, z: 0 },
			});
		let variant = old::Ty::EntityMetadata(v1_17_1::play_to_client_packet_entity_metadata::Ty {
			entity_id: 20,
			metadata: vec![v1_17_1::entity_metadata_items::Ty {
				key: 19,
				r#type: 1,
				value: v1_17_1::entity_metadata_item::Ty::V1(3),
			}],
		});
		let to_1_19 = |p: &old::Ty, cx: &mut Session| -> Vec<v1_19::play_to_client_packet::Ty> {
			convert(
				Root::PlayToClient,
				Version::V1_17_1,
				Version::V1_19,
				&encode(p, old::serialize),
				cx,
			)
			.unwrap()
			.iter()
			.map(|b| decode(b, v1_19::play_to_client_packet::deserialize))
			.collect()
		};
		to_1_19(&cat, &mut cx);
		let out = to_1_19(&variant, &mut cx);
		let [v1_19::play_to_client_packet::Ty::EntityMetadata(m)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			(
				m.metadata[0].key,
				m.metadata[0].r#type,
				&m.metadata[0].value
			),
			(19, 19, &v1_19::entity_metadata_item::Ty::V19(3))
		);
		up(&cat, &mut cx);
		let out = up(&variant, &mut cx);
		let [new::Ty::EntityMetadata(m)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			m.metadata[0].value,
			v1_19_4::entity_metadata_item::Ty::CatVariant(3)
		);
	}

	fn down_client(p: &new::Ty, cx: &mut Session) -> Vec<old::Ty> {
		convert(
			Root::PlayToClient,
			Version::V1_19_4,
			Version::V1_17_1,
			&encode(p, new::serialize),
			cx,
		)
		.unwrap()
		.iter()
		.map(|b| decode(b, old::deserialize))
		.collect()
	}

	fn spawn_new(id: i32, ty: &str, data: i32) -> new::Ty {
		new::Ty::SpawnEntity(v1_19_4::play_to_client_packet_spawn_entity::Ty {
			entity_id: id,
			object_uuid: id as u128,
			r#type: entity_id(Version::V1_19_4, ty),
			x: 4.5,
			y: 70.5,
			z: -2.5,
			pitch: 0,
			yaw: 0,
			head_pitch: 0,
			object_data: data,
			velocity: v1_19_4::vec3i16::Ty { x: 0, y: 0, z: 0 },
		})
	}

	fn metadata_new(
		id: i32,
		key: u8,
		ty: v1_19_4::entity_metadata_items_type::Ty,
		value: v1_19_4::entity_metadata_item::Ty,
	) -> new::Ty {
		new::Ty::EntityMetadata(v1_19_4::play_to_client_packet_entity_metadata::Ty {
			entity_id: id,
			metadata: vec![v1_19_4::entity_metadata_items::Ty {
				key,
				r#type: ty,
				value,
			}],
		})
	}

	#[test]
	fn old_clients_see_paintings_and_new_mobs() {
		use v1_19_4::entity_metadata_item::Ty as Item;
		use v1_19_4::entity_metadata_items_type::Ty as Type;
		let mut cx = Session::new();
		assert!(down_client(&spawn_new(30, "painting", 4), &mut cx).is_empty());
		let out = down_client(
			&metadata_new(30, 8, Type::PaintingVariant, Item::PaintingVariant(29)),
			&mut cx,
		);
		let [old::Ty::SpawnEntityPainting(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			(
				p.entity_id,
				p.title,
				p.direction,
				p.location.x,
				p.location.y,
				p.location.z
			),
			(30, 25, 1, 4, 70, -3)
		);
		let out = down_client(&spawn_new(31, "frog", 0), &mut cx);
		let [old::Ty::SpawnEntityLiving(s)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(s.r#type, entity_id(Version::V1_17_1, "rabbit"));
		let out = down_client(
			&metadata_new(31, 17, Type::FrogVariant, Item::FrogVariant(1)),
			&mut cx,
		);
		let [old::Ty::EntityMetadata(m)] = out.as_slice() else {
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
		let window = new::Ty::WindowItems(v1_19_4::play_to_client_packet_window_items::Ty {
			window_id: 0,
			state_id: 0,
			items: vec![v1_19_4::slot::Ty::True(v1_19_4::slot_true::Ty {
				item_id: mangrove,
				item_count: 2,
				nbt_data: None,
			})],
			carried_item: v1_19_4::slot::Ty::False,
		});
		let out = down_client(&window, &mut cx);
		let [old::Ty::WindowItems(w)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let v1_17_1::slot::Ty::True(s) = &w.items[0] else {
			panic!("{w:?}")
		};
		assert_eq!(
			registry::names(Version::V1_17_1, "items")
				.unwrap()
				.name(s.item_id as u32),
			Some("acacia_planks")
		);
		let cherry = state(Version::V1_19_4, "minecraft:cherry_planks");
		let change = new::Ty::BlockChange(v1_19_4::play_to_client_packet_block_change::Ty {
			location: v1_19_4::position::Ty { x: 0, y: 0, z: 0 },
			r#type: cherry as i32,
		});
		let out = down_client(&change, &mut cx);
		let [old::Ty::BlockChange(b)] = out.as_slice() else {
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
			new::Ty::WorldParticles(v1_19_4::play_to_client_packet_world_particles::Ty {
				particle_id: marker,
				long_distance: false,
				x: 0.0,
				y: 0.0,
				z: 0.0,
				offset_x: 0.0,
				offset_y: 0.0,
				offset_z: 0.0,
				particle_data: 0.0,
				particles: 1,
				data: v1_19_4::particle_data::Ty::V3(state(
					Version::V1_19_4,
					"minecraft:light[level=15,waterlogged=false]",
				) as i32),
			});
		let out = down_client(&particles, &mut cx);
		let [old::Ty::WorldParticles(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(
			registry::names(Version::V1_17_1, "particles")
				.unwrap()
				.name(p.particle_id as u32),
			Some("light")
		);
	}

	#[test]
	fn old_clients_send_sequenced_actions() {
		let mut cx = Session::new();
		let dig = old_server::Ty::BlockDig(v1_17_1::play_to_server_packet_block_dig::Ty {
			status: 0,
			location: v1_17_1::position::Ty { x: 1, y: 2, z: 3 },
			face: 1,
		});
		let sequences: Vec<i32> = (0..2)
			.map(|_| {
				let out = convert(
					Root::PlayToServer,
					Version::V1_17_1,
					Version::V1_19_4,
					&encode(&dig, old_server::serialize),
					&mut cx,
				)
				.unwrap();
				let new_server::Ty::BlockDig(d) = decode(&out[0], new_server::deserialize) else {
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
		let v: Value =
			serde_json::from_str(registry::login_packet(Version::V1_19_2).unwrap()).unwrap();
		let login =
			v1_19_2::play_to_client_packet::Ty::Login(v1_19_2::play_to_client_packet_login::Ty {
				entity_id: 1,
				is_hardcore: false,
				game_mode: 0,
				previous_game_mode: -1,
				world_names: vec!["minecraft:overworld".into()],
				dimension_codec: named("", nbt::json::typed::read(&v["dimensionCodec"]).unwrap()),
				world_type: "minecraft:overworld".into(),
				world_name: "minecraft:overworld".into(),
				hashed_seed: 0,
				max_players: 20,
				view_distance: 8,
				simulation_distance: 6,
				reduced_debug_info: false,
				enable_respawn_screen: true,
				is_debug: false,
				is_flat: false,
				death: None,
			});
		encode(&login, v1_19_2::play_to_client_packet::serialize)
	}

	fn captured() -> Vec<u8> {
		let gz = std::fs::read(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/tests/data/1.19.2-map_chunk.bin.gz"
		))
		.unwrap();
		nbt::gzip::decompress(&gz).unwrap()
	}

	fn down(to: Version) -> Vec<Vec<u8>> {
		let mut cx = Session::new();
		convert(Root::PlayToClient, Version::V1_19_2, to, &login(), &mut cx).unwrap();
		convert(
			Root::PlayToClient,
			Version::V1_19_2,
			to,
			&captured(),
			&mut cx,
		)
		.unwrap()
	}

	#[test]
	fn vanilla_padding_after_the_last_section_is_left_out() {
		let out = down(Version::V1_18_2);
		let [bytes] = out.as_slice() else {
			panic!("{} packets", out.len())
		};
		let v1_18_2::play_to_client_packet::Ty::MapChunk(c) =
			decode(bytes, v1_18_2::play_to_client_packet::deserialize)
		else {
			panic!()
		};
		assert_eq!((c.x, c.z), (-8, 1));
		let mut at = 0;
		for _ in 0..24 {
			at += 2;
			read_container(&c.chunk_data, &mut at, 4096, 4, 8);
			read_container(&c.chunk_data, &mut at, 64, 1, 3);
		}
		assert_eq!(at, c.chunk_data.len());
		let chunks = |packets: Vec<Vec<u8>>, de: fn(&[u8]) -> Option<(i32, i32)>| {
			packets.iter().filter_map(|b| de(b)).collect::<Vec<_>>()
		};
		let at_1_12_2 = chunks(down(Version::V1_12_2), |b| {
			match decode(b, v1_12_2::play_to_client_packet::deserialize) {
				v1_12_2::play_to_client_packet::Ty::MapChunk(c) if c.bit_map != 0 => {
					Some((c.x, c.z))
				}
				_ => None,
			}
		});
		let at_1_8 = chunks(down(Version::V1_8), |b| {
			match decode(b, v1_8::play_to_client_packet::deserialize) {
				v1_8::play_to_client_packet::Ty::MapChunk(c) if c.bit_map != 0 => Some((c.x, c.z)),
				_ => None,
			}
		});
		assert_eq!((at_1_12_2, at_1_8), (vec![(-8, 1)], vec![(-8, 1)]));
	}
}
