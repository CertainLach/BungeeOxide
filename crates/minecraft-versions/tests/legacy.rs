use minecraft_versions::gen::convert::{index_of, step, v1_7, v1_8, Root};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::Session;

type C7 = v1_7::play_toClient_packet::Ty;
type C8 = v1_8::play_toClient_packet::Ty;
type S7 = v1_7::play_toServer_packet::Ty;
type S8 = v1_8::play_toServer_packet::Ty;

fn at(v: &str) -> usize {
	index_of(v).unwrap()
}

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

codec!(enc_c8, dec_c8, v1_8::play_toClient_packet);
codec!(enc_c7, dec_c7, v1_7::play_toClient_packet);
codec!(enc_s8, dec_s8, v1_8::play_toServer_packet);
codec!(enc_s7, dec_s7, v1_7::play_toServer_packet);

fn down(p: C8, cx: &mut Session) -> Vec<C7> {
	step(Root::PlayToClient, at("1.8"), at("1.7"), &enc_c8(&p), cx)
		.unwrap()
		.iter()
		.map(|b| dec_c7(b))
		.collect()
}

fn up(p: C7, cx: &mut Session) -> Vec<C8> {
	step(Root::PlayToClient, at("1.7"), at("1.8"), &enc_c7(&p), cx)
		.unwrap()
		.iter()
		.map(|b| dec_c8(b))
		.collect()
}

fn serve(p: S7, cx: &mut Session) -> Vec<S8> {
	step(Root::PlayToServer, at("1.7"), at("1.8"), &enc_s7(&p), cx)
		.unwrap()
		.iter()
		.map(|b| dec_s8(b))
		.collect()
}

fn serve_down(p: S8, cx: &mut Session) -> Vec<S7> {
	step(Root::PlayToServer, at("1.8"), at("1.7"), &enc_s8(&p), cx)
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
			at("1.8"),
			at("1.7"),
			&packet,
			&mut there,
		)
		.unwrap();
		for b in &old {
			let again = step(Root::PlayToClient, at("1.7"), at("1.8"), b, &mut back).unwrap();
			if matches!(
				original,
				C8::map_chunk(_)
					| C8::map_chunk_bulk(_)
					| C8::keep_alive(_)
					| C8::entity_head_rotation(_)
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
	let login = v1_8::login_toClient_packet::Ty::compress(256);
	let mut o = Output::new();
	v1_8::login_toClient_packet::serialize(&login, &mut o).unwrap();
	assert!(step(
		Root::LoginToClient,
		at("1.8"),
		at("1.7"),
		&o.into_inner(),
		&mut cx
	)
	.unwrap()
	.is_empty());
	let join = C8::login(v1_8::play_toClient_packet_login::Ty {
		entityId: 7,
		gameMode: 1,
		dimension: -1,
		difficulty: 2,
		maxPlayers: 20,
		levelType: "default".into(),
		reducedDebugInfo: true,
	});
	let C7::login(l) = one(down(join.clone(), &mut cx)) else {
		panic!()
	};
	assert_eq!(
		(l.entityId, l.gameMode, l.dimension, l.levelType.as_str()),
		(7, 1, -1, "default")
	);
	let C8::login(back) = one(up(C7::login(l), &mut cx)) else {
		panic!()
	};
	assert!(!back.reducedDebugInfo);
	assert_eq!(down(C8::keep_alive(99), &mut cx), vec![C7::keep_alive(99)]);
	assert_eq!(serve(S7::keep_alive(99), &mut cx), vec![S8::keep_alive(99)]);
}

#[test]
fn block_changes_use_1_7_ids_and_record_layout() {
	let mut cx = Session::new();
	let change = C8::block_change(v1_8::play_toClient_packet_block_change::Ty {
		location: v1_8::position::Ty {
			x: -5,
			y: 70,
			z: 12,
		},
		r#type: 169 << 4,
	});
	let C7::block_change(b) = one(down(change, &mut cx)) else {
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
	let multi = C8::multi_block_change(v1_8::play_toClient_packet_multi_block_change::Ty {
		chunkX: 1,
		chunkZ: 2,
		records: vec![
			v1_8::play_toClient_packet_multi_block_change__records__items::Ty {
				horizontalPos: 0x3a,
				y: 64,
				blockId: 35 << 4 | 14,
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
		C8::world_particles(v1_8::play_toClient_packet_world_particles::Ty {
			longDistance: true,
			x: 1.0,
			y: 2.0,
			z: 3.0,
			offsetX: 0.0,
			offsetY: 0.0,
			offsetZ: 0.0,
			particleData: 0.5,
			particles: 10,
			data,
		})
	};
	use v1_8::play_toClient_packet_world_particles__data::Ty as D;
	let name = |p, cx: &mut Session| match down(p, cx).as_slice() {
		[C7::world_particles(w)] => w.particleName.clone(),
		o => format!("{o:?}"),
	};
	assert_eq!(name(particles(D::default(26)), &mut cx), "flame");
	assert_eq!(
		name(particles(D::v37(vec![1 | 3 << 12])), &mut cx),
		"blockcrack_1_0"
	);
	assert_eq!(
		name(particles(D::v36(vec![409, 0])), &mut cx),
		"iconcrack_406_0"
	);
	assert_eq!(name(particles(D::default(41)), &mut cx), "[]");
	let event = C8::world_event(v1_8::play_toClient_packet_world_event::Ty {
		effectId: 2001,
		location: v1_8::position::Ty { x: 0, y: 5, z: 0 },
		data: 168 | 2 << 12,
		global: false,
	});
	let C7::world_event(e) = one(down(event, &mut cx)) else {
		panic!()
	};
	assert_eq!(e.data, 98 | 1 << 12);
}

#[test]
fn entities_spawn_as_1_7_entities() {
	let mut cx = Session::new();
	use v1_8::play_toClient_packet_spawn_entity__velocity::Ty as V8;
	let object = |id, ty, velocity| {
		C8::spawn_entity(v1_8::play_toClient_packet_spawn_entity::Ty {
			entityId: id,
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
		V8::default(12 | 1 << 12, v1_8::vec3i16::Ty { x: 0, y: -10, z: 0 }),
	);
	let C7::spawn_entity(s) = one(down(sand, &mut cx)) else {
		panic!()
	};
	assert_eq!(s.y, 2048 + 16);
	assert_eq!(
		v1_7::play_toClient_packet_spawn_entity__objectData::virtual_tag(&s.objectData),
		12 | 1 << 16
	);
	let teleport = C8::entity_teleport(v1_8::play_toClient_packet_entity_teleport::Ty {
		entityId: 1,
		x: 0,
		y: 100,
		z: 0,
		yaw: 0,
		pitch: 0,
		onGround: true,
	});
	let C7::entity_teleport(t) = one(down(teleport, &mut cx)) else {
		panic!()
	};
	assert_eq!(t.y, 116);
	let C7::spawn_entity(f) = one(down(object(2, 71, V8::v0), &mut cx)) else {
		panic!()
	};
	assert_eq!((f.x, f.z, f.yaw), (320, -352, 0));
	assert!(down(object(3, 78, V8::v0), &mut cx).is_empty());
	let meta = C8::entity_metadata(v1_8::play_toClient_packet_entity_metadata::Ty {
		entityId: 3,
		metadata: vec![],
	});
	assert!(down(meta, &mut cx).is_empty());
	let rabbit = C8::spawn_entity_living(v1_8::play_toClient_packet_spawn_entity_living::Ty {
		entityId: 4,
		r#type: 101,
		x: 0,
		y: 0,
		z: 0,
		yaw: 0,
		pitch: 0,
		headPitch: 0,
		velocity: v1_8::vec3i16::Ty { x: 0, y: 0, z: 0 },
		metadata: vec![],
	});
	let C7::spawn_entity_living(r) = one(down(rabbit, &mut cx)) else {
		panic!()
	};
	assert_eq!(r.r#type, 93);
	let many = down(C8::entity_destroy((0..300).collect()), &mut cx);
	assert_eq!(
		many.iter()
			.map(|p| match p {
				C7::entity_destroy(ids) => ids.len(),
				_ => 0,
			})
			.collect::<Vec<_>>(),
		vec![127, 127, 46]
	);
}

#[test]
fn players_get_names_and_skins_from_the_tab_list() {
	let mut cx = Session::new();
	use v1_8::play_toClient_packet_player_info__action::Ty as Action;
	use v1_8::play_toClient_packet_player_info__data__items__ANON::Ty as A;
	let uuid = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdefu128;
	let info = |action, entry| {
		C8::player_info(v1_8::play_toClient_packet_player_info::Ty {
			action,
			data: vec![v1_8::play_toClient_packet_player_info__data__items::Ty {
				uuid,
				ANON: entry,
			}],
		})
	};
	let add = info(
		Action::add_player,
		A::add_player(v1_8::play_toClient_packet_player_info__data__items__ANON__add_player::Ty {
			name: "Notch".into(),
			properties: vec![v1_8::play_toClient_packet_player_info__data__items__ANON__add_player__properties__items::Ty {
				name: "textures".into(),
				value: "abc".into(),
				signature: Some("sig".into()),
			}],
			gamemode: 0,
			ping: 42,
			displayName: Some(r#"{"text":"The Notch","color":"gold"}"#.into()),
		}),
	);
	let C7::player_info(p) = one(down(add, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		(p.playerName.as_str(), p.online, p.ping),
		("\u{a7}6The Notch", true, 42)
	);
	let spawn = C8::named_entity_spawn(v1_8::play_toClient_packet_named_entity_spawn::Ty {
		entityId: 9,
		playerUUID: uuid,
		x: 1,
		y: 2,
		z: 3,
		yaw: 0,
		pitch: 0,
		currentItem: 0,
		metadata: vec![],
	});
	let C7::named_entity_spawn(s) = one(down(spawn, &mut cx)) else {
		panic!()
	};
	assert_eq!(s.playerUUID, "01234567-89ab-cdef-0123-456789abcdef");
	assert_eq!(
		(
			s.playerName.as_str(),
			s.data.len(),
			s.data[0].signature.as_str()
		),
		("Notch", 1, "sig")
	);
	let C7::player_info(gone) = one(down(info(Action::remove_player, A::remove_player), &mut cx))
	else {
		panic!()
	};
	assert_eq!(
		(gone.playerName.as_str(), gone.online),
		("\u{a7}6The Notch", false)
	);
	let back = up(C7::named_entity_spawn(s), &mut Session::new());
	assert!(
		matches!(back.as_slice(), [C8::player_info(_), C8::named_entity_spawn(n)] if n.playerUUID == uuid)
	);
}

#[test]
fn chat_titles_and_signs() {
	let mut cx = Session::new();
	let chat = |position| {
		C8::chat(v1_8::play_toClient_packet_chat::Ty {
			message: "\"hi\"".into(),
			position,
		})
	};
	assert_eq!(down(chat(0), &mut cx), vec![C7::chat("\"hi\"".into())]);
	assert!(down(chat(2), &mut cx).is_empty());
	let sign = C8::update_sign(v1_8::play_toClient_packet_update_sign::Ty {
		location: v1_8::position::Ty { x: 1, y: 2, z: 3 },
		text1: r#"{"text":"short"}"#.into(),
		text2: r#""a line that is far too long for 1.7""#.into(),
		text3: "\"\"".into(),
		text4: r#"{"text":"red","color":"red"}"#.into(),
	});
	let C7::update_sign(s) = one(down(sign, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		[s.text1.as_str(), &s.text2, &s.text3, &s.text4],
		["short", "a line that is ", "", "\u{a7}cred"]
	);
	let edited = S7::update_sign(v1_7::play_toServer_packet_update_sign::Ty {
		location: v1_7::position_isi::Ty { x: 1, y: 2, z: 3 },
		text1: "say \"hi\"".into(),
		text2: String::new(),
		text3: String::new(),
		text4: String::new(),
	});
	let S8::update_sign(u) = one(serve(edited, &mut cx)) else {
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
	use v1_8::play_toClient_packet_open_window__entityId::Ty as E8;
	let open = C8::open_window(v1_8::play_toClient_packet_open_window::Ty {
		windowId: 3,
		inventoryType: "minecraft:enchanting_table".into(),
		windowTitle: r#"{"translate":"container.enchant"}"#.into(),
		slotCount: 0,
		entityId: E8::default("minecraft:enchanting_table".into()),
	});
	let C7::open_window(w) = one(down(open, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		v1_7::play_toClient_packet_open_window__entityId::virtual_tag(&w.entityId),
		4
	);
	assert_eq!(
		(w.windowTitle.as_str(), w.useProvidedTitle),
		("container.enchant", false)
	);
	let stack = |id, damage| {
		v1_8::slot::Ty::default(
			id,
			v1_8::slot__default::Ty {
				itemCount: 1,
				itemDamage: damage,
				nbtData: None,
			},
		)
	};
	let items = C8::window_items(v1_8::play_toClient_packet_window_items::Ty {
		windowId: 3,
		items: vec![stack(276, 0), stack(351, 4), stack(1, 3)],
	});
	let C7::window_items(i) = one(down(items, &mut cx)) else {
		panic!()
	};
	assert_eq!(i.items.len(), 2);
	assert!(
		matches!(&i.items[1], v1_7::slot::Ty::default(1, s) if s.itemDamage == 0 && s.nbtData.is_some())
	);
	let set = |slot| {
		C8::set_slot(v1_8::play_toClient_packet_set_slot::Ty {
			windowId: 3,
			slot,
			item: v1_8::slot::Ty::_1,
		})
	};
	assert!(down(set(1), &mut cx).is_empty());
	assert!(matches!(down(set(5), &mut cx).as_slice(), [C7::set_slot(s)] if s.slot == 4));
	let click = S7::window_click(v1_7::play_toServer_packet_window_click::Ty {
		windowId: 3,
		slot: 4,
		mouseButton: 0,
		action: 1,
		mode: 0,
		item: i.items[1].clone(),
	});
	let S8::window_click(c) = one(serve(click, &mut cx)) else {
		panic!()
	};
	assert_eq!((c.slot, c.item.clone()), (5, stack(1, 3)));
}

#[test]
fn player_movement_from_1_7() {
	let mut cx = Session::new();
	let moved = S7::position(v1_7::play_toServer_packet_position::Ty {
		x: 10.0,
		y: 64.0,
		stance: 65.62,
		z: -3.0,
		onGround: true,
	});
	let S8::position(p) = one(serve(moved, &mut cx)) else {
		panic!()
	};
	assert_eq!((p.x, p.y, p.z, p.onGround), (10.0, 64.0, -3.0, true));
	let S7::position(back) = one(serve_down(S8::position(p), &mut cx)) else {
		panic!()
	};
	assert_eq!((back.y, back.stance), (64.0, 64.0 + 1.6200000047683716));
	let teleport = C8::position(v1_8::play_toClient_packet_position::Ty {
		x: 1.0,
		y: 0.0,
		z: 0.0,
		yaw: 0.0,
		pitch: 0.0,
		flags: 0b111,
	});
	let C7::position(t) = one(down(teleport, &mut cx)) else {
		panic!()
	};
	assert_eq!(
		(t.x, t.y, t.z, t.onGround),
		(11.0, 64.0 + 1.6200000047683716, -3.0, true)
	);
	let place = S7::block_place(v1_7::play_toServer_packet_block_place::Ty {
		location: v1_7::position_ibi::Ty {
			x: -1,
			y: 255,
			z: -1,
		},
		direction: -1,
		heldItem: v1_7::slot::Ty::_1,
		cursorX: 0,
		cursorY: 0,
		cursorZ: 0,
	});
	let S8::block_place(b) = one(serve(place, &mut cx)) else {
		panic!()
	};
	assert_eq!((b.location.x, b.location.y, b.location.z), (-1, -1, -1));
	use v1_7::play_toServer_packet_entity_action__actionId::Ty as A7;
	use v1_8::play_toServer_packet_entity_action__actionId::Ty as A8;
	let action = |id| {
		S7::entity_action(v1_7::play_toServer_packet_entity_action::Ty {
			entityId: 7,
			actionId: id,
			jumpBoost: 0,
		})
	};
	assert!(
		matches!(serve(action(A7::open_inventory), &mut cx).as_slice(), [S8::entity_action(a)] if a.actionId == A8::open_inventory)
	);
	assert!(
		matches!(serve(action(A7::start_sneaking), &mut cx).as_slice(), [S8::entity_action(a)] if a.actionId == A8::start_sneaking)
	);
	let swing = |animation| {
		S7::arm_animation(v1_7::play_toServer_packet_arm_animation::Ty {
			entityId: 7,
			animation,
		})
	};
	assert_eq!(serve(swing(1), &mut cx).len(), 1);
	assert!(serve(swing(104), &mut cx).is_empty());
	let brand = S7::custom_payload(v1_7::play_toServer_packet_custom_payload::Ty {
		channel: "MC|Brand".into(),
		data: b"vanilla".to_vec(),
	});
	let S8::custom_payload(c) = one(serve(brand, &mut cx)) else {
		panic!()
	};
	assert_eq!(c.data, b"\x07vanilla");
	let S7::custom_payload(c) = one(serve_down(S8::custom_payload(c), &mut cx)) else {
		panic!()
	};
	assert_eq!(c.data, b"vanilla");
}

#[test]
fn maps_split_into_1_7_updates() {
	let mut cx = Session::new();
	use v1_8::play_toClient_packet_map__columns::Ty as Columns;
	let map = C8::map(v1_8::play_toClient_packet_map::Ty {
		itemDamage: 5,
		scale: 2,
		icons: vec![v1_8::play_toClient_packet_map__icons__items::Ty {
			directionAndType: 0x13,
			x: 4,
			z: -4,
		}],
		columns: Columns::default(
			2,
			v1_8::play_toClient_packet_map__columns__default::Ty {
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
			C7::map(m) => m.data,
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
