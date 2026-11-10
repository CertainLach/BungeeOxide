use nbt::{CompoundExt, CompoundVecExt, Entry, List, ListItems, Payload, Tag};
use serde_json::{Map, Value};

use crate::gen::convert::Version;
use crate::languages::{self, pieces, Languages, Piece};
use crate::pds::convert::Result;
use crate::Session;

const KEY: &str = "\"translate\"";

pub struct Target<'a> {
	languages: &'a Languages,
	version: usize,
	fallback: bool,
}

impl<'a> Target<'a> {
	pub fn new(languages: &'a Languages, version: usize) -> Self {
		Self {
			languages,
			version,
			fallback: version >= Version::V1_19_4.index(),
		}
	}

	fn pattern(&self, key: &str, fallback: Option<&str>) -> Option<String> {
		if self.languages.knows(self.version, key)? {
			return None;
		}
		match (self.languages.pattern(key), fallback) {
			(Some(_), Some(f)) => Some(f.to_owned()),
			(Some(p), None) => Some(p.to_owned()),
			(None, Some(f)) if !self.fallback => Some(f.to_owned()),
			_ => None,
		}
	}
}

pub trait Text {
	fn mentions(&self) -> bool;
	fn rewrite(&mut self, target: &Target) -> bool;
}

impl Text for String {
	fn mentions(&self) -> bool {
		self.contains(KEY)
	}

	fn rewrite(&mut self, target: &Target) -> bool {
		json_text(self, target)
	}
}

impl Text for Tag {
	fn mentions(&self) -> bool {
		names_key(&self.value)
	}

	fn rewrite(&mut self, target: &Target) -> bool {
		nbt(&mut self.value, target)
	}
}

impl<T: Text> Text for Option<T> {
	fn mentions(&self) -> bool {
		self.as_ref().is_some_and(T::mentions)
	}

	fn rewrite(&mut self, target: &Target) -> bool {
		self.as_mut().is_some_and(|t| t.rewrite(target))
	}
}

impl<T: Text> Text for Vec<T> {
	fn mentions(&self) -> bool {
		self.iter().any(T::mentions)
	}

	fn rewrite(&mut self, target: &Target) -> bool {
		self.iter_mut()
			.fold(false, |changed, t| t.rewrite(target) | changed)
	}
}

pub trait Data {
	fn mentions(&self) -> bool;
	fn rewrite(&mut self, target: &Target) -> bool;
}

impl Data for Tag {
	fn mentions(&self) -> bool {
		mentions_data(&self.value)
	}

	fn rewrite(&mut self, target: &Target) -> bool {
		data(&mut self.value, target)
	}
}

impl Data for Entry {
	fn mentions(&self) -> bool {
		mentions_data(&self.value)
	}

	fn rewrite(&mut self, target: &Target) -> bool {
		data(&mut self.value, target)
	}
}

impl Data for Vec<u8> {
	fn mentions(&self) -> bool {
		false
	}

	fn rewrite(&mut self, _target: &Target) -> bool {
		false
	}
}

impl<T: Data> Data for Option<T> {
	fn mentions(&self) -> bool {
		self.as_ref().is_some_and(T::mentions)
	}

	fn rewrite(&mut self, target: &Target) -> bool {
		self.as_mut().is_some_and(|t| t.rewrite(target))
	}
}

pub fn localize<T: Text>(value: &mut T, _from: usize, to: usize, _cx: &mut Session) -> Result<()> {
	if value.mentions() {
		if let Some(languages) = languages::get() {
			value.rewrite(&Target::new(languages, to));
		}
	}
	Ok(())
}

pub fn localize_nbt<T: Data>(
	value: &mut T,
	_from: usize,
	to: usize,
	_cx: &mut Session,
) -> Result<()> {
	if value.mentions() {
		if let Some(languages) = languages::get() {
			value.rewrite(&Target::new(languages, to));
		}
	}
	Ok(())
}

pub fn localize_status(
	value: &mut String,
	_from: usize,
	to: usize,
	_cx: &mut Session,
) -> Result<()> {
	if !value.contains(KEY) {
		return Ok(());
	}
	let Some(languages) = languages::get() else {
		return Ok(());
	};
	let Ok(mut status) = serde_json::from_str::<Value>(value) else {
		return Ok(());
	};
	if let Some(description) = status.get_mut("description") {
		if json(description, &Target::new(languages, to)) {
			*value = status.to_string();
		}
	}
	Ok(())
}

pub fn legacy(json: &str) -> String {
	super::v1_7_to_1_8::text::to_legacy(json)
}

fn json_text(text: &mut String, target: &Target) -> bool {
	if !text.contains(KEY) {
		return false;
	}
	let Ok(mut value) = serde_json::from_str::<Value>(text) else {
		return false;
	};
	if !json(&mut value, target) {
		return false;
	}
	*text = value.to_string();
	true
}

pub fn json(value: &mut Value, target: &Target) -> bool {
	match value {
		Value::Array(items) => items
			.iter_mut()
			.fold(false, |changed, v| json(v, target) | changed),
		Value::Object(map) => {
			let changed = map
				.values_mut()
				.fold(false, |changed, v| json(v, target) | changed);
			let pattern = map
				.get("translate")
				.and_then(Value::as_str)
				.and_then(|key| target.pattern(key, map.get("fallback").and_then(Value::as_str)));
			match pattern {
				Some(pattern) => {
					literal_json(map, &pattern);
					true
				}
				None => changed,
			}
		}
		_ => false,
	}
}

fn literal_json(map: &mut Map<String, Value>, pattern: &str) {
	let args = match map.remove("with") {
		Some(Value::Array(args)) => args,
		Some(arg) => vec![arg],
		None => vec![],
	};
	map.remove("translate");
	map.remove("fallback");
	let mut parts: Vec<Value> = Vec::new();
	for piece in pieces(pattern) {
		let part = match piece {
			Piece::Text(t) => Value::String(t.to_owned()),
			Piece::Arg(i) => match args.get(i) {
				Some(a @ (Value::String(_) | Value::Object(_) | Value::Array(_))) => a.clone(),
				Some(Value::Null) | None => continue,
				Some(other) => Value::String(other.to_string()),
			},
		};
		match (parts.last_mut(), part) {
			(Some(Value::String(last)), Value::String(s)) => last.push_str(&s),
			(_, part) => parts.push(part),
		}
	}
	let text = match parts.first() {
		Some(Value::String(_)) => parts.remove(0),
		_ => Value::String(String::new()),
	};
	match map.remove("extra") {
		Some(Value::Array(extra)) => parts.extend(extra),
		Some(extra) => parts.push(extra),
		None => {}
	}
	map.insert("text".to_owned(), text);
	if !parts.is_empty() {
		map.insert("extra".to_owned(), Value::Array(parts));
	}
}

fn names_key(p: &Payload) -> bool {
	mentions(p, false)
}

fn mentions_data(p: &Payload) -> bool {
	mentions(p, true)
}

fn mentions(p: &Payload, strings: bool) -> bool {
	fn entries(c: &[Entry], strings: bool) -> bool {
		c.iter()
			.any(|e| e.name == "translate" || mentions(&e.value, strings))
	}
	fn list(l: &List, strings: bool) -> bool {
		match &l.items {
			ListItems::string(ss) => strings && ss.iter().any(|s| s.contains(KEY)),
			ListItems::compound(cs) => cs.iter().any(|c| entries(c, strings)),
			ListItems::list(ls) => ls.iter().any(|l| list(l, strings)),
			_ => false,
		}
	}
	match p {
		Payload::string(s) => strings && s.contains(KEY),
		Payload::compound(c) => entries(c, strings),
		Payload::list(l) => list(l, strings),
		_ => false,
	}
}

pub fn nbt(p: &mut Payload, target: &Target) -> bool {
	match p {
		Payload::compound(entries) => compound(entries, target, false),
		Payload::list(l) => list(l, target, false),
		_ => false,
	}
}

fn data(p: &mut Payload, target: &Target) -> bool {
	match p {
		Payload::string(s) => json_text(s, target),
		Payload::compound(entries) => compound(entries, target, true),
		Payload::list(l) => list(l, target, true),
		_ => false,
	}
}

fn list(l: &mut List, target: &Target, strings: bool) -> bool {
	match &mut l.items {
		ListItems::string(ss) if strings => ss
			.iter_mut()
			.fold(false, |changed, s| json_text(s, target) | changed),
		ListItems::compound(cs) => cs
			.iter_mut()
			.fold(false, |changed, c| compound(c, target, strings) | changed),
		ListItems::list(ls) => ls
			.iter_mut()
			.fold(false, |changed, l| list(l, target, strings) | changed),
		_ => false,
	}
}

fn compound(entries: &mut Vec<Entry>, target: &Target, strings: bool) -> bool {
	let changed = entries.iter_mut().fold(false, |changed, e| {
		let inner = if strings {
			data(&mut e.value, target)
		} else {
			nbt(&mut e.value, target)
		};
		inner | changed
	});
	let pattern = entries
		.field("translate")
		.and_then(Payload::as_str)
		.and_then(|key| target.pattern(key, entries.field("fallback").and_then(Payload::as_str)));
	match pattern {
		Some(pattern) => {
			literal_nbt(entries, &pattern);
			true
		}
		None => changed,
	}
}

fn plain(text: String) -> Vec<Entry> {
	vec![Entry::new("text", Payload::string(text))]
}

pub(super) fn component(p: Payload) -> Vec<Entry> {
	match p {
		Payload::compound(mut c) if c.len() == 1 && c[0].name.is_empty() => {
			component(c.remove(0).value)
		}
		Payload::compound(c) => c,
		Payload::string(s) => plain(s),
		Payload::list(l) => {
			let parts: Vec<Vec<Entry>> = l.into_values().into_iter().map(component).collect();
			vec![
				Entry::new("text", ""),
				Entry::new("extra", ListItems::compound(parts)),
			]
		}
		Payload::byte(v) => plain(v.to_string()),
		Payload::short(v) => plain(v.to_string()),
		Payload::int(v) => plain(v.to_string()),
		Payload::long(v) => plain(v.to_string()),
		Payload::float(v) => plain(v.to_string()),
		Payload::double(v) => plain(v.to_string()),
		_ => plain(String::new()),
	}
}

fn plain_text(c: &mut [Entry]) -> Option<&mut String> {
	match c {
		[Entry {
			name,
			value: Payload::string(s),
			..
		}] if name == "text" => Some(s),
		_ => None,
	}
}

fn literal_nbt(entries: &mut Vec<Entry>, pattern: &str) {
	let args = match entries.take("with") {
		Some(Payload::list(l)) => l.into_values(),
		Some(arg) => vec![arg],
		None => vec![],
	};
	entries.take("translate");
	entries.take("fallback");
	let mut parts: Vec<Vec<Entry>> = Vec::new();
	for piece in pieces(pattern) {
		let mut part = match piece {
			Piece::Text(t) => plain(t.to_owned()),
			Piece::Arg(i) => match args.get(i) {
				Some(a) => component(a.clone()),
				None => continue,
			},
		};
		match (
			parts.last_mut().and_then(|c| plain_text(c)),
			plain_text(&mut part),
		) {
			(Some(last), Some(s)) => last.push_str(s),
			_ => parts.push(part),
		}
	}
	let text = match parts.first_mut().and_then(|c| plain_text(c)) {
		Some(s) => {
			let s = std::mem::take(s);
			parts.remove(0);
			s
		}
		None => String::new(),
	};
	match entries.take("extra") {
		Some(Payload::list(l)) => parts.extend(l.into_values().into_iter().map(component)),
		Some(extra) => parts.push(component(extra)),
		None => {}
	}
	entries.take("text");
	entries.push(Entry::new("text", Payload::string(text)));
	if !parts.is_empty() {
		entries.push(Entry::new("extra", ListItems::compound(parts)));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::gen::convert::index_of;
	use serde_json::json;

	fn strings(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
		pairs
			.iter()
			.map(|(k, v)| (k.to_string(), v.to_string()))
			.collect()
	}

	fn languages() -> Languages {
		let old = [("chat.type.text", "<%s> %s")];
		let new = [
			("chat.type.text", "<%s> %s"),
			("death.attack.genericKill", "%1$s was killed"),
			("gui.swap", "%2$s, then %1$s"),
		];
		Languages::new(
			[("1.12.2", "old"), ("1.20.5", "new")],
			[("old", strings(&old)), ("new", strings(&new))],
		)
	}

	fn at(v: &str) -> usize {
		index_of(v).unwrap()
	}

	#[test]
	fn unknown_keys_become_literal_text_with_their_arguments() {
		let l = languages();
		let mut v = json!({
			"translate": "death.attack.genericKill",
			"color": "gold",
			"clickEvent": {"action": "suggest_command", "value": "/msg Steve"},
			"with": [{"text": "Steve", "color": "red"}],
			"extra": [" ", {"translate": "chat.type.text", "with": ["a", {"translate": "gui.swap", "with": ["x", 5]}]}]
		});
		assert!(json(&mut v, &Target::new(&l, at("1.12.2"))));
		assert_eq!(
			v,
			json!({
				"text": "",
				"color": "gold",
				"clickEvent": {"action": "suggest_command", "value": "/msg Steve"},
				"extra": [
					{"text": "Steve", "color": "red"},
					" was killed",
					" ",
					{"translate": "chat.type.text", "with": ["a", {"text": "5, then x"}]}
				]
			})
		);
	}

	#[test]
	fn known_keys_and_unknown_data_stay() {
		let l = languages();
		let mut v = json!({"translate": "gui.swap", "with": ["a", "b"]});
		assert!(!json(&mut v, &Target::new(&l, at("1.20.5"))));
		assert!(!json(&mut v, &Target::new(&l, at("26.1"))));
		let mut custom = json!({"translate": "custom.key"});
		assert!(!json(&mut custom, &Target::new(&l, at("1.12.2"))));
		let mut text = String::from("{\"text\":\"plain\"}");
		assert!(!text.rewrite(&Target::new(&l, at("1.12.2"))));
	}

	#[test]
	fn fallback_wins_and_covers_keys_without_data() {
		let l = languages();
		let mut v = json!({"translate": "custom.key", "fallback": "Hi %s", "with": ["you"]});
		assert!(!json(&mut v.clone(), &Target::new(&l, at("1.19.4"))));
		assert!(json(&mut v, &Target::new(&l, at("1.19.3"))));
		assert_eq!(v, json!({"text": "Hi you"}));
		let mut known = json!({"translate": "death.attack.genericKill", "fallback": "%s fell", "with": ["Bob"]});
		assert!(json(&mut known, &Target::new(&l, at("1.12.2"))));
		assert_eq!(known, json!({"text": "Bob fell"}));
	}

	fn compound(entries: Vec<(&str, Payload)>) -> Payload {
		Payload::compound(entries.into_iter().map(|(n, v)| Entry::new(n, v)).collect())
	}

	fn s(v: &str) -> Payload {
		Payload::string(v.to_owned())
	}

	#[test]
	fn nbt_components_keep_their_style() {
		let l = languages();
		let arg = vec![
			Entry::new("text", s("Steve")),
			Entry::new("color", s("red")),
		];
		let mut tag = Tag::new(compound(vec![
			("translate", s("death.attack.genericKill")),
			("bold", Payload::byte(1)),
			("with", ListItems::compound(vec![arg.clone()]).into()),
			("extra", ListItems::string(vec!["!".to_owned()]).into()),
		]));
		assert!(Text::mentions(&tag));
		assert!(Text::rewrite(&mut tag, &Target::new(&l, at("1.12.2"))));
		let expected = compound(vec![
			("bold", Payload::byte(1)),
			("text", s("")),
			(
				"extra",
				ListItems::compound(vec![
					arg,
					plain(" was killed".to_owned()),
					plain("!".to_owned()),
				])
				.into(),
			),
		]);
		assert_eq!(tag.value, expected);
		let mut known = Tag::new(compound(vec![("translate", s("chat.type.text"))]));
		assert!(!Text::rewrite(&mut known, &Target::new(&l, at("1.12.2"))));
	}

	#[test]
	fn item_data_holds_components_as_json_strings() {
		let l = languages();
		let name = r#"{"translate":"gui.swap","with":["a","b"],"italic":false}"#;
		let lore = Payload::from(ListItems::string(vec![
			name.to_owned(),
			"\"plain\"".to_owned(),
		]));
		let mut item = Some(Entry::new(
			"",
			compound(vec![(
				"display",
				compound(vec![("Name", s(name)), ("Lore", lore)]),
			)]),
		));
		assert!(item.mentions());
		assert!(item.rewrite(&Target::new(&l, at("1.12.2"))));
		let Some(Entry {
			value: Payload::compound(root),
			..
		}) = &item
		else {
			panic!()
		};
		let Payload::compound(display) = &root[0].value else {
			panic!()
		};
		let Payload::string(name) = &display[0].value else {
			panic!()
		};
		assert_eq!(
			serde_json::from_str::<Value>(name).unwrap(),
			json!({"text": "b, then a", "italic": false})
		);
	}
}
