use minecraft_versions::gen::convert::{step, v1_7, v1_8, Root};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::Session;
use minecraft_versions::Version;

type C7 = v1_7::play_to_client_packet::Ty;
type C8 = v1_8::play_to_client_packet::Ty;
type S7 = v1_7::play_to_server_packet::Ty;
type S8 = v1_8::play_to_server_packet::Ty;

macro_rules! codec {
	($enc:ident, $dec:ident, $m:ident :: $t:ident) => {
		fn $enc(p: &$m::$t::Ty) -> Vec<u8> {
			let mut o = Output::new();
			$m::$t::serialize(p, &mut o).unwrap();
			o.into_inner()
		}

		fn $dec(b: &[u8]) -> $m::$t::Ty {
			let mut input = Input::new(b);
			let p = $m::$t::deserialize(&mut input).unwrap();
			assert_eq!(input.remaining(), 0, "{p:?}");
			p
		}
	};
}

codec!(enc_c8, dec_c8, v1_8::play_to_client_packet);
codec!(enc_c7, dec_c7, v1_7::play_to_client_packet);
codec!(enc_s8, dec_s8, v1_8::play_to_server_packet);
codec!(enc_s7, dec_s7, v1_7::play_to_server_packet);

fn down(p: C8, cx: &mut Session) -> Vec<C7> {
	step(
		Root::PlayToClient,
		Version::V1_8,
		Version::V1_7,
		&enc_c8(&p),
		cx,
	)
	.unwrap()
	.iter()
	.map(|b| dec_c7(b))
	.collect()
}

fn up(p: C7, cx: &mut Session) -> Vec<C8> {
	step(
		Root::PlayToClient,
		Version::V1_7,
		Version::V1_8,
		&enc_c7(&p),
		cx,
	)
	.unwrap()
	.iter()
	.map(|b| dec_c8(b))
	.collect()
}

fn serve(p: S7, cx: &mut Session) -> Vec<S8> {
	step(
		Root::PlayToServer,
		Version::V1_7,
		Version::V1_8,
		&enc_s7(&p),
		cx,
	)
	.unwrap()
	.iter()
	.map(|b| dec_s8(b))
	.collect()
}

fn serve_down(p: S8, cx: &mut Session) -> Vec<S7> {
	step(
		Root::PlayToServer,
		Version::V1_8,
		Version::V1_7,
		&enc_s8(&p),
		cx,
	)
	.unwrap()
	.iter()
	.map(|b| dec_s7(b))
	.collect()
}

fn one<T: std::fmt::Debug>(v: Vec<T>) -> T {
	assert_eq!(v.len(), 1, "{v:?}");
	v.into_iter().next().unwrap()
}

fn dump() -> Vec<Vec<u8>> {
	let bytes = std::fs::read(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/tests/data/1.8-toClient.bin"
	))
	.unwrap();
	let mut out = Vec::new();
	let mut i = 0;
	while i + 4 <= bytes.len() {
		let len = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
		out.push(bytes[i + 4..i + 4 + len].to_vec());
		i += 4 + len;
	}
	out
}

#[test]
fn real_traffic_survives_a_round_trip_through_1_7() {
	let (mut there, mut back) = (Session::new(), Session::new());
	let mut chunks = 0;
	for packet in dump() {
		let original = dec_c8(&packet);
		let old = step(
			Root::PlayToClient,
			Version::V1_8,
			Version::V1_7,
			&packet,
			&mut there,
		)
		.unwrap();
		for b in &old {
			let again = step(
				Root::PlayToClient,
				Version::V1_7,
				Version::V1_8,
				b,
				&mut back,
			)
			.unwrap();
			if matches!(
				original,
				C8::MapChunk(_)
					| C8::MapChunkBulk(_)
					| C8::KeepAlive(_)
					| C8::EntityHeadRotation(_)
			) {
				chunks += 1;
				assert_eq!(again, vec![packet.clone()], "{original:?}");
			}
		}
	}
	assert!(chunks > 50);
}

#[test]
fn login_and_join() {
	let mut cx = Session::new();
	let login = v1_8::login_to_client_packet::Ty::Compress(256);
	let mut o = Output::new();
	v1_8::login_to_client_packet::serialize(&login, &mut o).unwrap();
	assert!(step(
		Root::LoginToClient,
		Version::V1_8,
		Version::V1_7,
		&o.into_inner(),
		&mut cx
	)
	.unwrap()
	.is_empty());
	let join = C8::Login(v1_8::play_to_client_packet_login::Ty {
		entity_id: 7,
		game_mode: 1,
		dimension: -1,
		difficulty: 2,
		max_players: 20,
		level_type: "default".into(),
		reduced_debug_info: true,
	});
	let C7::Login(l) = one(down(join.clone(), &mut cx)) else {
		panic!()
	};
	assert_eq!(
		(l.entity_id, l.game_mode, l.dimension, l.level_type.as_str()),
		(7, 1, -1, "default")
	);
	let C8::Login(back) = one(up(C7::Login(l), &mut cx)) else {
		panic!()
	};
	assert!(!back.reduced_debug_info);
	assert_eq!(down(C8::KeepAlive(99), &mut cx), vec![C7::KeepAlive(99)]);
	assert_eq!(serve(S7::KeepAlive(99), &mut cx), vec![S8::KeepAlive(99)]);
}

#[test]
fn block_changes_use_1_7_ids_and_record_layout() {
	let mut cx = Session::new();
	let change = C8::BlockChange(v1_8::play_to_client_packet_block_change::Ty {
		location: v1_8::position::Ty {
			x: -5,
			y: 70,
			z: 12,
		},
		r#type: 169 << 4,
	});
	let C7::BlockChange(b) = one(down(change, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		(
			b.location.x,
			b.location.y,
			b.location.z,
			b.r#type,
			b.metadata
		),
		(-5, 70, 12, 89, 0)
	);
	let multi = C8::MultiBlockChange(v1_8::play_to_client_packet_multi_block_change::Ty {
		chunk_x: 1,
		chunk_z: 2,
		records: vec![
			v1_8::play_to_client_packet_multi_block_change_records_items::Ty {
				horizontal_pos: 0x3a,
				y: 64,
				block_id: 35 << 4 | 14,
			},
		],
	});
	let old = down(multi.clone(), &mut cx);
	let bytes = enc_c7(&old[0]);
	assert_eq!(
		&bytes[bytes.len() - 10..],
		&[0, 1, 0, 0, 0, 4, 0x3a, 64, 0x02, 0x3e]
	);
	assert_eq!(up(old[0].clone(), &mut cx), vec![multi]);
}

#[test]
fn world_effects_and_particles() {
	let mut cx = Session::new();
	let particles = |data| {
		C8::WorldParticles(v1_8::play_to_client_packet_world_particles::Ty {
			long_distance: true,
			x: 1.0,
			y: 2.0,
			z: 3.0,
			offset_x: 0.0,
			offset_y: 0.0,
			offset_z: 0.0,
			particle_data: 0.5,
			particles: 10,
			data,
		})
	};
	use v1_8::play_to_client_packet_world_particles_data::Ty as D;
	let name = |p, cx: &mut Session| match down(p, cx).as_slice() {
		[C7::WorldParticles(w)] => w.particle_name.clone(),
		o => format!("{o:?}"),
	};
	assert_eq!(name(particles(D::Default(26)), &mut cx), "flame");
	assert_eq!(
		name(particles(D::V37(vec![1 | 3 << 12])), &mut cx),
		"blockcrack_1_0"
	);
	assert_eq!(
		name(particles(D::V36(vec![409, 0])), &mut cx),
		"iconcrack_406_0"
	);
	assert_eq!(name(particles(D::Default(41)), &mut cx), "[]");
	let event = C8::WorldEvent(v1_8::play_to_client_packet_world_event::Ty {
		effect_id: 2001,
		location: v1_8::position::Ty { x: 0, y: 5, z: 0 },
		data: 168 | 2 << 12,
		global: false,
	});
	let C7::WorldEvent(e) = one(down(event, &mut cx)) else {
		panic!()
	};
	assert_eq!(e.data, 98 | 1 << 12);
}

#[test]
fn entities_spawn_as_1_7_entities() {
	let mut cx = Session::new();
	use v1_8::play_to_client_packet_spawn_entity_velocity::Ty as V8;
	let object = |id, ty, velocity| {
		C8::SpawnEntity(v1_8::play_to_client_packet_spawn_entity::Ty {
			entity_id: id,
			r#type: ty,
			x: 320,
			y: 2048,
			z: -320,
			pitch: 0,
			yaw: 0,
			velocity,
		})
	};
	let sand = object(
		1,
		70,
		V8::Default(12 | 1 << 12, v1_8::vec3i16::Ty { x: 0, y: -10, z: 0 }),
	);
	let C7::SpawnEntity(s) = one(down(sand, &mut cx)) else {
		panic!()
	};
	assert_eq!(s.y, 2048 + 16);
	assert_eq!(
		v1_7::play_to_client_packet_spawn_entity_object_data::virtual_tag(&s.object_data),
		12 | 1 << 16
	);
	let teleport = C8::EntityTeleport(v1_8::play_to_client_packet_entity_teleport::Ty {
		entity_id: 1,
		x: 0,
		y: 100,
		z: 0,
		yaw: 0,
		pitch: 0,
		on_ground: true,
	});
	let C7::EntityTeleport(t) = one(down(teleport, &mut cx)) else {
		panic!()
	};
	assert_eq!(t.y, 116);
	let C7::SpawnEntity(f) = one(down(object(2, 71, V8::V0), &mut cx)) else {
		panic!()
	};
	assert_eq!((f.x, f.z, f.yaw), (320, -352, 0));
	assert!(down(object(3, 78, V8::V0), &mut cx).is_empty());
	let meta = C8::EntityMetadata(v1_8::play_to_client_packet_entity_metadata::Ty {
		entity_id: 3,
		metadata: vec![],
	});
	assert!(down(meta, &mut cx).is_empty());
	let rabbit = C8::SpawnEntityLiving(v1_8::play_to_client_packet_spawn_entity_living::Ty {
		entity_id: 4,
		r#type: 101,
		x: 0,
		y: 0,
		z: 0,
		yaw: 0,
		pitch: 0,
		head_pitch: 0,
		velocity: v1_8::vec3i16::Ty { x: 0, y: 0, z: 0 },
		metadata: vec![],
	});
	let C7::SpawnEntityLiving(r) = one(down(rabbit, &mut cx)) else {
		panic!()
	};
	assert_eq!(r.r#type, 93);
	let many = down(C8::EntityDestroy((0..300).collect()), &mut cx);
	assert_eq!(
		many.iter()
			.map(|p| match p {
				C7::EntityDestroy(ids) => ids.len(),
				_ => 0,
			})
			.collect::<Vec<_>>(),
		vec![127, 127, 46]
	);
}

#[test]
fn players_get_names_and_skins_from_the_tab_list() {
	let mut cx = Session::new();
	use v1_8::play_to_client_packet_player_info_action::Ty as Action;
	use v1_8::play_to_client_packet_player_info_data_items_anon::Ty as A;
	let uuid = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdefu128;
	let info = |action, entry| {
		C8::PlayerInfo(v1_8::play_to_client_packet_player_info::Ty {
			action,
			data: vec![v1_8::play_to_client_packet_player_info_data_items::Ty {
				uuid,
				anon: entry,
			}],
		})
	};
	let add = info(
		Action::AddPlayer,
		A::AddPlayer(v1_8::play_to_client_packet_player_info_data_items_anon_add_player::Ty {
			name: "Notch".into(),
			properties: vec![v1_8::play_to_client_packet_player_info_data_items_anon_add_player_properties_items::Ty {
				name: "textures".into(),
				value: "abc".into(),
				signature: Some("sig".into()),
			}],
			gamemode: 0,
			ping: 42,
			display_name: Some(r#"{"text":"The Notch","color":"gold"}"#.into()),
		}),
	);
	let C7::PlayerInfo(p) = one(down(add, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		(p.player_name.as_str(), p.online, p.ping),
		("\u{a7}6The Notch", true, 42)
	);
	let spawn = C8::NamedEntitySpawn(v1_8::play_to_client_packet_named_entity_spawn::Ty {
		entity_id: 9,
		player_uuid: uuid,
		x: 1,
		y: 2,
		z: 3,
		yaw: 0,
		pitch: 0,
		current_item: 0,
		metadata: vec![],
	});
	let C7::NamedEntitySpawn(s) = one(down(spawn, &mut cx)) else {
		panic!()
	};
	assert_eq!(s.player_uuid, "01234567-89ab-cdef-0123-456789abcdef");
	assert_eq!(
		(
			s.player_name.as_str(),
			s.data.len(),
			s.data[0].signature.as_str()
		),
		("Notch", 1, "sig")
	);
	let C7::PlayerInfo(gone) = one(down(info(Action::RemovePlayer, A::RemovePlayer), &mut cx))
	else {
		panic!()
	};
	assert_eq!(
		(gone.player_name.as_str(), gone.online),
		("\u{a7}6The Notch", false)
	);
	let back = up(C7::NamedEntitySpawn(s), &mut Session::new());
	assert!(
		matches!(back.as_slice(), [C8::PlayerInfo(_), C8::NamedEntitySpawn(n)] if n.player_uuid == uuid)
	);
}

#[test]
fn chat_titles_and_signs() {
	let mut cx = Session::new();
	let chat = |position| {
		C8::Chat(v1_8::play_to_client_packet_chat::Ty {
			message: "\"hi\"".into(),
			position,
		})
	};
	assert_eq!(down(chat(0), &mut cx), vec![C7::Chat("\"hi\"".into())]);
	assert!(down(chat(2), &mut cx).is_empty());
	let sign = C8::UpdateSign(v1_8::play_to_client_packet_update_sign::Ty {
		location: v1_8::position::Ty { x: 1, y: 2, z: 3 },
		text1: r#"{"text":"short"}"#.into(),
		text2: r#""a line that is far too long for 1.7""#.into(),
		text3: "\"\"".into(),
		text4: r#"{"text":"red","color":"red"}"#.into(),
	});
	let C7::UpdateSign(s) = one(down(sign, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		[s.text1.as_str(), &s.text2, &s.text3, &s.text4],
		["short", "a line that is ", "", "\u{a7}cred"]
	);
	let edited = S7::UpdateSign(v1_7::play_to_server_packet_update_sign::Ty {
		location: v1_7::position_isi::Ty { x: 1, y: 2, z: 3 },
		text1: "say \"hi\"".into(),
		text2: String::new(),
		text3: String::new(),
		text4: String::new(),
	});
	let S8::UpdateSign(u) = one(serve(edited, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		(u.text1.as_str(), u.text2.as_str()),
		(r#""say \"hi\"""#, "\"\"")
	);
}

#[test]
fn windows_items_and_enchanting() {
	let mut cx = Session::new();
	use v1_8::play_to_client_packet_open_window_entity_id::Ty as E8;
	let open = C8::OpenWindow(v1_8::play_to_client_packet_open_window::Ty {
		window_id: 3,
		inventory_type: "minecraft:enchanting_table".into(),
		window_title: r#"{"translate":"container.enchant"}"#.into(),
		slot_count: 0,
		entity_id: E8::Default("minecraft:enchanting_table".into()),
	});
	let C7::OpenWindow(w) = one(down(open, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		v1_7::play_to_client_packet_open_window_entity_id::virtual_tag(&w.entity_id),
		4
	);
	assert_eq!(
		(w.window_title.as_str(), w.use_provided_title),
		("container.enchant", false)
	);
	let stack = |id, damage| {
		v1_8::slot::Ty::Default(
			id,
			v1_8::slot_default::Ty {
				item_count: 1,
				item_damage: damage,
				nbt_data: None,
			},
		)
	};
	let items = C8::WindowItems(v1_8::play_to_client_packet_window_items::Ty {
		window_id: 3,
		items: vec![stack(276, 0), stack(351, 4), stack(1, 3)],
	});
	let C7::WindowItems(i) = one(down(items, &mut cx)) else {
		panic!()
	};
	assert_eq!(i.items.len(), 2);
	assert!(
		matches!(&i.items[1], v1_7::slot::Ty::Default(1, s) if s.item_damage == 0 && s.nbt_data.is_some())
	);
	let set = |slot| {
		C8::SetSlot(v1_8::play_to_client_packet_set_slot::Ty {
			window_id: 3,
			slot,
			item: v1_8::slot::Ty::_1,
		})
	};
	assert!(down(set(1), &mut cx).is_empty());
	assert!(matches!(down(set(5), &mut cx).as_slice(), [C7::SetSlot(s)] if s.slot == 4));
	let click = S7::WindowClick(v1_7::play_to_server_packet_window_click::Ty {
		window_id: 3,
		slot: 4,
		mouse_button: 0,
		action: 1,
		mode: 0,
		item: i.items[1].clone(),
	});
	let S8::WindowClick(c) = one(serve(click, &mut cx)) else {
		panic!()
	};
	assert_eq!((c.slot, c.item.clone()), (5, stack(1, 3)));
}

#[test]
fn player_movement_from_1_7() {
	let mut cx = Session::new();
	let moved = S7::Position(v1_7::play_to_server_packet_position::Ty {
		x: 10.0,
		y: 64.0,
		stance: 65.62,
		z: -3.0,
		on_ground: true,
	});
	let S8::Position(p) = one(serve(moved, &mut cx)) else {
		panic!()
	};
	assert_eq!((p.x, p.y, p.z, p.on_ground), (10.0, 64.0, -3.0, true));
	let S7::Position(back) = one(serve_down(S8::Position(p), &mut cx)) else {
		panic!()
	};
	assert_eq!((back.y, back.stance), (64.0, 64.0 + 1.6200000047683716));
	let teleport = C8::Position(v1_8::play_to_client_packet_position::Ty {
		x: 1.0,
		y: 0.0,
		z: 0.0,
		yaw: 0.0,
		pitch: 0.0,
		flags: 0b111,
	});
	let C7::Position(t) = one(down(teleport, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		(t.x, t.y, t.z, t.on_ground),
		(11.0, 64.0 + 1.6200000047683716, -3.0, true)
	);
	let place = S7::BlockPlace(v1_7::play_to_server_packet_block_place::Ty {
		location: v1_7::position_ibi::Ty {
			x: -1,
			y: 255,
			z: -1,
		},
		direction: -1,
		held_item: v1_7::slot::Ty::_1,
		cursor_x: 0,
		cursor_y: 0,
		cursor_z: 0,
	});
	let S8::BlockPlace(b) = one(serve(place, &mut cx)) else {
		panic!()
	};
	assert_eq!((b.location.x, b.location.y, b.location.z), (-1, -1, -1));
	use v1_7::play_to_server_packet_entity_action_action_id::Ty as A7;
	use v1_8::play_to_server_packet_entity_action_action_id::Ty as A8;
	let action = |id| {
		S7::EntityAction(v1_7::play_to_server_packet_entity_action::Ty {
			entity_id: 7,
			action_id: id,
			jump_boost: 0,
		})
	};
	assert!(
		matches!(serve(action(A7::OpenInventory), &mut cx).as_slice(), [S8::EntityAction(a)] if a.action_id == A8::OpenInventory)
	);
	assert!(
		matches!(serve(action(A7::StartSneaking), &mut cx).as_slice(), [S8::EntityAction(a)] if a.action_id == A8::StartSneaking)
	);
	let swing = |animation| {
		S7::ArmAnimation(v1_7::play_to_server_packet_arm_animation::Ty {
			entity_id: 7,
			animation,
		})
	};
	assert_eq!(serve(swing(1), &mut cx).len(), 1);
	assert!(serve(swing(104), &mut cx).is_empty());
	let brand = S7::CustomPayload(v1_7::play_to_server_packet_custom_payload::Ty {
		channel: "MC|Brand".into(),
		data: b"vanilla".to_vec(),
	});
	let S8::CustomPayload(c) = one(serve(brand, &mut cx)) else {
		panic!()
	};
	assert_eq!(c.data, b"\x07vanilla");
	let S7::CustomPayload(c) = one(serve_down(S8::CustomPayload(c), &mut cx)) else {
		panic!()
	};
	assert_eq!(c.data, b"vanilla");
}

#[test]
fn maps_split_into_1_7_updates() {
	let mut cx = Session::new();
	use v1_8::play_to_client_packet_map_columns::Ty as Columns;
	let map = C8::Map(v1_8::play_to_client_packet_map::Ty {
		item_damage: 5,
		scale: 2,
		icons: vec![v1_8::play_to_client_packet_map_icons_items::Ty {
			direction_and_type: 0x13,
			x: 4,
			z: -4,
		}],
		columns: Columns::Default(
			2,
			v1_8::play_to_client_packet_map_columns_default::Ty {
				rows: 2,
				x: 10,
				y: 20,
				data: vec![1, 2, 3, 4],
			},
		),
	});
	let data: Vec<Vec<u8>> = down(map, &mut cx)
		.into_iter()
		.map(|p| match p {
			C7::Map(m) => m.data,
			p => panic!("{p:?}"),
		})
		.collect();
	assert_eq!(
		data,
		vec![
			vec![0, 10, 20, 1, 3],
			vec![0, 11, 20, 2, 4],
			vec![1, 0x13, 4, 0xfc],
			vec![2, 2]
		]
	);
}
