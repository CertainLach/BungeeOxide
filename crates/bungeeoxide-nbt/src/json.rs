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
		Value::Null => Payload::String(String::new()),
		Value::Bool(b) => Payload::Byte(*b as i8),
		Value::Number(n) => match (n.as_i64(), n.as_f64()) {
			(Some(i), _) if i32::try_from(i).is_ok() => Payload::Int(i as i32),
			(Some(i), _) => Payload::Long(i),
			(_, f) => Payload::Double(f.unwrap_or(0.0)),
		},
		Value::String(s) => Payload::String(s.clone()),
		Value::Array(a) => {
			let parts: Vec<Payload> = a.iter().map(to_nbt).collect();
			if parts.windows(2).all(|w| w[0].kind() == w[1].kind()) {
				Payload::List(List::of(parts))
			} else {
				Payload::List(List::of(
					parts
						.into_iter()
						.map(|p| Payload::Compound(vec![Entry::new("", p)]))
						.collect(),
				))
			}
		}
		Value::Object(m) => {
			Payload::Compound(m.iter().map(|(k, x)| Entry::new(k, to_nbt(x))).collect())
		}
	}
}

fn unwrap(p: &Payload) -> &Payload {
	match p {
		Payload::Compound(c) if c.len() == 1 && c[0].name.is_empty() => &c[0].value,
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
		Payload::End => Value::Null,
		Payload::Byte(b) if booleans.contains(&key) => Value::Bool(*b != 0),
		Payload::Byte(v) => Value::from(*v),
		Payload::Short(v) => Value::from(*v),
		Payload::Int(v) => Value::from(*v),
		Payload::Long(v) => Value::from(*v),
		Payload::Float(v) => num(*v as f64),
		Payload::Double(v) => num(*v),
		Payload::String(s) => Value::String(s.clone()),
		Payload::ByteArray(v) => Value::from(v.clone()),
		Payload::IntArray(v) => Value::from(v.clone()),
		Payload::LongArray(v) => Value::from(v.clone()),
		Payload::List(l) => {
			Value::Array(l.values().iter().map(|x| keyed(x, "", booleans)).collect())
		}
		Payload::Compound(c) => Value::Object(
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
		Payload::Compound(c) => c,
		Payload::String(s) => vec![Entry::new("text", s)],
		p => vec![Entry::new("text", format!("{p:?}"))],
	}
}

fn components(v: &Value) -> Payload {
	let parts = match v {
		Value::Array(a) => a
			.iter()
			.map(|x| Payload::Compound(as_entries(text_to_nbt(x))))
			.collect(),
		v => vec![Payload::Compound(as_entries(text_to_nbt(v)))],
	};
	Payload::List(List::of(parts))
}

pub fn text_to_nbt(v: &Value) -> Payload {
	match v {
		Value::Object(m) => object(m),
		Value::Array(a) => match a.split_first() {
			None => Payload::String(String::new()),
			Some((first, rest)) => {
				let mut c = as_entries(text_to_nbt(first));
				let mut extra = match c.iter().position(|e| e.name == "extra") {
					Some(i) => c.remove(i).value.values(),
					None => vec![],
				};
				extra.extend(
					rest.iter()
						.map(|x| Payload::Compound(as_entries(text_to_nbt(x)))),
				);
				if !extra.is_empty() {
					c.push(Entry::new("extra", List::of(extra)));
				}
				Payload::Compound(c)
			}
		},
		v => Payload::String(scalar(v)),
	}
}

fn flag(v: &Value) -> Payload {
	Payload::Byte(match v {
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
					Payload::Compound(o.iter().map(|(k, x)| Entry::new(k, scalar(x))).collect())
				}
				v => to_nbt(v),
			},
			_ => match v {
				Value::Object(_) | Value::Array(_) => to_nbt(v),
				v => Payload::String(scalar(v)),
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
			value: Payload::String(s),
			..
		}] if name == "text" => Payload::String(s.clone()),
		_ => Payload::Compound(out),
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
			("show_entity", Value::Object(e)) => Payload::Compound(
				e.iter()
					.map(|(k, x)| match (k.as_str(), x) {
						("id", Value::String(s)) => Entry::new(
							"id",
							parse_uuid(s).map_or_else(|| Payload::String(s.clone()), Payload::uuid),
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
	Payload::Compound(out)
}

pub fn text_from_nbt(p: &Payload) -> Value {
	match unwrap(p) {
		Payload::String(s) => Value::String(s.clone()),
		Payload::Compound(c) => {
			let mut m = Map::new();
			for e in c {
				let v = match e.name.as_str() {
					k if FLAGS.contains(&k) => Value::Bool(e.value.as_f64().unwrap_or(0.0) != 0.0),
					"with" | "extra" => match &e.value {
						Payload::List(l) => {
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
		Payload::List(l) => Value::Array(l.values().iter().map(text_from_nbt).collect()),
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
								("id", id @ Payload::IntArray(_)) => id
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
			"end" => ListItems::End(0),
			"byte" => ListItems::Byte(each(values, |x| Some(x.as_i64()? as i8))?),
			"short" => ListItems::Short(each(values, |x| Some(x.as_i64()? as i16))?),
			"int" => ListItems::Int(each(values, |x| Some(x.as_i64()? as i32))?),
			"long" => ListItems::Long(each(values, long)?),
			"float" => ListItems::Float(each(values, |x| Some(x.as_f64()? as f32))?),
			"double" => ListItems::Double(each(values, Value::as_f64)?),
			"byteArray" => {
				ListItems::ByteArray(each(values, |x| each(x, |b| Some(b.as_i64()? as i8)))?)
			}
			"string" => ListItems::String(each(values, |x| Some(x.as_str()?.to_owned()))?),
			"list" => ListItems::List(each(values, list)?),
			"compound" => ListItems::Compound(each(values, fields)?),
			"intArray" => {
				ListItems::IntArray(each(values, |x| each(x, |i| Some(i.as_i64()? as i32)))?)
			}
			"longArray" => ListItems::LongArray(each(values, |x| each(x, long))?),
			_ => return None,
		}))
	}

	pub fn payload(kind: &str, v: &Value) -> Option<Payload> {
		Some(match kind {
			"byte" => Payload::Byte(v.as_i64()? as i8),
			"short" => Payload::Short(v.as_i64()? as i16),
			"int" => Payload::Int(v.as_i64()? as i32),
			"long" => Payload::Long(long(v)?),
			"float" => Payload::Float(v.as_f64()? as f32),
			"double" => Payload::Double(v.as_f64()?),
			"string" => Payload::String(v.as_str()?.to_owned()),
			"byteArray" => Payload::ByteArray(each(v, |x| Some(x.as_i64()? as i8))?),
			"intArray" => Payload::IntArray(each(v, |x| Some(x.as_i64()? as i32))?),
			"longArray" => Payload::LongArray(each(v, long)?),
			"compound" => Payload::Compound(fields(v)?),
			"list" => Payload::List(list(v)?),
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
		Payload::Compound(entries.into_iter().map(|(k, v)| Entry::new(k, v)).collect())
	}

	#[test]
	fn generic_scalars() {
		assert_eq!(to_nbt(&json!(true)), Payload::Byte(1));
		assert_eq!(to_nbt(&json!(false)), Payload::Byte(0));
		assert_eq!(to_nbt(&json!(5)), Payload::Int(5));
		assert_eq!(
			to_nbt(&json!(5_000_000_000i64)),
			Payload::Long(5_000_000_000)
		);
		assert_eq!(to_nbt(&json!(1.5)), Payload::Double(1.5));
		assert_eq!(to_nbt(&json!(null)), Payload::String(String::new()));
		assert_eq!(to_nbt(&json!("s")), Payload::String("s".into()));
		assert_eq!(from_nbt(&Payload::Byte(1)), json!(1));
		assert_eq!(from_nbt(&Payload::Float(0.5)), json!(0.5));
		assert_eq!(from_nbt(&Payload::Double(f64::NAN)), Value::Null);
		assert_eq!(from_nbt(&Payload::End), Value::Null);
		assert_eq!(from_nbt(&Payload::LongArray(vec![1, 2])), json!([1, 2]));
	}

	#[test]
	fn mixed_lists_wrap_in_empty_keys() {
		let mixed = json!(["a", 1, {"k": true}]);
		let nbt = to_nbt(&mixed);
		let wrap = |p: Payload| vec![Entry::new("", p)];
		assert_eq!(
			nbt,
			ListItems::Compound(vec![
				wrap("a".into()),
				wrap(Payload::Int(1)),
				wrap(compound(vec![("k", Payload::Byte(1))]))
			])
			.into()
		);
		assert_eq!(from_nbt(&nbt), json!(["a", 1, {"k": 1}]));
		assert_eq!(to_nbt(&json!([1, 2])), ListItems::Int(vec![1, 2]).into());
		assert_eq!(to_nbt(&json!([])), Payload::List(List::of(vec![])));
		assert_eq!(
			from_nbt(&compound(vec![("", "inner".into())])),
			json!("inner")
		);
	}

	#[test]
	fn booleans_by_key() {
		let p = compound(vec![
			("bold", Payload::Byte(1)),
			("n", Payload::Byte(1)),
			("w", compound(vec![("", Payload::Byte(0))])),
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
			Payload::String("plain".into())
		);
		assert_eq!(
			text_to_nbt(&json!({"text": "hi"})),
			Payload::String("hi".into())
		);
		assert_eq!(text_to_nbt(&json!([])), Payload::String(String::new()));
		let v = json!({"text": "a", "bold": true, "italic": "false", "extra": ["b", {"text": "c", "color": "red"}], "with": []});
		assert_eq!(
			text_to_nbt(&v),
			compound(vec![
				("bold", Payload::Byte(1)),
				(
					"extra",
					ListItems::Compound(vec![
						vec![Entry::new("text", "b")],
						vec![Entry::new("color", "red"), Entry::new("text", "c")]
					])
					.into()
				),
				("italic", Payload::Byte(0)),
				("text", "a".into()),
			])
		);
		assert_eq!(
			text_to_nbt(&json!(["a", {"text": "b"}])),
			compound(vec![
				("text", "a".into()),
				(
					"extra",
					ListItems::Compound(vec![vec![Entry::new("text", "b")]]).into()
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
					ListItems::Compound(vec![
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
			Some(&Payload::IntArray(vec![1, 0x20003, 0x40000, 5]))
		);
		assert_eq!(contents.field("name"), Some(&Payload::String("Bob".into())));
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
		let wrapped = Payload::List(List::new(ListItems::Compound(vec![
			vec![Entry::new("", "a")],
			vec![Entry::new("text", "b"), Entry::new("bold", 1i8)],
		])));
		let p = compound(vec![
			("text", "".into()),
			("extra", wrapped),
			("italic", Payload::Int(0)),
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
		assert_eq!(text_from_nbt(&Payload::Int(3)), json!(3));
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
		assert_eq!(p.field("b"), Some(&Payload::Byte(-1)));
		assert_eq!(p.field("l"), Some(&Payload::Long((-1i64 << 32) | 5)));
		assert_eq!(p.field("l2"), p.field("l"));
		assert_eq!(p.field("f"), Some(&Payload::Float(0.5)));
		assert_eq!(p.field("s"), Some(&Payload::String("x".into())));
		assert_eq!(p.field("ia"), Some(&Payload::IntArray(vec![1, 2])));
		assert_eq!(p.field("la"), Some(&Payload::LongArray(vec![1, 7])));
		assert_eq!(p.field("ba"), Some(&Payload::ByteArray(vec![1])));
		assert_eq!(p.field("empty"), Some(&ListItems::Compound(vec![]).into()));
		assert_eq!(p.field("ends"), Some(&ListItems::End(0).into()));
		assert_eq!(
			p.field("nested"),
			Some(
				&ListItems::List(vec![
					List::new(ListItems::Int(vec![1, 2])),
					List::new(ListItems::String(vec!["a".into()]))
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
		assert_eq!(typed::payload("short", &json!(7)), Some(Payload::Short(7)));
		assert_eq!(
			typed::entry(&json!({"type": "int", "value": 1})),
			Some(Entry::new("", 1i32))
		);
	}
}
