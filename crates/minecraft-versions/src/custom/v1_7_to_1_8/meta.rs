use super::item;
use crate::gen::convert::{v1_7, v1_8};
use crate::pds::convert::Result;
use crate::session::Kind;

type M7 = v1_7::entityMetadataItem::Ty;
type M8 = v1_8::entityMetadataItem::Ty;
type E7 = v1_7::entityMetadata__items::Ty;
type E8 = v1_8::entityMetadata__items::Ty;

const AGEABLE: &[u8] = &[90, 91, 92, 93, 95, 96, 98, 100, 101, 120];

fn mob_keys(ty: u8) -> &'static [u8] {
	match ty {
		50 => &[16, 17, 18],
		51 => &[13],
		52 | 59 | 61 | 55 | 62 | 56 | 65 | 90 | 91 | 99 => &[16],
		54 | 57 => &[12, 13, 14],
		58 => &[17, 18],
		64 => &[17, 18, 19, 20],
		66 => &[21],
		95 => &[16, 17, 18, 19, 20],
		98 => &[16, 17, 18],
		100 => &[16, 19, 20, 21, 22],
		120 => &[16],
		_ => &[],
	}
}

fn object_keys(ty: u8) -> &'static [u8] {
	match ty {
		1 => &[17, 18, 19],
		10 => &[16, 17, 18, 19, 21, 22, 23, 24],
		51 => &[8],
		60 => &[16],
		66 => &[10],
		_ => &[],
	}
}

fn same_down(v: M8) -> Result<Option<M7>> {
	Ok(Some(match v {
		M8::v0(n) => M7::v0(n),
		M8::v1(n) => M7::v1(n),
		M8::v2(n) => M7::v2(n),
		M8::v3(n) => M7::v3(n),
		M8::v4(s) => M7::v4(s),
		M8::v5(s) => M7::v5(item::down(s)?),
		M8::v6(p) => M7::v6(v1_7::entityMetadataItem__v6::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
		}),
		M8::v7(_) | M8::default(_) => return Ok(None),
	}))
}

fn same_up(v: M7) -> Result<Option<M8>> {
	Ok(Some(match v {
		M7::v0(n) => M8::v0(n),
		M7::v1(n) => M8::v1(n),
		M7::v2(n) => M8::v2(n),
		M7::v3(n) => M8::v3(n),
		M7::v4(s) => M8::v4(s),
		M7::v5(s) => M8::v5(item::up(s)?),
		M7::v6(p) => M8::v6(v1_8::entityMetadataItem__v6::Ty {
			x: p.x,
			y: p.y,
			z: p.z,
		}),
		M7::v7(_) | M7::default(_) => return Ok(None),
	}))
}

fn int8(v: &M8) -> Option<i32> {
	match v {
		M8::v0(n) => Some(*n as i32),
		M8::v1(n) => Some(*n as i32),
		M8::v2(n) => Some(*n),
		_ => None,
	}
}

fn int7(v: &M7) -> Option<i32> {
	match v {
		M7::v0(n) => Some(*n as i32),
		M7::v1(n) => Some(*n as i32),
		M7::v2(n) => Some(*n),
		_ => None,
	}
}

fn block_data_1_7(state: i32) -> i32 {
	let s = item::state_1_7(((state & 0xfff) << 4 | (state >> 12 & 15)) as u16) as i32;
	s >> 4 | (s & 15) << 16
}

fn block_data_1_8(data: i32) -> i32 {
	(data & 0xffff) | (data >> 16 & 15) << 12
}

fn key_down(kind: Kind, key: u8, v: M8) -> Result<Option<(u8, M7)>> {
	let keep = |key: u8, v: M8| -> Result<Option<(u8, M7)>> { Ok(same_down(v)?.map(|m| (key, m))) };
	match (kind, key) {
		(_, 0 | 1) => keep(key, v),
		(Kind::Player | Kind::Mob(_), 6..=9) => keep(key, v),
		(Kind::Player, 10) => Ok(int8(&v).map(|f| (16, M7::v0(if f & 1 != 0 { 0 } else { 2 })))),
		(Kind::Player, 17 | 18) => keep(key, v),
		(Kind::Mob(_), 2) => keep(10, v),
		(Kind::Mob(_), 3) => keep(11, v),
		(Kind::Mob(t), 12) if AGEABLE.contains(&t) => Ok(int8(&v).map(|n| (12, M7::v2(n)))),
		(Kind::Mob(58), 16) => {
			Ok(int8(&v).map(|id| (16, M7::v0(item::block_1_7(id as u16, 0).0 as i8))))
		}
		(Kind::Mob(t), k) if mob_keys(t).contains(&k) => keep(k, v),
		(Kind::Object(71), 8) => keep(2, v),
		(Kind::Object(71), 9) => Ok(int8(&v).map(|r| (3, M7::v0((r.rem_euclid(8) / 2) as i8)))),
		(Kind::Object(2), 10) | (Kind::Object(76), 8) => keep(key, v),
		(Kind::Object(10), 20) => Ok(int8(&v).map(|s| (20, M7::v2(block_data_1_7(s))))),
		(Kind::Object(t), k) if object_keys(t).contains(&k) => keep(k, v),
		_ => Ok(None),
	}
}

fn key_up(kind: Kind, key: u8, v: M7) -> Result<Option<(u8, M8)>> {
	let keep = |key: u8, v: M7| -> Result<Option<(u8, M8)>> { Ok(same_up(v)?.map(|m| (key, m))) };
	match (kind, key) {
		(_, 0 | 1) => keep(key, v),
		(Kind::Player | Kind::Mob(_), 6..=9) => keep(key, v),
		(Kind::Player, 16) => {
			Ok(int7(&v).map(|f| (10, M8::v0(if f & 2 != 0 { 0x7e } else { 0x7f }))))
		}
		(Kind::Player, 17 | 18) => keep(key, v),
		(Kind::Mob(_), 10) => keep(2, v),
		(Kind::Mob(_), 11) => keep(3, v),
		(Kind::Mob(t), 12) if AGEABLE.contains(&t) => {
			Ok(int7(&v).map(|n| (12, M8::v0(n.signum() as i8))))
		}
		(Kind::Mob(58), 16) => Ok(int7(&v).map(|id| (16, M8::v1(id as u8 as i16)))),
		(Kind::Mob(t), k) if mob_keys(t).contains(&k) => keep(k, v),
		(Kind::Object(71), 2) => keep(8, v),
		(Kind::Object(71), 3) => Ok(int7(&v).map(|r| (9, M8::v0((r.rem_euclid(4) * 2) as i8)))),
		(Kind::Object(2), 10) | (Kind::Object(76), 8) => keep(key, v),
		(Kind::Object(10), 20) => Ok(int7(&v).map(|d| (20, M8::v2(block_data_1_8(d))))),
		(Kind::Object(t), k) if object_keys(t).contains(&k) => keep(k, v),
		_ => Ok(None),
	}
}

fn tag7(v: &M7) -> u8 {
	match v {
		M7::v0(_) => 0,
		M7::v1(_) => 1,
		M7::v2(_) => 2,
		M7::v3(_) => 3,
		M7::v4(_) => 4,
		M7::v5(_) => 5,
		M7::v6(_) => 6,
		M7::v7(_) => 7,
		M7::default(t) => *t,
	}
}

fn tag8(v: &M8) -> u8 {
	match v {
		M8::v0(_) => 0,
		M8::v1(_) => 1,
		M8::v2(_) => 2,
		M8::v3(_) => 3,
		M8::v4(_) => 4,
		M8::v5(_) => 5,
		M8::v6(_) => 6,
		M8::v7(_) => 7,
		M8::default(t) => *t,
	}
}

pub(super) fn down(kind: Kind, items: Vec<E8>) -> Result<Vec<E7>> {
	let mut out = Vec::with_capacity(items.len());
	for e in items {
		if let Some((key, value)) = key_down(kind, e.ANON.key, e.value)? {
			out.push(E7 {
				ANON: v1_7::entityMetadata__items__ANON::Ty {
					r#type: tag7(&value),
					key,
				},
				value,
			});
		}
	}
	Ok(out)
}

pub(super) fn up(kind: Kind, items: Vec<E7>) -> Result<Vec<E8>> {
	let mut out = Vec::with_capacity(items.len());
	for e in items {
		if let Some((key, value)) = key_up(kind, e.ANON.key, e.value)? {
			out.push(E8 {
				ANON: v1_8::entityMetadata__items__ANON::Ty {
					r#type: tag8(&value),
					key,
				},
				value,
			});
		}
	}
	Ok(out)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn e8(key: u8, value: M8) -> E8 {
		E8 {
			ANON: v1_8::entityMetadata__items__ANON::Ty {
				r#type: tag8(&value),
				key,
			},
			value,
		}
	}

	fn pairs(v: &[E7]) -> Vec<(u8, u8, M7)> {
		v.iter()
			.map(|e| (e.ANON.key, e.ANON.r#type, e.value.clone()))
			.collect()
	}

	#[test]
	fn mob_names_and_ages_move() {
		let out = down(
			Kind::Mob(92),
			vec![
				e8(0, M8::v0(0)),
				e8(2, M8::v4("Bessie".into())),
				e8(3, M8::v0(1)),
				e8(4, M8::v0(1)),
				e8(12, M8::v0(-1)),
				e8(15, M8::v0(0)),
			],
		)
		.unwrap();
		assert_eq!(
			pairs(&out),
			vec![
				(0, 0, M7::v0(0)),
				(10, 4, M7::v4("Bessie".into())),
				(11, 0, M7::v0(1)),
				(12, 2, M7::v2(-1))
			]
		);
		let back = up(Kind::Mob(92), out).unwrap();
		assert_eq!(
			back.iter().map(|e| e.ANON.key).collect::<Vec<_>>(),
			vec![0, 2, 3, 12]
		);
		assert_eq!(back[3].value, M8::v0(-1));
	}

	#[test]
	fn player_skin_and_objects() {
		let out = down(
			Kind::Player,
			vec![
				e8(10, M8::v0(0x7f)),
				e8(16, M8::v0(0)),
				e8(2, M8::v4("x".into())),
				e8(17, M8::v3(2.0)),
			],
		)
		.unwrap();
		assert_eq!(pairs(&out), vec![(16, 0, M7::v0(0)), (17, 3, M7::v3(2.0))]);
		let frame = down(
			Kind::Object(71),
			vec![e8(8, M8::v5(v1_8::slot::Ty::_1)), e8(9, M8::v0(6))],
		)
		.unwrap();
		assert_eq!(
			pairs(&frame),
			vec![(2, 5, M7::v5(v1_7::slot::Ty::_1)), (3, 0, M7::v0(3))]
		);
		let cart = down(Kind::Object(10), vec![e8(20, M8::v2(54 | 2 << 12))]).unwrap();
		assert_eq!(pairs(&cart), vec![(20, 2, M7::v2(54 | 2 << 16))]);
	}
}
