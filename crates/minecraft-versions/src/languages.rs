use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::gen::convert::{index_of, VERSIONS};

pub struct Languages {
	folders: Vec<Option<u32>>,
	keys: HashMap<String, Key>,
}

struct Key {
	folders: u128,
	pattern: String,
}

impl Languages {
	pub fn new<'a>(
		index: impl IntoIterator<Item = (&'a str, &'a str)>,
		strings: impl IntoIterator<Item = (&'a str, Vec<(String, String)>)>,
	) -> Self {
		let index: Vec<(String, String)> = index
			.into_iter()
			.map(|(v, f)| (v.to_owned(), f.to_owned()))
			.collect();
		let mut strings: HashMap<&str, Vec<(String, String)>> = strings.into_iter().collect();
		Self::build(index, |folder| {
			Ok(strings.remove(folder).unwrap_or_default())
		})
		.expect("strings in memory")
	}

	pub fn read(dir: &Path) -> io::Result<Self> {
		let index = std::fs::read_to_string(dir.join("index.tsv"))?
			.lines()
			.filter_map(|l| l.split_once('\t'))
			.map(|(v, f)| (v.to_owned(), f.to_owned()))
			.collect();
		Self::build(index, |folder| {
			let text = std::fs::read_to_string(dir.join(format!("{folder}.json")))?;
			let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&text)?;
			Ok(map
				.into_iter()
				.filter_map(|(k, v)| Some((k, v.as_str()?.to_owned())))
				.collect())
		})
	}

	fn build(
		index: Vec<(String, String)>,
		mut load: impl FnMut(&str) -> io::Result<Vec<(String, String)>>,
	) -> io::Result<Self> {
		let mut rows: Vec<(usize, String)> = index
			.into_iter()
			.filter_map(|(v, f)| Some((index_of(&v)?, f)))
			.collect();
		rows.sort_by_key(|(v, _)| *v);
		let mut order: Vec<String> = Vec::new();
		let mut at = vec![None; VERSIONS.len()];
		for (v, folder) in &rows {
			let i = match order.iter().position(|f| f == folder) {
				Some(i) => i,
				None => {
					order.push(folder.clone());
					order.len() - 1
				}
			};
			at[*v] = Some(i as u32);
		}
		if order.len() > 128 {
			return Err(io::Error::other("more than 128 language folders"));
		}
		let mut folders = vec![None; VERSIONS.len()];
		let mut next = None;
		for v in (0..VERSIONS.len()).rev() {
			next = at[v].or(next);
			folders[v] = next;
		}
		let mut keys: HashMap<String, Key> = HashMap::new();
		for (i, folder) in order.iter().enumerate() {
			for (k, v) in load(folder)? {
				let key = keys.entry(k).or_insert_with(|| Key {
					folders: 0,
					pattern: String::new(),
				});
				key.folders |= 1 << i;
				key.pattern = v;
			}
		}
		Ok(Self { folders, keys })
	}

	pub fn knows(&self, version: usize, key: &str) -> Option<bool> {
		let folder = (*self.folders.get(version)?)?;
		Some(
			self.keys
				.get(key)
				.is_some_and(|k| k.folders >> folder & 1 == 1),
		)
	}

	pub fn pattern(&self, key: &str) -> Option<&str> {
		self.keys.get(key).map(|k| k.pattern.as_str())
	}
}

static DIR: OnceLock<PathBuf> = OnceLock::new();
static LOADED: OnceLock<Option<Languages>> = OnceLock::new();

pub fn load(dir: impl Into<PathBuf>) -> bool {
	DIR.set(dir.into()).is_ok()
}

pub fn install(languages: Languages) -> bool {
	LOADED.set(Some(languages)).is_ok()
}

pub fn get() -> Option<&'static Languages> {
	LOADED
		.get_or_init(|| DIR.get().and_then(|dir| Languages::read(dir).ok()))
		.as_ref()
}

const BUILTIN: [(&str, &str); 12] = [
	("chat.type.text", "<%s> %s"),
	("chat.type.emote", "* %s %s"),
	("chat.type.announcement", "[%s] %s"),
	("chat.type.admin", "[%s: %s]"),
	(
		"chat.type.achievement",
		"%s has just earned the achievement %s",
	),
	("multiplayer.player.joined", "%s joined the game"),
	("multiplayer.player.left", "%s left the game"),
	(
		"commands.generic.notFound",
		"Unknown command. Try /help for a list of commands",
	),
	(
		"commands.generic.permission",
		"You do not have permission to use this command",
	),
	("death.attack.generic", "%s died"),
	("death.fell.accident.generic", "%s fell from a high place"),
	(
		"chat.link.confirm",
		"Are you sure you want to open the following website?",
	),
];

pub fn english(key: &str) -> Option<&'static str> {
	match get() {
		Some(languages) => languages.pattern(key),
		None => BUILTIN.iter().find(|(k, _)| *k == key).map(|(_, p)| *p),
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Piece<'a> {
	Text(&'a str),
	Arg(usize),
}

pub fn pieces(pattern: &str) -> Vec<Piece<'_>> {
	let mut out = Vec::new();
	let mut next = 0;
	let mut rest = pattern;
	while let Some(at) = rest.find('%') {
		if at > 0 {
			out.push(Piece::Text(&rest[..at]));
		}
		let tail = &rest[at + 1..];
		let digits = tail.bytes().take_while(u8::is_ascii_digit).count();
		let explicit = digits > 0 && tail.as_bytes().get(digits) == Some(&b'$');
		let kind = tail
			.as_bytes()
			.get(if explicit { digits + 1 } else { 0 })
			.copied();
		match kind {
			Some(b'%') if !explicit => {
				out.push(Piece::Text("%"));
				rest = &tail[1..];
			}
			Some(c) if c.is_ascii_alphabetic() => {
				let index = if explicit {
					tail[..digits]
						.parse::<usize>()
						.unwrap_or(0)
						.saturating_sub(1)
				} else {
					next += 1;
					next - 1
				};
				out.push(Piece::Arg(index));
				rest = &tail[if explicit { digits + 2 } else { 1 }..];
			}
			_ => {
				out.push(Piece::Text("%"));
				rest = tail;
			}
		}
	}
	if !rest.is_empty() {
		out.push(Piece::Text(rest));
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn patterns_split_into_text_and_arguments() {
		use Piece::*;
		assert_eq!(
			pieces("<%s> %s"),
			vec![Text("<"), Arg(0), Text("> "), Arg(1)]
		);
		assert_eq!(
			pieces("%2$s by %1$s, %s"),
			vec![Arg(1), Text(" by "), Arg(0), Text(", "), Arg(0)]
		);
		assert_eq!(
			pieces("100%% %.2f %"),
			vec![
				Text("100"),
				Text("%"),
				Text(" "),
				Text("%"),
				Text(".2f "),
				Text("%")
			]
		);
		assert_eq!(pieces("plain"), vec![Text("plain")]);
	}

	#[test]
	fn versions_without_a_language_borrow_the_next_one() {
		let l = Languages::new(
			[("1.7", "a"), ("1.8", "b"), ("1.9", "b")],
			[
				("a", vec![("old".to_owned(), "Old %s".to_owned())]),
				(
					"b",
					vec![
						("old".to_owned(), "Older %s".to_owned()),
						("new".to_owned(), "New".to_owned()),
					],
				),
			],
		);
		let at = |v| index_of(v).unwrap();
		assert_eq!(l.knows(at("1.6.4"), "old"), Some(true));
		assert_eq!(l.knows(at("1.6.4"), "new"), Some(false));
		assert_eq!(l.knows(at("1.9"), "new"), Some(true));
		assert_eq!(l.knows(at("1.12.2"), "new"), None);
		assert_eq!(l.pattern("old"), Some("Older %s"));
		assert_eq!(l.pattern("missing"), None);
	}
}
