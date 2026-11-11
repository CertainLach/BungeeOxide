use crate::gen::convert::{v1_10, v1_11, v1_12, v1_8, v1_9, v1_9_2, v1_9_4, Version};
use crate::pds::convert::{Error, Result};
use crate::registry;
use crate::session::Session;

type C8 = v1_8::play_to_client_packet::Ty;

const HOSTILE: &[&str] = &[
	"blaze",
	"creeper",
	"elder_guardian",
	"enderdragon",
	"endermen",
	"endermite",
	"ghast",
	"guardian",
	"husk",
	"magmacube",
	"shulker",
	"silverfish",
	"skeleton",
	"slime",
	"small_magmacube",
	"small_slime",
	"spider",
	"stray",
	"witch",
	"wither",
	"wither_skeleton",
	"zombie",
	"zombie_pig",
	"zombie_villager",
];

fn rename(name: &str, from: Version, to: Version) -> Option<&'static str> {
	let id = registry::names(from, "sounds")?.id(name)?;
	registry::names(to, "sounds")?.name(registry::remap_name("sounds", from, to, id)?)
}

fn remap(id: i32, from: Version, to: Version) -> Result<i32> {
	u32::try_from(id)
		.ok()
		.and_then(|id| registry::remap_name("sounds", from, to, id))
		.map(|id| id as i32)
		.ok_or(Error::Dropped)
}

fn category(name: &str) -> i32 {
	let mob = name
		.strip_prefix("entity.")
		.and_then(|r| r.split('.').next());
	match name {
		_ if name.starts_with("music.") => 1,
		_ if name.starts_with("record.") || name.starts_with("block.note.") => 2,
		_ if name.starts_with("weather.") || name.starts_with("entity.lightning.") => 3,
		_ if name.starts_with("block.") => 4,
		_ if mob.is_some_and(|m| HOSTILE.contains(&m)) => 5,
		_ if name.starts_with("entity.player.")
			|| name.starts_with("item.")
			|| name.starts_with("ui.") =>
		{
			7
		}
		_ if name.starts_with("ambient.") => 8,
		_ => 6,
	}
}

pub fn legacy_sound_name_1_9(
	src: &v1_8::play_to_client_packet_named_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(rename(&src.sound_name, Version::V1_8, Version::V1_9)
		.map_or_else(|| src.sound_name.clone(), str::to_owned))
}

pub fn legacy_sound_category_1_9(
	src: &v1_8::play_to_client_packet_named_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(category(
		rename(&src.sound_name, Version::V1_8, Version::V1_9).unwrap_or(&src.sound_name),
	))
}

pub fn legacy_sound_name_1_8(
	src: &v1_9::play_to_client_packet_named_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<String> {
	let vanilla = Version::ALL
		.iter()
		.any(|v| registry::names(*v, "sounds").is_some_and(|n| n.id(&src.sound_name).is_some()));
	match registry::sound_fallback(&src.sound_name, Version::V1_8) {
		Some(name) => Ok(name.to_owned()),
		None if vanilla => Err(Error::Dropped),
		None => Ok(src.sound_name.clone()),
	}
}

pub fn legacy_down_sound_effect_1_8(
	p: v1_9::play_to_client_packet_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<Vec<C8>> {
	let name = u32::try_from(p.sound_id)
		.ok()
		.and_then(|id| registry::remap_name("sounds", Version::V1_9, Version::V1_8, id))
		.and_then(|id| registry::names(Version::V1_8, "sounds")?.name(id));
	let Some(name) = name else { return Ok(vec![]) };
	Ok(vec![C8::NamedSoundEffect(
		v1_8::play_to_client_packet_named_sound_effect::Ty {
			sound_name: name.to_owned(),
			x: p.x,
			y: p.y,
			z: p.z,
			volume: p.volume,
			pitch: p.pitch,
		},
	)])
}

macro_rules! sound_ids {
	($($old:ident $a:ident => $new:ident $b:ident: $up:ident, $down:ident;)*) => {$(
		pub fn $up(src: &$old::play_to_client_packet_sound_effect::Ty, _cx: &mut Session) -> Result<i32> {
			remap(src.sound_id, Version::$a, Version::$b)
		}

		pub fn $down(src: &$new::play_to_client_packet_sound_effect::Ty, _cx: &mut Session) -> Result<i32> {
			remap(src.sound_id, Version::$b, Version::$a)
		}
	)*};
}

sound_ids! {
	v1_9_2 V1_9_2 => v1_9_4 V1_9_4: legacy_sound_id_1_9_4, legacy_sound_id_down_1_9_2;
	v1_9_4 V1_9_4 => v1_10 V1_10: legacy_sound_id_1_10, legacy_sound_id_down_1_9_4;
	v1_10 V1_10 => v1_11 V1_11: legacy_sound_id_1_11, legacy_sound_id_down_1_10;
	v1_11 V1_11 => v1_12 V1_12: legacy_sound_id_1_12, legacy_sound_id_down_1_11;
}
