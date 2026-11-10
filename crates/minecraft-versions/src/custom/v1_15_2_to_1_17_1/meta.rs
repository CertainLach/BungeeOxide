use ::nbt::{CompoundExt, CompoundVecExt, Entry, Payload, Payload as P};

use super::{item, remap, state, substitute};
use crate::gen::convert::{v1_15_2, v1_16, v1_17, Version};
use crate::registry;

pub(super) type Slot = v1_16::slot::Ty;
pub(super) type Pos = v1_16::position::Ty;

pub(super) enum PData {
	None,
	Dust([f32; 4]),
	Transition([f32; 7]),
	State(i32),
	Item(Slot),
	Vibration(v1_17::particleData__v37::Ty),
}

pub(super) struct Particle {
	pub(super) id: i32,
	pub(super) data: PData,
}

pub(super) enum Meta {
	Byte(i8),
	VarInt(i32),
	Float(f32),
	Str(String),
	Chat(String),
	OptChat(Option<String>),
	Slot(Slot),
	Bool(bool),
	Rotation(f32, f32, f32),
	Pos(Pos),
	OptPos(Option<Pos>),
	Direction(i32),
	OptUuid(Option<u128>),
	OptState(i32),
	Nbt(Entry),
	Particle(Particle),
	Villager(i32, i32, i32),
	OptVarInt(i32),
	Pose(i32),
	Other(i32),
}

pub(super) fn item_name(version: Version, id: i32) -> &'static str {
	registry::names(version, "items")
		.and_then(|n| n.name(id as u32))
		.unwrap_or("")
}

fn item_id(version: Version, name: &str) -> Option<i32> {
	registry::names(version, "items")?
		.id(name)
		.map(|v| v as i32)
}

fn title(name: &str) -> String {
	name.split('_')
		.map(|w| {
			let mut c = w.chars();
			c.next().map_or_else(String::new, |f| {
				f.to_ascii_uppercase().to_string() + c.as_str()
			})
		})
		.collect::<Vec<_>>()
		.join(" ")
}

fn restored(tag: &mut Vec<Entry>, to: Version) -> Option<String> {
	let key = format!("nether:{}", to.name());
	let original = tag.take(&key).and_then(|p| p.as_str().map(str::to_owned))?;
	if tag.take(&format!("{key}:named")).is_some() {
		if let Some(display) = tag.compound_mut("display") {
			display.take("Name");
		}
		if tag
			.field("display")
			.and_then(Payload::as_compound)
			.is_some_and(Vec::is_empty)
		{
			tag.take("display");
		}
	}
	Some(original)
}

fn tagged(s: &mut v1_16::slot__true::Ty, from: Version, original: &str) {
	let tag = s
		.nbtData
		.get_or_insert_with(|| Entry::new("", P::compound(vec![])));
	let Some(c) = tag.value.as_compound_mut() else {
		return;
	};
	let key = format!("nether:{}", from.name());
	c.put(&key, P::string(original.to_owned()));
	let display = c.compound_or_insert("display");
	if display.field("Name").is_none() {
		let name = serde_json::json!({"text": title(original), "italic": false}).to_string();
		display.put("Name", P::string(name));
		c.put(&format!("{key}:named"), P::byte(1));
	}
}

pub(super) fn slot(
	v: Slot,
	from: Version,
	to: Version,
	fix: impl FnOnce(&str, &mut Vec<Entry>),
) -> Slot {
	let Slot::r#true(mut s) = v else { return v };
	let original = match &mut s.nbtData {
		Some(Entry {
			value: P::compound(c),
			..
		}) if from < to => restored(c, to),
		_ => None,
	};
	if s.nbtData
		.as_ref()
		.is_some_and(|e| e.value.as_compound().is_some_and(Vec::is_empty))
		&& original.is_some()
	{
		s.nbtData = None;
	}
	let id = match original.as_deref().and_then(|name| item_id(to, name)) {
		Some(id) => id,
		None => match item(from, to, s.itemId)
			.filter(|_| s.itemId == 0 || super::direct("items", from, to, s.itemId).is_some())
		{
			Some(id) => id,
			None => {
				let name = item_name(from, s.itemId);
				let Some(id) = substitute("items", to, name)
					.filter(|_| from > to)
					.and_then(|sub| item_id(to, sub))
				else {
					return Slot::r#false;
				};
				tagged(&mut s, from, name);
				id
			}
		},
	};
	s.itemId = id;
	if let Some(Entry {
		value: P::compound(c),
		..
	}) = &mut s.nbtData
	{
		fix(item_name(to, id), c);
	}
	Slot::r#true(s)
}

fn adapt(name: &str, data: PData) -> Option<PData> {
	Some(match (name, data) {
		("dust", PData::Transition([r, g, b, scale, ..])) => PData::Dust([r, g, b, scale]),
		("dust", d @ PData::Dust(_)) | ("dust_color_transition", d @ PData::Transition(_)) => d,
		("block" | "falling_dust", d @ PData::State(_))
		| ("item", d @ PData::Item(_))
		| ("vibration", d @ PData::Vibration(_)) => d,
		("dust" | "dust_color_transition" | "block" | "falling_dust" | "item" | "vibration", _) => {
			return None
		}
		_ => PData::None,
	})
}

pub(super) fn particle(
	p: Particle,
	from: Version,
	to: Version,
	slot: impl FnOnce(Slot) -> Slot,
) -> Option<Particle> {
	let id = remap("particles", from, to, p.id)?;
	let data = match p.data {
		PData::State(s) => PData::State(state(from, to, s)),
		PData::Item(i) => PData::Item(slot(i)),
		d => d,
	};
	let name = registry::names(to, "particles")?.name(id as u32)?;
	Some(Particle {
		id,
		data: adapt(name, data)?,
	})
}

pub(super) fn particle_15(id: i32, d: v1_15_2::particleData::Ty) -> Particle {
	use v1_15_2::particleData::Ty as D;
	let data = match d {
		D::v14(c) => PData::Dust([c.red, c.green, c.blue, c.scale]),
		D::v3(s) | D::v23(s) => PData::State(s),
		D::v32(i) => PData::Item(i),
		D::default(_) => PData::None,
	};
	Particle { id, data }
}

pub(super) fn data_15(p: Particle) -> Option<v1_15_2::particleData::Ty> {
	use v1_15_2::particleData::Ty as D;
	Some(match (p.id, p.data) {
		(14, PData::Dust([red, green, blue, scale])) => D::v14(v1_15_2::particleData__v14::Ty {
			red,
			green,
			blue,
			scale,
		}),
		(3, PData::State(s)) => D::v3(s),
		(23, PData::State(s)) => D::v23(s),
		(32, PData::Item(i)) => D::v32(i),
		(id, PData::None) => D::default(id),
		_ => return None,
	})
}

pub(super) fn particle_16(id: i32, d: v1_16::particleData::Ty) -> Particle {
	use v1_16::particleData::Ty as D;
	let data = match d {
		D::v14(c) => PData::Dust([c.red, c.green, c.blue, c.scale]),
		D::v3(s) | D::v23(s) => PData::State(s),
		D::v34(i) => PData::Item(i),
		D::default(_) => PData::None,
	};
	Particle { id, data }
}

pub(super) fn data_16(p: Particle) -> Option<v1_16::particleData::Ty> {
	use v1_16::particleData::Ty as D;
	Some(match (p.id, p.data) {
		(14, PData::Dust([red, green, blue, scale])) => D::v14(v1_16::particleData__v14::Ty {
			red,
			green,
			blue,
			scale,
		}),
		(3, PData::State(s)) => D::v3(s),
		(23, PData::State(s)) => D::v23(s),
		(34, PData::Item(i)) => D::v34(i),
		(id, PData::None) => D::default(id),
		_ => return None,
	})
}

pub(super) fn particle_17(id: i32, d: v1_17::particleData::Ty) -> Particle {
	use v1_17::particleData::Ty as D;
	let data = match d {
		D::v15(c) => PData::Dust([c.red, c.green, c.blue, c.scale]),
		D::v16(c) => PData::Transition([
			c.fromRed,
			c.fromGreen,
			c.fromBlue,
			c.scale,
			c.toRed,
			c.toGreen,
			c.toBlue,
		]),
		D::v4(s) | D::v25(s) => PData::State(s),
		D::v36(i) => PData::Item(i),
		D::v37(v) => PData::Vibration(v),
		D::default(_) => PData::None,
	};
	Particle { id, data }
}

pub(super) fn data_17(p: Particle) -> Option<v1_17::particleData::Ty> {
	use v1_17::particleData::Ty as D;
	Some(match (p.id, p.data) {
		(15, PData::Dust([red, green, blue, scale])) => D::v15(v1_17::particleData__v15::Ty {
			red,
			green,
			blue,
			scale,
		}),
		(16, PData::Transition([fromRed, fromGreen, fromBlue, scale, toRed, toGreen, toBlue])) => {
			D::v16(v1_17::particleData__v16::Ty {
				fromRed,
				fromGreen,
				fromBlue,
				scale,
				toRed,
				toGreen,
				toBlue,
			})
		}
		(4, PData::State(s)) => D::v4(s),
		(25, PData::State(s)) => D::v25(s),
		(36, PData::Item(i)) => D::v36(i),
		(37, PData::Vibration(v)) => D::v37(v),
		(id, PData::None) => D::default(id),
		_ => return None,
	})
}

macro_rules! meta_codec {
	($m:ident, $decode:ident, $encode:ident, $pfrom:ident, $pto:ident) => {
		pub(super) fn $decode(items: $m::entityMetadata::Ty) -> Vec<(u8, Meta)> {
			use $m::entityMetadataItem::Ty as M;
			items
				.into_iter()
				.map(|e| {
					let value = match e.value {
						M::v0(x) => Meta::Byte(x),
						M::v1(x) => Meta::VarInt(x),
						M::v2(x) => Meta::Float(x),
						M::v3(x) => Meta::Str(x),
						M::v4(x) => Meta::Chat(x),
						M::v5(x) => Meta::OptChat(x),
						M::v6(x) => Meta::Slot(x),
						M::v7(x) => Meta::Bool(x),
						M::v8(r) => Meta::Rotation(r.pitch, r.yaw, r.roll),
						M::v9(x) => Meta::Pos(x),
						M::v10(x) => Meta::OptPos(x),
						M::v11(x) => Meta::Direction(x),
						M::v12(x) => Meta::OptUuid(x),
						M::v13(x) => Meta::OptState(x),
						M::v14(x) => Meta::Nbt(x),
						M::v15(p) => Meta::Particle($pfrom(p.particleId, p.data)),
						M::v16(v) => Meta::Villager(v.villagerType, v.villagerProfession, v.level),
						M::v17(x) => Meta::OptVarInt(x),
						M::v18(x) => Meta::Pose(x),
						M::default(t) => Meta::Other(t),
					};
					(e.key, value)
				})
				.collect()
		}

		pub(super) fn $encode(items: Vec<(u8, Meta)>) -> $m::entityMetadata::Ty {
			use $m::entityMetadataItem::Ty as M;
			items
				.into_iter()
				.filter_map(|(key, value)| {
					let (ty, value) = match value {
						Meta::Byte(x) => (0, M::v0(x)),
						Meta::VarInt(x) => (1, M::v1(x)),
						Meta::Float(x) => (2, M::v2(x)),
						Meta::Str(x) => (3, M::v3(x)),
						Meta::Chat(x) => (4, M::v4(x)),
						Meta::OptChat(x) => (5, M::v5(x)),
						Meta::Slot(x) => (6, M::v6(x)),
						Meta::Bool(x) => (7, M::v7(x)),
						Meta::Rotation(pitch, yaw, roll) => (
							8,
							M::v8($m::entityMetadataItem__v8::Ty { pitch, yaw, roll }),
						),
						Meta::Pos(x) => (9, M::v9(x)),
						Meta::OptPos(x) => (10, M::v10(x)),
						Meta::Direction(x) => (11, M::v11(x)),
						Meta::OptUuid(x) => (12, M::v12(x)),
						Meta::OptState(x) => (13, M::v13(x)),
						Meta::Nbt(x) => (14, M::v14(x)),
						Meta::Particle(p) => {
							let particleId = p.id;
							(
								15,
								M::v15($m::particle::Ty {
									particleId,
									data: $pto(p)?,
								}),
							)
						}
						Meta::Villager(villagerType, villagerProfession, level) => (
							16,
							M::v16($m::entityMetadataItem__v16::Ty {
								villagerType,
								villagerProfession,
								level,
							}),
						),
						Meta::OptVarInt(x) => (17, M::v17(x)),
						Meta::Pose(x) => (18, M::v18(x)),
						Meta::Other(t) => (t, M::default(t)),
					};
					Some($m::entityMetadata__items::Ty {
						key,
						r#type: ty,
						value,
					})
				})
				.collect()
		}
	};
}

meta_codec!(v1_15_2, decode_15, encode_15, particle_15, data_15);
meta_codec!(v1_16, decode_16, encode_16, particle_16, data_16);
meta_codec!(v1_17, decode_17, encode_17, particle_17, data_17);

pub(super) fn remap_values(
	items: Vec<(u8, Meta)>,
	from: Version,
	to: Version,
	slot: &mut impl FnMut(Slot) -> Slot,
) -> Vec<(u8, Meta)> {
	items
		.into_iter()
		.filter_map(|(key, value)| {
			let value = match value {
				Meta::Slot(s) => Meta::Slot(slot(s)),
				Meta::OptState(s) if s != 0 => Meta::OptState(state(from, to, s)),
				Meta::Particle(p) => Meta::Particle(particle(p, from, to, &mut *slot)?),
				v => v,
			};
			Some((key, value))
		})
		.collect()
}

pub(super) fn minecart_block(
	items: &mut [(u8, Meta)],
	kind: &str,
	key: u8,
	from: Version,
	to: Version,
) {
	if !is_minecart(kind) {
		return;
	}
	for (k, v) in items.iter_mut() {
		if let (true, Meta::VarInt(s)) = (*k == key, &mut *v) {
			*s = state(from, to, *s);
		}
	}
}

pub(super) fn substituted(items: &mut Vec<(u8, Meta)>, kind: super::Kind) {
	if !kind.substitute || matches!(kind.original, "glow_item_frame" | "piglin_brute") {
		return;
	}
	if kind.original == "strider" {
		for (k, m) in items.iter_mut() {
			if let (15, Meta::Bool(baby)) = (*k, &*m) {
				*m = Meta::VarInt(if *baby { 1 } else { 3 });
			}
		}
	}
	let last = if matches!(kind.original, "strider" | "piglin") {
		15
	} else {
		14
	};
	items.retain(|(k, _)| *k <= last);
}

pub(super) fn legacy_texts(items: &mut [(u8, Meta)]) {
	for (_, m) in items.iter_mut() {
		match m {
			Meta::Chat(t) | Meta::OptChat(Some(t)) => *t = super::text::legacy(t),
			_ => {}
		}
	}
}

pub(super) fn is_arrow(kind: &str) -> bool {
	matches!(kind, "arrow" | "spectral_arrow" | "trident")
}

pub(super) fn is_minecart(kind: &str) -> bool {
	matches!(
		kind,
		"minecart"
			| "chest_minecart"
			| "command_block_minecart"
			| "furnace_minecart"
			| "hopper_minecart"
			| "spawner_minecart"
			| "tnt_minecart"
	)
}
