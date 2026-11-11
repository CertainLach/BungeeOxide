use bungeeoxide_converter::gen::convert::{convert, v1_12_2, v1_8, Root, Version};
use bungeeoxide_converter::pds::{Input, Output};
use bungeeoxide_converter::Session;

type C8 = v1_8::play_to_client_packet::Ty;
type C12 = v1_12_2::play_to_client_packet::Ty;
type S8 = v1_8::play_to_server_packet::Ty;
type S12 = v1_12_2::play_to_server_packet::Ty;
type M12 = v1_12_2::entity_metadata_item::Ty;
type M8 = v1_8::entity_metadata_item::Ty;

fn down(p: C12, cx: &mut Session) -> Vec<C8> {
	let mut o = Output::new();
	v1_12_2::play_to_client_packet::serialize(&p, &mut o).unwrap();
	convert(
		Root::PlayToClient,
		Version::V1_12_2,
		Version::V1_8,
		&o.into_inner(),
		cx,
	)
	.unwrap()
	.iter()
	.map(|b| {
		let mut input = Input::new(b);
		let p = v1_8::play_to_client_packet::deserialize(&mut input).unwrap();
		assert_eq!(input.remaining(), 0, "{p:?}");
		p
	})
	.collect()
}

fn up(p: S8, cx: &mut Session) -> Vec<S12> {
	let mut o = Output::new();
	v1_8::play_to_server_packet::serialize(&p, &mut o).unwrap();
	convert(
		Root::PlayToServer,
		Version::V1_8,
		Version::V1_12_2,
		&o.into_inner(),
		cx,
	)
	.unwrap()
	.iter()
	.map(|b| {
		let mut input = Input::new(b);
		let p = v1_12_2::play_to_server_packet::deserialize(&mut input).unwrap();
		assert_eq!(input.remaining(), 0, "{p:?}");
		p
	})
	.collect()
}

fn one<T: std::fmt::Debug>(v: Vec<T>) -> T {
	assert_eq!(v.len(), 1, "{v:?}");
	v.into_iter().next().unwrap()
}

fn entry(key: u8, value: M12) -> v1_12_2::entity_metadata_items::Ty {
	let r#type = match &value {
		M12::V0(_) => 0,
		M12::V1(_) => 1,
		M12::V2(_) => 2,
		M12::V3(_) => 3,
		M12::V6(_) => 6,
		M12::V11(_) => 11,
		_ => 0,
	};
	v1_12_2::entity_metadata_items::Ty { key, r#type, value }
}

fn meta8(v: &[v1_8::entity_metadata_items::Ty]) -> Vec<(u8, M8)> {
	v.iter().map(|e| (e.anon.key, e.value.clone())).collect()
}

fn mob(id: i32, ty: i32, metadata: Vec<v1_12_2::entity_metadata_items::Ty>) -> C12 {
	C12::SpawnEntityLiving(v1_12_2::play_to_client_packet_spawn_entity_living::Ty {
		entity_id: id,
		entity_uuid: id as u128,
		r#type: ty,
		x: 10.5,
		y: 64.0,
		z: -3.25,
		yaw: 0,
		pitch: 0,
		head_pitch: 0,
		velocity: v1_12_2::vec3i16::Ty { x: 0, y: 0, z: 0 },
		metadata,
	})
}

#[test]
fn keep_alive_ids_survive_the_round_trip() {
	let mut cx = Session::new();
	let id = 1_700_000_000_123i64;
	let C8::KeepAlive(short) = one(down(C12::KeepAlive(id), &mut cx)) else {
		panic!()
	};
	assert_eq!(one(up(S8::KeepAlive(short), &mut cx)), S12::KeepAlive(id));
}

#[test]
fn new_mobs_spawn_as_old_mobs_with_valid_metadata() {
	let mut cx = Session::new();
	let husk = mob(
		1,
		23,
		vec![
			entry(0, M12::V0(0)),
			entry(7, M12::V2(20.0)),
			entry(12, M12::V6(true)),
			entry(14, M12::V6(true)),
		],
	);
	let C8::SpawnEntityLiving(z) = one(down(husk, &mut cx)) else {
		panic!()
	};
	assert_eq!((z.r#type, z.x, z.y, z.z), (54, 336, 2048, -104));
	assert_eq!(
		meta8(&z.metadata),
		vec![
			(0, M8::V0(0)),
			(6, M8::V3(20.0)),
			(12, M8::V0(1)),
			(13, M8::V0(0))
		]
	);
	let C8::SpawnEntityLiving(s) = one(down(mob(2, 5, vec![entry(0, M12::V0(0))]), &mut cx)) else {
		panic!()
	};
	assert_eq!(
		(s.r#type, meta8(&s.metadata)),
		(51, vec![(0, M8::V0(0)), (13, M8::V0(1))])
	);
	let C8::SpawnEntityLiving(p) = one(down(mob(3, 105, vec![entry(0, M12::V0(0))]), &mut cx))
	else {
		panic!()
	};
	assert_eq!(
		(p.r#type, meta8(&p.metadata)),
		(65, vec![(0, M8::V0(0)), (16, M8::V0(0))])
	);
	let C8::SpawnEntityLiving(m) = one(down(mob(4, 69, vec![]), &mut cx)) else {
		panic!()
	};
	assert_eq!(m.r#type, 62);
	let update = C12::EntityMetadata(v1_12_2::play_to_client_packet_entity_metadata::Ty {
		entity_id: 1,
		metadata: vec![entry(12, M12::V6(false))],
	});
	let C8::EntityMetadata(u) = one(down(update, &mut cx)) else {
		panic!()
	};
	assert_eq!(meta8(&u.metadata), vec![(12, M8::V0(0))]);
}

#[test]
fn players_spawn_with_1_8_metadata() {
	let mut cx = Session::new();
	let spawn = C12::NamedEntitySpawn(v1_12_2::play_to_client_packet_named_entity_spawn::Ty {
		entity_id: 7,
		player_uuid: 9,
		x: -1.0,
		y: 70.0,
		z: 1.0,
		yaw: 0,
		pitch: 0,
		metadata: vec![
			entry(0, M12::V0(0)),
			entry(8, M12::V1(0)),
			entry(13, M12::V0(0x7f)),
		],
	});
	let C8::NamedEntitySpawn(s) = one(down(spawn, &mut cx)) else {
		panic!()
	};
	assert_eq!((s.x, s.y, s.z), (-32, 2240, 32));
	assert_eq!(
		meta8(&s.metadata),
		vec![(0, M8::V0(0)), (7, M8::V2(0)), (10, M8::V0(0x7f))]
	);
}

#[test]
fn relative_moves_do_not_drift() {
	let mut cx = Session::new();
	down(mob(1, 54, vec![entry(0, M12::V0(0))]), &mut cx);
	let mut sum = 0i32;
	for _ in 0..64 {
		let step = C12::RelEntityMove(v1_12_2::play_to_client_packet_rel_entity_move::Ty {
			entity_id: 1,
			d_x: 100,
			d_y: 0,
			d_z: 0,
			on_ground: true,
		});
		for p in down(step, &mut cx) {
			let C8::RelEntityMove(m) = p else { panic!() };
			sum += m.d_x as i32;
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
		v1_12_2::slot::Ty::Default(
			id,
			v1_12_2::slot_default::Ty {
				item_count: 1,
				item_damage: damage,
				nbt_data: None,
			},
		)
	};
	let mut items: Vec<_> = (0..46).map(|_| v1_12_2::slot::Ty::_1).collect();
	items[36] = stack(251, 5);
	let C8::WindowItems(w) = one(down(
		C12::WindowItems(v1_12_2::play_to_client_packet_window_items::Ty {
			window_id: 0,
			items,
		}),
		&mut cx,
	)) else {
		panic!()
	};
	assert_eq!(w.items.len(), 45);
	let v1_8::slot::Ty::Default(159, clay) = &w.items[36] else {
		panic!("{:?}", w.items[36])
	};
	assert_eq!(clay.item_damage, 5);
	assert!(down(
		C12::SetSlot(v1_12_2::play_to_client_packet_set_slot::Ty {
			window_id: 0,
			slot: 45,
			item: stack(1, 0)
		}),
		&mut cx
	)
	.is_empty());
	let click = S8::WindowClick(v1_8::play_to_server_packet_window_click::Ty {
		window_id: 0,
		slot: 36,
		mouse_button: 0,
		action: 1,
		mode: 0,
		item: w.items[36].clone(),
	});
	let S12::WindowClick(c) = one(up(click, &mut cx)) else {
		panic!()
	};
	assert_eq!(c.item, stack(251, 5));
}

#[test]
fn teleports_are_confirmed_and_swings_follow_attacks() {
	let mut cx = Session::new();
	let tp = C12::Position(v1_12_2::play_to_client_packet_position::Ty {
		x: 5.5,
		y: 64.0,
		z: 7.5,
		yaw: 0.0,
		pitch: 0.0,
		flags: 0,
		teleport_id: 42,
	});
	let C8::Position(p) = one(down(tp, &mut cx)) else {
		panic!()
	};
	assert_eq!((p.x, p.y, p.z, p.flags), (5.5, 64.0, 7.5, 0));
	let echo = S8::PositionLook(v1_8::play_to_server_packet_position_look::Ty {
		x: 5.5,
		y: 64.0,
		z: 7.5,
		yaw: 0.0,
		pitch: 0.0,
		on_ground: false,
	});
	let out = up(echo, &mut cx);
	assert!(
		matches!(
			out.as_slice(),
			[S12::TeleportConfirm(42), S12::PositionLook(_)]
		),
		"{out:?}"
	);
	assert!(up(
		S8::ArmAnimation(v1_8::play_to_server_packet_arm_animation::Ty {}),
		&mut cx
	)
	.is_empty());
	let attack = up(
		S8::UseEntity(v1_8::play_to_server_packet_use_entity::Ty {
			target: 3,
			mouse: v1_8::play_to_server_packet_use_entity_mouse::Ty::Default(1),
		}),
		&mut cx,
	);
	assert!(matches!(attack.as_slice(), [S12::UseEntity(_)]));
	assert!(matches!(
		up(S8::Flying(true), &mut cx).as_slice(),
		[S12::ArmAnimation(0), S12::Flying(true)]
	));
	let eat = S8::BlockPlace(v1_8::play_to_server_packet_block_place::Ty {
		location: v1_8::position::Ty {
			x: -1,
			y: -1,
			z: -1,
		},
		direction: -1,
		held_item: v1_8::slot::Ty::_1,
		cursor_x: 0,
		cursor_y: 0,
		cursor_z: 0,
	});
	assert_eq!(one(up(eat, &mut cx)), S12::UseItem(0));
}

fn tp(x: f64, y: f64, z: f64, flags: i8, id: i32) -> C12 {
	C12::Position(v1_12_2::play_to_client_packet_position::Ty {
		x,
		y,
		z,
		yaw: 0.0,
		pitch: 0.0,
		flags,
		teleport_id: id,
	})
}

fn at(x: f64, y: f64, z: f64, look: bool) -> S8 {
	match look {
		true => S8::PositionLook(v1_8::play_to_server_packet_position_look::Ty {
			x,
			y,
			z,
			yaw: 0.0,
			pitch: 0.0,
			on_ground: false,
		}),
		false => S8::Position(v1_8::play_to_server_packet_position::Ty {
			x,
			y,
			z,
			on_ground: true,
		}),
	}
}

#[test]
fn relative_teleports_stay_relative_and_late_answers_win() {
	let mut cx = Session::new();
	down(tp(5.5, 64.0, 7.5, 0, 1), &mut cx);
	assert!(matches!(
		up(at(5.5, 64.0, 7.5, true), &mut cx).as_slice(),
		[S12::TeleportConfirm(1), S12::PositionLook(_)]
	));
	up(at(5.5, 60.0, 7.5, false), &mut cx);
	let C8::Position(p) = one(down(tp(0.0, 64.0, 0.0, 0x1f, 2), &mut cx)) else {
		panic!()
	};
	assert_eq!((p.x, p.y, p.z, p.flags), (0.0, 64.0, 0.0, 0x1f));
	assert_eq!(up(at(5.5, 120.0, 7.5, false), &mut cx).len(), 1);
	assert_eq!(
		up(at(5.5, 120.0, 7.5, true), &mut cx)[0],
		S12::TeleportConfirm(2)
	);

	down(tp(0.0, -64.0, 0.0, 0x1f, 3), &mut cx);
	down(tp(1.5, 70.0, 2.5, 0x18, 4), &mut cx);
	down(tp(1.5, 70.0, 2.5, 0x18, 5), &mut cx);
	let out = up(at(1.5, 70.0, 2.5, true), &mut cx);
	assert!(
		matches!(
			out.as_slice(),
			[S12::TeleportConfirm(5), S12::PositionLook(_)]
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
		S12::TeleportConfirm(7)
	);
}

#[test]
fn world_updates_use_1_8_blocks_effects_and_particles() {
	let mut cx = Session::new();
	let change = C12::BlockChange(v1_12_2::play_to_client_packet_block_change::Ty {
		location: v1_12_2::position::Ty { x: 1, y: 2, z: 3 },
		r#type: 251 << 4 | 14,
	});
	let C8::BlockChange(b) = one(down(change, &mut cx)) else {
		panic!()
	};
	assert_eq!(b.r#type, 159 << 4 | 14);
	let effect = |id| v1_12_2::play_to_client_packet_entity_effect::Ty {
		entity_id: 1,
		effect_id: id,
		amplifier: 0,
		duration: 20,
		hide_particles: 0,
	};
	assert!(down(C12::EntityEffect(effect(25)), &mut cx).is_empty());
	assert!(
		matches!(down(C12::EntityEffect(effect(1)), &mut cx).as_slice(), [C8::EntityEffect(e)] if e.hide_particles)
	);
	use v1_12_2::play_to_client_packet_world_particles_data::Ty as D;
	let particle = |data| {
		C12::WorldParticles(v1_12_2::play_to_client_packet_world_particles::Ty {
			long_distance: false,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			offset_x: 0.0,
			offset_y: 0.0,
			offset_z: 0.0,
			particle_data: 0.0,
			particles: 1,
			data,
		})
	};
	assert!(
		matches!(down(particle(D::Default(45)), &mut cx).as_slice(), [C8::WorldParticles(w)] if w.data == v1_8::play_to_client_packet_world_particles_data::Ty::Default(1))
	);
	assert!(down(particle(D::Default(47)), &mut cx).is_empty());
	let riding = C12::SetPassengers(v1_12_2::play_to_client_packet_set_passengers::Ty {
		entity_id: 5,
		passengers: vec![6, 7],
	});
	let links: Vec<_> = down(riding, &mut cx)
		.into_iter()
		.map(|p| match p {
			C8::AttachEntity(a) => (a.entity_id, a.vehicle_id, a.leash),
			p => panic!("{p:?}"),
		})
		.collect();
	assert_eq!(links, vec![(6, 5, false), (7, 6, false)]);
	let dismount = C12::SetPassengers(v1_12_2::play_to_client_packet_set_passengers::Ty {
		entity_id: 5,
		passengers: vec![],
	});
	assert_eq!(down(dismount, &mut cx).len(), 2);
}

#[test]
fn signs_arrive_as_sign_updates() {
	use bungeeoxide_nbt::{Entry, Payload};
	let mut cx = Session::new();
	let line = |k: &str, t: &str| Entry::new(k, Payload::String(t.into()));
	let tag = Entry::new(
		"",
		Payload::Compound(vec![
			line("Text1", "{\"text\":\"Hi\"}"),
			line("Text2", "\"\""),
			line("Text3", "\"\""),
			line("Text4", "\"\""),
		]),
	);
	let data = C12::TileEntityData(v1_12_2::play_to_client_packet_tile_entity_data::Ty {
		location: v1_12_2::position::Ty { x: 1, y: 2, z: 3 },
		action: 9,
		nbt_data: Some(tag),
	});
	let C8::UpdateSign(s) = one(down(data, &mut cx)) else {
		panic!()
	};
	assert_eq!(s.text1, "{\"text\":\"Hi\"}");
	let edit = S8::UpdateSign(v1_8::play_to_server_packet_update_sign::Ty {
		location: v1_8::position::Ty { x: 1, y: 2, z: 3 },
		text1: "{\"text\":\"Yo\",\"color\":\"red\"}".into(),
		text2: "\"\"".into(),
		text3: "\"\"".into(),
		text4: "\"\"".into(),
	});
	let S12::UpdateSign(u) = one(up(edit, &mut cx)) else {
		panic!()
	};
	assert_eq!((u.text1.as_str(), u.text2.as_str()), ("\u{a7}cYo", ""));
}
