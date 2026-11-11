use super::util::{decode, encode};
use super::world;
use crate::gen::convert::v1_21_5::configuration_to_client_packet_registry_data as registry_data;
use crate::gen::convert::{step, Root, Version};
use crate::pds::convert::{unconvertible, Error, Result};
use crate::Session;
use bungeeoxide_nbt::Payload;

#[derive(Default)]
pub struct Meta {
	pub entity: Option<String>,
	pub key: u8,
}

pub fn meta(cx: &mut Session) -> &mut Meta {
	cx.state::<Meta>()
}

pub fn meta_entity(cx: &mut Session, version: Version, id: i32) {
	let entity = world::kind(cx, version, id);
	meta(cx).entity = entity;
}

pub fn fallback_key(cx: &mut Session, from: Version, to: Version, key: u8) -> Option<Result<u8>> {
	let name = meta(cx).entity.clone()?;
	let id = crate::registry::names(from, "entities")?.id(&name)?;
	if crate::registry::names(to, "entities")?.id(&name).is_some() {
		return None;
	}
	let mapped = crate::registry::remap_name("entities", from, to, id).and_then(|target| {
		let field = crate::registry::entity_metadata_keys(from, id)?.get(key as usize)?;
		crate::registry::entity_metadata_keys(to, target)?
			.iter()
			.position(|k| k == field)
	});
	Some(mapped.map(|i| i as u8).ok_or(Error::Dropped))
}

pub fn is(cx: &mut Session, names: &[&str]) -> bool {
	meta(cx)
		.entity
		.as_deref()
		.is_some_and(|e| names.contains(&e))
}

pub fn spawned(cx: &mut Session, id: i32, version: Version, ty: i32) -> Option<&'static str> {
	world::spawned(cx, id, version, ty);
	crate::registry::names(version, "entities")?.name(ty.max(0) as u32)
}

pub fn via_step<A, B>(
	from: Version,
	to: Version,
	root: Root,
	packet: &A,
	ser: impl Fn(&A, &mut crate::pds::Output) -> crate::pds::Result<()>,
	de: impl Fn(&mut crate::pds::Input) -> crate::pds::Result<B>,
	cx: &mut Session,
) -> Result<B> {
	let out = step(root, from, to, &encode(packet, ser)?, cx)?;
	match out.as_slice() {
		[bytes] => decode(bytes, de),
		[] => Err(Error::Dropped),
		_ => unconvertible("a nested conversion produced several packets"),
	}
}

pub fn registry_entries(
	p: &mut registry_data::Ty,
	id: &str,
	mut f: impl FnMut(&str, &mut Payload),
) {
	if p.id.strip_prefix("minecraft:").unwrap_or(&p.id) != id {
		return;
	}
	for e in &mut p.entries {
		if let Some(tag) = &mut e.value {
			f(&e.key, &mut tag.value_mut());
		}
	}
}

pub fn count_biomes(p: &registry_data::Ty, cx: &mut Session) {
	if p.id.strip_prefix("minecraft:").unwrap_or(&p.id) == "worldgen/biome" {
		world::world(cx).biomes = Some(p.entries.len() as u32);
	}
}

pub fn registry(
	id: &str,
	entries: Vec<(String, Option<bungeeoxide_nbt::Tag>)>,
) -> registry_data::Ty {
	registry_data::Ty {
		id: format!("minecraft:{id}"),
		entries: entries
			.into_iter()
			.map(|(key, value)| crate::gen::convert::v1_21_5::configuration_to_client_packet_registry_data_entries_items::Ty { key, value })
			.collect(),
	}
}

macro_rules! same_bytes {
	($name:ident: $am:ident::$a:ident => $bm:ident::$b:ident) => {
		pub fn $name(
			v: $am::$a::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$bm::$b::Ty> {
			super::util::recode(&v, $am::$a::serialize, $bm::$b::deserialize)
		}
	};
}

pub(crate) use same_bytes;

macro_rules! text_down {
	($b:ident => $a:ident, $f:path, {
		tag: $tag:ident,
		links: [$config_links:ident, $play_links:ident],
		window: $window:ident,
		chat: $chat:ident,
		packets: [$($ty:ident: $visit:ident => $name:ident),* $(,)?],
		arms: [$($root:ident: $arm:ident => $arm_name:ident),* $(,)?] $(,)?
	}) => {
		pub fn $tag(v: ::bungeeoxide_nbt::Tag, _cx: &mut crate::Session) -> crate::pds::convert::Result<::bungeeoxide_nbt::Tag> {
			Ok($f(v))
		}

		$(
			pub fn $name(v: $b::$ty::Ty, _cx: &mut crate::Session) -> crate::pds::convert::Result<$a::$ty::Ty> {
				Ok(super::s1_21_5::$visit(v, $f))
			}
		)*

		$(
			pub fn $arm_name(v: ::bungeeoxide_nbt::Tag, _cx: &mut crate::Session) -> crate::pds::convert::Result<Vec<$a::$root::Ty>> {
				Ok(vec![$a::$root::Ty::$arm($f(v))])
			}
		)*

		pub fn $config_links(v: super::s1_21_5::Links, _cx: &mut crate::Session) -> crate::pds::convert::Result<Vec<$a::configuration_to_client_packet::Ty>> {
			Ok(vec![$a::configuration_to_client_packet::Ty::ServerLinks(super::s1_21_5::links(v, $f))])
		}

		pub fn $play_links(v: super::s1_21_5::Links, _cx: &mut crate::Session) -> crate::pds::convert::Result<Vec<$a::play_to_client_packet::Ty>> {
			Ok(vec![$a::play_to_client_packet::Ty::ServerLinks(super::s1_21_5::links(v, $f))])
		}

		pub fn $window(
			mut v: $b::play_to_client_packet_open_window::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$a::play_to_client_packet_open_window::Ty> {
			v.window_title = $f(v.window_title);
			Ok(v)
		}

		pub fn $chat(
			mut v: $b::play_to_client_packet_player_chat::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$a::play_to_client_packet_player_chat::Ty> {
			v.unsigned_chat_content = v.unsigned_chat_content.map($f);
			v.network_name = $f(v.network_name);
			v.network_target_name = v.network_target_name.map($f);
			Ok(v)
		}
	};
}

pub(crate) use text_down;
