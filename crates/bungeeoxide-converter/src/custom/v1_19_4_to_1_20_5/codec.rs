use crate::gen::convert::Version;
use crate::pds::convert::{unconvertible, Result};
use bungeeoxide_nbt::Tag;
use serde_json::Value;

pub fn login(version: Version) -> Result<Value> {
	let Some(text) = crate::registry::login_packet(version) else {
		return unconvertible(format!("no vanilla login packet for {}", version.name()));
	};
	serde_json::from_str(text)
		.or_else(|e| unconvertible(format!("loginPacket.json of {}: {e}", version.name())))
}

pub fn vanilla(version: Version) -> Result<Tag> {
	match bungeeoxide_nbt::json::typed::read(&login(version)?["dimensionCodec"]) {
		Some(codec) => Ok(Tag::new(codec)),
		None => unconvertible(format!("no typed dimension codec for {}", version.name())),
	}
}
