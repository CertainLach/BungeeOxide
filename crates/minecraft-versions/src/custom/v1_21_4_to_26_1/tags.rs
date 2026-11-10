use super::util::remap;
use crate::gen::convert::v1_21_5::configuration_toClient_packet__tags__items__tags__items as tag;
use crate::gen::convert::v1_21_5::{
	configuration_toClient_packet__tags__items as config_tags,
	play_toClient_packet__tags__items as play_tags,
};
use crate::gen::convert::Version;

fn kind(registry: &str) -> Option<&'static str> {
	match registry.strip_prefix("minecraft:").unwrap_or(registry) {
		"block" => Some("blocks"),
		"item" => Some("items"),
		"entity_type" => Some("entities"),
		_ => None,
	}
}

fn known(to: Version, registry: &str) -> bool {
	let since = match registry.strip_prefix("minecraft:").unwrap_or(registry) {
		"pig_variant" | "cow_variant" | "chicken_variant" | "wolf_sound_variant"
		| "test_environment" | "test_instance" => Version::V1_21_5,
		"dialog" => Version::V1_21_6,
		"timeline" | "zombie_nautilus_variant" => Version::V1_21_11,
		"world_clock"
		| "cat_sound_variant"
		| "cow_sound_variant"
		| "pig_sound_variant"
		| "chicken_sound_variant" => Version::V26_1,
		_ => return true,
	};
	to >= since
}

fn convert(
	from: Version,
	to: Version,
	registry: &str,
	renames: &[(&str, &str, &str)],
	tags: Vec<tag::Ty>,
) -> Vec<tag::Ty> {
	let kind = kind(registry);
	let short = registry.strip_prefix("minecraft:").unwrap_or(registry);
	tags.into_iter()
		.map(|t| {
			let bare = t.tagName.strip_prefix("minecraft:").unwrap_or(&t.tagName);
			let tagName = match renames
				.iter()
				.find(|(r, old, _)| *r == short && *old == bare)
			{
				Some((_, _, new)) => format!("minecraft:{new}"),
				None => t.tagName,
			};
			let entries = match kind {
				Some(k) => t
					.entries
					.into_iter()
					.filter_map(|id| remap(k, from, to, id))
					.collect(),
				None => t.entries,
			};
			tag::Ty { tagName, entries }
		})
		.collect()
}

pub fn config(
	from: Version,
	to: Version,
	renames: &[(&str, &str, &str)],
	v: Vec<config_tags::Ty>,
) -> Vec<config_tags::Ty> {
	v.into_iter()
		.filter(|r| known(to, &r.tagType))
		.map(|r| config_tags::Ty {
			tags: convert(from, to, &r.tagType, renames, r.tags),
			tagType: r.tagType,
		})
		.collect()
}

pub fn play(
	from: Version,
	to: Version,
	renames: &[(&str, &str, &str)],
	v: Vec<play_tags::Ty>,
) -> Vec<play_tags::Ty> {
	v.into_iter()
		.filter(|r| known(to, &r.tagType))
		.map(|r| play_tags::Ty {
			tags: convert(from, to, &r.tagType, renames, r.tags),
			tagType: r.tagType,
		})
		.collect()
}
