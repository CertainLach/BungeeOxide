use super::util::remap;
use crate::generated::convert::Version;
use crate::generated::convert::v1_21_5::configuration_to_client_packet_tags_items_tags_items as tag;
use crate::generated::convert::v1_21_5::{
	configuration_to_client_packet_tags_items as config_tags,
	play_to_client_packet_tags_items as play_tags,
};

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
		"sulfur_cube_archetype" => Version::V26_2,
		"decorated_pot_pattern" | "block_transformer" | "worldgen/block_state_provider" => {
			Version::V26_3
		}
		"block_sound_set" | "dispense_behavior" => Version::V26_4_snapshot_10,
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
			let bare = t.tag_name.strip_prefix("minecraft:").unwrap_or(&t.tag_name);
			let tag_name = match renames
				.iter()
				.find(|(r, old, _)| *r == short && *old == bare)
			{
				Some((_, _, new)) => format!("minecraft:{new}"),
				None => t.tag_name,
			};
			let entries = match kind {
				Some(k) => t
					.entries
					.into_iter()
					.filter_map(|id| remap(k, from, to, id))
					.collect(),
				None => t.entries,
			};
			tag::Ty { tag_name, entries }
		})
		.collect()
}

pub type Derived = (&'static str, &'static str, &'static [&'static str]);

pub fn derive(to: Version, v: &mut Vec<config_tags::Ty>, derived: &[Derived]) {
	for (registry, name, values) in derived {
		let full = format!("minecraft:{registry}");
		let at = match v
			.iter()
			.position(|r| r.tag_type == full || r.tag_type == *registry)
		{
			Some(at) => at,
			None => {
				v.push(config_tags::Ty {
					tag_type: full,
					tags: Vec::new(),
				});
				v.len() - 1
			}
		};
		let tag_name = format!("minecraft:{name}");
		let group = &v[at].tags;
		if group
			.iter()
			.any(|t| t.tag_name == tag_name || t.tag_name == *name)
		{
			continue;
		}
		let names = kind(registry).and_then(|k| crate::registry::names(to, k));
		let mut entries = Vec::new();
		for value in *values {
			match value.strip_prefix('#') {
				Some(reference) => entries.extend(
					group
						.iter()
						.filter(|t| {
							t.tag_name == reference
								|| t.tag_name.strip_prefix("minecraft:")
									== reference.strip_prefix("minecraft:")
						})
						.flat_map(|t| t.entries.iter().copied()),
				),
				None => entries.extend(names.and_then(|n| n.id(value)).map(|id| id as i32)),
			}
		}
		v[at].tags.push(tag::Ty { tag_name, entries });
	}
}

pub fn config(
	from: Version,
	to: Version,
	renames: &[(&str, &str, &str)],
	v: Vec<config_tags::Ty>,
) -> Vec<config_tags::Ty> {
	v.into_iter()
		.filter(|r| known(to, &r.tag_type))
		.map(|r| config_tags::Ty {
			tags: convert(from, to, &r.tag_type, renames, r.tags),
			tag_type: r.tag_type,
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
		.filter(|r| known(to, &r.tag_type))
		.map(|r| play_tags::Ty {
			tags: convert(from, to, &r.tag_type, renames, r.tags),
			tag_type: r.tag_type,
		})
		.collect()
}
