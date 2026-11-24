use bungeeoxide_converter::generated::convert::{Root, v26_3 as z, v26_4_snapshot_10 as w};
use bungeeoxide_converter::pds::{Input, Output};
use bungeeoxide_converter::{Session, Version, convert};

type Props3 = z::command_node_extra_node_data_v2_properties::Ty;
type Props4 = w::command_node_extra_node_data_v2_properties::Ty;
type Mode = z::command_node_extra_node_data_v2_properties_brigadier_string::Ty;
type Holder3 = z::command_node_extra_node_data_v2_properties_minecraft_score_holder::Ty;
type Time4 = w::command_node_extra_node_data_v2_properties_minecraft_time::Ty;
type TimeFlags4 = w::command_node_extra_node_data_v2_properties_minecraft_time_flags::Ty;
type TimeMin4 = w::command_node_extra_node_data_v2_properties_minecraft_time_min::Ty;
type TimeMax4 = w::command_node_extra_node_data_v2_properties_minecraft_time_max::Ty;
type Suggestion4 = w::command_node_extra_node_data_v2_suggestion_type_v1::Ty;

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

fn up(p: &z::play_to_client_packet::Ty) -> w::play_to_client_packet::Ty {
	let out = convert(
		Root::PlayToClient,
		Version::V26_3,
		Version::V26_4_snapshot_10,
		&encode(p, z::play_to_client_packet::serialize),
		&mut Session::new(),
	)
	.unwrap();
	let [bytes] = out.as_slice() else {
		panic!("{out:?}")
	};
	decode(bytes, w::play_to_client_packet::deserialize)
}

fn down(p: &w::play_to_client_packet::Ty) -> z::play_to_client_packet::Ty {
	let out = convert(
		Root::PlayToClient,
		Version::V26_4_snapshot_10,
		Version::V26_3,
		&encode(p, w::play_to_client_packet::serialize),
		&mut Session::new(),
	)
	.unwrap();
	let [bytes] = out.as_slice() else {
		panic!("{out:?}")
	};
	decode(bytes, z::play_to_client_packet::deserialize)
}

const ROOT: u8 = 0;
const LITERAL: u8 = 1;
const ARGUMENT: u8 = 2;
const COMMAND: &str = "probe";

fn tree_26_3(arguments: Vec<(&str, Props3, Option<&str>)>) -> z::play_to_client_packet::Ty {
	let node = |kind: u8, suggests: bool, children: Vec<i32>, extra| z::command_node::Ty {
		flags: z::command_node_flags::Ty {
			unused: 0,
			allows_restricted: 0,
			has_custom_suggestions: suggests as u8,
			has_redirect_node: 0,
			has_command: (kind == ARGUMENT) as u8,
			command_node_type: kind,
		},
		children,
		redirect_node: z::command_node_redirect_node::Ty::Default(0),
		extra_node_data: extra,
	};
	let count = arguments.len() as i32;
	let mut nodes = vec![
		node(
			ROOT,
			false,
			vec![1],
			z::command_node_extra_node_data::Ty::V0,
		),
		node(
			LITERAL,
			false,
			(2..2 + count).collect(),
			z::command_node_extra_node_data::Ty::V1(COMMAND.into()),
		),
	];
	for (name, properties, suggestion) in arguments {
		nodes.push(node(
			ARGUMENT,
			suggestion.is_some(),
			Vec::new(),
			z::command_node_extra_node_data::Ty::V2(z::command_node_extra_node_data_v2::Ty {
				name: name.into(),
				properties,
				suggestion_type: match suggestion {
					Some(s) => z::command_node_extra_node_data_v2_suggestion_type::Ty::V1(s.into()),
					None => z::command_node_extra_node_data_v2_suggestion_type::Ty::Default(0),
				},
			}),
		));
	}
	z::play_to_client_packet::Ty::DeclareCommands(z::play_to_client_packet_declare_commands::Ty {
		nodes,
		root_index: 0,
	})
}

fn tree_26_4(arguments: Vec<(&str, Props4, Option<Suggestion4>)>) -> w::play_to_client_packet::Ty {
	let node = |kind: u8, suggests: bool, children: Vec<i32>, extra| w::command_node::Ty {
		flags: w::command_node_flags::Ty {
			unused: 0,
			allows_restricted: 0,
			has_custom_suggestions: suggests as u8,
			has_redirect_node: 0,
			has_command: (kind == ARGUMENT) as u8,
			command_node_type: kind,
		},
		children,
		redirect_node: w::command_node_redirect_node::Ty::Default(0),
		extra_node_data: extra,
	};
	let count = arguments.len() as i32;
	let mut nodes = vec![
		node(
			ROOT,
			false,
			vec![1],
			w::command_node_extra_node_data::Ty::V0,
		),
		node(
			LITERAL,
			false,
			(2..2 + count).collect(),
			w::command_node_extra_node_data::Ty::V1(COMMAND.into()),
		),
	];
	for (name, properties, suggestion) in arguments {
		nodes.push(node(
			ARGUMENT,
			suggestion.is_some(),
			Vec::new(),
			w::command_node_extra_node_data::Ty::V2(w::command_node_extra_node_data_v2::Ty {
				name: name.into(),
				properties,
				suggestion_type: match suggestion {
					Some(s) => w::command_node_extra_node_data_v2_suggestion_type::Ty::V1(s),
					None => w::command_node_extra_node_data_v2_suggestion_type::Ty::Default(0),
				},
			}),
		));
	}
	w::play_to_client_packet::Ty::DeclareCommands(w::play_to_client_packet_declare_commands::Ty {
		nodes,
		root_index: 0,
	})
}

fn arguments_26_3(p: &z::play_to_client_packet::Ty) -> Vec<(String, Props3, Option<String>)> {
	let z::play_to_client_packet::Ty::DeclareCommands(p) = p else {
		panic!("{p:?}")
	};
	assert_eq!(p.root_index, 0);
	assert_eq!(p.nodes[0].children, vec![1]);
	assert_eq!(
		p.nodes[0].extra_node_data,
		z::command_node_extra_node_data::Ty::V0
	);
	assert_eq!(
		p.nodes[1].extra_node_data,
		z::command_node_extra_node_data::Ty::V1(COMMAND.into())
	);
	assert_eq!(
		p.nodes[1].children,
		(2..p.nodes.len() as i32).collect::<Vec<_>>()
	);
	p.nodes[2..]
		.iter()
		.map(|n| {
			assert_eq!(n.flags.command_node_type, ARGUMENT);
			assert_eq!(n.flags.has_command, 1);
			let z::command_node_extra_node_data::Ty::V2(a) = &n.extra_node_data else {
				panic!("{n:?}")
			};
			let suggestion = match &a.suggestion_type {
				z::command_node_extra_node_data_v2_suggestion_type::Ty::V1(s) => Some(s.clone()),
				z::command_node_extra_node_data_v2_suggestion_type::Ty::Default(_) => None,
			};
			assert_eq!(n.flags.has_custom_suggestions, suggestion.is_some() as u8);
			(a.name.clone(), a.properties.clone(), suggestion)
		})
		.collect()
}

fn arguments_26_4(p: &w::play_to_client_packet::Ty) -> Vec<(String, Props4, Option<Suggestion4>)> {
	let w::play_to_client_packet::Ty::DeclareCommands(p) = p else {
		panic!("{p:?}")
	};
	assert_eq!(p.root_index, 0);
	assert_eq!(p.nodes[0].children, vec![1]);
	assert_eq!(
		p.nodes[0].extra_node_data,
		w::command_node_extra_node_data::Ty::V0
	);
	assert_eq!(
		p.nodes[1].extra_node_data,
		w::command_node_extra_node_data::Ty::V1(COMMAND.into())
	);
	assert_eq!(
		p.nodes[1].children,
		(2..p.nodes.len() as i32).collect::<Vec<_>>()
	);
	p.nodes[2..]
		.iter()
		.map(|n| {
			assert_eq!(n.flags.command_node_type, ARGUMENT);
			assert_eq!(n.flags.has_command, 1);
			let w::command_node_extra_node_data::Ty::V2(a) = &n.extra_node_data else {
				panic!("{n:?}")
			};
			let suggestion = match &a.suggestion_type {
				w::command_node_extra_node_data_v2_suggestion_type::Ty::V1(s) => Some(s.clone()),
				w::command_node_extra_node_data_v2_suggestion_type::Ty::Default(_) => None,
			};
			assert_eq!(n.flags.has_custom_suggestions, suggestion.is_some() as u8);
			(a.name.clone(), a.properties.clone(), suggestion)
		})
		.collect()
}

fn named<P, S>(v: Vec<(&str, P, Option<S>)>) -> Vec<(String, P, Option<S>)> {
	v.into_iter()
		.map(|(n, p, s)| (n.to_owned(), p, s))
		.collect()
}

fn time(min: Option<i32>, max: Option<i32>) -> Props4 {
	Props4::MinecraftTime(Time4 {
		flags: TimeFlags4 {
			unused: 0,
			max_present: max.is_some() as u8,
			min_present: min.is_some() as u8,
		},
		min: min.map_or(TimeMin4::Default(0), TimeMin4::V1),
		max: max.map_or(TimeMax4::Default(0), TimeMax4::V1),
	})
}

const FEATURE: &str = "minecraft:worldgen/configured_feature";
const STRUCTURE: &str = "minecraft:worldgen/structure";

#[test]
fn command_tree_goes_up_and_back() {
	let tree = tree_26_3(vec![
		("word", Props3::BrigadierString(Mode::SingleWord), None),
		(
			"quoted",
			Props3::BrigadierString(Mode::QuotablePhrase),
			None,
		),
		("greedy", Props3::BrigadierString(Mode::GreedyPhrase), None),
		(
			"feature",
			Props3::MinecraftResourceKey(FEATURE.into()),
			None,
		),
		(
			"structure",
			Props3::MinecraftResourceOrTagKey(STRUCTURE.into()),
			None,
		),
		(
			"targets",
			Props3::MinecraftScoreHolder(Holder3 {
				unused: 0,
				allow_multiple: 1,
			}),
			None,
		),
		(
			"target",
			Props3::MinecraftScoreHolder(Holder3 {
				unused: 0,
				allow_multiple: 0,
			}),
			None,
		),
		("delay", Props3::MinecraftTime(20), None),
		("pos", Props3::MinecraftVec3, None),
		("table", Props3::MinecraftLootTable, None),
		(
			"sound",
			Props3::MinecraftResourceLocation,
			Some("minecraft:available_sounds"),
		),
		(
			"entity",
			Props3::MinecraftResource("minecraft:entity_type".into()),
			Some("summonable_entities"),
		),
		(
			"custom",
			Props3::MinecraftResourceLocation,
			Some("example:unknown"),
		),
	]);

	let upgraded = up(&tree);
	assert_eq!(
		arguments_26_4(&upgraded),
		named(vec![
			("word", Props4::MinecraftWord, None),
			("quoted", Props4::MinecraftQuotedString, None),
			("greedy", Props4::MinecraftGreedyString, None),
			("feature", Props4::MinecraftResource(FEATURE.into()), None),
			(
				"structure",
				Props4::MinecraftResourceOrTag(STRUCTURE.into()),
				None
			),
			("targets", Props4::MinecraftScoreHolder(true), None),
			("target", Props4::MinecraftScoreHolder(false), None),
			("delay", time(Some(20), None), None),
			("pos", Props4::MinecraftVec3(true), None),
			("table", Props4::MinecraftLootTable(2), None),
			(
				"sound",
				Props4::MinecraftResourceLocation,
				Some(Suggestion4::AvailableSounds)
			),
			(
				"entity",
				Props4::MinecraftResource("minecraft:entity_type".into()),
				Some(Suggestion4::SummonableEntities)
			),
			(
				"custom",
				Props4::MinecraftResourceLocation,
				Some(Suggestion4::AskServer)
			),
		])
	);

	assert_eq!(
		arguments_26_3(&down(&upgraded)),
		named(vec![
			("word", Props3::BrigadierString(Mode::SingleWord), None),
			(
				"quoted",
				Props3::BrigadierString(Mode::QuotablePhrase),
				None
			),
			("greedy", Props3::BrigadierString(Mode::GreedyPhrase), None),
			("feature", Props3::MinecraftResource(FEATURE.into()), None),
			(
				"structure",
				Props3::MinecraftResourceOrTag(STRUCTURE.into()),
				None
			),
			(
				"targets",
				Props3::MinecraftScoreHolder(Holder3 {
					unused: 0,
					allow_multiple: 1,
				}),
				None
			),
			(
				"target",
				Props3::MinecraftScoreHolder(Holder3 {
					unused: 0,
					allow_multiple: 0,
				}),
				None
			),
			("delay", Props3::MinecraftTime(20), None),
			("pos", Props3::MinecraftVec3, None),
			("table", Props3::MinecraftLootTable, None),
			(
				"sound",
				Props3::MinecraftResourceLocation,
				Some("minecraft:available_sounds".to_owned())
			),
			(
				"entity",
				Props3::MinecraftResource("minecraft:entity_type".into()),
				Some("minecraft:summonable_entities".to_owned())
			),
			(
				"custom",
				Props3::MinecraftResourceLocation,
				Some("minecraft:ask_server".to_owned())
			),
		])
	);
}

#[test]
fn new_arguments_go_down_and_context_sets_come_back() {
	let tree = tree_26_4(vec![
		(
			"id",
			Props4::MinecraftPattern(
				w::command_node_extra_node_data_v2_properties_minecraft_pattern::Ty {
					pattern: "[a-z_]+".into(),
					error_message: None,
				},
			),
			None,
		),
		("open", time(None, Some(100)), None),
		("bounded", time(Some(5), Some(10)), None),
		("slot", Props4::MinecraftSlotSource(5), None),
		("value", Props4::MinecraftContextFloatProvider(7), None),
		("count", Props4::MinecraftContextIntProvider(8), None),
		("predicate", Props4::MinecraftLootPredicate(3), None),
		("modifier", Props4::MinecraftLootModifier(4), None),
		("fish", Props4::MinecraftLootTable(11), None),
		("rot", Props4::MinecraftVec2(false), None),
		(
			"choice",
			Props4::MinecraftWord,
			Some(Suggestion4::Literal(vec!["a".into(), "b".into()])),
		),
		(
			"effect",
			Props4::MinecraftResourceLocation,
			Some(Suggestion4::PostEffects),
		),
	]);

	let downgraded = down(&tree);
	assert_eq!(
		arguments_26_3(&downgraded),
		named(vec![
			("id", Props3::BrigadierString(Mode::QuotablePhrase), None),
			("open", Props3::MinecraftTime(i32::MIN), None),
			("bounded", Props3::MinecraftTime(5), None),
			("slot", Props3::MinecraftSlotSource, None),
			("value", Props3::MinecraftContextFloatProvider, None),
			("count", Props3::MinecraftContextIntProvider, None),
			("predicate", Props3::MinecraftLootPredicate, None),
			("modifier", Props3::MinecraftLootModifier, None),
			("fish", Props3::MinecraftLootTable, None),
			("rot", Props3::MinecraftVec2, None),
			(
				"choice",
				Props3::BrigadierString(Mode::SingleWord),
				Some("minecraft:ask_server".to_owned())
			),
			(
				"effect",
				Props3::MinecraftResourceLocation,
				Some("minecraft:post_effects".to_owned())
			),
		])
	);

	assert_eq!(
		arguments_26_4(&up(&downgraded)),
		named(vec![
			("id", Props4::MinecraftQuotedString, None),
			("open", time(Some(i32::MIN), None), None),
			("bounded", time(Some(5), None), None),
			("slot", Props4::MinecraftSlotSource(5), None),
			("value", Props4::MinecraftContextFloatProvider(6), None),
			("count", Props4::MinecraftContextIntProvider(6), None),
			("predicate", Props4::MinecraftLootPredicate(3), None),
			("modifier", Props4::MinecraftLootModifier(4), None),
			("fish", Props4::MinecraftLootTable(2), None),
			("rot", Props4::MinecraftVec2(true), None),
			(
				"choice",
				Props4::MinecraftWord,
				Some(Suggestion4::AskServer)
			),
			(
				"effect",
				Props4::MinecraftResourceLocation,
				Some(Suggestion4::PostEffects)
			),
		])
	);
}

#[test]
fn unknown_string_mode_is_unconvertible() {
	let tree = tree_26_3(vec![(
		"odd",
		Props3::BrigadierString(Mode::Default(7)),
		None,
	)]);
	assert!(
		convert(
			Root::PlayToClient,
			Version::V26_3,
			Version::V26_4_snapshot_10,
			&encode(&tree, z::play_to_client_packet::serialize),
			&mut Session::new(),
		)
		.is_err()
	);
}
