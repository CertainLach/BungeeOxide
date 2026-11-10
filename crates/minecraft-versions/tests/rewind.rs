use minecraft_versions::gen::convert::{convert, v1_12_2, v1_8, Root, Version};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::Session;

type C8 = v1_8::play_toClient_packet::Ty;
type C12 = v1_12_2::play_toClient_packet::Ty;
type S8 = v1_8::play_toServer_packet::Ty;
type S12 = v1_12_2::play_toServer_packet::Ty;
type M12 = v1_12_2::entityMetadataItem::Ty;
type M8 = v1_8::entityMetadataItem::Ty;

fn down(p: C12, cx: &mut Session) -> Vec<C8> {
	let mut o = Output::new();
	v1_12_2::play_toClient_packet::serialize(&p, &mut o).unwrap();
	convert(
		Root::PlayToClient,
		Version::V1_12_2.index(),
		Version::V1_8.index(),
		&o.into_inner(),
		cx,
	)
	.unwrap()
	.iter()
	.map(|b| {
		let mut input = Input::new(b);
		let p = v1_8::play_toClient_packet::deserialize(&mut input).unwrap();
		assert_eq!(input.remaining(), 0, "{p:?}");
		p
	})
	.collect()
}

fn up(p: S8, cx: &mut Session) -> Vec<S12> {
	let mut o = Output::new();
	v1_8::play_toServer_packet::serialize(&p, &mut o).unwrap();
	convert(
		Root::PlayToServer,
		Version::V1_8.index(),
		Version::V1_12_2.index(),
		&o.into_inner(),
		cx,
	)
	.unwrap()
	.iter()
	.map(|b| {
		let mut input = Input::new(b);
		let p = v1_12_2::play_toServer_packet::deserialize(&mut input).unwrap();
		assert_eq!(input.remaining(), 0, "{p:?}");
		p
	})
	.collect()
}

fn one<T: std::fmt::Debug>(v: Vec<T>) -> T {
	assert_eq!(v.len(), 1, "{v:?}");
	v.into_iter().next().unwrap()
}

fn entry(key: u8, value: M12) -> v1_12_2::entityMetadata__items::Ty {
	let r#type = match &value {
		M12::v0(_) => 0,
		M12::v1(_) => 1,
		M12::v2(_) => 2,
		M12::v3(_) => 3,
		M12::v6(_) => 6,
		M12::v11(_) => 11,
		_ => 0,
	};
	v1_12_2::entityMetadata__items::Ty { key, r#type, value }
}

fn meta8(v: &[v1_8::entityMetadata__items::Ty]) -> Vec<(u8, M8)> {
	v.iter().map(|e| (e.ANON.key, e.value.clone())).collect()
}

fn mob(id: i32, ty: i32, metadata: Vec<v1_12_2::entityMetadata__items::Ty>) -> C12 {
	C12::spawn_entity_living(v1_12_2::play_toClient_packet_spawn_entity_living::Ty {
		entityId: id,
		entityUUID: id as u128,
		r#type: ty,
		x: 10.5,
		y: 64.0,
		z: -3.25,
		yaw: 0,
		pitch: 0,
		headPitch: 0,
		velocity: v1_12_2::vec3i16::Ty { x: 0, y: 0, z: 0 },
		metadata,
	})
}

#[test]
fn keep_alive_ids_survive_the_round_trip() {
	let mut cx = Session::new();
	let id = 1_700_000_000_123i64;
	let C8::keep_alive(short) = one(down(C12::keep_alive(id), &mut cx)) else {
		panic!()
	};
	assert_eq!(one(up(S8::keep_alive(short), &mut cx)), S12::keep_alive(id));
}

#[test]
fn new_mobs_spawn_as_old_mobs_with_valid_metadata() {
	let mut cx = Session::new();
	let husk = mob(
		1,
		23,
		vec![
			entry(0, M12::v0(0)),
			entry(7, M12::v2(20.0)),
			entry(12, M12::v6(true)),
			entry(14, M12::v6(true)),
		],
	);
	let C8::spawn_entity_living(z) = one(down(husk, &mut cx)) else {
		panic!()
	};
	assert_eq!((z.r#type, z.x, z.y, z.z), (54, 336, 2048, -104));
	assert_eq!(
		meta8(&z.metadata),
		vec![
			(0, M8::v0(0)),
			(6, M8::v3(20.0)),
			(12, M8::v0(1)),
			(13, M8::v0(0))
		]
	);
	let C8::spawn_entity_living(s) = one(down(mob(2, 5, vec![entry(0, M12::v0(0))]), &mut cx))
	else {
		panic!()
	};
	assert_eq!(
		(s.r#type, meta8(&s.metadata)),
		(51, vec![(0, M8::v0(0)), (13, M8::v0(1))])
	);
	let C8::spawn_entity_living(p) = one(down(mob(3, 105, vec![entry(0, M12::v0(0))]), &mut cx))
	else {
		panic!()
	};
	assert_eq!(
		(p.r#type, meta8(&p.metadata)),
		(65, vec![(0, M8::v0(0)), (16, M8::v0(0))])
	);
	let C8::spawn_entity_living(m) = one(down(mob(4, 69, vec![]), &mut cx)) else {
		panic!()
	};
	assert_eq!(m.r#type, 62);
	let update = C12::entity_metadata(v1_12_2::play_toClient_packet_entity_metadata::Ty {
		entityId: 1,
		metadata: vec![entry(12, M12::v6(false))],
	});
	let C8::entity_metadata(u) = one(down(update, &mut cx)) else {
		panic!()
	};
	assert_eq!(meta8(&u.metadata), vec![(12, M8::v0(0))]);
}

#[test]
fn players_spawn_with_1_8_metadata() {
	let mut cx = Session::new();
	let spawn = C12::named_entity_spawn(v1_12_2::play_toClient_packet_named_entity_spawn::Ty {
		entityId: 7,
		playerUUID: 9,
		x: -1.0,
		y: 70.0,
		z: 1.0,
		yaw: 0,
		pitch: 0,
		metadata: vec![
			entry(0, M12::v0(0)),
			entry(8, M12::v1(0)),
			entry(13, M12::v0(0x7f)),
		],
	});
	let C8::named_entity_spawn(s) = one(down(spawn, &mut cx)) else {
		panic!()
	};
	assert_eq!((s.x, s.y, s.z), (-32, 2240, 32));
	assert_eq!(
		meta8(&s.metadata),
		vec![(0, M8::v0(0)), (7, M8::v2(0)), (10, M8::v0(0x7f))]
	);
}

#[test]
fn relative_moves_do_not_drift() {
	let mut cx = Session::new();
	down(mob(1, 54, vec![entry(0, M12::v0(0))]), &mut cx);
	let mut sum = 0i32;
	for _ in 0..64 {
		let step = C12::rel_entity_move(v1_12_2::play_toClient_packet_rel_entity_move::Ty {
			entityId: 1,
			dX: 100,
			dY: 0,
			dZ: 0,
			onGround: true,
		});
		for p in down(step, &mut cx) {
			let C8::rel_entity_move(m) = p else { panic!() };
			sum += m.dX as i32;
		}
	}
	assert_eq!(
		sum,
		((10.5 * 4096.0 + 6400.0) / 128.0f64).floor() as i32 - 336
	);
}

#[test]
fn inventory_drops_the_offhand_and_maps_new_items() {
	let mut cx = Session::new();
	let stack = |id, damage| {
		v1_12_2::slot::Ty::default(
			id,
			v1_12_2::slot__default::Ty {
				itemCount: 1,
				itemDamage: damage,
				nbtData: None,
			},
		)
	};
	let mut items: Vec<_> = (0..46).map(|_| v1_12_2::slot::Ty::_1).collect();
	items[36] = stack(251, 5);
	let C8::window_items(w) = one(down(
		C12::window_items(v1_12_2::play_toClient_packet_window_items::Ty { windowId: 0, items }),
		&mut cx,
	)) else {
		panic!()
	};
	assert_eq!(w.items.len(), 45);
	let v1_8::slot::Ty::default(159, clay) = &w.items[36] else {
		panic!("{:?}", w.items[36])
	};
	assert_eq!(clay.itemDamage, 5);
	assert!(down(
		C12::set_slot(v1_12_2::play_toClient_packet_set_slot::Ty {
			windowId: 0,
			slot: 45,
			item: stack(1, 0)
		}),
		&mut cx
	)
	.is_empty());
	let click = S8::window_click(v1_8::play_toServer_packet_window_click::Ty {
		windowId: 0,
		slot: 36,
		mouseButton: 0,
		action: 1,
		mode: 0,
		item: w.items[36].clone(),
	});
	let S12::window_click(c) = one(up(click, &mut cx)) else {
		panic!()
	};
	assert_eq!(c.item, stack(251, 5));
}

#[test]
fn teleports_are_confirmed_and_swings_follow_attacks() {
	let mut cx = Session::new();
	let tp = C12::position(v1_12_2::play_toClient_packet_position::Ty {
		x: 5.5,
		y: 64.0,
		z: 7.5,
		yaw: 0.0,
		pitch: 0.0,
		flags: 0,
		teleportId: 42,
	});
	let C8::position(p) = one(down(tp, &mut cx)) else {
		panic!()
	};
	assert_eq!((p.x, p.y, p.z, p.flags), (5.5, 64.0, 7.5, 0));
	let echo = S8::position_look(v1_8::play_toServer_packet_position_look::Ty {
		x: 5.5,
		y: 64.0,
		z: 7.5,
		yaw: 0.0,
		pitch: 0.0,
		onGround: false,
	});
	let out = up(echo, &mut cx);
	assert!(
		matches!(
			out.as_slice(),
			[S12::teleport_confirm(42), S12::position_look(_)]
		),
		"{out:?}"
	);
	assert!(up(
		S8::arm_animation(v1_8::play_toServer_packet_arm_animation::Ty {}),
		&mut cx
	)
	.is_empty());
	let attack = up(
		S8::use_entity(v1_8::play_toServer_packet_use_entity::Ty {
			target: 3,
			mouse: v1_8::play_toServer_packet_use_entity__mouse::Ty::default(1),
		}),
		&mut cx,
	);
	assert!(matches!(attack.as_slice(), [S12::use_entity(_)]));
	assert!(matches!(
		up(S8::flying(true), &mut cx).as_slice(),
		[S12::arm_animation(0), S12::flying(true)]
	));
	let eat = S8::block_place(v1_8::play_toServer_packet_block_place::Ty {
		location: v1_8::position::Ty {
			x: -1,
			y: -1,
			z: -1,
		},
		direction: -1,
		heldItem: v1_8::slot::Ty::_1,
		cursorX: 0,
		cursorY: 0,
		cursorZ: 0,
	});
	assert_eq!(one(up(eat, &mut cx)), S12::use_item(0));
}

fn tp(x: f64, y: f64, z: f64, flags: i8, id: i32) -> C12 {
	C12::position(v1_12_2::play_toClient_packet_position::Ty {
		x,
		y,
		z,
		yaw: 0.0,
		pitch: 0.0,
		flags,
		teleportId: id,
	})
}

fn at(x: f64, y: f64, z: f64, look: bool) -> S8 {
	match look {
		true => S8::position_look(v1_8::play_toServer_packet_position_look::Ty {
			x,
			y,
			z,
			yaw: 0.0,
			pitch: 0.0,
			onGround: false,
		}),
		false => S8::position(v1_8::play_toServer_packet_position::Ty {
			x,
			y,
			z,
			onGround: true,
		}),
	}
}

#[test]
fn relative_teleports_stay_relative_and_late_answers_win() {
	let mut cx = Session::new();
	down(tp(5.5, 64.0, 7.5, 0, 1), &mut cx);
	assert!(matches!(
		up(at(5.5, 64.0, 7.5, true), &mut cx).as_slice(),
		[S12::teleport_confirm(1), S12::position_look(_)]
	));
	up(at(5.5, 60.0, 7.5, false), &mut cx);
	let C8::position(p) = one(down(tp(0.0, 64.0, 0.0, 0x1f, 2), &mut cx)) else {
		panic!()
	};
	assert_eq!((p.x, p.y, p.z, p.flags), (0.0, 64.0, 0.0, 0x1f));
	assert_eq!(up(at(5.5, 120.0, 7.5, false), &mut cx).len(), 1);
	assert_eq!(
		up(at(5.5, 120.0, 7.5, true), &mut cx)[0],
		S12::teleport_confirm(2)
	);

	down(tp(0.0, -64.0, 0.0, 0x1f, 3), &mut cx);
	down(tp(1.5, 70.0, 2.5, 0x18, 4), &mut cx);
	down(tp(1.5, 70.0, 2.5, 0x18, 5), &mut cx);
	let out = up(at(1.5, 70.0, 2.5, true), &mut cx);
	assert!(
		matches!(
			out.as_slice(),
			[S12::teleport_confirm(5), S12::position_look(_)]
		),
		"{out:?}"
	);
	assert_eq!(up(at(1.5, 70.0, 2.5, true), &mut cx).len(), 1);

	down(tp(0.0, 300.0, 0.0, 0x1f, 6), &mut cx);
	for _ in 0..60 {
		assert_eq!(up(at(9.5, 70.0, 9.5, false), &mut cx).len(), 1);
	}
	up(at(9.5, 70.0, 9.5, false), &mut cx);
	down(tp(9.5, 80.0, 9.5, 0, 7), &mut cx);
	assert_eq!(
		up(at(9.5, 80.0, 9.5, true), &mut cx)[0],
		S12::teleport_confirm(7)
	);
}

#[test]
fn world_updates_use_1_8_blocks_effects_and_particles() {
	let mut cx = Session::new();
	let change = C12::block_change(v1_12_2::play_toClient_packet_block_change::Ty {
		location: v1_12_2::position::Ty { x: 1, y: 2, z: 3 },
		r#type: 251 << 4 | 14,
	});
	let C8::block_change(b) = one(down(change, &mut cx)) else {
		panic!()
	};
	assert_eq!(b.r#type, 159 << 4 | 14);
	let effect = |id| v1_12_2::play_toClient_packet_entity_effect::Ty {
		entityId: 1,
		effectId: id,
		amplifier: 0,
		duration: 20,
		hideParticles: 0,
	};
	assert!(down(C12::entity_effect(effect(25)), &mut cx).is_empty());
	assert!(
		matches!(down(C12::entity_effect(effect(1)), &mut cx).as_slice(), [C8::entity_effect(e)] if e.hideParticles)
	);
	use v1_12_2::play_toClient_packet_world_particles__data::Ty as D;
	let particle = |data| {
		C12::world_particles(v1_12_2::play_toClient_packet_world_particles::Ty {
			longDistance: false,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			offsetX: 0.0,
			offsetY: 0.0,
			offsetZ: 0.0,
			particleData: 0.0,
			particles: 1,
			data,
		})
	};
	assert!(
		matches!(down(particle(D::default(45)), &mut cx).as_slice(), [C8::world_particles(w)] if w.data == v1_8::play_toClient_packet_world_particles__data::Ty::default(1))
	);
	assert!(down(particle(D::default(47)), &mut cx).is_empty());
	let riding = C12::set_passengers(v1_12_2::play_toClient_packet_set_passengers::Ty {
		entityId: 5,
		passengers: vec![6, 7],
	});
	let links: Vec<_> = down(riding, &mut cx)
		.into_iter()
		.map(|p| match p {
			C8::attach_entity(a) => (a.entityId, a.vehicleId, a.leash),
			p => panic!("{p:?}"),
		})
		.collect();
	assert_eq!(links, vec![(6, 5, false), (7, 6, false)]);
	let dismount = C12::set_passengers(v1_12_2::play_toClient_packet_set_passengers::Ty {
		entityId: 5,
		passengers: vec![],
	});
	assert_eq!(down(dismount, &mut cx).len(), 2);
}

#[test]
fn signs_arrive_as_sign_updates() {
	use nbt::{Entry, Payload};
	let mut cx = Session::new();
	let line = |k: &str, t: &str| Entry::new(k, Payload::string(t.into()));
	let tag = Entry::new(
		"",
		Payload::compound(vec![
			line("Text1", "{\"text\":\"Hi\"}"),
			line("Text2", "\"\""),
			line("Text3", "\"\""),
			line("Text4", "\"\""),
		]),
	);
	let data = C12::tile_entity_data(v1_12_2::play_toClient_packet_tile_entity_data::Ty {
		location: v1_12_2::position::Ty { x: 1, y: 2, z: 3 },
		action: 9,
		nbtData: Some(tag),
	});
	let C8::update_sign(s) = one(down(data, &mut cx)) else {
		panic!()
	};
	assert_eq!(s.text1, "{\"text\":\"Hi\"}");
	let edit = S8::update_sign(v1_8::play_toServer_packet_update_sign::Ty {
		location: v1_8::position::Ty { x: 1, y: 2, z: 3 },
		text1: "{\"text\":\"Yo\",\"color\":\"red\"}".into(),
		text2: "\"\"".into(),
		text3: "\"\"".into(),
		text4: "\"\"".into(),
	});
	let S12::update_sign(u) = one(up(edit, &mut cx)) else {
		panic!()
	};
	assert_eq!((u.text1.as_str(), u.text2.as_str()), ("\u{a7}cYo", ""));
}
