use super::items;
use crate::gen::convert::{v1_12, v1_8, v1_9, Version};
use crate::session::Kind;

pub(super) type M9 = v1_9::entityMetadataItem::Ty;
pub(super) type E9 = v1_9::entityMetadata__items::Ty;
type M8 = v1_8::entityMetadataItem::Ty;
type E8 = v1_8::entityMetadata__items::Ty;
type M12 = v1_12::entityMetadataItem::Ty;
pub(super) type E12 = v1_12::entityMetadata__items::Ty;

const AGEABLE: &[u8] = &[90, 91, 92, 93, 95, 96, 98, 100, 101, 102, 120];

fn tag9(v: &M9) -> i8 {
	match v {
		M9::v0(_) => 0,
		M9::v1(_) => 1,
		M9::v2(_) => 2,
		M9::v3(_) => 3,
		M9::v4(_) => 4,
		M9::v5(_) => 5,
		M9::v6(_) => 6,
		M9::v7(_) => 7,
		M9::v8(_) => 8,
		M9::v9(_) => 9,
		M9::v10(_) => 10,
		M9::v11(_) => 11,
		M9::v12(_) => 12,
		M9::default(t) => *t,
	}
}

pub(super) fn e9(key: u8, value: M9) -> E9 {
	E9 {
		key,
		r#type: tag9(&value),
		value,
	}
}

fn e8(key: u8, value: M8) -> E8 {
	let r#type = match &value {
		M8::v0(_) => 0,
		M8::v1(_) => 1,
		M8::v2(_) => 2,
		M8::v3(_) => 3,
		M8::v4(_) => 4,
		M8::v5(_) => 5,
		M8::v6(_) => 6,
		M8::v7(_) => 7,
		M8::default(t) => *t,
	};
	E8 {
		ANON: v1_8::entityMetadata__items__ANON::Ty { r#type, key },
		value,
	}
}

pub(super) fn mob_1_8(ty: u32) -> u32 {
	match ty {
		69 => 62,
		t => t,
	}
}

fn byte(b: bool) -> M8 {
	M8::v0(b as i8)
}

fn uuid_string(u: Option<u128>) -> String {
	u.map(|u| {
		let h = format!("{u:032x}");
		format!(
			"{}-{}-{}-{}-{}",
			&h[..8],
			&h[8..12],
			&h[12..16],
			&h[16..20],
			&h[20..]
		)
	})
	.unwrap_or_default()
}

fn key_1_8(kind: Kind, key: u8, value: M9) -> Option<(u8, M8)> {
	use M9 as N;
	let living = matches!(kind, Kind::Player | Kind::Mob(_) | Kind::Object(78));
	let mob = |t: &[u8]| matches!(kind, Kind::Mob(m) if t.contains(&m));
	let object = |t: u8| kind == Kind::Object(t);
	let stand = matches!(kind, Kind::Object(78) | Kind::Mob(30));
	Some(match (key, value) {
		(0, N::v0(b)) => (0, M8::v0(b)),
		(1, N::v1(n)) => (1, M8::v1(n.clamp(i16::MIN as i32, i16::MAX as i32) as i16)),
		(2, N::v3(s)) => (2, M8::v4(s)),
		(3 | 4, N::v6(b)) => (key, byte(b)),
		(6, N::v2(f)) if living => (6, M8::v3(f)),
		(7, N::v1(n)) if living => (7, M8::v2(n)),
		(8, N::v6(b)) if living => (8, byte(b)),
		(9, N::v1(n)) if living => (9, M8::v0(n as i8)),
		(10, N::v2(f)) if kind == Kind::Player => (17, M8::v3(f)),
		(11, N::v1(n)) if kind == Kind::Player => (18, M8::v2(n)),
		(12, N::v0(b)) if kind == Kind::Player => (10, M8::v0(b)),
		(10, N::v0(b)) if stand => (10, M8::v0(b)),
		(11..=16, N::v7(r)) if stand => (
			key,
			M8::v7(v1_8::entityMetadataItem__v7::Ty {
				pitch: r.pitch,
				yaw: r.yaw,
				roll: r.roll,
			}),
		),
		(10, N::v0(b)) if matches!(kind, Kind::Mob(_)) => (15, M8::v0(b)),
		(11, N::v6(b)) if mob(AGEABLE) => (12, M8::v0(-(b as i8))),
		(12, N::v0(b)) if mob(&[100]) => (16, M8::v2(b as u8 as i32)),
		(13, N::v1(n)) if mob(&[100]) => (19, M8::v0(n as i8)),
		(14, N::v1(n)) if mob(&[100]) => (20, M8::v2(n)),
		(15, N::v11(u)) if mob(&[100]) => (21, M8::v4(uuid_string(u))),
		(16, N::v1(n)) if mob(&[100]) => (22, M8::v2(n)),
		(12, N::v6(b)) if mob(&[90]) => (16, byte(b)),
		(12, N::v1(n)) if mob(&[101]) => (18, M8::v0(n as i8)),
		(12, N::v0(b)) if mob(&[91]) => (16, M8::v0(b)),
		(12, N::v1(n)) if mob(&[120]) => (16, M8::v2(n)),
		(12, N::v0(b)) if mob(&[95, 98]) => (16, M8::v0(b)),
		(13, N::v11(u)) if mob(&[95, 98]) => (17, M8::v4(uuid_string(u))),
		(14, N::v1(n)) if mob(&[98]) => (18, M8::v0(n as i8)),
		(14, N::v2(f)) if mob(&[95]) => (18, M8::v3(f)),
		(15, N::v6(b)) if mob(&[95]) => (19, byte(b)),
		(16, N::v1(n)) if mob(&[95]) => (20, M8::v0(n as i8)),
		(11, N::v0(b)) if mob(&[65, 61, 52, 59, 99]) => (16, M8::v0(b)),
		(11, N::v12(s)) if mob(&[58]) => (16, M8::v1(items::state_back(Version::V1_8, s) as i16)),
		(12, N::v6(b)) if mob(&[58]) => (18, byte(b)),
		(11, N::v6(b)) if mob(&[54, 57]) => (12, byte(b)),
		(12, N::v1(n)) if mob(&[54, 57]) => (13, byte((1..=5).contains(&n))),
		(13, N::v6(b)) if mob(&[54, 57]) => (14, byte(b)),
		(11, N::v1(n)) if mob(&[50]) => (16, M8::v0(n as i8)),
		(12 | 13, N::v6(b)) if mob(&[50]) => (key + 5, byte(b)),
		(11, N::v6(b)) if mob(&[56]) => (16, byte(b)),
		(11, N::v1(n)) if mob(&[55, 62]) => (16, M8::v0(n as i8)),
		(11, N::v1(n)) if mob(&[51]) => (13, M8::v0((n == 1) as i8)),
		(11, N::v6(b)) if mob(&[66]) => (21, byte(b)),
		(11..=14, N::v1(n)) if mob(&[64]) => (key + 6, M8::v2(n)),
		(11, N::v0(b)) if mob(&[68]) => (16, M8::v2(b as u8 as i32)),
		(12, N::v1(n)) if mob(&[68]) => (17, M8::v2(n)),
		(5, N::v5(s)) if object(2) => (
			10,
			M8::v5(items::down(Version::V1_8, Version::V1_9, s, mob_1_8)),
		),
		(5, N::v0(b)) if object(60) => (16, M8::v0(b)),
		(5, N::v5(s)) if object(76) => (
			8,
			M8::v5(items::down(Version::V1_8, Version::V1_9, s, mob_1_8)),
		),
		(5, N::v5(s)) if object(71) => (
			8,
			M8::v5(items::down(Version::V1_8, Version::V1_9, s, mob_1_8)),
		),
		(6, N::v1(n)) if object(71) => (9, M8::v0(n as i8)),
		(5 | 6, N::v1(n)) if object(1) => (key + 12, M8::v2(n)),
		(7, N::v2(f)) if object(1) => (19, M8::v3(f)),
		(5 | 6, N::v1(n)) if object(10) => (key + 12, M8::v2(n)),
		(7, N::v2(f)) if object(10) => (19, M8::v3(f)),
		(8, N::v1(n)) if object(10) => (20, M8::v2(items::state_back(Version::V1_8, n))),
		(9, N::v1(n)) if object(10) => (21, M8::v2(n)),
		(10, N::v6(b)) if object(10) => (22, byte(b)),
		(11, N::v3(s)) if object(10) => (23, M8::v4(s)),
		(11, N::v6(b)) if object(10) => (16, byte(b)),
		(12, N::v4(s)) if object(10) => (24, M8::v4(s)),
		(5, N::v6(b)) if object(66) => (10, byte(b)),
		_ => return None,
	})
}

#[derive(Default, Clone, Copy)]
pub(super) struct Status {
	flags: i8,
	using: bool,
}

pub(super) fn down_1_8(kind: Kind, items: Vec<E9>, status: &mut Status) -> Vec<E8> {
	let mut out = Vec::with_capacity(items.len());
	let player = kind == Kind::Player;
	let mut flags = None;
	for e in items {
		match (e.key, e.value) {
			(0, M9::v0(b)) => {
				status.flags = b;
				flags = Some(b);
			}
			(5, M9::v0(hand)) if player => {
				status.using = hand & 1 != 0;
				flags = Some(status.flags);
			}
			(key, value) => {
				if let Some((key, value)) = key_1_8(kind, key, value) {
					out.push(e8(key, value));
				}
			}
		}
	}
	if let Some(f) = flags {
		let f = if player && status.using {
			f | 0x10
		} else {
			f & !0x10
		};
		out.insert(0, e8(0, M8::v0(f)));
	}
	out
}

pub(super) fn fix_1_10(kind: Kind, items: &mut Vec<E9>) {
	for e in items.iter_mut() {
		match (kind, e.key, &e.value) {
			(Kind::Mob(102), 13, M9::v6(standing)) => {
				*e = e9(13, M9::v0(if *standing { 14 } else { 0 }))
			}
			(Kind::Mob(54 | 57), 13, M9::v1(6)) => *e = e9(13, M9::v1(0)),
			(Kind::Mob(51), 12, M9::v1(2)) => *e = e9(12, M9::v1(0)),
			_ => {}
		}
	}
}

pub(super) fn mob_1_10(ty: u32) -> (u32, Option<(u8, i32)>) {
	match ty {
		4 => (68, None),
		5 => (51, Some((12, 1))),
		6 => (51, Some((12, 2))),
		23 => (54, Some((13, 6))),
		27 => (54, Some((13, 1))),
		100 => (100, Some((14, 0))),
		31 => (100, Some((14, 1))),
		32 => (100, Some((14, 2))),
		29 => (100, Some((14, 3))),
		28 => (100, Some((14, 4))),
		34 => (120, None),
		35 => (65, None),
		36 => (120, Some((13, 4))),
		103 => (100, Some((14, 1))),
		t => (t, None),
	}
}

#[derive(Default, Clone, Copy)]
pub(super) struct Horse {
	chested: bool,
}

pub(super) fn fix_1_11(ty: u32, items: Vec<E9>, horse: &mut Horse) -> Vec<E9> {
	let horses = [100, 31, 32, 28, 29, 103];
	let mut out = Vec::with_capacity(items.len());
	for e in items {
		let (key, value) = (e.key, e.value);
		let mapped = match (ty, key, value) {
			(68 | 4, 12, M9::v6(b)) => Some((
				12,
				M9::v0(if b { 2 } else { 0 } | if ty == 4 { 4 } else { 0 }),
			)),
			(51 | 5 | 6, 12, v) => Some((13, v)),
			(54 | 23 | 27 | 57, 13, _) => None,
			(54 | 23 | 27 | 57, 14, v) => Some((15, v)),
			(54 | 23 | 27 | 57, 15, v) => Some((14, v)),
			(54 | 23 | 27 | 57, 16, M9::v1(n)) => Some((13, M9::v1(n + 1))),
			(34, 12, M9::v0(b)) => Some((13, M9::v1(b as i32))),
			(35, 12, _) => Some((12, M9::v0(0))),
			(36, 12, M9::v0(b)) => Some((13, M9::v1(if b == 1 { 2 } else { 4 }))),
			(t, 13, M9::v0(b)) if horses.contains(&t) => {
				Some((13, M9::v0(if horse.chested { b | 8 } else { b })))
			}
			(t, 14, v) if horses.contains(&t) => Some((16, v)),
			(100, 16, v) => Some((17, v)),
			(31 | 32 | 103, 15, M9::v6(b)) => {
				horse.chested = b;
				None
			}
			(103, 16..=18, _) => None,
			(120, 13, M9::v1(5)) => Some((13, M9::v1(0))),
			(69, 15, _) => None,
			(_, key, v) => Some((key, v)),
		};
		if let Some((key, value)) = mapped {
			out.push(e9(key, value));
		}
	}
	out
}

pub(super) fn mob_1_11(ty: u32) -> (u32, Option<(u8, i32)>) {
	match ty {
		105 => (65, Some((12, 0))),
		37 => (34, None),
		t => (t, None),
	}
}

fn meta_9(v: M12) -> Option<M9> {
	Some(match v {
		M12::v0(n) => M9::v0(n),
		M12::v1(n) => M9::v1(n),
		M12::v2(f) => M9::v2(f),
		M12::v3(s) => M9::v3(s),
		M12::v4(s) => M9::v4(s),
		M12::v5(s) => M9::v5(s),
		M12::v6(b) => M9::v6(b),
		M12::v7(r) => M9::v7(v1_9::entityMetadataItem__v7::Ty {
			pitch: r.pitch,
			yaw: r.yaw,
			roll: r.roll,
		}),
		M12::v8(p) => M9::v8(p),
		M12::v9(p) => M9::v9(p),
		M12::v10(n) => M9::v10(n),
		M12::v11(u) => M9::v11(u),
		M12::v12(n) => M9::v12(n),
		M12::v13(_) | M12::default(_) => return None,
	})
}

pub(super) fn fix_1_12(ty: Option<u32>, items: Vec<E12>) -> Vec<E9> {
	let mut out = Vec::with_capacity(items.len());
	for e in items {
		let Some(value) = meta_9(e.value) else {
			continue;
		};
		let mapped = match (ty, e.key, value) {
			(Some(34 | 37), 12, _) => None,
			(Some(34 | 37), k, v) if k > 12 => Some((k - 1, v)),
			(Some(37), 0, M9::v0(b)) => Some((0, M9::v0(b & !0x20))),
			(Some(105), 12 | 14 | 15, _) => None,
			(Some(105), 13, M9::v0(b)) => Some((12, M9::v0(b & 1))),
			(_, k, v) => Some((k, v)),
		};
		if let Some((key, value)) = mapped {
			out.push(e9(key, value));
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	fn keys(v: &[E8]) -> Vec<(u8, M8)> {
		v.iter().map(|e| (e.ANON.key, e.value.clone())).collect()
	}

	#[test]
	fn mobs_and_players_map_to_1_8_indices() {
		let mut st = Status::default();
		let cow = down_1_8(
			Kind::Mob(92),
			vec![
				e9(0, M9::v0(0)),
				e9(2, M9::v3("Bessie".into())),
				e9(6, M9::v2(10.0)),
				e9(10, M9::v0(1)),
				e9(11, M9::v6(true)),
				e9(12, M9::v1(3)),
			],
			&mut st,
		);
		assert_eq!(
			keys(&cow),
			vec![
				(0, M8::v0(0)),
				(2, M8::v4("Bessie".into())),
				(6, M8::v3(10.0)),
				(15, M8::v0(1)),
				(12, M8::v0(-1))
			]
		);
		let mut st = Status::default();
		let eating = down_1_8(
			Kind::Player,
			vec![
				e9(0, M9::v0(2)),
				e9(5, M9::v0(1)),
				e9(12, M9::v0(0x7f)),
				e9(10, M9::v2(4.0)),
			],
			&mut st,
		);
		assert_eq!(
			keys(&eating),
			vec![(0, M8::v0(0x12)), (10, M8::v0(0x7f)), (17, M8::v3(4.0))]
		);
		let wrong = down_1_8(
			Kind::Mob(51),
			vec![e9(11, M9::v6(true))],
			&mut Status::default(),
		);
		assert!(wrong.is_empty());
	}

	#[test]
	fn armor_stands_spawned_as_mobs_keep_the_armor_stand_layout() {
		let rot = |pitch| {
			M9::v7(v1_9::entityMetadataItem__v7::Ty {
				pitch,
				yaw: 0.0,
				roll: 0.0,
			})
		};
		let stand = down_1_8(
			Kind::Mob(30),
			vec![
				e9(0, M9::v0(0x20)),
				e9(10, M9::v0(0x19)),
				e9(11, rot(10.0)),
				e9(15, rot(20.0)),
			],
			&mut Status::default(),
		);
		let keys: Vec<(u8, u8)> = stand.iter().map(|e| (e.ANON.key, e.ANON.r#type)).collect();
		assert_eq!(keys, vec![(0, 0), (10, 0), (11, 7), (15, 7)]);
	}

	#[test]
	fn new_mobs_become_old_mobs() {
		assert_eq!(mob_1_10(5), (51, Some((12, 1))));
		let mut horse = Horse::default();
		let donkey = fix_1_11(
			31,
			vec![
				e9(15, M9::v6(true)),
				e9(13, M9::v0(4)),
				e9(14, M9::v11(None)),
			],
			&mut horse,
		);
		assert_eq!(
			donkey
				.iter()
				.map(|e| (e.key, e.value.clone()))
				.collect::<Vec<_>>(),
			vec![(13, M9::v0(12)), (16, M9::v11(None))]
		);
	}
}
