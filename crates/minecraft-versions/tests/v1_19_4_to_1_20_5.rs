use minecraft_versions::custom::{configuration_1_20_2, configuration_capture_1_20_2};
use minecraft_versions::gen::convert::{
	convert, index_of, inspect, v1_19_4, v1_20, v1_20_2, v1_20_3, v1_20_5, Root,
};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::registry;
use minecraft_versions::Session;
use nbt::{Entry, List, ListItems, Payload};
use serde_json::Value;

fn at(v: &str) -> usize {
	index_of(v).unwrap()
}

fn encode<T>(v: &T, f: fn(&T, &mut Output) -> minecraft_versions::pds::Result<()>) -> Vec<u8> {
	let mut o = Output::new();
	f(v, &mut o).unwrap();
	o.into_inner()
}

fn decode<T>(b: &[u8], f: fn(&mut Input) -> minecraft_versions::pds::Result<T>) -> T {
	let mut i = Input::new(b);
	let v = f(&mut i).unwrap();
	assert_eq!(i.remaining(), 0, "leftover bytes");
	v
}

fn entry(name: &str, value: Payload) -> Entry {
	Entry::new(name, value)
}

fn vanilla_codec(version: &str) -> Entry {
	let v: Value = serde_json::from_str(registry::login_packet(version).unwrap()).unwrap();
	entry("", nbt::json::typed::read(&v["dimensionCodec"]).unwrap())
}

fn names(root: Root, version: &str, packets: &[Vec<u8>]) -> Vec<String> {
	packets
		.iter()
		.map(|p| inspect(root, at(version), p).unwrap())
		.collect()
}

fn login_1_19_4() -> v1_19_4::play_to_client_packet::Ty {
	v1_19_4::play_to_client_packet::Ty::Login(v1_19_4::play_to_client_packet_login::Ty {
		entity_id: 42,
		is_hardcore: false,
		game_mode: 1,
		previous_game_mode: -1,
		world_names: vec!["minecraft:overworld".to_owned()],
		dimension_codec: vanilla_codec("1.19.4"),
		world_type: "minecraft:overworld".to_owned(),
		world_name: "minecraft:overworld".to_owned(),
		hashed_seed: 7,
		max_players: 20,
		view_distance: 8,
		simulation_distance: 8,
		reduced_debug_info: false,
		enable_respawn_screen: true,
		is_debug: false,
		is_flat: false,
		death: None,
	})
}

#[test]
fn configuration_reaches_1_20_5() {
	let mut cx = Session::new();
	let packets = configuration_1_20_2(&mut cx).unwrap();
	assert_eq!(
		names(Root::ConfigurationToClient, "1.20.2", &packets),
		[
			"registry_data",
			"feature_flags",
			"tags",
			"finish_configuration"
		]
	);
	let mut out = Vec::new();
	for p in &packets {
		out.extend(
			convert(
				Root::ConfigurationToClient,
				at("1.20.2"),
				at("1.20.5"),
				p,
				&mut cx,
			)
			.unwrap(),
		);
	}
	let kinds = names(Root::ConfigurationToClient, "1.20.5", &out);
	assert_eq!(
		kinds.first().map(String::as_str),
		Some("select_known_packs")
	);
	assert_eq!(
		kinds.last().map(String::as_str),
		Some("finish_configuration")
	);
	let registries: Vec<String> = out
		.iter()
		.filter_map(
			|p| match decode(p, v1_20_5::configuration_to_client_packet::deserialize) {
				v1_20_5::configuration_to_client_packet::Ty::RegistryData(r) => {
					assert!(!r.entries.is_empty(), "{} is empty", r.id);
					assert!(r.entries.iter().all(|e| e.value.is_some()));
					Some(r.id)
				}
				_ => None,
			},
		)
		.collect();
	for id in [
		"minecraft:worldgen/biome",
		"minecraft:chat_type",
		"minecraft:trim_pattern",
		"minecraft:trim_material",
		"minecraft:wolf_variant",
		"minecraft:dimension_type",
		"minecraft:damage_type",
		"minecraft:banner_pattern",
	] {
		assert!(
			registries.iter().any(|r| r == id),
			"missing {id} in {registries:?}"
		);
	}
	let damage = out.iter().find_map(|p| {
		match decode(p, v1_20_5::configuration_to_client_packet::deserialize) {
			v1_20_5::configuration_to_client_packet::Ty::RegistryData(r)
				if r.id == "minecraft:damage_type" =>
			{
				Some(r)
			}
			_ => None,
		}
	});
	assert!(damage
		.unwrap()
		.entries
		.iter()
		.any(|e| e.key == "minecraft:spit"));
}

#[test]
fn join_game_reaches_1_20_5_and_feeds_configuration() {
	let mut cx = Session::new();
	let login = encode(&login_1_19_4(), v1_19_4::play_to_client_packet::serialize);
	let up = convert(
		Root::PlayToClient,
		at("1.19.4"),
		at("1.20.2"),
		&login,
		&mut cx,
	)
	.unwrap();
	assert_eq!(names(Root::PlayToClient, "1.20.2", &up), ["login"]);
	let config = configuration_1_20_2(&mut cx).unwrap();
	let v1_20_2::configuration_to_client_packet::Ty::RegistryData(codec) = decode(
		&config[0],
		v1_20_2::configuration_to_client_packet::deserialize,
	) else {
		panic!("first configuration packet is not registry data")
	};
	let Payload::Compound(registries) = &codec.value else {
		panic!()
	};
	let damage = registries
		.iter()
		.find(|e| e.name == "minecraft:damage_type")
		.unwrap();
	let text = format!("{damage:?}");
	assert!(text.contains("minecraft:outside_border") && text.contains("minecraft:generic_kill"));
	for p in &config {
		convert(
			Root::ConfigurationToClient,
			at("1.20.2"),
			at("1.20.5"),
			p,
			&mut cx,
		)
		.unwrap();
	}
	let out = convert(
		Root::PlayToClient,
		at("1.19.4"),
		at("1.20.5"),
		&login,
		&mut cx,
	)
	.unwrap();
	assert_eq!(
		names(Root::PlayToClient, "1.20.5", &out),
		["login", "entity_update_attributes", "game_state_change"]
	);
	let v1_20_5::play_to_client_packet::Ty::Login(l) =
		decode(&out[0], v1_20_5::play_to_client_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(
		(
			l.entity_id,
			l.world_state.dimension,
			l.world_state.name.as_str()
		),
		(42, 0, "minecraft:overworld")
	);
}

#[test]
fn chat_becomes_nbt_text() {
	let mut cx = Session::new();
	let chat = v1_19_4::play_to_client_packet::Ty::SystemChat(
		v1_19_4::play_to_client_packet_system_chat::Ty {
			content: r#"{"text":"hi ","bold":true,"extra":[{"text":"there","color":"red"},"!"]}"#
				.to_owned(),
			is_action_bar: false,
		},
	);
	let out = convert(
		Root::PlayToClient,
		at("1.19.4"),
		at("1.20.5"),
		&encode(&chat, v1_19_4::play_to_client_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let v1_20_5::play_to_client_packet::Ty::SystemChat(c) =
		decode(&out[0], v1_20_5::play_to_client_packet::deserialize)
	else {
		panic!()
	};
	let Payload::Compound(root) = &c.content.value else {
		panic!("{c:?}")
	};
	assert!(root
		.iter()
		.any(|e| e.name == "bold" && e.value == Payload::Byte(1)));
	assert!(root
		.iter()
		.any(|e| e.name == "text" && e.value == Payload::String("hi ".to_owned())));
	let Some(Payload::List(extra)) = root.iter().find(|e| e.name == "extra").map(|e| &e.value)
	else {
		panic!()
	};
	assert!(matches!(&extra.items, ListItems::Compound(c) if c.len() == 2));
	let back = convert(
		Root::PlayToClient,
		at("1.20.5"),
		at("1.19.4"),
		&out[0],
		&mut cx,
	)
	.unwrap();
	let v1_19_4::play_to_client_packet::Ty::SystemChat(c) =
		decode(&back[0], v1_19_4::play_to_client_packet::deserialize)
	else {
		panic!()
	};
	let json: Value = serde_json::from_str(&c.content).unwrap();
	assert_eq!(json["bold"], Value::Bool(true));
	assert_eq!(json["extra"][0]["color"], "red");
	assert_eq!(json["extra"][1]["text"], "!");
}

#[test]
fn text_edge_cases() {
	let mut cx = Session::new();
	let mut nbt_of = |json: &str| {
		let chat = v1_19_4::play_to_client_packet::Ty::SystemChat(
			v1_19_4::play_to_client_packet_system_chat::Ty {
				content: json.to_owned(),
				is_action_bar: false,
			},
		);
		let out = convert(
			Root::PlayToClient,
			at("1.19.4"),
			at("1.20.3"),
			&encode(&chat, v1_19_4::play_to_client_packet::serialize),
			&mut cx,
		)
		.unwrap();
		let v1_20_3::play_to_client_packet::Ty::SystemChat(c) =
			decode(&out[0], v1_20_3::play_to_client_packet::deserialize)
		else {
			panic!()
		};
		c.content.value
	};
	assert_eq!(
		nbt_of("plain \u{a7}cred"),
		Payload::String("plain \u{a7}cred".to_owned())
	);
	assert_eq!(
		nbt_of(r#"{"text":"","extra":[]}"#),
		Payload::String(String::new())
	);
	let Payload::Compound(click) =
		nbt_of(r#"{"text":"p","clickEvent":{"action":"change_page","value":2}}"#)
	else {
		panic!()
	};
	let event = format!("{click:?}");
	assert!(event.contains(r#"String("2")"#), "{event}");
	let Payload::Compound(args) =
		nbt_of(r#"{"translate":"chat.type.text","with":[{"text":"Steve"},"hi",3]}"#)
	else {
		panic!()
	};
	let Some(Payload::List(with)) = args.iter().find(|e| e.name == "with").map(|e| &e.value) else {
		panic!()
	};
	assert!(matches!(&with.items, ListItems::Compound(c) if c.len() == 3));
}

fn slot_1_19_4(name: &str, nbt: Vec<Entry>) -> v1_19_4::slot::Ty {
	let id = registry::names("1.19.4", "items")
		.unwrap()
		.id(name)
		.unwrap() as i32;
	v1_19_4::slot::Ty::True(v1_19_4::slot_true::Ty {
		item_id: id,
		item_count: 3,
		nbt_data: Some(entry("", Payload::Compound(nbt))),
	})
}

fn strings(values: &[&str]) -> Payload {
	Payload::List(List::new(ListItems::String(
		values.iter().map(|s| (*s).to_owned()).collect(),
	)))
}

#[test]
fn items_become_components() {
	let mut cx = Session::new();
	let display = Payload::Compound(vec![
		entry(
			"Name",
			Payload::String(r#"{"text":"Excalibur","italic":false}"#.to_owned()),
		),
		entry("Lore", strings(&[r#"{"text":"old"}"#, r#""line""#])),
		entry("color", Payload::Int(0x123456)),
	]);
	let enchantment = vec![
		entry("id", Payload::String("minecraft:sharpness".to_owned())),
		entry("lvl", Payload::Short(5)),
	];
	let nbt = vec![
		entry("display", display),
		entry(
			"Enchantments",
			Payload::List(List::new(ListItems::Compound(vec![enchantment]))),
		),
		entry("Damage", Payload::Int(12)),
		entry("Unbreakable", Payload::Byte(1)),
		entry("CustomModelData", Payload::Int(77)),
		entry("HideFlags", Payload::Int(1)),
		entry("Custom", Payload::String("kept".to_owned())),
	];
	let packet =
		v1_19_4::play_to_client_packet::Ty::SetSlot(v1_19_4::play_to_client_packet_set_slot::Ty {
			window_id: 0,
			state_id: 1,
			slot: 36,
			item: slot_1_19_4("diamond_sword", nbt),
		});
	let out = convert(
		Root::PlayToClient,
		at("1.19.4"),
		at("1.20.5"),
		&encode(&packet, v1_19_4::play_to_client_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let v1_20_5::play_to_client_packet::Ty::SetSlot(s) =
		decode(&out[0], v1_20_5::play_to_client_packet::deserialize)
	else {
		panic!()
	};
	let v1_20_5::slot::Ty::Default(3, item) = s.item else {
		panic!("{:?}", s.item)
	};
	assert_eq!(
		registry::names("1.20.5", "items")
			.unwrap()
			.name(item.item_id as u32),
		Some("diamond_sword")
	);
	assert_eq!(item.added_component_count as usize, item.components.len());
	use v1_20_5::slot_component::Ty as C;
	let has = |f: &dyn Fn(&C) -> bool| item.components.iter().any(f);
	assert!(has(&|c| matches!(c, C::CustomName(_))));
	assert!(has(&|c| matches!(c, C::Lore(l) if l.len() == 2)));
	assert!(has(
		&|c| matches!(c, C::DyedColor(d) if d.color == 0x123456)
	));
	assert!(has(&|c| matches!(c, C::Damage(12))));
	assert!(has(&|c| matches!(c, C::Unbreakable(true))));
	assert!(has(&|c| matches!(c, C::CustomModelData(77))));
	let sharpness = registry::names("1.20.5", "enchantments")
		.unwrap()
		.id("sharpness")
		.unwrap() as i32;
	assert!(has(
		&|c| matches!(c, C::Enchantments(e) if !e.show_tooltip && e.enchantments.len() == 1 && e.enchantments[0].id == sharpness && e.enchantments[0].level == 5)
	));
	assert!(has(&|c| matches!(c, C::CustomData(_))));

	let click = |components: Vec<C>| {
		let slot = v1_20_5::slot::Ty::Default(
			3,
			v1_20_5::slot_default::Ty {
				item_id: item.item_id,
				added_component_count: components.len() as i32,
				removed_component_count: 0,
				components,
				remove_components: vec![],
			},
		);
		let packet = v1_20_5::play_to_server_packet::Ty::SetCreativeSlot(
			v1_20_5::play_to_server_packet_set_creative_slot::Ty {
				slot: 36,
				item: slot,
			},
		);
		let down = convert(
			Root::PlayToServer,
			at("1.20.5"),
			at("1.19.4"),
			&encode(&packet, v1_20_5::play_to_server_packet::serialize),
			&mut Session::new(),
		)
		.unwrap();
		let v1_19_4::play_to_server_packet::Ty::SetCreativeSlot(s) =
			decode(&down[0], v1_19_4::play_to_server_packet::deserialize)
		else {
			panic!()
		};
		let v1_19_4::slot::Ty::True(s) = s.item else {
			panic!()
		};
		assert_eq!(
			registry::names("1.19.4", "items")
				.unwrap()
				.name(s.item_id as u32),
			Some("diamond_sword")
		);
		format!("{:?}", s.nbt_data.unwrap())
	};
	let restored = click(item.components.clone());
	assert!(
		restored.contains("Custom")
			&& restored.contains("Excalibur")
			&& !restored.contains("pds2:legacy_nbt")
	);
	let rebuilt = click(
		item.components
			.iter()
			.filter(|c| !matches!(c, C::CustomData(_)))
			.cloned()
			.collect(),
	);
	for needle in [
		"Excalibur",
		"minecraft:sharpness",
		"Damage",
		"Unbreakable",
		"CustomModelData",
		"1193046",
		"HideFlags",
	] {
		assert!(rebuilt.contains(needle), "{needle} missing in {rebuilt}");
	}
}

fn section(states: &[u32]) -> Vec<u8> {
	let mut out = vec![0x10, 0x00, 4];
	out.push(states.len() as u8);
	for s in states {
		let mut v = *s;
		loop {
			let b = (v & 0x7f) as u8;
			v >>= 7;
			if v == 0 {
				out.push(b);
				break;
			}
			out.push(b | 0x80);
		}
	}
	out.push(0x80);
	out.push(0x02);
	for i in 0..256u64 {
		out.extend_from_slice(&(i % 3).wrapping_mul(0x1111_1111_1111_1111).to_be_bytes());
	}
	out.extend_from_slice(&[0, 0, 0]);
	out
}

fn first_palette(data: &[u8]) -> Vec<u32> {
	let mut at = 3;
	let mut read = || {
		let mut v = 0u32;
		for shift in (0..35).step_by(7) {
			let b = data[at];
			at += 1;
			v |= ((b & 0x7f) as u32) << shift;
			if b & 0x80 == 0 {
				break;
			}
		}
		v
	};
	let n = read();
	(0..n).map(|_| read()).collect()
}

#[test]
fn chunk_states_follow_block_renames() {
	let mut cx = Session::new();
	let b = registry::blocks("1.19.4").unwrap();
	let palette = [
		0,
		b.parse_state("minecraft:stone").unwrap(),
		b.parse_state("minecraft:grass").unwrap(),
	];
	let mut data = section(&palette);
	data.extend(section(&palette));
	let chunk = v1_19_4::play_to_client_packet::Ty::MapChunk(
		v1_19_4::play_to_client_packet_map_chunk::Ty {
			x: 1,
			z: -1,
			heightmaps: entry("", Payload::Compound(vec![])),
			chunk_data: data,
			block_entities: vec![],
			trust_edges: true,
			sky_light_mask: vec![],
			block_light_mask: vec![],
			empty_sky_light_mask: vec![],
			empty_block_light_mask: vec![],
			sky_light: vec![],
			block_light: vec![],
		},
	);
	let out = convert(
		Root::PlayToClient,
		at("1.19.4"),
		at("1.20.5"),
		&encode(&chunk, v1_19_4::play_to_client_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let v1_20_5::play_to_client_packet::Ty::MapChunk(c) =
		decode(&out[0], v1_20_5::play_to_client_packet::deserialize)
	else {
		panic!()
	};
	let n = registry::blocks("1.20.5").unwrap();
	let described: Vec<String> = first_palette(&c.chunk_data)
		.into_iter()
		.map(|s| n.describe(s).unwrap())
		.collect();
	assert_eq!(
		described,
		["minecraft:air", "minecraft:stone", "minecraft:short_grass"]
	);
	assert_eq!(c.chunk_data.len(), 2 * section(&palette).len());
}

#[test]
fn player_spawn_and_metadata() {
	let mut cx = Session::new();
	let spawn = v1_20::play_to_client_packet::Ty::NamedEntitySpawn(
		v1_20::play_to_client_packet_named_entity_spawn::Ty {
			entity_id: 9,
			player_uuid: 1,
			x: 0.0,
			y: 64.0,
			z: 0.0,
			yaw: 10,
			pitch: 20,
		},
	);
	let out = convert(
		Root::PlayToClient,
		at("1.20"),
		at("1.20.5"),
		&encode(&spawn, v1_20::play_to_client_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let v1_20_5::play_to_client_packet::Ty::SpawnEntity(s) =
		decode(&out[0], v1_20_5::play_to_client_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(
		registry::names("1.20.5", "entities")
			.unwrap()
			.name(s.r#type as u32),
		Some("player")
	);
	assert_eq!((s.yaw, s.pitch, s.head_pitch), (10, 20, 10));
	let meta = v1_20_3::play_to_client_packet::Ty::EntityMetadata(
		v1_20_3::play_to_client_packet_entity_metadata::Ty {
			entity_id: 9,
			metadata: vec![
				decode(
					&[10, 1, 0x80, 0x80, 0x04],
					v1_20_3::entity_metadata_items::deserialize,
				),
				decode(
					&[9, 3, 0x41, 0xa0, 0, 0],
					v1_20_3::entity_metadata_items::deserialize,
				),
			],
		},
	);
	let out = convert(
		Root::PlayToClient,
		at("1.20.3"),
		at("1.20.5"),
		&encode(&meta, v1_20_3::play_to_client_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let v1_20_5::play_to_client_packet::Ty::EntityMetadata(m) =
		decode(&out[0], v1_20_5::play_to_client_packet::deserialize)
	else {
		panic!()
	};
	assert!(
		matches!(&m.metadata[0].value, v1_20_5::entity_metadata_entry_value::Ty::Particles(p) if p.len() == 1)
	);
	assert!(
		matches!(&m.metadata[1].value, v1_20_5::entity_metadata_entry_value::Ty::Float(h) if *h == 20.0)
	);
}

#[test]
fn commands_and_clicks_reach_1_19_4() {
	let mut cx = Session::new();
	let command = v1_20_5::play_to_server_packet::Ty::ChatCommand("spawn".to_owned());
	let down = convert(
		Root::PlayToServer,
		at("1.20.5"),
		at("1.19.4"),
		&encode(&command, v1_20_5::play_to_server_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let v1_19_4::play_to_server_packet::Ty::ChatCommand(c) =
		decode(&down[0], v1_19_4::play_to_server_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(c.command, "spawn");
	let stone = registry::names("1.20.5", "items")
		.unwrap()
		.id("stone")
		.unwrap() as i32;
	let click = v1_20_5::play_to_server_packet::Ty::WindowClick(
		v1_20_5::play_to_server_packet_window_click::Ty {
			window_id: 0,
			state_id: 3,
			slot: 36,
			mouse_button: 0,
			mode: 0,
			changed_slots: vec![],
			cursor_item: v1_20_5::slot::Ty::Default(
				64,
				v1_20_5::slot_default::Ty {
					item_id: stone,
					added_component_count: 0,
					removed_component_count: 0,
					components: vec![],
					remove_components: vec![],
				},
			),
		},
	);
	let down = convert(
		Root::PlayToServer,
		at("1.20.5"),
		at("1.19.4"),
		&encode(&click, v1_20_5::play_to_server_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let v1_19_4::play_to_server_packet::Ty::WindowClick(c) =
		decode(&down[0], v1_19_4::play_to_server_packet::deserialize)
	else {
		panic!()
	};
	let v1_19_4::slot::Ty::True(s) = c.cursor_item else {
		panic!()
	};
	assert_eq!(
		(
			registry::names("1.19.4", "items")
				.unwrap()
				.name(s.item_id as u32),
			s.item_count,
			s.nbt_data
		),
		(Some("stone"), 64, None)
	);
}

#[test]
fn newer_server_configuration_feeds_old_clients() {
	let mut client = Session::new();
	let login = encode(&login_1_19_4(), v1_19_4::play_to_client_packet::serialize);
	convert(
		Root::PlayToClient,
		at("1.19.4"),
		at("1.20.2"),
		&login,
		&mut client,
	)
	.unwrap();
	let mut server = Vec::new();
	for p in &configuration_1_20_2(&mut client).unwrap() {
		server.extend(
			convert(
				Root::ConfigurationToClient,
				at("1.20.2"),
				at("1.20.5"),
				p,
				&mut client,
			)
			.unwrap(),
		);
	}
	let join = convert(
		Root::PlayToClient,
		at("1.19.4"),
		at("1.20.5"),
		&login,
		&mut client,
	)
	.unwrap()
	.remove(0);

	let mut cx = Session::new();
	let mut seen = Vec::new();
	let mut replies = Vec::new();
	for p in &server {
		for down in convert(
			Root::ConfigurationToClient,
			at("1.20.5"),
			at("1.20.2"),
			p,
			&mut cx,
		)
		.unwrap()
		{
			seen.push(inspect(Root::ConfigurationToClient, at("1.20.2"), &down).unwrap());
			replies.extend(configuration_capture_1_20_2(&mut cx, &down).unwrap());
		}
	}
	assert_eq!(
		seen,
		[
			"feature_flags",
			"tags",
			"registry_data",
			"finish_configuration"
		]
	);
	let outgoing = cx.take_outgoing();
	assert_eq!(outgoing.len(), 1);
	assert_eq!(
		(outgoing[0].root, outgoing[0].version),
		(Root::ConfigurationToServer, at("1.20.5"))
	);
	assert!(
		matches!(decode(&outgoing[0].data, v1_20_5::configuration_to_server_packet::deserialize), v1_20_5::configuration_to_server_packet::Ty::SelectKnownPacks(p) if p.is_empty())
	);
	assert_eq!(
		names(Root::ConfigurationToServer, "1.20.2", &replies),
		["finish_configuration"]
	);

	let out = convert(Root::PlayToClient, at("1.20.5"), at("1.20"), &join, &mut cx).unwrap();
	assert_eq!(
		names(Root::PlayToClient, "1.20", &out),
		["login", "feature_flags", "tags"]
	);
	let v1_20::play_to_client_packet::Ty::Login(l) =
		decode(&out[0], v1_20::play_to_client_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(
		(l.entity_id, l.world_type.as_str()),
		(42, "minecraft:overworld")
	);
	let Payload::Compound(registries) = &l.dimension_codec.value else {
		panic!()
	};
	for id in [
		"minecraft:dimension_type",
		"minecraft:worldgen/biome",
		"minecraft:damage_type",
		"minecraft:trim_material",
	] {
		assert!(registries.iter().any(|e| e.name == id), "{id} missing");
	}
	assert!(!registries
		.iter()
		.any(|e| e.name == "minecraft:wolf_variant"));
	let v1_20::play_to_client_packet::Ty::Tags(tags) =
		decode(&out[2], v1_20::play_to_client_packet::deserialize)
	else {
		panic!()
	};
	let blocks = tags
		.iter()
		.find(|t| t.tag_type == "minecraft:block")
		.unwrap();
	let pickaxe = blocks
		.tags
		.iter()
		.find(|t| t.tag_name == "minecraft:mineable/pickaxe")
		.unwrap();
	let stone = registry::blocks("1.20")
		.unwrap()
		.list
		.iter()
		.find(|b| b.name == "stone")
		.unwrap()
		.id as i32;
	assert!(pickaxe.entries.contains(&stone));
}

#[test]
fn old_client_handshake_with_newer_server() {
	let mut cx = Session::new();
	let hello = v1_20::login_to_server_packet::Ty::LoginStart(
		v1_20::login_to_server_packet_login_start::Ty {
			username: "Notch".to_owned(),
			player_uuid: None,
		},
	);
	let up = convert(
		Root::LoginToServer,
		at("1.20"),
		at("1.20.2"),
		&encode(&hello, v1_20::login_to_server_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let v1_20_2::login_to_server_packet::Ty::LoginStart(s) =
		decode(&up[0], v1_20_2::login_to_server_packet::deserialize)
	else {
		panic!()
	};
	assert_eq!(s.player_uuid, 0xb50ad385_829d_3141_a216_7e7d7539ba7f);

	let batch = v1_20_2::play_to_client_packet::Ty::ChunkBatchFinished(4);
	let out = convert(
		Root::PlayToClient,
		at("1.20.2"),
		at("1.20"),
		&encode(&batch, v1_20_2::play_to_client_packet::serialize),
		&mut cx,
	)
	.unwrap();
	assert!(out.is_empty());
	let cookie = v1_20_5::play_to_client_packet::Ty::CookieRequest("k".to_owned());
	let out = convert(
		Root::PlayToClient,
		at("1.20.5"),
		at("1.20.3"),
		&encode(&cookie, v1_20_5::play_to_client_packet::serialize),
		&mut cx,
	)
	.unwrap();
	assert!(out.is_empty());
	let outgoing = cx.take_outgoing();
	assert_eq!(
		outgoing
			.iter()
			.map(|o| (o.root, o.version))
			.collect::<Vec<_>>(),
		[
			(Root::PlayToServer, at("1.20.2")),
			(Root::PlayToServer, at("1.20.5"))
		]
	);
	assert!(matches!(
		decode(
			&outgoing[0].data,
			v1_20_2::play_to_server_packet::deserialize
		),
		v1_20_2::play_to_server_packet::Ty::ChunkBatchReceived(_)
	));
	assert!(
		matches!(decode(&outgoing[1].data, v1_20_5::play_to_server_packet::deserialize), v1_20_5::play_to_server_packet::Ty::CookieResponse(r) if r.key == "k" && r.value.is_none())
	);
}

#[test]
fn item_display_rotates_both_ways() {
	let mut cx = Session::new();
	let display = registry::names("1.20", "entities")
		.unwrap()
		.id("item_display")
		.unwrap() as i32;
	let spawn = v1_20::play_to_client_packet::Ty::SpawnEntity(
		v1_20::play_to_client_packet_spawn_entity::Ty {
			entity_id: 5,
			object_uuid: 2,
			r#type: display,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			head_pitch: 0,
			object_data: 0,
			velocity: v1_20::vec3i16::Ty { x: 0, y: 0, z: 0 },
		},
	);
	convert(
		Root::PlayToClient,
		at("1.20"),
		at("1.19.4"),
		&encode(&spawn, v1_20::play_to_client_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let meta = |entries: Vec<v1_20::entity_metadata_items::Ty>| {
		let p = v1_20::play_to_client_packet::Ty::EntityMetadata(
			v1_20::play_to_client_packet_entity_metadata::Ty {
				entity_id: 5,
				metadata: entries,
			},
		);
		encode(&p, v1_20::play_to_client_packet::serialize)
	};
	let rotations = |out: &[Vec<u8>]| -> Vec<(u8, [f32; 4])> {
		let v1_19_4::play_to_client_packet::Ty::EntityMetadata(m) =
			decode(&out[0], v1_19_4::play_to_client_packet::deserialize)
		else {
			panic!()
		};
		m.metadata
			.into_iter()
			.filter_map(|e| match e.value {
				v1_19_4::entity_metadata_item::Ty::Quaternion(q) => {
					Some((e.key, [q.x, q.y, q.z, q.w]))
				}
				_ => None,
			})
			.collect()
	};
	let out = convert(
		Root::PlayToClient,
		at("1.20"),
		at("1.19.4"),
		&meta(vec![decode(
			&[0, 0, 0],
			v1_20::entity_metadata_items::deserialize,
		)]),
		&mut cx,
	)
	.unwrap();
	assert_eq!(rotations(&out), [(12, [0.0, 1.0, 0.0, 0.0])]);
	let mut q = vec![12, 27];
	for f in [0.0f32, 0.0, 0.0, 1.0] {
		q.extend(f.to_be_bytes());
	}
	let out = convert(
		Root::PlayToClient,
		at("1.20"),
		at("1.19.4"),
		&meta(vec![decode(&q, v1_20::entity_metadata_items::deserialize)]),
		&mut cx,
	)
	.unwrap();
	assert_eq!(rotations(&out), [(12, [-0.0, 1.0, 0.0, -0.0])]);
	let out = convert(
		Root::PlayToClient,
		at("1.20"),
		at("1.19.4"),
		&meta(vec![decode(
			&[0, 0, 0],
			v1_20::entity_metadata_items::deserialize,
		)]),
		&mut cx,
	)
	.unwrap();
	assert!(rotations(&out).is_empty());
}

#[test]
fn hover_items_switch_between_tag_and_components() {
	let mut cx = Session::new();
	let tag = r#"{display:{Name:'{"text":"Blade"}'},Enchantments:[{id:"minecraft:sharpness",lvl:5s}],Custom:[I;1,2]}"#;
	let json = serde_json::json!({
		"text": "Gave ",
		"extra": [{"text": "[Sword]", "hoverEvent": {"action": "show_item", "contents": {"id": "minecraft:diamond_sword", "count": 1, "tag": tag}}}],
	});
	let chat = v1_20_2::play_to_client_packet::Ty::SystemChat(
		v1_20_2::play_to_client_packet_system_chat::Ty {
			content: json.to_string(),
			is_action_bar: false,
		},
	);
	let out = convert(
		Root::PlayToClient,
		at("1.20.2"),
		at("1.20.5"),
		&encode(&chat, v1_20_2::play_to_client_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let v1_20_5::play_to_client_packet::Ty::SystemChat(s) =
		decode(&out[0], v1_20_5::play_to_client_packet::deserialize)
	else {
		panic!()
	};
	let text = format!("{:?}", s.content);
	for needle in [
		"components",
		"minecraft:custom_name",
		"minecraft:enchantments",
		"minecraft:custom_data",
	] {
		assert!(text.contains(needle), "{needle} missing in {text}");
	}
	assert!(!text.contains("\"tag\""));
	let back = convert(
		Root::PlayToClient,
		at("1.20.5"),
		at("1.20.2"),
		&out[0],
		&mut cx,
	)
	.unwrap();
	let v1_20_2::play_to_client_packet::Ty::SystemChat(s) =
		decode(&back[0], v1_20_2::play_to_client_packet::deserialize)
	else {
		panic!()
	};
	let value: Value = serde_json::from_str(&s.content).unwrap();
	let snbt = value["extra"][0]["hoverEvent"]["contents"]["tag"]
		.as_str()
		.unwrap();
	for needle in ["Blade", "minecraft:sharpness", "lvl:5s", "[I;1,2]"] {
		assert!(snbt.contains(needle), "{needle} missing in {snbt}");
	}
}

#[test]
fn llama_carpet_becomes_swag() {
	let mut cx = Session::new();
	let llama = registry::names("1.20.5", "entities")
		.unwrap()
		.id("llama")
		.unwrap() as i32;
	let spawn = v1_20_5::play_to_client_packet::Ty::SpawnEntity(
		v1_20_5::play_to_client_packet_spawn_entity::Ty {
			entity_id: 7,
			object_uuid: 3,
			r#type: llama,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			head_pitch: 0,
			object_data: 0,
			velocity: v1_20_5::vec3i16::Ty { x: 0, y: 0, z: 0 },
		},
	);
	convert(
		Root::PlayToClient,
		at("1.20.5"),
		at("1.20.3"),
		&encode(&spawn, v1_20_5::play_to_client_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let carpet = registry::names("1.20.5", "items")
		.unwrap()
		.id("red_carpet")
		.unwrap() as i32;
	let item = v1_20_5::slot::Ty::Default(
		1,
		v1_20_5::slot_default::Ty {
			item_id: carpet,
			added_component_count: 0,
			removed_component_count: 0,
			components: vec![],
			remove_components: vec![],
		},
	);
	let equip = v1_20_5::play_to_client_packet::Ty::EntityEquipment(
		v1_20_5::play_to_client_packet_entity_equipment::Ty {
			entity_id: 7,
			equipments: vec![
				v1_20_5::play_to_client_packet_entity_equipment_equipments_items::Ty {
					slot: 6,
					item,
				},
			],
		},
	);
	let out = convert(
		Root::PlayToClient,
		at("1.20.5"),
		at("1.20.3"),
		&encode(&equip, v1_20_5::play_to_client_packet::serialize),
		&mut cx,
	)
	.unwrap();
	let v1_20_3::play_to_client_packet::Ty::EntityMetadata(m) =
		decode(&out[0], v1_20_3::play_to_client_packet::deserialize)
	else {
		panic!()
	};
	let swag = registry::entity_metadata_keys(
		"1.20.3",
		registry::names("1.20.3", "entities")
			.unwrap()
			.id("llama")
			.unwrap(),
	)
	.unwrap()
	.iter()
	.position(|k| k == "swag")
	.unwrap();
	assert_eq!(m.metadata.len(), 1);
	assert_eq!(m.metadata[0].key as usize, swag);
	assert!(matches!(
		m.metadata[0].value,
		v1_20_3::entity_metadata_item::Ty::Int(14)
	));
}
