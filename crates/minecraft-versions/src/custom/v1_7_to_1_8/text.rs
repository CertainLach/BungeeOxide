use std::fmt::Write;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Json {
	Null,
	Bool(bool),
	Num(String),
	Str(String),
	Arr(Vec<Json>),
	Obj(Vec<(String, Json)>),
}

impl Json {
	pub(super) fn get(&self, key: &str) -> Option<&Json> {
		match self {
			Json::Obj(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
			_ => None,
		}
	}

	pub(super) fn str(&self) -> Option<&str> {
		match self {
			Json::Str(s) => Some(s),
			_ => None,
		}
	}
}

struct Parser<'a> {
	s: &'a [u8],
	at: usize,
}

impl Parser<'_> {
	fn ws(&mut self) {
		while matches!(self.s.get(self.at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
			self.at += 1;
		}
	}

	fn eat(&mut self, c: u8) -> bool {
		let hit = self.s.get(self.at) == Some(&c);
		self.at += hit as usize;
		hit
	}

	fn hex(&mut self) -> Option<u32> {
		let digits = std::str::from_utf8(self.s.get(self.at..self.at + 4)?).ok()?;
		self.at += 4;
		u32::from_str_radix(digits, 16).ok()
	}

	fn string(&mut self) -> Option<String> {
		if !self.eat(b'"') {
			return None;
		}
		let mut out = String::new();
		loop {
			let start = self.at;
			while !matches!(self.s.get(self.at), None | Some(b'"' | b'\\')) {
				self.at += 1;
			}
			out.push_str(std::str::from_utf8(&self.s[start..self.at]).ok()?);
			if self.eat(b'"') {
				return Some(out);
			}
			self.at += 1;
			let e = *self.s.get(self.at)?;
			self.at += 1;
			out.push(match e {
				b'"' => '"',
				b'\\' => '\\',
				b'/' => '/',
				b'b' => '\u{8}',
				b'f' => '\u{c}',
				b'n' => '\n',
				b'r' => '\r',
				b't' => '\t',
				b'u' => {
					let hi = self.hex()?;
					let code = if (0xd800..0xdc00).contains(&hi)
						&& self.s.get(self.at..self.at + 2) == Some(b"\\u")
					{
						self.at += 2;
						let lo = self.hex()?;
						match lo {
							0xdc00..0xe000 => 0x10000 + ((hi - 0xd800) << 10) + (lo - 0xdc00),
							_ => 0xfffd,
						}
					} else {
						hi
					};
					char::from_u32(code).unwrap_or('\u{fffd}')
				}
				_ => return None,
			});
		}
	}

	fn value(&mut self, depth: usize) -> Option<Json> {
		if depth > 64 {
			return None;
		}
		self.ws();
		match *self.s.get(self.at)? {
			b'{' => {
				self.at += 1;
				let mut m = Vec::new();
				self.ws();
				if self.eat(b'}') {
					return Some(Json::Obj(m));
				}
				loop {
					self.ws();
					let k = self.string()?;
					self.ws();
					if !self.eat(b':') {
						return None;
					}
					m.push((k, self.value(depth + 1)?));
					self.ws();
					if self.eat(b'}') {
						return Some(Json::Obj(m));
					}
					if !self.eat(b',') {
						return None;
					}
				}
			}
			b'[' => {
				self.at += 1;
				let mut items = Vec::new();
				self.ws();
				if self.eat(b']') {
					return Some(Json::Arr(items));
				}
				loop {
					items.push(self.value(depth + 1)?);
					self.ws();
					if self.eat(b']') {
						return Some(Json::Arr(items));
					}
					if !self.eat(b',') {
						return None;
					}
				}
			}
			b'"' => self.string().map(Json::Str),
			_ => {
				let start = self.at;
				while matches!(self.s.get(self.at), Some(c) if c.is_ascii_alphanumeric() || matches!(c, b'-' | b'+' | b'.'))
				{
					self.at += 1;
				}
				match std::str::from_utf8(&self.s[start..self.at]).ok()? {
					"true" => Some(Json::Bool(true)),
					"false" => Some(Json::Bool(false)),
					"null" => Some(Json::Null),
					n if n.parse::<f64>().is_ok() => Some(Json::Num(n.to_owned())),
					_ => None,
				}
			}
		}
	}
}

pub(super) fn parse(s: &str) -> Option<Json> {
	let mut p = Parser {
		s: s.as_bytes(),
		at: 0,
	};
	let v = p.value(0)?;
	p.ws();
	(p.at == s.len()).then_some(v)
}

fn quote_into(s: &str, out: &mut String) {
	out.push('"');
	for c in s.chars() {
		match c {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			'\n' => out.push_str("\\n"),
			'\r' => out.push_str("\\r"),
			'\t' => out.push_str("\\t"),
			c if (c as u32) < 0x20 => {
				let _ = write!(out, "\\u{:04x}", c as u32);
			}
			c => out.push(c),
		}
	}
	out.push('"');
}

pub(super) fn quote(s: &str) -> String {
	let mut out = String::with_capacity(s.len() + 2);
	quote_into(s, &mut out);
	out
}

fn write_json(j: &Json, out: &mut String) {
	match j {
		Json::Null => out.push_str("null"),
		Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
		Json::Num(n) => out.push_str(n),
		Json::Str(s) => quote_into(s, out),
		Json::Arr(items) => {
			out.push('[');
			for (i, item) in items.iter().enumerate() {
				if i > 0 {
					out.push(',');
				}
				write_json(item, out);
			}
			out.push(']');
		}
		Json::Obj(m) => {
			out.push('{');
			for (i, (k, v)) in m.iter().enumerate() {
				if i > 0 {
					out.push(',');
				}
				quote_into(k, out);
				out.push(':');
				write_json(v, out);
			}
			out.push('}');
		}
	}
}

pub(super) fn to_string(j: &Json) -> String {
	let mut out = String::new();
	write_json(j, &mut out);
	out
}

const COLORS: [(&str, char); 16] = [
	("black", '0'),
	("dark_blue", '1'),
	("dark_green", '2'),
	("dark_aqua", '3'),
	("dark_red", '4'),
	("dark_purple", '5'),
	("gold", '6'),
	("gray", '7'),
	("dark_gray", '8'),
	("blue", '9'),
	("green", 'a'),
	("aqua", 'b'),
	("red", 'c'),
	("light_purple", 'd'),
	("yellow", 'e'),
	("white", 'f'),
];

const FORMATS: [(&str, char); 5] = [
	("obfuscated", 'k'),
	("bold", 'l'),
	("strikethrough", 'm'),
	("underlined", 'n'),
	("italic", 'o'),
];

const SECTION: char = '\u{a7}';

#[derive(Clone, Copy, Default, PartialEq)]
struct Style {
	color: Option<char>,
	formats: [bool; 5],
}

fn style_of(j: &Json, parent: Style) -> Style {
	let mut s = parent;
	match j.get("color").and_then(Json::str) {
		Some("reset") => s.color = None,
		Some(c) => {
			if let Some((_, code)) = COLORS.iter().find(|(n, _)| *n == c) {
				s.color = Some(*code);
			}
		}
		None => {}
	}
	for (i, (name, _)) in FORMATS.iter().enumerate() {
		match j.get(name) {
			Some(Json::Bool(b)) => s.formats[i] = *b,
			Some(Json::Str(v)) => s.formats[i] = v == "true",
			_ => {}
		}
	}
	s
}

#[derive(Default)]
struct Legacy {
	out: String,
	current: Style,
}

impl Legacy {
	fn emit(&mut self, text: &str, style: Style) {
		if text.is_empty() {
			return;
		}
		if style != self.current {
			match style.color {
				Some(c) => {
					self.out.push(SECTION);
					self.out.push(c);
				}
				None => {
					self.out.push(SECTION);
					self.out.push('r');
				}
			}
			for (on, (_, code)) in style.formats.iter().zip(FORMATS) {
				if *on {
					self.out.push(SECTION);
					self.out.push(code);
				}
			}
			self.current = style;
		}
		self.out.push_str(text);
	}

	fn translate(
		&mut self,
		key: &str,
		args: &[Json],
		fallback: Option<&str>,
		style: Style,
		depth: usize,
	) {
		let template = fallback
			.or_else(|| crate::languages::english(key))
			.or(args.is_empty().then_some(key));
		let Some(template) = template else {
			self.emit(key, style);
			for a in args {
				self.emit(" ", style);
				self.walk(a, style, depth + 1);
			}
			return;
		};
		for piece in crate::languages::pieces(template) {
			match piece {
				crate::languages::Piece::Text(t) => self.emit(t, style),
				crate::languages::Piece::Arg(i) => {
					if let Some(a) = args.get(i) {
						self.walk(a, style, depth + 1);
					}
				}
			}
		}
	}

	fn walk(&mut self, j: &Json, parent: Style, depth: usize) {
		if depth > 32 {
			return;
		}
		match j {
			Json::Str(s) | Json::Num(s) => self.emit(s, parent),
			Json::Bool(b) => self.emit(if *b { "true" } else { "false" }, parent),
			Json::Null => {}
			Json::Arr(items) => {
				let Some((first, rest)) = items.split_first() else {
					return;
				};
				self.walk(first, parent, depth + 1);
				let style = style_of(first, parent);
				for item in rest {
					self.walk(item, style, depth + 1);
				}
			}
			Json::Obj(_) => {
				let style = style_of(j, parent);
				if let Some(t) = j.get("text") {
					self.walk(t, style, depth + 1);
				} else if let Some(key) = j.get("translate").and_then(Json::str) {
					let args = match j.get("with") {
						Some(Json::Arr(a)) => a.as_slice(),
						_ => &[],
					};
					self.translate(
						key,
						args,
						j.get("fallback").and_then(Json::str),
						style,
						depth,
					);
				} else if let Some(score) = j.get("score") {
					if let Some(v) = score.get("value") {
						self.walk(v, style, depth + 1);
					}
				} else if let Some(s) = j.get("selector").and_then(Json::str) {
					self.emit(s, style);
				}
				if let Some(Json::Arr(extra)) = j.get("extra") {
					for e in extra {
						self.walk(e, style, depth + 1);
					}
				}
			}
		}
	}
}

pub(super) fn json_to_legacy(j: &Json) -> String {
	let mut l = Legacy::default();
	l.walk(j, Style::default(), 0);
	l.out
}

pub(crate) fn to_legacy(s: &str) -> String {
	match parse(s) {
		Some(j) => json_to_legacy(&j),
		None => s.to_owned(),
	}
}

pub(crate) fn from_legacy(s: &str) -> String {
	quote(s)
}

pub(super) fn title_1_7(s: &str) -> (String, bool) {
	if let Some(j @ Json::Obj(m)) = parse(s).as_ref() {
		if let (1, Some(key)) = (m.len(), j.get("translate").and_then(Json::str)) {
			return (truncate(key, 32), false);
		}
	}
	(truncate(&to_legacy(s), 32), true)
}

fn key_name(key: &str) -> String {
	let last = key.rsplit('.').next().unwrap_or(key);
	let mut c = last.chars();
	c.next()
		.map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
		.unwrap_or_default()
}

fn sanitize(j: &mut Json, all: bool, depth: usize) {
	if depth > 32 {
		return;
	}
	match j {
		Json::Arr(items) => items.iter_mut().for_each(|i| sanitize(i, all, depth + 1)),
		Json::Obj(m) => {
			let has = |m: &[(String, Json)], k: &str| m.iter().any(|(n, _)| n == k);
			let keybind = has(m, "keybind");
			if !has(m, "text") && !has(m, "translate") && (all || keybind) {
				let text = match (j.get("keybind"), j.get("score"), j.get("selector")) {
					(Some(k), ..) => k.str().map(key_name).unwrap_or_default(),
					(None, Some(score), _) if all => {
						score.get("value").map(json_to_legacy).unwrap_or_default()
					}
					(None, None, Some(s)) if all => json_to_legacy(s),
					_ => String::new(),
				};
				let Json::Obj(m) = j else { return };
				m.retain(|(k, _)| k != "keybind" && (!all || (k != "score" && k != "selector")));
				m.push(("text".to_owned(), Json::Str(text)));
			}
			let Json::Obj(m) = j else { return };
			for (k, v) in m.iter_mut() {
				if k == "extra" || k == "with" {
					sanitize(v, all, depth + 1);
				}
			}
		}
		_ => {}
	}
}

fn sanitized(s: &str, all: bool) -> String {
	match parse(s) {
		Some(mut j) => {
			sanitize(&mut j, all, 0);
			to_string(&j)
		}
		None => s.to_owned(),
	}
}

pub(super) fn chat_1_7(s: &str) -> String {
	if !["\"score\"", "\"selector\"", "\"keybind\""]
		.iter()
		.any(|k| s.contains(k))
	{
		return s.to_owned();
	}
	sanitized(s, true)
}

pub(crate) fn without_keybinds(s: &str) -> String {
	if !s.contains("\"keybind\"") {
		return s.to_owned();
	}
	sanitized(s, false)
}

pub(super) fn truncate(s: &str, n: usize) -> String {
	s.chars().take(n).collect()
}

pub(super) fn strip(s: &str) -> String {
	let mut out = String::with_capacity(s.len());
	let mut chars = s.chars();
	while let Some(c) = chars.next() {
		if c == SECTION {
			chars.next();
		} else {
			out.push(c);
		}
	}
	out
}

pub(crate) fn fit(s: String, n: usize) -> String {
	if s.chars().count() <= n {
		return s;
	}
	truncate(&strip(&s), n)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn legacy_from_components() {
		let s = r#"{"text":"","extra":[{"text":"Hi ","color":"red","bold":true},"there",{"translate":"chat.type.text","with":["Steve",{"text":"yo","color":"blue"}]}]}"#;
		assert_eq!(
			to_legacy(s),
			"\u{a7}c\u{a7}lHi \u{a7}rthere<Steve> \u{a7}9yo"
		);
		assert_eq!(to_legacy("\"plain\""), "plain");
		assert_eq!(to_legacy("not json"), "not json");
	}

	#[test]
	fn titles_keep_translation_keys() {
		assert_eq!(
			title_1_7(r#"{"translate":"container.chest"}"#),
			("container.chest".to_owned(), false)
		);
		assert_eq!(title_1_7(r#"{"text":"Shop"}"#), ("Shop".to_owned(), true));
	}

	#[test]
	fn keybinds_become_text() {
		let s = r#"{"translate":"mount.onboard","with":[{"keybind":"key.sneak"}]}"#;
		let out = parse(&without_keybinds(s)).unwrap();
		let Some(Json::Arr(with)) = out.get("with") else {
			panic!("{out:?}")
		};
		assert_eq!(
			with[0],
			Json::Obj(vec![("text".into(), Json::Str("Sneak".into()))])
		);
		assert_eq!(
			without_keybinds(r#"{"score":{"name":"a"}}"#),
			r#"{"score":{"name":"a"}}"#
		);
	}

	#[test]
	fn chat_replaces_scores() {
		let s = r#"{"score":{"name":"a","objective":"b","value":"7"},"extra":[{"selector":"@p"}]}"#;
		let out = parse(&chat_1_7(s)).unwrap();
		assert_eq!(json_to_legacy(&out), "7@p");
		assert!(out.get("text").is_some() && out.get("score").is_none());
	}

	#[test]
	fn quoting_round_trips() {
		let s = "a\"b\\c\n\u{e9}";
		assert_eq!(parse(&quote(s)), Some(Json::Str(s.to_owned())));
		let escaped = [
			"\"", "\\", "ud83d", "\\", "ude00", "\\", "ud83d", "\\", "u0041", "\"",
		]
		.concat();
		assert_eq!(
			parse(&escaped),
			Some(Json::Str("\u{1f600}\u{fffd}".to_owned()))
		);
	}
}
