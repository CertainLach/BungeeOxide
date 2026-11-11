use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
	Byte,
	Int,
	Long,
	Float,
	Str,
	Chat,
	OptChat,
	Item,
	Bool,
	Rotation,
	Pos,
	OptPos,
	Direction,
	OptUuid,
	BlockState,
	OptBlockState,
	Nbt,
	Particle,
	Villager,
	OptVarInt,
	Pose,
	Cat,
	Frog,
	OptGlobalPos,
	Painting,
	Sniffer,
	Vector3,
	Quaternion,
}

use Kind::*;

const KINDS_1_19: &[Kind] = &[
	Byte,
	Int,
	Float,
	Str,
	Chat,
	OptChat,
	Item,
	Bool,
	Rotation,
	Pos,
	OptPos,
	Direction,
	OptUuid,
	OptBlockState,
	Nbt,
	Particle,
	Villager,
	OptVarInt,
	Pose,
	Cat,
	Frog,
	OptGlobalPos,
	Painting,
];

const KINDS_1_19_3: &[Kind] = &[
	Byte,
	Int,
	Long,
	Float,
	Str,
	Chat,
	OptChat,
	Item,
	Bool,
	Rotation,
	Pos,
	OptPos,
	Direction,
	OptUuid,
	OptBlockState,
	Nbt,
	Particle,
	Villager,
	OptVarInt,
	Pose,
	Cat,
	Frog,
	OptGlobalPos,
	Painting,
];

const KINDS_1_19_4: &[Kind] = &[
	Byte,
	Int,
	Long,
	Float,
	Str,
	Chat,
	OptChat,
	Item,
	Bool,
	Rotation,
	Pos,
	OptPos,
	Direction,
	OptUuid,
	BlockState,
	OptBlockState,
	Nbt,
	Particle,
	Villager,
	OptVarInt,
	Pose,
	Cat,
	Frog,
	OptGlobalPos,
	Painting,
	Sniffer,
	Vector3,
	Quaternion,
];

fn kinds(version: V) -> &'static [Kind] {
	if version >= V::V1_19_4 {
		KINDS_1_19_4
	} else if version >= V::V1_19_3 {
		KINDS_1_19_3
	} else if version >= V::V1_19 {
		KINDS_1_19
	} else {
		&KINDS_1_19[..19]
	}
}

#[derive(Debug, Clone)]
pub(super) struct Meta {
	pub key: u8,
	pub kind: Kind,
	pub value: Vec<u8>,
}

impl Meta {
	pub fn varint(key: u8, kind: Kind, v: i32) -> Self {
		let mut value = Vec::new();
		write_varint(&mut value, v);
		Meta { key, kind, value }
	}

	pub fn int(&self) -> Option<i32> {
		let mut at = 0;
		read_varint(&self.value, &mut at).ok()
	}
}

fn skip_varint(data: &[u8], at: &mut usize) -> Result<()> {
	loop {
		let b = take(data, at, 1)?[0];
		if b & 0x80 == 0 {
			return Ok(());
		}
	}
}

fn skip_string(data: &[u8], at: &mut usize) -> Result<()> {
	let n = read_varint(data, at)? as usize;
	take(data, at, n).map(|_| ())
}

fn skip_with<T>(
	data: &[u8],
	at: &mut usize,
	de: fn(&mut Input) -> crate::pds::Result<T>,
) -> Result<()> {
	let input = &mut Input::new(&data[*at..]);
	de(input).map_err(encoding)?;
	*at += input.offset();
	Ok(())
}

fn skip_slot(data: &[u8], at: &mut usize) -> Result<()> {
	skip_with(data, at, v1_13_2::slot::deserialize)
}

fn particle_len(version: V, data: &[u8]) -> Result<usize> {
	let mut at = 0;
	let id = read_varint(data, &mut at)?;
	match particle_name(version, id) {
		Some("block" | "block_marker" | "falling_dust" | "shriek") => skip_varint(data, &mut at)?,
		Some("dust") => at += 16,
		Some("dust_color_transition") => at += 28,
		Some("sculk_charge") => at += 4,
		Some("item") => skip_slot(data, &mut at)?,
		Some("vibration") => {
			if vibration_has_origin(version) {
				at += 8;
			}
			let start = at;
			skip_string(data, &mut at)?;
			let kind = std::str::from_utf8(&data[start..at]).unwrap_or_default();
			match (
				kind.ends_with("minecraft:block"),
				vibration_has_origin(version),
				unpatched(version),
			) {
				(true, _, _) => at += 8,
				(false, true, _) => skip_varint(data, &mut at)?,
				(false, false, true) => {
					for _ in 0..3 {
						skip_varint(data, &mut at)?;
					}
				}
				(false, false, false) => {
					skip_varint(data, &mut at)?;
					at += 4;
				}
			}
			skip_varint(data, &mut at)?;
		}
		_ => {}
	}
	match at <= data.len() {
		true => Ok(at),
		false => unconvertible("truncated particle"),
	}
}

fn skip(version: V, kind: Kind, data: &[u8], at: &mut usize) -> Result<()> {
	let bool = |data: &[u8], at: &mut usize| -> Result<bool> { Ok(take(data, at, 1)?[0] != 0) };
	match kind {
		Byte | Bool => *at += 1,
		Int | Direction | BlockState | OptBlockState | OptVarInt | Pose | Cat | Frog | Painting
		| Sniffer | Long => skip_varint(data, at)?,
		Float => *at += 4,
		Rotation | Vector3 => *at += 12,
		Quaternion => *at += 16,
		Pos => *at += 8,
		Str | Chat => skip_string(data, at)?,
		OptChat => {
			if bool(data, at)? {
				skip_string(data, at)?
			}
		}
		Item => skip_slot(data, at)?,
		OptPos => {
			if bool(data, at)? {
				*at += 8
			}
		}
		OptUuid => {
			if bool(data, at)? {
				*at += 16
			}
		}
		Nbt => skip_with(data, at, nbt::gen::entry::deserialize)?,
		Particle => *at += particle_len(version, &data[*at..])?,
		Villager => {
			for _ in 0..3 {
				skip_varint(data, at)?;
			}
		}
		OptGlobalPos => {
			if bool(data, at)? {
				skip_string(data, at)?;
				if !unpatched(version) {
					*at += 8;
				}
			}
		}
	}
	match *at <= data.len() {
		true => Ok(()),
		false => unconvertible("truncated entity metadata"),
	}
}

pub(super) fn parse(version: V, data: &[u8]) -> Result<Vec<Meta>> {
	let table = kinds(version);
	let mut at = 0;
	let mut out = Vec::new();
	loop {
		let key = take(data, &mut at, 1)?[0];
		if key == 0xff {
			return Ok(out);
		}
		let ty = read_varint(data, &mut at)?;
		let Some(&kind) = table.get(ty as usize) else {
			return unconvertible(format!(
				"unknown entity metadata type {ty} in {}",
				version.name()
			));
		};
		let start = at;
		skip(version, kind, data, &mut at)?;
		out.push(Meta {
			key,
			kind,
			value: data[start..at].to_vec(),
		});
	}
}

pub(super) fn write(version: V, list: &[Meta]) -> Vec<u8> {
	let table = kinds(version);
	let mut out = Vec::new();
	for m in list {
		let Some(ty) = table.iter().position(|k| *k == m.kind) else {
			continue;
		};
		out.push(m.key);
		write_varint(&mut out, ty as i32);
		out.extend_from_slice(&m.value);
	}
	out.push(0xff);
	out
}

fn remap(from: V, to: V, m: Meta) -> Result<Option<Meta>> {
	let value = match m.kind {
		OptGlobalPos if unpatched(from) != unpatched(to) && m.value.first() != Some(&0) => {
			return Ok(None)
		}
		Item => {
			let v = decode(&m.value, v1_13_2::slot::deserialize)?;
			encode(&slot(from, to, v), v1_13_2::slot::serialize)?
		}
		BlockState | OptBlockState => {
			let mut out = Vec::new();
			write_varint(
				&mut out,
				m.int()
					.map_or(0, |s| if s == 0 { 0 } else { state(from, to, s) }),
			);
			out
		}
		Particle => {
			let mut at = 0;
			let id = read_varint(&m.value, &mut at)?;
			let Some((id, data)) = particle_data(from, to, id, &m.value[at..])? else {
				return Ok(None);
			};
			let mut out = Vec::new();
			write_varint(&mut out, id);
			out.extend(data);
			out
		}
		_ => return Ok(Some(m)),
	};
	Ok(Some(Meta { value, ..m }))
}

pub(super) const MINECARTS: &[&str] = &[
	"minecart",
	"chest_minecart",
	"command_block_minecart",
	"furnace_minecart",
	"hopper_minecart",
	"spawner_minecart",
	"tnt_minecart",
];

pub(super) fn convert(
	from: V,
	to: V,
	data: &[u8],
	mut patch: impl FnMut(Meta) -> Result<Option<Meta>>,
) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	for m in parse(from, data)? {
		let Some(m) = patch(m)? else { continue };
		if let Some(m) = remap(from, to, m)? {
			out.push(m);
		}
	}
	Ok(write(to, &out))
}
