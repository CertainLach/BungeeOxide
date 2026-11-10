use minecraft_versions::gen::convert::{
	convert, index_of, v1_12_2, v1_21_11, v1_21_4 as a, v1_21_5, v1_21_8, v1_21_9, v1_8,
	v26_1 as z, Root,
};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::registry;
use minecraft_versions::Session;
use nbt::{Entry, List, ListItems, Payload, Tag};

fn at(v: &str) -> usize {
	index_of(v).unwrap()
}

fn encode<T>(v: &T, ser: fn(&T, &mut Output) -> minecraft_versions::pds::Result<()>) -> Vec<u8> {
	let mut o = Output::new();
	ser(v, &mut o).unwrap();
	o.into_inner()
}

fn decode<T>(b: &[u8], de: fn(&mut Input) -> minecraft_versions::pds::Result<T>) -> T {
	let mut i = Input::new(b);
	let v = de(&mut i).unwrap();
	assert_eq!(i.remaining(), 0, "left over bytes");
	v
}

fn run<A, B>(
	root: Root,
	from: &str,
	to: &str,
	packet: &A,
	ser: fn(&A, &mut Output) -> minecraft_versions::pds::Result<()>,
	de: fn(&mut Input) -> minecraft_versions::pds::Result<B>,
	cx: &mut Session,
) -> Vec<B> {
	convert(root, at(from), at(to), &encode(packet, ser), cx)
		.unwrap()
		.iter()
		.map(|b| decode(b, de))
		.collect()
}

fn up(p: &a::play_toClient_packet::Ty, cx: &mut Session) -> Vec<z::play_toClient_packet::Ty> {
	run(
		Root::PlayToClient,
		"1.21.4",
		"26.1",
		p,
		a::play_toClient_packet::serialize,
		z::play_toClient_packet::deserialize,
		cx,
	)
}

fn down(p: &z::play_toServer_packet::Ty, cx: &mut Session) -> Vec<a::play_toServer_packet::Ty> {
	run(
		Root::PlayToServer,
		"26.1",
		"1.21.4",
		p,
		z::play_toServer_packet::serialize,
		a::play_toServer_packet::deserialize,
		cx,
	)
}

fn compound(entries: Vec<(&str, Payload)>) -> Payload {
	Payload::compound(entries.into_iter().map(|(n, v)| Entry::new(n, v)).collect())
}

fn tag(p: Payload) -> Tag {
	Tag::new(p)
}

fn s(v: &str) -> Payload {
	Payload::string(v.into())
}

fn get<'a>(p: &'a Payload, path: &[&str]) -> Option<&'a Payload> {
	path.iter().try_fold(p, |p, k| p.field(k))
}

fn id(version: &str, kind: &str, name: &str) -> i32 {
	registry::names(version, kind).unwrap().id(name).unwrap() as i32
}

fn item(
	version: &str,
	name: &str,
	count: i32,
	components: Vec<a::SlotComponent::Ty>,
) -> a::Slot::Ty {
	a::Slot::Ty::default(
		count,
		a::Slot__default::Ty {
			itemId: id(version, "items", name),
			addedComponentCount: components.len() as i32,
			removedComponentCount: 0,
			components,
			removeComponents: vec![],
		},
	)
}

fn sample_config() -> Vec<a::configuration_toClient_packet::Ty> {
	use a::configuration_toClient_packet as c4;
	let registry = |id: &str, entries: Vec<(&str, Payload)>| {
		c4::Ty::registry_data(a::configuration_toClient_packet_registry_data::Ty {
			id: id.into(),
			entries: entries
				.into_iter()
				.map(
					|(k, v)| a::configuration_toClient_packet_registry_data__entries__items::Ty {
						key: k.into(),
						value: Some(tag(v)),
					},
				)
				.collect(),
		})
	};
	let overworld = compound(vec![
		("min_y", Payload::int(0)),
		("height", Payload::int(256)),
		("effects", s("minecraft:overworld")),
		("natural", Payload::byte(1)),
		("ambient_light", Payload::float(0.0)),
		("coordinate_scale", Payload::double(1.0)),
	]);
	let plains = compound(vec![(
		"effects",
		compound(vec![
			("sky_color", Payload::int(7907327)),
			("fog_color", Payload::int(12638463)),
			("water_color", Payload::int(4159204)),
		]),
	)]);
	let wolf = compound(vec![
		("wild_texture", s("minecraft:entity/wolf/wolf")),
		("tame_texture", s("minecraft:entity/wolf/wolf_tame")),
		("angry_texture", s("minecraft:entity/wolf/wolf_angry")),
		("biomes", s("minecraft:taiga")),
	]);
	vec![
		registry(
			"minecraft:dimension_type",
			vec![("minecraft:overworld", overworld)],
		),
		registry(
			"minecraft:worldgen/biome",
			vec![("minecraft:plains", plains)],
		),
		registry("minecraft:wolf_variant", vec![("minecraft:pale", wolf)]),
		c4::Ty::finish_configuration(a::configuration_toClient_packet_finish_configuration::Ty {}),
	]
}

#[test]
fn configuration_registries_reach_26_1() {
	use a::configuration_toClient_packet as c4;
	use z::configuration_toClient_packet as c26;
	let mut cx = Session::new();
	let mut out = Vec::new();
	for p in &sample_config() {
		out.extend(run(
			Root::ConfigurationToClient,
			"1.21.4",
			"26.1",
			p,
			c4::serialize,
			c26::deserialize,
			&mut cx,
		));
	}
	assert!(matches!(out.last(), Some(c26::Ty::finish_configuration(_))));
	let registries: Vec<&z::configuration_toClient_packet_registry_data::Ty> = out
		.iter()
		.filter_map(|p| match p {
			c26::Ty::registry_data(r) => Some(r),
			_ => None,
		})
		.collect();
	let find = |id: &str| {
		registries
			.iter()
			.find(|r| r.id == format!("minecraft:{id}"))
			.unwrap_or_else(|| panic!("no {id}"))
	};
	let entry = |id: &str, key: &str| -> Payload {
		find(id)
			.entries
			.iter()
			.find(|e| e.key == key)
			.and_then(|e| e.value.clone())
			.map(|t| t.value)
			.unwrap()
	};
	let dim = entry("dimension_type", "minecraft:overworld");
	assert_eq!(get(&dim, &["height"]), Some(&Payload::int(256)));
	assert_eq!(
		get(&dim, &["default_clock"]),
		Some(&s("minecraft:overworld"))
	);
	assert_eq!(
		get(&dim, &["attributes", "visual/cloud_height"]),
		Some(&Payload::float(192.0))
	);
	assert!(get(&dim, &["attributes", "visual/ambient_light_color"]).is_some());
	assert!(matches!(get(&dim, &["timelines"]), Some(Payload::list(_))));
	let biome = entry("worldgen/biome", "minecraft:plains");
	assert_eq!(
		get(&biome, &["attributes", "visual/sky_color"]),
		Some(&Payload::int(7907327))
	);
	let wolf = entry("wolf_variant", "minecraft:pale");
	assert_eq!(
		get(&wolf, &["assets", "wild"]),
		Some(&s("minecraft:entity/wolf/wolf"))
	);
	assert_eq!(
		get(&wolf, &["baby_assets", "tame"]),
		Some(&s("minecraft:entity/wolf/wolf_tame_baby"))
	);
	let frog = entry("frog_variant", "minecraft:temperate");
	assert_eq!(
		get(&frog, &["asset_id"]),
		Some(&s("minecraft:entity/frog/frog_temperate"))
	);
	let pig = entry("pig_variant", "minecraft:temperate");
	assert_eq!(
		get(&pig, &["baby_asset_id"]),
		Some(&s("minecraft:entity/pig/pig_temperate_baby"))
	);
	let sounds = entry("wolf_sound_variant", "minecraft:classic");
	assert_eq!(
		get(&sounds, &["adult_sounds", "step_sound"]),
		Some(&s("entity.wolf.step"))
	);
	assert_eq!(find("chicken_variant").entries.len(), 3);
	for id in [
		"zombie_nautilus_variant",
		"timeline",
		"world_clock",
		"cat_sound_variant",
		"pig_sound_variant",
		"jukebox_song",
		"trim_material",
	] {
		assert!(!find(id).entries.is_empty(), "{id}");
	}
	let day = entry("timeline", "minecraft:day");
	assert_eq!(get(&day, &["clock"]), Some(&s("overworld")));
	assert!(out.iter().any(
		|p| matches!(p, c26::Ty::tags(t) if t.iter().any(|r| r.tagType.ends_with("damage_type")))
	));
}

fn section_1_21_4(states: &[u32]) -> Vec<u8> {
	let mut out = vec![];
	out.extend_from_slice(&4096i16.to_be_bytes());
	out.push(4);
	out.push(states.len() as u8);
	for s in states {
		let mut v = *s;
		loop {
			if v < 0x80 {
				out.push(v as u8);
				break;
			}
			out.push((v as u8 & 0x7f) | 0x80);
			v >>= 7;
		}
	}
	out.extend_from_slice(&[0x80, 0x02]);
	for i in 0..256u64 {
		let mut word = 0u64;
		for j in 0..16u64 {
			word |= ((i * 16 + j) % states.len() as u64) << (j * 4);
		}
		out.extend_from_slice(&word.to_be_bytes());
	}
	out.extend_from_slice(&[0, 0, 0]);
	out
}

fn read_varint(d: &[u8], at: &mut usize) -> u32 {
	let mut v = 0;
	for shift in (0..35).step_by(7) {
		let b = d[*at];
		*at += 1;
		v |= ((b & 0x7f) as u32) << shift;
		if b & 0x80 == 0 {
			break;
		}
	}
	v
}

#[test]
fn chunk_sections_reach_26_1() {
	let blocks = registry::blocks("1.21.4").unwrap();
	let names = [
		"minecraft:stone",
		"minecraft:water[level=0]",
		"minecraft:oak_stairs[facing=east,half=top,shape=straight,waterlogged=true]",
		"minecraft:chain[axis=y,waterlogged=false]",
	];
	let states: Vec<u32> = names
		.iter()
		.map(|n| blocks.parse_state(n).unwrap())
		.collect();
	let heightmaps = tag(compound(vec![(
		"MOTION_BLOCKING",
		Payload::longArray(vec![7; 37]),
	)]));
	let packet = a::play_toClient_packet::Ty::map_chunk(a::play_toClient_packet_map_chunk::Ty {
		x: 1,
		z: 2,
		heightmaps,
		chunkData: section_1_21_4(&states),
		blockEntities: vec![],
		skyLightMask: vec![],
		blockLightMask: vec![],
		emptySkyLightMask: vec![],
		emptyBlockLightMask: vec![],
		skyLight: vec![],
		blockLight: vec![],
	});
	let out = up(&packet, &mut Session::new());
	let [z::play_toClient_packet::Ty::map_chunk(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(c.heightmaps.len(), 1);
	let d = &c.chunkData;
	assert_eq!(i16::from_be_bytes([d[0], d[1]]), 4096);
	assert_eq!(i16::from_be_bytes([d[2], d[3]]), 2048);
	assert_eq!(d[4], 4);
	let mut at = 5;
	let n = read_varint(d, &mut at);
	let palette: Vec<u32> = (0..n).map(|_| read_varint(d, &mut at)).collect();
	let new = registry::blocks("26.1").unwrap();
	let described: Vec<String> = palette.iter().map(|p| new.describe(*p).unwrap()).collect();
	assert_eq!(described[0], "minecraft:stone");
	assert_eq!(described[1], "minecraft:water[level=0]");
	assert!(described[2].starts_with("minecraft:oak_stairs[facing=east,half=top"));
	assert_eq!(
		described[3],
		"minecraft:iron_chain[axis=y,waterlogged=false]"
	);
	assert_eq!(d.len(), at + 256 * 8 + 2);
}

#[test]
fn hidden_tooltips_become_tooltip_display() {
	use a::SlotComponent::Ty as C;
	let enchantments = C::enchantments(a::SlotComponent__enchantments::Ty {
		enchantments: vec![a::SlotComponent__enchantments__enchantments__items::Ty {
			id: 9,
			level: 3,
		}],
		showTooltip: false,
	});
	let name = C::custom_name(tag(compound(vec![
		("text", s("hi")),
		(
			"clickEvent",
			compound(vec![("action", s("run_command")), ("value", s("/spawn"))]),
		),
	])));
	let sword = item(
		"1.21.4",
		"diamond_sword",
		1,
		vec![enchantments, C::hide_additional_tooltip, name],
	);
	let packet = a::play_toClient_packet::Ty::set_slot(a::play_toClient_packet_set_slot::Ty {
		windowId: 0,
		stateId: 1,
		slot: 36,
		item: sword,
	});
	let out = run(
		Root::PlayToClient,
		"1.21.4",
		"1.21.5",
		&packet,
		a::play_toClient_packet::serialize,
		v1_21_5::play_toClient_packet::deserialize,
		&mut Session::new(),
	);
	let [v1_21_5::play_toClient_packet::Ty::set_slot(s5)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let v1_21_5::Slot::Ty::default(1, d) = &s5.item else {
		panic!()
	};
	assert_eq!(d.itemId, id("1.21.5", "items", "diamond_sword"));
	assert_eq!(d.addedComponentCount as usize, d.components.len());
	let display = d
		.components
		.iter()
		.find_map(|c| match c {
			v1_21_5::SlotComponent::Ty::tooltip_display(t) => Some(t),
			_ => None,
		})
		.unwrap();
	assert!(!display.hideTooltip);
	assert!(display.hiddenComponents.contains(&10));
	assert!(display.hiddenComponents.contains(&66));
	let name = d
		.components
		.iter()
		.find_map(|c| match c {
			v1_21_5::SlotComponent::Ty::custom_name(t) => Some(t),
			_ => None,
		})
		.unwrap();
	assert_eq!(
		get(&name.value, &["click_event", "command"]),
		Some(&s("/spawn"))
	);
}

#[test]
fn containers_become_templates_and_clicks_recover_items() {
	use a::SlotComponent::Ty as C;
	let mut cx = Session::new();
	let stones = C::container(vec![item("1.21.4", "stone", 3, vec![]), a::Slot::Ty::v0]);
	let shulker = item("1.21.4", "shulker_box", 1, vec![stones]);
	let packet =
		a::play_toClient_packet::Ty::window_items(a::play_toClient_packet_window_items::Ty {
			windowId: 1,
			stateId: 5,
			items: vec![shulker.clone(), a::Slot::Ty::v0],
			carriedItem: a::Slot::Ty::v0,
		});
	let out = up(&packet, &mut cx);
	let [z::play_toClient_packet::Ty::window_items(w)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let z::Slot::Ty::default(1, d) = &w.items[0] else {
		panic!()
	};
	let container = d
		.components
		.iter()
		.find_map(|c| match c {
			z::SlotComponent::Ty::container(v) => Some(v),
			_ => None,
		})
		.unwrap();
	assert_eq!(container.len(), 2);
	let first = container[0].as_ref().unwrap();
	assert_eq!(
		(first.itemId, first.itemCount),
		(id("26.1", "items", "stone"), 3)
	);
	assert!(container[1].is_none());
	let hashed = |v: &z::Slot::Ty| match v {
		z::Slot::Ty::default(count, d) => z::HashedSlot::Ty {
			itemId: d.itemId,
			itemCount: *count,
			components: d
				.components
				.iter()
				.map(|c| {
					let bytes = encode(c, z::SlotComponent::serialize);
					z::HashedSlot__components__items::Ty {
						r#type: decode(&bytes[..1], z::SlotComponentType::deserialize),
						hash: 0,
					}
				})
				.collect(),
			removeComponents: vec![],
		},
		z::Slot::Ty::v0 => unreachable!(),
	};
	let click =
		z::play_toServer_packet::Ty::window_click(z::play_toServer_packet_window_click::Ty {
			windowId: 1,
			stateId: 5,
			slot: 0,
			mouseButton: 0,
			mode: 0,
			changedSlots: vec![
				z::play_toServer_packet_window_click__changedSlots__items::Ty {
					location: 0,
					item: None,
				},
			],
			cursorItem: Some(hashed(&w.items[0])),
		});
	let out = down(&click, &mut cx);
	let [a::play_toServer_packet::Ty::window_click(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(c.cursorItem, shulker);
	assert_eq!(c.changedSlots[0].item, a::Slot::Ty::v0);
}

#[test]
fn creative_items_are_decoded_down() {
	let name = tag(compound(vec![("text", s("named"))]));
	let mut data = Output::new();
	nbt::gen::pds_Tag::serialize(&name, &mut data).unwrap();
	let ty = decode(&[6], z::SlotComponentType::deserialize);
	let slot = z::UntrustedSlot::Ty {
		itemCount: 2,
		ANON: z::UntrustedSlot__ANON::Ty::default(
			2,
			z::UntrustedSlot__ANON__default::Ty {
				itemId: id("26.1", "items", "golden_dandelion"),
				addedComponentCount: 1,
				removedComponentCount: 0,
				components: vec![z::UntrustedSlotComponent::Ty {
					r#type: ty,
					data: data.into_inner(),
				}],
				removeComponents: vec![],
			},
		),
	};
	let p = z::play_toServer_packet::Ty::set_creative_slot(
		z::play_toServer_packet_set_creative_slot::Ty {
			slot: 36,
			item: slot,
		},
	);
	let out = down(&p, &mut Session::new());
	let [a::play_toServer_packet::Ty::set_creative_slot(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let a::Slot::Ty::default(2, d) = &c.item else {
		panic!("{:?}", c.item)
	};
	assert!(
		matches!(d.components.as_slice(), [a::SlotComponent::Ty::custom_name(t)] if get(&t.value, &["text"]) == Some(&s("named")))
	);
}

fn lp(bytes: &[u8]) -> (f64, f64, f64) {
	if bytes[0] == 0 {
		return (0.0, 0.0, 0.0);
	}
	let q = (u32::from_be_bytes(bytes[2..6].try_into().unwrap()) as u64) << 16
		| (bytes[1] as u64) << 8
		| bytes[0] as u64;
	let mut scale = (bytes[0] & 3) as u64;
	if bytes[0] & 4 != 0 {
		let mut at = 6;
		scale |= (read_varint(bytes, &mut at) as u64) << 2;
	}
	let unpack = |v: u64| ((v & 32767) as f64).min(32766.0) * 2.0 / 32766.0 - 1.0;
	(
		unpack(q >> 3) * scale as f64,
		unpack(q >> 18) * scale as f64,
		unpack(q >> 33) * scale as f64,
	)
}

#[test]
fn spawn_velocity_uses_lp_vec3() {
	let mut cx = Session::new();
	let packet = v1_21_8::play_toClient_packet::Ty::spawn_entity(
		v1_21_8::play_toClient_packet_spawn_entity::Ty {
			entityId: 7,
			objectUUID: 1,
			r#type: id("1.21.8", "entities", "arrow"),
			x: 0.0,
			y: 64.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			headPitch: 0,
			objectData: 0,
			velocity: v1_21_8::vec3i16::Ty {
				x: 8000,
				y: -4000,
				z: 30000,
			},
		},
	);
	let out = run(
		Root::PlayToClient,
		"1.21.8",
		"1.21.9",
		&packet,
		v1_21_8::play_toClient_packet::serialize,
		v1_21_9::play_toClient_packet::deserialize,
		&mut cx,
	);
	let [v1_21_9::play_toClient_packet::Ty::spawn_entity(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let (x, y, z) = lp(&s.velocity);
	assert!(
		(x - 1.0).abs() < 1e-3 && (y + 0.5).abs() < 1e-3 && (z - 3.75).abs() < 1e-3,
		"{x} {y} {z}"
	);
	let back = run(
		Root::PlayToClient,
		"1.21.9",
		"1.21.8",
		&out[0],
		v1_21_9::play_toClient_packet::serialize,
		v1_21_8::play_toClient_packet::deserialize,
		&mut cx,
	);
	let [v1_21_8::play_toClient_packet::Ty::spawn_entity(b)] = back.as_slice() else {
		panic!("{back:?}")
	};
	assert!(
		(b.velocity.x - 8000).abs() <= 2
			&& (b.velocity.y + 4000).abs() <= 2
			&& (b.velocity.z - 30000).abs() <= 8,
		"{:?}",
		b.velocity
	);
	assert_eq!(s.r#type, id("1.21.9", "entities", "arrow"));
	let still = v1_21_8::play_toClient_packet::Ty::entity_velocity(
		v1_21_8::play_toClient_packet_entity_velocity::Ty {
			entityId: 7,
			velocity: v1_21_8::vec3i16::Ty { x: 0, y: 0, z: 0 },
		},
	);
	let out = run(
		Root::PlayToClient,
		"1.21.8",
		"1.21.9",
		&still,
		v1_21_8::play_toClient_packet::serialize,
		v1_21_9::play_toClient_packet::deserialize,
		&mut cx,
	);
	assert!(
		matches!(out.as_slice(), [v1_21_9::play_toClient_packet::Ty::entity_velocity(v)] if v.velocity == vec![0])
	);
}

fn spawn(entity: i32, kind: &str) -> a::play_toClient_packet::Ty {
	a::play_toClient_packet::Ty::spawn_entity(a::play_toClient_packet_spawn_entity::Ty {
		entityId: entity,
		objectUUID: entity as u128,
		r#type: id("1.21.4", "entities", kind),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		pitch: 0,
		yaw: 0,
		headPitch: 0,
		objectData: 0,
		velocity: a::vec3i16::Ty { x: 0, y: 0, z: 0 },
	})
}

#[test]
fn pig_metadata_moves_saddle_and_age_index() {
	let mut cx = Session::new();
	let out = up(&spawn(3, "pig"), &mut cx);
	assert!(
		matches!(out.as_slice(), [z::play_toClient_packet::Ty::spawn_entity(s)] if s.r#type == id("26.1", "entities", "pig"))
	);
	use a::entityMetadataEntry__value::Ty as V;
	let meta =
		a::play_toClient_packet::Ty::entity_metadata(a::play_toClient_packet_entity_metadata::Ty {
			entityId: 3,
			metadata: vec![
				a::entityMetadataEntry::Ty {
					key: 0,
					value: V::byte(0),
				},
				a::entityMetadataEntry::Ty {
					key: 17,
					value: V::boolean(true),
				},
				a::entityMetadataEntry::Ty {
					key: 18,
					value: V::int(40),
				},
			],
		});
	let out = up(&meta, &mut cx);
	let [z::play_toClient_packet::Ty::entity_metadata(m), z::play_toClient_packet::Ty::entity_equipment(e)] =
		out.as_slice()
	else {
		panic!("{out:?}")
	};
	let keys: Vec<u8> = m.metadata.iter().map(|e| e.key).collect();
	assert_eq!(keys, vec![0, 18]);
	assert_eq!(e.equipments[0].slot, 7);
	assert!(
		matches!(&e.equipments[0].item, z::Slot::Ty::default(1, d) if d.itemId == id("26.1", "items", "saddle"))
	);
}

fn login(entity: i32) -> a::play_toClient_packet::Ty {
	a::play_toClient_packet::Ty::login(a::play_toClient_packet_login::Ty {
		entityId: entity,
		isHardcore: false,
		worldNames: vec!["minecraft:overworld".into()],
		maxPlayers: 20,
		viewDistance: 8,
		simulationDistance: 8,
		reducedDebugInfo: false,
		enableRespawnScreen: true,
		doLimitedCrafting: false,
		worldState: a::play_toClient_SpawnInfo::Ty {
			dimension: 0,
			name: "minecraft:overworld".into(),
			hashedSeed: 0,
			gamemode: a::play_toClient_SpawnInfo__gamemode::Ty::survival,
			previousGamemode: 255,
			isDebug: false,
			isFlat: false,
			death: None,
			portalCooldown: 0,
			seaLevel: 63,
		},
		enforcesSecureChat: false,
	})
}

#[test]
fn sneaking_input_becomes_entity_action() {
	let mut cx = Session::new();
	up(&login(42), &mut cx);
	let input = |shift| {
		z::play_toServer_packet::Ty::player_input(z::play_toServer_packet_player_input::Ty {
			forward: false,
			backward: false,
			left: false,
			right: false,
			jump: false,
			shift,
			sprint: false,
		})
	};
	let out = down(&input(true), &mut cx);
	let [a::play_toServer_packet::Ty::entity_action(e), a::play_toServer_packet::Ty::player_input(_)] =
		out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!(e.entityId, 42);
	assert!(matches!(
		e.actionId,
		a::play_toServer_packet_entity_action__actionId::Ty::start_sneaking
	));
	assert_eq!(down(&input(true), &mut cx).len(), 1);
	let out = down(&input(false), &mut cx);
	assert!(
		matches!(&out[0], a::play_toServer_packet::Ty::entity_action(e) if matches!(e.actionId, a::play_toServer_packet_entity_action__actionId::Ty::stop_sneaking))
	);
}

#[test]
fn time_becomes_clock_updates() {
	let p = a::play_toClient_packet::Ty::update_time(a::play_toClient_packet_update_time::Ty {
		age: 100,
		time: 6000,
		tickDayTime: true,
	});
	let out = up(&p, &mut Session::new());
	let [z::play_toClient_packet::Ty::update_time(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(t.age, 100);
	assert_eq!(t.clockUpdates.len(), 2);
	assert_eq!(
		(t.clockUpdates[0].totalTicks, t.clockUpdates[0].rate),
		(6000, 1.0)
	);
}

fn sample_play() -> Vec<a::play_toClient_packet::Ty> {
	use a::play_toClient_packet::Ty as P;
	use a::SlotComponent::Ty as C;
	let blocks = registry::blocks("1.21.4").unwrap();
	let state = |n: &str| blocks.parse_state(n).unwrap() as i32;
	let pos = a::position::Ty { x: 1, z: 2, y: 64 };
	let sword = item(
		"1.21.4",
		"diamond_sword",
		1,
		vec![
			C::enchantments(a::SlotComponent__enchantments::Ty {
				enchantments: vec![a::SlotComponent__enchantments__enchantments__items::Ty {
					id: 1,
					level: 2,
				}],
				showTooltip: true,
			}),
			C::custom_name(tag(compound(vec![("text", s("Blade"))]))),
			C::lore(vec![Some(tag(compound(vec![("text", s("line"))]))), None]),
			C::unbreakable(false),
			C::hide_tooltip,
		],
	);
	let sign = compound(vec![(
		"front_text",
		compound(vec![(
			"messages",
			Payload::list(List::new(ListItems::string(vec![
				"{\"text\":\"a\"}".into(),
				"\"b\"".into(),
				"\"\"".into(),
				"\"\"".into(),
			]))),
		)]),
	)]);
	let mut out = vec![
		login(1),
		spawn(5, "pig"),
		spawn(6, "falling_block"),
		P::map_chunk(a::play_toClient_packet_map_chunk::Ty {
			x: 0,
			z: 0,
			heightmaps: tag(compound(vec![
				("MOTION_BLOCKING", Payload::longArray(vec![1; 37])),
				("WORLD_SURFACE", Payload::longArray(vec![2; 37])),
			])),
			chunkData: [section_1_21_4(&[1, 9, 2000]), section_1_21_4(&[1])].concat(),
			blockEntities: vec![a::chunkBlockEntity::Ty {
				ANON: a::chunkBlockEntity__ANON::Ty { x: 1, z: 2 },
				y: 64,
				r#type: 7,
				nbtData: Some(tag(sign)),
			}],
			skyLightMask: vec![],
			blockLightMask: vec![],
			emptySkyLightMask: vec![],
			emptyBlockLightMask: vec![],
			skyLight: vec![],
			blockLight: vec![],
		}),
		P::set_slot(a::play_toClient_packet_set_slot::Ty {
			windowId: 0,
			stateId: 1,
			slot: 36,
			item: sword.clone(),
		}),
		P::window_items(a::play_toClient_packet_window_items::Ty {
			windowId: 0,
			stateId: 2,
			items: vec![a::Slot::Ty::v0, sword],
			carriedItem: a::Slot::Ty::v0,
		}),
		P::block_change(a::play_toClient_packet_block_change::Ty {
			location: pos.clone(),
			r#type: state("minecraft:oak_log[axis=x]"),
		}),
		P::multi_block_change(a::play_toClient_packet_multi_block_change::Ty {
			chunkCoordinates: a::play_toClient_packet_multi_block_change__chunkCoordinates::Ty {
				x: 0,
				z: 0,
				y: 4,
			},
			records: vec![state("minecraft:glass") << 12 | 0x123],
		}),
		P::world_event(a::play_toClient_packet_world_event::Ty {
			effectId: 2001,
			location: pos.clone(),
			data: state("minecraft:stone"),
			global: false,
		}),
		P::world_particles(a::play_toClient_packet_world_particles::Ty {
			longDistance: false,
			alwaysShow: false,
			x: 0.0,
			y: 64.0,
			z: 0.0,
			offsetX: 0.0,
			offsetY: 0.0,
			offsetZ: 0.0,
			velocityOffset: 0.0,
			amount: 3,
			particle: a::Particle::Ty::block(state("minecraft:dirt")),
		}),
		P::sound_effect(a::play_toClient_packet_sound_effect::Ty {
			sound: a::ItemSoundHolder::Ty::Id(
				registry::names("1.21.4", "sounds")
					.unwrap()
					.id("entity.pig.ambient")
					.unwrap() as i32
					- 1,
			),
			soundCategory: a::soundSource::Ty::neutral,
			x: 0,
			y: 512,
			z: 0,
			volume: 1.0,
			pitch: 1.0,
			seed: 1,
		}),
		P::spawn_position(a::play_toClient_packet_spawn_position::Ty {
			location: pos.clone(),
			angle: 90.0,
		}),
		P::explosion(a::play_toClient_packet_explosion::Ty {
			x: 0.0,
			y: 64.0,
			z: 0.0,
			playerKnockback: None,
			explosionParticle: a::Particle::Ty::explosion,
			sound: a::ItemSoundHolder::Ty::Id(
				registry::names("1.21.4", "sounds")
					.unwrap()
					.id("entity.generic.explode")
					.unwrap() as i32
					- 1,
			),
		}),
		P::update_time(a::play_toClient_packet_update_time::Ty {
			age: 10,
			time: 1000,
			tickDayTime: true,
		}),
		P::entity_velocity(a::play_toClient_packet_entity_velocity::Ty {
			entityId: 5,
			velocity: a::vec3i16::Ty {
				x: 100,
				y: 2000,
				z: -300,
			},
		}),
		P::entity_status(a::play_toClient_packet_entity_status::Ty {
			entityId: 1,
			entityStatus: 29,
		}),
		P::spawn_entity_experience_orb(a::play_toClient_packet_spawn_entity_experience_orb::Ty {
			entityId: 9,
			x: 1.0,
			y: 64.0,
			z: 1.0,
			count: 7,
		}),
		P::system_chat(a::play_toClient_packet_system_chat::Ty {
			content: tag(compound(vec![
				("text", s("hi")),
				(
					"hoverEvent",
					compound(vec![("action", s("show_text")), ("contents", s("x"))]),
				),
			])),
			isActionBar: false,
		}),
		P::teams(a::play_toClient_packet_teams::Ty {
			team: "t".into(),
			mode: a::play_toClient_packet_teams__mode::Ty::v0(
				a::play_toClient_packet_teams__mode__v0::Ty {
					name: tag(s("team")),
					friendlyFire: 1,
					nameTagVisibility: "hideForOtherTeams".into(),
					collisionRule: "never".into(),
					formatting: 0,
					prefix: tag(s("[p] ")),
					suffix: tag(s("")),
					players: vec!["a".into()],
				},
			),
		}),
	];
	use a::entityMetadataEntry__value::Ty as V;
	out.push(P::entity_metadata(
		a::play_toClient_packet_entity_metadata::Ty {
			entityId: 5,
			metadata: vec![
				a::entityMetadataEntry::Ty {
					key: 2,
					value: V::optional_component(Some(tag(s("Pig")))),
				},
				a::entityMetadataEntry::Ty {
					key: 17,
					value: V::boolean(true),
				},
				a::entityMetadataEntry::Ty {
					key: 18,
					value: V::int(3),
				},
			],
		},
	));
	out
}

#[test]
#[ignore]
fn write_dumps() {
	let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/late-dumps");
	std::fs::create_dir_all(&dir).unwrap();
	let frame = |packets: &[Vec<u8>]| {
		let mut bytes = Vec::new();
		for p in packets {
			bytes.extend((p.len() as u32).to_be_bytes());
			bytes.extend(p);
		}
		bytes
	};
	for version in ["1.21.5", "1.21.6", "1.21.8", "1.21.9", "1.21.11", "26.1"] {
		let mut cx = Session::new();
		let mut play = Vec::new();
		for p in sample_play() {
			play.extend(
				convert(
					Root::PlayToClient,
					at("1.21.4"),
					at(version),
					&encode(&p, a::play_toClient_packet::serialize),
					&mut cx,
				)
				.unwrap(),
			);
		}
		std::fs::write(dir.join(format!("{version}.bin")), frame(&play)).unwrap();
		let mut config = Vec::new();
		for p in sample_config() {
			let data = encode(&p, a::configuration_toClient_packet::serialize);
			config.extend(
				convert(
					Root::ConfigurationToClient,
					at("1.21.4"),
					at(version),
					&data,
					&mut cx,
				)
				.unwrap(),
			);
		}
		std::fs::write(
			dir.join(format!("{version}-configuration.bin")),
			frame(&config),
		)
		.unwrap();
		let mut cx = Session::new();
		let mut back = Vec::new();
		for p in &config {
			back.extend(
				convert(
					Root::ConfigurationToClient,
					at(version),
					at("1.21.4"),
					p,
					&mut cx,
				)
				.unwrap(),
			);
		}
		std::fs::write(
			dir.join(format!("{version}-configuration-down.bin")),
			frame(&back),
		)
		.unwrap();
		let mut back = Vec::new();
		for p in &play {
			back.extend(
				convert(Root::PlayToClient, at(version), at("1.21.4"), p, &mut cx).unwrap(),
			);
		}
		std::fs::write(dir.join(format!("{version}-down.bin")), frame(&back)).unwrap();
	}
}

#[test]
fn sample_packets_convert_to_every_version() {
	for version in ["1.21.5", "1.21.6", "1.21.8", "1.21.9", "1.21.11", "26.1"] {
		let mut cx = Session::new();
		for p in sample_play() {
			let out = convert(
				Root::PlayToClient,
				at("1.21.4"),
				at(version),
				&encode(&p, a::play_toClient_packet::serialize),
				&mut cx,
			)
			.unwrap_or_else(|e| panic!("{version}: {e} for {p:?}"));
			for b in out {
				minecraft_versions::gen::convert::inspect(Root::PlayToClient, at(version), &b)
					.unwrap_or_else(|e| panic!("{version}: {e}"));
			}
		}
	}
}

#[test]
fn chat_click_events_are_renamed() {
	let content = tag(compound(vec![
		("text", s("click")),
		(
			"clickEvent",
			compound(vec![
				("action", s("open_url")),
				("value", s("https://example.com")),
			]),
		),
		(
			"hoverEvent",
			compound(vec![
				("action", s("show_text")),
				("contents", compound(vec![("text", s("tip"))])),
			]),
		),
		(
			"extra",
			Payload::list(List::new(ListItems::compound(vec![vec![Entry::new(
				"clickEvent",
				compound(vec![("action", s("change_page")), ("value", s("2"))]),
			)]]))),
		),
	]));
	let p = a::play_toClient_packet::Ty::system_chat(a::play_toClient_packet_system_chat::Ty {
		content,
		isActionBar: false,
	});
	let out = up(&p, &mut Session::new());
	let [z::play_toClient_packet::Ty::system_chat(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		get(&c.content.value, &["click_event", "url"]),
		Some(&s("https://example.com"))
	);
	assert_eq!(
		get(&c.content.value, &["hover_event", "value", "text"]),
		Some(&s("tip"))
	);
	let Some(Payload::list(List {
		items: ListItems::compound(extra),
		..
	})) = get(&c.content.value, &["extra"])
	else {
		panic!()
	};
	assert_eq!(
		get(
			&Payload::compound(extra[0].clone()),
			&["click_event", "page"]
		),
		Some(&Payload::int(2))
	);
}

fn crc32c(data: &[u8]) -> i32 {
	let mut c = !0u32;
	for b in data {
		c ^= *b as u32;
		for _ in 0..8 {
			c = if c & 1 != 0 {
				c >> 1 ^ 0x82F6_3B78
			} else {
				c >> 1
			};
		}
	}
	!c as i32
}

fn hash_int(v: i32) -> i32 {
	let mut b = vec![8];
	b.extend(v.to_le_bytes());
	crc32c(&b)
}

fn hash_str(s: &str) -> i32 {
	let units: Vec<u16> = s.encode_utf16().collect();
	let mut b = vec![12];
	b.extend((units.len() as i32).to_le_bytes());
	units.iter().for_each(|u| b.extend(u.to_le_bytes()));
	crc32c(&b)
}

fn hash_map(entries: &[(i32, i32)]) -> i32 {
	let mut e: Vec<(u32, u32)> = entries
		.iter()
		.map(|(k, v)| (*k as u32, *v as u32))
		.collect();
	e.sort();
	let mut b = vec![2];
	for (k, v) in e {
		b.extend(k.to_le_bytes());
		b.extend(v.to_le_bytes());
	}
	b.push(3);
	crc32c(&b)
}

fn enchantment_registry(cx: &mut Session) {
	let p = v1_21_5::configuration_toClient_packet::Ty::registry_data(
		v1_21_5::configuration_toClient_packet_registry_data::Ty {
			id: "minecraft:enchantment".into(),
			entries: ["minecraft:protection", "minecraft:sharpness"]
				.into_iter()
				.map(
					|k| v1_21_5::configuration_toClient_packet_registry_data__entries__items::Ty {
						key: k.into(),
						value: None,
					},
				)
				.collect(),
		},
	);
	run(
		Root::ConfigurationToClient,
		"1.21.5",
		"1.21.4",
		&p,
		v1_21_5::configuration_toClient_packet::serialize,
		a::configuration_toClient_packet::deserialize,
		cx,
	);
}

fn click(item: a::Slot::Ty, cx: &mut Session) -> v1_21_5::play_toServer_packet_window_click::Ty {
	let p = a::play_toServer_packet::Ty::window_click(a::play_toServer_packet_window_click::Ty {
		windowId: 0,
		stateId: 1,
		slot: 36,
		mouseButton: 0,
		mode: 0,
		changedSlots: vec![
			a::play_toServer_packet_window_click__changedSlots__items::Ty {
				location: 36,
				item: a::Slot::Ty::v0,
			},
		],
		cursorItem: item,
	});
	let out = run(
		Root::PlayToServer,
		"1.21.4",
		"1.21.5",
		&p,
		a::play_toServer_packet::serialize,
		v1_21_5::play_toServer_packet::deserialize,
		cx,
	);
	let [v1_21_5::play_toServer_packet::Ty::window_click(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	c.clone()
}

fn hashes(c: &v1_21_5::play_toServer_packet_window_click::Ty) -> Vec<(String, i32)> {
	let slot = c.cursorItem.as_ref().expect("cursor item");
	let mut out: Vec<(String, i32)> = slot
		.components
		.iter()
		.map(|h| (format!("{:?}", h.r#type), h.hash))
		.collect();
	out.sort();
	out
}

#[test]
fn old_client_clicks_send_vanilla_hashes() {
	let mut cx = Session::new();
	enchantment_registry(&mut cx);
	let sword = item(
		"1.21.4",
		"diamond_sword",
		1,
		vec![
			a::SlotComponent::Ty::damage(5),
			a::SlotComponent::Ty::enchantments(a::SlotComponent__enchantments::Ty {
				enchantments: vec![a::SlotComponent__enchantments__enchantments__items::Ty {
					id: 1,
					level: 3,
				}],
				showTooltip: true,
			}),
		],
	);
	let c = click(sword, &mut cx);
	assert!(c.changedSlots[0].item.is_none());
	let enchantments = hash_map(&[(hash_str("minecraft:sharpness"), hash_int(3))]);
	assert_eq!(
		hashes(&c),
		vec![
			("damage".to_owned(), hash_int(5)),
			("enchantments".to_owned(), enchantments)
		]
	);
	let slot = c.cursorItem.as_ref().unwrap();
	assert_eq!(
		(slot.itemId, slot.itemCount),
		(id("1.21.5", "items", "diamond_sword"), 1)
	);
}

#[test]
fn clicks_reuse_components_the_server_sent() {
	let mut cx = Session::new();
	enchantment_registry(&mut cx);
	let enchantments = v1_21_5::SlotComponent::Ty::enchantments(vec![
		v1_21_5::SlotComponent__enchantments__items::Ty { id: 1, level: 2 },
	]);
	let hidden =
		v1_21_5::SlotComponent::Ty::tooltip_display(v1_21_5::SlotComponent__tooltip_display::Ty {
			hideTooltip: false,
			hiddenComponents: vec![10, 3],
		});
	let sent = v1_21_5::Slot::Ty::default(
		1,
		v1_21_5::Slot__default::Ty {
			itemId: id("1.21.5", "items", "diamond_sword"),
			addedComponentCount: 2,
			removedComponentCount: 0,
			components: vec![enchantments, hidden],
			removeComponents: vec![],
		},
	);
	let p =
		v1_21_5::play_toClient_packet::Ty::set_slot(v1_21_5::play_toClient_packet_set_slot::Ty {
			windowId: 0,
			stateId: 1,
			slot: 36,
			item: sent,
		});
	let out = run(
		Root::PlayToClient,
		"1.21.5",
		"1.21.4",
		&p,
		v1_21_5::play_toClient_packet::serialize,
		a::play_toClient_packet::deserialize,
		&mut cx,
	);
	let [a::play_toClient_packet::Ty::set_slot(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let c = click(s.item.clone(), &mut cx);
	let hidden = {
		let mut b = vec![4];
		b.extend(hash_str("minecraft:enchantments").to_le_bytes());
		b.extend(hash_str("minecraft:damage").to_le_bytes());
		b.push(5);
		hash_map(&[(hash_str("hidden_components"), crc32c(&b))])
	};
	let enchantments = hash_map(&[(hash_str("minecraft:sharpness"), hash_int(2))]);
	assert_eq!(
		hashes(&c),
		vec![
			("enchantments".to_owned(), enchantments),
			("tooltip_display".to_owned(), hidden)
		]
	);
}

#[test]
fn new_server_text_reaches_old_client() {
	let list = |items: Vec<Vec<Entry>>| Payload::list(List::new(ListItems::compound(items)));
	let entry = |n: &str, v: Payload| Entry::new(n, v);
	let content = tag(compound(vec![
		("text", s("a")),
		(
			"click_event",
			compound(vec![
				("action", s("show_dialog")),
				("dialog", s("minecraft:x")),
			]),
		),
		(
			"hover_event",
			compound(vec![
				("action", s("show_text")),
				("value", compound(vec![("text", s("tip"))])),
			]),
		),
		(
			"extra",
			list(vec![
				vec![
					entry("type", s("object")),
					entry("sprite", s("minecraft:item/apple")),
					entry("fallback", s("apple")),
				],
				vec![
					entry("text", s("b")),
					entry(
						"click_event",
						compound(vec![
							("action", s("change_page")),
							("page", Payload::int(3)),
						]),
					),
				],
			]),
		),
	]));
	let p = z::play_toClient_packet::Ty::system_chat(z::play_toClient_packet_system_chat::Ty {
		content,
		isActionBar: false,
	});
	let out = run(
		Root::PlayToClient,
		"26.1",
		"1.21.4",
		&p,
		z::play_toClient_packet::serialize,
		a::play_toClient_packet::deserialize,
		&mut Session::new(),
	);
	let [a::play_toClient_packet::Ty::system_chat(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let v = &c.content.value;
	assert_eq!(get(v, &["clickEvent"]), None);
	assert_eq!(get(v, &["click_event"]), None);
	assert_eq!(get(v, &["hoverEvent", "contents", "text"]), Some(&s("tip")));
	let Some(Payload::list(List {
		items: ListItems::compound(extra),
		..
	})) = get(v, &["extra"])
	else {
		panic!("{v:?}")
	};
	let first = Payload::compound(extra[0].clone());
	assert_eq!(get(&first, &["text"]), Some(&s("apple")));
	assert_eq!(get(&first, &["sprite"]), None);
	let second = Payload::compound(extra[1].clone());
	assert_eq!(get(&second, &["clickEvent", "value"]), Some(&s("3")));
}

fn spawn5(entity: i32, kind: &str, data: i32) -> v1_21_5::play_toClient_packet::Ty {
	v1_21_5::play_toClient_packet::Ty::spawn_entity(
		v1_21_5::play_toClient_packet_spawn_entity::Ty {
			entityId: entity,
			objectUUID: entity as u128,
			r#type: id("1.21.5", "entities", kind),
			x: 0.0,
			y: 64.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			headPitch: 0,
			objectData: data,
			velocity: v1_21_5::vec3i16::Ty { x: 0, y: 0, z: 0 },
		},
	)
}

fn down5(
	p: &v1_21_5::play_toClient_packet::Ty,
	cx: &mut Session,
) -> Vec<a::play_toClient_packet::Ty> {
	run(
		Root::PlayToClient,
		"1.21.5",
		"1.21.4",
		p,
		v1_21_5::play_toClient_packet::serialize,
		a::play_toClient_packet::deserialize,
		cx,
	)
}

#[test]
fn saddles_and_orbs_reach_old_clients() {
	let mut cx = Session::new();
	down5(&spawn5(7, "pig", 0), &mut cx);
	let saddle = v1_21_5::Slot::Ty::default(
		1,
		v1_21_5::Slot__default::Ty {
			itemId: id("1.21.5", "items", "saddle"),
			addedComponentCount: 0,
			removedComponentCount: 0,
			components: vec![],
			removeComponents: vec![],
		},
	);
	let p = v1_21_5::play_toClient_packet::Ty::entity_equipment(
		v1_21_5::play_toClient_packet_entity_equipment::Ty {
			entityId: 7,
			equipments: vec![
				v1_21_5::play_toClient_packet_entity_equipment__equipments__items::Ty {
					slot: 7,
					item: saddle,
				},
			],
		},
	);
	let out = down5(&p, &mut cx);
	let [a::play_toClient_packet::Ty::entity_metadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(m.metadata[0].key, &m.metadata[0].value),
		(17, &a::entityMetadataEntry__value::Ty::boolean(true))
	);
	let out = down5(&spawn5(8, "experience_orb", 12), &mut cx);
	let [a::play_toClient_packet::Ty::spawn_entity_experience_orb(o)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((o.entityId, o.count), (8, 12));
}

#[test]
fn registries_downgrade_for_old_clients() {
	use v1_21_5::configuration_toClient_packet as c5;
	let registry = |id: &str, entries: Vec<(&str, Payload)>| {
		c5::Ty::registry_data(v1_21_5::configuration_toClient_packet_registry_data::Ty {
			id: id.into(),
			entries: entries
				.into_iter()
				.map(|(k, v)| {
					v1_21_5::configuration_toClient_packet_registry_data__entries__items::Ty {
						key: k.into(),
						value: Some(tag(v)),
					}
				})
				.collect(),
		})
	};
	let wolf = compound(vec![(
		"assets",
		compound(vec![
			("wild", s("minecraft:entity/wolf/wolf")),
			("tame", s("minecraft:entity/wolf/wolf_tame")),
			("angry", s("minecraft:entity/wolf/wolf_angry")),
		]),
	)]);
	let mut cx = Session::new();
	let conv = |p: &c5::Ty, cx: &mut Session| {
		run(
			Root::ConfigurationToClient,
			"1.21.5",
			"1.21.4",
			p,
			c5::serialize,
			a::configuration_toClient_packet::deserialize,
			cx,
		)
	};
	let out = conv(
		&registry("minecraft:wolf_variant", vec![("minecraft:pale", wolf)]),
		&mut cx,
	);
	let [a::configuration_toClient_packet::Ty::registry_data(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let v = &r.entries[0].value.as_ref().unwrap().value;
	assert_eq!(
		get(v, &["wild_texture"]),
		Some(&s("minecraft:entity/wolf/wolf"))
	);
	assert!(get(v, &["biomes"]).is_some());
	assert!(conv(
		&registry(
			"minecraft:pig_variant",
			vec![("minecraft:temperate", compound(vec![]))]
		),
		&mut cx
	)
	.is_empty());
}

#[test]
fn modern_dimension_types_downgrade() {
	use v1_21_11::configuration_toClient_packet as c11;
	let attributes = compound(vec![
		("minecraft:visual/cloud_height", Payload::float(192.33)),
		("minecraft:gameplay/piglins_zombify", Payload::byte(1)),
		("minecraft:gameplay/fast_lava", Payload::byte(0)),
	]);
	let overworld = compound(vec![
		("min_y", Payload::int(-64)),
		("height", Payload::int(384)),
		("has_fixed_time", Payload::byte(0)),
		("attributes", attributes),
	]);
	let p = c11::Ty::registry_data(v1_21_11::configuration_toClient_packet_registry_data::Ty {
		id: "minecraft:dimension_type".into(),
		entries: vec![
			v1_21_11::configuration_toClient_packet_registry_data__entries__items::Ty {
				key: "minecraft:overworld".into(),
				value: Some(tag(overworld)),
			},
		],
	});
	let out = run(
		Root::ConfigurationToClient,
		"1.21.11",
		"1.21.9",
		&p,
		c11::serialize,
		v1_21_9::configuration_toClient_packet::deserialize,
		&mut Session::new(),
	);
	let [v1_21_9::configuration_toClient_packet::Ty::registry_data(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let v = &r.entries[0].value.as_ref().unwrap().value;
	assert_eq!(get(v, &["cloud_height"]), Some(&Payload::int(192)));
	assert_eq!(get(v, &["piglin_safe"]), Some(&Payload::byte(0)));
	assert_eq!(get(v, &["effects"]), Some(&s("minecraft:overworld")));
	assert_eq!(get(v, &["attributes"]), None);
	let timeline =
		c11::Ty::registry_data(v1_21_11::configuration_toClient_packet_registry_data::Ty {
			id: "minecraft:timeline".into(),
			entries: vec![],
		});
	let out = run(
		Root::ConfigurationToClient,
		"1.21.11",
		"1.21.9",
		&timeline,
		c11::serialize,
		v1_21_9::configuration_toClient_packet::deserialize,
		&mut Session::new(),
	);
	assert!(out.is_empty());
}

#[test]
fn code_of_conduct_is_accepted_for_old_clients() {
	let mut cx = Session::new();
	let p = v1_21_9::configuration_toClient_packet::Ty::code_of_conduct("be nice".into());
	let out = run(
		Root::ConfigurationToClient,
		"1.21.9",
		"1.21.8",
		&p,
		v1_21_9::configuration_toClient_packet::serialize,
		v1_21_8::configuration_toClient_packet::deserialize,
		&mut cx,
	);
	assert!(out.is_empty());
	let sent = cx.take_outgoing();
	assert_eq!(sent.len(), 1);
	assert_eq!(sent[0].version, at("1.21.9"));
	let accept: v1_21_9::configuration_toServer_packet::Ty = decode(
		&sent[0].data,
		v1_21_9::configuration_toServer_packet::deserialize,
	);
	assert!(matches!(
		accept,
		v1_21_9::configuration_toServer_packet::Ty::accept_code_of_conduct(_)
	));
}

#[test]
fn relative_rotation_uses_last_look() {
	let mut cx = Session::new();
	let look = v1_21_8::play_toServer_packet::Ty::look(v1_21_8::play_toServer_packet_look::Ty {
		yaw: 90.0,
		pitch: 10.0,
		flags: decode(&[0], v1_21_8::play_toServer_MovementFlags::deserialize),
	});
	run(
		Root::PlayToServer,
		"1.21.8",
		"1.21.9",
		&look,
		v1_21_8::play_toServer_packet::serialize,
		v1_21_9::play_toServer_packet::deserialize,
		&mut cx,
	);
	let rotate = v1_21_9::play_toClient_packet::Ty::player_rotation(
		v1_21_9::play_toClient_packet_player_rotation::Ty {
			yaw: 10.0,
			relativeYaw: true,
			pitch: 5.0,
			relativePitch: false,
		},
	);
	let out = run(
		Root::PlayToClient,
		"1.21.9",
		"1.21.8",
		&rotate,
		v1_21_9::play_toClient_packet::serialize,
		v1_21_8::play_toClient_packet::deserialize,
		&mut cx,
	);
	let [v1_21_8::play_toClient_packet::Ty::player_rotation(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((r.yaw, r.pitch), (100.0, 5.0));
}

#[test]
fn old_client_interactions_reach_26_1() {
	use v1_21_11::play_toServer_packet_use_entity__mouse::Ty as Mouse;
	let send = |mouse: Mouse| {
		let p = v1_21_11::play_toServer_packet::Ty::use_entity(
			v1_21_11::play_toServer_packet_use_entity::Ty {
				target: 5,
				mouse,
				sneaking: false,
			},
		);
		run(
			Root::PlayToServer,
			"1.21.11",
			"26.1",
			&p,
			v1_21_11::play_toServer_packet::serialize,
			z::play_toServer_packet::deserialize,
			&mut Session::new(),
		)
	};
	assert!(matches!(
		send(Mouse::default(1)).as_slice(),
		[z::play_toServer_packet::Ty::attack(5)]
	));
	assert!(send(Mouse::v0(0)).is_empty());
	let out = send(Mouse::v2(
		v1_21_11::play_toServer_packet_use_entity__mouse__v2::Ty {
			x: 0.5,
			y: 1.0,
			z: 0.0,
			hand: 1,
		},
	));
	let [z::play_toServer_packet::Ty::use_entity(u)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(u.target, &u.hand),
		(5, &z::play_toServer_packet_use_entity__hand::Ty::off_hand)
	);
}

fn joined(entity: i32) -> z::play_toClient_packet::Ty {
	let out = up(&login(entity), &mut Session::new());
	out.into_iter()
		.find(|p| matches!(p, z::play_toClient_packet::Ty::login(_)))
		.unwrap()
}

fn data(
	entity: i32,
	metadata: Vec<(u8, z::entityMetadataEntry__value::Ty)>,
) -> z::play_toClient_packet::Ty {
	z::play_toClient_packet::Ty::entity_metadata(z::play_toClient_packet_entity_metadata::Ty {
		entityId: entity,
		metadata: metadata
			.into_iter()
			.map(|(key, value)| z::entityMetadataEntry::Ty { key, value })
			.collect(),
	})
}

fn rotation(pitch: f32) -> z::entityMetadataEntry__value::Ty {
	z::entityMetadataEntry__value::Ty::rotations(z::entityMetadataEntry__value__rotations::Ty {
		pitch,
		yaw: 0.0,
		roll: 0.0,
	})
}

fn layout_1_8(packets: &[v1_8::play_toClient_packet::Ty]) -> Vec<(u8, u8)> {
	packets
		.iter()
		.flat_map(|p| match p {
			v1_8::play_toClient_packet::Ty::entity_metadata(m) => m.metadata.clone(),
			v1_8::play_toClient_packet::Ty::spawn_entity_living(s) => s.metadata.clone(),
			_ => vec![],
		})
		.map(|e| (e.ANON.key, e.ANON.r#type))
		.collect()
}

fn layout_1_12(packets: &[v1_12_2::play_toClient_packet::Ty]) -> Vec<(u8, i8)> {
	packets
		.iter()
		.flat_map(|p| match p {
			v1_12_2::play_toClient_packet::Ty::entity_metadata(m) => m.metadata.clone(),
			v1_12_2::play_toClient_packet::Ty::spawn_entity_living(s) => s.metadata.clone(),
			_ => vec![],
		})
		.map(|e| (e.key, e.r#type))
		.collect()
}

#[test]
fn armor_stands_reach_1_8_and_1_12_2_with_their_layout() {
	use z::entityMetadataEntry__value::Ty as V;
	let stand =
		z::play_toClient_packet::Ty::spawn_entity(z::play_toClient_packet_spawn_entity::Ty {
			entityId: 11,
			objectUUID: 11,
			r#type: id("26.1", "entities", "armor_stand"),
			x: 0.5,
			y: 64.0,
			z: 0.5,
			velocity: vec![0],
			pitch: 0,
			yaw: 0,
			headPitch: 0,
			objectData: 0,
		});
	let hologram = data(
		11,
		vec![
			(0, V::byte(0x20)),
			(3, V::boolean(true)),
			(15, V::byte(0x19)),
			(16, rotation(10.0)),
			(20, rotation(20.0)),
		],
	);
	let mut cx = Session::new();
	run(
		Root::PlayToClient,
		"26.1",
		"1.8",
		&joined(3),
		z::play_toClient_packet::serialize,
		v1_8::play_toClient_packet::deserialize,
		&mut cx,
	);
	let mut old = Vec::new();
	for p in [&stand, &hologram] {
		old.extend(run(
			Root::PlayToClient,
			"26.1",
			"1.8",
			p,
			z::play_toClient_packet::serialize,
			v1_8::play_toClient_packet::deserialize,
			&mut cx,
		));
	}
	assert!(
		matches!(old.first(), Some(v1_8::play_toClient_packet::Ty::spawn_entity_living(s)) if s.r#type == 30),
		"{old:?}"
	);
	assert_eq!(
		layout_1_8(&old),
		vec![(0, 0), (0, 0), (3, 0), (10, 0), (11, 7), (15, 7)]
	);
	let mut cx = Session::new();
	run(
		Root::PlayToClient,
		"26.1",
		"1.12.2",
		&joined(3),
		z::play_toClient_packet::serialize,
		v1_12_2::play_toClient_packet::deserialize,
		&mut cx,
	);
	let mut old = Vec::new();
	for p in [&stand, &hologram] {
		old.extend(run(
			Root::PlayToClient,
			"26.1",
			"1.12.2",
			p,
			z::play_toClient_packet::serialize,
			v1_12_2::play_toClient_packet::deserialize,
			&mut cx,
		));
	}
	assert_eq!(
		layout_1_12(&old),
		vec![(0, 0), (3, 6), (11, 0), (12, 7), (16, 7)]
	);
}

#[test]
fn own_player_metadata_keeps_the_player_layout_on_old_clients() {
	use z::entityMetadataEntry__value::Ty as V;
	let me = data(
		3,
		vec![
			(
				15,
				V::humanoid_arm(z::entityMetadataEntry__value__humanoid_arm::Ty::right),
			),
			(16, V::byte(0x7f)),
			(17, V::float(4.0)),
			(18, V::int(7)),
		],
	);
	let mut cx = Session::new();
	run(
		Root::PlayToClient,
		"26.1",
		"1.12.2",
		&joined(3),
		z::play_toClient_packet::serialize,
		v1_12_2::play_toClient_packet::deserialize,
		&mut cx,
	);
	let old = run(
		Root::PlayToClient,
		"26.1",
		"1.12.2",
		&me,
		z::play_toClient_packet::serialize,
		v1_12_2::play_toClient_packet::deserialize,
		&mut cx,
	);
	let mut keys = layout_1_12(&old);
	keys.sort();
	assert_eq!(keys, vec![(11, 2), (12, 1), (13, 0), (14, 0)]);
}
