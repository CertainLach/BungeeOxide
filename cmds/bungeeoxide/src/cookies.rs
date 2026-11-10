use std::collections::HashMap;

pub const MAX_PAYLOAD: usize = 5120;

#[derive(Default)]
pub struct Jar {
	cookies: HashMap<String, Vec<u8>>,
}

pub fn identifier(key: &str) -> Option<String> {
	let (namespace, path) = key.split_once(':').unwrap_or(("minecraft", key));
	let namespace = if namespace.is_empty() {
		"minecraft"
	} else {
		namespace
	};
	let valid = |s: &str, slash: bool| {
		s.bytes().all(|b| {
			matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.') || (slash && b == b'/')
		})
	};
	(!path.is_empty() && valid(namespace, false) && valid(path, true))
		.then(|| format!("{namespace}:{path}"))
}

impl Jar {
	pub fn store(&mut self, key: &str, value: Vec<u8>) -> Result<(), String> {
		let id = identifier(key).ok_or_else(|| format!("{key:?} is not an identifier"))?;
		if value.len() > MAX_PAYLOAD {
			return Err(format!(
				"{id} holds {} bytes, more than {MAX_PAYLOAD}",
				value.len()
			));
		}
		self.cookies.insert(id, value);
		Ok(())
	}

	pub fn get(&self, key: &str) -> Option<Vec<u8>> {
		self.cookies.get(&identifier(key)?).cloned()
	}

	pub fn len(&self) -> usize {
		self.cookies.len()
	}

	pub fn is_empty(&self) -> bool {
		self.cookies.is_empty()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn keys_are_identifiers() {
		assert_eq!(identifier("a"), Some("minecraft:a".to_owned()));
		assert_eq!(identifier("minecraft:a"), Some("minecraft:a".to_owned()));
		assert_eq!(
			identifier("my_pack:dir/x.y-z"),
			Some("my_pack:dir/x.y-z".to_owned())
		);
		assert_eq!(identifier(":a"), Some("minecraft:a".to_owned()));
		assert_eq!(identifier("Upper:a"), None);
		assert_eq!(identifier("a/b:c"), None);
		assert_eq!(identifier("a:"), None);
	}

	#[test]
	fn the_jar_keeps_vanilla_limits() {
		let mut jar = Jar::default();
		jar.store("a", vec![1]).unwrap();
		assert_eq!(jar.get("minecraft:a"), Some(vec![1]));
		jar.store("minecraft:a", vec![2]).unwrap();
		assert_eq!(jar.get("a"), Some(vec![2]));
		assert!(jar.store("big", vec![0; MAX_PAYLOAD + 1]).is_err());
		jar.store("full", vec![0; MAX_PAYLOAD]).unwrap();
		assert!(jar.store("Bad", vec![]).is_err());
		assert_eq!(jar.get("missing"), None);
		assert_eq!(jar.len(), 2);
	}
}
