use crate::{Entry, List, ListItems, Payload};
use serde_json::{Map, Number, Value};

const FLAGS: &[&str] = &[
	"bold",
	"italic",
	"underlined",
	"strikethrough",
	"obfuscated",
	"interpret",
];
const CONTENT: &[&str] = &["text", "translate", "score", "selector", "keybind", "nbt"];

pub fn to_nbt(v: &Value) -> Payload {
	match v {
		Value::Null => Payload::string(String::new()),
		Value::Bool(b) => Payload::byte(*b as i8),
		Value::Number(n) => match (n.as_i64(), n.as_f64()) {
			(Some(i), _) if i32::try_from(i).is_ok() => Payload::int(i as i32),
			(Some(i), _) => Payload::long(i),
			(_, f) => Payload::double(f.unwrap_or(0.0)),
		},
		Value::String(s) => Payload::string(s.clone()),
		Value::Array(a) => {
			let parts: Vec<Payload> = a.iter().map(to_nbt).collect();
			if parts.windows(2).all(|w| w[0].kind() == w[1].kind()) {
				Payload::list(List::of(parts))
			} else {
				Payload::list(List::of(
					parts
						.into_iter()
						.map(|p| Payload::compound(vec![Entry::new("", p)]))
						.collect(),
				))
			}
		}
		Value::Object(m) => {
			Payload::compound(m.iter().map(|(k, x)| Entry::new(k, to_nbt(x))).collect())
		}
	}
}

fn unwrap(p: &Payload) -> &Payload {
	match p {
		Payload::compound(c) if c.len() == 1 && c[0].name.is_empty() => &c[0].value,
		p => p,
	}
}

pub fn from_nbt(p: &Payload) -> Value {
	keyed(p, "", &[])
}

pub fn from_nbt_with_booleans(p: &Payload, booleans: &[&str]) -> Value {
	keyed(p, "", booleans)
}

fn keyed(p: &Payload, key: &str, booleans: &[&str]) -> Value {
	let num = |f: f64| Number::from_f64(f).map_or(Value::Null, Value::Number);
	match unwrap(p) {
		Payload::end => Value::Null,
		Payload::byte(b) if booleans.contains(&key) => Value::Bool(*b != 0),
		Payload::byte(v) => Value::from(*v),
		Payload::short(v) => Value::from(*v),
		Payload::int(v) => Value::from(*v),
		Payload::long(v) => Value::from(*v),
		Payload::float(v) => num(*v as f64),
		Payload::double(v) => num(*v),
		Payload::string(s) => Value::String(s.clone()),
		Payload::byteArray(v) => Value::from(v.clone()),
		Payload::intArray(v) => Value::from(v.clone()),
		Payload::longArray(v) => Value::from(v.clone()),
		Payload::list(l) => {
			Value::Array(l.values().iter().map(|x| keyed(x, "", booleans)).collect())
		}
		Payload::compound(c) => Value::Object(
			c.iter()
				.map(|e| (e.name.clone(), keyed(&e.value, &e.name, booleans)))
				.collect(),
		),
	}
}

pub fn parse_text(json: &str) -> Value {
	serde_json::from_str(json).unwrap_or_else(|_| Value::String(json.to_owned()))
}

pub fn parse_uuid(s: &str) -> Option<u128> {
	let hex: String = s.chars().filter(|c| *c != '-').collect();
	(hex.len() == 32)
		.then(|| u128::from_str_radix(&hex, 16).ok())
		.flatten()
}

pub fn format_uuid(u: u128) -> String {
	let h = format!("{u:032x}");
	format!(
		"{}-{}-{}-{}-{}",
		&h[0..8],
		&h[8..12],
		&h[12..16],
		&h[16..20],
		&h[20..32]
	)
}

fn scalar(v: &Value) -> String {
	match v {
		Value::String(s) => s.clone(),
		Value::Null => String::new(),
		v => v.to_string(),
	}
}

fn as_entries(p: Payload) -> Vec<Entry> {
	match p {
		Payload::compound(c) => c,
		Payload::string(s) => vec![Entry::new("text", s)],
		p => vec![Entry::new("text", format!("{p:?}"))],
	}
}

fn components(v: &Value) -> Payload {
	let parts = match v {
		Value::Array(a) => a
			.iter()
			.map(|x| Payload::compound(as_entries(text_to_nbt(x))))
			.collect(),
		v => vec![Payload::compound(as_entries(text_to_nbt(v)))],
	};
	Payload::list(List::of(parts))
}

pub fn text_to_nbt(v: &Value) -> Payload {
	match v {
		Value::Object(m) => object(m),
		Value::Array(a) => match a.split_first() {
			None => Payload::string(String::new()),
			Some((first, rest)) => {
				let mut c = as_entries(text_to_nbt(first));
				let mut extra = match c.iter().position(|e| e.name == "extra") {
					Some(i) => c.remove(i).value.values(),
					None => vec![],
				};
				extra.extend(
					rest.iter()
						.map(|x| Payload::compound(as_entries(text_to_nbt(x)))),
				);
				if !extra.is_empty() {
					c.push(Entry::new("extra", List::of(extra)));
				}
				Payload::compound(c)
			}
		},
		v => Payload::string(scalar(v)),
	}
}

fn flag(v: &Value) -> Payload {
	Payload::byte(match v {
		Value::Bool(b) => *b as i8,
		Value::String(s) => (s == "true") as i8,
		Value::Number(n) => (n.as_f64().unwrap_or(0.0) != 0.0) as i8,
		_ => 0,
	})
}

fn object(m: &Map<String, Value>) -> Payload {
	let mut out = Vec::new();
	for (k, v) in m {
		let value = match k.as_str() {
			"with" | "extra" if v.as_array().is_some_and(Vec::is_empty) => continue,
			k if FLAGS.contains(&k) => flag(v),
			"with" | "extra" => components(v),
			"separator" => text_to_nbt(v),
			"hoverEvent" => hover(v),
			"clickEvent" | "score" => match v {
				Value::Object(o) => {
					Payload::compound(o.iter().map(|(k, x)| Entry::new(k, scalar(x))).collect())
				}
				v => to_nbt(v),
			},
			_ => match v {
				Value::Object(_) | Value::Array(_) => to_nbt(v),
				v => Payload::string(scalar(v)),
			},
		};
		out.push(Entry::new(k, value));
	}
	if !out.iter().any(|e| CONTENT.contains(&e.name.as_str())) {
		out.insert(0, Entry::new("text", ""));
	}
	match out.as_slice() {
		[Entry {
			name,
			value: Payload::string(s),
			..
		}] if name == "text" => Payload::string(s.clone()),
		_ => Payload::compound(out),
	}
}

fn hover(v: &Value) -> Payload {
	let Value::Object(m) = v else {
		return to_nbt(v);
	};
	let action = m.get("action").map(scalar).unwrap_or_default();
	let mut out = vec![Entry::new("action", action.clone())];
	if let Some(contents) = m.get("contents") {
		let value = match (action.as_str(), contents) {
			("show_text", c) => text_to_nbt(c),
			("show_entity", Value::Object(e)) => Payload::compound(
				e.iter()
					.map(|(k, x)| match (k.as_str(), x) {
						("id", Value::String(s)) => Entry::new(
							"id",
							parse_uuid(s).map_or_else(|| Payload::string(s.clone()), Payload::uuid),
						),
						("name", x) => Entry::new("name", text_to_nbt(x)),
						(k, x) => Entry::new(k, to_nbt(x)),
					})
					.collect(),
			),
			(_, c) => to_nbt(c),
		};
		out.push(Entry::new("contents", value));
	}
	if let Some(value) = m.get("value") {
		out.push(Entry::new("value", text_to_nbt(value)));
	}
	Payload::compound(out)
}

pub fn text_from_nbt(p: &Payload) -> Value {
	match unwrap(p) {
		Payload::string(s) => Value::String(s.clone()),
		Payload::compound(c) => {
			let mut m = Map::new();
			for e in c {
				let v = match e.name.as_str() {
					k if FLAGS.contains(&k) => Value::Bool(e.value.as_f64().unwrap_or(0.0) != 0.0),
					"with" | "extra" => match &e.value {
						Payload::list(l) => {
							Value::Array(l.values().iter().map(text_from_nbt).collect())
						}
						p => Value::Array(vec![text_from_nbt(p)]),
					},
					"separator" => text_from_nbt(&e.value),
					"hoverEvent" => hover_json(&e.value),
					_ => from_nbt(&e.value),
				};
				m.insert(e.name.clone(), v);
			}
			Value::Object(m)
		}
		Payload::list(l) => Value::Array(l.values().iter().map(text_from_nbt).collect()),
		p => from_nbt(p),
	}
}

fn hover_json(p: &Payload) -> Value {
	let Some(c) = p.as_compound() else {
		return from_nbt(p);
	};
	let action = c
		.iter()
		.find(|e| e.name == "action")
		.and_then(|e| e.value.as_str())
		.unwrap_or_default();
	let mut m = Map::new();
	for e in c {
		let v = match (e.name.as_str(), action) {
			("contents", "show_text") | ("value", _) => text_from_nbt(&e.value),
			("contents", "show_entity") => match e.value.as_compound() {
				Some(entity) => Value::Object(
					entity
						.iter()
						.map(|x| {
							let v = match (x.name.as_str(), &x.value) {
								("id", id @ Payload::intArray(_)) => id
									.as_uuid()
									.map_or(Value::Null, |u| Value::String(format_uuid(u))),
								("name", v) => text_from_nbt(v),
								(_, v) => from_nbt(v),
							};
							(x.name.clone(), v)
						})
						.collect(),
				),
				None => from_nbt(&e.value),
			},
			_ => from_nbt(&e.value),
		};
		m.insert(e.name.clone(), v);
	}
	Value::Object(m)
}

pub mod typed {
	use crate::{Entry, List, ListItems, Payload};
	use serde_json::Value;

	fn long(v: &Value) -> Option<i64> {
		match v {
			Value::Array(parts) => match parts.as_slice() {
				[hi, lo] => {
					Some(((hi.as_i64()? as i32 as i64) << 32) | (lo.as_i64()? as u32 as i64))
				}
				_ => None,
			},
			v => v.as_i64(),
		}
	}

	fn each<T>(v: &Value, f: impl Fn(&Value) -> Option<T>) -> Option<Vec<T>> {
		v.as_array()?.iter().map(f).collect()
	}

	fn fields(v: &Value) -> Option<Vec<Entry>> {
		v.as_object()?
			.iter()
			.map(|(k, f)| Some(Entry::new(k, read(f)?)))
			.collect()
	}

	fn list(v: &Value) -> Option<List> {
		let values = v.get("value")?;
		Some(List::new(match v.get("type")?.as_str()? {
			"end" => ListItems::end(0),
			"byte" => ListItems::byte(each(values, |x| Some(x.as_i64()? as i8))?),
			"short" => ListItems::short(each(values, |x| Some(x.as_i64()? as i16))?),
			"int" => ListItems::int(each(values, |x| Some(x.as_i64()? as i32))?),
			"long" => ListItems::long(each(values, long)?),
			"float" => ListItems::float(each(values, |x| Some(x.as_f64()? as f32))?),
			"double" => ListItems::double(each(values, Value::as_f64)?),
			"byteArray" => {
				ListItems::byteArray(each(values, |x| each(x, |b| Some(b.as_i64()? as i8)))?)
			}
			"string" => ListItems::string(each(values, |x| Some(x.as_str()?.to_owned()))?),
			"list" => ListItems::list(each(values, list)?),
			"compound" => ListItems::compound(each(values, fields)?),
			"intArray" => {
				ListItems::intArray(each(values, |x| each(x, |i| Some(i.as_i64()? as i32)))?)
			}
			"longArray" => ListItems::longArray(each(values, |x| each(x, long))?),
			_ => return None,
		}))
	}

	pub fn payload(kind: &str, v: &Value) -> Option<Payload> {
		Some(match kind {
			"byte" => Payload::byte(v.as_i64()? as i8),
			"short" => Payload::short(v.as_i64()? as i16),
			"int" => Payload::int(v.as_i64()? as i32),
			"long" => Payload::long(long(v)?),
			"float" => Payload::float(v.as_f64()? as f32),
			"double" => Payload::double(v.as_f64()?),
			"string" => Payload::string(v.as_str()?.to_owned()),
			"byteArray" => Payload::byteArray(each(v, |x| Some(x.as_i64()? as i8))?),
			"intArray" => Payload::intArray(each(v, |x| Some(x.as_i64()? as i32))?),
			"longArray" => Payload::longArray(each(v, long)?),
			"compound" => Payload::compound(fields(v)?),
			"list" => Payload::list(list(v)?),
			_ => return None,
		})
	}

	pub fn read(v: &Value) -> Option<Payload> {
		payload(v.get("type")?.as_str()?, v.get("value")?)
	}

	pub fn entry(v: &Value) -> Option<Entry> {
		let name = v.get("name").and_then(Value::as_str).unwrap_or("");
		Some(Entry::new(name, read(v)?))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	fn compound(entries: Vec<(&str, Payload)>) -> Payload {
		Payload::compound(entries.into_iter().map(|(k, v)| Entry::new(k, v)).collect())
	}

	#[test]
	fn generic_scalars() {
		assert_eq!(to_nbt(&json!(true)), Payload::byte(1));
		assert_eq!(to_nbt(&json!(false)), Payload::byte(0));
		assert_eq!(to_nbt(&json!(5)), Payload::int(5));
		assert_eq!(
			to_nbt(&json!(5_000_000_000i64)),
			Payload::long(5_000_000_000)
		);
		assert_eq!(to_nbt(&json!(1.5)), Payload::double(1.5));
		assert_eq!(to_nbt(&json!(null)), Payload::string(String::new()));
		assert_eq!(to_nbt(&json!("s")), Payload::string("s".into()));
		assert_eq!(from_nbt(&Payload::byte(1)), json!(1));
		assert_eq!(from_nbt(&Payload::float(0.5)), json!(0.5));
		assert_eq!(from_nbt(&Payload::double(f64::NAN)), Value::Null);
		assert_eq!(from_nbt(&Payload::end), Value::Null);
		assert_eq!(from_nbt(&Payload::longArray(vec![1, 2])), json!([1, 2]));
	}

	#[test]
	fn mixed_lists_wrap_in_empty_keys() {
		let mixed = json!(["a", 1, {"k": true}]);
		let nbt = to_nbt(&mixed);
		let wrap = |p: Payload| vec![Entry::new("", p)];
		assert_eq!(
			nbt,
			ListItems::compound(vec![
				wrap("a".into()),
				wrap(Payload::int(1)),
				wrap(compound(vec![("k", Payload::byte(1))]))
			])
			.into()
		);
		assert_eq!(from_nbt(&nbt), json!(["a", 1, {"k": 1}]));
		assert_eq!(to_nbt(&json!([1, 2])), ListItems::int(vec![1, 2]).into());
		assert_eq!(to_nbt(&json!([])), Payload::list(List::of(vec![])));
		assert_eq!(
			from_nbt(&compound(vec![("", "inner".into())])),
			json!("inner")
		);
	}

	#[test]
	fn booleans_by_key() {
		let p = compound(vec![
			("bold", Payload::byte(1)),
			("n", Payload::byte(1)),
			("w", compound(vec![("", Payload::byte(0))])),
		]);
		assert_eq!(
			from_nbt_with_booleans(&p, &["bold", "w"]),
			json!({"bold": true, "n": 1, "w": false})
		);
		assert_eq!(from_nbt(&p), json!({"bold": 1, "n": 1, "w": 0}));
	}

	#[test]
	fn text_components_to_nbt() {
		assert_eq!(
			text_to_nbt(&json!("plain")),
			Payload::string("plain".into())
		);
		assert_eq!(
			text_to_nbt(&json!({"text": "hi"})),
			Payload::string("hi".into())
		);
		assert_eq!(text_to_nbt(&json!([])), Payload::string(String::new()));
		let v = json!({"text": "a", "bold": true, "italic": "false", "extra": ["b", {"text": "c", "color": "red"}], "with": []});
		assert_eq!(
			text_to_nbt(&v),
			compound(vec![
				("bold", Payload::byte(1)),
				(
					"extra",
					ListItems::compound(vec![
						vec![Entry::new("text", "b")],
						vec![Entry::new("color", "red"), Entry::new("text", "c")]
					])
					.into()
				),
				("italic", Payload::byte(0)),
				("text", "a".into()),
			])
		);
		assert_eq!(
			text_to_nbt(&json!(["a", {"text": "b"}])),
			compound(vec![
				("text", "a".into()),
				(
					"extra",
					ListItems::compound(vec![vec![Entry::new("text", "b")]]).into()
				)
			])
		);
		assert_eq!(
			text_to_nbt(&json!({"color": "red"})),
			compound(vec![("text", "".into()), ("color", "red".into())])
		);
		assert_eq!(
			text_to_nbt(&json!({"translate": "k", "with": [1, "x"]})),
			compound(vec![
				("translate", "k".into()),
				(
					"with",
					ListItems::compound(vec![
						vec![Entry::new("text", "1")],
						vec![Entry::new("text", "x")]
					])
					.into()
				)
			])
		);
	}

	#[test]
	fn hover_events() {
		let uuid = "00000001-0002-0003-0004-000000000005";
		let v = json!({"text": "", "hoverEvent": {"action": "show_entity", "contents": {"type": "minecraft:pig", "id": uuid, "name": {"text": "Bob"}}}});
		let nbt = text_to_nbt(&v);
		let hover = nbt.field("hoverEvent").unwrap();
		let contents = hover.field("contents").unwrap();
		assert_eq!(
			contents.field("id"),
			Some(&Payload::intArray(vec![1, 0x20003, 0x40000, 5]))
		);
		assert_eq!(contents.field("name"), Some(&Payload::string("Bob".into())));
		assert_eq!(
			text_from_nbt(&nbt),
			json!({"text": "", "hoverEvent": {"action": "show_entity", "contents": {"type": "minecraft:pig", "id": uuid, "name": "Bob"}}})
		);
		let item = json!({"text": "", "hoverEvent": {"action": "show_item", "contents": {"id": "minecraft:stone", "count": 2, "tag": "{a:1b}"}}});
		assert_eq!(text_from_nbt(&text_to_nbt(&item)), item);
		let shown = json!({"text": "", "hoverEvent": {"action": "show_text", "contents": ["a", {"text": "b", "bold": true}]}});
		assert_eq!(
			text_from_nbt(&text_to_nbt(&shown)),
			json!({"text": "", "hoverEvent": {"action": "show_text", "contents": {"text": "a", "extra": [{"text": "b", "bold": true}]}}})
		);
	}

	#[test]
	fn text_from_nbt_reads_vanilla_shapes() {
		let wrapped = Payload::list(List::new(ListItems::compound(vec![
			vec![Entry::new("", "a")],
			vec![Entry::new("text", "b"), Entry::new("bold", 1i8)],
		])));
		let p = compound(vec![
			("text", "".into()),
			("extra", wrapped),
			("italic", Payload::int(0)),
		]);
		assert_eq!(
			text_from_nbt(&p),
			json!({"text": "", "extra": ["a", {"text": "b", "bold": true}], "italic": false})
		);
		assert_eq!(
			text_from_nbt(&compound(vec![(
				"extra",
				compound(vec![("text", "x".into())])
			)])),
			json!({"extra": [{"text": "x"}]})
		);
		assert_eq!(text_from_nbt(&Payload::int(3)), json!(3));
		assert_eq!(parse_text("not json"), json!("not json"));
		assert_eq!(parse_text("{\"text\":\"a\"}"), json!({"text": "a"}));
	}

	#[test]
	fn uuids() {
		let u = 0x0123_4567_89ab_cdef_0011_2233_4455_6677u128;
		assert_eq!(format_uuid(u), "01234567-89ab-cdef-0011-223344556677");
		assert_eq!(parse_uuid(&format_uuid(u)), Some(u));
		assert_eq!(parse_uuid("0123456789abcdef0011223344556677"), Some(u));
		assert_eq!(parse_uuid("nope"), None);
	}

	#[test]
	fn typed_values() {
		let v = json!({"type": "compound", "name": "root", "value": {
			"b": {"type": "byte", "value": -1},
			"l": {"type": "long", "value": [-1, 5]},
			"l2": {"type": "long", "value": [4294967295u32, 5]},
			"f": {"type": "float", "value": 0.5},
			"s": {"type": "string", "value": "x"},
			"ia": {"type": "intArray", "value": [1, 2]},
			"la": {"type": "longArray", "value": [[0, 1], 7]},
			"ba": {"type": "byteArray", "value": [1]},
			"empty": {"type": "list", "value": {"type": "compound", "value": []}},
			"ends": {"type": "list", "value": {"type": "end", "value": []}},
			"nested": {"type": "list", "value": {"type": "list", "value": [{"type": "int", "value": [1, 2]}, {"type": "string", "value": ["a"]}]}},
			"objs": {"type": "list", "value": {"type": "compound", "value": [{"x": {"type": "short", "value": 3}}]}}
		}});
		let e = typed::entry(&v).unwrap();
		assert_eq!(e.name, "root");
		let p = &e.value;
		assert_eq!(p.field("b"), Some(&Payload::byte(-1)));
		assert_eq!(p.field("l"), Some(&Payload::long((-1i64 << 32) | 5)));
		assert_eq!(p.field("l2"), p.field("l"));
		assert_eq!(p.field("f"), Some(&Payload::float(0.5)));
		assert_eq!(p.field("s"), Some(&Payload::string("x".into())));
		assert_eq!(p.field("ia"), Some(&Payload::intArray(vec![1, 2])));
		assert_eq!(p.field("la"), Some(&Payload::longArray(vec![1, 7])));
		assert_eq!(p.field("ba"), Some(&Payload::byteArray(vec![1])));
		assert_eq!(p.field("empty"), Some(&ListItems::compound(vec![]).into()));
		assert_eq!(p.field("ends"), Some(&ListItems::end(0).into()));
		assert_eq!(
			p.field("nested"),
			Some(
				&ListItems::list(vec![
					List::new(ListItems::int(vec![1, 2])),
					List::new(ListItems::string(vec!["a".into()]))
				])
				.into()
			)
		);
		assert_eq!(
			p.field("objs").map(Payload::compounds),
			Some(&[vec![Entry::new("x", 3i16)]][..])
		);
		assert_eq!(typed::read(&json!({"type": "int", "value": "x"})), None);
		assert_eq!(typed::read(&json!({"type": "nope", "value": 1})), None);
		assert_eq!(typed::payload("short", &json!(7)), Some(Payload::short(7)));
		assert_eq!(
			typed::entry(&json!({"type": "int", "value": 1})),
			Some(Entry::new("", 1i32))
		);
	}
}
