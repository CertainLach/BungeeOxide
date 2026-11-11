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

fn up(p: &a::play_to_client_packet::Ty, cx: &mut Session) -> Vec<d::play_to_client_packet::Ty> {
	run(
		Root::PlayToClient,
		("1.20.5", "1.21.4"),
		p,
		a::play_to_client_packet::serialize,
		d::play_to_client_packet::deserialize,
		cx,
	)
}

fn down(p: &d::play_to_server_packet::Ty, cx: &mut Session) -> Vec<a::play_to_server_packet::Ty> {
	run(
		Root::PlayToServer,
		("1.21.4", "1.20.5"),
		p,
		d::play_to_server_packet::serialize,
		a::play_to_server_packet::deserialize,
		cx,
	)
}

fn config(
	p: &a::configuration_to_client_packet::Ty,
	cx: &mut Session,
) -> Vec<d::configuration_to_client_packet::Ty> {
	run(
		Root::ConfigurationToClient,
		("1.20.5", "1.21.4"),
		p,
		a::configuration_to_client_packet::serialize,
		d::configuration_to_client_packet::deserialize,
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

fn item_a(name: &str, components: Vec<a::slot_component::Ty>) -> a::slot::Ty {
	a::slot::Ty::Default(
		1,
		a::slot_default::Ty {
			item_id: id("1.20.5", "items", name),
			added_component_count: components.len() as i32,
			removed_component_count: 0,
			components,
			remove_components: vec![],
		},
	)
}

fn tag(entries: Vec<nbt::Entry>) -> nbt::gen::tag::Ty {
	nbt::Tag::new(entries)
}

#[test]
fn configuration_adds_required_registries() {
	use a::configuration_to_client_packet::Ty as P;
	let mut cx = Session::new();
	let damage = a::configuration_to_client_packet_registry_data::Ty {
		id: "minecraft:damage_type".to_owned(),
		entries: vec![
			a::configuration_to_client_packet_registry_data_entries_items::Ty {
				key: "minecraft:arrow".to_owned(),
				value: Some(tag(vec![nbt::Entry::new(
					"message_id",
					nbt::gen::payload::Ty::String("arrow".to_owned()),
				)])),
			},
		],
	};
	let mut out = config(&P::RegistryData(damage), &mut cx);
	out.extend(config(
		&P::FeatureFlags(vec!["minecraft:vanilla".to_owned()]),
		&mut cx,
	));
	out.extend(config(
		&P::FinishConfiguration(a::configuration_to_client_packet_finish_configuration::Ty {}),
		&mut cx,
	));
	use d::configuration_to_client_packet::Ty as Q;
	assert!(matches!(out.last(), Some(Q::FinishConfiguration(_))));
	let registries: Vec<&d::configuration_to_client_packet_registry_data::Ty> = out
		.iter()
		.filter_map(|p| match p {
			Q::RegistryData(r) => Some(r),
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
		matches!(&damage.entries[0].value, Some(t) if matches!(&t.value, nbt::gen::payload::Ty::Compound(e) if e.len() == 1))
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
	let nbt::gen::payload::Ty::Compound(biome) = &badlands.value.as_ref().unwrap().value else {
		panic!()
	};
	let effects = biome.iter().find(|e| e.name == "effects").unwrap();
	let nbt::gen::payload::Ty::Compound(effects) = &effects.value else {
		panic!()
	};
	let music = effects.iter().find(|e| e.name == "music").unwrap();
	assert_eq!(music.r#type, 9, "1.21.4 music is a weighted list");
	let again = config(
		&P::FinishConfiguration(a::configuration_to_client_packet_finish_configuration::Ty {}),
		&mut cx,
	);
	assert_eq!(
		again
			.iter()
			.filter(|p| matches!(p, Q::RegistryData(_)))
			.count(),
		registries.len(),
		"reconfiguration resends them"
	);
}

#[test]
fn tags_get_remapped_and_completed() {
	use a::configuration_to_client_packet::Ty as P;
	use d::configuration_to_client_packet::Ty as Q;
	let out = config(
		&P::Tags(vec![a::configuration_to_client_packet_tags_items::Ty {
			tag_type: "minecraft:block".to_owned(),
			tags: vec![
				a::configuration_to_client_packet_tags_items_tags_items::Ty {
					tag_name: "minecraft:logs".to_owned(),
					entries: vec![
						id("1.20.5", "blocks", "oak_log"),
						id("1.20.5", "blocks", "cherry_log"),
					],
				},
			],
		}]),
		&mut Session::new(),
	);
	let [Q::Tags(groups)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let blocks = groups
		.iter()
		.find(|g| g.tag_type == "minecraft:block")
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
		.find(|g| g.tag_type == "minecraft:enchantment")
		.unwrap();
	let curse = enchantments
		.tags
		.iter()
		.find(|t| t.tag_name == "minecraft:curse")
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
		.any(|t| t.tag_name == "minecraft:exclusive_set/armor"));
}

#[test]
fn positions_and_teleports() {
	use a::play_to_client_packet::Ty as P;
	use d::play_to_client_packet::Ty as Q;
	let mut cx = Session::new();
	let out = up(
		&P::Position(a::play_to_client_packet_position::Ty {
			x: 1.5,
			y: 64.0,
			z: -3.0,
			yaw: 90.0,
			pitch: 10.0,
			flags: a::play_to_client_position_update_relatives::Ty {
				x: false,
				y: true,
				z: false,
				yaw: true,
				pitch: true,
			},
			teleport_id: 7,
		}),
		&mut cx,
	);
	let [Q::Position(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(p.teleport_id, p.x, p.y, p.z, p.yaw, p.pitch),
		(7, 1.5, 64.0, -3.0, 90.0, 10.0)
	);
	assert_eq!((p.dx, p.dy, p.dz), (0.0, 0.0, 0.0));
	let f = &p.flags;
	assert_eq!(
		(f.x, f.y, f.z, f.yaw, f.pitch, f.dx, f.dy, f.dz, f.yaw_delta),
		(false, true, false, true, true, false, true, false, false)
	);
	let out = up(
		&P::EntityTeleport(a::play_to_client_packet_entity_teleport::Ty {
			entity_id: 5,
			x: 1.0,
			y: 2.0,
			z: 3.0,
			yaw: 64,
			pitch: -32,
			on_ground: true,
		}),
		&mut cx,
	);
	let [Q::SyncEntityPosition(t)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(t.entity_id, t.x, t.y, t.z, t.yaw, t.pitch, t.on_ground),
		(5, 1.0, 2.0, 3.0, 90.0, -45.0, true)
	);
}

#[test]
fn items_and_components() {
	use a::play_to_client_packet::Ty as P;
	use d::play_to_client_packet::Ty as Q;
	let mut cx = Session::new();
	let enchant = |name: &str, level: i32| a::slot_component_enchantments_enchantments_items::Ty {
		id: id("1.20.5", "enchantments", name),
		level,
	};
	let sword = item_a(
		"diamond_sword",
		vec![a::slot_component::Ty::Enchantments(
			a::slot_component_enchantments::Ty {
				enchantments: vec![enchant("sharpness", 5), enchant("unbreaking", 3)],
				show_tooltip: true,
			},
		)],
	);
	let glowing = item_a(
		"stone",
		vec![a::slot_component::Ty::Enchantments(
			a::slot_component_enchantments::Ty {
				enchantments: vec![enchant("protection", 0)],
				show_tooltip: false,
			},
		)],
	);
	let fire = item_a(
		"netherite_ingot",
		vec![a::slot_component::Ty::FireResistant],
	);
	let out = up(
		&P::WindowItems(a::play_to_client_packet_window_items::Ty {
			window_id: 0,
			state_id: 1,
			items: vec![sword, a::slot::Ty::V0, glowing, fire],
			carried_item: a::slot::Ty::V0,
		}),
		&mut cx,
	);
	let [Q::WindowItems(w)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(w.items.len(), 4);
	let d::slot::Ty::Default(1, s) = &w.items[0] else {
		panic!("{:?}", w.items[0])
	};
	assert_eq!(s.item_id, id("1.21.4", "items", "diamond_sword"));
	let [d::slot_component::Ty::Enchantments(e)] = s.components.as_slice() else {
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
	assert!(matches!(&w.items[1], d::slot::Ty::V0));
	let d::slot::Ty::Default(_, s) = &w.items[2] else {
		panic!()
	};
	assert!(
		matches!(
			s.components.as_slice(),
			[d::slot_component::Ty::EnchantmentGlintOverride(true)]
		),
		"{s:?}"
	);
	let d::slot::Ty::Default(_, s) = &w.items[3] else {
		panic!()
	};
	assert!(
		matches!(s.components.as_slice(), [d::slot_component::Ty::DamageResistant(t)] if t == "minecraft:is_fire"),
		"{s:?}"
	);
	assert_eq!(s.added_component_count, 1);

	let out = up(
		&P::SetSlot(a::play_to_client_packet_set_slot::Ty {
			window_id: 255,
			state_id: 0,
			slot: -1,
			item: item_a("apple", vec![]),
		}),
		&mut cx,
	);
	let [Q::SetCursorItem(d::slot::Ty::Default(1, s))] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.item_id, id("1.21.4", "items", "apple"));
}

#[test]
fn creative_items_go_down() {
	use a::play_to_server_packet::Ty as P;
	use d::play_to_server_packet::Ty as Q;
	let mut cx = Session::new();
	let slot = |name: &str| {
		d::slot::Ty::Default(
			3,
			d::slot_default::Ty {
				item_id: id("1.21.4", "items", name),
				added_component_count: 2,
				removed_component_count: 0,
				components: vec![
					d::slot_component::Ty::MaxStackSize(16),
					d::slot_component::Ty::Glider,
				],
				remove_components: vec![],
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
			&Q::SetCreativeSlot(d::play_to_server_packet_set_creative_slot::Ty {
				slot: 36,
				item: slot(new),
			}),
			&mut cx,
		);
		let [P::SetCreativeSlot(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		let a::slot::Ty::Default(3, s) = &p.item else {
			panic!("{p:?}")
		};
		assert_eq!(s.item_id, id("1.20.5", "items", old), "{new}");
		assert_eq!(s.added_component_count, 1);
		assert!(matches!(
			s.components.as_slice(),
			[a::slot_component::Ty::MaxStackSize(16)]
		));
	}
}

#[test]
fn update_attributes_get_modifier_ids() {
	use a::play_to_client_packet::Ty as P;
	use d::play_to_client_packet::Ty as Q;
	let out = up(
		&P::EntityUpdateAttributes(a::play_to_client_packet_entity_update_attributes::Ty {
			entity_id: 1,
			properties: vec![a::play_to_client_packet_entity_update_attributes_properties_items::Ty {
				key: a::play_to_client_packet_entity_update_attributes_properties_items_key::Ty::GenericMovementSpeed,
				value: 0.1,
				modifiers: vec![a::play_to_client_packet_entity_update_attributes_properties_items_modifiers_items::Ty {
					uuid: 0x662a6b8dda3e4c1c_881396ea6097278d,
					amount: 0.3,
					operation: 2,
				}],
			}],
		}),
		&mut Session::new(),
	);
	let [Q::EntityUpdateAttributes(p)] = out.as_slice() else {
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
	use a::play_to_client_packet::Ty as P;
	use d::play_to_client_packet::Ty as Q;
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
		&P::MapChunk(a::play_to_client_packet_map_chunk::Ty {
			x: 1,
			z: 2,
			heightmaps: tag(vec![]),
			chunk_data: data,
			block_entities: vec![a::chunk_block_entity::Ty {
				anon: a::chunk_block_entity_anon::Ty { x: 1, z: 2 },
				y: 64,
				r#type: 1,
				nbt_data: None,
			}],
			sky_light_mask: vec![],
			block_light_mask: vec![],
			empty_sky_light_mask: vec![],
			empty_block_light_mask: vec![],
			sky_light: vec![],
			block_light: vec![],
		}),
		&mut Session::new(),
	);
	let [Q::MapChunk(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(c.block_entities[0].r#type, 1);
	let d = &c.chunk_data;
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
	use a::play_to_client_packet::Ty as P;
	use d::play_to_client_packet::Ty as Q;
	let mut cx = Session::new();
	let spawn = |entity: i32, ty: &str, data: i32| {
		P::SpawnEntity(a::play_to_client_packet_spawn_entity::Ty {
			entity_id: entity,
			object_uuid: entity as u128,
			r#type: id("1.20.5", "entities", ty),
			x: 0.0,
			y: 70.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			head_pitch: 0,
			object_data: data,
			velocity: a::vec3i16::Ty { x: 0, y: 0, z: 0 },
		})
	};
	let out = up(&spawn(10, "boat", 0), &mut cx);
	let [Q::SpawnEntity(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(s.r#type, id("1.21.4", "entities", "oak_boat"));
	let sand = state("1.20.5", "minecraft:sand");
	let out = up(&spawn(11, "falling_block", sand), &mut cx);
	let [Q::SpawnEntity(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(describe("1.21.4", s.object_data), "minecraft:sand");
	let entry = |key: u8, value: a::entity_metadata_entry_value::Ty| a::entity_metadata_entry::Ty {
		key,
		value,
	};
	let out = up(
		&P::EntityMetadata(a::play_to_client_packet_entity_metadata::Ty {
			entity_id: 10,
			metadata: vec![
				entry(0, a::entity_metadata_entry_value::Ty::Byte(0)),
				entry(11, a::entity_metadata_entry_value::Ty::Int(0)),
				entry(12, a::entity_metadata_entry_value::Ty::Boolean(true)),
			],
		}),
		&mut cx,
	);
	let [Q::EntityMetadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let keys: Vec<u8> = m.metadata.iter().map(|e| e.key).collect();
	assert_eq!(keys, vec![0, 11]);
	assert!(matches!(
		m.metadata[1].value,
		d::entity_metadata_entry_value::Ty::Boolean(true)
	));
	let out = up(
		&P::SpawnEntity(a::play_to_client_packet_spawn_entity::Ty {
			entity_id: 12,
			object_uuid: 12,
			r#type: id("1.20.5", "entities", "painting"),
			x: 0.0,
			y: 0.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			head_pitch: 0,
			object_data: 2,
			velocity: a::vec3i16::Ty { x: 0, y: 0, z: 0 },
		}),
		&mut cx,
	);
	assert_eq!(out.len(), 1);
	let out = up(
		&P::EntityMetadata(a::play_to_client_packet_entity_metadata::Ty {
			entity_id: 12,
			metadata: vec![entry(
				8,
				a::entity_metadata_entry_value::Ty::PaintingVariant(0),
			)],
		}),
		&mut cx,
	);
	let [Q::EntityMetadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let d::entity_metadata_entry_value::Ty::PaintingVariant(
		d::entity_metadata_entry_value_painting_variant::Ty::Id(kebab),
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
	use a::play_to_client_packet::Ty as P;
	use d::play_to_client_packet::Ty as Q;
	let mut cx = Session::new();
	let particle = |particle| {
		P::WorldParticles(a::play_to_client_packet_world_particles::Ty {
			long_distance: false,
			x: 0.0,
			y: 0.0,
			z: 0.0,
			offset_x: 0.0,
			offset_y: 0.0,
			offset_z: 0.0,
			velocity_offset: 0.0,
			amount: 1,
			particle,
		})
	};
	let out = up(
		&particle(a::particle::Ty::Dust(a::particle_dust::Ty {
			red: 1.0,
			green: 0.5,
			blue: 0.0,
			scale: 1.0,
		})),
		&mut cx,
	);
	let [Q::WorldParticles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert!(
		matches!(&p.particle, d::particle::Ty::Dust(d) if d.color == (0xffu32 << 24 | 0xff << 16 | 127 << 8) as i32),
		"{p:?}"
	);
	let out = up(
		&particle(a::particle::Ty::Block(state(
			"1.20.5",
			"minecraft:oak_log[axis=x]",
		))),
		&mut cx,
	);
	let [Q::WorldParticles(p)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let d::particle::Ty::Block(s) = p.particle else {
		panic!()
	};
	assert_eq!(describe("1.21.4", s), "minecraft:oak_log[axis=x]");
	let sound = "entity.player.levelup";
	let out = up(
		&P::SoundEffect(a::play_to_client_packet_sound_effect::Ty {
			sound: a::item_sound_holder::Ty::Id(id("1.20.5", "sounds", sound)),
			sound_category: a::sound_source::Ty::Master,
			x: 0,
			y: 0,
			z: 0,
			volume: 1.0,
			pitch: 1.0,
			seed: 0,
		}),
		&mut cx,
	);
	let [Q::SoundEffect(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		s.sound,
		d::item_sound_holder::Ty::Id(id("1.21.4", "sounds", sound))
	);
}

fn sample_packets() -> Vec<a::play_to_client_packet::Ty> {
	use a::play_to_client_packet::Ty as P;
	let enchant = |name: &str, level: i32| a::slot_component_enchantments_enchantments_items::Ty {
		id: id("1.20.5", "enchantments", name),
		level,
	};
	let sword = item_a(
		"diamond_sword",
		vec![
			a::slot_component::Ty::Enchantments(a::slot_component_enchantments::Ty {
				enchantments: vec![enchant("sharpness", 2)],
				show_tooltip: true,
			}),
			a::slot_component::Ty::AttributeModifiers(a::slot_component_attribute_modifiers::Ty {
				attributes: vec![a::slot_component_attribute_modifiers_attributes_items::Ty {
					type_id: 2,
					uuid: 0xcb3f55d3645c4f38_a4979c13a33db5cf,
					name: "Weapon modifier".to_owned(),
					value: 7.0,
					operation:
						a::slot_component_attribute_modifiers_attributes_items_operation::Ty::Add,
					slot: a::slot_component_attribute_modifiers_attributes_items_slot::Ty::MainHand,
				}],
				show_tooltip: true,
			}),
			a::slot_component::Ty::CustomModelData(3),
			a::slot_component::Ty::FireResistant,
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
		P::SpawnEntity(a::play_to_client_packet_spawn_entity::Ty {
			entity_id: entity,
			object_uuid: entity as u128,
			r#type: id("1.20.5", "entities", ty),
			x: 0.5,
			y: 70.0,
			z: 0.5,
			pitch: 0,
			yaw: 64,
			head_pitch: 0,
			object_data: data,
			velocity: a::vec3i16::Ty { x: 0, y: 0, z: 0 },
		})
	};
	let entry = |key: u8, value: a::entity_metadata_entry_value::Ty| a::entity_metadata_entry::Ty {
		key,
		value,
	};
	let particles = |particle| {
		P::WorldParticles(a::play_to_client_packet_world_particles::Ty {
			long_distance: false,
			x: 0.0,
			y: 70.0,
			z: 0.0,
			offset_x: 0.1,
			offset_y: 0.1,
			offset_z: 0.1,
			velocity_offset: 0.0,
			amount: 3,
			particle,
		})
	};
	let sound = a::item_sound_holder::Ty::Id(id("1.20.5", "sounds", "entity.generic.explode"));
	let small = || a::particle::Ty::Explosion;
	vec![
		P::Position(a::play_to_client_packet_position::Ty {
			x: 0.5,
			y: 80.0,
			z: 0.5,
			yaw: 0.0,
			pitch: 0.0,
			flags: a::play_to_client_position_update_relatives::Ty { x: false, y: false, z: false, yaw: false, pitch: false },
			teleport_id: 1,
		}),
		P::MapChunk(a::play_to_client_packet_map_chunk::Ty {
			x: 0,
			z: 0,
			heightmaps: tag(vec![]),
			chunk_data: chunk,
			block_entities: vec![a::chunk_block_entity::Ty { anon: a::chunk_block_entity_anon::Ty { x: 1, z: 1 }, y: 0, r#type: 7, nbt_data: None }],
			sky_light_mask: vec![],
			block_light_mask: vec![],
			empty_sky_light_mask: vec![],
			empty_block_light_mask: vec![],
			sky_light: vec![],
			block_light: vec![],
		}),
		P::BlockChange(a::play_to_client_packet_block_change::Ty { location: pos(1, 70, 1), r#type: state("1.20.5", "minecraft:oak_stairs[facing=east,half=top,shape=straight,waterlogged=true]") }),
		P::MultiBlockChange(a::play_to_client_packet_multi_block_change::Ty {
			chunk_coordinates: a::play_to_client_packet_multi_block_change_chunk_coordinates::Ty { x: 0, z: 0, y: 4 },
			records: vec![(stone << 12) | 0x123],
		}),
		spawn(10, "boat", 0),
		spawn(11, "falling_block", state("1.20.5", "minecraft:sand")),
		spawn(12, "zombie", 0),
		P::EntityMetadata(a::play_to_client_packet_entity_metadata::Ty {
			entity_id: 10,
			metadata: vec![entry(11, a::entity_metadata_entry_value::Ty::Int(0)), entry(12, a::entity_metadata_entry_value::Ty::Boolean(false))],
		}),
		P::EntityMetadata(a::play_to_client_packet_entity_metadata::Ty {
			entity_id: 12,
			metadata: vec![entry(0, a::entity_metadata_entry_value::Ty::Byte(0)), entry(9, a::entity_metadata_entry_value::Ty::Float(20.0))],
		}),
		P::EntityEquipment(a::play_to_client_packet_entity_equipment::Ty {
			entity_id: 12,
			equipments: vec![a::play_to_client_packet_entity_equipment_equipments_items::Ty { slot: 0, item: sword.clone() }],
		}),
		P::WindowItems(a::play_to_client_packet_window_items::Ty {
			window_id: 0,
			state_id: 1,
			items: vec![sword.clone(), a::slot::Ty::V0, item_a("apple", vec![])],
			carried_item: a::slot::Ty::V0,
		}),
		P::SetSlot(a::play_to_client_packet_set_slot::Ty { window_id: 255, state_id: 2, slot: -1, item: item_a("stone", vec![]) }),
		particles(a::particle::Ty::Dust(a::particle_dust::Ty { red: 1.0, green: 0.0, blue: 0.0, scale: 1.0 })),
		particles(a::particle::Ty::Block(stone)),
		particles(a::particle::Ty::Item(sword)),
		P::SoundEffect(a::play_to_client_packet_sound_effect::Ty { sound: sound.clone(), sound_category: a::sound_source::Ty::Block, x: 0, y: 560, z: 0, volume: 1.0, pitch: 1.0, seed: 1 }),
		P::Explosion(a::play_to_client_packet_explosion::Ty {
			x: 0.0,
			y: 70.0,
			z: 0.0,
			radius: 4.0,
			affected_block_offsets: vec![a::play_to_client_packet_explosion_affected_block_offsets_items::Ty { x: 1, y: 0, z: -1 }],
			player_motion_x: 0.1,
			player_motion_y: 0.2,
			player_motion_z: 0.0,
			block_interaction_type: 1,
			small_explosion_particle: small(),
			large_explosion_particle: a::particle::Ty::ExplosionEmitter,
			sound,
		}),
		P::UpdateTime(a::play_to_client_packet_update_time::Ty { age: 100, time: -6000 }),
		P::EntityUpdateAttributes(a::play_to_client_packet_entity_update_attributes::Ty {
			entity_id: 12,
			properties: vec![a::play_to_client_packet_entity_update_attributes_properties_items::Ty {
				key: a::play_to_client_packet_entity_update_attributes_properties_items_key::Ty::GenericMaxHealth,
				value: 20.0,
				modifiers: vec![],
			}],
		}),
		P::SetCooldown(a::play_to_client_packet_set_cooldown::Ty { item_id: id("1.20.5", "items", "ender_pearl"), cooldown_ticks: 20 }),
		P::EntityTeleport(a::play_to_client_packet_entity_teleport::Ty { entity_id: 12, x: 1.0, y: 70.0, z: 1.0, yaw: 32, pitch: 0, on_ground: true }),
		P::WorldEvent(a::play_to_client_packet_world_event::Ty { effect_id: 2001, location: pos(0, 70, 0), data: stone, global: false }),
		P::WorldEvent(a::play_to_client_packet_world_event::Ty { effect_id: 1010, location: pos(0, 70, 0), data: id("1.20.5", "items", "music_disc_cat"), global: false }),
		P::TileEntityData(a::play_to_client_packet_tile_entity_data::Ty { location: pos(1, 0, 1), action: 7, nbt_data: Some(tag(vec![])) }),
		P::SetPassengers(a::play_to_client_packet_set_passengers::Ty { entity_id: 10, passengers: vec![12] }),
		P::EntityDestroy(vec![10, 11]),
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
			a::play_to_client_packet::serialize(p, &mut o).unwrap();
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
	use a::configuration_to_client_packet::Ty as C;
	let configuration = [
		C::FeatureFlags(vec!["minecraft:vanilla".to_owned()]),
		C::Tags(vec![a::configuration_to_client_packet_tags_items::Ty {
			tag_type: "minecraft:item".to_owned(),
			tags: vec![
				a::configuration_to_client_packet_tags_items_tags_items::Ty {
					tag_name: "minecraft:logs".to_owned(),
					entries: vec![id("1.20.5", "items", "oak_log")],
				},
			],
		}]),
		C::FinishConfiguration(a::configuration_to_client_packet_finish_configuration::Ty {}),
	];
	for to in ["1.21.1", "1.21.3", "1.21.4"] {
		let mut cx = Session::new();
		let mut out = Vec::new();
		for p in &configuration {
			let mut o = Output::new();
			a::configuration_to_client_packet::serialize(p, &mut o).unwrap();
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
	use d::play_to_server_packet::Ty as S;
	let flags = |on_ground| d::play_to_server_movement_flags::Ty {
		on_ground: on_ground,
		has_horizontal_collision: false,
	};
	let slot = d::slot::Ty::Default(
		1,
		d::slot_default::Ty {
			item_id: id("1.21.4", "items", "pale_oak_log"),
			added_component_count: 1,
			removed_component_count: 0,
			components: vec![d::slot_component::Ty::ItemModel(
				"minecraft:stone".to_owned(),
			)],
			remove_components: vec![],
		},
	);
	let server = [
		S::PositionLook(d::play_to_server_packet_position_look::Ty {
			x: 0.5,
			y: 70.0,
			z: 0.5,
			yaw: 0.0,
			pitch: 0.0,
			flags: flags(true),
		}),
		S::TickEnd(d::play_to_server_packet_tick_end::Ty {}),
		S::TickEnd(d::play_to_server_packet_tick_end::Ty {}),
		S::Look(d::play_to_server_packet_look::Ty {
			yaw: 90.0,
			pitch: 5.0,
			flags: flags(false),
		}),
		S::Flying(flags(true)),
		S::PlayerInput(d::play_to_server_packet_player_input::Ty {
			forward: true,
			backward: false,
			left: false,
			right: false,
			jump: false,
			shift: false,
			sprint: false,
		}),
		S::SetCreativeSlot(d::play_to_server_packet_set_creative_slot::Ty {
			slot: 36,
			item: slot.clone(),
		}),
		S::WindowClick(d::play_to_server_packet_window_click::Ty {
			window_id: 0,
			state_id: 3,
			slot: 36,
			mouse_button: 0,
			mode: 0,
			changed_slots: vec![
				d::play_to_server_packet_window_click_changed_slots_items::Ty {
					location: 36,
					item: d::slot::Ty::V0,
				},
			],
			cursor_item: slot,
		}),
		S::BlockPlace(d::play_to_server_packet_block_place::Ty {
			hand: 0,
			location: d::position::Ty { x: 1, y: 64, z: 1 },
			direction: 1,
			cursor_x: 0.5,
			cursor_y: 1.0,
			cursor_z: 0.5,
			inside_block: false,
			world_border_hit: false,
			sequence: 4,
		}),
		S::UseItem(d::play_to_server_packet_use_item::Ty {
			hand: 0,
			sequence: 5,
			rotation: d::vec2f::Ty { x: 90.0, y: 5.0 },
		}),
		S::Settings(d::packet_common_settings::Ty {
			locale: "en_us".to_owned(),
			view_distance: 8,
			chat_flags: 0,
			chat_colors: true,
			skin_parts: 127,
			main_hand: 1,
			enable_text_filtering: false,
			enable_server_listing: true,
			particle_status: d::packet_common_settings_particle_status::Ty::All,
		}),
		S::CloseWindow(0),
	];
	for to in ["1.21.4", "1.21.3", "1.21.1", "1.20.5"] {
		let mut cx = Session::new();
		let mut out = Vec::new();
		for p in &server {
			let mut o = Output::new();
			d::play_to_server_packet::serialize(p, &mut o).unwrap();
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
			d::play_to_client_packet::serialize,
		);
	}
	use a::play_to_server_packet::Ty as O;
	let old = [
		O::Position(a::play_to_server_packet_position::Ty {
			x: 0.5,
			y: 70.0,
			z: 0.5,
			on_ground: true,
		}),
		O::Look(a::play_to_server_packet_look::Ty {
			yaw: 10.0,
			pitch: 0.0,
			on_ground: true,
		}),
		O::Flying(true),
		O::SteerVehicle(a::play_to_server_packet_steer_vehicle::Ty {
			sideways: 0.98,
			forward: 0.0,
			jump: 3,
		}),
		O::CraftRecipeRequest(a::play_to_server_packet_craft_recipe_request::Ty {
			window_id: 1,
			recipe: "minecraft:7".to_owned(),
			make_all: true,
		}),
		O::DisplayedRecipe("minecraft:7".to_owned()),
		O::UseItem(a::play_to_server_packet_use_item::Ty {
			hand: 0,
			sequence: 2,
		}),
		O::SetCreativeSlot(a::play_to_server_packet_set_creative_slot::Ty {
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
			a::play_to_server_packet::serialize,
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

fn reverse_samples() -> Vec<d::play_to_client_packet::Ty> {
	use d::play_to_client_packet::Ty as C;
	use d::play_to_client_slot_display::Ty as S;
	let flags = |relative: bool| d::play_to_client_position_update_relatives::Ty {
		x: relative,
		y: relative,
		z: relative,
		yaw: false,
		pitch: false,
		dx: relative,
		dy: relative,
		dz: relative,
		yaw_delta: false,
	};
	let beef = item_d(
		"cooked_beef",
		vec![
			d::slot_component::Ty::Food(d::slot_component_food::Ty {
				nutrition: 8,
				saturation_modifier: 12.8,
				can_always_eat: false,
			}),
			d::slot_component::Ty::Consumable(d::slot_component_consumable::Ty {
				consume_seconds: 0.8,
				animation: d::slot_component_consumable_animation::Ty::Eat,
				sound: d::item_sound_holder::Ty::Id(id("1.21.4", "sounds", "entity.generic.eat")),
				makes_particles: true,
				effects: vec![d::item_consume_effect::Ty::ApplyEffects(
					d::item_consume_effect_apply_effects::Ty {
						effects: vec![potion("Hunger")],
						probability: 0.25,
					},
				)],
			}),
			d::slot_component::Ty::UseRemainder(item_d("bowl", vec![])),
			d::slot_component::Ty::CustomModelData(d::slot_component_custom_model_data::Ty {
				floats: vec![3.0],
				flags: vec![],
				strings: vec![],
				colors: vec![],
			}),
		],
	);
	let decoration = d::play_to_client_chat_type::Ty {
		translation_key: "chat.type.text".to_owned(),
		parameters: vec![
			d::play_to_client_chat_type_parameter_type::Ty::Sender,
			d::play_to_client_chat_type_parameter_type::Ty::Content,
		],
		style: tag(vec![]),
	};
	vec![
		C::Login(d::play_to_client_packet_login::Ty {
			entity_id: 42,
			is_hardcore: false,
			world_names: vec!["minecraft:overworld".to_owned()],
			max_players: 20,
			view_distance: 8,
			simulation_distance: 8,
			reduced_debug_info: false,
			enable_respawn_screen: true,
			do_limited_crafting: false,
			world_state: d::play_to_client_spawn_info::Ty {
				dimension: 0,
				name: "minecraft:overworld".to_owned(),
				hashed_seed: 0,
				gamemode: d::play_to_client_spawn_info_gamemode::Ty::Survival,
				previous_gamemode: 255,
				is_debug: false,
				is_flat: false,
				death: None,
				portal_cooldown: 0,
				sea_level: 63,
			},
			enforces_secure_chat: false,
		}),
		C::Position(d::play_to_client_packet_position::Ty { teleport_id: 1, x: 0.5, y: 70.0, z: 0.5, dx: 0.0, dy: 0.0, dz: 0.0, yaw: 0.0, pitch: 0.0, flags: flags(false) }),
		C::Position(d::play_to_client_packet_position::Ty { teleport_id: 2, x: 1.0, y: 0.0, z: 0.0, dx: 0.1, dy: 0.2, dz: 0.0, yaw: 0.0, pitch: 0.0, flags: flags(true) }),
		C::EntityTeleport(d::play_to_client_packet_entity_teleport::Ty {
			entity_id: 7,
			x: 5.0,
			y: 64.0,
			z: 5.0,
			dx: 0.0,
			dy: 0.0,
			dz: 0.0,
			yaw: 90.0,
			pitch: 0.0,
			flags: flags(false),
			on_ground: true,
		}),
		C::SyncEntityPosition(d::play_to_client_packet_sync_entity_position::Ty { entity_id: 7, x: 5.0, y: 64.0, z: 5.0, dx: 0.0, dy: 0.0, dz: 0.0, yaw: 0.0, pitch: 0.0, on_ground: true }),
		C::PlayerRotation(d::play_to_client_packet_player_rotation::Ty { yaw: 45.0, pitch: 10.0 }),
		C::Explosion(d::play_to_client_packet_explosion::Ty {
			x: 1.0,
			y: 2.0,
			z: 3.0,
			player_knockback: Some(d::vec3f64::Ty { x: 0.25, y: 0.5, z: 0.0 }),
			explosion_particle: d::particle::Ty::Explosion,
			sound: d::item_sound_holder::Ty::Id(id("1.21.4", "sounds", "entity.generic.explode")),
		}),
		C::SpawnEntity(d::play_to_client_packet_spawn_entity::Ty {
			entity_id: 9,
			object_uuid: 9,
			r#type: id("1.21.4", "entities", "spruce_boat"),
			x: 0.0,
			y: 64.0,
			z: 0.0,
			pitch: 0,
			yaw: 0,
			head_pitch: 0,
			object_data: 0,
			velocity: d::vec3i16::Ty { x: 0, y: 0, z: 0 },
		}),
		C::DeclareRecipes(d::play_to_client_packet_declare_recipes::Ty {
			recipes: vec![d::play_to_client_packet_declare_recipes_recipes_items::Ty { name: "minecraft:furnace_input".to_owned(), items: vec![id("1.21.4", "items", "beef")] }],
			stone_cutter_recipes: vec![d::play_to_client_packet_declare_recipes_stone_cutter_recipes_items::Ty {
				input: d::id_set::Ty::Ids(vec![id("1.21.4", "items", "stone")]),
				slot_display: S::ItemStack(item_d("stone_slab", vec![])),
			}],
		}),
		C::RecipeBookSettings(d::play_to_client_packet_recipe_book_settings::Ty {
			crafting_gui_open: true,
			crafting_filtering_craftable: false,
			smelting_gui_open: false,
			smelting_filtering_craftable: false,
			blast_gui_open: false,
			blast_filtering_craftable: false,
			smoker_gui_open: false,
			smoker_filtering_craftable: false,
		}),
		C::RecipeBookAdd(d::play_to_client_packet_recipe_book_add::Ty {
			entries: vec![d::play_to_client_packet_recipe_book_add_entries_items::Ty {
				recipe: d::play_to_client_packet_recipe_book_add_entries_items_recipe::Ty {
					display_id: 7,
					display: d::play_to_client_recipe_display::Ty::CraftingShapeless(d::play_to_client_recipe_display_crafting_shapeless::Ty {
						ingredients: vec![S::Item(id("1.21.4", "items", "oak_log"))],
						result: S::ItemStack(item_d("oak_planks", vec![])),
						crafting_station: S::Item(id("1.21.4", "items", "crafting_table")),
					}),
					group: 0,
					category: d::play_to_client_packet_recipe_book_add_entries_items_recipe_category::Ty::CraftingBuildingBlocks,
					crafting_requirements: None,
				},
				flags: d::play_to_client_packet_recipe_book_add_entries_items_flags::Ty { notification: true, highlight: false },
			}],
			replace: true,
		}),
		C::RecipeBookRemove(vec![7]),
		C::SetSlot(d::play_to_client_packet_set_slot::Ty { window_id: 0, state_id: 1, slot: 5, item: beef.clone() }),
		C::SetCursorItem(item_d("pale_oak_log", vec![])),
		C::SetPlayerInventory(d::play_to_client_packet_set_player_inventory::Ty { slot_id: 3, contents: beef.clone() }),
		C::WindowItems(d::play_to_client_packet_window_items::Ty { window_id: 0, state_id: 2, items: vec![beef, d::slot::Ty::V0], carried_item: d::slot::Ty::V0 }),
		C::ProfilelessChat(d::play_to_client_packet_profileless_chat::Ty {
			message: string_tag("hi"),
			r#type: d::play_to_client_chat_types_holder::Ty::Inline(d::play_to_client_chat_types::Ty { chat: decoration.clone(), narration: decoration }),
			name: string_tag("Bob"),
			target: None,
		}),
		C::ProfilelessChat(d::play_to_client_packet_profileless_chat::Ty {
			message: string_tag("hi"),
			r#type: d::play_to_client_chat_types_holder::Ty::Id(0),
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
		"SetCursorItem",
		"SyncEntityPosition",
		"Explosion",
		"BlockChange",
		"MapChunk",
		"WorldEvent",
	] {
		assert!(
			kinds.iter().any(|k| k == kind),
			"{kind} missing in {kinds:?}"
		);
	}
}

#[test]
fn movement_goes_down_with_ticks() {
	use a::play_to_server_packet::Ty as P;
	use d::play_to_server_packet::Ty as Q;
	let mut cx = Session::new();
	let flags = |on_ground| d::play_to_server_movement_flags::Ty {
		on_ground: on_ground,
		has_horizontal_collision: false,
	};
	let out = down(
		&Q::PositionLook(d::play_to_server_packet_position_look::Ty {
			x: 1.0,
			y: 2.0,
			z: 3.0,
			yaw: 4.0,
			pitch: 5.0,
			flags: flags(true),
		}),
		&mut cx,
	);
	assert!(matches!(out.as_slice(), [P::PositionLook(p)] if p.on_ground && p.yaw == 4.0));
	let tick = Q::TickEnd(d::play_to_server_packet_tick_end::Ty {});
	assert!(down(&tick, &mut cx).is_empty(), "the tick already moved");
	assert!(matches!(down(&tick, &mut cx).as_slice(), [P::Flying(true)]));
	let out = down(&Q::Flying(flags(false)), &mut cx);
	assert!(matches!(out.as_slice(), [P::Flying(false)]));
	let input = d::play_to_server_packet_player_input::Ty {
		forward: true,
		backward: false,
		left: false,
		right: false,
		jump: false,
		shift: true,
		sprint: false,
	};
	assert!(
		down(&Q::PlayerInput(input.clone()), &mut cx).is_empty(),
		"not riding"
	);

	use a::play_to_client_packet::Ty as C;
	let login = C::Login(a::play_to_client_packet_login::Ty {
		entity_id: 42,
		is_hardcore: false,
		world_names: vec!["minecraft:overworld".to_owned()],
		max_players: 20,
		view_distance: 8,
		simulation_distance: 8,
		reduced_debug_info: false,
		enable_respawn_screen: true,
		do_limited_crafting: false,
		world_state: a::play_to_client_spawn_info::Ty {
			dimension: 0,
			name: "minecraft:overworld".to_owned(),
			hashed_seed: 0,
			gamemode: a::play_to_client_spawn_info_gamemode::Ty::Survival,
			previous_gamemode: 255,
			is_debug: false,
			is_flat: false,
			death: None,
			portal_cooldown: 0,
		},
		enforces_secure_chat: false,
	});
	let out = up(&login, &mut cx);
	let [d::play_to_client_packet::Ty::Login(l)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(l.world_state.sea_level, 63);
	up(
		&C::SetPassengers(a::play_to_client_packet_set_passengers::Ty {
			entity_id: 7,
			passengers: vec![42],
		}),
		&mut cx,
	);
	let out = down(&Q::PlayerInput(input), &mut cx);
	assert!(
		matches!(out.as_slice(), [P::SteerVehicle(s)] if s.forward > 0.9 && s.sideways == 0.0 && s.jump == 2),
		"{out:?}"
	);
	let out = down(&tick, &mut cx);
	assert!(
		matches!(out.as_slice(), [P::Look(_), P::SteerVehicle(_)]),
		"{out:?}"
	);
}

fn client_down(
	p: &d::play_to_client_packet::Ty,
	cx: &mut Session,
) -> Vec<a::play_to_client_packet::Ty> {
	run(
		Root::PlayToClient,
		("1.21.4", "1.20.5"),
		p,
		d::play_to_client_packet::serialize,
		a::play_to_client_packet::deserialize,
		cx,
	)
}

fn server_up(
	p: &a::play_to_server_packet::Ty,
	cx: &mut Session,
) -> Vec<d::play_to_server_packet::Ty> {
	run(
		Root::PlayToServer,
		("1.20.5", "1.21.4"),
		p,
		a::play_to_server_packet::serialize,
		d::play_to_server_packet::deserialize,
		cx,
	)
}

fn config_down(
	p: &d::configuration_to_client_packet::Ty,
	cx: &mut Session,
) -> Vec<a::configuration_to_client_packet::Ty> {
	run(
		Root::ConfigurationToClient,
		("1.21.4", "1.20.5"),
		p,
		d::configuration_to_client_packet::serialize,
		a::configuration_to_client_packet::deserialize,
		cx,
	)
}

fn text(name: &str, value: &str) -> nbt::gen::entry::Ty {
	nbt::gen::entry::Ty::new(
		name.to_owned(),
		nbt::gen::payload::Ty::String(value.to_owned()),
	)
}

fn string_tag(value: &str) -> nbt::gen::tag::Ty {
	nbt::gen::tag::Ty::new(nbt::gen::payload::Ty::String(value.to_owned()))
}

fn compound(tag: &nbt::Tag) -> &[nbt::Entry] {
	match &tag.value {
		nbt::gen::payload::Ty::Compound(e) => e,
		other => panic!("{other:?}"),
	}
}

fn field<'a>(entries: &'a [nbt::Entry], name: &str) -> &'a nbt::gen::payload::Ty {
	&entries
		.iter()
		.find(|e| e.name == name)
		.unwrap_or_else(|| panic!("{name} missing"))
		.value
}

fn item_d(name: &str, components: Vec<d::slot_component::Ty>) -> d::slot::Ty {
	d::slot::Ty::Default(
		1,
		d::slot_default::Ty {
			item_id: id("1.21.4", "items", name),
			added_component_count: components.len() as i32,
			removed_component_count: 0,
			components,
			remove_components: vec![],
		},
	)
}

fn potion(effect: &str) -> d::item_potion_effect::Ty {
	d::item_potion_effect::Ty {
		id: id("1.21.4", "effects", effect),
		details: d::item_effect_detail::Ty {
			amplifier: 0,
			duration: 600,
			ambient: false,
			show_particles: true,
			show_icon: true,
			hidden_effect: None,
		},
	}
}

#[test]
fn registries_rebuilt_from_server_going_down() {
	use a::configuration_to_client_packet::Ty as P;
	use d::configuration_to_client_packet::Ty as Q;
	let mut cx = Session::new();
	let entry =
		|key: &str, value| d::configuration_to_client_packet_registry_data_entries_items::Ty {
			key: key.to_owned(),
			value,
		};
	let registry = |id: &str, entries| {
		Q::RegistryData(d::configuration_to_client_packet_registry_data::Ty {
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
	let [P::RegistryData(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let fields = compound(r.entries[0].value.as_ref().unwrap());
	assert_eq!(
		field(fields, "asset_name"),
		&nbt::Payload::String("gold".to_owned())
	);
	assert_eq!(
		field(fields, "ingredient"),
		&nbt::Payload::String("minecraft:copper_ingot".to_owned())
	);
	assert!(matches!(
		field(fields, "item_model_index"),
		nbt::gen::payload::Ty::Float(_)
	));
	let music = nbt::gen::entry::Ty::new(
		"music",
		nbt::gen::payload::Ty::List(nbt::gen::list::Ty::new(nbt::gen::list_0::Ty::Compound(
			vec![vec![
				nbt::gen::entry::Ty::new(
					"data",
					nbt::gen::payload::Ty::Compound(vec![text("sound", "minecraft:music.game")]),
				),
				nbt::gen::entry::Ty::new("weight", nbt::gen::payload::Ty::Int(1)),
			]],
		))),
	);
	let dust = nbt::gen::entry::Ty::new(
		"particle",
		nbt::gen::payload::Ty::Compound(vec![nbt::Entry::new(
			"options",
			nbt::gen::payload::Ty::Compound(vec![
				text("type", "minecraft:dust"),
				nbt::gen::entry::Ty::new("color", nbt::gen::payload::Ty::Int(0xff0000)),
				nbt::gen::entry::Ty::new("scale", nbt::gen::payload::Ty::Float(1.0)),
			]),
		)]),
	);
	let effects = nbt::gen::entry::Ty::new(
		"effects",
		nbt::gen::payload::Ty::Compound(vec![music, dust]),
	);
	let out = config_down(
		&registry(
			"minecraft:worldgen/biome",
			vec![entry("minecraft:plains", Some(tag(vec![effects])))],
		),
		&mut cx,
	);
	let [P::RegistryData(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let nbt::gen::payload::Ty::Compound(effects) =
		field(compound(r.entries[0].value.as_ref().unwrap()), "effects")
	else {
		panic!()
	};
	assert!(
		matches!(field(effects, "music"), nbt::gen::payload::Ty::Compound(m) if m.len() == 1),
		"music unwrapped"
	);
	let nbt::gen::payload::Ty::Compound(particle) = field(effects, "particle") else {
		panic!()
	};
	let nbt::gen::payload::Ty::Compound(options) = field(particle, "options") else {
		panic!()
	};
	assert!(
		matches!(field(options, "color"), nbt::gen::payload::Ty::List(nbt::gen::list::Ty { items: nbt::gen::list_0::Ty::Float(c), .. }) if c == &[1.0, 0.0, 0.0]),
		"dust color is a float list again"
	);

	use a::play_to_client_packet::Ty as A;
	use d::play_to_client_packet::Ty as C;
	let painting = C::EntityMetadata(d::play_to_client_packet_entity_metadata::Ty {
		entity_id: 5,
		metadata: vec![d::entity_metadata_entry::Ty {
			key: 8,
			value: d::entity_metadata_entry_value::Ty::PaintingVariant(
				d::entity_metadata_entry_value_painting_variant::Ty::Id(0),
			),
		}],
	});
	let out = client_down(&painting, &mut cx);
	let [A::EntityMetadata(m)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert!(
		matches!(
			&m.metadata[0].value,
			a::entity_metadata_entry_value::Ty::PaintingVariant(21)
		),
		"server painting 0 is pointer: {m:?}"
	);
	let sword = item_d(
		"diamond_sword",
		vec![d::slot_component::Ty::Enchantments(
			d::slot_component_enchantments::Ty {
				enchantments: vec![d::slot_component_enchantments_enchantments_items::Ty {
					id: 1,
					level: 5,
				}],
				show_tooltip: true,
			},
		)],
	);
	let out = client_down(
		&C::SetSlot(d::play_to_client_packet_set_slot::Ty {
			window_id: 0,
			state_id: 1,
			slot: 36,
			item: sword,
		}),
		&mut cx,
	);
	let [A::SetSlot(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let a::slot::Ty::Default(_, item) = &s.item else {
		panic!()
	};
	let [a::slot_component::Ty::Enchantments(e)] = item.components.as_slice() else {
		panic!("{item:?}")
	};
	assert_eq!(
		e.enchantments[0].id,
		id("1.20.5", "enchantments", "sharpness")
	);
}

#[test]
fn signed_inline_chat_keeps_the_registered_type() {
	use d::play_to_client_chat_type_parameter_type::Ty as Param;
	let mut cx = Session::new();
	let entry = |key: &str| d::configuration_to_client_packet_registry_data_entries_items::Ty {
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
	let types = d::configuration_to_client_packet::Ty::RegistryData(
		d::configuration_to_client_packet_registry_data::Ty {
			id: "minecraft:chat_type".to_owned(),
			entries: keys.iter().map(|k| entry(k)).collect(),
		},
	);
	assert_eq!(config_down(&types, &mut cx).len(), 1);
	let signed = |key: &str| {
		let decoration = d::play_to_client_chat_type::Ty {
			translation_key: key.to_owned(),
			parameters: vec![Param::Sender, Param::Content],
			style: tag(vec![]),
		};
		d::play_to_client_packet::Ty::PlayerChat(d::play_to_client_packet_player_chat::Ty {
			sender_uuid: 1,
			index: 0,
			signature: Some(vec![7; 256]),
			plain_message: "hi".to_owned(),
			timestamp: 0,
			salt: 0,
			previous_messages: vec![],
			unsigned_chat_content: None,
			filter_type_mask: d::play_to_client_packet_player_chat_filter_type_mask::Ty::Default(0),
			r#type: d::play_to_client_chat_types_holder::Ty::Inline(
				d::play_to_client_chat_types::Ty {
					chat: decoration.clone(),
					narration: decoration,
				},
			),
			network_name: string_tag("Bob"),
			network_target_name: None,
		})
	};
	for (key, id) in [
		("chat.type.announcement", 4),
		("chat.type.emote", 1),
		("custom.key", 0),
	] {
		let out = client_down(&signed(key), &mut cx);
		let [a::play_to_client_packet::Ty::PlayerChat(p)] = out.as_slice() else {
			panic!("{out:?}")
		};
		assert_eq!(p.r#type, id, "{key}");
	}
}

#[test]
fn world_and_recipes_going_down() {
	use a::play_to_client_packet::Ty as A;
	use d::play_to_client_packet::Ty as C;
	let mut cx = Session::new();
	let login = C::Login(d::play_to_client_packet_login::Ty {
		entity_id: 42,
		is_hardcore: false,
		world_names: vec!["minecraft:overworld".to_owned()],
		max_players: 20,
		view_distance: 8,
		simulation_distance: 8,
		reduced_debug_info: false,
		enable_respawn_screen: true,
		do_limited_crafting: false,
		world_state: d::play_to_client_spawn_info::Ty {
			dimension: 0,
			name: "minecraft:overworld".to_owned(),
			hashed_seed: 0,
			gamemode: d::play_to_client_spawn_info_gamemode::Ty::Survival,
			previous_gamemode: 255,
			is_debug: false,
			is_flat: false,
			death: None,
			portal_cooldown: 0,
			sea_level: 63,
		},
		enforces_secure_chat: false,
	});
	let out = client_down(&login, &mut cx);
	assert!(
		matches!(out.as_slice(), [A::Login(l)] if l.entity_id == 42),
		"{out:?}"
	);
	let queued = cx.take_outgoing();
	let [loaded] = queued.as_slice() else {
		panic!("player_loaded not queued")
	};
	assert_eq!(loaded.version, at("1.21.4"));
	let mut input = Input::new(&loaded.data);
	assert!(matches!(
		d::play_to_server_packet::deserialize(&mut input).unwrap(),
		d::play_to_server_packet::Ty::PlayerLoaded(_)
	));

	let flags = |relative: bool, delta: bool| d::play_to_client_position_update_relatives::Ty {
		x: relative,
		y: relative,
		z: relative,
		yaw: false,
		pitch: false,
		dx: delta,
		dy: delta,
		dz: delta,
		yaw_delta: false,
	};
	let position = |x: f64, relative: bool, delta: (f64, bool)| {
		C::Position(d::play_to_client_packet_position::Ty {
			teleport_id: 1,
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
		matches!(out.as_slice(), [A::Position(p), A::EntityVelocity(_)] if p.x == 10.0),
		"{out:?}"
	);
	let out = client_down(&position(1.0, true, (0.5, true)), &mut cx);
	assert!(
		matches!(out.as_slice(), [A::Position(p), A::Explosion(e)] if p.flags.x && e.player_motion_x == 0.5),
		"{out:?}"
	);
	let teleport = |entity: i32| {
		C::EntityTeleport(d::play_to_client_packet_entity_teleport::Ty {
			entity_id: entity,
			x: 2.0,
			y: 0.0,
			z: 0.0,
			dx: 0.0,
			dy: 0.0,
			dz: 0.0,
			yaw: 0.0,
			pitch: 0.0,
			flags: flags(true, true),
			on_ground: true,
		})
	};
	let out = client_down(&teleport(42), &mut cx);
	assert!(
		matches!(out.as_slice(), [A::EntityTeleport(t)] if t.x == 13.0 && t.y == 128.0),
		"relative to tracked player: {out:?}"
	);
	assert!(
		client_down(&teleport(7), &mut cx).is_empty(),
		"relative teleport of unknown entity"
	);

	let sound = |v: &str| id(v, "sounds", "entity.generic.explode");
	let explosion = C::Explosion(d::play_to_client_packet_explosion::Ty {
		x: 1.0,
		y: 2.0,
		z: 3.0,
		player_knockback: Some(d::vec3f64::Ty {
			x: 0.25,
			y: 0.5,
			z: 0.0,
		}),
		explosion_particle: d::particle::Ty::Explosion,
		sound: d::item_sound_holder::Ty::Id(sound("1.21.4")),
	});
	let out = client_down(&explosion, &mut cx);
	let [A::Explosion(e)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		(e.player_motion_x, e.player_motion_y, e.radius),
		(0.25, 0.5, 0.0)
	);
	assert!(matches!(
		e.small_explosion_particle,
		a::particle::Ty::Explosion
	));
	assert_eq!(e.sound, a::item_sound_holder::Ty::Id(sound("1.20.5")));

	let stone = id("1.21.4", "items", "stone");
	let declare = C::DeclareRecipes(d::play_to_client_packet_declare_recipes::Ty {
		recipes: vec![],
		stone_cutter_recipes: vec![
			d::play_to_client_packet_declare_recipes_stone_cutter_recipes_items::Ty {
				input: d::id_set::Ty::Ids(vec![stone]),
				slot_display: d::play_to_client_slot_display::Ty::ItemStack(item_d(
					"stone_slab",
					vec![],
				)),
			},
		],
	});
	let out = client_down(&declare, &mut cx);
	let [A::DeclareRecipes(r)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert!(
		matches!(&r[0].data, a::play_to_client_packet_declare_recipes_items_data::Ty::MinecraftStonecutting(s) if s.ingredient.len() == 1)
	);
	use d::play_to_client_slot_display::Ty as S;
	let book = C::RecipeBookAdd(d::play_to_client_packet_recipe_book_add::Ty {
		entries: vec![d::play_to_client_packet_recipe_book_add_entries_items::Ty {
			recipe: d::play_to_client_packet_recipe_book_add_entries_items_recipe::Ty {
				display_id: 7,
				display: d::play_to_client_recipe_display::Ty::CraftingShapeless(d::play_to_client_recipe_display_crafting_shapeless::Ty {
					ingredients: vec![S::Item(id("1.21.4", "items", "oak_log"))],
					result: S::ItemStack(item_d("oak_planks", vec![])),
					crafting_station: S::Item(id("1.21.4", "items", "crafting_table")),
				}),
				group: 0,
				category: d::play_to_client_packet_recipe_book_add_entries_items_recipe_category::Ty::CraftingBuildingBlocks,
				crafting_requirements: None,
			},
			flags: d::play_to_client_packet_recipe_book_add_entries_items_flags::Ty { notification: true, highlight: true },
		}],
		replace: true,
	});
	let out = client_down(&book, &mut cx);
	let [A::DeclareRecipes(r), A::UnlockRecipes(u)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(r.len(), 2, "stonecutter recipe kept alongside the book");
	assert!(
		matches!(&r[1].data, a::play_to_client_packet_declare_recipes_items_data::Ty::MinecraftCraftingShapeless(s) if s.ingredients.len() == 1)
	);
	assert_eq!(u.recipes1, vec!["minecraft:7".to_owned()]);
	assert!(
		matches!(&u.recipes2, a::play_to_client_packet_unlock_recipes_recipes2::Ty::V0(h) if h == &u.recipes1)
	);
	let request = a::play_to_server_packet::Ty::CraftRecipeRequest(
		a::play_to_server_packet_craft_recipe_request::Ty {
			window_id: 1,
			recipe: "minecraft:7".to_owned(),
			make_all: false,
		},
	);
	let out = server_up(&request, &mut cx);
	assert!(
		matches!(out.as_slice(), [d::play_to_server_packet::Ty::CraftRecipeRequest(r)] if r.recipe_id == 7 && r.window_id == 1),
		"{out:?}"
	);

	use d::play_to_client_chat_type_parameter_type::Ty as Param;
	let decoration = d::play_to_client_chat_type::Ty {
		translation_key: "chat.type.text".to_owned(),
		parameters: vec![Param::Sender, Param::Content],
		style: tag(vec![text("color", "red")]),
	};
	let chat = C::ProfilelessChat(d::play_to_client_packet_profileless_chat::Ty {
		message: string_tag("hi"),
		r#type: d::play_to_client_chat_types_holder::Ty::Inline(d::play_to_client_chat_types::Ty {
			chat: decoration.clone(),
			narration: decoration,
		}),
		name: string_tag("Bob"),
		target: None,
	});
	let out = client_down(&chat, &mut cx);
	let [A::SystemChat(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let content = compound(&s.content);
	assert_eq!(
		field(content, "translate"),
		&nbt::Payload::String("chat.type.text".to_owned())
	);
	assert_eq!(
		field(content, "color"),
		&nbt::Payload::String("red".to_owned())
	);
	let nbt::gen::payload::Ty::List(nbt::gen::list::Ty {
		items: nbt::gen::list_0::Ty::Compound(with),
		..
	}) = field(content, "with")
	else {
		panic!()
	};
	assert_eq!(
		field(&with[0], "text"),
		&nbt::Payload::String("Bob".to_owned())
	);
	assert_eq!(
		field(&with[1], "text"),
		&nbt::Payload::String("hi".to_owned())
	);
}

#[test]
fn food_and_sword_blocking() {
	use a::play_to_client_packet::Ty as A;
	use d::play_to_client_packet::Ty as C;
	let mut cx = Session::new();
	let stew = item_a(
		"suspicious_stew",
		vec![a::slot_component::Ty::Food(a::slot_component_food::Ty {
			nutrition: 6,
			saturation_modifier: 0.6,
			can_always_eat: true,
			seconds_to_eat: 1.6,
			effects: vec![a::slot_component_food_effects_items::Ty {
				effect: a::item_potion_effect::Ty {
					id: id("1.20.5", "effects", "Regeneration"),
					details: a::item_effect_detail::Ty {
						amplifier: 0,
						duration: 160,
						ambient: false,
						show_particles: true,
						show_icon: true,
						hidden_effect: None,
					},
				},
				probability: 0.5,
			}],
		})],
	);
	minecraft_versions::custom::tricky_enable_legacy_combat(&mut cx);
	let window = A::WindowItems(a::play_to_client_packet_window_items::Ty {
		window_id: 0,
		state_id: 1,
		items: vec![
			stew,
			item_a("diamond_sword", vec![]),
			item_a("diamond", vec![]),
		],
		carried_item: a::slot::Ty::V0,
	});
	let out = up(&window, &mut cx);
	let [C::WindowItems(w)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let components = |slot: &d::slot::Ty| match slot {
		d::slot::Ty::Default(_, s) => {
			assert_eq!(s.added_component_count as usize, s.components.len());
			s.components.clone()
		}
		_ => vec![],
	};
	let food = components(&w.items[0]);
	let consumable = food
		.iter()
		.find_map(|c| match c {
			d::slot_component::Ty::Consumable(c) => Some(c),
			_ => None,
		})
		.expect("food split into consumable");
	assert_eq!(consumable.consume_seconds, 1.6);
	assert!(
		matches!(&consumable.effects[..], [d::item_consume_effect::Ty::ApplyEffects(e)] if e.probability == 0.5 && e.effects.len() == 1)
	);
	assert!(
		components(&w.items[1]).iter().any(|c| matches!(c, d::slot_component::Ty::Consumable(c) if matches!(c.animation, d::slot_component_consumable_animation::Ty::Block))),
		"sword blocks under legacy combat"
	);
	assert!(components(&w.items[2]).is_empty());

	let beef = item_d(
		"cooked_beef",
		vec![
			d::slot_component::Ty::Food(d::slot_component_food::Ty {
				nutrition: 8,
				saturation_modifier: 12.8,
				can_always_eat: false,
			}),
			d::slot_component::Ty::Consumable(d::slot_component_consumable::Ty {
				consume_seconds: 0.8,
				animation: d::slot_component_consumable_animation::Ty::Eat,
				sound: d::item_sound_holder::Ty::Id(id("1.21.4", "sounds", "entity.generic.eat")),
				makes_particles: true,
				effects: vec![d::item_consume_effect::Ty::ApplyEffects(
					d::item_consume_effect_apply_effects::Ty {
						effects: vec![potion("Hunger")],
						probability: 0.25,
					},
				)],
			}),
			d::slot_component::Ty::UseRemainder(item_d("bowl", vec![])),
		],
	);
	let out = client_down(
		&C::SetSlot(d::play_to_client_packet_set_slot::Ty {
			window_id: 0,
			state_id: 2,
			slot: 36,
			item: beef,
		}),
		&mut cx,
	);
	let [A::SetSlot(s)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let a::slot::Ty::Default(_, item) = &s.item else {
		panic!()
	};
	assert_eq!(item.added_component_count as usize, item.components.len());
	let [a::slot_component::Ty::Food(f)] = item.components.as_slice() else {
		panic!("{item:?}")
	};
	assert_eq!(f.seconds_to_eat, 0.8);
	assert!(
		matches!(&f.effects[..], [e] if e.probability == 0.25 && e.effect.id == id("1.20.5", "effects", "Hunger"))
	);
}

#[test]
fn projectile_power_going_down() {
	use a::play_to_client_packet::Ty as A;
	use d::play_to_client_packet::Ty as C;
	let mut cx = Session::new();
	let spawn = C::SpawnEntity(d::play_to_client_packet_spawn_entity::Ty {
		entity_id: 3,
		object_uuid: 3,
		r#type: id("1.21.4", "entities", "fireball"),
		x: 0.0,
		y: 64.0,
		z: 0.0,
		pitch: 0,
		yaw: 0,
		head_pitch: 0,
		object_data: 0,
		velocity: d::vec3i16::Ty { x: 800, y: 0, z: 0 },
	});
	client_down(&spawn, &mut cx);
	let power = |cx: &mut Session| {
		client_down(
			&C::SetProjectilePower(d::play_to_client_packet_set_projectile_power::Ty {
				id: 3,
				acceleration_power: 0.1,
			}),
			cx,
		)
	};
	let out = power(&mut cx);
	assert!(
		matches!(out.as_slice(), [A::SetProjectilePower(p)] if (p.power.x - 0.1).abs() < 1e-9 && p.power.y == 0.0),
		"{out:?}"
	);
	client_down(
		&C::EntityVelocity(d::play_to_client_packet_entity_velocity::Ty {
			entity_id: 3,
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
		matches!(out.as_slice(), [A::SetProjectilePower(p)] if (p.power.y + 0.1).abs() < 1e-9),
		"{out:?}"
	);
	client_down(&C::EntityDestroy(vec![3]), &mut cx);
	assert!(power(&mut cx).is_empty());
}

#[test]
fn old_client_movement_going_up() {
	use a::play_to_server_packet::Ty as P;
	use d::play_to_server_packet::Ty as Q;
	let mut cx = Session::new();
	let out = server_up(
		&P::Position(a::play_to_server_packet_position::Ty {
			x: 1.0,
			y: 64.0,
			z: 2.0,
			on_ground: true,
		}),
		&mut cx,
	);
	assert!(
		matches!(out.as_slice(), [Q::Position(p), Q::TickEnd(_)] if p.flags.on_ground && p.x == 1.0),
		"{out:?}"
	);
	let out = server_up(
		&P::SteerVehicle(a::play_to_server_packet_steer_vehicle::Ty {
			sideways: 0.0,
			forward: 0.98,
			jump: 2,
		}),
		&mut cx,
	);
	use d::play_to_server_packet_entity_action_action_id::Ty as Action;
	assert!(
		matches!(out.as_slice(), [Q::PlayerInput(i), Q::EntityAction(e)] if i.forward && i.shift && !i.jump && matches!(e.action_id, Action::StartSneaking)),
		"{out:?}"
	);
	let out = server_up(
		&P::SteerVehicle(a::play_to_server_packet_steer_vehicle::Ty {
			sideways: 0.0,
			forward: 0.0,
			jump: 0,
		}),
		&mut cx,
	);
	assert!(
		matches!(out.as_slice(), [Q::PlayerInput(_), Q::EntityAction(e)] if matches!(e.action_id, Action::StopSneaking)),
		"{out:?}"
	);
}
