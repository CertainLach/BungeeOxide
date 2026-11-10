use serde_json::{json, Value};

pub fn rename_key(text: &str, from: &str, to: &str) -> String {
	fn walk(value: &mut Value, from: &str, to: &str) {
		match value {
			Value::Object(map) => {
				if let Some(v) = map.remove(from) {
					map.insert(to.to_owned(), v);
				}
				map.values_mut().for_each(|v| walk(v, from, to));
			}
			Value::Array(items) => items.iter_mut().for_each(|v| walk(v, from, to)),
			_ => {}
		}
	}
	match serde_json::from_str::<Value>(text) {
		Ok(mut value) if text.contains(from) => {
			walk(&mut value, from, to);
			value.to_string()
		}
		_ => text.to_owned(),
	}
}

pub fn legacy_to_json(text: &str) -> String {
	json!({ "text": text }).to_string()
}

pub fn json_to_legacy(text: &str) -> String {
	super::super::v1_7_to_1_8::text::to_legacy(&rename_key(text, "using", "with"))
}

pub fn truncate(text: &str, max: usize) -> String {
	let mut out = String::new();
	for c in text.chars() {
		if out.encode_utf16().count() + c.len_utf16() > max {
			break;
		}
		out.push(c);
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn translated_chat_reads_like_a_legacy_message() {
		let json = r#"{"translate":"chat.type.text","with":["Notch",{"text":"hi","color":"red","bold":true}]}"#;
		assert_eq!(json_to_legacy(json), "<Notch> \u{a7}c\u{a7}lhi");
		let death = r#"{"translate":"death.attack.generic","color":"gold","with":[{"text":"Notch","color":"red"}],"extra":["!"]}"#;
		assert_eq!(json_to_legacy(death), "\u{a7}cNotch\u{a7}6 died!");
	}

	#[test]
	fn translation_arguments_move_between_1_6_and_1_7_names() {
		let old = r#"{"translate":"chat.type.text","using":["a","b"]}"#;
		let new = rename_key(old, "using", "with");
		assert!(new.contains("\"with\"") && !new.contains("using"));
		assert_eq!(rename_key(&new, "with", "using"), old);
		assert_eq!(json_to_legacy(old), "<a> b");
		assert_eq!(rename_key("plain", "using", "with"), "plain");
	}

	#[test]
	fn plain_text_survives_a_round_trip() {
		let text = "\u{a7}eHello \"world\"";
		assert_eq!(json_to_legacy(&legacy_to_json(text)), text);
		assert_eq!(json_to_legacy("not json"), "not json");
	}
}
