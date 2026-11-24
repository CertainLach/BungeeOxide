use bungeeoxide_converter::gen::convert::{convert, v26_3 as a, v26_4_snapshot_10 as b, Root};
use bungeeoxide_converter::pds::{Input, Output};
use bungeeoxide_converter::pds_native::pds_entityDelta::{self, Step};
use bungeeoxide_converter::pds_native::pds_vecDelta;
use bungeeoxide_converter::registry;
use bungeeoxide_converter::Session;
use bungeeoxide_converter::Version;

const OLD: Version = Version::V26_3;
const NEW: Version = Version::V26_4_snapshot_10;

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

fn up(p: &a::play_to_client_packet::Ty, cx: &mut Session) -> Vec<b::play_to_client_packet::Ty> {
	run(
		Root::PlayToClient,
		OLD,
		NEW,
		p,
		a::play_to_client_packet::serialize,
		b::play_to_client_packet::deserialize,
		cx,
	)
}

fn down(p: &b::play_to_client_packet::Ty, cx: &mut Session) -> Vec<a::play_to_client_packet::Ty> {
	run(
		Root::PlayToClient,
		NEW,
		OLD,
		p,
		b::play_to_client_packet::serialize,
		a::play_to_client_packet::deserialize,
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

fn step(ticks: i32, d: i16) -> Step {
	Step {
		ticks,
		dX: d,
		dY: -d,
		dZ: 0,
	}
}

#[test]
fn stepped_moves_start_late_going_up_and_lose_the_flag_going_down() {
	let mut cx = Session::new();
	let out = up(
		&a::play_to_client_packet::Ty::RelEntityMove(
			a::play_to_client_packet_rel_entity_move::Ty {
				entity_id: 5,
				r#move: pds_entityDelta::Ty {
					onGround: true,
					delta: pds_entityDelta::Delta::Stepped(vec![step(3, 40), step(2, -7)]),
				},
			},
		),
		&mut cx,
	);
	let [b::play_to_client_packet::Ty::RelEntityMove(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		m.r#move,
		pds_vecDelta::Ty {
			onGround: true,
			delta: pds_vecDelta::Delta::Stepped {
				startImmediately: false,
				steps: vec![step(3, 40), step(2, -7)],
			},
		}
	);
	let look = b::play_to_client_packet::Ty::EntityMoveLook(
		b::play_to_client_packet_entity_move_look::Ty {
			entity_id: 5,
			r#move: pds_vecDelta::Ty {
				onGround: false,
				delta: pds_vecDelta::Delta::Stepped {
					startImmediately: true,
					steps: vec![step(1, 9)],
				},
			},
			yaw: 10,
			pitch: -3,
		},
	);
	let back = down(&look, &mut cx);
	let [a::play_to_client_packet::Ty::EntityMoveLook(l)] = back.as_slice() else {
		panic!("{back:?}")
	};
	assert_eq!(
		l.r#move,
		pds_entityDelta::Ty {
			onGround: false,
			delta: pds_entityDelta::Delta::Stepped(vec![step(1, 9)]),
		}
	);
	assert_eq!((l.yaw, l.pitch), (10, -3));
	let linear = down(
		&b::play_to_client_packet::Ty::RelEntityMove(
			b::play_to_client_packet_rel_entity_move::Ty {
				entity_id: 5,
				r#move: pds_vecDelta::Ty {
					onGround: true,
					delta: pds_vecDelta::Delta::Linear {
						dX: 1,
						dY: 2,
						dZ: 3,
					},
				},
			},
		),
		&mut cx,
	);
	assert!(matches!(
		linear.as_slice(),
		[a::play_to_client_packet::Ty::RelEntityMove(m)]
			if m.r#move.delta == pds_entityDelta::Delta::Linear { dX: 1, dY: 2, dZ: 3 }
	));
}

#[test]
fn position_sync_carries_the_head_yaw() {
	let mut cx = Session::new();
	let head = a::play_to_client_packet::Ty::EntityHeadRotation(
		a::play_to_client_packet_entity_head_rotation::Ty {
			entity_id: 4,
			head_yaw: 64,
		},
	);
	up(&head, &mut cx);
	let sync = a::play_to_client_packet::Ty::SyncEntityPosition(
		a::play_to_client_packet_sync_entity_position::Ty {
			entity_id: 4,
			anon: a::play_to_client_packet_sync_entity_position_anon::Ty::Stepped(vec![
				a::play_to_client_packet_sync_entity_position_anon_stepped_items::Ty {
					x: 1.0,
					y: 2.0,
					z: 3.0,
					tick_offset: 3,
				},
			]),
			yaw: 10.0,
			pitch: 0.0,
			on_ground: true,
		},
	);
	let out = up(&sync, &mut cx);
	let [b::play_to_client_packet::Ty::SyncEntityPosition(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.head_yaw, 90.0);
	assert_eq!(s.yaw, 10.0);
	let b::play_to_client_packet_sync_entity_position_anon::Ty::Stepped(stepped) = &s.anon else {
		panic!("{s:?}")
	};
	assert!(!stepped.start_immediately);
	assert_eq!(stepped.steps.len(), 1);
	let back = down(&out[0], &mut cx);
	let [a::play_to_client_packet::Ty::SyncEntityPosition(p), a::play_to_client_packet::Ty::EntityHeadRotation(h)] =
		back.as_slice()
	else {
		panic!("{back:?}")
	};
	assert_eq!(
		p,
		&match sync {
			a::play_to_client_packet::Ty::SyncEntityPosition(p) => p,
			_ => unreachable!(),
		}
	);
	assert_eq!((h.entity_id, h.head_yaw), (4, 64));
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
			hashed_seed: 77,
			gamemode: a::play_to_client_spawn_info_gamemode::Ty::Survival,
			previous_gamemode: 0,
			is_debug: false,
			is_flat: false,
			death: None,
			portal_cooldown: 0,
			sea_level: 63,
		},
		enforces_secure_chat: false,
		online_mode: false,
	})
}

#[test]
fn explosion_knockback_moves_to_add_entity_motion() {
	let mut cx = Session::new();
	let joined = up(&login(42), &mut cx);
	let [b::play_to_client_packet::Ty::Login(l)] = joined.as_slice() else {
		panic!("{joined:?}")
	};
	assert_eq!(l.entity_id, 42);
	let explosion =
		a::play_to_client_packet::Ty::Explosion(a::play_to_client_packet_explosion::Ty {
			center: a::vec3f64::Ty {
				x: 1.0,
				y: 2.0,
				z: 3.0,
			},
			radius: 4.0,
			block_count: 2,
			player_knockback: Some(a::vec3f64::Ty {
				x: 0.5,
				y: 0.25,
				z: -1.0,
			}),
			explosion_particle: a::particle::Ty::ExplosionEmitter,
			sound: a::item_sound_holder::Ty::Id(id(OLD, "sounds", "entity.generic.explode") - 1),
			block_particles: vec![],
			play_sound: true,
		});
	let out = up(&explosion, &mut cx);
	let [b::play_to_client_packet::Ty::AddEntityMotion(m), b::play_to_client_packet::Ty::Explosion(e)] =
		out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!(m.entity_id, 42);
	assert!(matches!(
		e.explosion_particle,
		Some(b::particle::Ty::ExplosionEmitter)
	));
	let Some(b::item_sound_holder::Ty::Id(sound)) = e.sound else {
		panic!("{e:?}")
	};
	assert_eq!(
		registry::names(NEW, "sounds")
			.unwrap()
			.name(sound as u32 + 1),
		Some("entity.generic.explode")
	);
	let mut cx = Session::new();
	let joined = run(
		Root::PlayToClient,
		NEW,
		OLD,
		&joined[0],
		b::play_to_client_packet::serialize,
		a::play_to_client_packet::deserialize,
		&mut cx,
	);
	let [a::play_to_client_packet::Ty::Login(l)] = joined.as_slice() else {
		panic!("{joined:?}")
	};
	assert_eq!(
		(l.world_state.hashed_seed, l.world_state.is_debug),
		(0, false)
	);
	assert!(down(&out[0], &mut cx).is_empty());
	let mut silent = out[1].clone();
	if let b::play_to_client_packet::Ty::Explosion(e) = &mut silent {
		e.sound = None;
	}
	let back = down(&silent, &mut cx);
	let [a::play_to_client_packet::Ty::Explosion(e)] = back.as_slice() else {
		panic!("{back:?}")
	};
	let k = e.player_knockback.as_ref().expect("knockback");
	assert!((k.x - 0.5).abs() < 0.01 && (k.y - 0.25).abs() < 0.01 && (k.z + 1.0).abs() < 0.01);
	assert!(!e.play_sound);
	let again = down(&silent, &mut cx);
	assert!(matches!(
		again.as_slice(),
		[a::play_to_client_packet::Ty::Explosion(e)] if e.player_knockback.is_none()
	));
}

#[test]
fn particles_follow_the_registry() {
	let mut cx = Session::new();
	let particle = |p: b::particle::Ty| {
		b::play_to_client_packet::Ty::WorldParticles(b::play_to_client_packet_world_particles::Ty {
			particle: p,
			long_distance: false,
			always_show: false,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			offset_x: 0.0,
			offset_y: 0.0,
			offset_z: 0.0,
			x_max_speed: 0.0,
			y_max_speed: 0.0,
			z_max_speed: 0.0,
			amount: 1,
			randomization_type:
				b::play_to_client_packet_world_particles_randomization_type::Ty::default,
		})
	};
	let of = |out: Vec<a::play_to_client_packet::Ty>| match out.as_slice() {
		[a::play_to_client_packet::Ty::WorldParticles(w)] => w.particle.clone(),
		other => panic!("{other:?}"),
	};
	assert!(matches!(
		of(down(&particle(b::particle::Ty::Freezing), &mut cx)),
		a::particle::Ty::Snowflake
	));
	assert!(matches!(
		of(down(&particle(b::particle::Ty::CurrentDown(2.0)), &mut cx)),
		a::particle::Ty::CurrentDown
	));
	let old = |p: a::particle::Ty| {
		a::play_to_client_packet::Ty::WorldParticles(a::play_to_client_packet_world_particles::Ty {
			particle: p,
			long_distance: false,
			always_show: false,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			offset_x: 0.0,
			offset_y: 0.0,
			offset_z: 0.0,
			x_max_speed: 0.0,
			y_max_speed: 0.0,
			z_max_speed: 0.0,
			amount: 1,
			randomization_type:
				a::play_to_client_packet_world_particles_randomization_type::Ty::default,
		})
	};
	let new = |out: Vec<b::play_to_client_packet::Ty>| match out.as_slice() {
		[b::play_to_client_packet::Ty::WorldParticles(w)] => w.particle.clone(),
		other => panic!("{other:?}"),
	};
	let b::particle::Ty::Item(t) = new(up(&old(a::particle::Ty::ItemSnowball), &mut cx)) else {
		panic!("snowball")
	};
	assert_eq!(name(NEW, "items", t.item_id), "snowball");
	assert!(matches!(
		new(up(&old(a::particle::Ty::BubbleColumnUp), &mut cx)),
		b::particle::Ty::BubbleColumnUp(c) if c == 0.5
	));
	assert!(matches!(
		new(up(&old(a::particle::Ty::Glow), &mut cx)),
		b::particle::Ty::Glow
	));
}

fn cursor(item: &str, components: Vec<b::slot_component::Ty>) -> b::play_to_client_packet::Ty {
	b::play_to_client_packet::Ty::SetCursorItem(b::slot::Ty::Default(
		1,
		b::slot_default::Ty {
			item_id: id(NEW, "items", item),
			added_component_count: components.len() as i32,
			removed_component_count: 0,
			components,
			remove_components: vec![],
		},
	))
}

#[test]
fn new_items_fall_back_and_old_ones_follow_names() {
	let mut cx = Session::new();
	for (new, old) in [
		("frostbite_spawn_egg", "stray_spawn_egg"),
		("music_disc_flurry", "music_disc_13"),
		("packed_ice_bricks", "packed_ice"),
		("diamond_sword", "diamond_sword"),
	] {
		let out = down(&cursor(new, vec![]), &mut cx);
		let [a::play_to_client_packet::Ty::SetCursorItem(a::slot::Ty::Default(_, d))] =
			out.as_slice()
		else {
			panic!("{out:?}")
		};
		assert_eq!(name(OLD, "items", d.item_id), old);
	}
	assert_ne!(
		id(OLD, "items", "diamond_sword"),
		id(NEW, "items", "diamond_sword")
	);
}

#[test]
fn dispense_behavior_is_dropped_with_its_count() {
	let mut cx = Session::new();
	let tag = bungeeoxide_nbt::Tag::new(bungeeoxide_nbt::Payload::from("minecraft:snowball"));
	let packet = cursor(
		"snowball",
		vec![
			b::slot_component::Ty::DispenseBehavior(tag),
			b::slot_component::Ty::MaxStackSize(16),
		],
	);
	let out = down(&packet, &mut cx);
	let [a::play_to_client_packet::Ty::SetCursorItem(a::slot::Ty::Default(_, d))] = out.as_slice()
	else {
		panic!("{out:?}")
	};
	assert_eq!(d.components, vec![a::slot_component::Ty::MaxStackSize(16)]);
	assert_eq!(d.added_component_count, 1);
}

#[test]
fn the_jukebox_menu_shifts_window_types() {
	let mut cx = Session::new();
	let window = |t: i32| {
		a::play_to_client_packet::Ty::OpenWindow(a::play_to_client_packet_open_window::Ty {
			window_id: 1,
			inventory_type: t,
			window_title: bungeeoxide_nbt::Tag::new(bungeeoxide_nbt::Payload::from("x")),
		})
	};
	for (old, new) in [(2, 2), (8, 9), (20, 21)] {
		let out = up(&window(old), &mut cx);
		assert!(
			matches!(out.as_slice(), [b::play_to_client_packet::Ty::OpenWindow(w)] if w.inventory_type == new),
			"{out:?}"
		);
	}
	let jukebox =
		b::play_to_client_packet::Ty::OpenWindow(b::play_to_client_packet_open_window::Ty {
			window_id: 1,
			inventory_type: 8,
			window_title: bungeeoxide_nbt::Tag::new(bungeeoxide_nbt::Payload::from("x")),
		});
	assert!(down(&jukebox, &mut cx).is_empty());
}

fn spawn(entity: i32, kind: &str) -> a::play_to_client_packet::Ty {
	a::play_to_client_packet::Ty::SpawnEntity(a::play_to_client_packet_spawn_entity::Ty {
		entity_id: entity,
		object_uuid: entity as u128,
		r#type: id(OLD, "entities", kind),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		velocity: vec![0],
		pitch: 0,
		yaw: 0,
		head_pitch: 0,
		object_data: 0,
	})
}

#[test]
fn zombie_villager_metadata_skips_the_frostbite_conversion() {
	let mut cx = Session::new();
	let out = up(&spawn(3, "zombie_villager"), &mut cx);
	assert!(matches!(
		out.as_slice(),
		[b::play_to_client_packet::Ty::SpawnEntity(s)] if name(NEW, "entities", s.r#type) == "zombie_villager"
	));
	use a::entity_metadata_entry_value::Ty as V;
	let meta = a::play_to_client_packet::Ty::EntityMetadata(
		a::play_to_client_packet_entity_metadata::Ty {
			entity_id: 3,
			metadata: vec![
				a::entity_metadata_entry::Ty {
					key: 18,
					value: V::Boolean(true),
				},
				a::entity_metadata_entry::Ty {
					key: 19,
					value: V::Boolean(true),
				},
			],
		},
	);
	let out = up(&meta, &mut cx);
	let [b::play_to_client_packet::Ty::EntityMetadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		m.metadata.iter().map(|e| e.key).collect::<Vec<_>>(),
		[18, 20]
	);
	let mut cx = Session::new();
	let spawned = up(&spawn(3, "zombie_villager"), &mut Session::new());
	down(&spawned[0], &mut cx);
	use b::entity_metadata_entry_value::Ty as W;
	let back = down(
		&b::play_to_client_packet::Ty::EntityMetadata(
			b::play_to_client_packet_entity_metadata::Ty {
				entity_id: 3,
				metadata: vec![
					b::entity_metadata_entry::Ty {
						key: 19,
						value: W::Boolean(true),
					},
					b::entity_metadata_entry::Ty {
						key: 20,
						value: W::Boolean(false),
					},
				],
			},
		),
		&mut cx,
	);
	let [a::play_to_client_packet::Ty::EntityMetadata(m)] = back.as_slice() else {
		panic!("{back:?}")
	};
	assert_eq!(m.metadata.iter().map(|e| e.key).collect::<Vec<_>>(), [19]);
}

fn login_up(
	p: &a::login_to_client_packet::Ty,
	cx: &mut Session,
) -> Vec<b::login_to_client_packet::Ty> {
	run(
		Root::LoginToClient,
		OLD,
		NEW,
		p,
		a::login_to_client_packet::serialize,
		b::login_to_client_packet::deserialize,
		cx,
	)
}

fn login_down(
	p: &b::login_to_client_packet::Ty,
	cx: &mut Session,
) -> Vec<a::login_to_client_packet::Ty> {
	run(
		Root::LoginToClient,
		NEW,
		OLD,
		p,
		b::login_to_client_packet::serialize,
		a::login_to_client_packet::deserialize,
		cx,
	)
}

#[test]
fn compression_is_announced_in_the_login_options() {
	let mut cx = Session::new();
	let out = login_up(&a::login_to_client_packet::Ty::Compress(256), &mut cx);
	assert!(matches!(
		out.as_slice(),
		[b::login_to_client_packet::Ty::SetupCompression(256)]
	));
	let success = a::login_to_client_packet::Ty::Success(a::login_to_client_packet_success::Ty {
		uuid: 7,
		username: "snap".into(),
		properties: vec![],
		session_id: 9,
	});
	let out = login_up(&success, &mut cx);
	let [b::login_to_client_packet::Ty::Success(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert!(s.options.compression && !s.options.encryption);
	assert_eq!((s.username.as_str(), s.session_id), ("snap", 9));
	let out = login_up(&success, &mut Session::new());
	let [b::login_to_client_packet::Ty::Success(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert!(!s.options.compression);

	let mut cx = Session::new();
	assert!(login_down(
		&b::login_to_client_packet::Ty::SetupCompression(64),
		&mut cx
	)
	.is_empty());
	let back = login_down(&out[0].clone().with_compression(), &mut cx);
	let [a::login_to_client_packet::Ty::Success(s), a::login_to_client_packet::Ty::Compress(64)] =
		back.as_slice()
	else {
		panic!("{back:?}")
	};
	assert_eq!(s.username, "snap");
}

trait WithCompression {
	fn with_compression(self) -> Self;
}

impl WithCompression for b::login_to_client_packet::Ty {
	fn with_compression(mut self) -> Self {
		if let b::login_to_client_packet::Ty::Success(s) = &mut self {
			s.options.compression = true;
		}
		self
	}
}

#[test]
fn online_mode_logins_are_refused() {
	let key = b::login_to_client_packet::Ty::Key(b::login_to_client_packet_key::Ty {
		salt: 1,
		server_id: "server".into(),
		public_key: vec![1, 2, 3],
		should_authenticate: true,
	});
	assert!(convert(
		Root::LoginToClient,
		NEW,
		OLD,
		&encode(&key, b::login_to_client_packet::serialize),
		&mut Session::new()
	)
	.is_err());
	assert!(login_down(
		&b::login_to_client_packet::Ty::ApplyOptions(b::login_to_client_connection_options::Ty {
			encryption: true,
			compression: false,
		}),
		&mut Session::new()
	)
	.is_empty());
}

fn tags_of<'a>(
	v: &'a [a::configuration_to_client_packet_tags_items::Ty],
	registry: &str,
	tag: &str,
) -> Option<&'a Vec<i32>> {
	v.iter()
		.find(|r| r.tag_type == registry)?
		.tags
		.iter()
		.find(|t| t.tag_name == tag)
		.map(|t| &t.entries)
}

#[test]
fn fluid_tags_lose_and_regain_the_flowing_fluids() {
	let fluid_tags = |water: Vec<i32>, lava: Vec<i32>| {
		vec![a::configuration_to_client_packet_tags_items::Ty {
			tag_type: "minecraft:fluid".into(),
			tags: vec![
				a::configuration_to_client_packet_tags_items_tags_items::Ty {
					tag_name: "minecraft:water".into(),
					entries: water,
				},
				a::configuration_to_client_packet_tags_items_tags_items::Ty {
					tag_name: "minecraft:lava".into(),
					entries: lava,
				},
			],
		}]
	};
	let mut cx = Session::new();
	let out = run(
		Root::ConfigurationToClient,
		OLD,
		NEW,
		&a::configuration_to_client_packet::Ty::Tags(fluid_tags(vec![2, 1], vec![4, 3])),
		a::configuration_to_client_packet::serialize,
		b::configuration_to_client_packet::deserialize,
		&mut cx,
	);
	let [b::configuration_to_client_packet::Ty::Tags(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		tags_of(t, "minecraft:fluid", "minecraft:water"),
		Some(&vec![1])
	);
	assert_eq!(
		tags_of(t, "minecraft:fluid", "minecraft:lava"),
		Some(&vec![2])
	);
	assert_eq!(
		tags_of(t, "minecraft:fluid", "minecraft:swimmable"),
		Some(&vec![1])
	);
	let back = run(
		Root::ConfigurationToClient,
		NEW,
		OLD,
		&out[0],
		b::configuration_to_client_packet::serialize,
		a::configuration_to_client_packet::deserialize,
		&mut Session::new(),
	);
	let [a::configuration_to_client_packet::Ty::Tags(t)] = back.as_slice() else {
		panic!("{back:?}")
	};
	assert_eq!(
		tags_of(t, "minecraft:fluid", "minecraft:water"),
		Some(&vec![2, 1])
	);
	assert_eq!(
		tags_of(t, "minecraft:fluid", "minecraft:lava"),
		Some(&vec![4, 3])
	);
}

#[test]
fn new_registries_arrive_before_the_end_of_configuration() {
	let mut cx = Session::new();
	let out = run(
		Root::ConfigurationToClient,
		OLD,
		NEW,
		&a::configuration_to_client_packet::Ty::FinishConfiguration(
			a::configuration_to_client_packet_finish_configuration::Ty {},
		),
		a::configuration_to_client_packet::serialize,
		b::configuration_to_client_packet::deserialize,
		&mut cx,
	);
	let ids: Vec<&str> = out
		.iter()
		.filter_map(|p| match p {
			b::configuration_to_client_packet::Ty::RegistryData(r) => Some(r.id.as_str()),
			_ => None,
		})
		.collect();
	for registry in [
		"minecraft:block_sound_set",
		"minecraft:dispense_behavior",
		"minecraft:jukebox_song",
		"minecraft:sulfur_cube_archetype",
	] {
		assert!(ids.contains(&registry), "{registry} missing from {ids:?}");
	}
	assert!(matches!(
		out.last(),
		Some(b::configuration_to_client_packet::Ty::FinishConfiguration(
			_
		))
	));
	for p in &out {
		if let b::configuration_to_client_packet::Ty::RegistryData(r) = p {
			let back = run(
				Root::ConfigurationToClient,
				NEW,
				OLD,
				p,
				b::configuration_to_client_packet::serialize,
				a::configuration_to_client_packet::deserialize,
				&mut Session::new(),
			);
			let dropped = matches!(
				r.id.as_str(),
				"minecraft:block_sound_set" | "minecraft:dispense_behavior"
			);
			assert_eq!(back.is_empty(), dropped, "{}", r.id);
		}
	}
}

#[test]
fn a_second_brand_is_not_sent_to_26_4_servers() {
	let mut cx = Session::new();
	let brand = |channel: &str| a::play_to_server_packet_custom_payload::Ty {
		channel: channel.into(),
		data: b"\x07vanilla".to_vec(),
	};
	let config = |cx: &mut Session| {
		run(
			Root::ConfigurationToServer,
			OLD,
			NEW,
			&a::configuration_to_server_packet::Ty::CustomPayload(
				a::configuration_to_server_packet_custom_payload::Ty {
					channel: "minecraft:brand".into(),
					data: b"\x07vanilla".to_vec(),
				},
			),
			a::configuration_to_server_packet::serialize,
			b::configuration_to_server_packet::deserialize,
			cx,
		)
	};
	assert_eq!(config(&mut cx).len(), 1);
	assert!(config(&mut cx).is_empty());
	let play = |p: a::play_to_server_packet_custom_payload::Ty, cx: &mut Session| {
		run(
			Root::PlayToServer,
			OLD,
			NEW,
			&a::play_to_server_packet::Ty::CustomPayload(p),
			a::play_to_server_packet::serialize,
			b::play_to_server_packet::deserialize,
			cx,
		)
	};
	assert!(play(brand("minecraft:brand"), &mut cx).is_empty());
	assert_eq!(play(brand("example:other"), &mut cx).len(), 1);
	assert_eq!(play(brand("minecraft:brand"), &mut Session::new()).len(), 1);
}

#[test]
fn the_freezing_effect_and_potions_stay_on_26_4() {
	let mut cx = Session::new();
	let freezing = id(NEW, "effects", "Freezing");
	let speed = id(NEW, "effects", "Speed");
	let effect = |effect_id: i32| {
		b::play_to_client_packet::Ty::EntityEffect(b::play_to_client_packet_entity_effect::Ty {
			entity_id: 1,
			effect_id,
			amplifier: 0,
			duration: 100,
			flags: 0,
		})
	};
	assert!(down(&effect(freezing), &mut cx).is_empty());
	assert!(matches!(
		down(&effect(speed), &mut cx).as_slice(),
		[a::play_to_client_packet::Ty::EntityEffect(e)] if e.effect_id == id(OLD, "effects", "Speed")
	));
	let detail = b::item_effect_detail::Ty {
		amplifier: 0,
		duration: 20,
		ambient: false,
		show_particles: true,
		show_icon: true,
		hidden_effect: None,
	};
	let potion = cursor(
		"potion",
		vec![b::slot_component::Ty::PotionContents(
			b::slot_component_potion_contents::Ty {
				potion_id: Some(46),
				custom_color: None,
				custom_effects: vec![
					b::item_potion_effect::Ty {
						id: freezing,
						details: detail.clone(),
					},
					b::item_potion_effect::Ty {
						id: speed,
						details: detail,
					},
				],
				custom_name: None,
			},
		)],
	);
	let out = down(&potion, &mut cx);
	let [a::play_to_client_packet::Ty::SetCursorItem(a::slot::Ty::Default(_, d))] = out.as_slice()
	else {
		panic!("{out:?}")
	};
	let [a::slot_component::Ty::PotionContents(p)] = d.components.as_slice() else {
		panic!("{d:?}")
	};
	assert_eq!(p.potion_id, None);
	assert_eq!(
		p.custom_effects.iter().map(|e| e.id).collect::<Vec<_>>(),
		[id(OLD, "effects", "Speed")]
	);
}

fn registry_down(
	id: &str,
	entries: Vec<(String, Option<bungeeoxide_nbt::Tag>)>,
) -> Vec<(String, Option<bungeeoxide_nbt::Tag>)> {
	let packet = b::configuration_to_client_packet::Ty::RegistryData(
		b::configuration_to_client_packet_registry_data::Ty {
			id: id.into(),
			entries: entries
				.into_iter()
				.map(|(key, value)| {
					b::configuration_to_client_packet_registry_data_entries_items::Ty { key, value }
				})
				.collect(),
		},
	);
	let out = run(
		Root::ConfigurationToClient,
		NEW,
		OLD,
		&packet,
		b::configuration_to_client_packet::serialize,
		a::configuration_to_client_packet::deserialize,
		&mut Session::new(),
	);
	let [a::configuration_to_client_packet::Ty::RegistryData(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	r.entries
		.iter()
		.map(|e| (e.key.clone(), e.value.clone()))
		.collect()
}

#[test]
fn registry_data_loses_what_26_3_cannot_read() {
	use bungeeoxide_nbt::{Entry, Payload, Tag};
	let attributes = Payload::Compound(vec![
		Entry::new("minecraft:visual/has_sky_occluder", Payload::Byte(1)),
		Entry::new("minecraft:visual/sky_color", Payload::from("#78a7ff")),
	]);
	let custom = Tag::new(Payload::Compound(vec![Entry::new(
		"attributes",
		attributes,
	)]));
	let out = registry_down(
		"minecraft:dimension_type",
		vec![("example:custom".into(), Some(custom))],
	);
	let value = out[0].1.as_ref().unwrap();
	let attributes = value.value.field("attributes").unwrap();
	assert!(attributes
		.field("minecraft:visual/has_sky_occluder")
		.is_none());
	assert!(attributes.field("minecraft:visual/sky_color").is_some());
	let flurry = Tag::new(Payload::Compound(vec![Entry::new(
		"sound_event",
		Payload::from("minecraft:music_disc.flurry"),
	)]));
	let out = registry_down(
		"minecraft:jukebox_song",
		vec![("minecraft:flurry".into(), Some(flurry))],
	);
	assert_eq!(out[0].0, "minecraft:flurry");
	let sound = out[0]
		.1
		.as_ref()
		.unwrap()
		.value
		.field("sound_event")
		.cloned();
	assert_ne!(sound, Some(Payload::from("minecraft:music_disc.flurry")));
}

#[test]
fn tag_updates_in_play_keep_the_derived_tags() {
	let mut cx = Session::new();
	let update =
		a::play_to_client_packet::Ty::Tags(vec![a::play_to_client_packet_tags_items::Ty {
			tag_type: "minecraft:fluid".into(),
			tags: vec![
				a::configuration_to_client_packet_tags_items_tags_items::Ty {
					tag_name: "minecraft:water".into(),
					entries: vec![2, 1],
				},
			],
		}]);
	let out = up(&update, &mut cx);
	let [b::play_to_client_packet::Ty::Tags(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let fluid = t.iter().find(|r| r.tag_type == "minecraft:fluid").unwrap();
	let swimmable = fluid
		.tags
		.iter()
		.find(|t| t.tag_name == "minecraft:swimmable")
		.unwrap();
	assert_eq!(swimmable.entries, vec![1]);
}
