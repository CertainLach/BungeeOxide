use crate::{Entry, List, Payload};

struct Reader<'a> {
	s: &'a [u8],
	at: usize,
}

impl Reader<'_> {
	fn skip(&mut self) {
		while self.s.get(self.at).is_some_and(u8::is_ascii_whitespace) {
			self.at += 1;
		}
	}

	fn peek(&mut self) -> Option<u8> {
		self.skip();
		self.s.get(self.at).copied()
	}

	fn eat(&mut self, c: u8) -> bool {
		let hit = self.peek() == Some(c);
		self.at += hit as usize;
		hit
	}

	fn quoted(&mut self, q: u8) -> Option<String> {
		self.at += 1;
		let mut out = Vec::new();
		loop {
			let c = *self.s.get(self.at)?;
			self.at += 1;
			match c {
				b'\\' => {
					out.push(*self.s.get(self.at)?);
					self.at += 1;
				}
				c if c == q => return String::from_utf8(out).ok(),
				c => out.push(c),
			}
		}
	}

	fn bare(&mut self) -> &str {
		let start = self.at;
		while self.s.get(self.at).is_some_and(|c| bare_char(*c)) {
			self.at += 1;
		}
		std::str::from_utf8(&self.s[start..self.at]).unwrap_or_default()
	}

	fn key(&mut self) -> Option<String> {
		match self.peek()? {
			q @ (b'"' | b'\'') => self.quoted(q),
			_ => Some(self.bare().to_owned()).filter(|k| !k.is_empty()),
		}
	}

	fn value(&mut self) -> Option<Payload> {
		match self.peek()? {
			b'{' => {
				self.at += 1;
				let mut c = Vec::new();
				if self.eat(b'}') {
					return Some(Payload::compound(c));
				}
				loop {
					let k = self.key()?;
					self.eat(b':').then_some(())?;
					c.push(Entry::new(k, self.value()?));
					if self.eat(b'}') {
						return Some(Payload::compound(c));
					}
					self.eat(b',').then_some(())?;
				}
			}
			b'[' => {
				self.at += 1;
				let typed = match (self.s.get(self.at), self.s.get(self.at + 1)) {
					(Some(t @ (b'B' | b'I' | b'L')), Some(b';')) => Some(*t),
					_ => None,
				};
				if typed.is_some() {
					self.at += 2;
				}
				let mut values = Vec::new();
				if !self.eat(b']') {
					loop {
						values.push(self.value()?);
						if self.eat(b']') {
							break;
						}
						self.eat(b',').then_some(())?;
					}
				}
				let num = |p: &Payload| p.as_i64().unwrap_or(0);
				Some(match typed {
					Some(b'B') => Payload::byteArray(values.iter().map(|v| num(v) as i8).collect()),
					Some(b'I') => Payload::intArray(values.iter().map(|v| num(v) as i32).collect()),
					Some(_) => Payload::longArray(values.iter().map(num).collect()),
					None => Payload::list(List::of(values)),
				})
			}
			q @ (b'"' | b'\'') => self.quoted(q).map(Payload::string),
			_ => Some(scalar(self.bare())),
		}
	}
}

fn bare_char(c: u8) -> bool {
	c.is_ascii_alphanumeric() || b"_-.+".contains(&c)
}

fn scalar(s: &str) -> Payload {
	let lower = s.to_ascii_lowercase();
	let (body, suffix) = match lower.as_bytes().last() {
		Some(c @ (b'b' | b's' | b'l' | b'f' | b'd')) if s.len() > 1 => {
			(&s[..s.len() - 1], Some(*c))
		}
		_ => (s, None),
	};
	let parsed = match suffix {
		Some(b'b') => body.parse().ok().map(Payload::byte),
		Some(b's') => body.parse().ok().map(Payload::short),
		Some(b'l') => body.parse().ok().map(Payload::long),
		Some(b'f') => body.parse().ok().map(Payload::float),
		Some(_) => body.parse().ok().map(Payload::double),
		None if lower == "true" => Some(Payload::byte(1)),
		None if lower == "false" => Some(Payload::byte(0)),
		None => s.parse().ok().map(Payload::int).or_else(|| {
			s.contains(['.', 'e', 'E'])
				.then(|| s.parse().ok().map(Payload::double))
				.flatten()
		}),
	};
	parsed.unwrap_or_else(|| Payload::string(s.to_owned()))
}

pub fn parse(s: &str) -> Option<Payload> {
	let mut r = Reader {
		s: s.as_bytes(),
		at: 0,
	};
	let v = r.value()?;
	(r.peek().is_none()).then_some(v)
}

pub fn quote(s: &str) -> String {
	format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn key(s: &str) -> String {
	match !s.is_empty() && s.bytes().all(bare_char) {
		true => s.to_owned(),
		false => quote(s),
	}
}

fn join<T>(values: impl Iterator<Item = T>, f: impl Fn(T) -> String) -> String {
	values.map(f).collect::<Vec<_>>().join(",")
}

pub fn write(p: &Payload) -> String {
	match p {
		Payload::end => String::new(),
		Payload::byte(v) => format!("{v}b"),
		Payload::short(v) => format!("{v}s"),
		Payload::int(v) => v.to_string(),
		Payload::long(v) => format!("{v}L"),
		Payload::float(v) => format!("{v}f"),
		Payload::double(v) => format!("{v}d"),
		Payload::string(s) => quote(s),
		Payload::byteArray(v) => format!("[B;{}]", join(v.iter(), |x| format!("{x}b"))),
		Payload::intArray(v) => format!("[I;{}]", join(v.iter(), |x| x.to_string())),
		Payload::longArray(v) => format!("[L;{}]", join(v.iter(), |x| format!("{x}L"))),
		Payload::list(l) => format!("[{}]", join(l.values().iter(), write)),
		Payload::compound(c) => format!(
			"{{{}}}",
			join(c.iter(), |e| format!(
				"{}:{}",
				key(&e.name),
				write(&e.value)
			))
		),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ListItems;

	fn compound(entries: Vec<(&str, Payload)>) -> Payload {
		Payload::compound(entries.into_iter().map(|(k, v)| Entry::new(k, v)).collect())
	}

	#[test]
	fn number_suffixes() {
		let cases: [(&str, Payload); 16] = [
			("1b", Payload::byte(1)),
			("-3B", Payload::byte(-3)),
			("7s", Payload::short(7)),
			("7S", Payload::short(7)),
			("42", Payload::int(42)),
			("-42", Payload::int(-42)),
			("9000000000L", Payload::long(9000000000)),
			("5l", Payload::long(5)),
			("1.5f", Payload::float(1.5)),
			("2F", Payload::float(2.0)),
			("1.25d", Payload::double(1.25)),
			("1.25", Payload::double(1.25)),
			("1e3", Payload::double(1000.0)),
			("true", Payload::byte(1)),
			("FALSE", Payload::byte(0)),
			("300b", Payload::string("300b".into())),
		];
		for (text, value) in cases {
			assert_eq!(parse(text), Some(value), "{text}");
		}
		assert_eq!(
			parse("9000000000"),
			Some(Payload::string("9000000000".into()))
		);
		assert_eq!(parse("bed"), Some(Payload::string("bed".into())));
		assert_eq!(parse("d"), Some(Payload::string("d".into())));
	}

	#[test]
	fn strings_quotes_and_escapes() {
		assert_eq!(
			parse(r#""a \"b\" \\ c""#),
			Some(Payload::string(r#"a "b" \ c"#.into()))
		);
		assert_eq!(
			parse(r#"'it\'s "x"'"#),
			Some(Payload::string(r#"it's "x""#.into()))
		);
		assert_eq!(
			parse("minecraft.stone_1"),
			Some(Payload::string("minecraft.stone_1".into()))
		);
		assert_eq!(
			parse("\"caf\u{e9}\""),
			Some(Payload::string("caf\u{e9}".into()))
		);
		assert_eq!(parse("\"open"), None);
		assert_eq!(
			write(&Payload::string(r#"say "hi" \o/"#.into())),
			r#""say \"hi\" \\o/""#
		);
		assert_eq!(quote("a\"b"), r#""a\"b""#);
	}

	#[test]
	fn compounds_lists_and_keys() {
		let parsed = parse(r#" { id : "minecraft:stone", Count: 1b, 'odd key': [1, 2, 3], "": {}, tag: {display: {Lore: ["a", 'b']}} } "#).unwrap();
		let expected = compound(vec![
			("id", "minecraft:stone".into()),
			("Count", Payload::byte(1)),
			("odd key", ListItems::int(vec![1, 2, 3]).into()),
			("", Payload::compound(vec![])),
			(
				"tag",
				compound(vec![(
					"display",
					compound(vec![(
						"Lore",
						ListItems::string(vec!["a".into(), "b".into()]).into(),
					)]),
				)]),
			),
		]);
		assert_eq!(parsed, expected);
		assert_eq!(
			write(&parsed),
			r#"{id:"minecraft:stone",Count:1b,"odd key":[1,2,3],"":{},tag:{display:{Lore:["a","b"]}}}"#
		);
		assert_eq!(parse(&write(&parsed)), Some(parsed));
		assert_eq!(parse("[]"), Some(Payload::list(List::of(vec![]))));
		assert_eq!(parse("[1, 2b]"), Some(ListItems::int(vec![1]).into()));
		assert_eq!(parse("{a:1,}"), None);
		assert_eq!(parse("{a:1} x"), None);
		assert_eq!(parse("{:1}"), None);
		assert_eq!(
			parse("[[1],[2s]]"),
			Some(
				ListItems::list(vec![
					List::new(ListItems::int(vec![1])),
					List::new(ListItems::short(vec![2]))
				])
				.into()
			)
		);
	}

	#[test]
	fn typed_arrays() {
		assert_eq!(parse("[B;1b,-2b]"), Some(Payload::byteArray(vec![1, -2])));
		assert_eq!(
			parse("[I; 1, 2 ,3]"),
			Some(Payload::intArray(vec![1, 2, 3]))
		);
		assert_eq!(parse("[L;1L,2l]"), Some(Payload::longArray(vec![1, 2])));
		assert_eq!(parse("[I;]"), Some(Payload::intArray(vec![])));
		assert_eq!(parse("[I;x]"), Some(Payload::intArray(vec![0])));
		assert_eq!(write(&Payload::byteArray(vec![1, -2])), "[B;1b,-2b]");
		assert_eq!(write(&Payload::intArray(vec![1, 2])), "[I;1,2]");
		assert_eq!(write(&Payload::longArray(vec![])), "[L;]");
		for v in [
			Payload::byteArray(vec![1, -2]),
			Payload::intArray(vec![7]),
			Payload::longArray(vec![1 << 40]),
		] {
			assert_eq!(parse(&write(&v)), Some(v));
		}
	}

	#[test]
	fn numbers_write_with_suffixes() {
		let v = compound(vec![
			("b", Payload::byte(-1)),
			("s", Payload::short(2)),
			("i", Payload::int(3)),
			("l", Payload::long(4)),
			("f", Payload::float(1.5)),
			("d", Payload::double(2.0)),
		]);
		assert_eq!(write(&v), "{b:-1b,s:2s,i:3,l:4L,f:1.5f,d:2d}");
		assert_eq!(parse(&write(&v)), Some(v));
		assert_eq!(write(&Payload::end), "");
	}
}
