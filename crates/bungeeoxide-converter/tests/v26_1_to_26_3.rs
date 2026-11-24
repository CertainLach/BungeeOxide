use bungeeoxide_converter::gen::convert::{
	convert, v1_21_5, v26_1 as x, v26_2 as y, v26_3 as z, Root,
};
use bungeeoxide_converter::pds::{Input, Output};
use bungeeoxide_converter::pds_native::pds_entityDelta::{Delta, Step, Ty as EntityDelta};
use bungeeoxide_converter::registry;
use bungeeoxide_converter::Session;
use bungeeoxide_converter::Version;

fn encode<T>(v: &T, ser: fn(&T, &mut Output) -> bungeeoxide_converter::pds::Result<()>) -> Vec<u8> {
	let mut o = Output::new();
	ser(v, &mut o).unwrap();
	o.into_inner()
}

fn decode<T>(b: &[u8], de: fn(&mut Input) -> bungeeoxide_converter::pds::Result<T>) -> T {
	let mut i = Input::new(b);
	let v = de(&mut i).unwrap();
	assert_eq!(i.remaining(), 0, "left over bytes");
	v
}

fn run<A, B>(
	root: Root,
	from: Version,
	to: Version,
	packet: &A,
	ser: fn(&A, &mut Output) -> bungeeoxide_converter::pds::Result<()>,
	de: fn(&mut Input) -> bungeeoxide_converter::pds::Result<B>,
	cx: &mut Session,
) -> Vec<B> {
	convert(root, from, to, &encode(packet, ser), cx)
		.unwrap()
		.iter()
		.map(|b| decode(b, de))
		.collect()
}

fn up(p: &y::play_to_client_packet::Ty, cx: &mut Session) -> Vec<z::play_to_client_packet::Ty> {
	run(
		Root::PlayToClient,
		Version::V26_2,
		Version::V26_3,
		p,
		y::play_to_client_packet::serialize,
		z::play_to_client_packet::deserialize,
		cx,
	)
}

fn down(p: &z::play_to_client_packet::Ty, cx: &mut Session) -> Vec<y::play_to_client_packet::Ty> {
	run(
		Root::PlayToClient,
		Version::V26_3,
		Version::V26_2,
		p,
		z::play_to_client_packet::serialize,
		y::play_to_client_packet::deserialize,
		cx,
	)
}

fn id(version: Version, kind: &str, name: &str) -> i32 {
	registry::names(version, kind).unwrap().id(name).unwrap() as i32
}

fn name(version: Version, kind: &str, id: i32) -> &'static str {
	registry::names(version, kind)
		.unwrap()
		.name(id as u32)
		.unwrap()
}

#[test]
fn moves_become_single_steps() {
	let mut cx = Session::new();
	let out = up(
		&y::play_to_client_packet::Ty::RelEntityMove(
			y::play_to_client_packet_rel_entity_move::Ty {
				entity_id: 5,
				d_x: 100,
				d_y: -20,
				d_z: 3,
				on_ground: true,
			},
		),
		&mut cx,
	);
	let [z::play_to_client_packet::Ty::RelEntityMove(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(m.entity_id, 5);
	assert_eq!(
		m.r#move,
		EntityDelta {
			onGround: true,
			delta: Delta::Stepped(vec![Step {
				ticks: 3,
				dX: 100,
				dY: -20,
				dZ: 3
			}]),
		}
	);
}

#[test]
fn stepped_moves_sum_without_overflowing() {
	let mut cx = Session::new();
	let steps = vec![
		Step {
			ticks: 1,
			dX: 30000,
			dY: 10,
			dZ: -5,
		},
		Step {
			ticks: 1,
			dX: 30000,
			dY: 10,
			dZ: -5,
		},
		Step {
			ticks: 1,
			dX: -1000,
			dY: 10,
			dZ: -5,
		},
	];
	let out = down(
		&z::play_to_client_packet::Ty::EntityMoveLook(
			z::play_to_client_packet_entity_move_look::Ty {
				entity_id: 9,
				r#move: EntityDelta {
					onGround: false,
					delta: Delta::Stepped(steps),
				},
				yaw: 64,
				pitch: -10,
			},
		),
		&mut cx,
	);
	let [y::play_to_client_packet::Ty::EntityMoveLook(first), y::play_to_client_packet::Ty::RelEntityMove(second)] =
		out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!((first.d_x, first.d_y, first.d_z), (30000, 10, -5));
	assert_eq!((first.yaw, first.pitch, first.on_ground), (64, -10, false));
	assert_eq!((second.d_x, second.d_y, second.d_z), (29000, 20, -10));
	assert_eq!(second.entity_id, 9);

	let out = down(
		&z::play_to_client_packet::Ty::RelEntityMove(
			z::play_to_client_packet_rel_entity_move::Ty {
				entity_id: 9,
				r#move: EntityDelta {
					onGround: true,
					delta: Delta::Linear {
						dX: -7,
						dY: 8,
						dZ: 9,
					},
				},
			},
		),
		&mut cx,
	);
	let [y::play_to_client_packet::Ty::RelEntityMove(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((m.d_x, m.d_y, m.d_z, m.on_ground), (-7, 8, 9, true));
}

#[test]
fn position_sync_keeps_the_last_step() {
	let mut cx = Session::new();
	use z::play_to_client_packet_sync_entity_position_anon::Ty as Path;
	let step = |x, t| z::play_to_client_packet_sync_entity_position_anon_stepped_items::Ty {
		x,
		y: 64.0,
		z: 0.0,
		tick_offset: t,
	};
	let out = down(
		&z::play_to_client_packet::Ty::SyncEntityPosition(
			z::play_to_client_packet_sync_entity_position::Ty {
				entity_id: 3,
				anon: Path::Stepped(vec![step(1.0, 1), step(4.0, 2)]),
				yaw: 0.0,
				pitch: 0.0,
				on_ground: true,
			},
		),
		&mut cx,
	);
	let [y::play_to_client_packet::Ty::SyncEntityPosition(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!((p.x, p.y, p.z), (4.0, 64.0, 0.0));
	assert_eq!((p.dx, p.dy, p.dz), (1.5, 0.0, 0.0));
}

#[test]
fn light_masks_switch_between_longs_and_bytes() {
	let mut cx = Session::new();
	let light = y::play_to_client_packet_update_light::Ty {
		chunk_x: 1,
		chunk_z: -2,
		sky_light_mask: vec![0b101, 1 << 40],
		block_light_mask: vec![],
		empty_sky_light_mask: vec![0xff],
		empty_block_light_mask: vec![0, 0],
		sky_light: vec![vec![0xaa; 2048], vec![0x11; 2048]],
		block_light: vec![],
	};
	let out = up(
		&y::play_to_client_packet::Ty::UpdateLight(light.clone()),
		&mut cx,
	);
	let [z::play_to_client_packet::Ty::UpdateLight(l)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(l.sky_light_mask, [5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
	assert_eq!(l.block_light_mask, Vec::<u8>::new());
	assert_eq!(l.empty_sky_light_mask, [0xff]);
	assert_eq!(l.empty_block_light_mask, Vec::<u8>::new());
	let back = down(&out[0], &mut cx);
	let [y::play_to_client_packet::Ty::UpdateLight(b)] = back.as_slice() else {
		panic!("{back:?}")
	};
	assert_eq!(b.sky_light_mask, light.sky_light_mask);
	assert_eq!(b.empty_block_light_mask, Vec::<i64>::new());
	assert_eq!(b.sky_light, light.sky_light);
}

#[test]
fn particles_get_three_speeds_and_a_varint_count() {
	let mut cx = Session::new();
	let out = up(
		&y::play_to_client_packet::Ty::WorldParticles(
			y::play_to_client_packet_world_particles::Ty {
				long_distance: true,
				always_show: false,
				x: 1.0,
				y: 2.0,
				z: 3.0,
				offset_x: 0.5,
				offset_y: 0.25,
				offset_z: 0.0,
				velocity_offset: 0.75,
				amount: 300,
				particle: y::particle::Ty::Cloud,
			},
		),
		&mut cx,
	);
	let [z::play_to_client_packet::Ty::WorldParticles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(p.x_max_speed, p.y_max_speed, p.z_max_speed),
		(0.75, 0.75, 0.75)
	);
	assert_eq!(p.amount, 300);
	assert!(matches!(p.particle, z::particle::Ty::Cloud));
	let back = down(&out[0], &mut cx);
	let [y::play_to_client_packet::Ty::WorldParticles(b)] = back.as_slice() else {
		panic!("{back:?}")
	};
	assert_eq!((b.velocity_offset, b.amount, b.offset_y), (0.75, 300, 0.25));
	let poplar = down(
		&z::play_to_client_packet::Ty::WorldParticles(
			z::play_to_client_packet_world_particles::Ty {
				particle: z::particle::Ty::RedPoplarLeaves,
				..p.clone()
			},
		),
		&mut cx,
	);
	let [y::play_to_client_packet::Ty::WorldParticles(b)] = poplar.as_slice() else {
		panic!("{poplar:?}")
	};
	assert!(matches!(b.particle, y::particle::Ty::TintedLeaves(_)));
}

#[test]
fn item_ids_follow_names_across_inserted_items() {
	let mut cx = Session::new();
	for item in ["diamond_sword", "stone", "music_disc_bounce", "sulfur"] {
		let slot = y::slot::Ty::Default(
			1,
			y::slot_default::Ty {
				item_id: id(Version::V26_2, "items", item),
				added_component_count: 0,
				removed_component_count: 0,
				components: vec![],
				remove_components: vec![],
			},
		);
		let out = up(&y::play_to_client_packet::Ty::SetCursorItem(slot), &mut cx);
		let [z::play_to_client_packet::Ty::SetCursorItem(z::slot::Ty::Default(_, d))] =
			out.as_slice()
		else {
			panic!("{out:?}")
		};
		assert_eq!(name(Version::V26_3, "items", d.item_id), item);
	}
	assert_ne!(
		id(Version::V26_2, "items", "diamond_sword"),
		id(Version::V26_3, "items", "diamond_sword")
	);
}

#[test]
fn block_states_follow_names() {
	let mut cx = Session::new();
	let from = registry::blocks(Version::V26_2).unwrap();
	let to = registry::blocks(Version::V26_3).unwrap();
	let state = from
		.parse_state("minecraft:oak_stairs[facing=east,half=top]")
		.unwrap();
	let out = up(
		&y::play_to_client_packet::Ty::BlockChange(y::play_to_client_packet_block_change::Ty {
			location: y::position::Ty { x: 1, z: 2, y: 3 },
			r#type: state as i32,
		}),
		&mut cx,
	);
	let [z::play_to_client_packet::Ty::BlockChange(b)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_ne!(b.r#type, state as i32);
	assert_eq!(
		to.describe(b.r#type as u32).unwrap(),
		from.describe(state).unwrap()
	);
}

fn cursor_26_2(components: Vec<y::slot_component::Ty>) -> y::play_to_client_packet::Ty {
	y::play_to_client_packet::Ty::SetCursorItem(y::slot::Ty::Default(
		1,
		y::slot_default::Ty {
			item_id: id(Version::V26_2, "items", "diamond_sword"),
			added_component_count: components.len() as i32,
			removed_component_count: 1,
			components,
			remove_components: vec![y::slot_component_type::Ty::SwingAnimation],
		},
	))
}

#[test]
fn swing_animation_becomes_attack_animation() {
	let mut cx = Session::new();
	let swing = y::slot_component::Ty::SwingAnimation(y::slot_component_swing_animation::Ty {
		r#type: y::slot_component_swing_animation_type::Ty::Stab,
		duration: 9,
	});
	let map_color = y::slot_component::Ty::MapColor(0x123456);
	let out = up(&cursor_26_2(vec![swing.clone(), map_color]), &mut cx);
	let [z::play_to_client_packet::Ty::SetCursorItem(z::slot::Ty::Default(_, d))] = out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!(d.components.len(), 1);
	assert!(matches!(
		&d.components[0],
		z::slot_component::Ty::AttackAnimation(a) if a.duration == 9
	));
	assert!(matches!(
		d.remove_components.as_slice(),
		[z::slot_component_type::Ty::AttackAnimation]
	));
	let back = down(&out[0], &mut cx);
	let [y::play_to_client_packet::Ty::SetCursorItem(y::slot::Ty::Default(_, d))] = back.as_slice()
	else {
		panic!("{back:?}")
	};
	assert_eq!(d.components, vec![swing]);
}

#[test]
fn pot_decorations_become_templates() {
	let mut cx = Session::new();
	let sherd = id(Version::V26_2, "items", "angler_pottery_sherd");
	let brick = id(Version::V26_2, "items", "brick");
	let out = up(
		&cursor_26_2(vec![y::slot_component::Ty::PotDecorations(vec![
			sherd, brick, brick, sherd,
		])]),
		&mut cx,
	);
	let [z::play_to_client_packet::Ty::SetCursorItem(z::slot::Ty::Default(_, d))] = out.as_slice()
	else {
		panic!("{out:?}")
	};
	let [z::slot_component::Ty::PotDecorations(p)] = d.components.as_slice() else {
		panic!("{d:?}")
	};
	let item = |t: &Option<z::item_stack_template::Ty>| {
		name(Version::V26_3, "items", t.as_ref().unwrap().item_id)
	};
	assert_eq!(
		[item(&p.back), item(&p.left), item(&p.right), item(&p.front)],
		[
			"angler_pottery_sherd",
			"brick",
			"brick",
			"angler_pottery_sherd"
		]
	);
	let back = down(&out[0], &mut cx);
	let [y::play_to_client_packet::Ty::SetCursorItem(y::slot::Ty::Default(_, d))] = back.as_slice()
	else {
		panic!("{back:?}")
	};
	assert_eq!(
		d.components,
		vec![y::slot_component::Ty::PotDecorations(vec![
			sherd, brick, brick, sherd
		])]
	);
}

#[test]
fn new_components_are_dropped_with_their_count() {
	let mut cx = Session::new();
	let content = y::slot_component::Ty::SulfurCubeContent(y::item_stack_template::Ty {
		item_id: id(Version::V26_2, "items", "tnt"),
		item_count: 1,
		added_component_count: 0,
		removed_component_count: 0,
		components: vec![],
		remove_components: vec![],
	});
	let packet = cursor_26_2(vec![content, y::slot_component::Ty::MaxStackSize(16)]);
	let out = run(
		Root::PlayToClient,
		Version::V26_2,
		Version::V26_1,
		&packet,
		y::play_to_client_packet::serialize,
		x::play_to_client_packet::deserialize,
		&mut cx,
	);
	let [x::play_to_client_packet::Ty::SetCursorItem(x::slot::Ty::Default(_, d))] = out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!(d.components, vec![x::slot_component::Ty::MaxStackSize(16)]);
	assert_eq!(d.added_component_count, 1);
}

#[test]
fn team_colors_become_optional() {
	let mut cx = Session::new();
	let mut body = vec![4, b't', b'e', b'a', b'm', 0];
	body.extend([0x08, 0, 1, b'n', 0x01, 0x00, 0x01, 0x10]);
	body.extend([
		0x08, 0, 1, b'p', 0x08, 0, 1, b's', 0x01, 3, b'B', b'o', b'b',
	]);
	let team = decode(&body, x::play_to_client_packet_teams::deserialize);
	let x::play_to_client_packet_teams_mode::Ty::Add(add) = &team.mode else {
		panic!("{team:?}")
	};
	assert_eq!(add.formatting, 16);
	let reset = |formatting| {
		let mut t = team.clone();
		if let x::play_to_client_packet_teams_mode::Ty::Add(a) = &mut t.mode {
			a.formatting = formatting;
		}
		x::play_to_client_packet::Ty::Teams(t)
	};
	for (formatting, expected) in [(16, None), (12, Some(12))] {
		let out = run(
			Root::PlayToClient,
			Version::V26_1,
			Version::V26_2,
			&reset(formatting),
			x::play_to_client_packet::serialize,
			y::play_to_client_packet::deserialize,
			&mut cx,
		);
		let [y::play_to_client_packet::Ty::Teams(t)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let y::play_to_client_packet_teams_mode::Ty::Add(a) = &t.mode else {
			panic!("{t:?}")
		};
		assert_eq!(a.formatting, expected);
		assert_eq!(a.players, vec!["Bob".to_owned()]);
		let back = run(
			Root::PlayToClient,
			Version::V26_2,
			Version::V26_1,
			&out[0],
			y::play_to_client_packet::serialize,
			x::play_to_client_packet::deserialize,
			&mut cx,
		);
		let [x::play_to_client_packet::Ty::Teams(t)] = back.as_slice() else {
			panic!("{back:?}")
		};
		let x::play_to_client_packet_teams_mode::Ty::Add(a) = &t.mode else {
			panic!("{t:?}")
		};
		assert_eq!(a.formatting, if expected.is_some() { 12 } else { 21 });
	}
}

fn server_up(
	p: &y::play_to_server_packet::Ty,
	cx: &mut Session,
) -> Vec<z::play_to_server_packet::Ty> {
	run(
		Root::PlayToServer,
		Version::V26_2,
		Version::V26_3,
		p,
		y::play_to_server_packet::serialize,
		z::play_to_server_packet::deserialize,
		cx,
	)
}

#[test]
fn teleport_confirm_waits_for_the_position() {
	let mut cx = Session::new();
	assert!(server_up(&y::play_to_server_packet::Ty::TeleportConfirm(7), &mut cx).is_empty());
	let look = y::play_to_server_packet_position_look::Ty {
		x: 1.5,
		y: 70.0,
		z: -3.5,
		yaw: 90.0,
		pitch: 10.0,
		flags: decode(&[1], y::play_to_server_movement_flags::deserialize),
	};
	let out = server_up(
		&y::play_to_server_packet::Ty::PositionLook(look.clone()),
		&mut cx,
	);
	let [z::play_to_server_packet::Ty::TeleportConfirm(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(c.teleport_id, c.x, c.y, c.z, c.y_rot, c.x_rot),
		(7, 1.5, 70.0, -3.5, 90.0, 10.0)
	);
	let out = server_up(&y::play_to_server_packet::Ty::PositionLook(look), &mut cx);
	assert!(matches!(
		out.as_slice(),
		[z::play_to_server_packet::Ty::PositionLook(_)]
	));

	let back = run(
		Root::PlayToServer,
		Version::V26_3,
		Version::V26_2,
		&z::play_to_server_packet::Ty::TeleportConfirm(c.clone()),
		z::play_to_server_packet::serialize,
		y::play_to_server_packet::deserialize,
		&mut cx,
	);
	let [y::play_to_server_packet::Ty::TeleportConfirm(7), y::play_to_server_packet::Ty::PositionLook(p)] =
		back.as_slice()
	else {
		panic!("{back:?}")
	};
	assert_eq!(
		(p.x, p.y, p.z, p.yaw, p.pitch),
		(1.5, 70.0, -3.5, 90.0, 10.0)
	);
}

#[test]
fn positions_without_tick_end_get_one() {
	let mut cx = Session::new();
	let position = || {
		y::play_to_server_packet::Ty::Position(y::play_to_server_packet_position::Ty {
			x: 0.5,
			y: 64.0,
			z: 0.5,
			flags: decode(&[1], y::play_to_server_movement_flags::deserialize),
		})
	};
	let names = |out: Vec<z::play_to_server_packet::Ty>| {
		out.iter()
			.map(|p| match p {
				z::play_to_server_packet::Ty::Position(_) => "position",
				z::play_to_server_packet::Ty::TickEnd(_) => "tick_end",
				other => panic!("{other:?}"),
			})
			.collect::<Vec<_>>()
	};
	assert_eq!(names(server_up(&position(), &mut cx)), ["position"]);
	assert_eq!(
		names(server_up(&position(), &mut cx)),
		["tick_end", "position"]
	);
	let tick = y::play_to_server_packet::Ty::TickEnd(y::play_to_server_packet_tick_end::Ty {});
	assert_eq!(names(server_up(&tick, &mut cx)), ["tick_end"]);
	assert_eq!(names(server_up(&position(), &mut cx)), ["position"]);
}

#[test]
fn spectator_left_click_swings() {
	let mut cx = Session::new();
	for (action, expected) in [(0, None), (43, Some(42))] {
		let out = run(
			Root::PlayToServer,
			Version::V26_2,
			Version::V26_1,
			&y::play_to_server_packet::Ty::SpectatorAction(action),
			y::play_to_server_packet::serialize,
			x::play_to_server_packet::deserialize,
			&mut cx,
		);
		match (out.as_slice(), expected) {
			([x::play_to_server_packet::Ty::ArmAnimation(_)], None) => {}
			([x::play_to_server_packet::Ty::SpectateEntity(id)], Some(e)) => assert_eq!(*id, e),
			_ => panic!("{out:?}"),
		}
	}
}

#[test]
fn dig_status_skips_the_new_direction_action() {
	let mut cx = Session::new();
	let dig = |status| {
		z::play_to_server_packet::Ty::BlockDig(z::play_to_server_packet_block_dig::Ty {
			status,
			location: z::position::Ty { x: 0, z: 0, y: 64 },
			face: 1,
			sequence: 4,
		})
	};
	let statuses: Vec<i32> = [0, 1, 2, 3]
		.into_iter()
		.flat_map(|s| {
			run(
				Root::PlayToServer,
				Version::V26_3,
				Version::V26_2,
				&dig(s),
				z::play_to_server_packet::serialize,
				y::play_to_server_packet::deserialize,
				&mut cx,
			)
		})
		.map(|p| match p {
			y::play_to_server_packet::Ty::BlockDig(d) => d.status,
			other => panic!("{other:?}"),
		})
		.collect();
	assert_eq!(statuses, [0, 1, 2]);
}

#[test]
fn exact_matchers_carry_the_right_component_ids() {
	let mut cx = Session::new();
	let predicate = v1_21_5::item_block_predicate::Ty {
		block_set: None,
		properties: None,
		nbt: None,
		components: v1_21_5::data_component_matchers::Ty {
			exact_matchers: vec![v1_21_5::slot_component::Ty::Rarity(
				v1_21_5::slot_component_rarity::Ty::Rare,
			)],
			partial_matchers: vec![],
		},
	};
	let slot = v1_21_5::slot::Ty::Default(
		1,
		v1_21_5::slot_default::Ty {
			item_id: id(Version::V1_21_5, "items", "stone"),
			added_component_count: 1,
			removed_component_count: 0,
			components: vec![v1_21_5::slot_component::Ty::CanPlaceOn(vec![predicate])],
			remove_components: vec![],
		},
	);
	let out = run(
		Root::PlayToClient,
		Version::V1_21_5,
		Version::V26_3,
		&v1_21_5::play_to_client_packet::Ty::SetCursorItem(slot),
		v1_21_5::play_to_client_packet::serialize,
		z::play_to_client_packet::deserialize,
		&mut cx,
	);
	let [z::play_to_client_packet::Ty::SetCursorItem(z::slot::Ty::Default(_, d))] = out.as_slice()
	else {
		panic!("{out:?}")
	};
	let [z::slot_component::Ty::CanPlaceOn(predicates)] = d.components.as_slice() else {
		panic!("{d:?}")
	};
	assert!(matches!(
		predicates[0].components.exact_matchers.as_slice(),
		[z::slot_component::Ty::Rarity(
			z::slot_component_rarity::Ty::Rare
		)]
	));
}
