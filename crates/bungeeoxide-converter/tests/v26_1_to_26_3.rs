use bungeeoxide_converter::gen::convert::{
	convert, v1_21_5, v26_1 as x, v26_2 as y, v26_3 as z, Root,
};
use bungeeoxide_converter::pds::{Input, Output};
use bungeeoxide_converter::pds_native::pds_entityDelta::{Delta, Step, Ty as EntityDelta};
use bungeeoxide_converter::registry;
use bungeeoxide_converter::Session;
use bungeeoxide_converter::Version;
use bungeeoxide_nbt::{Payload, Tag};

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

fn login_26_1(entity: i32) -> x::play_to_client_packet::Ty {
	x::play_to_client_packet::Ty::Login(x::play_to_client_packet_login::Ty {
		entity_id: entity,
		is_hardcore: false,
		world_names: vec!["minecraft:overworld".into()],
		max_players: 20,
		view_distance: 8,
		simulation_distance: 8,
		reduced_debug_info: false,
		enable_respawn_screen: true,
		do_limited_crafting: false,
		world_state: x::play_to_client_spawn_info::Ty {
			dimension: 0,
			name: "minecraft:overworld".into(),
			hashed_seed: 0,
			gamemode: x::play_to_client_spawn_info_gamemode::Ty::Survival,
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

fn up_26_2(p: &x::play_to_client_packet::Ty, cx: &mut Session) -> y::play_to_client_packet::Ty {
	let out = run(
		Root::PlayToClient,
		Version::V26_1,
		Version::V26_2,
		p,
		x::play_to_client_packet::serialize,
		y::play_to_client_packet::deserialize,
		cx,
	);
	let [p] = <[_; 1]>::try_from(out).unwrap_or_else(|out| panic!("{out:?}"));
	p
}

fn server_down_26_1(p: &y::play_to_server_packet::Ty, cx: &mut Session) -> x::play_to_server_packet::Ty {
	let out = run(
		Root::PlayToServer,
		Version::V26_2,
		Version::V26_1,
		p,
		y::play_to_server_packet::serialize,
		x::play_to_server_packet::deserialize,
		cx,
	);
	let [p] = <[_; 1]>::try_from(out).unwrap_or_else(|out| panic!("{out:?}"));
	p
}

fn fake_id(cx: &mut Session) -> i32 {
	let y::play_to_client_packet::Ty::Login(l) = up_26_2(&login_26_1(0), cx) else {
		panic!("no login")
	};
	assert!(l.entity_id < -1, "{}", l.entity_id);
	l.entity_id
}

fn spawn_26_1(entity: i32, kind: &str, data: i32) -> x::play_to_client_packet::Ty {
	x::play_to_client_packet::Ty::SpawnEntity(x::play_to_client_packet_spawn_entity::Ty {
		entity_id: entity,
		object_uuid: 0x1234,
		r#type: id(Version::V26_1, "entities", kind),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		velocity: vec![0],
		pitch: 0,
		yaw: 0,
		head_pitch: 0,
		object_data: data,
	})
}

fn metadata_26_1(
	entity: i32,
	key: u8,
	value: x::entity_metadata_entry_value::Ty,
) -> x::play_to_client_packet::Ty {
	x::play_to_client_packet::Ty::EntityMetadata(x::play_to_client_packet_entity_metadata::Ty {
		entity_id: entity,
		metadata: vec![x::entity_metadata_entry::Ty { key, value }],
	})
}

#[test]
fn entity_id_zero_becomes_the_fake_id_for_26_2_clients() {
	let mut cx = Session::new();
	let y::play_to_client_packet::Ty::Login(l) = up_26_2(&login_26_1(5), &mut cx) else {
		panic!()
	};
	assert_eq!(l.entity_id, 5);
	let fake = fake_id(&mut cx);

	let y::play_to_client_packet::Ty::SpawnEntity(s) = up_26_2(&spawn_26_1(0, "arrow", 0), &mut cx)
	else {
		panic!()
	};
	assert_eq!((s.entity_id, s.object_data), (fake, fake));
	let y::play_to_client_packet::Ty::SpawnEntity(s) = up_26_2(&spawn_26_1(9, "fireball", 0), &mut cx)
	else {
		panic!()
	};
	assert_eq!((s.entity_id, s.object_data), (9, fake));
	let y::play_to_client_packet::Ty::SpawnEntity(s) = up_26_2(&spawn_26_1(10, "item_frame", 0), &mut cx)
	else {
		panic!()
	};
	assert_eq!((s.entity_id, s.object_data), (10, 0));

	let y::play_to_client_packet::Ty::EntityDestroy(ids) =
		up_26_2(&x::play_to_client_packet::Ty::EntityDestroy(vec![3, 0]), &mut cx)
	else {
		panic!()
	};
	assert_eq!(ids, [3, fake]);

	let y::play_to_client_packet::Ty::Camera(c) =
		up_26_2(&x::play_to_client_packet::Ty::Camera(0), &mut cx)
	else {
		panic!()
	};
	assert_eq!(c, fake);

	let y::play_to_client_packet::Ty::SetPassengers(p) = up_26_2(
		&x::play_to_client_packet::Ty::SetPassengers(x::play_to_client_packet_set_passengers::Ty {
			entity_id: 0,
			passengers: vec![0, 4],
		}),
		&mut cx,
	) else {
		panic!()
	};
	assert_eq!((p.entity_id, p.passengers), (fake, vec![fake, 4]));

	let y::play_to_client_packet::Ty::Collect(c) = up_26_2(
		&x::play_to_client_packet::Ty::Collect(x::play_to_client_packet_collect::Ty {
			collected_entity_id: 6,
			collector_entity_id: 0,
			pickup_item_count: 1,
		}),
		&mut cx,
	) else {
		panic!()
	};
	assert_eq!((c.collected_entity_id, c.collector_entity_id), (6, fake));

	let y::play_to_client_packet::Ty::FacePlayer(f) = up_26_2(
		&x::play_to_client_packet::Ty::FacePlayer(x::play_to_client_packet_face_player::Ty {
			feet_eyes: 0,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			is_entity: x::play_to_client_packet_face_player_is_entity::Ty::True(
				x::play_to_client_packet_face_player_is_entity_true::Ty {
					entity_id: 0,
					entity_feet_eyes: 1,
				},
			),
		}),
		&mut cx,
	) else {
		panic!()
	};
	assert!(matches!(
		f.is_entity,
		y::play_to_client_packet_face_player_is_entity::Ty::True(t) if t.entity_id == fake
	));

	let y::play_to_client_packet::Ty::WorldParticles(p) = up_26_2(
		&x::play_to_client_packet::Ty::WorldParticles(x::play_to_client_packet_world_particles::Ty {
			long_distance: false,
			always_show: false,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			offset_x: 0.0,
			offset_y: 0.0,
			offset_z: 0.0,
			velocity_offset: 0.0,
			amount: 1,
			particle: x::particle::Ty::Vibration(x::particle_vibration::Ty {
				position: x::particle_vibration_position::Ty::Entity(
					x::particle_vibration_position_entity::Ty {
						entity_id: 0,
						entity_eye_height: 1.5,
					},
				),
				ticks: 20,
			}),
		}),
		&mut cx,
	) else {
		panic!()
	};
	assert!(matches!(
		&p.particle,
		y::particle::Ty::Vibration(v) if matches!(&v.position, y::particle_vibration_position::Ty::Entity(e) if e.entity_id == fake)
	));
}

#[test]
fn zero_meaning_none_stays_zero() {
	let mut cx = Session::new();
	let fake = fake_id(&mut cx);

	let y::play_to_client_packet::Ty::DamageEvent(d) = up_26_2(
		&x::play_to_client_packet::Ty::DamageEvent(x::play_to_client_packet_damage_event::Ty {
			entity_id: 0,
			source_type_id: 2,
			source_cause_id: 1,
			source_direct_id: 0,
			source_position: None,
		}),
		&mut cx,
	) else {
		panic!()
	};
	assert_eq!(
		(d.entity_id, d.source_cause_id, d.source_direct_id),
		(fake, fake + 1, 0)
	);

	let y::play_to_client_packet::Ty::AttachEntity(a) = up_26_2(
		&x::play_to_client_packet::Ty::AttachEntity(x::play_to_client_packet_attach_entity::Ty {
			entity_id: 0,
			vehicle_id: 0,
		}),
		&mut cx,
	) else {
		panic!()
	};
	assert_eq!((a.entity_id, a.vehicle_id), (fake, 0));

	up_26_2(&spawn_26_1(7, "frog", 0), &mut cx);
	let unsigned = |v| x::entity_metadata_entry_value::Ty::OptionalUnsignedInt(v);
	for (value, expected) in [(1, fake + 1), (0, 0), (8, 8)] {
		let y::play_to_client_packet::Ty::EntityMetadata(m) =
			up_26_2(&metadata_26_1(7, 19, unsigned(value)), &mut cx)
		else {
			panic!()
		};
		assert!(matches!(
			m.metadata.as_slice(),
			[y::entity_metadata_entry::Ty { key: 19, value: y::entity_metadata_entry_value::Ty::OptionalUnsignedInt(v) }] if *v == expected
		), "{m:?}");
	}

	up_26_2(&spawn_26_1(8, "firework_rocket", 0), &mut cx);
	let y::play_to_client_packet::Ty::EntityMetadata(m) =
		up_26_2(&metadata_26_1(8, 9, unsigned(1)), &mut cx)
	else {
		panic!()
	};
	assert!(matches!(
		m.metadata.as_slice(),
		[y::entity_metadata_entry::Ty { value: y::entity_metadata_entry_value::Ty::OptionalUnsignedInt(v), .. }] if *v == fake + 1
	));

	up_26_2(&spawn_26_1(11, "guardian", 0), &mut cx);
	let y::play_to_client_packet::Ty::EntityMetadata(m) = up_26_2(
		&metadata_26_1(11, 17, x::entity_metadata_entry_value::Ty::Int(0)),
		&mut cx,
	) else {
		panic!()
	};
	assert!(matches!(
		m.metadata.as_slice(),
		[y::entity_metadata_entry::Ty { value: y::entity_metadata_entry_value::Ty::Int(0), .. }]
	));

	let y::play_to_client_packet::Ty::EntityMetadata(m) = up_26_2(
		&metadata_26_1(0, 1, x::entity_metadata_entry_value::Ty::Int(300)),
		&mut cx,
	) else {
		panic!()
	};
	assert_eq!(m.entity_id, fake);
}

#[test]
fn fake_id_goes_back_to_zero_for_the_server() {
	let mut cx = Session::new();
	let fake = fake_id(&mut cx);
	use x::play_to_server_packet::Ty as X;
	use y::play_to_server_packet::Ty as Y;

	assert!(matches!(server_down_26_1(&Y::Attack(fake), &mut cx), X::Attack(0)));
	assert!(matches!(server_down_26_1(&Y::Attack(5), &mut cx), X::Attack(5)));
	assert!(matches!(server_down_26_1(&Y::Attack(-1), &mut cx), X::Attack(-1)));
	assert!(matches!(
		server_down_26_1(&Y::SpectatorAction(fake + 1), &mut cx),
		X::SpectateEntity(0)
	));
	let X::UseEntity(u) = server_down_26_1(
		&Y::UseEntity(y::play_to_server_packet_use_entity::Ty {
			target: fake,
			hand: y::play_to_server_packet_use_entity_hand::Ty::MainHand,
			location: vec![0],
			sneaking: false,
		}),
		&mut cx,
	) else {
		panic!()
	};
	assert_eq!(u.target, 0);
	let X::PickItemFromEntity(p) = server_down_26_1(
		&Y::PickItemFromEntity(y::play_to_server_packet_pick_item_from_entity::Ty {
			entity_id: fake,
			include_data: true,
		}),
		&mut cx,
	) else {
		panic!()
	};
	assert_eq!(p.entity_id, 0);
	let X::EntityAction(a) = server_down_26_1(
		&Y::EntityAction(y::play_to_server_packet_entity_action::Ty {
			entity_id: fake,
			action_id: y::play_to_server_packet_entity_action_action_id::Ty::StartSprinting,
			jump_boost: 0,
		}),
		&mut cx,
	) else {
		panic!()
	};
	assert_eq!(a.entity_id, 0);
	let X::QueryEntityNbt(q) = server_down_26_1(
		&Y::QueryEntityNbt(y::play_to_server_packet_query_entity_nbt::Ty {
			transaction_id: 3,
			entity_id: fake,
		}),
		&mut cx,
	) else {
		panic!()
	};
	assert_eq!((q.transaction_id, q.entity_id), (3, 0));
	let X::UpdateCommandBlockMinecart(m) = server_down_26_1(
		&Y::UpdateCommandBlockMinecart(y::play_to_server_packet_update_command_block_minecart::Ty {
			entity_id: fake,
			command: "say hi".into(),
			track_output: false,
		}),
		&mut cx,
	) else {
		panic!()
	};
	assert_eq!(m.entity_id, 0);
}

#[test]
fn fake_id_survives_the_26_3_step() {
	let mut cx = Session::new();
	let to_26_3 = |p: &x::play_to_client_packet::Ty, cx: &mut Session| {
		run(
			Root::PlayToClient,
			Version::V26_1,
			Version::V26_3,
			p,
			x::play_to_client_packet::serialize,
			z::play_to_client_packet::deserialize,
			cx,
		)
	};
	let out = to_26_3(&login_26_1(0), &mut cx);
	let [z::play_to_client_packet::Ty::Login(l)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let fake = l.entity_id;
	assert!(fake < -1);
	let out = to_26_3(
		&metadata_26_1(0, 1, x::entity_metadata_entry_value::Ty::Int(300)),
		&mut cx,
	);
	let [z::play_to_client_packet::Ty::EntityMetadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(m.entity_id, fake);
	let out = to_26_3(
		&x::play_to_client_packet::Ty::RelEntityMove(x::play_to_client_packet_rel_entity_move::Ty {
			entity_id: 0,
			d_x: 1,
			d_y: 2,
			d_z: 3,
			on_ground: true,
		}),
		&mut cx,
	);
	let [z::play_to_client_packet::Ty::RelEntityMove(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(r.entity_id, fake);
	let back = run(
		Root::PlayToServer,
		Version::V26_3,
		Version::V26_1,
		&z::play_to_server_packet::Ty::Attack(fake),
		z::play_to_server_packet::serialize,
		x::play_to_server_packet::deserialize,
		&mut cx,
	);
	assert!(matches!(back.as_slice(), [x::play_to_server_packet::Ty::Attack(0)]));
}

#[test]
fn brewing_stands_get_their_totals() {
	let mut cx = Session::new();
	let open = |id, kind| y::play_to_client_packet_open_window::Ty {
		window_id: id,
		inventory_type: kind,
		window_title: Tag::new(Payload::String("Brewing Stand".into())),
	};
	let out = up(&y::play_to_client_packet::Ty::OpenWindow(open(3, 11)), &mut cx);
	let totals: Vec<(i32, i16, i16)> = out[1..]
		.iter()
		.map(|p| match p {
			z::play_to_client_packet::Ty::CraftProgressBar(b) => (b.window_id, b.property, b.value),
			other => panic!("{other:?}"),
		})
		.collect();
	assert!(matches!(&out[0], z::play_to_client_packet::Ty::OpenWindow(w) if w.window_id == 3));
	assert_eq!(totals, [(3, 2, 400), (3, 3, 20)]);
	let out = up(&y::play_to_client_packet::Ty::OpenWindow(open(4, 14)), &mut cx);
	assert!(matches!(out.as_slice(), [z::play_to_client_packet::Ty::OpenWindow(_)]));

	let bar = |id, property| {
		z::play_to_client_packet::Ty::CraftProgressBar(z::play_to_client_packet_craft_progress_bar::Ty {
			window_id: id,
			property,
			value: 7,
		})
	};
	down(&z::play_to_client_packet::Ty::OpenWindow(open(5, 11)), &mut cx);
	let kept: Vec<usize> = [bar(5, 0), bar(5, 1), bar(5, 2), bar(5, 3)]
		.iter()
		.map(|p| down(p, &mut cx).len())
		.collect();
	assert_eq!(kept, [1, 1, 0, 0]);
	down(&z::play_to_client_packet::Ty::OpenWindow(open(6, 14)), &mut cx);
	assert_eq!(down(&bar(6, 2), &mut cx).len(), 1);
}

#[test]
fn new_map_decorations_become_target_x() {
	let mut cx = Session::new();
	let icon = |kind| z::play_to_client_packet_map_icons_items_items::Ty {
		r#type: kind,
		x: 1,
		z: 2,
		direction: 0,
		display_name: None,
	};
	let out = down(
		&z::play_to_client_packet::Ty::Map(z::play_to_client_packet_map::Ty {
			item_damage: 0,
			scale: 0,
			locked: false,
			icons: Some(vec![icon(37), icon(26), icon(39)]),
			columns: z::play_to_client_packet_map_columns::Ty::V0,
		}),
		&mut cx,
	);
	let [y::play_to_client_packet::Ty::Map(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let kinds: Vec<i32> = m.icons.iter().flatten().map(|i| i.r#type).collect();
	assert_eq!(kinds, [4, 26, 4]);

	let decoration = |kind: &str| {
		Payload::Compound(vec![
			bungeeoxide_nbt::Entry::new("type", Payload::String(kind.into())),
			bungeeoxide_nbt::Entry::new("x", Payload::Double(1.0)),
			bungeeoxide_nbt::Entry::new("z", Payload::Double(2.0)),
			bungeeoxide_nbt::Entry::new("rotation", Payload::Float(0.0)),
		])
	};
	let decorations = Tag::new(Payload::Compound(vec![
		bungeeoxide_nbt::Entry::new("camp", decoration("minecraft:abandoned_camp")),
		bungeeoxide_nbt::Entry::new("x", decoration("minecraft:red_x")),
	]));
	let out = down(
		&z::play_to_client_packet::Ty::SetCursorItem(z::slot::Ty::Default(
			1,
			z::slot_default::Ty {
				item_id: id(Version::V26_3, "items", "filled_map"),
				added_component_count: 1,
				removed_component_count: 0,
				components: vec![z::slot_component::Ty::MapDecorations(decorations)],
				remove_components: vec![],
			},
		)),
		&mut cx,
	);
	let [y::play_to_client_packet::Ty::SetCursorItem(y::slot::Ty::Default(_, d))] = out.as_slice() else {
		panic!("{out:?}")
	};
	let [y::slot_component::Ty::MapDecorations(t)] = d.components.as_slice() else {
		panic!("{d:?}")
	};
	let kind = |name: &str| t.value.field(name).and_then(|p| p.field("type")).and_then(Payload::as_str);
	assert_eq!(kind("camp"), Some("minecraft:target_x"));
	assert_eq!(kind("x"), Some("minecraft:red_x"));
}

fn sherd_list(ids: &[&str]) -> Payload {
	let mut pot = Payload::Compound(vec![]);
	pot.put(
		"sherds",
		Payload::List(bungeeoxide_nbt::List::of(
			ids.iter().map(|i| Payload::String((*i).into())).collect(),
		)),
	);
	pot
}

fn sides_of(pot: &Payload) -> Vec<&str> {
	["back", "left", "right", "front"]
		.iter()
		.map(|s| {
			pot.field("sherds")
				.and_then(|p| p.field(s))
				.and_then(|p| p.field("id"))
				.and_then(Payload::as_str)
				.unwrap_or("missing")
		})
		.collect()
}

fn list_of(pot: &Payload) -> Vec<String> {
	match pot.field("sherds") {
		Some(Payload::List(l)) => l
			.values()
			.iter()
			.map(|v| v.as_str().unwrap_or("?").to_owned())
			.collect(),
		other => panic!("{other:?}"),
	}
}

#[test]
fn pot_sherds_become_sides_and_back() {
	let mut cx = Session::new();
	let tile = |nbt: Payload| y::play_to_client_packet_tile_entity_data::Ty {
		location: y::position::Ty { x: 1, z: 2, y: 3 },
		action: 0,
		nbt_data: Some(Tag::new(nbt)),
	};
	let pot = sherd_list(&["minecraft:angler_pottery_sherd", "minecraft:brick", "minecraft:skull_pottery_sherd"]);
	let out = up(&y::play_to_client_packet::Ty::TileEntityData(tile(pot)), &mut cx);
	let [z::play_to_client_packet::Ty::TileEntityData(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let nbt = &t.nbt_data.as_ref().unwrap().value;
	assert_eq!(
		sides_of(nbt),
		[
			"minecraft:angler_pottery_sherd",
			"minecraft:brick",
			"minecraft:skull_pottery_sherd",
			"minecraft:brick"
		]
	);
	let back = down(&out[0], &mut cx);
	let [y::play_to_client_packet::Ty::TileEntityData(b)] = back.as_slice() else {
		panic!("{back:?}")
	};
	assert_eq!(
		list_of(&b.nbt_data.as_ref().unwrap().value),
		[
			"minecraft:angler_pottery_sherd",
			"minecraft:brick",
			"minecraft:skull_pottery_sherd",
			"minecraft:brick"
		]
	);

	let pot_item = |nbt: Payload| {
		y::play_to_client_packet::Ty::SetCursorItem(y::slot::Ty::Default(
			1,
			y::slot_default::Ty {
				item_id: id(Version::V26_2, "items", "decorated_pot"),
				added_component_count: 1,
				removed_component_count: 0,
				components: vec![y::slot_component::Ty::BlockEntityData(y::slot_component_block_entity_data::Ty {
					r#type: 0,
					data: Tag::new(nbt),
				})],
				remove_components: vec![],
			},
		))
	};
	let out = up(&pot_item(sherd_list(&["minecraft:arms_up_pottery_sherd"])), &mut cx);
	let [z::play_to_client_packet::Ty::SetCursorItem(z::slot::Ty::Default(_, d))] = out.as_slice() else {
		panic!("{out:?}")
	};
	let [z::slot_component::Ty::BlockEntityData(e)] = d.components.as_slice() else {
		panic!("{d:?}")
	};
	assert_eq!(sides_of(&e.data.value)[0], "minecraft:arms_up_pottery_sherd");
	let back = down(&out[0], &mut cx);
	let [y::play_to_client_packet::Ty::SetCursorItem(y::slot::Ty::Default(_, d))] = back.as_slice() else {
		panic!("{back:?}")
	};
	let [y::slot_component::Ty::BlockEntityData(e)] = d.components.as_slice() else {
		panic!("{d:?}")
	};
	assert_eq!(list_of(&e.data.value)[0], "minecraft:arms_up_pottery_sherd");
}

#[test]
fn pot_sherds_in_chunks() {
	use bungeeoxide_converter::custom::chunks::{decode as decode_chunk, encode_chunk, BlockEntity, Column, Light, Packet, Section};
	let mut cx = Session::new();
	let mut col = Column::new(0, 0, true, Version::V26_2);
	col.sections = (0..24).map(|_| Some(Section::filled(0, Some(0)))).collect();
	col.light = Some(Light::new(24, true));
	col.entities = vec![BlockEntity {
		x: 1,
		y: 64,
		z: 2,
		kind: Some(0),
		name: String::new(),
		nbt: Some(Tag::new(sherd_list(&["minecraft:heart_pottery_sherd"]))),
	}];
	let mut converted = Vec::new();
	for bytes in encode_chunk(Version::V26_2, &col).unwrap() {
		converted.extend(convert(Root::PlayToClient, Version::V26_2, Version::V26_3, &bytes, &mut cx).unwrap());
	}
	let entities: Vec<BlockEntity> = converted
		.iter()
		.filter_map(|b| match decode_chunk(Version::V26_3, b).unwrap() {
			Some(Packet::Chunk(c)) => Some(c.entities),
			_ => None,
		})
		.flatten()
		.collect();
	let [e] = entities.as_slice() else {
		panic!("{entities:?}")
	};
	assert_eq!(sides_of(&e.nbt.as_ref().unwrap().value)[0], "minecraft:heart_pottery_sherd");
	let mut back = Vec::new();
	for bytes in &converted {
		back.extend(convert(Root::PlayToClient, Version::V26_3, Version::V26_2, bytes, &mut cx).unwrap());
	}
	let entities: Vec<BlockEntity> = back
		.iter()
		.filter_map(|b| match decode_chunk(Version::V26_2, b).unwrap() {
			Some(Packet::Chunk(c)) => Some(c.entities),
			_ => None,
		})
		.flatten()
		.collect();
	assert_eq!(list_of(&entities[0].nbt.as_ref().unwrap().value)[0], "minecraft:heart_pottery_sherd");
}
