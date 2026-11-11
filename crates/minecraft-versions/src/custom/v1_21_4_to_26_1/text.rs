use nbt::{snbt, CompoundExt, Entry, List, ListItems, Payload, Tag};
use serde_json::Value;

fn map_list(items: ListItems, f: fn(Payload) -> Payload) -> ListItems {
	match items {
		ListItems::Compound(list) => ListItems::Compound(
			list.into_iter()
				.map(|c| match f(Payload::Compound(c)) {
					Payload::Compound(c) => c,
					_ => Vec::new(),
				})
				.collect(),
		),
		other => other,
	}
}

fn text_list(items: ListItems) -> ListItems {
	map_list(items, payload)
}

fn rename(entries: &mut [Entry], from: &str, to: &str) {
	if entries.field(to).is_none() {
		entries.rename(from, to);
	}
}

fn click(mut event: Payload) -> Option<Payload> {
	let action = event.field("action").and_then(Payload::as_str)?.to_owned();
	let value = event.take("value");
	let key = match action.as_str() {
		"open_url" => "url",
		"run_command" | "suggest_command" => "command",
		"open_file" => "path",
		"change_page" => "page",
		_ => "value",
	};
	match (key, value) {
		("page", Some(Payload::String(s))) => event.put(
			"page",
			Payload::Int(s.trim().parse().ok().filter(|p| *p >= 1)?),
		),
		("url", Some(Payload::String(s)))
			if !(s.starts_with("http://") || s.starts_with("https://")) =>
		{
			return None
		}
		("command", Some(Payload::String(s))) => {
			let clean: String = s
				.chars()
				.filter(|c| *c != '\u{a7}' && (*c >= ' ' || *c == '\n') && *c != '\u{7f}')
				.collect();
			event.put("command", Payload::String(clean));
		}
		(key, Some(v)) => event.put(key, v),
		(_, None) => {}
	}
	Some(event)
}

fn legacy_snbt(value: &Payload) -> Option<Payload> {
	let text = match value {
		Payload::String(s) => s.as_str(),
		p => p.field("text").and_then(Payload::as_str)?,
	};
	snbt::parse(text)
}

fn legacy_name(name: &str) -> Payload {
	serde_json::from_str::<Value>(name)
		.ok()
		.filter(|v| v.is_object() || v.is_array())
		.and_then(|v| nbt::json::typed::read(&v))
		.map(payload)
		.unwrap_or_else(|| Payload::from(name))
}

fn legacy_hover(mut event: Payload, action: &str, value: Payload) -> Option<Payload> {
	match action {
		"show_entity" => {
			let tag = legacy_snbt(&value)?;
			let ty = tag
				.field("type")
				.and_then(Payload::as_str)
				.filter(|t| !t.is_empty())
				.unwrap_or("minecraft:player");
			event.put("id", ty);
			event.put("uuid", tag.field("id").and_then(Payload::as_str)?);
			if let Some(name) = tag.field("name").and_then(Payload::as_str) {
				event.put("name", legacy_name(name));
			}
		}
		"show_item" => {
			let tag = legacy_snbt(&value)?;
			event.put("id", tag.field("id").and_then(Payload::as_str)?);
			let count = tag
				.field("Count")
				.and_then(Payload::as_i64)
				.or_else(|| tag.field("count").and_then(Payload::as_i64))
				.unwrap_or(1);
			event.put("count", Payload::Int(count.clamp(1, 99) as i32));
		}
		_ => event.put("value", payload(value)),
	}
	Some(event)
}

fn hover(mut event: Payload) -> Option<Payload> {
	let action = event
		.field("action")
		.and_then(Payload::as_str)
		.unwrap_or_default()
		.to_owned();
	let Some(contents) = event.take("contents") else {
		return match event.take("value") {
			Some(value) => legacy_hover(event, &action, value),
			None => Some(event),
		};
	};
	match action.as_str() {
		"show_text" => event.put("value", payload(contents)),
		"show_entity" => {
			if let Some(ty) = contents.field("type") {
				event.put("id", ty.clone());
			}
			if let Some(id) = contents.field("id") {
				event.put("uuid", id.clone());
			}
			if let Some(name) = contents.field("name") {
				event.put("name", payload(name.clone()));
			}
		}
		"show_item" => match contents {
			Payload::Compound(entries) => {
				for e in entries {
					event.put(&e.name, e.value);
				}
			}
			Payload::String(id) => event.put("id", Payload::String(id)),
			_ => {}
		},
		_ => event.put("contents", contents),
	}
	Some(event)
}

pub fn payload(p: Payload) -> Payload {
	match p {
		Payload::Compound(mut entries) => {
			rename(&mut entries, "clickEvent", "click_event");
			rename(&mut entries, "hoverEvent", "hover_event");
			Payload::Compound(
				entries
					.into_iter()
					.filter_map(|e| {
						let value = match e.name.as_str() {
							"click_event" => click(e.value)?,
							"hover_event" => hover(e.value)?,
							"extra" | "with" => match e.value {
								Payload::List(l) => Payload::List(List::new(text_list(l.items))),
								v => v,
							},
							"separator" => payload(e.value),
							_ => e.value,
						};
						Some(Entry::new(&e.name, value))
					})
					.collect(),
			)
		}
		Payload::List(l) => Payload::List(List::new(text_list(l.items))),
		other => other,
	}
}

pub fn up(tag: Tag) -> Tag {
	Tag::new(payload(tag.value))
}

fn click_down(mut event: Payload) -> Payload {
	let key = match event
		.field("action")
		.and_then(Payload::as_str)
		.unwrap_or_default()
	{
		"open_url" => "url",
		"run_command" | "suggest_command" => "command",
		"open_file" => "path",
		"change_page" => "page",
		_ => return event,
	};
	match event.take(key) {
		Some(Payload::Int(page)) => event.put("value", Payload::String(page.to_string())),
		Some(v) => event.put("value", v),
		None => {}
	}
	event
}

fn hover_down(mut event: Payload) -> Payload {
	let action = event
		.field("action")
		.and_then(Payload::as_str)
		.unwrap_or_default()
		.to_owned();
	let mut contents = Payload::Compound(vec![]);
	let fields: &[(&str, &str)] = match action.as_str() {
		"show_text" => {
			if let Some(value) = event.take("value") {
				event.put("contents", payload_down(value));
			}
			return event;
		}
		"show_entity" => &[("id", "type"), ("uuid", "id"), ("name", "name")],
		"show_item" => &[
			("id", "id"),
			("count", "count"),
			("components", "components"),
		],
		_ => return event,
	};
	for (from, to) in fields {
		if let Some(v) = event.take(from) {
			contents.put(to, if *to == "name" { payload_down(v) } else { v });
		}
	}
	event.put("contents", contents);
	event
}

pub fn payload_down(p: Payload) -> Payload {
	match p {
		Payload::Compound(mut entries) => {
			rename(&mut entries, "click_event", "clickEvent");
			rename(&mut entries, "hover_event", "hoverEvent");
			Payload::Compound(
				entries
					.into_iter()
					.map(|e| {
						let value = match e.name.as_str() {
							"clickEvent" => click_down(e.value),
							"hoverEvent" => hover_down(e.value),
							"extra" | "with" => match e.value {
								Payload::List(l) => {
									Payload::List(List::new(map_list(l.items, payload_down)))
								}
								v => v,
							},
							"separator" => payload_down(e.value),
							_ => e.value,
						};
						Entry::new(&e.name, value)
					})
					.collect(),
			)
		}
		Payload::List(l) => Payload::List(List::new(map_list(l.items, payload_down))),
		other => other,
	}
}

pub fn down(tag: Tag) -> Tag {
	Tag::new(payload_down(tag.value))
}

fn undialog_payload(p: Payload) -> Payload {
	match p {
		Payload::Compound(entries) => Payload::Compound(
			entries
				.into_iter()
				.filter(|e| {
					e.name != "click_event"
						|| !matches!(
							e.value.field("action").and_then(Payload::as_str),
							Some("show_dialog" | "custom")
						)
				})
				.map(|e| {
					let value = match e.name.as_str() {
						"extra" | "with" => match e.value {
							Payload::List(l) => {
								Payload::List(List::new(map_list(l.items, undialog_payload)))
							}
							v => v,
						},
						"separator" | "hover_event" | "value" | "name" => undialog_payload(e.value),
						_ => e.value,
					};
					Entry::new(&e.name, value)
				})
				.collect(),
		),
		Payload::List(l) => Payload::List(List::new(map_list(l.items, undialog_payload))),
		other => other,
	}
}

pub fn undialog(tag: Tag) -> Tag {
	Tag::new(undialog_payload(tag.value))
}

fn unobject_payload(p: Payload) -> Payload {
	match p {
		Payload::Compound(mut entries) => {
			let object = entries.iter().any(|e| match (e.name.as_str(), &e.value) {
				("type", Payload::String(t)) => t == "object",
				(name, _) => matches!(name, "sprite" | "player"),
			});
			if object {
				let fallback = entries
					.field("fallback")
					.cloned()
					.unwrap_or_else(|| Payload::from(""));
				entries.retain(|e| {
					!matches!(
						e.name.as_str(),
						"type" | "sprite" | "player" | "atlas" | "fallback" | "hat"
					)
				});
				entries.push(Entry::new("text", fallback));
			}
			Payload::Compound(
				entries
					.into_iter()
					.map(|e| {
						let value = match e.name.as_str() {
							"extra" | "with" => match e.value {
								Payload::List(l) => {
									Payload::List(List::new(map_list(l.items, unobject_payload)))
								}
								v => v,
							},
							"separator" | "hover_event" | "value" | "name" => {
								unobject_payload(e.value)
							}
							_ => e.value,
						};
						Entry::new(&e.name, value)
					})
					.collect(),
			)
		}
		Payload::List(l) => Payload::List(List::new(map_list(l.items, unobject_payload))),
		other => other,
	}
}

pub fn unobject(tag: Tag) -> Tag {
	Tag::new(unobject_payload(tag.value))
}

const BOOLS: &[&str] = &[
	"bold",
	"italic",
	"underlined",
	"strikethrough",
	"obfuscated",
	"interpret",
];

pub fn to_json(p: Payload) -> String {
	nbt::json::from_nbt_with_booleans(&payload_down(p), BOOLS).to_string()
}

pub fn json_lines(p: &Payload) -> Option<Vec<String>> {
	let Payload::List(l) = p else { return None };
	Some(l.values().into_iter().map(to_json).collect())
}

fn json_value(v: &Value) -> Payload {
	match v {
		Value::Array(items) => Payload::List(List::new(ListItems::Compound(
			items
				.iter()
				.map(|i| match json_component(i) {
					Payload::Compound(c) => c,
					other => vec![Entry::new("text", other)],
				})
				.collect(),
		))),
		Value::Object(map) => Payload::Compound(
			map.iter()
				.map(|(k, v)| {
					let value = match k.as_str() {
						"extra" | "with" => match v {
							Value::Array(items) => Payload::List(List::new(ListItems::Compound(
								items
									.iter()
									.map(|i| match json_component(i) {
										Payload::Compound(c) => c,
										other => vec![Entry::new("text", other)],
									})
									.collect(),
							))),
							v => json_value(v),
						},
						"hoverEvent" | "clickEvent" | "contents" | "value" | "separator" => match v
						{
							Value::Object(_) | Value::Array(_) => json_component(v),
							v => json_value(v),
						},
						_ => json_value(v),
					};
					Entry::new(k, value)
				})
				.collect(),
		),
		v => nbt::json::to_nbt(v),
	}
}

fn json_component(v: &Value) -> Payload {
	match v {
		Value::String(s) => Payload::Compound(vec![Entry::new("text", s.as_str())]),
		Value::Array(items) if !items.is_empty() => {
			let mut first = match json_component(&items[0]) {
				Payload::Compound(c) => c,
				_ => Vec::new(),
			};
			let extra: Vec<Vec<Entry>> = items[1..]
				.iter()
				.map(|i| match json_component(i) {
					Payload::Compound(c) => c,
					_ => Vec::new(),
				})
				.collect();
			first.push(Entry::new("extra", ListItems::Compound(extra)));
			Payload::Compound(first)
		}
		v => json_value(v),
	}
}

pub fn from_json(text: &str) -> Payload {
	let parsed = match serde_json::from_str::<Value>(text) {
		Ok(v) => json_component(&v),
		Err(_) => Payload::Compound(vec![Entry::new("text", text)]),
	};
	match payload(parsed) {
		Payload::Compound(c) => Payload::Compound(vec![
			Entry::new("text", ""),
			Entry::new("extra", ListItems::Compound(vec![c])),
		]),
		other => other,
	}
}
