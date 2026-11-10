use crate::gen::convert::Version;
use crate::pds::convert::Result;
use crate::registry;
use nbt::Tag;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static PARSED: OnceLock<Mutex<HashMap<String, &'static Value>>> = OnceLock::new();

fn login(version: Version) -> Option<&'static Value> {
	let folder = registry::folder(version, "loginPacket")?;
	let mut map = PARSED
		.get_or_init(Default::default)
		.lock()
		.expect("login packet cache poisoned");
	if let Some(v) = map.get(folder) {
		return Some(v);
	}
	let v: &'static Value = Box::leak(Box::new(
		serde_json::from_str(registry::login_packet(version)?).ok()?,
	));
	map.insert(folder.to_owned(), v);
	Some(v)
}

pub fn vanilla(version: Version, id: &str) -> Result<Vec<(String, Option<Tag>)>> {
	let Some(entries) =
		login(version).and_then(|v| v.get("dimensionCodec")?.get(id)?.get("entries")?.as_array())
	else {
		return crate::pds::convert::unconvertible(format!(
			"no vanilla {id} registry for {}",
			version.name()
		));
	};
	entries
		.iter()
		.map(|e| {
			let key = e
				.get("key")
				.and_then(Value::as_str)
				.unwrap_or_default()
				.to_owned();
			let value = e.get("value").map(typed).transpose()?;
			Ok((key, value))
		})
		.collect()
}

fn typed(v: &Value) -> Result<Tag> {
	match nbt::json::typed::read(v) {
		Some(p) => Ok(Tag::new(p)),
		None => crate::pds::convert::unconvertible(format!(
			"not a typed nbt value: {}",
			v.to_string().chars().take(80).collect::<String>()
		)),
	}
}
