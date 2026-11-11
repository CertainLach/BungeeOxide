const COLORS: [&str; 16] = [
	"black",
	"dark_blue",
	"dark_green",
	"dark_aqua",
	"dark_red",
	"dark_purple",
	"gold",
	"gray",
	"dark_gray",
	"blue",
	"green",
	"aqua",
	"red",
	"light_purple",
	"yellow",
	"white",
];

const FORMATS: [(char, &str); 5] = [
	('k', "obfuscated"),
	('l', "bold"),
	('m', "strikethrough"),
	('n', "underlined"),
	('o', "italic"),
];

const CODES: &str = "0123456789abcdefklmnor";

#[derive(Clone, Default, PartialEq)]
struct Style {
	color: Option<usize>,
	formats: [bool; 5],
}

impl Style {
	fn is_empty(&self) -> bool {
		*self == Style::default()
	}
}

fn quote(out: &mut String, s: &str) {
	out.push('"');
	for c in s.chars() {
		match c {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			'\n' => out.push_str("\\n"),
			'\r' => out.push_str("\\r"),
			'\t' => out.push_str("\\t"),
			c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
			c => out.push(c),
		}
	}
	out.push('"');
}

pub(super) fn legacy_to_json(s: &str, item: bool) -> String {
	let mut parts: Vec<(Style, String)> = Vec::new();
	let mut style = Style::default();
	let mut text = String::new();
	let mut chars = s.chars();
	while let Some(c) = chars.next() {
		if c != '\u{a7}' {
			text.push(c);
			continue;
		}
		let Some(code) = chars.next() else { break };
		if !text.is_empty() {
			parts.push((style.clone(), std::mem::take(&mut text)));
		}
		let code = code.to_ascii_lowercase();
		match CODES.find(code) {
			Some(i) if i < 16 => {
				style = Style {
					color: Some(i),
					..Style::default()
				}
			}
			Some(21) => style = Style::default(),
			Some(i) => style.formats[i - 16] = true,
			None => {
				style = Style {
					color: Some(15),
					..Style::default()
				}
			}
		}
	}
	if !text.is_empty() || parts.is_empty() {
		parts.push((style, text));
	}
	let mut out = String::new();
	let component = |out: &mut String, style: &Style, text: &str| {
		out.push_str("{\"text\":");
		quote(out, text);
		if let Some(c) = style.color {
			out.push_str(",\"color\":");
			quote(out, COLORS[c]);
		}
		for (on, (_, name)) in style.formats.iter().zip(FORMATS) {
			if *on {
				out.push_str(&format!(",\"{name}\":true"));
			}
		}
		if item && !style.is_empty() && !style.formats[4] {
			out.push_str(",\"italic\":false");
		}
		out.push('}');
	};
	if let [(style, text)] = parts.as_slice() {
		component(&mut out, style, text);
		return out;
	}
	out.push_str("{\"text\":\"\",\"extra\":[");
	for (i, (style, text)) in parts.iter().enumerate() {
		if i > 0 {
			out.push(',');
		}
		component(&mut out, style, text);
	}
	out.push_str("]}");
	out
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Json {
	Null,
	Bool(bool),
	Number(f64),
	Str(String),
	Array(Vec<Json>),
	Object(Vec<(String, Json)>),
}

impl Json {
	pub fn get(&self, key: &str) -> Option<&Json> {
		match self {
			Json::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
			_ => None,
		}
	}

	pub fn str(&self) -> Option<&str> {
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
		while self.s.get(self.at).is_some_and(u8::is_ascii_whitespace) {
			self.at += 1;
		}
	}

	fn eat(&mut self, c: u8) -> Option<()> {
		self.ws();
		(self.s.get(self.at) == Some(&c)).then(|| self.at += 1)
	}

	fn value(&mut self) -> Option<Json> {
		self.ws();
		match *self.s.get(self.at)? {
			b'{' => {
				self.at += 1;
				let mut fields = Vec::new();
				if self.eat(b'}').is_some() {
					return Some(Json::Object(fields));
				}
				loop {
					self.ws();
					let key = self.string()?;
					self.eat(b':')?;
					fields.push((key, self.value()?));
					if self.eat(b',').is_none() {
						self.eat(b'}')?;
						return Some(Json::Object(fields));
					}
				}
			}
			b'[' => {
				self.at += 1;
				let mut items = Vec::new();
				if self.eat(b']').is_some() {
					return Some(Json::Array(items));
				}
				loop {
					items.push(self.value()?);
					if self.eat(b',').is_none() {
						self.eat(b']')?;
						return Some(Json::Array(items));
					}
				}
			}
			b'"' => self.string().map(Json::Str),
			b't' => self.word("true", Json::Bool(true)),
			b'f' => self.word("false", Json::Bool(false)),
			b'n' => self.word("null", Json::Null),
			_ => {
				let start = self.at;
				while self
					.s
					.get(self.at)
					.is_some_and(|c| c.is_ascii_digit() || b"+-.eE".contains(c))
				{
					self.at += 1;
				}
				std::str::from_utf8(&self.s[start..self.at])
					.ok()?
					.parse()
					.ok()
					.map(Json::Number)
			}
		}
	}

	fn word(&mut self, w: &str, v: Json) -> Option<Json> {
		self.s[self.at..].starts_with(w.as_bytes()).then(|| {
			self.at += w.len();
			v
		})
	}

	fn string(&mut self) -> Option<String> {
		if self.s.get(self.at) != Some(&b'"') {
			return None;
		}
		self.at += 1;
		let mut out = Vec::new();
		loop {
			let c = *self.s.get(self.at)?;
			self.at += 1;
			match c {
				b'"' => return String::from_utf8(out).ok(),
				b'\\' => {
					let e = *self.s.get(self.at)?;
					self.at += 1;
					let ch = match e {
						b'n' => '\n',
						b't' => '\t',
						b'r' => '\r',
						b'b' => '\u{8}',
						b'f' => '\u{c}',
						b'u' => {
							let mut code = self.hex()?;
							if (0xd800..0xdc00).contains(&code)
								&& self.s[self.at..].starts_with(b"\\u")
							{
								self.at += 2;
								let low = self.hex()?;
								code = 0x10000
									+ ((code - 0xd800) << 10) + (low.wrapping_sub(0xdc00)
									& 0x3ff);
							}
							char::from_u32(code).unwrap_or('\u{fffd}')
						}
						c => c as char,
					};
					let mut buf = [0; 4];
					out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
				}
				c => out.push(c),
			}
		}
	}

	fn hex(&mut self) -> Option<u32> {
		let digits = std::str::from_utf8(self.s.get(self.at..self.at + 4)?).ok()?;
		self.at += 4;
		u32::from_str_radix(digits, 16).ok()
	}
}

pub(super) fn parse(s: &str) -> Option<Json> {
	let mut p = Parser {
		s: s.as_bytes(),
		at: 0,
	};
	let v = p.value()?;
	p.ws();
	(p.at == s.len()).then_some(v)
}

fn flatten(c: &Json, parent: &Style, out: &mut Vec<(Style, String)>) {
	match c {
		Json::Str(s) => out.push((parent.clone(), s.clone())),
		Json::Number(n) => out.push((parent.clone(), n.to_string())),
		Json::Bool(b) => out.push((parent.clone(), b.to_string())),
		Json::Array(items) => {
			let mut it = items.iter();
			if let Some(first) = it.next() {
				flatten(first, parent, out);
				let base = match first {
					Json::Object(_) => style_of(first, parent),
					_ => parent.clone(),
				};
				for i in it {
					flatten(i, &base, out);
				}
			}
		}
		Json::Object(_) => {
			let style = style_of(c, parent);
			let pattern = match (c.get("text"), c.get("translate").and_then(Json::str)) {
				(None, Some(key)) => c
					.get("fallback")
					.and_then(Json::str)
					.or_else(|| crate::languages::english(key)),
				_ => None,
			};
			match pattern {
				Some(pattern) => {
					let args = match c.get("with") {
						Some(Json::Array(args)) => args.as_slice(),
						_ => &[],
					};
					for piece in crate::languages::pieces(pattern) {
						match piece {
							crate::languages::Piece::Text(t) => {
								out.push((style.clone(), t.to_owned()))
							}
							crate::languages::Piece::Arg(i) => {
								if let Some(a) = args.get(i) {
									flatten(a, &style, out);
								}
							}
						}
					}
				}
				None => {
					let text = c
						.get("text")
						.and_then(Json::str)
						.or_else(|| c.get("translate").and_then(Json::str))
						.or_else(|| c.get("keybind").and_then(Json::str))
						.unwrap_or_default();
					out.push((style.clone(), text.to_owned()));
				}
			}
			if let Some(Json::Array(extra)) = c.get("extra") {
				for e in extra {
					flatten(e, &style, out);
				}
			}
		}
		Json::Null => {}
	}
}

fn style_of(c: &Json, parent: &Style) -> Style {
	let mut style = parent.clone();
	if let Some(color) = c.get("color").and_then(Json::str) {
		style.color = COLORS.iter().position(|n| *n == color);
	}
	for (i, (_, name)) in FORMATS.iter().enumerate() {
		if let Some(Json::Bool(b)) = c.get(name) {
			style.formats[i] = *b;
		}
	}
	style
}

pub(super) fn json_to_legacy(s: &str) -> String {
	let Some(json) = parse(s) else {
		return s.to_owned();
	};
	let mut parts = Vec::new();
	flatten(&json, &Style::default(), &mut parts);
	let mut out = String::new();
	let mut current = Style::default();
	for (style, text) in parts {
		if text.is_empty() {
			continue;
		}
		if style != current {
			match style.color {
				Some(c) => out.push_str(&format!("\u{a7}{}", CODES.as_bytes()[c] as char)),
				None if !current.is_empty() => out.push_str("\u{a7}r"),
				None => {}
			}
			for (on, (code, _)) in style.formats.iter().zip(FORMATS) {
				if *on {
					out.push_str(&format!("\u{a7}{code}"));
				}
			}
			current = style;
		}
		out.push_str(&text);
	}
	out
}

pub(super) fn legacy_title(s: &str) -> String {
	if matches!(parse(s), Some(Json::Object(fields)) if fields.len() == 1 && fields[0].0 == "translate")
	{
		return s.to_owned();
	}
	let mut out = String::from("{\"text\":");
	quote(&mut out, &json_to_legacy(s));
	out.push('}');
	out
}

pub(super) fn last_color(s: &str) -> char {
	let chars: Vec<char> = s.chars().collect();
	for i in (0..chars.len().saturating_sub(1)).rev() {
		if chars[i] == '\u{a7}' {
			let c = chars[i + 1].to_ascii_lowercase();
			if CODES.find(c).is_some_and(|i| i < 16) {
				return c;
			}
		}
	}
	'r'
}

pub(super) fn color_ordinal(c: char) -> i32 {
	CODES.find(c).map_or(21, |i| i as i32)
}

#[derive(Clone, Copy, PartialEq)]
struct Format {
	color: char,
	flags: u8,
}

impl Format {
	fn write(self, out: &mut String) {
		out.push('\u{a7}');
		out.push(self.color);
		for (i, code) in ['l', 'm', 'n', 'o'].into_iter().enumerate() {
			if self.flags & 1 << i != 0 {
				out.push('\u{a7}');
				out.push(code);
			}
		}
	}
}

pub(super) fn truncate_legacy(legacy: &str, default: char, limit: usize, prefix: bool) -> String {
	let start = Format {
		color: default,
		flags: 0,
	};
	let (mut written, mut current) = (start, start);
	let chars: Vec<char> = legacy.chars().collect();
	let mut out = String::new();
	let mut i = 0;
	while i < chars.len() {
		if chars[i] != '\u{a7}' || i + 1 == chars.len() {
			if current != written {
				current.write(&mut out);
				written = current;
			}
			out.push(chars[i]);
		} else {
			i += 1;
			match chars[i] {
				'r' => current = start,
				c @ ('l' | 'm' | 'n' | 'o') => current.flags |= 1 << "lmno".find(c).unwrap_or(0),
				c => current = Format { color: c, flags: 0 },
			}
		}
		i += 1;
	}
	if prefix && current != written {
		current.write(&mut out);
	}
	let mut units = 0;
	let mut cut: String = out
		.chars()
		.take_while(|c| {
			units += c.len_utf16();
			units <= limit
		})
		.collect();
	if cut.ends_with('\u{a7}') {
		cut.pop();
	}
	cut
}

pub(super) fn strip_colors(s: &str) -> String {
	let mut out = String::new();
	let mut chars = s.chars();
	while let Some(c) = chars.next() {
		if c == '\u{a7}' {
			chars.next();
		} else {
			out.push(c);
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn legacy_round_trips() {
		assert_eq!(legacy_to_json("plain", false), "{\"text\":\"plain\"}");
		let json = legacy_to_json("\u{a7}aGreen \u{a7}lbold\u{a7}r plain", false);
		assert_eq!(
			json,
			"{\"text\":\"\",\"extra\":[{\"text\":\"Green \",\"color\":\"green\"},{\"text\":\"bold\",\"color\":\"green\",\"bold\":true},{\"text\":\" plain\"}]}"
		);
		assert_eq!(
			json_to_legacy(&json),
			"\u{a7}aGreen \u{a7}a\u{a7}lbold\u{a7}r plain"
		);
		assert_eq!(
			legacy_to_json("\u{a7}cRed", true),
			"{\"text\":\"Red\",\"color\":\"red\",\"italic\":false}"
		);
		assert_eq!(json_to_legacy("{\"text\":\"a\\\"b\\u00e9\"}"), "a\"b\u{e9}");
	}
}
