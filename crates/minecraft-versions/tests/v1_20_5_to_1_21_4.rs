use minecraft_versions::gen::convert::{
	convert, index_of, inspect, v1_20_5 as a, v1_21_4 as d, Root,
};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::registry::{blocks, names};
use minecraft_versions::Session;

fn at(version: &str) -> usize {
	index_of(version).unwrap()
}

fn run<A, B>(
	root: Root,
	(from, to): (&str, &str),
	packet: &A,
	ser: fn(&A, &mut Output) -> minecraft_versions::pds::Result<()>,
	de: fn(&mut Input) -> minecraft_versions::pds::Result<B>,
	cx: &mut Session,
) -> Vec<B> {
	let mut o = Output::new();
	ser(packet, &mut o).unwrap();
	convert(root, at(from), at(to), &o.into_inner(), cx)
		.unwrap()
		.iter()
		.map(|bytes| {
			inspect(root, at(to), bytes).unwrap();
			let mut input = Input::new(bytes);
			let p = de(&mut input).unwrap();
			assert_eq!(input.remaining(), 0);
			p
		})
		.collect()
}

fn up(p: &a::play_toClient_packet::Ty, cx: &mut Session) -> Vec<d::play_toClient_packet::Ty> {
	run(
		Root::PlayToClient,
		("1.20.5", "1.21.4"),
		p,
		a::play_toClient_packet::serialize,
		d::play_toClient_packet::deserialize,
		cx,
	)
}

fn down(p: &d::play_toServer_packet::Ty, cx: &mut Session) -> Vec<a::play_toServer_packet::Ty> {
	run(
		Root::PlayToServer,
		("1.21.4", "1.20.5"),
		p,
		d::play_toServer_packet::serialize,
		a::play_toServer_packet::deserialize,
		cx,
	)
}

fn config(
	p: &a::configuration_toClient_packet::Ty,
	cx: &mut Session,
) -> Vec<d::configuration_toClient_packet::Ty> {
	run(
		Root::ConfigurationToClient,
		("1.20.5", "1.21.4"),
		p,
		a::configuration_toClient_packet::serialize,
		d::configuration_toClient_packet::deserialize,
		cx,
	)
}

fn id(version: &str, kind: &str, name: &str) -> i32 {
	let table = names(version, kind).unwrap();
	let base = u32::from(kind == "sounds" && table.name(0).is_none());
	(table.id(name).unwrap() - base) as i32
}

fn state(version: &str, text: &str) -> i32 {
	blocks(version).unwrap().parse_state(text).unwrap() as i32
}

fn describe(version: &str, state: i32) -> String {
	blocks(version).unwrap().describe(state as u32).unwrap()
}

fn item_a(name: &str, components: Vec<a::SlotComponent::Ty>) -> a::Slot::Ty {
	a::Slot::Ty::default(
		1,
		a::Slot__default::Ty {
			itemId: id("1.20.5", "items", name),
			addedComponentCount: components.len() as i32,
			removedComponentCount: 0,
			components,
			removeComponents: vec![],
		},
	)
}

fn tag(entries: Vec<nbt::gen::pds_Entry::Ty>) -> nbt::gen::pds_Tag::Ty {
	nbt::Tag::new(entries)
}

#[test]
fn configuration_adds_required_registries() {
	use a::configuration_toClient_packet::Ty as P;
	let mut cx = Session::new();
	let damage = a::configuration_toClient_packet_registry_data::Ty {
		id: "minecraft:damage_type".to_owned(),
		entries: vec![
			a::configuration_toClient_packet_registry_data__entries__items::Ty {
				key: "minecraft:arrow".to_owned(),
				value: Some(tag(vec![nbt::gen::pds_Entry::Ty::new(
					"message_id",
					nbt::gen::pds_Payload::Ty::string("arrow".to_owned()),
				)])),
			},
		],
	};
	let mut out = config(&P::registry_data(damage), &mut cx);
	out.extend(config(
		&P::feature_flags(vec!["minecraft:vanilla".to_owned()]),
		&mut cx,
	));
	out.extend(config(
		&P::finish_configuration(a::configuration_toClient_packet_finish_configuration::Ty {}),
		&mut cx,
	));
	use d::configuration_toClient_packet::Ty as Q;
	assert!(matches!(out.last(), Some(Q::finish_configuration(_))));
	let registries: Vec<&d::configuration_toClient_packet_registry_data::Ty> = out
		.iter()
		.filter_map(|p| match p {
			Q::registry_data(r) => Some(r),
			_ => None,
		})
		.collect();
	let find = |name: &str| {
		registries
			.iter()
			.find(|r| r.id == name)
			.unwrap_or_else(|| panic!("{name} missing"))
	};
	for required in [
		"minecraft:painting_variant",
		"minecraft:enchantment",
		"minecraft:jukebox_song",
		"minecraft:instrument",
		"minecraft:wolf_variant",
		"minecraft:dimension_type",
		"minecraft:worldgen/biome",
	] {
		assert!(!find(required).entries.is_empty(), "{required} is empty");
	}
	let damage = find("minecraft:damage_type");
	assert_eq!(damage.entries[0].key, "minecraft:arrow");
	assert!(
		matches!(&damage.entries[0].value, Some(t) if matches!(&t.value, nbt::gen::pds_Payload::Ty::compound(e) if e.len() == 1))
	);
	for key in [
		"minecraft:campfire",
		"minecraft:ender_pearl",
		"minecraft:mace_smash",
	] {
		assert!(damage.entries.iter().any(|e| e.key == key), "{key} missing");
	}
	let enchantments = find("minecraft:enchantment");
	assert_eq!(enchantments.entries.len(), 42);
	assert_eq!(
		enchantments.entries[id("1.21.4", "enchantments", "sharpness") as usize].key,
		"minecraft:sharpness"
	);
	let badlands = find("minecraft:worldgen/biome")
		.entries
		.iter()
		.find(|e| e.key == "minecraft:badlands")
		.unwrap();
	let nbt::gen::pds_Payload::Ty::compound(biome) = &badlands.value.as_ref().unwrap().value else {
		panic!()
	};
	let effects = biome.iter().find(|e| e.name == "effects").unwrap();
	let nbt::gen::pds_Payload::Ty::compound(effects) = &effects.value else {
		panic!()
	};
	let music = effects.iter().find(|e| e.name == "music").unwrap();
	assert_eq!(music.r#type, 9, "1.21.4 music is a weighted list");
	let again = config(
		&P::finish_configuration(a::configuration_toClient_packet_finish_configuration::Ty {}),
		&mut cx,
	);
	assert_eq!(
		again
			.iter()
			.filter(|p| matches!(p, Q::registry_data(_)))
			.count(),
		registries.len(),
		"reconfiguration resends them"
	);
}

#[test]
fn tags_get_remapped_and_completed() {
	use a::configuration_toClient_packet::Ty as P;
	use d::configuration_toClient_packet::Ty as Q;
	let out = config(
		&P::tags(vec![a::configuration_toClient_packet__tags__items::Ty {
			tagType: "minecraft:block".to_owned(),
			tags: vec![
				a::configuration_toClient_packet__tags__items__tags__items::Ty {
					tagName: "minecraft:logs".to_owned(),
					entries: vec![
						id("1.20.5", "blocks", "oak_log"),
						id("1.20.5", "blocks", "cherry_log"),
					],
				},
			],
		}]),
		&mut Session::new(),
	);
	let [Q::tags(groups)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let blocks = groups
		.iter()
		.find(|g| g.tagType == "minecraft:block")
		.unwrap();
	assert_eq!(
		blocks.tags[0].entries,
		vec![
			id("1.21.4", "blocks", "oak_log"),
			id("1.21.4", "blocks", "cherry_log")
		]
	);
	let enchantments = groups
		.iter()
		.find(|g| g.tagType == "minecraft:enchantment")
		.unwrap();
	let curse = enchantments
		.tags
		.iter()
		.find(|t| t.tagName == "minecraft:curse")
		.unwrap();
	assert_eq!(
		curse.entries,
		vec![
			id("1.21.4", "enchantments", "binding_curse"),
			id("1.21.4", "enchantments", "vanishing_curse")
		]
	);
	assert!(enchantments
		.tags
		.iter()
		.any(|t| t.tagName == "minecraft:exclusive_set/armor"));
}

#[test]
fn positions_and_teleports() {
	use a::play_toClient_packet::Ty as P;
	use d::play_toClient_packet::Ty as Q;
	let mut cx = Session::new();
	let out = up(
		&P::position(a::play_toClient_packet_position::Ty {
			x: 1.5,
			y: 64.0,
			z: -3.0,
			yaw: 90.0,
			pitch: 10.0,
			flags: a::play_toClient_PositionUpdateRelatives::Ty {
				x: false,
				y: true,
				z: false,
				yaw: true,
				pitch: true,
			},
			teleportId: 7,
		}),
		&mut cx,
	);
	let [Q::position(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(p.teleportId, p.x, p.y, p.z, p.yaw, p.pitch),
		(7, 1.5, 64.0, -3.0, 90.0, 10.0)
	);
	assert_eq!((p.dx, p.dy, p.dz), (0.0, 0.0, 0.0));
	let f = &p.flags;
	assert_eq!(
		(f.x, f.y, f.z, f.yaw, f.pitch, f.dx, f.dy, f.dz, f.yawDelta),
		(false, true, false, true, true, false, true, false, false)
	);
	let out = up(
		&P::entity_teleport(a::play_toClient_packet_entity_teleport::Ty {
			entityId: 5,
			x: 1.0,
			y: 2.0,
			z: 3.0,
			yaw: 64,
			pitch: -32,
			onGround: true,
		}),
		&mut cx,
	);
	let [Q::sync_entity_position(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(t.entityId, t.x, t.y, t.z, t.yaw, t.pitch, t.onGround),
		(5, 1.0, 2.0, 3.0, 90.0, -45.0, true)
	);
}

#[test]
fn items_and_components() {
	use a::play_toClient_packet::Ty as P;
	use d::play_toClient_packet::Ty as Q;
	let mut cx = Session::new();
	let enchant =
		|name: &str, level: i32| a::SlotComponent__enchantments__enchantments__items::Ty {
			id: id("1.20.5", "enchantments", name),
			level,
		};
	let sword = item_a(
		"diamond_sword",
		vec![a::SlotComponent::Ty::enchantments(
			a::SlotComponent__enchantments::Ty {
				enchantments: vec![enchant("sharpness", 5), enchant("unbreaking", 3)],
				showTooltip: true,
			},
		)],
	);
	let glowing = item_a(
		"stone",
		vec![a::SlotComponent::Ty::enchantments(
			a::SlotComponent__enchantments::Ty {
				enchantments: vec![enchant("protection", 0)],
				showTooltip: false,
			},
		)],
	);
	let fire = item_a(
		"netherite_ingot",
		vec![a::SlotComponent::Ty::fire_resistant],
	);
	let out = up(
		&P::window_items(a::play_toClient_packet_window_items::Ty {
			windowId: 0,
			stateId: 1,
			items: vec![sword, a::Slot::Ty::v0, glowing, fire],
			carriedItem: a::Slot::Ty::v0,
		}),
		&mut cx,
	);
	let [Q::window_items(w)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(w.items.len(), 4);
	let d::Slot::Ty::default(1, s) = &w.items[0] else {
		panic!("{:?}", w.items[0])
	};
	assert_eq!(s.itemId, id("1.21.4", "items", "diamond_sword"));
	let [d::SlotComponent::Ty::enchantments(e)] = s.components.as_slice() else {
		panic!("{s:?}")
	};
	let ids: Vec<(i32, i32)> = e.enchantments.iter().map(|e| (e.id, e.level)).collect();
	assert_eq!(
		ids,
		vec![
			(id("1.21.4", "enchantments", "sharpness"), 5),
			(id("1.21.4", "enchantments", "unbreaking"), 3)
		]
	);
	assert!(matches!(&w.items[1], d::Slot::Ty::v0));
	let d::Slot::Ty::default(_, s) = &w.items[2] else {
		panic!()
	};
	assert!(
		matches!(
			s.components.as_slice(),
			[d::SlotComponent::Ty::enchantment_glint_override(true)]
		),
		"{s:?}"
	);
	let d::Slot::Ty::default(_, s) = &w.items[3] else {
		panic!()
	};
	assert!(
		matches!(s.components.as_slice(), [d::SlotComponent::Ty::damage_resistant(t)] if t == "minecraft:is_fire"),
		"{s:?}"
	);
	assert_eq!(s.addedComponentCount, 1);

	let out = up(
		&P::set_slot(a::play_toClient_packet_set_slot::Ty {
			windowId: 255,
			stateId: 0,
			slot: -1,
			item: item_a("apple", vec![]),
		}),
		&mut cx,
	);
	let [Q::set_cursor_item(d::Slot::Ty::default(1, s))] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.itemId, id("1.21.4", "items", "apple"));
}

#[test]
fn creative_items_go_down() {
	use a::play_toServer_packet::Ty as P;
	use d::play_toServer_packet::Ty as Q;
	let mut cx = Session::new();
	let slot = |name: &str| {
		d::Slot::Ty::default(
			3,
			d::Slot__default::Ty {
				itemId: id("1.21.4", "items", name),
				addedComponentCount: 2,
				removedComponentCount: 0,
				components: vec![
					d::SlotComponent::Ty::max_stack_size(16),
					d::SlotComponent::Ty::glider,
				],
				removeComponents: vec![],
			},
		)
	};
	for (new, old) in [
		("pale_oak_planks", "birch_planks"),
		("diamond", "diamond"),
		("resin_brick", "copper_ingot"),
		("music_disc_creator", "music_disc_far"),
	] {
		let out = down(
			&Q::set_creative_slot(d::play_toServer_packet_set_creative_slot::Ty {
				slot: 36,
				item: slot(new),
			}),
			&mut cx,
		);
		let [P::set_creative_slot(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let a::Slot::Ty::default(3, s) = &p.item else {
			panic!("{p:?}")
		};
		assert_eq!(s.itemId, id("1.20.5", "items", old), "{new}");
		assert_eq!(s.addedComponentCount, 1);
		assert!(matches!(
			s.components.as_slice(),
			[a::SlotComponent::Ty::max_stack_size(16)]
		));
	}
}

#[test]
fn update_attributes_get_modifier_ids() {
	use a::play_toClient_packet::Ty as P;
	use d::play_toClient_packet::Ty as Q;
	let out = up(
		&P::entity_update_attributes(a::play_toClient_packet_entity_update_attributes::Ty {
			entityId: 1,
			properties: vec![a::play_toClient_packet_entity_update_attributes__properties__items::Ty {
				key: a::play_toClient_packet_entity_update_attributes__properties__items__key::Ty::generic_movement_speed,
				value: 0.1,
				modifiers: vec![a::play_toClient_packet_entity_update_attributes__properties__items__modifiers__items::Ty {
					uuid: 0x662a6b8dda3e4c1c_881396ea6097278d,
					amount: 0.3,
					operation: 2,
				}],
			}],
		}),
		&mut Session::new(),
	);
	let [Q::entity_update_attributes(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(p.properties[0].modifiers[0].uuid, "minecraft:sprinting");
	assert_eq!(p.properties[0].value, 0.1);
}

fn section(states: &[i32]) -> Vec<u8> {
	let mut out = vec![0x10, 0x00, 8];
	out.push(states.len() as u8);
	for s in states {
		let mut v = *s as u32;
		loop {
			if v & !0x7f == 0 {
				out.push(v as u8);
				break;
			}
			out.push((v as u8 & 0x7f) | 0x80);
			v >>= 7;
		}
	}
	let longs = 4096 / 8;
	out.extend([0x80, 0x04]);
	for i in 0..longs {
		let mut l = 0u64;
		for j in 0..8 {
			l |= (((i * 8 + j) % states.len()) as u64) << (j * 8);
		}
		out.extend(l.to_be_bytes());
	}
	out.extend([0, 3, 0]);
	out
}

#[test]
fn chunks_keep_block_states() {
	use a::play_toClient_packet::Ty as P;
	use d::play_toClient_packet::Ty as Q;
	let names = [
		"minecraft:stone",
		"minecraft:oak_stairs[facing=east,half=top,shape=straight,waterlogged=true]",
		"minecraft:grass_block[snowy=false]",
		"minecraft:chest[facing=west,type=single,waterlogged=false]",
	];
	let old: Vec<i32> = names.iter().map(|n| state("1.20.5", n)).collect();
	let mut data = section(&old);
	data.extend(section(&old[..2]));
	let out = up(
		&P::map_chunk(a::play_toClient_packet_map_chunk::Ty {
			x: 1,
			z: 2,
			heightmaps: tag(vec![]),
			chunkData: data,
			blockEntities: vec![a::chunkBlockEntity::Ty {
				ANON: a::chunkBlockEntity__ANON::Ty { x: 1, z: 2 },
				y: 64,
				r#type: 1,
				nbtData: None,
			}],
			skyLightMask: vec![],
			blockLightMask: vec![],
			emptySkyLightMask: vec![],
			emptyBlockLightMask: vec![],
			skyLight: vec![],
			blockLight: vec![],
		}),
		&mut Session::new(),
	);
	let [Q::map_chunk(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(c.blockEntities[0].r#type, 1);
	let d = &c.chunkData;
	let mut at = 3;
	let n = d[at] as usize;
	at += 1;
	let palette: Vec<String> = (0..n)
		.map(|_| {
			let mut v = 0u32;
			for shift in (0..35).step_by(7) {
				let b = d[at];
				at += 1;
				v |= ((b & 0x7f) as u32) << shift;
				if b & 0x80 == 0 {
					break;
				}
			}
			describe("1.21.4", v as i32)
		})
		.collect();
	assert_eq!(palette, names.to_vec());
}

#[test]
fn spawns_and_metadata() {
	use a::play_toClient_packet::Ty as P;
	use d::play_toClient_packet::Ty as Q;
	let mut cx = Session::new();
	let spawn = |entity: i32, ty: &str, data: i32| {
		P::spawn_entity(a::play_toClient_packet_spawn_entity::Ty {
			entityId: entity,
			objectUUID: entity as u128,
			r#type: id("1.20.5", "entities", ty),
			x: 0.0,
			y: 70.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			headPitch: 0,
			objectData: data,
			velocity: a::vec3i16::Ty { x: 0, y: 0, z: 0 },
		})
	};
	let out = up(&spawn(10, "boat", 0), &mut cx);
	let [Q::spawn_entity(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.r#type, id("1.21.4", "entities", "oak_boat"));
	let sand = state("1.20.5", "minecraft:sand");
	let out = up(&spawn(11, "falling_block", sand), &mut cx);
	let [Q::spawn_entity(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(describe("1.21.4", s.objectData), "minecraft:sand");
	let entry = |key: u8, value: a::entityMetadataEntry__value::Ty| a::entityMetadataEntry::Ty {
		key,
		value,
	};
	let out = up(
		&P::entity_metadata(a::play_toClient_packet_entity_metadata::Ty {
			entityId: 10,
			metadata: vec![
				entry(0, a::entityMetadataEntry__value::Ty::byte(0)),
				entry(11, a::entityMetadataEntry__value::Ty::int(0)),
				entry(12, a::entityMetadataEntry__value::Ty::boolean(true)),
			],
		}),
		&mut cx,
	);
	let [Q::entity_metadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let keys: Vec<u8> = m.metadata.iter().map(|e| e.key).collect();
	assert_eq!(keys, vec![0, 11]);
	assert!(matches!(
		m.metadata[1].value,
		d::entityMetadataEntry__value::Ty::boolean(true)
	));
	let out = up(
		&P::spawn_entity(a::play_toClient_packet_spawn_entity::Ty {
			entityId: 12,
			objectUUID: 12,
			r#type: id("1.20.5", "entities", "painting"),
			x: 0.0,
			y: 0.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			headPitch: 0,
			objectData: 2,
			velocity: a::vec3i16::Ty { x: 0, y: 0, z: 0 },
		}),
		&mut cx,
	);
	assert_eq!(out.len(), 1);
	let out = up(
		&P::entity_metadata(a::play_toClient_packet_entity_metadata::Ty {
			entityId: 12,
			metadata: vec![entry(
				8,
				a::entityMetadataEntry__value::Ty::painting_variant(0),
			)],
		}),
		&mut cx,
	);
	let [Q::entity_metadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let d::entityMetadataEntry__value::Ty::painting_variant(
		d::entityMetadataEntry__value__painting_variant::Ty::Id(kebab),
	) = &m.metadata[0].value
	else {
		panic!("{m:?}")
	};
	assert_eq!(
		*kebab, 23,
		"kebab is the 24th painting in the vanilla 1.21 registry"
	);
}

#[test]
fn particles_and_sounds() {
	use a::play_toClient_packet::Ty as P;
	use d::play_toClient_packet::Ty as Q;
	let mut cx = Session::new();
	let particle = |particle| {
		P::world_particles(a::play_toClient_packet_world_particles::Ty {
			longDistance: false,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			offsetX: 0.0,
			offsetY: 0.0,
			offsetZ: 0.0,
			velocityOffset: 0.0,
			amount: 1,
			particle,
		})
	};
	let out = up(
		&particle(a::Particle::Ty::dust(a::Particle__dust::Ty {
			red: 1.0,
			green: 0.5,
			blue: 0.0,
			scale: 1.0,
		})),
		&mut cx,
	);
	let [Q::world_particles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert!(
		matches!(&p.particle, d::Particle::Ty::dust(d) if d.color == (0xffu32 << 24 | 0xff << 16 | 127 << 8) as i32),
		"{p:?}"
	);
	let out = up(
		&particle(a::Particle::Ty::block(state(
			"1.20.5",
			"minecraft:oak_log[axis=x]",
		))),
		&mut cx,
	);
	let [Q::world_particles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let d::Particle::Ty::block(s) = p.particle else {
		panic!()
	};
	assert_eq!(describe("1.21.4", s), "minecraft:oak_log[axis=x]");
	let sound = "entity.player.levelup";
	let out = up(
		&P::sound_effect(a::play_toClient_packet_sound_effect::Ty {
			sound: a::ItemSoundHolder::Ty::Id(id("1.20.5", "sounds", sound)),
			soundCategory: a::soundSource::Ty::master,
			x: 0,
			y: 0,
			z: 0,
			volume: 1.0,
			pitch: 1.0,
			seed: 0,
		}),
		&mut cx,
	);
	let [Q::sound_effect(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		s.sound,
		d::ItemSoundHolder::Ty::Id(id("1.21.4", "sounds", sound))
	);
}

fn sample_packets() -> Vec<a::play_toClient_packet::Ty> {
	use a::play_toClient_packet::Ty as P;
	let enchant =
		|name: &str, level: i32| a::SlotComponent__enchantments__enchantments__items::Ty {
			id: id("1.20.5", "enchantments", name),
			level,
		};
	let sword = item_a(
		"diamond_sword",
		vec![
			a::SlotComponent::Ty::enchantments(a::SlotComponent__enchantments::Ty {
				enchantments: vec![enchant("sharpness", 2)],
				showTooltip: true,
			}),
			a::SlotComponent::Ty::attribute_modifiers(a::SlotComponent__attribute_modifiers::Ty {
				attributes: vec![a::SlotComponent__attribute_modifiers__attributes__items::Ty {
					typeId: 2,
					uuid: 0xcb3f55d3645c4f38_a4979c13a33db5cf,
					name: "Weapon modifier".to_owned(),
					value: 7.0,
					operation: a::SlotComponent__attribute_modifiers__attributes__items__operation::Ty::add,
					slot: a::SlotComponent__attribute_modifiers__attributes__items__slot::Ty::main_hand,
				}],
				showTooltip: true,
			}),
			a::SlotComponent::Ty::custom_model_data(3),
			a::SlotComponent::Ty::fire_resistant,
		],
	);
	let pos = |x, y, z| a::position::Ty { x, y, z };
	let stone = state("1.20.5", "minecraft:stone");
	let mut chunk = Vec::new();
	for s in 0..24 {
		let states = match s {
			0..=3 => vec![state("1.20.5", "minecraft:bedrock"), stone],
			4 => vec![
				stone,
				state("1.20.5", "minecraft:grass_block[snowy=false]"),
				state("1.20.5", "minecraft:oak_log[axis=y]"),
			],
			_ => vec![0],
		};
		if states.len() == 1 {
			chunk.extend([0, 0, 0, 0, 0, 0, 1, 0]);
		} else {
			chunk.extend(section(&states));
		}
	}
	let spawn = |entity: i32, ty: &str, data: i32| {
		P::spawn_entity(a::play_toClient_packet_spawn_entity::Ty {
			entityId: entity,
			objectUUID: entity as u128,
			r#type: id("1.20.5", "entities", ty),
			x: 0.5,
			y: 70.0,
			z: 0.5,
			pitch: 0,
			yaw: 64,
			headPitch: 0,
			objectData: data,
			velocity: a::vec3i16::Ty { x: 0, y: 0, z: 0 },
		})
	};
	let entry = |key: u8, value: a::entityMetadataEntry__value::Ty| a::entityMetadataEntry::Ty {
		key,
		value,
	};
	let particles = |particle| {
		P::world_particles(a::play_toClient_packet_world_particles::Ty {
			longDistance: false,
			x: 0.0,
			y: 70.0,
			z: 0.0,
			offsetX: 0.1,
			offsetY: 0.1,
			offsetZ: 0.1,
			velocityOffset: 0.0,
			amount: 3,
			particle,
		})
	};
	let sound = a::ItemSoundHolder::Ty::Id(id("1.20.5", "sounds", "entity.generic.explode"));
	let small = || a::Particle::Ty::explosion;
	vec![
		P::position(a::play_toClient_packet_position::Ty {
			x: 0.5,
			y: 80.0,
			z: 0.5,
			yaw: 0.0,
			pitch: 0.0,
			flags: a::play_toClient_PositionUpdateRelatives::Ty { x: false, y: false, z: false, yaw: false, pitch: false },
			teleportId: 1,
		}),
		P::map_chunk(a::play_toClient_packet_map_chunk::Ty {
			x: 0,
			z: 0,
			heightmaps: tag(vec![]),
			chunkData: chunk,
			blockEntities: vec![a::chunkBlockEntity::Ty { ANON: a::chunkBlockEntity__ANON::Ty { x: 1, z: 1 }, y: 0, r#type: 7, nbtData: None }],
			skyLightMask: vec![],
			blockLightMask: vec![],
			emptySkyLightMask: vec![],
			emptyBlockLightMask: vec![],
			skyLight: vec![],
			blockLight: vec![],
		}),
		P::block_change(a::play_toClient_packet_block_change::Ty { location: pos(1, 70, 1), r#type: state("1.20.5", "minecraft:oak_stairs[facing=east,half=top,shape=straight,waterlogged=true]") }),
		P::multi_block_change(a::play_toClient_packet_multi_block_change::Ty {
			chunkCoordinates: a::play_toClient_packet_multi_block_change__chunkCoordinates::Ty { x: 0, z: 0, y: 4 },
			records: vec![(stone << 12) | 0x123],
		}),
		spawn(10, "boat", 0),
		spawn(11, "falling_block", state("1.20.5", "minecraft:sand")),
		spawn(12, "zombie", 0),
		P::entity_metadata(a::play_toClient_packet_entity_metadata::Ty {
			entityId: 10,
			metadata: vec![entry(11, a::entityMetadataEntry__value::Ty::int(0)), entry(12, a::entityMetadataEntry__value::Ty::boolean(false))],
		}),
		P::entity_metadata(a::play_toClient_packet_entity_metadata::Ty {
			entityId: 12,
			metadata: vec![entry(0, a::entityMetadataEntry__value::Ty::byte(0)), entry(9, a::entityMetadataEntry__value::Ty::float(20.0))],
		}),
		P::entity_equipment(a::play_toClient_packet_entity_equipment::Ty {
			entityId: 12,
			equipments: vec![a::play_toClient_packet_entity_equipment__equipments__items::Ty { slot: 0, item: sword.clone() }],
		}),
		P::window_items(a::play_toClient_packet_window_items::Ty {
			windowId: 0,
			stateId: 1,
			items: vec![sword.clone(), a::Slot::Ty::v0, item_a("apple", vec![])],
			carriedItem: a::Slot::Ty::v0,
		}),
		P::set_slot(a::play_toClient_packet_set_slot::Ty { windowId: 255, stateId: 2, slot: -1, item: item_a("stone", vec![]) }),
		particles(a::Particle::Ty::dust(a::Particle__dust::Ty { red: 1.0, green: 0.0, blue: 0.0, scale: 1.0 })),
		particles(a::Particle::Ty::block(stone)),
		particles(a::Particle::Ty::item(sword)),
		P::sound_effect(a::play_toClient_packet_sound_effect::Ty { sound: sound.clone(), soundCategory: a::soundSource::Ty::block, x: 0, y: 560, z: 0, volume: 1.0, pitch: 1.0, seed: 1 }),
		P::explosion(a::play_toClient_packet_explosion::Ty {
			x: 0.0,
			y: 70.0,
			z: 0.0,
			radius: 4.0,
			affectedBlockOffsets: vec![a::play_toClient_packet_explosion__affectedBlockOffsets__items::Ty { x: 1, y: 0, z: -1 }],
			playerMotionX: 0.1,
			playerMotionY: 0.2,
			playerMotionZ: 0.0,
			block_interaction_type: 1,
			small_explosion_particle: small(),
			large_explosion_particle: a::Particle::Ty::explosion_emitter,
			sound,
		}),
		P::update_time(a::play_toClient_packet_update_time::Ty { age: 100, time: -6000 }),
		P::entity_update_attributes(a::play_toClient_packet_entity_update_attributes::Ty {
			entityId: 12,
			properties: vec![a::play_toClient_packet_entity_update_attributes__properties__items::Ty {
				key: a::play_toClient_packet_entity_update_attributes__properties__items__key::Ty::generic_max_health,
				value: 20.0,
				modifiers: vec![],
			}],
		}),
		P::set_cooldown(a::play_toClient_packet_set_cooldown::Ty { itemID: id("1.20.5", "items", "ender_pearl"), cooldownTicks: 20 }),
		P::entity_teleport(a::play_toClient_packet_entity_teleport::Ty { entityId: 12, x: 1.0, y: 70.0, z: 1.0, yaw: 32, pitch: 0, onGround: true }),
		P::world_event(a::play_toClient_packet_world_event::Ty { effectId: 2001, location: pos(0, 70, 0), data: stone, global: false }),
		P::world_event(a::play_toClient_packet_world_event::Ty { effectId: 1010, location: pos(0, 70, 0), data: id("1.20.5", "items", "music_disc_cat"), global: false }),
		P::tile_entity_data(a::play_toClient_packet_tile_entity_data::Ty { location: pos(1, 0, 1), action: 7, nbtData: Some(tag(vec![])) }),
		P::set_passengers(a::play_toClient_packet_set_passengers::Ty { entityId: 10, passengers: vec![12] }),
		P::entity_destroy(vec![10, 11]),
	]
}

#[test]
#[ignore]
fn write_dumps() {
	let dir = std::env::var("TRICKY_DUMPS").expect("TRICKY_DUMPS names an output directory");
	let packets = sample_packets();
	for to in ["1.20.5", "1.21.1", "1.21.3", "1.21.4"] {
		let mut cx = Session::new();
		let mut out = Vec::new();
		for p in &packets {
			let mut o = Output::new();
			a::play_toClient_packet::serialize(p, &mut o).unwrap();
			for bytes in convert(
				Root::PlayToClient,
				at("1.20.5"),
				at(to),
				&o.into_inner(),
				&mut cx,
			)
			.unwrap()
			{
				out.extend((bytes.len() as u32).to_be_bytes());
				out.extend(bytes);
			}
		}
		std::fs::write(format!("{dir}/play-{to}.bin"), out).unwrap();
	}
	let newest = std::fs::read(format!("{dir}/play-1.21.4.bin")).unwrap();
	let mut cx = Session::new();
	let (mut out, mut at_byte) = (Vec::new(), 0);
	while at_byte < newest.len() {
		let len = u32::from_be_bytes(newest[at_byte..at_byte + 4].try_into().unwrap()) as usize;
		let bytes = &newest[at_byte + 4..at_byte + 4 + len];
		at_byte += 4 + len;
		for bytes in convert(
			Root::PlayToClient,
			at("1.21.4"),
			at("1.20.5"),
			bytes,
			&mut cx,
		)
		.unwrap()
		{
			out.extend((bytes.len() as u32).to_be_bytes());
			out.extend(bytes);
		}
	}
	std::fs::write(format!("{dir}/play-down-1.20.5.bin"), out).unwrap();
	use a::configuration_toClient_packet::Ty as C;
	let configuration = [
		C::feature_flags(vec!["minecraft:vanilla".to_owned()]),
		C::tags(vec![a::configuration_toClient_packet__tags__items::Ty {
			tagType: "minecraft:item".to_owned(),
			tags: vec![
				a::configuration_toClient_packet__tags__items__tags__items::Ty {
					tagName: "minecraft:logs".to_owned(),
					entries: vec![id("1.20.5", "items", "oak_log")],
				},
			],
		}]),
		C::finish_configuration(a::configuration_toClient_packet_finish_configuration::Ty {}),
	];
	for to in ["1.21.1", "1.21.3", "1.21.4"] {
		let mut cx = Session::new();
		let mut out = Vec::new();
		for p in &configuration {
			let mut o = Output::new();
			a::configuration_toClient_packet::serialize(p, &mut o).unwrap();
			for bytes in convert(
				Root::ConfigurationToClient,
				at("1.20.5"),
				at(to),
				&o.into_inner(),
				&mut cx,
			)
			.unwrap()
			{
				out.extend((bytes.len() as u32).to_be_bytes());
				out.extend(bytes);
			}
		}
		std::fs::write(format!("{dir}/configuration-{to}.bin"), out).unwrap();
	}
	use d::play_toServer_packet::Ty as S;
	let flags = |on_ground| d::play_toServer_MovementFlags::Ty {
		onGround: on_ground,
		hasHorizontalCollision: false,
	};
	let slot = d::Slot::Ty::default(
		1,
		d::Slot__default::Ty {
			itemId: id("1.21.4", "items", "pale_oak_log"),
			addedComponentCount: 1,
			removedComponentCount: 0,
			components: vec![d::SlotComponent::Ty::item_model(
				"minecraft:stone".to_owned(),
			)],
			removeComponents: vec![],
		},
	);
	let server = [
		S::position_look(d::play_toServer_packet_position_look::Ty {
			x: 0.5,
			y: 70.0,
			z: 0.5,
			yaw: 0.0,
			pitch: 0.0,
			flags: flags(true),
		}),
		S::tick_end(d::play_toServer_packet_tick_end::Ty {}),
		S::tick_end(d::play_toServer_packet_tick_end::Ty {}),
		S::look(d::play_toServer_packet_look::Ty {
			yaw: 90.0,
			pitch: 5.0,
			flags: flags(false),
		}),
		S::flying(flags(true)),
		S::player_input(d::play_toServer_packet_player_input::Ty {
			forward: true,
			backward: false,
			left: false,
			right: false,
			jump: false,
			shift: false,
			sprint: false,
		}),
		S::set_creative_slot(d::play_toServer_packet_set_creative_slot::Ty {
			slot: 36,
			item: slot.clone(),
		}),
		S::window_click(d::play_toServer_packet_window_click::Ty {
			windowId: 0,
			stateId: 3,
			slot: 36,
			mouseButton: 0,
			mode: 0,
			changedSlots: vec![
				d::play_toServer_packet_window_click__changedSlots__items::Ty {
					location: 36,
					item: d::Slot::Ty::v0,
				},
			],
			cursorItem: slot,
		}),
		S::block_place(d::play_toServer_packet_block_place::Ty {
			hand: 0,
			location: d::position::Ty { x: 1, y: 64, z: 1 },
			direction: 1,
			cursorX: 0.5,
			cursorY: 1.0,
			cursorZ: 0.5,
			insideBlock: false,
			worldBorderHit: false,
			sequence: 4,
		}),
		S::use_item(d::play_toServer_packet_use_item::Ty {
			hand: 0,
			sequence: 5,
			rotation: d::vec2f::Ty { x: 90.0, y: 5.0 },
		}),
		S::settings(d::packet_common_settings::Ty {
			locale: "en_us".to_owned(),
			viewDistance: 8,
			chatFlags: 0,
			chatColors: true,
			skinParts: 127,
			mainHand: 1,
			enableTextFiltering: false,
			enableServerListing: true,
			particleStatus: d::packet_common_settings__particleStatus::Ty::all,
		}),
		S::close_window(0),
	];
	for to in ["1.21.4", "1.21.3", "1.21.1", "1.20.5"] {
		let mut cx = Session::new();
		let mut out = Vec::new();
		for p in &server {
			let mut o = Output::new();
			d::play_toServer_packet::serialize(p, &mut o).unwrap();
			for bytes in convert(
				Root::PlayToServer,
				at("1.21.4"),
				at(to),
				&o.into_inner(),
				&mut cx,
			)
			.unwrap()
			{
				out.extend((bytes.len() as u32).to_be_bytes());
				out.extend(bytes);
			}
		}
		std::fs::write(format!("{dir}/server-{to}.bin"), out).unwrap();
	}
	let reverse = reverse_samples();
	for to in ["1.21.4", "1.21.3", "1.21.1", "1.20.5"] {
		dump(
			&format!("{dir}/reverse-{to}.bin"),
			Root::PlayToClient,
			("1.21.4", to),
			&reverse,
			d::play_toClient_packet::serialize,
		);
	}
	use a::play_toServer_packet::Ty as O;
	let old = [
		O::position(a::play_toServer_packet_position::Ty {
			x: 0.5,
			y: 70.0,
			z: 0.5,
			onGround: true,
		}),
		O::look(a::play_toServer_packet_look::Ty {
			yaw: 10.0,
			pitch: 0.0,
			onGround: true,
		}),
		O::flying(true),
		O::steer_vehicle(a::play_toServer_packet_steer_vehicle::Ty {
			sideways: 0.98,
			forward: 0.0,
			jump: 3,
		}),
		O::craft_recipe_request(a::play_toServer_packet_craft_recipe_request::Ty {
			windowId: 1,
			recipe: "minecraft:7".to_owned(),
			makeAll: true,
		}),
		O::displayed_recipe("minecraft:7".to_owned()),
		O::use_item(a::play_toServer_packet_use_item::Ty {
			hand: 0,
			sequence: 2,
		}),
		O::set_creative_slot(a::play_toServer_packet_set_creative_slot::Ty {
			slot: 36,
			item: item_a("diamond", vec![]),
		}),
	];
	for to in ["1.20.5", "1.21.1", "1.21.3", "1.21.4"] {
		dump(
			&format!("{dir}/server-up-{to}.bin"),
			Root::PlayToServer,
			("1.20.5", to),
			&old,
			a::play_toServer_packet::serialize,
		);
	}
}

fn dump<A>(
	file: &str,
	root: Root,
	(from, to): (&str, &str),
	packets: &[A],
	ser: fn(&A, &mut Output) -> minecraft_versions::pds::Result<()>,
) {
	let mut cx = Session::new();
	let mut out = Vec::new();
	for p in packets {
		let mut o = Output::new();
		ser(p, &mut o).unwrap();
		for bytes in convert(root, at(from), at(to), &o.into_inner(), &mut cx).unwrap() {
			out.extend((bytes.len() as u32).to_be_bytes());
			out.extend(bytes);
		}
	}
	std::fs::write(file, out).unwrap();
}

fn reverse_samples() -> Vec<d::play_toClient_packet::Ty> {
	use d::play_toClient_SlotDisplay::Ty as S;
	use d::play_toClient_packet::Ty as C;
	let flags = |relative: bool| d::play_toClient_PositionUpdateRelatives::Ty {
		x: relative,
		y: relative,
		z: relative,
		yaw: false,
		pitch: false,
		dx: relative,
		dy: relative,
		dz: relative,
		yawDelta: false,
	};
	let beef = item_d(
		"cooked_beef",
		vec![
			d::SlotComponent::Ty::food(d::SlotComponent__food::Ty {
				nutrition: 8,
				saturationModifier: 12.8,
				canAlwaysEat: false,
			}),
			d::SlotComponent::Ty::consumable(d::SlotComponent__consumable::Ty {
				consume_seconds: 0.8,
				animation: d::SlotComponent__consumable__animation::Ty::eat,
				sound: d::ItemSoundHolder::Ty::Id(id("1.21.4", "sounds", "entity.generic.eat")),
				makes_particles: true,
				effects: vec![d::ItemConsumeEffect::Ty::apply_effects(
					d::ItemConsumeEffect__apply_effects::Ty {
						effects: vec![potion("Hunger")],
						probability: 0.25,
					},
				)],
			}),
			d::SlotComponent::Ty::use_remainder(item_d("bowl", vec![])),
			d::SlotComponent::Ty::custom_model_data(d::SlotComponent__custom_model_data::Ty {
				floats: vec![3.0],
				flags: vec![],
				strings: vec![],
				colors: vec![],
			}),
		],
	);
	let decoration = d::play_toClient_ChatType::Ty {
		translationKey: "chat.type.text".to_owned(),
		parameters: vec![
			d::play_toClient_ChatTypeParameterType::Ty::sender,
			d::play_toClient_ChatTypeParameterType::Ty::content,
		],
		style: tag(vec![]),
	};
	vec![
		C::login(d::play_toClient_packet_login::Ty {
			entityId: 42,
			isHardcore: false,
			worldNames: vec!["minecraft:overworld".to_owned()],
			maxPlayers: 20,
			viewDistance: 8,
			simulationDistance: 8,
			reducedDebugInfo: false,
			enableRespawnScreen: true,
			doLimitedCrafting: false,
			worldState: d::play_toClient_SpawnInfo::Ty {
				dimension: 0,
				name: "minecraft:overworld".to_owned(),
				hashedSeed: 0,
				gamemode: d::play_toClient_SpawnInfo__gamemode::Ty::survival,
				previousGamemode: 255,
				isDebug: false,
				isFlat: false,
				death: None,
				portalCooldown: 0,
				seaLevel: 63,
			},
			enforcesSecureChat: false,
		}),
		C::position(d::play_toClient_packet_position::Ty { teleportId: 1, x: 0.5, y: 70.0, z: 0.5, dx: 0.0, dy: 0.0, dz: 0.0, yaw: 0.0, pitch: 0.0, flags: flags(false) }),
		C::position(d::play_toClient_packet_position::Ty { teleportId: 2, x: 1.0, y: 0.0, z: 0.0, dx: 0.1, dy: 0.2, dz: 0.0, yaw: 0.0, pitch: 0.0, flags: flags(true) }),
		C::entity_teleport(d::play_toClient_packet_entity_teleport::Ty {
			entityId: 7,
			x: 5.0,
			y: 64.0,
			z: 5.0,
			dx: 0.0,
			dy: 0.0,
			dz: 0.0,
			yaw: 90.0,
			pitch: 0.0,
			flags: flags(false),
			onGround: true,
		}),
		C::sync_entity_position(d::play_toClient_packet_sync_entity_position::Ty { entityId: 7, x: 5.0, y: 64.0, z: 5.0, dx: 0.0, dy: 0.0, dz: 0.0, yaw: 0.0, pitch: 0.0, onGround: true }),
		C::player_rotation(d::play_toClient_packet_player_rotation::Ty { yaw: 45.0, pitch: 10.0 }),
		C::explosion(d::play_toClient_packet_explosion::Ty {
			x: 1.0,
			y: 2.0,
			z: 3.0,
			playerKnockback: Some(d::vec3f64::Ty { x: 0.25, y: 0.5, z: 0.0 }),
			explosionParticle: d::Particle::Ty::explosion,
			sound: d::ItemSoundHolder::Ty::Id(id("1.21.4", "sounds", "entity.generic.explode")),
		}),
		C::spawn_entity(d::play_toClient_packet_spawn_entity::Ty {
			entityId: 9,
			objectUUID: 9,
			r#type: id("1.21.4", "entities", "spruce_boat"),
			x: 0.0,
			y: 64.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			headPitch: 0,
			objectData: 0,
			velocity: d::vec3i16::Ty { x: 0, y: 0, z: 0 },
		}),
		C::declare_recipes(d::play_toClient_packet_declare_recipes::Ty {
			recipes: vec![d::play_toClient_packet_declare_recipes__recipes__items::Ty { name: "minecraft:furnace_input".to_owned(), items: vec![id("1.21.4", "items", "beef")] }],
			stoneCutterRecipes: vec![d::play_toClient_packet_declare_recipes__stoneCutterRecipes__items::Ty {
				input: d::IDSet::Ty::Ids(vec![id("1.21.4", "items", "stone")]),
				slotDisplay: S::item_stack(item_d("stone_slab", vec![])),
			}],
		}),
		C::recipe_book_settings(d::play_toClient_packet_recipe_book_settings::Ty {
			craftingGuiOpen: true,
			craftingFilteringCraftable: false,
			smeltingGuiOpen: false,
			smeltingFilteringCraftable: false,
			blastGuiOpen: false,
			blastFilteringCraftable: false,
			smokerGuiOpen: false,
			smokerFilteringCraftable: false,
		}),
		C::recipe_book_add(d::play_toClient_packet_recipe_book_add::Ty {
			entries: vec![d::play_toClient_packet_recipe_book_add__entries__items::Ty {
				recipe: d::play_toClient_packet_recipe_book_add__entries__items__recipe::Ty {
					displayId: 7,
					display: d::play_toClient_RecipeDisplay::Ty::crafting_shapeless(d::play_toClient_RecipeDisplay__crafting_shapeless::Ty {
						ingredients: vec![S::item(id("1.21.4", "items", "oak_log"))],
						result: S::item_stack(item_d("oak_planks", vec![])),
						craftingStation: S::item(id("1.21.4", "items", "crafting_table")),
					}),
					group: 0,
					category: d::play_toClient_packet_recipe_book_add__entries__items__recipe__category::Ty::crafting_building_blocks,
					craftingRequirements: None,
				},
				flags: d::play_toClient_packet_recipe_book_add__entries__items__flags::Ty { notification: true, highlight: false },
			}],
			replace: true,
		}),
		C::recipe_book_remove(vec![7]),
		C::set_slot(d::play_toClient_packet_set_slot::Ty { windowId: 0, stateId: 1, slot: 5, item: beef.clone() }),
		C::set_cursor_item(item_d("pale_oak_log", vec![])),
		C::set_player_inventory(d::play_toClient_packet_set_player_inventory::Ty { slotId: 3, contents: beef.clone() }),
		C::window_items(d::play_toClient_packet_window_items::Ty { windowId: 0, stateId: 2, items: vec![beef, d::Slot::Ty::v0], carriedItem: d::Slot::Ty::v0 }),
		C::profileless_chat(d::play_toClient_packet_profileless_chat::Ty {
			message: string_tag("hi"),
			r#type: d::play_toClient_ChatTypesHolder::Ty::Inline(d::play_toClient_ChatTypes::Ty { chat: decoration.clone(), narration: decoration }),
			name: string_tag("Bob"),
			target: None,
		}),
		C::profileless_chat(d::play_toClient_packet_profileless_chat::Ty {
			message: string_tag("hi"),
			r#type: d::play_toClient_ChatTypesHolder::Ty::Id(0),
			name: string_tag("Bob"),
			target: None,
		}),
	]
}

#[test]
fn sample_packets_convert_and_reparse() {
	let mut cx = Session::new();
	let mut kinds = Vec::new();
	for p in sample_packets() {
		for q in up(&p, &mut cx) {
			kinds.push(format!("{q:?}").split('(').next().unwrap().to_owned());
		}
	}
	for kind in [
		"set_cursor_item",
		"sync_entity_position",
		"explosion",
		"block_change",
		"map_chunk",
		"world_event",
	] {
		assert!(
			kinds.iter().any(|k| k == kind),
			"{kind} missing in {kinds:?}"
		);
	}
}

#[test]
fn movement_goes_down_with_ticks() {
	use a::play_toServer_packet::Ty as P;
	use d::play_toServer_packet::Ty as Q;
	let mut cx = Session::new();
	let flags = |on_ground| d::play_toServer_MovementFlags::Ty {
		onGround: on_ground,
		hasHorizontalCollision: false,
	};
	let out = down(
		&Q::position_look(d::play_toServer_packet_position_look::Ty {
			x: 1.0,
			y: 2.0,
			z: 3.0,
			yaw: 4.0,
			pitch: 5.0,
			flags: flags(true),
		}),
		&mut cx,
	);
	assert!(matches!(out.as_slice(), [P::position_look(p)] if p.onGround && p.yaw == 4.0));
	let tick = Q::tick_end(d::play_toServer_packet_tick_end::Ty {});
	assert!(down(&tick, &mut cx).is_empty(), "the tick already moved");
	assert!(matches!(down(&tick, &mut cx).as_slice(), [P::flying(true)]));
	let out = down(&Q::flying(flags(false)), &mut cx);
	assert!(matches!(out.as_slice(), [P::flying(false)]));
	let input = d::play_toServer_packet_player_input::Ty {
		forward: true,
		backward: false,
		left: false,
		right: false,
		jump: false,
		shift: true,
		sprint: false,
	};
	assert!(
		down(&Q::player_input(input.clone()), &mut cx).is_empty(),
		"not riding"
	);

	use a::play_toClient_packet::Ty as C;
	let login = C::login(a::play_toClient_packet_login::Ty {
		entityId: 42,
		isHardcore: false,
		worldNames: vec!["minecraft:overworld".to_owned()],
		maxPlayers: 20,
		viewDistance: 8,
		simulationDistance: 8,
		reducedDebugInfo: false,
		enableRespawnScreen: true,
		doLimitedCrafting: false,
		worldState: a::play_toClient_SpawnInfo::Ty {
			dimension: 0,
			name: "minecraft:overworld".to_owned(),
			hashedSeed: 0,
			gamemode: a::play_toClient_SpawnInfo__gamemode::Ty::survival,
			previousGamemode: 255,
			isDebug: false,
			isFlat: false,
			death: None,
			portalCooldown: 0,
		},
		enforcesSecureChat: false,
	});
	let out = up(&login, &mut cx);
	let [d::play_toClient_packet::Ty::login(l)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(l.worldState.seaLevel, 63);
	up(
		&C::set_passengers(a::play_toClient_packet_set_passengers::Ty {
			entityId: 7,
			passengers: vec![42],
		}),
		&mut cx,
	);
	let out = down(&Q::player_input(input), &mut cx);
	assert!(
		matches!(out.as_slice(), [P::steer_vehicle(s)] if s.forward > 0.9 && s.sideways == 0.0 && s.jump == 2),
		"{out:?}"
	);
	let out = down(&tick, &mut cx);
	assert!(
		matches!(out.as_slice(), [P::look(_), P::steer_vehicle(_)]),
		"{out:?}"
	);
}

fn client_down(
	p: &d::play_toClient_packet::Ty,
	cx: &mut Session,
) -> Vec<a::play_toClient_packet::Ty> {
	run(
		Root::PlayToClient,
		("1.21.4", "1.20.5"),
		p,
		d::play_toClient_packet::serialize,
		a::play_toClient_packet::deserialize,
		cx,
	)
}

fn server_up(
	p: &a::play_toServer_packet::Ty,
	cx: &mut Session,
) -> Vec<d::play_toServer_packet::Ty> {
	run(
		Root::PlayToServer,
		("1.20.5", "1.21.4"),
		p,
		a::play_toServer_packet::serialize,
		d::play_toServer_packet::deserialize,
		cx,
	)
}

fn config_down(
	p: &d::configuration_toClient_packet::Ty,
	cx: &mut Session,
) -> Vec<a::configuration_toClient_packet::Ty> {
	run(
		Root::ConfigurationToClient,
		("1.21.4", "1.20.5"),
		p,
		d::configuration_toClient_packet::serialize,
		a::configuration_toClient_packet::deserialize,
		cx,
	)
}

fn text(name: &str, value: &str) -> nbt::gen::pds_Entry::Ty {
	nbt::gen::pds_Entry::Ty::new(
		name.to_owned(),
		nbt::gen::pds_Payload::Ty::string(value.to_owned()),
	)
}

fn string_tag(value: &str) -> nbt::gen::pds_Tag::Ty {
	nbt::gen::pds_Tag::Ty::new(nbt::gen::pds_Payload::Ty::string(value.to_owned()))
}

fn compound(tag: &nbt::gen::pds_Tag::Ty) -> &[nbt::gen::pds_Entry::Ty] {
	match &tag.value {
		nbt::gen::pds_Payload::Ty::compound(e) => e,
		other => panic!("{other:?}"),
	}
}

fn field<'a>(entries: &'a [nbt::gen::pds_Entry::Ty], name: &str) -> &'a nbt::gen::pds_Payload::Ty {
	&entries
		.iter()
		.find(|e| e.name == name)
		.unwrap_or_else(|| panic!("{name} missing"))
		.value
}

fn item_d(name: &str, components: Vec<d::SlotComponent::Ty>) -> d::Slot::Ty {
	d::Slot::Ty::default(
		1,
		d::Slot__default::Ty {
			itemId: id("1.21.4", "items", name),
			addedComponentCount: components.len() as i32,
			removedComponentCount: 0,
			components,
			removeComponents: vec![],
		},
	)
}

fn potion(effect: &str) -> d::ItemPotionEffect::Ty {
	d::ItemPotionEffect::Ty {
		id: id("1.21.4", "effects", effect),
		details: d::ItemEffectDetail::Ty {
			amplifier: 0,
			duration: 600,
			ambient: false,
			showParticles: true,
			showIcon: true,
			hiddenEffect: None,
		},
	}
}

#[test]
fn registries_rebuilt_from_server_going_down() {
	use a::configuration_toClient_packet::Ty as P;
	use d::configuration_toClient_packet::Ty as Q;
	let mut cx = Session::new();
	let entry =
		|key: &str, value| d::configuration_toClient_packet_registry_data__entries__items::Ty {
			key: key.to_owned(),
			value,
		};
	let registry = |id: &str, entries| {
		Q::registry_data(d::configuration_toClient_packet_registry_data::Ty {
			id: id.to_owned(),
			entries,
		})
	};
	let paintings = registry(
		"minecraft:painting_variant",
		vec![
			entry("minecraft:pointer", None),
			entry("minecraft:kebab", None),
		],
	);
	assert!(
		config_down(&paintings, &mut cx).is_empty(),
		"1.20.5 has no painting registry"
	);
	let enchantments = registry(
		"minecraft:enchantment",
		vec![
			entry("minecraft:mending", None),
			entry("minecraft:sharpness", None),
		],
	);
	assert!(
		config_down(&enchantments, &mut cx).is_empty(),
		"1.20.5 has no enchantment registry"
	);
	let resin = tag(vec![
		text("asset_name", "resin"),
		text("ingredient", "minecraft:resin_brick"),
		text("description", "resin"),
	]);
	let out = config_down(
		&registry(
			"minecraft:trim_material",
			vec![entry("minecraft:resin", Some(resin))],
		),
		&mut cx,
	);
	let [P::registry_data(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let fields = compound(r.entries[0].value.as_ref().unwrap());
	assert_eq!(
		field(fields, "asset_name"),
		&nbt::gen::pds_Payload::Ty::string("gold".to_owned())
	);
	assert_eq!(
		field(fields, "ingredient"),
		&nbt::gen::pds_Payload::Ty::string("minecraft:copper_ingot".to_owned())
	);
	assert!(matches!(
		field(fields, "item_model_index"),
		nbt::gen::pds_Payload::Ty::float(_)
	));
	let music = nbt::gen::pds_Entry::Ty::new(
		"music",
		nbt::gen::pds_Payload::Ty::list(nbt::gen::pds_List::Ty::new(
			nbt::gen::pds_List_0::Ty::compound(vec![vec![
				nbt::gen::pds_Entry::Ty::new(
					"data",
					nbt::gen::pds_Payload::Ty::compound(vec![text(
						"sound",
						"minecraft:music.game",
					)]),
				),
				nbt::gen::pds_Entry::Ty::new("weight", nbt::gen::pds_Payload::Ty::int(1)),
			]]),
		)),
	);
	let dust = nbt::gen::pds_Entry::Ty::new(
		"particle",
		nbt::gen::pds_Payload::Ty::compound(vec![nbt::gen::pds_Entry::Ty::new(
			"options",
			nbt::gen::pds_Payload::Ty::compound(vec![
				text("type", "minecraft:dust"),
				nbt::gen::pds_Entry::Ty::new("color", nbt::gen::pds_Payload::Ty::int(0xff0000)),
				nbt::gen::pds_Entry::Ty::new("scale", nbt::gen::pds_Payload::Ty::float(1.0)),
			]),
		)]),
	);
	let effects = nbt::gen::pds_Entry::Ty::new(
		"effects",
		nbt::gen::pds_Payload::Ty::compound(vec![music, dust]),
	);
	let out = config_down(
		&registry(
			"minecraft:worldgen/biome",
			vec![entry("minecraft:plains", Some(tag(vec![effects])))],
		),
		&mut cx,
	);
	let [P::registry_data(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let nbt::gen::pds_Payload::Ty::compound(effects) =
		field(compound(r.entries[0].value.as_ref().unwrap()), "effects")
	else {
		panic!()
	};
	assert!(
		matches!(field(effects, "music"), nbt::gen::pds_Payload::Ty::compound(m) if m.len() == 1),
		"music unwrapped"
	);
	let nbt::gen::pds_Payload::Ty::compound(particle) = field(effects, "particle") else {
		panic!()
	};
	let nbt::gen::pds_Payload::Ty::compound(options) = field(particle, "options") else {
		panic!()
	};
	assert!(
		matches!(field(options, "color"), nbt::gen::pds_Payload::Ty::list(nbt::gen::pds_List::Ty { items: nbt::gen::pds_List_0::Ty::float(c), .. }) if c == &[1.0, 0.0, 0.0]),
		"dust color is a float list again"
	);

	use a::play_toClient_packet::Ty as A;
	use d::play_toClient_packet::Ty as C;
	let painting = C::entity_metadata(d::play_toClient_packet_entity_metadata::Ty {
		entityId: 5,
		metadata: vec![d::entityMetadataEntry::Ty {
			key: 8,
			value: d::entityMetadataEntry__value::Ty::painting_variant(
				d::entityMetadataEntry__value__painting_variant::Ty::Id(0),
			),
		}],
	});
	let out = client_down(&painting, &mut cx);
	let [A::entity_metadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert!(
		matches!(
			&m.metadata[0].value,
			a::entityMetadataEntry__value::Ty::painting_variant(21)
		),
		"server painting 0 is pointer: {m:?}"
	);
	let sword = item_d(
		"diamond_sword",
		vec![d::SlotComponent::Ty::enchantments(
			d::SlotComponent__enchantments::Ty {
				enchantments: vec![d::SlotComponent__enchantments__enchantments__items::Ty {
					id: 1,
					level: 5,
				}],
				showTooltip: true,
			},
		)],
	);
	let out = client_down(
		&C::set_slot(d::play_toClient_packet_set_slot::Ty {
			windowId: 0,
			stateId: 1,
			slot: 36,
			item: sword,
		}),
		&mut cx,
	);
	let [A::set_slot(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let a::Slot::Ty::default(_, item) = &s.item else {
		panic!()
	};
	let [a::SlotComponent::Ty::enchantments(e)] = item.components.as_slice() else {
		panic!("{item:?}")
	};
	assert_eq!(
		e.enchantments[0].id,
		id("1.20.5", "enchantments", "sharpness")
	);
}

#[test]
fn signed_inline_chat_keeps_the_registered_type() {
	use d::play_toClient_ChatTypeParameterType::Ty as Param;
	let mut cx = Session::new();
	let entry = |key: &str| d::configuration_toClient_packet_registry_data__entries__items::Ty {
		key: format!("minecraft:{key}"),
		value: None,
	};
	let keys = [
		"chat",
		"emote_command",
		"msg_command_incoming",
		"msg_command_outgoing",
		"say_command",
		"team_msg_command_incoming",
		"team_msg_command_outgoing",
	];
	let types = d::configuration_toClient_packet::Ty::registry_data(
		d::configuration_toClient_packet_registry_data::Ty {
			id: "minecraft:chat_type".to_owned(),
			entries: keys.iter().map(|k| entry(k)).collect(),
		},
	);
	assert_eq!(config_down(&types, &mut cx).len(), 1);
	let signed = |key: &str| {
		let decoration = d::play_toClient_ChatType::Ty {
			translationKey: key.to_owned(),
			parameters: vec![Param::sender, Param::content],
			style: tag(vec![]),
		};
		d::play_toClient_packet::Ty::player_chat(d::play_toClient_packet_player_chat::Ty {
			senderUuid: 1,
			index: 0,
			signature: Some(vec![7; 256]),
			plainMessage: "hi".to_owned(),
			timestamp: 0,
			salt: 0,
			previousMessages: vec![],
			unsignedChatContent: None,
			filterTypeMask: d::play_toClient_packet_player_chat__filterTypeMask::Ty::default(0),
			r#type: d::play_toClient_ChatTypesHolder::Ty::Inline(d::play_toClient_ChatTypes::Ty {
				chat: decoration.clone(),
				narration: decoration,
			}),
			networkName: string_tag("Bob"),
			networkTargetName: None,
		})
	};
	for (key, id) in [
		("chat.type.announcement", 4),
		("chat.type.emote", 1),
		("custom.key", 0),
	] {
		let out = client_down(&signed(key), &mut cx);
		let [a::play_toClient_packet::Ty::player_chat(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(p.r#type, id, "{key}");
	}
}

#[test]
fn world_and_recipes_going_down() {
	use a::play_toClient_packet::Ty as A;
	use d::play_toClient_packet::Ty as C;
	let mut cx = Session::new();
	let login = C::login(d::play_toClient_packet_login::Ty {
		entityId: 42,
		isHardcore: false,
		worldNames: vec!["minecraft:overworld".to_owned()],
		maxPlayers: 20,
		viewDistance: 8,
		simulationDistance: 8,
		reducedDebugInfo: false,
		enableRespawnScreen: true,
		doLimitedCrafting: false,
		worldState: d::play_toClient_SpawnInfo::Ty {
			dimension: 0,
			name: "minecraft:overworld".to_owned(),
			hashedSeed: 0,
			gamemode: d::play_toClient_SpawnInfo__gamemode::Ty::survival,
			previousGamemode: 255,
			isDebug: false,
			isFlat: false,
			death: None,
			portalCooldown: 0,
			seaLevel: 63,
		},
		enforcesSecureChat: false,
	});
	let out = client_down(&login, &mut cx);
	assert!(
		matches!(out.as_slice(), [A::login(l)] if l.entityId == 42),
		"{out:?}"
	);
	let queued = cx.take_outgoing();
	let [loaded] = queued.as_slice() else {
		panic!("player_loaded not queued")
	};
	assert_eq!(loaded.version, at("1.21.4"));
	let mut input = Input::new(&loaded.data);
	assert!(matches!(
		d::play_toServer_packet::deserialize(&mut input).unwrap(),
		d::play_toServer_packet::Ty::player_loaded(_)
	));

	let flags = |relative: bool, delta: bool| d::play_toClient_PositionUpdateRelatives::Ty {
		x: relative,
		y: relative,
		z: relative,
		yaw: false,
		pitch: false,
		dx: delta,
		dy: delta,
		dz: delta,
		yawDelta: false,
	};
	let position = |x: f64, relative: bool, delta: (f64, bool)| {
		C::position(d::play_toClient_packet_position::Ty {
			teleportId: 1,
			x,
			y: 64.0,
			z: 0.0,
			dx: delta.0,
			dy: 0.0,
			dz: 0.0,
			yaw: 0.0,
			pitch: 0.0,
			flags: flags(relative, delta.1),
		})
	};
	let out = client_down(&position(10.0, false, (0.0, false)), &mut cx);
	assert!(
		matches!(out.as_slice(), [A::position(p), A::entity_velocity(_)] if p.x == 10.0),
		"{out:?}"
	);
	let out = client_down(&position(1.0, true, (0.5, true)), &mut cx);
	assert!(
		matches!(out.as_slice(), [A::position(p), A::explosion(e)] if p.flags.x && e.playerMotionX == 0.5),
		"{out:?}"
	);
	let teleport = |entity: i32| {
		C::entity_teleport(d::play_toClient_packet_entity_teleport::Ty {
			entityId: entity,
			x: 2.0,
			y: 0.0,
			z: 0.0,
			dx: 0.0,
			dy: 0.0,
			dz: 0.0,
			yaw: 0.0,
			pitch: 0.0,
			flags: flags(true, true),
			onGround: true,
		})
	};
	let out = client_down(&teleport(42), &mut cx);
	assert!(
		matches!(out.as_slice(), [A::entity_teleport(t)] if t.x == 13.0 && t.y == 128.0),
		"relative to tracked player: {out:?}"
	);
	assert!(
		client_down(&teleport(7), &mut cx).is_empty(),
		"relative teleport of unknown entity"
	);

	let sound = |v: &str| id(v, "sounds", "entity.generic.explode");
	let explosion = C::explosion(d::play_toClient_packet_explosion::Ty {
		x: 1.0,
		y: 2.0,
		z: 3.0,
		playerKnockback: Some(d::vec3f64::Ty {
			x: 0.25,
			y: 0.5,
			z: 0.0,
		}),
		explosionParticle: d::Particle::Ty::explosion,
		sound: d::ItemSoundHolder::Ty::Id(sound("1.21.4")),
	});
	let out = client_down(&explosion, &mut cx);
	let [A::explosion(e)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(e.playerMotionX, e.playerMotionY, e.radius),
		(0.25, 0.5, 0.0)
	);
	assert!(matches!(
		e.small_explosion_particle,
		a::Particle::Ty::explosion
	));
	assert_eq!(e.sound, a::ItemSoundHolder::Ty::Id(sound("1.20.5")));

	let stone = id("1.21.4", "items", "stone");
	let declare = C::declare_recipes(d::play_toClient_packet_declare_recipes::Ty {
		recipes: vec![],
		stoneCutterRecipes: vec![
			d::play_toClient_packet_declare_recipes__stoneCutterRecipes__items::Ty {
				input: d::IDSet::Ty::Ids(vec![stone]),
				slotDisplay: d::play_toClient_SlotDisplay::Ty::item_stack(item_d(
					"stone_slab",
					vec![],
				)),
			},
		],
	});
	let out = client_down(&declare, &mut cx);
	let [A::declare_recipes(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert!(
		matches!(&r[0].data, a::play_toClient_packet__declare_recipes__items__data::Ty::minecraft_stonecutting(s) if s.ingredient.len() == 1)
	);
	use d::play_toClient_SlotDisplay::Ty as S;
	let book = C::recipe_book_add(d::play_toClient_packet_recipe_book_add::Ty {
		entries: vec![d::play_toClient_packet_recipe_book_add__entries__items::Ty {
			recipe: d::play_toClient_packet_recipe_book_add__entries__items__recipe::Ty {
				displayId: 7,
				display: d::play_toClient_RecipeDisplay::Ty::crafting_shapeless(d::play_toClient_RecipeDisplay__crafting_shapeless::Ty {
					ingredients: vec![S::item(id("1.21.4", "items", "oak_log"))],
					result: S::item_stack(item_d("oak_planks", vec![])),
					craftingStation: S::item(id("1.21.4", "items", "crafting_table")),
				}),
				group: 0,
				category: d::play_toClient_packet_recipe_book_add__entries__items__recipe__category::Ty::crafting_building_blocks,
				craftingRequirements: None,
			},
			flags: d::play_toClient_packet_recipe_book_add__entries__items__flags::Ty { notification: true, highlight: true },
		}],
		replace: true,
	});
	let out = client_down(&book, &mut cx);
	let [A::declare_recipes(r), A::unlock_recipes(u)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(r.len(), 2, "stonecutter recipe kept alongside the book");
	assert!(
		matches!(&r[1].data, a::play_toClient_packet__declare_recipes__items__data::Ty::minecraft_crafting_shapeless(s) if s.ingredients.len() == 1)
	);
	assert_eq!(u.recipes1, vec!["minecraft:7".to_owned()]);
	assert!(
		matches!(&u.recipes2, a::play_toClient_packet_unlock_recipes__recipes2::Ty::v0(h) if h == &u.recipes1)
	);
	let request = a::play_toServer_packet::Ty::craft_recipe_request(
		a::play_toServer_packet_craft_recipe_request::Ty {
			windowId: 1,
			recipe: "minecraft:7".to_owned(),
			makeAll: false,
		},
	);
	let out = server_up(&request, &mut cx);
	assert!(
		matches!(out.as_slice(), [d::play_toServer_packet::Ty::craft_recipe_request(r)] if r.recipeId == 7 && r.windowId == 1),
		"{out:?}"
	);

	use d::play_toClient_ChatTypeParameterType::Ty as Param;
	let decoration = d::play_toClient_ChatType::Ty {
		translationKey: "chat.type.text".to_owned(),
		parameters: vec![Param::sender, Param::content],
		style: tag(vec![text("color", "red")]),
	};
	let chat = C::profileless_chat(d::play_toClient_packet_profileless_chat::Ty {
		message: string_tag("hi"),
		r#type: d::play_toClient_ChatTypesHolder::Ty::Inline(d::play_toClient_ChatTypes::Ty {
			chat: decoration.clone(),
			narration: decoration,
		}),
		name: string_tag("Bob"),
		target: None,
	});
	let out = client_down(&chat, &mut cx);
	let [A::system_chat(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let content = compound(&s.content);
	assert_eq!(
		field(content, "translate"),
		&nbt::gen::pds_Payload::Ty::string("chat.type.text".to_owned())
	);
	assert_eq!(
		field(content, "color"),
		&nbt::gen::pds_Payload::Ty::string("red".to_owned())
	);
	let nbt::gen::pds_Payload::Ty::list(nbt::gen::pds_List::Ty {
		items: nbt::gen::pds_List_0::Ty::compound(with),
		..
	}) = field(content, "with")
	else {
		panic!()
	};
	assert_eq!(
		field(&with[0], "text"),
		&nbt::gen::pds_Payload::Ty::string("Bob".to_owned())
	);
	assert_eq!(
		field(&with[1], "text"),
		&nbt::gen::pds_Payload::Ty::string("hi".to_owned())
	);
}

#[test]
fn food_and_sword_blocking() {
	use a::play_toClient_packet::Ty as A;
	use d::play_toClient_packet::Ty as C;
	let mut cx = Session::new();
	let stew = item_a(
		"suspicious_stew",
		vec![a::SlotComponent::Ty::food(a::SlotComponent__food::Ty {
			nutrition: 6,
			saturationModifier: 0.6,
			canAlwaysEat: true,
			secondsToEat: 1.6,
			effects: vec![a::SlotComponent__food__effects__items::Ty {
				effect: a::ItemPotionEffect::Ty {
					id: id("1.20.5", "effects", "Regeneration"),
					details: a::ItemEffectDetail::Ty {
						amplifier: 0,
						duration: 160,
						ambient: false,
						showParticles: true,
						showIcon: true,
						hiddenEffect: None,
					},
				},
				probability: 0.5,
			}],
		})],
	);
	minecraft_versions::custom::tricky_enable_legacy_combat(&mut cx);
	let window = A::window_items(a::play_toClient_packet_window_items::Ty {
		windowId: 0,
		stateId: 1,
		items: vec![
			stew,
			item_a("diamond_sword", vec![]),
			item_a("diamond", vec![]),
		],
		carriedItem: a::Slot::Ty::v0,
	});
	let out = up(&window, &mut cx);
	let [C::window_items(w)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let components = |slot: &d::Slot::Ty| match slot {
		d::Slot::Ty::default(_, s) => {
			assert_eq!(s.addedComponentCount as usize, s.components.len());
			s.components.clone()
		}
		_ => vec![],
	};
	let food = components(&w.items[0]);
	let consumable = food
		.iter()
		.find_map(|c| match c {
			d::SlotComponent::Ty::consumable(c) => Some(c),
			_ => None,
		})
		.expect("food split into consumable");
	assert_eq!(consumable.consume_seconds, 1.6);
	assert!(
		matches!(&consumable.effects[..], [d::ItemConsumeEffect::Ty::apply_effects(e)] if e.probability == 0.5 && e.effects.len() == 1)
	);
	assert!(
		components(&w.items[1]).iter().any(|c| matches!(c, d::SlotComponent::Ty::consumable(c) if matches!(c.animation, d::SlotComponent__consumable__animation::Ty::block))),
		"sword blocks under legacy combat"
	);
	assert!(components(&w.items[2]).is_empty());

	let beef = item_d(
		"cooked_beef",
		vec![
			d::SlotComponent::Ty::food(d::SlotComponent__food::Ty {
				nutrition: 8,
				saturationModifier: 12.8,
				canAlwaysEat: false,
			}),
			d::SlotComponent::Ty::consumable(d::SlotComponent__consumable::Ty {
				consume_seconds: 0.8,
				animation: d::SlotComponent__consumable__animation::Ty::eat,
				sound: d::ItemSoundHolder::Ty::Id(id("1.21.4", "sounds", "entity.generic.eat")),
				makes_particles: true,
				effects: vec![d::ItemConsumeEffect::Ty::apply_effects(
					d::ItemConsumeEffect__apply_effects::Ty {
						effects: vec![potion("Hunger")],
						probability: 0.25,
					},
				)],
			}),
			d::SlotComponent::Ty::use_remainder(item_d("bowl", vec![])),
		],
	);
	let out = client_down(
		&C::set_slot(d::play_toClient_packet_set_slot::Ty {
			windowId: 0,
			stateId: 2,
			slot: 36,
			item: beef,
		}),
		&mut cx,
	);
	let [A::set_slot(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let a::Slot::Ty::default(_, item) = &s.item else {
		panic!()
	};
	assert_eq!(item.addedComponentCount as usize, item.components.len());
	let [a::SlotComponent::Ty::food(f)] = item.components.as_slice() else {
		panic!("{item:?}")
	};
	assert_eq!(f.secondsToEat, 0.8);
	assert!(
		matches!(&f.effects[..], [e] if e.probability == 0.25 && e.effect.id == id("1.20.5", "effects", "Hunger"))
	);
}

#[test]
fn projectile_power_going_down() {
	use a::play_toClient_packet::Ty as A;
	use d::play_toClient_packet::Ty as C;
	let mut cx = Session::new();
	let spawn = C::spawn_entity(d::play_toClient_packet_spawn_entity::Ty {
		entityId: 3,
		objectUUID: 3,
		r#type: id("1.21.4", "entities", "fireball"),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		pitch: 0,
		yaw: 0,
		headPitch: 0,
		objectData: 0,
		velocity: d::vec3i16::Ty { x: 800, y: 0, z: 0 },
	});
	client_down(&spawn, &mut cx);
	let power = |cx: &mut Session| {
		client_down(
			&C::set_projectile_power(d::play_toClient_packet_set_projectile_power::Ty {
				id: 3,
				accelerationPower: 0.1,
			}),
			cx,
		)
	};
	let out = power(&mut cx);
	assert!(
		matches!(out.as_slice(), [A::set_projectile_power(p)] if (p.power.x - 0.1).abs() < 1e-9 && p.power.y == 0.0),
		"{out:?}"
	);
	client_down(
		&C::entity_velocity(d::play_toClient_packet_entity_velocity::Ty {
			entityId: 3,
			velocity: d::vec3i16::Ty {
				x: 0,
				y: -400,
				z: 0,
			},
		}),
		&mut cx,
	);
	let out = power(&mut cx);
	assert!(
		matches!(out.as_slice(), [A::set_projectile_power(p)] if (p.power.y + 0.1).abs() < 1e-9),
		"{out:?}"
	);
	client_down(&C::entity_destroy(vec![3]), &mut cx);
	assert!(power(&mut cx).is_empty());
}

#[test]
fn old_client_movement_going_up() {
	use a::play_toServer_packet::Ty as P;
	use d::play_toServer_packet::Ty as Q;
	let mut cx = Session::new();
	let out = server_up(
		&P::position(a::play_toServer_packet_position::Ty {
			x: 1.0,
			y: 64.0,
			z: 2.0,
			onGround: true,
		}),
		&mut cx,
	);
	assert!(
		matches!(out.as_slice(), [Q::position(p), Q::tick_end(_)] if p.flags.onGround && p.x == 1.0),
		"{out:?}"
	);
	let out = server_up(
		&P::steer_vehicle(a::play_toServer_packet_steer_vehicle::Ty {
			sideways: 0.0,
			forward: 0.98,
			jump: 2,
		}),
		&mut cx,
	);
	use d::play_toServer_packet_entity_action__actionId::Ty as Action;
	assert!(
		matches!(out.as_slice(), [Q::player_input(i), Q::entity_action(e)] if i.forward && i.shift && !i.jump && matches!(e.actionId, Action::start_sneaking)),
		"{out:?}"
	);
	let out = server_up(
		&P::steer_vehicle(a::play_toServer_packet_steer_vehicle::Ty {
			sideways: 0.0,
			forward: 0.0,
			jump: 0,
		}),
		&mut cx,
	);
	assert!(
		matches!(out.as_slice(), [Q::player_input(_), Q::entity_action(e)] if matches!(e.actionId, Action::stop_sneaking)),
		"{out:?}"
	);
}
