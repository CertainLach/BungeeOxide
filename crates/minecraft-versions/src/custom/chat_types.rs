use std::collections::HashMap;

use nbt::{CompoundExt, Entry, List, ListItems, Payload, Tag};
use serde_json::{Map, Value};

use super::localize::{component, localize};
use crate::gen::convert::Version;
use crate::pds::convert::Result;
use crate::registry;
use crate::Session;

#[derive(Clone, Copy, PartialEq)]
pub enum Parameter {
	Sender,
	Target,
	Content,
}

impl Parameter {
	fn named(name: &str) -> Parameter {
		match name.strip_prefix("minecraft:").unwrap_or(name) {
			"sender" => Parameter::Sender,
			"target" | "team_name" => Parameter::Target,
			_ => Parameter::Content,
		}
	}
}

#[derive(Clone, PartialEq)]
pub struct Decoration {
	pub key: String,
	pub parameters: Vec<Parameter>,
	pub style: Vec<Entry>,
}

#[derive(Clone)]
pub struct ChatType {
	pub chat: Option<Decoration>,
	pub overlay: bool,
}

impl Decoration {
	fn read(p: &Payload) -> Option<Decoration> {
		Some(Decoration {
			key: p.field("translation_key")?.as_str()?.to_owned(),
			parameters: p
				.field("parameters")
				.and_then(Payload::as_list)
				.and_then(List::strings)
				.unwrap_or_default()
				.iter()
				.map(|n| Parameter::named(n))
				.collect(),
			style: p
				.field("style")
				.and_then(Payload::as_compound)
				.cloned()
				.unwrap_or_default(),
		})
	}

	fn arguments<T: Clone>(
		&self,
		sender: &T,
		target: Option<&T>,
		content: &T,
		empty: &T,
	) -> Vec<T> {
		self.parameters
			.iter()
			.map(|p| match p {
				Parameter::Sender => sender,
				Parameter::Target => target.unwrap_or(empty),
				Parameter::Content => content,
			})
			.cloned()
			.collect()
	}

	fn json(&self, sender: &str, target: Option<&str>, content: &str) -> String {
		let target = target.map(nbt::json::parse_text);
		let with = self.arguments(
			&nbt::json::parse_text(sender),
			target.as_ref(),
			&nbt::json::parse_text(content),
			&Value::String(String::new()),
		);
		let mut out = match nbt::json::text_from_nbt(&Payload::Compound(self.style.clone())) {
			Value::Object(style) => style,
			_ => Map::new(),
		};
		out.insert("translate".to_owned(), Value::String(self.key.clone()));
		if !with.is_empty() {
			out.insert("with".to_owned(), Value::Array(with));
		}
		Value::Object(out).to_string()
	}

	pub fn nbt(&self, sender: &Tag, target: Option<&Tag>, content: &Tag) -> Tag {
		let with: Vec<Vec<Entry>> = self
			.arguments(sender, target, content, &Tag::new(""))
			.into_iter()
			.map(|t| component(t.value))
			.collect();
		let mut entries = vec![Entry::new("translate", self.key.clone())];
		if !with.is_empty() {
			entries.push(Entry::new("with", ListItems::Compound(with)));
		}
		entries.extend(
			self.style
				.iter()
				.filter(|e| e.name != "translate" && e.name != "with")
				.cloned(),
		);
		Tag::new(entries)
	}
}

impl ChatType {
	pub fn read(element: &Payload) -> ChatType {
		let chat = element.field("chat");
		ChatType {
			chat: chat.and_then(|c| Decoration::read(c.field("decoration").unwrap_or(c))),
			overlay: chat.is_none() && element.field("overlay").is_some(),
		}
	}
}

#[derive(Default)]
struct Registries(HashMap<Version, HashMap<i32, ChatType>>);

fn from_codec(codec: &Payload) -> Option<HashMap<i32, ChatType>> {
	let entries = codec
		.field("minecraft:chat_type")?
		.field("value")?
		.compounds();
	Some(
		entries
			.iter()
			.enumerate()
			.filter_map(|(i, e)| {
				let id = e.field("id").and_then(Payload::to_i32).unwrap_or(i as i32);
				Some((id, ChatType::read(e.field("element")?)))
			})
			.collect(),
	)
}

fn vanilla(version: Version) -> HashMap<i32, ChatType> {
	let json = registry::login_packet(version).and_then(|t| serde_json::from_str::<Value>(t).ok());
	json.and_then(|j| nbt::json::typed::read(j.get("dimensionCodec")?))
		.and_then(|c| from_codec(&c))
		.unwrap_or_default()
}

pub fn track(cx: &mut Session, version: Version, codec: &Entry) {
	if let Some(types) = from_codec(&codec.value) {
		cx.state::<Registries>().0.insert(version, types);
	}
}

pub fn chat_type(cx: &mut Session, version: Version, id: i32) -> Option<ChatType> {
	cx.state::<Registries>()
		.0
		.entry(version)
		.or_insert_with(|| vanilla(version))
		.get(&id)
		.cloned()
}

pub fn decorate(
	cx: &mut Session,
	(from, to): (Version, Version),
	id: i32,
	sender: &str,
	target: Option<&str>,
	content: &str,
) -> Result<(String, bool)> {
	let (mut text, overlay) = match chat_type(cx, from, id) {
		Some(ChatType { chat: Some(d), .. }) => (d.json(sender, target, content), false),
		Some(t) => (content.to_owned(), t.overlay),
		None => (content.to_owned(), false),
	};
	localize(&mut text, from.index(), to.index(), cx)?;
	Ok((text, overlay))
}
