use minecraft_versions::gen::convert::{
	convert, index_of, v1_12_2, v1_18_2, v1_19, v1_19_2, v1_19_4, v1_8, Root, VERSIONS,
};
use minecraft_versions::languages::{self, pieces, Languages, Piece};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::{registry, Session};
use nbt::{CompoundVecExt, Entry, List, ListItems, Payload};
use serde_json::{json, Value};

const OLD: &[(&str, &str)] = &[
	("chat.type.text", "<%s> %s"),
	("chat.type.announcement", "[%s] %s"),
	("chat.type.emote", "* %s %s"),
	(
		"commands.message.display.incoming",
		"%s whispers to you: %s",
	),
	("commands.message.display.outgoing", "You whisper to %s: %s"),
];
const TEAMS: &[(&str, &str)] = &[
	("chat.type.team.text", "%s <%s> %s"),
	("chat.type.team.sent", "-> %s <%s> %s"),
];
const CLIENTS: [&str; 3] = ["1.18.2", "1.12.2", "1.8"];

fn at(v: &str) -> usize {
	index_of(v).unwrap()
}

fn setup() {
	let strings = |sets: &[&[(&str, &str)]]| -> Vec<(String, String)> {
		sets.iter()
			.flat_map(|s| s.iter())
			.map(|(k, v)| (k.to_string(), v.to_string()))
			.collect()
	};
	let index = VERSIONS
		.iter()
		.filter(|v| at(v) >= at("1.7"))
		.map(|v| (*v, if at(v) >= at("1.13") { "teams" } else { "old" }));
	languages::install(Languages::new(
		index,
		[("old", strings(&[OLD])), ("teams", strings(&[OLD, TEAMS]))],
	));
}

fn known(version: &str) -> Vec<(&'static str, &'static str)> {
	let teams: &[(&str, &str)] = if at(version) >= at("1.13") {
		TEAMS
	} else {
		&[]
	};
	OLD.iter().chain(teams).copied().collect()
}

fn render(v: &Value, known: &[(&str, &str)]) -> String {
	match v {
		Value::String(s) => s.clone(),
		Value::Array(items) => items.iter().map(|i| render(i, known)).collect(),
		Value::Object(m) => {
			let mut out = m.get("text").map(|t| render(t, known)).unwrap_or_default();
			if let Some(key) = m.get("translate").and_then(Value::as_str) {
				let args = m
					.get("with")
					.and_then(Value::as_array)
					.cloned()
					.unwrap_or_default();
				let pattern = known
					.iter()
					.find(|(k, _)| *k == key)
					.map_or(key, |(_, p)| p);
				for piece in pieces(pattern) {
					match piece {
						Piece::Text(t) => out.push_str(t),
						Piece::Arg(i) => {
							out.push_str(&args.get(i).map(|a| render(a, known)).unwrap_or_default())
						}
					}
				}
			}
			if let Some(extra) = m.get("extra") {
				out.push_str(&render(extra, known));
			}
			out
		}
		_ => String::new(),
	}
}

fn encode<T>(v: &T, ser: fn(&T, &mut Output) -> minecraft_versions::pds::Result<()>) -> Vec<u8> {
	let mut o = Output::new();
	ser(v, &mut o).unwrap();
	o.into_inner()
}

fn decode<T>(b: &[u8], de: fn(&mut Input) -> minecraft_versions::pds::Result<T>) -> T {
	let mut i = Input::new(b);
	let v = de(&mut i).unwrap();
	assert_eq!(i.remaining(), 0, "left over bytes");
	v
}

fn strings(values: &[&str]) -> Payload {
	ListItems::String(values.iter().map(|s| s.to_string()).collect()).into()
}

fn decoration(key: &str, parameters: &[&str], style: Vec<Entry>) -> Vec<Entry> {
	vec![
		Entry::new("translation_key", key),
		Entry::new("parameters", strings(parameters)),
		Entry::new("style", style),
	]
}

struct Kind {
	name: &'static str,
	key: &'static str,
	parameters: &'static [&'static str],
	style: fn() -> Vec<Entry>,
}

fn plain() -> Vec<Entry> {
	vec![]
}

fn whisper() -> Vec<Entry> {
	vec![Entry::new("color", "gray"), Entry::new("italic", 1i8)]
}

fn shout() -> Vec<Entry> {
	vec![Entry::new("color", "red"), Entry::new("bold", 1i8)]
}

const KINDS: &[Kind] = &[
	Kind {
		name: "minecraft:chat",
		key: "chat.type.text",
		parameters: &["sender", "content"],
		style: plain,
	},
	Kind {
		name: "minecraft:emote_command",
		key: "chat.type.emote",
		parameters: &["sender", "content"],
		style: plain,
	},
	Kind {
		name: "minecraft:msg_command_incoming",
		key: "commands.message.display.incoming",
		parameters: &["sender", "content"],
		style: whisper,
	},
	Kind {
		name: "minecraft:msg_command_outgoing",
		key: "commands.message.display.outgoing",
		parameters: &["target", "content"],
		style: whisper,
	},
	Kind {
		name: "minecraft:say_command",
		key: "chat.type.announcement",
		parameters: &["sender", "content"],
		style: plain,
	},
	Kind {
		name: "minecraft:team_msg_command_incoming",
		key: "chat.type.team.text",
		parameters: &["target", "sender", "content"],
		style: plain,
	},
	Kind {
		name: "minecraft:team_msg_command_outgoing",
		key: "chat.type.team.sent",
		parameters: &["target", "sender", "content"],
		style: plain,
	},
	Kind {
		name: "example:shout",
		key: "%2$s shouts: %1$s",
		parameters: &["content", "sender"],
		style: shout,
	},
];

fn order(names: &[&str]) -> Vec<&'static Kind> {
	names
		.iter()
		.map(|n| KINDS.iter().find(|k| k.name.ends_with(n)).unwrap())
		.collect()
}

fn orders() -> [Vec<&'static Kind>; 2] {
	[
		order(&[
			"chat",
			"emote_command",
			"msg_command_incoming",
			"msg_command_outgoing",
			"say_command",
			"team_msg_command_incoming",
			"team_msg_command_outgoing",
			"shout",
		]),
		order(&[
			"team_msg_command_outgoing",
			"shout",
			"say_command",
			"chat",
			"msg_command_incoming",
			"emote_command",
			"team_msg_command_incoming",
			"msg_command_outgoing",
		]),
	]
}

fn id(kinds: &[&Kind], name: &str) -> i32 {
	kinds.iter().position(|k| k.name.ends_with(name)).unwrap() as i32
}

fn registry_of(elements: Vec<(&str, Vec<Entry>)>) -> Payload {
	let list = elements
		.into_iter()
		.enumerate()
		.map(|(i, (name, element))| {
			vec![
				Entry::new("name", name),
				Entry::new("id", i as i32),
				Entry::new("element", element),
			]
		})
		.collect();
	Payload::Compound(vec![
		Entry::new("type", "minecraft:chat_type"),
		Entry::new("value", List::new(ListItems::Compound(list))),
	])
}

fn codec(version: &str, chat_types: Payload) -> Entry {
	let v: Value = serde_json::from_str(registry::login_packet(version).unwrap()).unwrap();
	let mut root = nbt::json::typed::read(&v["dimensionCodec"])
		.unwrap()
		.into_compound()
		.unwrap();
	root.put("minecraft:chat_type", chat_types);
	Entry::new("", root)
}

fn modern_codec(version: &str, kinds: &[&Kind]) -> Entry {
	let narration = || decoration("chat.type.text.narrate", &["sender", "content"], vec![]);
	let elements = kinds
		.iter()
		.map(|k| {
			(
				k.name,
				vec![
					Entry::new("chat", decoration(k.key, k.parameters, (k.style)())),
					Entry::new("narration", narration()),
				],
			)
		})
		.collect();
	codec(version, registry_of(elements))
}

fn chat_message(version: &str, bytes: &[u8]) -> (Value, i8) {
	let (message, position) = match version {
		"1.18.2" => match decode(bytes, v1_18_2::play_to_client_packet::deserialize) {
			v1_18_2::play_to_client_packet::Ty::Chat(c) => (c.message, c.position),
			p => panic!("{p:?}"),
		},
		"1.12.2" => match decode(bytes, v1_12_2::play_to_client_packet::deserialize) {
			v1_12_2::play_to_client_packet::Ty::Chat(c) => (c.message, c.position),
			p => panic!("{p:?}"),
		},
		"1.8" => match decode(bytes, v1_8::play_to_client_packet::deserialize) {
			v1_8::play_to_client_packet::Ty::Chat(c) => (c.message, c.position),
			p => panic!("{p:?}"),
		},
		v => panic!("{v}"),
	};
	(serde_json::from_str(&message).unwrap(), position)
}

fn down(from: &str, to: &str, packets: &[Vec<u8>]) -> Vec<(Value, i8)> {
	let mut cx = Session::new();
	let mut out = convert(Root::PlayToClient, at(from), at(to), &packets[0], &mut cx)
		.unwrap()
		.into_iter();
	assert!(out.next().is_some(), "{from} login reaches {to}");
	packets[1..]
		.iter()
		.map(|p| {
			let out = convert(Root::PlayToClient, at(from), at(to), p, &mut cx).unwrap();
			let [bytes] = out.as_slice() else {
				panic!("{to}: {} packets", out.len())
			};
			chat_message(to, bytes)
		})
		.collect()
}

fn text(s: &str) -> String {
	json!({ "text": s }).to_string()
}

fn login_1_19_4(codec: Entry) -> Vec<u8> {
	encode(
		&v1_19_4::play_to_client_packet::Ty::Login(v1_19_4::play_to_client_packet_login::Ty {
			entity_id: 1,
			is_hardcore: false,
			game_mode: 1,
			previous_game_mode: -1,
			world_names: vec!["minecraft:overworld".to_owned()],
			dimension_codec: codec,
			world_type: "minecraft:overworld".to_owned(),
			world_name: "minecraft:overworld".to_owned(),
			hashed_seed: 0,
			max_players: 20,
			view_distance: 8,
			simulation_distance: 8,
			reduced_debug_info: false,
			enable_respawn_screen: true,
			is_debug: false,
			is_flat: false,
			death: None,
		}),
		v1_19_4::play_to_client_packet::serialize,
	)
}

fn profileless(r#type: i32, name: &str, target: Option<&str>, message: &str) -> Vec<u8> {
	encode(
		&v1_19_4::play_to_client_packet::Ty::ProfilelessChat(
			v1_19_4::play_to_client_packet_profileless_chat::Ty {
				message: text(message),
				r#type,
				name: text(name),
				target: target.map(text),
			},
		),
		v1_19_4::play_to_client_packet::serialize,
	)
}

fn player_chat(r#type: i32, name: &str, message: &str) -> Vec<u8> {
	encode(
		&v1_19_4::play_to_client_packet::Ty::PlayerChat(
			v1_19_4::play_to_client_packet_player_chat::Ty {
				sender_uuid: 7,
				index: 0,
				signature: None,
				plain_message: message.to_owned(),
				timestamp: 0,
				salt: 0,
				previous_messages: vec![],
				unsigned_chat_content: None,
				filter_type_mask:
					v1_19_4::play_to_client_packet_player_chat_filter_type_mask::Ty::Default(0),
				r#type,
				network_name: text(name),
				network_target_name: None,
			},
		),
		v1_19_4::play_to_client_packet::serialize,
	)
}

#[test]
fn registry_order_picks_the_decoration() {
	setup();
	for kinds in orders() {
		let packets = vec![
			login_1_19_4(modern_codec("1.19.4", &kinds)),
			profileless(
				id(&kinds, "say_command"),
				"Server",
				None,
				"hello from the console",
			),
			profileless(
				id(&kinds, "msg_command_incoming"),
				"Server",
				Some("bot"),
				"hi",
			),
			profileless(id(&kinds, "emote_command"), "Server", None, "waves"),
			profileless(
				id(&kinds, "team_msg_command_incoming"),
				"Server",
				Some("Red"),
				"go",
			),
			profileless(
				id(&kinds, "team_msg_command_outgoing"),
				"bot",
				Some("Red"),
				"go",
			),
			player_chat(id(&kinds, "chat"), "bot", "hey"),
			profileless(id(&kinds, "shout"), "Server", None, "hi"),
		];
		for client in CLIENTS {
			let out = down("1.19.4", client, &packets);
			let rendered: Vec<String> =
				out.iter().map(|(v, _)| render(v, &known(client))).collect();
			assert_eq!(
				rendered,
				[
					"[Server] hello from the console",
					"Server whispers to you: hi",
					"* Server waves",
					"Red <Server> go",
					"-> Red <bot> go",
					"<bot> hey",
					"Server shouts: hi",
				],
				"{client}: {out:?}"
			);
			let (whisper, _) = &out[1];
			assert_eq!(
				(&whisper["color"], &whisper["italic"]),
				(&json!("gray"), &json!(true)),
				"{client}"
			);
			let (shout, _) = &out[6];
			assert_eq!(
				(&shout["translate"], &shout["color"], &shout["bold"]),
				(&json!("%2$s shouts: %1$s"), &json!("red"), &json!(true)),
				"{client}"
			);
			let teams = serde_json::to_string(&out[3].0).unwrap();
			assert_eq!(
				teams.contains("chat.type.team.text"),
				at(client) >= at("1.13"),
				"{client}: {teams}"
			);
		}
	}
}

fn login_1_19_2(codec: Entry) -> Vec<u8> {
	encode(
		&v1_19_2::play_to_client_packet::Ty::Login(v1_19_2::play_to_client_packet_login::Ty {
			entity_id: 1,
			is_hardcore: false,
			game_mode: 1,
			previous_game_mode: -1,
			world_names: vec!["minecraft:overworld".to_owned()],
			dimension_codec: codec,
			world_type: "minecraft:overworld".to_owned(),
			world_name: "minecraft:overworld".to_owned(),
			hashed_seed: 0,
			max_players: 20,
			view_distance: 8,
			simulation_distance: 8,
			reduced_debug_info: false,
			enable_respawn_screen: true,
			is_debug: false,
			is_flat: false,
			death: None,
		}),
		v1_19_2::play_to_client_packet::serialize,
	)
}

fn player_chat_1_19_2(r#type: i32, name: &str, target: Option<&str>, message: &str) -> Vec<u8> {
	encode(
		&v1_19_2::play_to_client_packet::Ty::PlayerChat(
			v1_19_2::play_to_client_packet_player_chat::Ty {
				previous_signature: None,
				sender_uuid: 0,
				signature: vec![],
				plain_message: message.to_owned(),
				formatted_message: Some(text(message)),
				timestamp: 0,
				salt: 0,
				previous_messages: vec![],
				unsigned_chat_content: None,
				filter_type_mask:
					v1_19_2::play_to_client_packet_player_chat_filter_type_mask::Ty::Default(0),
				r#type,
				network_name: text(name),
				network_target_name: target.map(text),
			},
		),
		v1_19_2::play_to_client_packet::serialize,
	)
}

#[test]
fn signed_chat_era_servers_use_their_registry() {
	setup();
	for kinds in orders() {
		let packets = vec![
			login_1_19_2(modern_codec("1.19.2", &kinds)),
			player_chat_1_19_2(id(&kinds, "say_command"), "Server", None, "hi"),
			player_chat_1_19_2(
				id(&kinds, "msg_command_incoming"),
				"Server",
				Some("bot"),
				"psst",
			),
			player_chat_1_19_2(id(&kinds, "shout"), "Server", None, "hi"),
		];
		for client in CLIENTS {
			let out = down("1.19.2", client, &packets);
			let rendered: Vec<String> =
				out.iter().map(|(v, _)| render(v, &known(client))).collect();
			assert_eq!(
				rendered,
				[
					"[Server] hi",
					"Server whispers to you: psst",
					"Server shouts: hi"
				],
				"{client}: {out:?}"
			);
		}
	}
}

fn login_1_19(codec: Entry) -> Vec<u8> {
	encode(
		&v1_19::play_to_client_packet::Ty::Login(v1_19::play_to_client_packet_login::Ty {
			entity_id: 1,
			is_hardcore: false,
			game_mode: 1,
			previous_game_mode: -1,
			world_names: vec!["minecraft:overworld".to_owned()],
			dimension_codec: codec,
			world_type: "minecraft:overworld".to_owned(),
			world_name: "minecraft:overworld".to_owned(),
			hashed_seed: 0,
			max_players: 20,
			view_distance: 8,
			simulation_distance: 8,
			reduced_debug_info: false,
			enable_respawn_screen: true,
			is_debug: false,
			is_flat: false,
			death: None,
		}),
		v1_19::play_to_client_packet::serialize,
	)
}

fn player_chat_1_19(r#type: i32, name: &str, team: Option<&str>, message: &str) -> Vec<u8> {
	encode(
		&v1_19::play_to_client_packet::Ty::PlayerChat(
			v1_19::play_to_client_packet_player_chat::Ty {
				signed_chat_content: text(message),
				unsigned_chat_content: None,
				r#type,
				sender_uuid: 0,
				sender_name: text(name),
				sender_team: team.map(text),
				timestamp: 0,
				salt: 0,
				signature: vec![],
			},
		),
		v1_19::play_to_client_packet::serialize,
	)
}

fn system_chat_1_19(r#type: i32, message: &str) -> Vec<u8> {
	encode(
		&v1_19::play_to_client_packet::Ty::SystemChat(
			v1_19::play_to_client_packet_system_chat::Ty {
				content: text(message),
				r#type,
			},
		),
		v1_19::play_to_client_packet::serialize,
	)
}

#[test]
fn first_chat_registry_has_decorations_and_overlays() {
	setup();
	let chat = |key: &str, parameters: &[&str]| {
		vec![Entry::new(
			"chat",
			vec![Entry::new(
				"decoration",
				decoration(key, parameters, vec![]),
			)],
		)]
	};
	let types = registry_of(vec![
		(
			"minecraft:game_info",
			vec![Entry::new("overlay", Vec::<Entry>::new())],
		),
		(
			"minecraft:team_msg_command",
			chat("chat.type.team.text", &["team_name", "sender", "content"]),
		),
		(
			"example:shout",
			chat("%2$s shouts: %1$s", &["content", "sender"]),
		),
		(
			"minecraft:say_command",
			chat("chat.type.announcement", &["sender", "content"]),
		),
		(
			"minecraft:system",
			vec![Entry::new("chat", Vec::<Entry>::new())],
		),
	]);
	let packets = vec![
		login_1_19(codec("1.19", types)),
		player_chat_1_19(3, "Server", None, "hi"),
		player_chat_1_19(1, "Server", Some("Red"), "go"),
		player_chat_1_19(2, "Server", None, "hi"),
		player_chat_1_19(4, "Server", None, "plain"),
		system_chat_1_19(0, "above the hotbar"),
		system_chat_1_19(4, "in the chat"),
	];
	for client in CLIENTS {
		let out = down("1.19", client, &packets);
		let rendered: Vec<(String, i8)> = out
			.iter()
			.map(|(v, p)| (render(v, &known(client)), *p))
			.collect();
		let expected = [
			("[Server] hi", 0),
			("Red <Server> go", 0),
			("Server shouts: hi", 0),
			("plain", 0),
			("above the hotbar", 2),
			("in the chat", 1),
		];
		assert_eq!(
			rendered,
			expected.map(|(t, p)| (t.to_owned(), p)),
			"{client}: {out:?}"
		);
	}
}
