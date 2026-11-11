use crate::gen::convert::Version;
use crate::pds::convert::{Error, Result};
use crate::pds::{Input, Output};
use crate::registry;

pub fn encoding(e: crate::pds::Error) -> Error {
	Error::Encoding(format!("{e:?}"))
}

pub fn encode<T>(
	v: &T,
	ser: impl Fn(&T, &mut Output) -> crate::pds::Result<()>,
) -> Result<Vec<u8>> {
	let mut o = Output::new();
	ser(v, &mut o).map_err(encoding)?;
	Ok(o.into_inner())
}

pub fn decode<T>(b: &[u8], de: impl Fn(&mut Input) -> crate::pds::Result<T>) -> Result<T> {
	let mut input = Input::new(b);
	let v = de(&mut input).map_err(encoding)?;
	match input.remaining() {
		0 => Ok(v),
		n => Err(Error::Encoding(format!("{n} bytes left over"))),
	}
}

pub fn recode<A, B>(
	a: &A,
	ser: impl Fn(&A, &mut Output) -> crate::pds::Result<()>,
	de: impl Fn(&mut Input) -> crate::pds::Result<B>,
) -> Result<B> {
	decode(&encode(a, ser)?, de)
}

pub fn remap(kind: &str, from: Version, to: Version, id: i32) -> Option<i32> {
	registry::remap_name(kind, from, to, u32::try_from(id).ok()?)
		.and_then(|v| i32::try_from(v).ok())
}

pub fn read_varint(data: &[u8], at: &mut usize) -> Result<i32> {
	let mut v: u32 = 0;
	for shift in (0..35).step_by(7) {
		let b = *data
			.get(*at)
			.ok_or_else(|| Error::Encoding("truncated varint".into()))?;
		*at += 1;
		v |= ((b & 0x7f) as u32) << shift;
		if b & 0x80 == 0 {
			return Ok(v as i32);
		}
	}
	Err(Error::Encoding("varint is too long".into()))
}

pub fn write_varint(out: &mut Vec<u8>, v: i32) {
	let mut v = v as u32;
	loop {
		if v & !0x7f == 0 {
			out.push(v as u8);
			return;
		}
		out.push((v as u8 & 0x7f) | 0x80);
		v >>= 7;
	}
}

pub fn bits_for(n: u32) -> u8 {
	(32 - n.saturating_sub(1).leading_zeros()) as u8
}

const LP_MAX: f64 = 1.7179869183e10;
const LP_MIN: f64 = 3.051944088384301e-5;

fn lp_pack(v: f64) -> u64 {
	((v * 0.5 + 0.5) * 32766.0).round() as u64
}

fn lp_unpack(v: u64) -> f64 {
	((v & 32767) as f64).min(32766.0) * 2.0 / 32766.0 - 1.0
}

pub fn lp_vec3(x: f64, y: f64, z: f64) -> Vec<u8> {
	let clean = |v: f64| {
		if v.is_nan() {
			0.0
		} else {
			v.clamp(-LP_MAX, LP_MAX)
		}
	};
	let (x, y, z) = (clean(x), clean(y), clean(z));
	let max = x.abs().max(y.abs()).max(z.abs());
	if max < LP_MIN {
		return vec![0];
	}
	let scale = max.ceil() as u64;
	let more = scale & 3 != scale;
	let head = if more { scale & 3 | 4 } else { scale };
	let s = scale as f64;
	let q = head | lp_pack(x / s) << 3 | lp_pack(y / s) << 18 | lp_pack(z / s) << 33;
	let mut out = vec![q as u8, (q >> 8) as u8];
	out.extend_from_slice(&((q >> 16) as u32).to_be_bytes());
	if more {
		write_varint(&mut out, (scale >> 2) as i32);
	}
	out
}

pub fn read_lp_vec3(data: &[u8]) -> Result<(f64, f64, f64)> {
	let Some(&first) = data.first() else {
		return Err(Error::Encoding("empty lpVec3".into()));
	};
	if first == 0 {
		return Ok((0.0, 0.0, 0.0));
	}
	let raw: [u8; 6] = data
		.get(..6)
		.and_then(|b| b.try_into().ok())
		.ok_or_else(|| Error::Encoding("short lpVec3".into()))?;
	let q = (u32::from_be_bytes(raw[2..6].try_into().expect("4 bytes")) as u64) << 16
		| (raw[1] as u64) << 8
		| raw[0] as u64;
	let mut scale = (first & 3) as u64;
	if first & 4 != 0 {
		let mut at = 6;
		scale |= (read_varint(data, &mut at)? as u32 as u64) << 2;
	}
	let s = scale as f64;
	Ok((
		lp_unpack(q >> 3) * s,
		lp_unpack(q >> 18) * s,
		lp_unpack(q >> 33) * s,
	))
}
