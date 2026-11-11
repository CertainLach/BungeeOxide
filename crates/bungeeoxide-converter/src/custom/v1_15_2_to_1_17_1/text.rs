use ::bungeeoxide_nbt::snbt::quote;
use serde_json::{Map, Value};

const COLORS: [(&str, u32); 16] = [
	("black", 0x000000),
	("dark_blue", 0x0000aa),
	("dark_green", 0x00aa00),
	("dark_aqua", 0x00aaaa),
	("dark_red", 0xaa0000),
	("dark_purple", 0xaa00aa),
	("gold", 0xffaa00),
	("gray", 0xaaaaaa),
	("dark_gray", 0x555555),
	("blue", 0x5555ff),
	("green", 0x55ff55),
	("aqua", 0x55ffff),
	("red", 0xff5555),
	("light_purple", 0xff55ff),
	("yellow", 0xffff55),
	("white", 0xffffff),
];

fn nearest(hex: &str) -> Option<&'static str> {
	let rgb = u32::from_str_radix(hex.strip_prefix('#')?, 16).ok()?;
	let channel = |c: u32, s: u32| ((c >> s) & 0xff) as i32;
	COLORS
		.iter()
		.min_by_key(|(_, c)| {
			(0..3)
				.map(|i| (channel(rgb, i * 8) - channel(*c, i * 8)).pow(2))
				.sum::<i32>()
		})
		.map(|(name, _)| *name)
}

fn snbt_item(contents: &Value) -> String {
	let id = contents
		.get("id")
		.and_then(Value::as_str)
		.unwrap_or("minecraft:air");
	let count = contents.get("count").and_then(Value::as_i64).unwrap_or(1);
	let tag = contents
		.get("tag")
		.and_then(Value::as_str)
		.map(|t| format!(",tag:{t}"))
		.unwrap_or_default();
	format!("{{id:{},Count:{count}b{tag}}}", quote(id))
}

fn snbt_entity(contents: &Value) -> String {
	let ty = contents
		.get("type")
		.and_then(Value::as_str)
		.unwrap_or("minecraft:pig");
	let id = match contents.get("id") {
		Some(Value::String(s)) => s.clone(),
		Some(Value::Array(ints)) => {
			let ints: Vec<i32> = ints
				.iter()
				.filter_map(Value::as_i64)
				.map(|v| v as i32)
				.collect();
			::bungeeoxide_nbt::Payload::IntArray(ints)
				.as_uuid()
				.map(::bungeeoxide_nbt::json::format_uuid)
				.unwrap_or_default()
		}
		_ => String::new(),
	};
	let name = contents
		.get("name")
		.map(|n| format!(",name:{}", quote(&legacy_value(n.clone()).to_string())))
		.unwrap_or_default();
	format!("{{type:{},id:{}{name}}}", quote(ty), quote(&id))
}

fn hover(mut event: Map<String, Value>) -> Map<String, Value> {
	let Some(contents) = event.remove("contents") else {
		if let Some(v) = event.remove("value") {
			event.insert("value".to_owned(), legacy_value(v));
		}
		return event;
	};
	let value = match event.get("action").and_then(Value::as_str) {
		Some("show_text") => legacy_value(contents),
		Some("show_item") => Value::String(snbt_item(&contents)),
		Some("show_entity") => Value::String(snbt_entity(&contents)),
		_ => contents,
	};
	event.insert("value".to_owned(), value);
	event
}

fn legacy_value(v: Value) -> Value {
	match v {
		Value::Array(items) => Value::Array(items.into_iter().map(legacy_value).collect()),
		Value::Object(mut o) => {
			if let Some(name) = o
				.get("color")
				.and_then(Value::as_str)
				.filter(|c| c.starts_with('#'))
				.map(|c| nearest(c).unwrap_or("white"))
			{
				o.insert("color".to_owned(), Value::String(name.to_owned()));
			}
			o.remove("font");
			for key in ["extra", "with"] {
				if let Some(list) = o.remove(key) {
					o.insert(key.to_owned(), legacy_value(list));
				}
			}
			if let Some(Value::Object(event)) = o.remove("hoverEvent") {
				o.insert("hoverEvent".to_owned(), Value::Object(hover(event)));
			}
			Value::Object(o)
		}
		v => v,
	}
}

pub(super) fn legacy(text: &str) -> String {
	match serde_json::from_str::<Value>(text) {
		Ok(v @ (Value::Object(_) | Value::Array(_))) => legacy_value(v).to_string(),
		_ => text.to_owned(),
	}
}

pub(super) fn legacy_opt(text: Option<String>) -> Option<String> {
	text.map(|t| legacy(&t))
}
