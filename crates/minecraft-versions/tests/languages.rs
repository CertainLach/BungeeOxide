use minecraft_versions::gen::convert::{
	convert, index_of, v1_0_1, v1_12_2, v1_16, v1_21_11, v1_5_2, v1_8, v26_1 as z, Root, VERSIONS,
};
use minecraft_versions::languages::{self, pieces, Languages, Piece};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::{registry, Session};
use nbt::{Entry, List, ListItems, Payload, Tag};
use serde_json::{json, Value};

const OLD: &[(&str, &str)] = &[
	("chat.type.text", "<%s> %s"),
	("death.attack.generic", "%1$s died"),
	("commands.kill.successful", "Killed %s"),
];
const MID: &[(&str, &str)] = &[("commands.kill.success.single", "Killed %s")];
const NEW: &[(&str, &str)] = &[
	("death.attack.genericKill", "%1$s was killed"),
	("multiplayer.disconnect.kicked", "Kicked by an operator"),
];

fn at(v: &str) -> usize {
	index_of(v).unwrap()
}

fn setup() {
	let folder = |v: &str| match at(v) {
		i if i >= at("1.20.5") => "new",
		i if i >= at("1.13") => "mid",
		_ => "old",
	};
	let strings = |sets: &[&[(&str, &str)]]| -> Vec<(String, String)> {
		sets.iter()
			.flat_map(|s| s.iter())
			.map(|(k, v)| (k.to_string(), v.to_string()))
			.collect()
	};
	let index = VERSIONS
		.iter()
		.filter(|v| at(v) >= at("1.7"))
		.map(|v| (*v, folder(v)));
	let maps = [
		("old", strings(&[OLD])),
		("mid", strings(&[OLD, MID])),
		("new", strings(&[OLD, MID, NEW])),
	];
	languages::install(Languages::new(index, maps));
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

fn down<T>(
	p: &z::play_to_client_packet::Ty,
	to: &str,
	de: fn(&mut Input) -> minecraft_versions::pds::Result<T>,
) -> Vec<T> {
	let bytes = encode(p, z::play_to_client_packet::serialize);
	let out = convert(
		Root::PlayToClient,
		at("26.1"),
		at(to),
		&bytes,
		&mut Session::new(),
	)
	.unwrap();
	out.iter().map(|b| decode(b, de)).collect()
}

fn entries(m: &serde_json::Map<String, Value>) -> Vec<Entry> {
	m.iter()
		.map(|(k, v)| {
			let value = nbt(v);
			Entry::new(k, value)
		})
		.collect()
}

fn nbt(v: &Value) -> Payload {
	match v {
		Value::String(s) => Payload::String(s.clone()),
		Value::Bool(b) => Payload::Byte(*b as i8),
		Value::Number(n) => Payload::Int(n.as_i64().unwrap() as i32),
		Value::Array(items) if items.iter().all(Value::is_string) => {
			Payload::List(List::new(ListItems::String(
				items
					.iter()
					.map(|s| s.as_str().unwrap().to_owned())
					.collect(),
			)))
		}
		Value::Array(items) => Payload::List(List::new(ListItems::Compound(
			items
				.iter()
				.map(|i| match i {
					Value::Object(m) => entries(m),
					other => entries(json!({ "text": other }).as_object().unwrap()),
				})
				.collect(),
		))),
		Value::Object(m) => Payload::Compound(entries(m)),
		Value::Null => Payload::String(String::new()),
	}
}

fn text(v: Value) -> Tag {
	Tag::new(nbt(&v))
}

fn render(v: &Value, known: &[&[(&str, &str)]]) -> String {
	match v {
		Value::String(s) => s.clone(),
		Value::Number(n) => n.to_string(),
		Value::Array(items) => items.iter().map(|i| render(i, known)).collect(),
		Value::Object(m) => {
			let mut out = m.get("text").map(|t| render(t, known)).unwrap_or_default();
			if let Some(key) = m.get("translate").and_then(Value::as_str) {
				let args: Vec<Value> = m
					.get("with")
					.and_then(Value::as_array)
					.cloned()
					.unwrap_or_default();
				match known.iter().flat_map(|s| s.iter()).find(|(k, _)| *k == key) {
					Some((_, pattern)) => {
						for piece in pieces(pattern) {
							match piece {
								Piece::Text(t) => out.push_str(t),
								Piece::Arg(i) => out.push_str(
									&args.get(i).map(|a| render(a, known)).unwrap_or_default(),
								),
							}
						}
					}
					None => out.push_str(key),
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

fn death() -> Value {
	json!({
		"translate": "death.attack.genericKill",
		"color": "gold",
		"with": [{"text": "Steve", "color": "red", "insertion": "Steve"}],
		"extra": [" ", {"translate": "chat.type.text", "with": ["a", {"translate": "commands.kill.success.single", "with": ["b"]}]}]
	})
}

fn system_chat(content: Value) -> z::play_to_client_packet::Ty {
	z::play_to_client_packet::Ty::SystemChat(z::play_to_client_packet_system_chat::Ty {
		content: text(content),
		is_action_bar: false,
	})
}

#[test]
fn nested_translations_reach_netty_clients() {
	setup();
	for (version, json) in [
		("1.12.2", {
			let out = down(
				&system_chat(death()),
				"1.12.2",
				v1_12_2::play_to_client_packet::deserialize,
			);
			let [v1_12_2::play_to_client_packet::Ty::Chat(c)] = out.as_slice() else {
				panic!("{out:?}")
			};
			c.message.clone()
		}),
		("1.8", {
			let out = down(
				&system_chat(death()),
				"1.8",
				v1_8::play_to_client_packet::deserialize,
			);
			let [v1_8::play_to_client_packet::Ty::Chat(c)] = out.as_slice() else {
				panic!("{out:?}")
			};
			c.message.clone()
		}),
	] {
		let v: Value = serde_json::from_str(&json).unwrap();
		assert_eq!(
			render(&v, &[OLD]),
			"Steve was killed <a> Killed b",
			"{version}: {json}"
		);
		assert!(
			json.contains("chat.type.text") && json.contains("gold") && json.contains("insertion"),
			"{version}: {json}"
		);
	}
}

#[test]
fn known_translations_stay_for_new_clients() {
	setup();
	let out = down(
		&system_chat(death()),
		"1.21.11",
		v1_21_11::play_to_client_packet::deserialize,
	);
	let [v1_21_11::play_to_client_packet::Ty::SystemChat(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(c.content, text(death()));
	let out = down(
		&system_chat(death()),
		"1.16",
		v1_16::play_to_client_packet::deserialize,
	);
	let [v1_16::play_to_client_packet::Ty::Chat(c)] = out.as_slice() else {
		panic!("{out:?}")
	};
	let v: Value = serde_json::from_str(&c.message).unwrap();
	assert_eq!(render(&v, &[OLD, MID]), "Steve was killed <a> Killed b");
	assert!(
		c.message.contains("commands.kill.success.single") && !c.message.contains("genericKill"),
		"{}",
		c.message
	);
}

#[test]
fn pre_netty_clients_read_rendered_text() {
	setup();
	let expected = "\u{a7}cSteve\u{a7}6 was killed <a> Killed b";
	let out = down(
		&system_chat(death()),
		"1.5.2",
		v1_5_2::play_to_client_packet::deserialize,
	);
	assert!(
		matches!(out.as_slice(), [v1_5_2::play_to_client_packet::Ty::Chat(m)] if m == expected),
		"{out:?}"
	);
	let out = down(
		&system_chat(death()),
		"1.0.1",
		v1_0_1::play_to_client_packet::deserialize,
	);
	assert!(
		matches!(out.as_slice(), [v1_0_1::play_to_client_packet::Ty::Chat(m)] if m == expected),
		"{out:?}"
	);
}

#[test]
fn kicks_and_titles_are_readable() {
	setup();
	let kick = z::play_to_client_packet::Ty::KickDisconnect(text(
		json!({"translate": "multiplayer.disconnect.kicked"}),
	));
	let out = down(&kick, "1.8", v1_8::play_to_client_packet::deserialize);
	let [v1_8::play_to_client_packet::Ty::KickDisconnect(reason)] = out.as_slice() else {
		panic!("{out:?}")
	};
	assert_eq!(
		render(&serde_json::from_str(reason).unwrap(), &[OLD]),
		"Kicked by an operator"
	);
	let out = down(&kick, "1.5.2", v1_5_2::play_to_client_packet::deserialize);
	assert!(
		matches!(out.as_slice(), [v1_5_2::play_to_client_packet::Ty::KickDisconnect(r)] if r == "Kicked by an operator"),
		"{out:?}"
	);

	let title = z::play_to_client_packet::Ty::SetTitleText(text(
		json!({"translate": "death.attack.genericKill", "with": ["Alex"]}),
	));
	for version in ["1.12.2", "1.8"] {
		let titles: Vec<String> = match version {
			"1.12.2" => down(&title, version, v1_12_2::play_to_client_packet::deserialize)
				.into_iter()
				.filter_map(|p| match p {
					v1_12_2::play_to_client_packet::Ty::Title(
						v1_12_2::play_to_client_packet_title::Ty::V0(t),
					) => Some(t),
					_ => None,
				})
				.collect(),
			_ => down(&title, version, v1_8::play_to_client_packet::deserialize)
				.into_iter()
				.filter_map(|p| match p {
					v1_8::play_to_client_packet::Ty::Title(
						v1_8::play_to_client_packet_title::Ty::V0(t),
					) => Some(t),
					_ => None,
				})
				.collect(),
		};
		let [t] = titles.as_slice() else {
			panic!("{version}: {titles:?}")
		};
		assert_eq!(
			render(&serde_json::from_str(t).unwrap(), &[OLD]),
			"Alex was killed",
			"{version}"
		);
	}
}

fn sword(name: Value) -> z::play_to_client_packet::Ty {
	let id = registry::names("26.1", "items")
		.unwrap()
		.id("diamond_sword")
		.unwrap() as i32;
	let item = z::slot::Ty::Default(
		1,
		z::slot_default::Ty {
			item_id: id,
			added_component_count: 1,
			removed_component_count: 0,
			components: vec![z::slot_component::Ty::CustomName(text(name))],
			remove_components: vec![],
		},
	);
	z::play_to_client_packet::Ty::SetSlot(z::play_to_client_packet_set_slot::Ty {
		window_id: 0,
		state_id: 1,
		slot: 36,
		item,
	})
}

fn contains(haystack: &[u8], needle: &str) -> bool {
	haystack
		.windows(needle.len())
		.any(|w| w == needle.as_bytes())
}

#[test]
fn item_names_follow_the_client_language() {
	setup();
	let p = sword(
		json!({"translate": "commands.kill.success.single", "with": ["Sword"], "color": "aqua"}),
	);
	let bytes = encode(&p, z::play_to_client_packet::serialize);
	let to_1_16 = convert(
		Root::PlayToClient,
		at("26.1"),
		at("1.16"),
		&bytes,
		&mut Session::new(),
	)
	.unwrap();
	let [item] = to_1_16.as_slice() else {
		panic!("{to_1_16:?}")
	};
	assert!(contains(item, "commands.kill.success.single"), "{item:?}");
	let to_1_12_2 = convert(
		Root::PlayToClient,
		at("26.1"),
		at("1.12.2"),
		&bytes,
		&mut Session::new(),
	)
	.unwrap();
	let [item] = to_1_12_2.as_slice() else {
		panic!("{to_1_12_2:?}")
	};
	assert!(
		contains(item, "\u{0}\u{4}Name\u{0}\u{f}\u{a7}bKilled Sword"),
		"{item:?}"
	);
}
