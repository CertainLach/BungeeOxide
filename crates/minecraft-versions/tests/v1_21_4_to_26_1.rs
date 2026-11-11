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

fn up(p: &a::play_to_client_packet::Ty, cx: &mut Session) -> Vec<z::play_to_client_packet::Ty> {
	run(
		Root::PlayToClient,
		"1.21.4",
		"26.1",
		p,
		a::play_to_client_packet::serialize,
		z::play_to_client_packet::deserialize,
		cx,
	)
}

fn down(p: &z::play_to_server_packet::Ty, cx: &mut Session) -> Vec<a::play_to_server_packet::Ty> {
	run(
		Root::PlayToServer,
		"26.1",
		"1.21.4",
		p,
		z::play_to_server_packet::serialize,
		a::play_to_server_packet::deserialize,
		cx,
	)
}

fn compound(entries: Vec<(&str, Payload)>) -> Payload {
	Payload::Compound(entries.into_iter().map(|(n, v)| Entry::new(n, v)).collect())
}

fn tag(p: Payload) -> Tag {
	Tag::new(p)
}

fn s(v: &str) -> Payload {
	Payload::String(v.into())
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
	components: Vec<a::slot_component::Ty>,
) -> a::slot::Ty {
	a::slot::Ty::Default(
		count,
		a::slot_default::Ty {
			item_id: id(version, "items", name),
			added_component_count: components.len() as i32,
			removed_component_count: 0,
			components,
			remove_components: vec![],
		},
	)
}

fn sample_config() -> Vec<a::configuration_to_client_packet::Ty> {
	use a::configuration_to_client_packet as c4;
	let registry = |id: &str, entries: Vec<(&str, Payload)>| {
		c4::Ty::RegistryData(a::configuration_to_client_packet_registry_data::Ty {
			id: id.into(),
			entries: entries
				.into_iter()
				.map(
					|(k, v)| a::configuration_to_client_packet_registry_data_entries_items::Ty {
						key: k.into(),
						value: Some(tag(v)),
					},
				)
				.collect(),
		})
	};
	let overworld = compound(vec![
		("min_y", Payload::Int(0)),
		("height", Payload::Int(256)),
		("effects", s("minecraft:overworld")),
		("natural", Payload::Byte(1)),
		("ambient_light", Payload::Float(0.0)),
		("coordinate_scale", Payload::Double(1.0)),
	]);
	let plains = compound(vec![(
		"effects",
		compound(vec![
			("sky_color", Payload::Int(7907327)),
			("fog_color", Payload::Int(12638463)),
			("water_color", Payload::Int(4159204)),
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
		c4::Ty::FinishConfiguration(a::configuration_to_client_packet_finish_configuration::Ty {}),
	]
}

#[test]
fn configuration_registries_reach_26_1() {
	use a::configuration_to_client_packet as c4;
	use z::configuration_to_client_packet as c26;
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
	assert!(matches!(out.last(), Some(c26::Ty::FinishConfiguration(_))));
	let registries: Vec<&z::configuration_to_client_packet_registry_data::Ty> = out
		.iter()
		.filter_map(|p| match p {
			c26::Ty::RegistryData(r) => Some(r),
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
	assert_eq!(get(&dim, &["height"]), Some(&Payload::Int(256)));
	assert_eq!(
		get(&dim, &["default_clock"]),
		Some(&s("minecraft:overworld"))
	);
	assert_eq!(
		get(&dim, &["attributes", "visual/cloud_height"]),
		Some(&Payload::Float(192.0))
	);
	assert!(get(&dim, &["attributes", "visual/ambient_light_color"]).is_some());
	assert!(matches!(get(&dim, &["timelines"]), Some(Payload::List(_))));
	let biome = entry("worldgen/biome", "minecraft:plains");
	assert_eq!(
		get(&biome, &["attributes", "visual/sky_color"]),
		Some(&Payload::Int(7907327))
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
		|p| matches!(p, c26::Ty::Tags(t) if t.iter().any(|r| r.tag_type.ends_with("damage_type")))
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
		Payload::LongArray(vec![7; 37]),
	)]));
	let packet = a::play_to_client_packet::Ty::MapChunk(a::play_to_client_packet_map_chunk::Ty {
		x: 1,
		z: 2,
		heightmaps,
		chunk_data: section_1_21_4(&states),
		block_entities: vec![],
		sky_light_mask: vec![],
		block_light_mask: vec![],
		empty_sky_light_mask: vec![],
		empty_block_light_mask: vec![],
		sky_light: vec![],
		block_light: vec![],
	});
	let out = up(&packet, &mut Session::new());
	let [z::play_to_client_packet::Ty::MapChunk(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(c.heightmaps.len(), 1);
	let d = &c.chunk_data;
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
	use a::slot_component::Ty as C;
	let enchantments = C::Enchantments(a::slot_component_enchantments::Ty {
		enchantments: vec![a::slot_component_enchantments_enchantments_items::Ty {
			id: 9,
			level: 3,
		}],
		show_tooltip: false,
	});
	let name = C::CustomName(tag(compound(vec![
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
		vec![enchantments, C::HideAdditionalTooltip, name],
	);
	let packet = a::play_to_client_packet::Ty::SetSlot(a::play_to_client_packet_set_slot::Ty {
		window_id: 0,
		state_id: 1,
		slot: 36,
		item: sword,
	});
	let out = run(
		Root::PlayToClient,
		"1.21.4",
		"1.21.5",
		&packet,
		a::play_to_client_packet::serialize,
		v1_21_5::play_to_client_packet::deserialize,
		&mut Session::new(),
	);
	let [v1_21_5::play_to_client_packet::Ty::SetSlot(s5)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let v1_21_5::slot::Ty::Default(1, d) = &s5.item else {
		panic!()
	};
	assert_eq!(d.item_id, id("1.21.5", "items", "diamond_sword"));
	assert_eq!(d.added_component_count as usize, d.components.len());
	let display = d
		.components
		.iter()
		.find_map(|c| match c {
			v1_21_5::slot_component::Ty::TooltipDisplay(t) => Some(t),
			_ => None,
		})
		.unwrap();
	assert!(!display.hide_tooltip);
	assert!(display.hidden_components.contains(&10));
	assert!(display.hidden_components.contains(&66));
	let name = d
		.components
		.iter()
		.find_map(|c| match c {
			v1_21_5::slot_component::Ty::CustomName(t) => Some(t),
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
	use a::slot_component::Ty as C;
	let mut cx = Session::new();
	let stones = C::Container(vec![item("1.21.4", "stone", 3, vec![]), a::slot::Ty::V0]);
	let shulker = item("1.21.4", "shulker_box", 1, vec![stones]);
	let packet =
		a::play_to_client_packet::Ty::WindowItems(a::play_to_client_packet_window_items::Ty {
			window_id: 1,
			state_id: 5,
			items: vec![shulker.clone(), a::slot::Ty::V0],
			carried_item: a::slot::Ty::V0,
		});
	let out = up(&packet, &mut cx);
	let [z::play_to_client_packet::Ty::WindowItems(w)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let z::slot::Ty::Default(1, d) = &w.items[0] else {
		panic!()
	};
	let container = d
		.components
		.iter()
		.find_map(|c| match c {
			z::slot_component::Ty::Container(v) => Some(v),
			_ => None,
		})
		.unwrap();
	assert_eq!(container.len(), 2);
	let first = container[0].as_ref().unwrap();
	assert_eq!(
		(first.item_id, first.item_count),
		(id("26.1", "items", "stone"), 3)
	);
	assert!(container[1].is_none());
	let hashed = |v: &z::slot::Ty| match v {
		z::slot::Ty::Default(count, d) => z::hashed_slot::Ty {
			item_id: d.item_id,
			item_count: *count,
			components: d
				.components
				.iter()
				.map(|c| {
					let bytes = encode(c, z::slot_component::serialize);
					z::hashed_slot_components_items::Ty {
						r#type: decode(&bytes[..1], z::slot_component_type::deserialize),
						hash: 0,
					}
				})
				.collect(),
			remove_components: vec![],
		},
		z::slot::Ty::V0 => unreachable!(),
	};
	let click =
		z::play_to_server_packet::Ty::WindowClick(z::play_to_server_packet_window_click::Ty {
			window_id: 1,
			state_id: 5,
			slot: 0,
			mouse_button: 0,
			mode: 0,
			changed_slots: vec![
				z::play_to_server_packet_window_click_changed_slots_items::Ty {
					location: 0,
					item: None,
				},
			],
			cursor_item: Some(hashed(&w.items[0])),
		});
	let out = down(&click, &mut cx);
	let [a::play_to_server_packet::Ty::WindowClick(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(c.cursor_item, shulker);
	assert_eq!(c.changed_slots[0].item, a::slot::Ty::V0);
}

#[test]
fn creative_items_are_decoded_down() {
	let name = tag(compound(vec![("text", s("named"))]));
	let mut data = Output::new();
	nbt::gen::tag::serialize(&name, &mut data).unwrap();
	let ty = decode(&[6], z::slot_component_type::deserialize);
	let slot = z::untrusted_slot::Ty {
		item_count: 2,
		anon: z::untrusted_slot_anon::Ty::Default(
			2,
			z::untrusted_slot_anon_default::Ty {
				item_id: id("26.1", "items", "golden_dandelion"),
				added_component_count: 1,
				removed_component_count: 0,
				components: vec![z::untrusted_slot_component::Ty {
					r#type: ty,
					data: data.into_inner(),
				}],
				remove_components: vec![],
			},
		),
	};
	let p = z::play_to_server_packet::Ty::SetCreativeSlot(
		z::play_to_server_packet_set_creative_slot::Ty {
			slot: 36,
			item: slot,
		},
	);
	let out = down(&p, &mut Session::new());
	let [a::play_to_server_packet::Ty::SetCreativeSlot(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let a::slot::Ty::Default(2, d) = &c.item else {
		panic!("{:?}", c.item)
	};
	assert!(
		matches!(d.components.as_slice(), [a::slot_component::Ty::CustomName(t)] if get(&t.value, &["text"]) == Some(&s("named")))
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
	let packet = v1_21_8::play_to_client_packet::Ty::SpawnEntity(
		v1_21_8::play_to_client_packet_spawn_entity::Ty {
			entity_id: 7,
			object_uuid: 1,
			r#type: id("1.21.8", "entities", "arrow"),
			x: 0.0,
			y: 64.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			head_pitch: 0,
			object_data: 0,
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
		v1_21_8::play_to_client_packet::serialize,
		v1_21_9::play_to_client_packet::deserialize,
		&mut cx,
	);
	let [v1_21_9::play_to_client_packet::Ty::SpawnEntity(s)] = out.as_slice() else {
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
		v1_21_9::play_to_client_packet::serialize,
		v1_21_8::play_to_client_packet::deserialize,
		&mut cx,
	);
	let [v1_21_8::play_to_client_packet::Ty::SpawnEntity(b)] = back.as_slice() else {
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
	let still = v1_21_8::play_to_client_packet::Ty::EntityVelocity(
		v1_21_8::play_to_client_packet_entity_velocity::Ty {
			entity_id: 7,
			velocity: v1_21_8::vec3i16::Ty { x: 0, y: 0, z: 0 },
		},
	);
	let out = run(
		Root::PlayToClient,
		"1.21.8",
		"1.21.9",
		&still,
		v1_21_8::play_to_client_packet::serialize,
		v1_21_9::play_to_client_packet::deserialize,
		&mut cx,
	);
	assert!(
		matches!(out.as_slice(), [v1_21_9::play_to_client_packet::Ty::EntityVelocity(v)] if v.velocity == vec![0])
	);
}

fn spawn(entity: i32, kind: &str) -> a::play_to_client_packet::Ty {
	a::play_to_client_packet::Ty::SpawnEntity(a::play_to_client_packet_spawn_entity::Ty {
		entity_id: entity,
		object_uuid: entity as u128,
		r#type: id("1.21.4", "entities", kind),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		pitch: 0,
		yaw: 0,
		head_pitch: 0,
		object_data: 0,
		velocity: a::vec3i16::Ty { x: 0, y: 0, z: 0 },
	})
}

#[test]
fn pig_metadata_moves_saddle_and_age_index() {
	let mut cx = Session::new();
	let out = up(&spawn(3, "pig"), &mut cx);
	assert!(
		matches!(out.as_slice(), [z::play_to_client_packet::Ty::SpawnEntity(s)] if s.r#type == id("26.1", "entities", "pig"))
	);
	use a::entity_metadata_entry_value::Ty as V;
	let meta = a::play_to_client_packet::Ty::EntityMetadata(
		a::play_to_client_packet_entity_metadata::Ty {
			entity_id: 3,
			metadata: vec![
				a::entity_metadata_entry::Ty {
					key: 0,
					value: V::Byte(0),
				},
				a::entity_metadata_entry::Ty {
					key: 17,
					value: V::Boolean(true),
				},
				a::entity_metadata_entry::Ty {
					key: 18,
					value: V::Int(40),
				},
			],
		},
	);
	let out = up(&meta, &mut cx);
	let [z::play_to_client_packet::Ty::EntityMetadata(m), z::play_to_client_packet::Ty::EntityEquipment(e)] =
		out.as_slice()
	else {
		panic!("{out:?}")
	};
	let keys: Vec<u8> = m.metadata.iter().map(|e| e.key).collect();
	assert_eq!(keys, vec![0, 18]);
	assert_eq!(e.equipments[0].slot, 7);
	assert!(
		matches!(&e.equipments[0].item, z::slot::Ty::Default(1, d) if d.item_id == id("26.1", "items", "saddle"))
	);
}

fn login(entity: i32) -> a::play_to_client_packet::Ty {
	a::play_to_client_packet::Ty::Login(a::play_to_client_packet_login::Ty {
		entity_id: entity,
		is_hardcore: false,
		world_names: vec!["minecraft:overworld".into()],
		max_players: 20,
		view_distance: 8,
		simulation_distance: 8,
		reduced_debug_info: false,
		enable_respawn_screen: true,
		do_limited_crafting: false,
		world_state: a::play_to_client_spawn_info::Ty {
			dimension: 0,
			name: "minecraft:overworld".into(),
			hashed_seed: 0,
			gamemode: a::play_to_client_spawn_info_gamemode::Ty::Survival,
			previous_gamemode: 255,
			is_debug: false,
			is_flat: false,
			death: None,
			portal_cooldown: 0,
			sea_level: 63,
		},
		enforces_secure_chat: false,
	})
}

#[test]
fn sneaking_input_becomes_entity_action() {
	let mut cx = Session::new();
	up(&login(42), &mut cx);
	let input = |shift| {
		z::play_to_server_packet::Ty::PlayerInput(z::play_to_server_packet_player_input::Ty {
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
	let [a::play_to_server_packet::Ty::EntityAction(e), a::play_to_server_packet::Ty::PlayerInput(_)] =
		out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!(e.entity_id, 42);
	assert!(matches!(
		e.action_id,
		a::play_to_server_packet_entity_action_action_id::Ty::StartSneaking
	));
	assert_eq!(down(&input(true), &mut cx).len(), 1);
	let out = down(&input(false), &mut cx);
	assert!(
		matches!(&out[0], a::play_to_server_packet::Ty::EntityAction(e) if matches!(e.action_id, a::play_to_server_packet_entity_action_action_id::Ty::StopSneaking))
	);
}

#[test]
fn time_becomes_clock_updates() {
	let p = a::play_to_client_packet::Ty::UpdateTime(a::play_to_client_packet_update_time::Ty {
		age: 100,
		time: 6000,
		tick_day_time: true,
	});
	let out = up(&p, &mut Session::new());
	let [z::play_to_client_packet::Ty::UpdateTime(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(t.age, 100);
	assert_eq!(t.clock_updates.len(), 2);
	assert_eq!(
		(t.clock_updates[0].total_ticks, t.clock_updates[0].rate),
		(6000, 1.0)
	);
}

fn sample_play() -> Vec<a::play_to_client_packet::Ty> {
	use a::play_to_client_packet::Ty as P;
	use a::slot_component::Ty as C;
	let blocks = registry::blocks("1.21.4").unwrap();
	let state = |n: &str| blocks.parse_state(n).unwrap() as i32;
	let pos = a::position::Ty { x: 1, z: 2, y: 64 };
	let sword = item(
		"1.21.4",
		"diamond_sword",
		1,
		vec![
			C::Enchantments(a::slot_component_enchantments::Ty {
				enchantments: vec![a::slot_component_enchantments_enchantments_items::Ty {
					id: 1,
					level: 2,
				}],
				show_tooltip: true,
			}),
			C::CustomName(tag(compound(vec![("text", s("Blade"))]))),
			C::Lore(vec![Some(tag(compound(vec![("text", s("line"))]))), None]),
			C::Unbreakable(false),
			C::HideTooltip,
		],
	);
	let sign = compound(vec![(
		"front_text",
		compound(vec![(
			"messages",
			Payload::List(List::new(ListItems::String(vec![
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
		P::MapChunk(a::play_to_client_packet_map_chunk::Ty {
			x: 0,
			z: 0,
			heightmaps: tag(compound(vec![
				("MOTION_BLOCKING", Payload::LongArray(vec![1; 37])),
				("WORLD_SURFACE", Payload::LongArray(vec![2; 37])),
			])),
			chunk_data: [section_1_21_4(&[1, 9, 2000]), section_1_21_4(&[1])].concat(),
			block_entities: vec![a::chunk_block_entity::Ty {
				anon: a::chunk_block_entity_anon::Ty { x: 1, z: 2 },
				y: 64,
				r#type: 7,
				nbt_data: Some(tag(sign)),
			}],
			sky_light_mask: vec![],
			block_light_mask: vec![],
			empty_sky_light_mask: vec![],
			empty_block_light_mask: vec![],
			sky_light: vec![],
			block_light: vec![],
		}),
		P::SetSlot(a::play_to_client_packet_set_slot::Ty {
			window_id: 0,
			state_id: 1,
			slot: 36,
			item: sword.clone(),
		}),
		P::WindowItems(a::play_to_client_packet_window_items::Ty {
			window_id: 0,
			state_id: 2,
			items: vec![a::slot::Ty::V0, sword],
			carried_item: a::slot::Ty::V0,
		}),
		P::BlockChange(a::play_to_client_packet_block_change::Ty {
			location: pos.clone(),
			r#type: state("minecraft:oak_log[axis=x]"),
		}),
		P::MultiBlockChange(a::play_to_client_packet_multi_block_change::Ty {
			chunk_coordinates: a::play_to_client_packet_multi_block_change_chunk_coordinates::Ty {
				x: 0,
				z: 0,
				y: 4,
			},
			records: vec![state("minecraft:glass") << 12 | 0x123],
		}),
		P::WorldEvent(a::play_to_client_packet_world_event::Ty {
			effect_id: 2001,
			location: pos.clone(),
			data: state("minecraft:stone"),
			global: false,
		}),
		P::WorldParticles(a::play_to_client_packet_world_particles::Ty {
			long_distance: false,
			always_show: false,
			x: 0.0,
			y: 64.0,
			z: 0.0,
			offset_x: 0.0,
			offset_y: 0.0,
			offset_z: 0.0,
			velocity_offset: 0.0,
			amount: 3,
			particle: a::particle::Ty::Block(state("minecraft:dirt")),
		}),
		P::SoundEffect(a::play_to_client_packet_sound_effect::Ty {
			sound: a::item_sound_holder::Ty::Id(
				registry::names("1.21.4", "sounds")
					.unwrap()
					.id("entity.pig.ambient")
					.unwrap() as i32
					- 1,
			),
			sound_category: a::sound_source::Ty::Neutral,
			x: 0,
			y: 512,
			z: 0,
			volume: 1.0,
			pitch: 1.0,
			seed: 1,
		}),
		P::SpawnPosition(a::play_to_client_packet_spawn_position::Ty {
			location: pos.clone(),
			angle: 90.0,
		}),
		P::Explosion(a::play_to_client_packet_explosion::Ty {
			x: 0.0,
			y: 64.0,
			z: 0.0,
			player_knockback: None,
			explosion_particle: a::particle::Ty::Explosion,
			sound: a::item_sound_holder::Ty::Id(
				registry::names("1.21.4", "sounds")
					.unwrap()
					.id("entity.generic.explode")
					.unwrap() as i32
					- 1,
			),
		}),
		P::UpdateTime(a::play_to_client_packet_update_time::Ty {
			age: 10,
			time: 1000,
			tick_day_time: true,
		}),
		P::EntityVelocity(a::play_to_client_packet_entity_velocity::Ty {
			entity_id: 5,
			velocity: a::vec3i16::Ty {
				x: 100,
				y: 2000,
				z: -300,
			},
		}),
		P::EntityStatus(a::play_to_client_packet_entity_status::Ty {
			entity_id: 1,
			entity_status: 29,
		}),
		P::SpawnEntityExperienceOrb(a::play_to_client_packet_spawn_entity_experience_orb::Ty {
			entity_id: 9,
			x: 1.0,
			y: 64.0,
			z: 1.0,
			count: 7,
		}),
		P::SystemChat(a::play_to_client_packet_system_chat::Ty {
			content: tag(compound(vec![
				("text", s("hi")),
				(
					"hoverEvent",
					compound(vec![("action", s("show_text")), ("contents", s("x"))]),
				),
			])),
			is_action_bar: false,
		}),
		P::Teams(a::play_to_client_packet_teams::Ty {
			team: "t".into(),
			mode: a::play_to_client_packet_teams_mode::Ty::V0(
				a::play_to_client_packet_teams_mode_v0::Ty {
					name: tag(s("team")),
					friendly_fire: 1,
					name_tag_visibility: "hideForOtherTeams".into(),
					collision_rule: "never".into(),
					formatting: 0,
					prefix: tag(s("[p] ")),
					suffix: tag(s("")),
					players: vec!["a".into()],
				},
			),
		}),
	];
	use a::entity_metadata_entry_value::Ty as V;
	out.push(P::EntityMetadata(
		a::play_to_client_packet_entity_metadata::Ty {
			entity_id: 5,
			metadata: vec![
				a::entity_metadata_entry::Ty {
					key: 2,
					value: V::OptionalComponent(Some(tag(s("Pig")))),
				},
				a::entity_metadata_entry::Ty {
					key: 17,
					value: V::Boolean(true),
				},
				a::entity_metadata_entry::Ty {
					key: 18,
					value: V::Int(3),
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
					&encode(&p, a::play_to_client_packet::serialize),
					&mut cx,
				)
				.unwrap(),
			);
		}
		std::fs::write(dir.join(format!("{version}.bin")), frame(&play)).unwrap();
		let mut config = Vec::new();
		for p in sample_config() {
			let data = encode(&p, a::configuration_to_client_packet::serialize);
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
				&encode(&p, a::play_to_client_packet::serialize),
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
			Payload::List(List::new(ListItems::Compound(vec![vec![Entry::new(
				"clickEvent",
				compound(vec![("action", s("change_page")), ("value", s("2"))]),
			)]]))),
		),
	]));
	let p = a::play_to_client_packet::Ty::SystemChat(a::play_to_client_packet_system_chat::Ty {
		content,
		is_action_bar: false,
	});
	let out = up(&p, &mut Session::new());
	let [z::play_to_client_packet::Ty::SystemChat(c)] = out.as_slice() else {
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
	let Some(Payload::List(List {
		items: ListItems::Compound(extra),
		..
	})) = get(&c.content.value, &["extra"])
	else {
		panic!()
	};
	assert_eq!(
		get(
			&Payload::Compound(extra[0].clone()),
			&["click_event", "page"]
		),
		Some(&Payload::Int(2))
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
	let p = v1_21_5::configuration_to_client_packet::Ty::RegistryData(
		v1_21_5::configuration_to_client_packet_registry_data::Ty {
			id: "minecraft:enchantment".into(),
			entries: ["minecraft:protection", "minecraft:sharpness"]
				.into_iter()
				.map(
					|k| v1_21_5::configuration_to_client_packet_registry_data_entries_items::Ty {
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
		v1_21_5::configuration_to_client_packet::serialize,
		a::configuration_to_client_packet::deserialize,
		cx,
	);
}

fn click(item: a::slot::Ty, cx: &mut Session) -> v1_21_5::play_to_server_packet_window_click::Ty {
	let p = a::play_to_server_packet::Ty::WindowClick(a::play_to_server_packet_window_click::Ty {
		window_id: 0,
		state_id: 1,
		slot: 36,
		mouse_button: 0,
		mode: 0,
		changed_slots: vec![
			a::play_to_server_packet_window_click_changed_slots_items::Ty {
				location: 36,
				item: a::slot::Ty::V0,
			},
		],
		cursor_item: item,
	});
	let out = run(
		Root::PlayToServer,
		"1.21.4",
		"1.21.5",
		&p,
		a::play_to_server_packet::serialize,
		v1_21_5::play_to_server_packet::deserialize,
		cx,
	);
	let [v1_21_5::play_to_server_packet::Ty::WindowClick(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	c.clone()
}

fn hashes(c: &v1_21_5::play_to_server_packet_window_click::Ty) -> Vec<(String, i32)> {
	let slot = c.cursor_item.as_ref().expect("cursor item");
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
			a::slot_component::Ty::Damage(5),
			a::slot_component::Ty::Enchantments(a::slot_component_enchantments::Ty {
				enchantments: vec![a::slot_component_enchantments_enchantments_items::Ty {
					id: 1,
					level: 3,
				}],
				show_tooltip: true,
			}),
		],
	);
	let c = click(sword, &mut cx);
	assert!(c.changed_slots[0].item.is_none());
	let enchantments = hash_map(&[(hash_str("minecraft:sharpness"), hash_int(3))]);
	assert_eq!(
		hashes(&c),
		vec![
			("Damage".to_owned(), hash_int(5)),
			("Enchantments".to_owned(), enchantments)
		]
	);
	let slot = c.cursor_item.as_ref().unwrap();
	assert_eq!(
		(slot.item_id, slot.item_count),
		(id("1.21.5", "items", "diamond_sword"), 1)
	);
}

#[test]
fn clicks_reuse_components_the_server_sent() {
	let mut cx = Session::new();
	enchantment_registry(&mut cx);
	let enchantments = v1_21_5::slot_component::Ty::Enchantments(vec![
		v1_21_5::slot_component_enchantments_items::Ty { id: 1, level: 2 },
	]);
	let hidden =
		v1_21_5::slot_component::Ty::TooltipDisplay(v1_21_5::slot_component_tooltip_display::Ty {
			hide_tooltip: false,
			hidden_components: vec![10, 3],
		});
	let sent = v1_21_5::slot::Ty::Default(
		1,
		v1_21_5::slot_default::Ty {
			item_id: id("1.21.5", "items", "diamond_sword"),
			added_component_count: 2,
			removed_component_count: 0,
			components: vec![enchantments, hidden],
			remove_components: vec![],
		},
	);
	let p =
		v1_21_5::play_to_client_packet::Ty::SetSlot(v1_21_5::play_to_client_packet_set_slot::Ty {
			window_id: 0,
			state_id: 1,
			slot: 36,
			item: sent,
		});
	let out = run(
		Root::PlayToClient,
		"1.21.5",
		"1.21.4",
		&p,
		v1_21_5::play_to_client_packet::serialize,
		a::play_to_client_packet::deserialize,
		&mut cx,
	);
	let [a::play_to_client_packet::Ty::SetSlot(s)] = out.as_slice() else {
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
			("Enchantments".to_owned(), enchantments),
			("TooltipDisplay".to_owned(), hidden)
		]
	);
}

#[test]
fn new_server_text_reaches_old_client() {
	let list = |items: Vec<Vec<Entry>>| Payload::List(List::new(ListItems::Compound(items)));
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
							("page", Payload::Int(3)),
						]),
					),
				],
			]),
		),
	]));
	let p = z::play_to_client_packet::Ty::SystemChat(z::play_to_client_packet_system_chat::Ty {
		content,
		is_action_bar: false,
	});
	let out = run(
		Root::PlayToClient,
		"26.1",
		"1.21.4",
		&p,
		z::play_to_client_packet::serialize,
		a::play_to_client_packet::deserialize,
		&mut Session::new(),
	);
	let [a::play_to_client_packet::Ty::SystemChat(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let v = &c.content.value;
	assert_eq!(get(v, &["clickEvent"]), None);
	assert_eq!(get(v, &["click_event"]), None);
	assert_eq!(get(v, &["hoverEvent", "contents", "text"]), Some(&s("tip")));
	let Some(Payload::List(List {
		items: ListItems::Compound(extra),
		..
	})) = get(v, &["extra"])
	else {
		panic!("{v:?}")
	};
	let first = Payload::Compound(extra[0].clone());
	assert_eq!(get(&first, &["text"]), Some(&s("apple")));
	assert_eq!(get(&first, &["sprite"]), None);
	let second = Payload::Compound(extra[1].clone());
	assert_eq!(get(&second, &["clickEvent", "value"]), Some(&s("3")));
}

fn spawn5(entity: i32, kind: &str, data: i32) -> v1_21_5::play_to_client_packet::Ty {
	v1_21_5::play_to_client_packet::Ty::SpawnEntity(
		v1_21_5::play_to_client_packet_spawn_entity::Ty {
			entity_id: entity,
			object_uuid: entity as u128,
			r#type: id("1.21.5", "entities", kind),
			x: 0.0,
			y: 64.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			head_pitch: 0,
			object_data: data,
			velocity: v1_21_5::vec3i16::Ty { x: 0, y: 0, z: 0 },
		},
	)
}

fn down5(
	p: &v1_21_5::play_to_client_packet::Ty,
	cx: &mut Session,
) -> Vec<a::play_to_client_packet::Ty> {
	run(
		Root::PlayToClient,
		"1.21.5",
		"1.21.4",
		p,
		v1_21_5::play_to_client_packet::serialize,
		a::play_to_client_packet::deserialize,
		cx,
	)
}

#[test]
fn saddles_and_orbs_reach_old_clients() {
	let mut cx = Session::new();
	down5(&spawn5(7, "pig", 0), &mut cx);
	let saddle = v1_21_5::slot::Ty::Default(
		1,
		v1_21_5::slot_default::Ty {
			item_id: id("1.21.5", "items", "saddle"),
			added_component_count: 0,
			removed_component_count: 0,
			components: vec![],
			remove_components: vec![],
		},
	);
	let p = v1_21_5::play_to_client_packet::Ty::EntityEquipment(
		v1_21_5::play_to_client_packet_entity_equipment::Ty {
			entity_id: 7,
			equipments: vec![
				v1_21_5::play_to_client_packet_entity_equipment_equipments_items::Ty {
					slot: 7,
					item: saddle,
				},
			],
		},
	);
	let out = down5(&p, &mut cx);
	let [a::play_to_client_packet::Ty::EntityMetadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(m.metadata[0].key, &m.metadata[0].value),
		(17, &a::entity_metadata_entry_value::Ty::Boolean(true))
	);
	let out = down5(&spawn5(8, "experience_orb", 12), &mut cx);
	let [a::play_to_client_packet::Ty::SpawnEntityExperienceOrb(o)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((o.entity_id, o.count), (8, 12));
}

#[test]
fn registries_downgrade_for_old_clients() {
	use v1_21_5::configuration_to_client_packet as c5;
	let registry = |id: &str, entries: Vec<(&str, Payload)>| {
		c5::Ty::RegistryData(v1_21_5::configuration_to_client_packet_registry_data::Ty {
			id: id.into(),
			entries: entries
				.into_iter()
				.map(|(k, v)| {
					v1_21_5::configuration_to_client_packet_registry_data_entries_items::Ty {
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
			a::configuration_to_client_packet::deserialize,
			cx,
		)
	};
	let out = conv(
		&registry("minecraft:wolf_variant", vec![("minecraft:pale", wolf)]),
		&mut cx,
	);
	let [a::configuration_to_client_packet::Ty::RegistryData(r)] = out.as_slice() else {
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
	use v1_21_11::configuration_to_client_packet as c11;
	let attributes = compound(vec![
		("minecraft:visual/cloud_height", Payload::Float(192.33)),
		("minecraft:gameplay/piglins_zombify", Payload::Byte(1)),
		("minecraft:gameplay/fast_lava", Payload::Byte(0)),
	]);
	let overworld = compound(vec![
		("min_y", Payload::Int(-64)),
		("height", Payload::Int(384)),
		("has_fixed_time", Payload::Byte(0)),
		("attributes", attributes),
	]);
	let p = c11::Ty::RegistryData(v1_21_11::configuration_to_client_packet_registry_data::Ty {
		id: "minecraft:dimension_type".into(),
		entries: vec![
			v1_21_11::configuration_to_client_packet_registry_data_entries_items::Ty {
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
		v1_21_9::configuration_to_client_packet::deserialize,
		&mut Session::new(),
	);
	let [v1_21_9::configuration_to_client_packet::Ty::RegistryData(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let v = &r.entries[0].value.as_ref().unwrap().value;
	assert_eq!(get(v, &["cloud_height"]), Some(&Payload::Int(192)));
	assert_eq!(get(v, &["piglin_safe"]), Some(&Payload::Byte(0)));
	assert_eq!(get(v, &["effects"]), Some(&s("minecraft:overworld")));
	assert_eq!(get(v, &["attributes"]), None);
	let timeline =
		c11::Ty::RegistryData(v1_21_11::configuration_to_client_packet_registry_data::Ty {
			id: "minecraft:timeline".into(),
			entries: vec![],
		});
	let out = run(
		Root::ConfigurationToClient,
		"1.21.11",
		"1.21.9",
		&timeline,
		c11::serialize,
		v1_21_9::configuration_to_client_packet::deserialize,
		&mut Session::new(),
	);
	assert!(out.is_empty());
}

#[test]
fn code_of_conduct_is_accepted_for_old_clients() {
	let mut cx = Session::new();
	let p = v1_21_9::configuration_to_client_packet::Ty::CodeOfConduct("be nice".into());
	let out = run(
		Root::ConfigurationToClient,
		"1.21.9",
		"1.21.8",
		&p,
		v1_21_9::configuration_to_client_packet::serialize,
		v1_21_8::configuration_to_client_packet::deserialize,
		&mut cx,
	);
	assert!(out.is_empty());
	let sent = cx.take_outgoing();
	assert_eq!(sent.len(), 1);
	assert_eq!(sent[0].version, at("1.21.9"));
	let accept: v1_21_9::configuration_to_server_packet::Ty = decode(
		&sent[0].data,
		v1_21_9::configuration_to_server_packet::deserialize,
	);
	assert!(matches!(
		accept,
		v1_21_9::configuration_to_server_packet::Ty::AcceptCodeOfConduct(_)
	));
}

#[test]
fn relative_rotation_uses_last_look() {
	let mut cx = Session::new();
	let look = v1_21_8::play_to_server_packet::Ty::Look(v1_21_8::play_to_server_packet_look::Ty {
		yaw: 90.0,
		pitch: 10.0,
		flags: decode(&[0], v1_21_8::play_to_server_movement_flags::deserialize),
	});
	run(
		Root::PlayToServer,
		"1.21.8",
		"1.21.9",
		&look,
		v1_21_8::play_to_server_packet::serialize,
		v1_21_9::play_to_server_packet::deserialize,
		&mut cx,
	);
	let rotate = v1_21_9::play_to_client_packet::Ty::PlayerRotation(
		v1_21_9::play_to_client_packet_player_rotation::Ty {
			yaw: 10.0,
			relative_yaw: true,
			pitch: 5.0,
			relative_pitch: false,
		},
	);
	let out = run(
		Root::PlayToClient,
		"1.21.9",
		"1.21.8",
		&rotate,
		v1_21_9::play_to_client_packet::serialize,
		v1_21_8::play_to_client_packet::deserialize,
		&mut cx,
	);
	let [v1_21_8::play_to_client_packet::Ty::PlayerRotation(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((r.yaw, r.pitch), (100.0, 5.0));
}

#[test]
fn old_client_interactions_reach_26_1() {
	use v1_21_11::play_to_server_packet_use_entity_mouse::Ty as Mouse;
	let send = |mouse: Mouse| {
		let p = v1_21_11::play_to_server_packet::Ty::UseEntity(
			v1_21_11::play_to_server_packet_use_entity::Ty {
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
			v1_21_11::play_to_server_packet::serialize,
			z::play_to_server_packet::deserialize,
			&mut Session::new(),
		)
	};
	assert!(matches!(
		send(Mouse::Default(1)).as_slice(),
		[z::play_to_server_packet::Ty::Attack(5)]
	));
	assert!(send(Mouse::V0(0)).is_empty());
	let out = send(Mouse::V2(
		v1_21_11::play_to_server_packet_use_entity_mouse_v2::Ty {
			x: 0.5,
			y: 1.0,
			z: 0.0,
			hand: 1,
		},
	));
	let [z::play_to_server_packet::Ty::UseEntity(u)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(u.target, &u.hand),
		(5, &z::play_to_server_packet_use_entity_hand::Ty::OffHand)
	);
}

fn joined(entity: i32) -> z::play_to_client_packet::Ty {
	let out = up(&login(entity), &mut Session::new());
	out.into_iter()
		.find(|p| matches!(p, z::play_to_client_packet::Ty::Login(_)))
		.unwrap()
}

fn data(
	entity: i32,
	metadata: Vec<(u8, z::entity_metadata_entry_value::Ty)>,
) -> z::play_to_client_packet::Ty {
	z::play_to_client_packet::Ty::EntityMetadata(z::play_to_client_packet_entity_metadata::Ty {
		entity_id: entity,
		metadata: metadata
			.into_iter()
			.map(|(key, value)| z::entity_metadata_entry::Ty { key, value })
			.collect(),
	})
}

fn rotation(pitch: f32) -> z::entity_metadata_entry_value::Ty {
	z::entity_metadata_entry_value::Ty::Rotations(z::entity_metadata_entry_value_rotations::Ty {
		pitch,
		yaw: 0.0,
		roll: 0.0,
	})
}

fn layout_1_8(packets: &[v1_8::play_to_client_packet::Ty]) -> Vec<(u8, u8)> {
	packets
		.iter()
		.flat_map(|p| match p {
			v1_8::play_to_client_packet::Ty::EntityMetadata(m) => m.metadata.clone(),
			v1_8::play_to_client_packet::Ty::SpawnEntityLiving(s) => s.metadata.clone(),
			_ => vec![],
		})
		.map(|e| (e.anon.key, e.anon.r#type))
		.collect()
}

fn layout_1_12(packets: &[v1_12_2::play_to_client_packet::Ty]) -> Vec<(u8, i8)> {
	packets
		.iter()
		.flat_map(|p| match p {
			v1_12_2::play_to_client_packet::Ty::EntityMetadata(m) => m.metadata.clone(),
			v1_12_2::play_to_client_packet::Ty::SpawnEntityLiving(s) => s.metadata.clone(),
			_ => vec![],
		})
		.map(|e| (e.key, e.r#type))
		.collect()
}

#[test]
fn armor_stands_reach_1_8_and_1_12_2_with_their_layout() {
	use z::entity_metadata_entry_value::Ty as V;
	let stand =
		z::play_to_client_packet::Ty::SpawnEntity(z::play_to_client_packet_spawn_entity::Ty {
			entity_id: 11,
			object_uuid: 11,
			r#type: id("26.1", "entities", "armor_stand"),
			x: 0.5,
			y: 64.0,
			z: 0.5,
			velocity: vec![0],
			pitch: 0,
			yaw: 0,
			head_pitch: 0,
			object_data: 0,
		});
	let hologram = data(
		11,
		vec![
			(0, V::Byte(0x20)),
			(3, V::Boolean(true)),
			(15, V::Byte(0x19)),
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
		z::play_to_client_packet::serialize,
		v1_8::play_to_client_packet::deserialize,
		&mut cx,
	);
	let mut old = Vec::new();
	for p in [&stand, &hologram] {
		old.extend(run(
			Root::PlayToClient,
			"26.1",
			"1.8",
			p,
			z::play_to_client_packet::serialize,
			v1_8::play_to_client_packet::deserialize,
			&mut cx,
		));
	}
	assert!(
		matches!(old.first(), Some(v1_8::play_to_client_packet::Ty::SpawnEntityLiving(s)) if s.r#type == 30),
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
		z::play_to_client_packet::serialize,
		v1_12_2::play_to_client_packet::deserialize,
		&mut cx,
	);
	let mut old = Vec::new();
	for p in [&stand, &hologram] {
		old.extend(run(
			Root::PlayToClient,
			"26.1",
			"1.12.2",
			p,
			z::play_to_client_packet::serialize,
			v1_12_2::play_to_client_packet::deserialize,
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
	use z::entity_metadata_entry_value::Ty as V;
	let me = data(
		3,
		vec![
			(
				15,
				V::HumanoidArm(z::entity_metadata_entry_value_humanoid_arm::Ty::Right),
			),
			(16, V::Byte(0x7f)),
			(17, V::Float(4.0)),
			(18, V::Int(7)),
		],
	);
	let mut cx = Session::new();
	run(
		Root::PlayToClient,
		"26.1",
		"1.12.2",
		&joined(3),
		z::play_to_client_packet::serialize,
		v1_12_2::play_to_client_packet::deserialize,
		&mut cx,
	);
	let old = run(
		Root::PlayToClient,
		"26.1",
		"1.12.2",
		&me,
		z::play_to_client_packet::serialize,
		v1_12_2::play_to_client_packet::deserialize,
		&mut cx,
	);
	let mut keys = layout_1_12(&old);
	keys.sort();
	assert_eq!(keys, vec![(11, 2), (12, 1), (13, 0), (14, 0)]);
}
