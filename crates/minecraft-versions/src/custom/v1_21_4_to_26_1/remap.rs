use super::util::remap;
use crate::gen::convert::v1_21_5::{IDSet, ItemSoundEvent, ItemSoundHolder};
use crate::gen::convert::Version;
use crate::pds::convert::{Error, Result};
use crate::registry;

pub fn item(from: Version, to: Version, v: i32) -> Result<i32> {
	Ok(remap("items", from, to, v).unwrap_or(0))
}

pub fn block(from: Version, to: Version, v: i32) -> Result<i32> {
	Ok(remap("blocks", from, to, v).unwrap_or(0))
}

pub fn state(from: Version, to: Version, v: i32) -> Result<i32> {
	Ok(registry::remap_state(from, to, v.max(0) as u32) as i32)
}

pub fn entity(from: Version, to: Version, v: i32) -> Result<i32> {
	remap("entities", from, to, v).ok_or(Error::Dropped)
}

pub fn sound_name(version: Version, id: i32) -> Option<&'static str> {
	registry::names(version, "sounds")?.name(u32::try_from(id + 1).ok()?)
}

pub fn sound_id(version: Version, name: &str) -> Option<i32> {
	registry::names(version, "sounds")?
		.id(name)
		.map(|v| v as i32 - 1)
}

pub fn sound(from: Version, to: Version, v: ItemSoundHolder::Ty) -> Result<ItemSoundHolder::Ty> {
	let ItemSoundHolder::Ty::Id(id) = v else {
		return Ok(v);
	};
	if let Some(new) = remap("sounds", from, to, id + 1) {
		return Ok(ItemSoundHolder::Ty::Id(new - 1));
	}
	match sound_name(from, id) {
		Some(name) => Ok(ItemSoundHolder::Ty::Inline(ItemSoundEvent::Ty {
			soundName: format!("minecraft:{name}"),
			fixedRange: None,
		})),
		None => Err(Error::Dropped),
	}
}

fn list(kind: &str, from: Version, to: Version, v: Vec<i32>) -> Vec<i32> {
	v.into_iter()
		.filter_map(|id| remap(kind, from, to, id))
		.collect()
}

pub fn items(from: Version, to: Version, v: Vec<i32>) -> Result<Vec<i32>> {
	Ok(list("items", from, to, v))
}

fn set(kind: &str, from: Version, to: Version, v: IDSet::Ty) -> Result<IDSet::Ty> {
	Ok(match v {
		IDSet::Ty::Ids(ids) => IDSet::Ty::Ids(list(kind, from, to, ids)),
		named => named,
	})
}

pub fn item_set(from: Version, to: Version, v: IDSet::Ty) -> Result<IDSet::Ty> {
	set("items", from, to, v)
}

pub fn block_set(from: Version, to: Version, v: IDSet::Ty) -> Result<IDSet::Ty> {
	set("blocks", from, to, v)
}

pub fn entity_set(from: Version, to: Version, v: IDSet::Ty) -> Result<IDSet::Ty> {
	set("entities", from, to, v)
}

macro_rules! remaps {
	($from:path => $to:path { $($name:ident: $ty:ty = $f:path;)* }) => {
		$(
			pub fn $name(v: $ty, _cx: &mut crate::Session) -> crate::pds::convert::Result<$ty> {
				$f($from, $to, v)
			}
		)*
	};
}

pub(crate) use remaps;
