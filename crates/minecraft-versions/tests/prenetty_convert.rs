use minecraft_versions::Version;
use std::io::{Read, Write};

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use minecraft_versions::gen::convert::{convert, inspect, Root};
use minecraft_versions::Session;

struct W(Vec<u8>);

fn w(id: u8) -> W {
	W(vec![id])
}

impl W {
	fn raw(mut self, bytes: &[u8]) -> Self {
		self.0.extend_from_slice(bytes);
		self
	}
	fn u8(self, v: u8) -> Self {
		self.raw(&[v])
	}
	fn i8(self, v: i8) -> Self {
		self.raw(&v.to_be_bytes())
	}
	fn bool(self, v: bool) -> Self {
		self.u8(v as u8)
	}
	fn i16(self, v: i16) -> Self {
		self.raw(&v.to_be_bytes())
	}
	fn u16(self, v: u16) -> Self {
		self.raw(&v.to_be_bytes())
	}
	fn i32(self, v: i32) -> Self {
		self.raw(&v.to_be_bytes())
	}
	fn i64(self, v: i64) -> Self {
		self.raw(&v.to_be_bytes())
	}
	fn f32(self, v: f32) -> Self {
		self.raw(&v.to_be_bytes())
	}
	fn str(self, s: &str) -> Self {
		let units: Vec<u16> = s.encode_utf16().collect();
		let mut out = self.i16(units.len() as i16);
		for unit in units {
			out = out.u16(unit);
		}
		out
	}
}

fn run(root: Root, from: Version, to: Version, packet: &[u8], cx: &mut Session) -> Vec<Vec<u8>> {
	let out = convert(root, from, to, packet, cx).unwrap();
	for p in &out {
		inspect(root, to, p).unwrap();
	}
	out
}

fn client(from: Version, to: Version, packet: &W) -> Vec<Vec<u8>> {
	run(Root::PlayToClient, from, to, &packet.0, &mut Session::new())
}

fn names(root: Root, version: Version, packets: &[Vec<u8>]) -> Vec<String> {
	packets
		.iter()
		.map(|p| inspect(root, version, p).unwrap())
		.collect()
}

fn deflate(data: &[u8]) -> Vec<u8> {
	let mut z = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
	z.write_all(data).unwrap();
	z.finish().unwrap()
}

fn inflate(data: &[u8]) -> Vec<u8> {
	let mut out = Vec::new();
	ZlibDecoder::new(data).read_to_end(&mut out).unwrap();
	out
}

const STEPS: [Version; 14] = [
	Version::V1_0_1,
	Version::V1_1,
	Version::V1_2_3,
	Version::V1_2_5,
	Version::V1_3_2,
	Version::V1_4_2,
	Version::V1_4_5,
	Version::V1_4_7,
	Version::V1_5_1,
	Version::V1_5_2,
	Version::V1_6_1,
	Version::V1_6_2,
	Version::V1_6_4,
	Version::V1_7,
];

#[test]
fn every_pre_netty_step_is_generated() {
	use minecraft_versions::STEPS as GENERATED;
	for pair in STEPS.windows(2) {
		assert!(GENERATED[pair[0].index()], "{} -> {}", pair[0], pair[1]);
	}
}

#[test]
fn keep_alive_and_chat_survive_every_step_both_ways() {
	let mut cx = Session::new();
	for pair in STEPS.windows(2) {
		let keep_alive = w(0x00).i32(7).0;
		let up = run(Root::PlayToClient, pair[0], pair[1], &keep_alive, &mut cx);
		let down = run(Root::PlayToClient, pair[1], pair[0], &up[0], &mut cx);
		assert_eq!(down, vec![keep_alive.clone()], "{pair:?}");
		let chat = if pair[1] == Version::V1_7 {
			vec![0x01, 2, b'h', b'i']
		} else {
			w(0x03).str("hi").0
		};
		let back = run(Root::PlayToServer, pair[1], pair[0], &chat, &mut cx);
		let again = run(Root::PlayToServer, pair[0], pair[1], &back[0], &mut cx);
		assert_eq!(again, vec![chat], "{pair:?}");
	}
}

#[test]
fn login_reaches_1_0_and_1_7_from_1_6_4() {
	let login = w(0x01).i32(42).str("flat").u8(1).i8(-1).u8(2).u8(0).u8(20);
	let old = client(Version::V1_6_4, Version::V1_0_1, &login);
	assert_eq!(names(Root::PlayToClient, Version::V1_0_1, &old), ["login"]);
	let mut expect = w(0x01)
		.i32(42)
		.str("")
		.i64(0)
		.i32(1)
		.i8(-1)
		.u8(2)
		.u8(0)
		.u8(20)
		.0;
	assert_eq!(old[0], expect);
	let new = client(Version::V1_6_4, Version::V1_7, &login);
	assert_eq!(
		new[0],
		[
			vec![0x01],
			42i32.to_be_bytes().to_vec(),
			vec![1, 0xff, 2, 20, 4],
			b"flat".to_vec()
		]
		.concat()
	);
	let up = client(Version::V1_0_1, Version::V1_6_4, &W(old.remove_first()));
	expect = w(0x01)
		.i32(42)
		.str("default")
		.u8(1)
		.i8(-1)
		.u8(2)
		.u8(0)
		.u8(20)
		.0;
	assert_eq!(up[0], expect);
}

trait First {
	fn remove_first(self) -> Vec<u8>;
}

impl First for Vec<Vec<u8>> {
	fn remove_first(mut self) -> Vec<u8> {
		self.remove(0)
	}
}

#[test]
fn chat_becomes_json_at_1_6_and_text_again_below() {
	let up = client(
		Version::V1_5_2,
		Version::V1_6_1,
		&w(0x03).str("\u{a7}ehello"),
	);
	assert_eq!(up[0], w(0x03).str("{\"text\":\"\u{a7}ehello\"}").0);
	let down = client(
		Version::V1_6_1,
		Version::V1_5_2,
		&w(0x03).str(r#"{"translate":"chat.type.text","with":["Notch","hi"]}"#),
	);
	assert_eq!(down[0], w(0x03).str("<Notch> hi").0);
}

#[test]
fn kick_messages_are_json_in_1_7() {
	let up = client(Version::V1_6_4, Version::V1_7, &w(0xff).str("bye"));
	assert_eq!(
		up[0],
		[vec![0x40, 14], br#"{"text":"bye"}"#.to_vec()].concat()
	);
	let down = client(Version::V1_7, Version::V1_6_4, &W(up[0].clone()));
	assert_eq!(down[0], w(0xff).str("bye").0);
}

fn legacy_items() -> W {
	w(0x68)
		.u8(0)
		.i16(3)
		.i16(-1)
		.i16(276)
		.i8(1)
		.i16(5)
		.i16(-1)
		.i16(1)
		.i8(64)
		.i16(0)
}

#[test]
fn legacy_slots_round_trip_through_1_3() {
	let up = client(Version::V1_2_5, Version::V1_3_2, &legacy_items());
	assert_eq!(
		up[0],
		w(0x68)
			.u8(0)
			.i16(3)
			.i16(-1)
			.i16(276)
			.i8(1)
			.i16(5)
			.i16(-1)
			.i16(1)
			.i8(64)
			.i16(0)
			.i16(-1)
			.0
	);
	let down = client(Version::V1_3_2, Version::V1_2_5, &W(up[0].clone()));
	assert_eq!(down[0], legacy_items().0);
}

#[test]
fn new_items_fall_back_to_old_ones() {
	let horse_armor = w(0x67).i8(0).i16(36).i16(417).i8(1).i16(0).i16(-1);
	let down = client(Version::V1_6_4, Version::V1_0_1, &horse_armor);
	assert_eq!(
		down[0],
		w(0x67).i8(0).i16(36).i16(307).i8(1).i16(0).i16(-1).0
	);
	let egg = w(0x67).i8(0).i16(36).i16(383).i8(1).i16(100).i16(-1);
	assert_eq!(
		client(Version::V1_6_4, Version::V1_5_2, &egg)[0],
		w(0x67).i8(0).i16(36).i16(383).i8(1).i16(90).i16(-1).0
	);
}

#[test]
fn metadata_indices_move_at_1_6() {
	let mut cx = Session::new();
	let spawn = w(0x18)
		.i32(5)
		.u8(90)
		.i32(0)
		.i32(0)
		.i32(0)
		.i8(0)
		.i8(0)
		.i8(0)
		.i16(0)
		.i16(0)
		.i16(0)
		.u8(0x7f)
		.0;
	run(
		Root::PlayToClient,
		Version::V1_5_2,
		Version::V1_6_1,
		&spawn,
		&mut cx,
	);
	let old = w(0x28).i32(5).u8(0x48).i32(0x123456).u8(0x7f).0;
	let up = run(
		Root::PlayToClient,
		Version::V1_5_2,
		Version::V1_6_1,
		&old,
		&mut cx,
	);
	assert_eq!(up[0], w(0x28).i32(5).u8(0x47).i32(0x123456).u8(0x7f).0);
	let health = w(0x28)
		.i32(5)
		.u8(0x66)
		.f32(10.0)
		.u8(0x47)
		.i32(0x123456)
		.u8(0x7f)
		.0;
	let down = run(
		Root::PlayToClient,
		Version::V1_6_1,
		Version::V1_5_2,
		&health,
		&mut cx,
	);
	assert_eq!(down[0], old);
}

#[test]
fn chunk_unloads_and_bulks_reach_1_2_5() {
	let unload = w(0x33).i32(3).i32(4).bool(true).u16(0).u16(0).i32(0);
	let down = client(Version::V1_3_2, Version::V1_2_5, &unload);
	assert_eq!(down, vec![w(0x32).i32(3).i32(4).bool(false).0]);
	let up = client(Version::V1_2_5, Version::V1_3_2, &W(down[0].clone()));
	assert_eq!(
		names(Root::PlayToClient, Version::V1_3_2, &up),
		["map_chunk"]
	);

	let mut column = vec![1u8; 4096];
	column.extend(vec![0u8; 2048 * 3]);
	column.extend(vec![1u8; 256]);
	let data = deflate(&[column.clone(), column.clone()].concat());
	let bulk = w(0x38)
		.i16(2)
		.i32(data.len() as i32)
		.raw(&data)
		.i32(0)
		.i32(0)
		.u16(1)
		.u16(0)
		.i32(1)
		.i32(0)
		.u16(1)
		.u16(0);
	let split = client(Version::V1_3_2, Version::V1_2_5, &bulk);
	assert_eq!(
		names(Root::PlayToClient, Version::V1_2_5, &split),
		["pre_chunk", "map_chunk", "pre_chunk", "map_chunk"]
	);
}

#[test]
fn old_chunks_become_sections_and_back() {
	let mut raw = vec![0u8; 16 * 16 * 128];
	for x in 0..16 {
		for z in 0..16 {
			for y in 0..64 {
				raw[x << 11 | z << 7 | y] = if y == 63 { 2 } else { 1 };
			}
		}
	}
	raw.extend(vec![0u8; 16 * 16 * 64]);
	raw.extend(vec![0u8; 16 * 16 * 64]);
	raw.extend(vec![0xffu8; 16 * 16 * 64]);
	let old = w(0x33).i32(16).i16(0).i32(-32).u8(15).u8(127).u8(15).i32(0);
	let mut old = old;
	let data = deflate(&raw);
	old.0.truncate(old.0.len() - 4);
	let old = old.i32(data.len() as i32).raw(&data);
	let up = client(Version::V1_1, Version::V1_2_3, &old);
	assert_eq!(
		names(Root::PlayToClient, Version::V1_2_3, &up),
		["map_chunk"]
	);
	let down = client(Version::V1_2_3, Version::V1_1, &W(up[0].clone()));
	let back = &down[0];
	let len = i32::from_be_bytes(back[14..18].try_into().unwrap()) as usize;
	let blocks = inflate(&back[18..18 + len]);
	assert_eq!(&back[..14], &old.0[..14]);
	assert_eq!(&blocks[..16 * 16 * 128], &raw[..16 * 16 * 128]);
}

#[test]
fn dropped_items_are_objects_after_1_4_6() {
	let mut cx = Session::new();
	let item = w(0x15)
		.i32(9)
		.i16(264)
		.i8(3)
		.i16(0)
		.i16(-1)
		.i32(1)
		.i32(2)
		.i32(3)
		.i8(16)
		.i8(0)
		.i8(-16)
		.0;
	let up = run(
		Root::PlayToClient,
		Version::V1_4_5,
		Version::V1_4_7,
		&item,
		&mut cx,
	);
	assert_eq!(
		names(Root::PlayToClient, Version::V1_4_7, &up),
		["spawn_entity", "entity_metadata"]
	);
	let mut down = Vec::new();
	for p in &up {
		down.extend(run(
			Root::PlayToClient,
			Version::V1_4_7,
			Version::V1_4_5,
			p,
			&mut cx,
		));
	}
	assert_eq!(down, vec![item]);
}

#[test]
fn statistics_and_tab_completion_cross_netty() {
	let mut cx = Session::new();
	let stat = w(0xc8).i32(2010).i32(3).0;
	let up = run(
		Root::PlayToClient,
		Version::V1_6_4,
		Version::V1_7,
		&stat,
		&mut cx,
	);
	assert_eq!(
		up[0],
		[vec![0x37, 1, 9], b"stat.jump".to_vec(), vec![3]].concat()
	);
	let tab = w(0xcb).str("/help\0/home").0;
	let up = run(
		Root::PlayToClient,
		Version::V1_6_4,
		Version::V1_7,
		&tab,
		&mut cx,
	);
	assert_eq!(
		up[0],
		[
			vec![0x3a, 2, 5],
			b"/help".to_vec(),
			vec![5],
			b"/home".to_vec()
		]
		.concat()
	);
	let down = run(
		Root::PlayToClient,
		Version::V1_7,
		Version::V1_6_4,
		&up[0],
		&mut cx,
	);
	assert_eq!(down[0], tab);
}

#[test]
fn respawning_uses_the_right_packet_per_release() {
	let mut cx = Session::new();
	let command = w(0xcd).i8(1).0;
	let down = run(
		Root::PlayToServer,
		Version::V1_3_2,
		Version::V1_2_5,
		&command,
		&mut cx,
	);
	assert_eq!(
		names(Root::PlayToServer, Version::V1_2_5, &down),
		["respawn"]
	);
	let up = run(
		Root::PlayToServer,
		Version::V1_2_5,
		Version::V1_3_2,
		&down[0],
		&mut cx,
	);
	assert_eq!(up, vec![command]);
	let spawn = w(0xcd).i8(0).0;
	assert!(run(
		Root::PlayToServer,
		Version::V1_6_4,
		Version::V1_7,
		&spawn,
		&mut cx
	)
	.is_empty());
}

fn login_1_6_4(entity: i32) -> Vec<u8> {
	w(0x01)
		.i32(entity)
		.str("flat")
		.u8(0)
		.i8(0)
		.u8(2)
		.u8(0)
		.u8(20)
		.0
}

fn mob_1_6_4(entity: i32, ty: u8) -> Vec<u8> {
	w(0x18)
		.i32(entity)
		.u8(ty)
		.i32(0)
		.i32(2048)
		.i32(0)
		.i8(0)
		.i8(0)
		.i8(0)
		.i16(0)
		.i16(0)
		.i16(0)
		.u8(0x7f)
		.0
}

fn painting_1_6_4(title: &str) -> W {
	w(0x19).i32(9).str(title).i32(4).i32(70).i32(6).i32(2)
}

#[test]
fn paintings_unknown_to_1_0_become_paintings_of_the_same_size() {
	let wither = client(Version::V1_6_4, Version::V1_0_1, &painting_1_6_4("Wither"));
	assert_eq!(
		wither,
		vec![
			w(0x19)
				.i32(9)
				.str("SkullAndRoses")
				.i32(4)
				.i32(70)
				.i32(6)
				.i32(0)
				.0
		]
	);
	let kebab = client(Version::V1_6_4, Version::V1_0_1, &painting_1_6_4("Kebab"));
	assert_eq!(
		kebab,
		vec![w(0x19).i32(9).str("Kebab").i32(4).i32(70).i32(6).i32(0).0]
	);
	let unknown = client(Version::V1_4_2, Version::V1_3_2, &painting_1_6_4("Earth"));
	assert_eq!(
		unknown,
		vec![w(0x19).i32(9).str("Kebab").i32(4).i32(70).i32(6).i32(0).0]
	);
	assert_eq!(
		client(Version::V1_6_4, Version::V1_4_2, &painting_1_6_4("Wither"))[0],
		painting_1_6_4("Wither").0
	);
}

#[test]
fn only_players_swing_sleep_and_wake_on_1_0() {
	let mut cx = Session::new();
	let to_old = |p: &[u8], cx: &mut Session| {
		run(Root::PlayToClient, Version::V1_6_4, Version::V1_0_1, p, cx)
	};
	to_old(&login_1_6_4(42), &mut cx);
	to_old(&mob_1_6_4(5, 58), &mut cx);
	let player = w(0x14)
		.i32(6)
		.str("Notch")
		.i32(0)
		.i32(2048)
		.i32(0)
		.i8(0)
		.i8(0)
		.i16(0)
		.u8(0x7f)
		.0;
	to_old(&player, &mut cx);
	to_old(
		&w(0x17)
			.i32(7)
			.i8(10)
			.i32(0)
			.i32(2048)
			.i32(0)
			.i8(0)
			.i8(0)
			.i32(0)
			.0,
		&mut cx,
	);
	let swing = |entity: i32, animation: u8| w(0x12).i32(entity).u8(animation).0;
	for animation in [1, 3, 4] {
		assert!(
			to_old(&swing(5, animation), &mut cx).is_empty(),
			"{animation}"
		);
		assert!(
			to_old(&swing(77, animation), &mut cx).is_empty(),
			"{animation}"
		);
		assert_eq!(
			to_old(&swing(6, animation), &mut cx),
			vec![swing(6, animation)]
		);
		assert_eq!(
			to_old(&swing(42, animation), &mut cx),
			vec![swing(42, animation)]
		);
	}
	assert_eq!(to_old(&swing(5, 2), &mut cx), vec![swing(5, 2)]);
	assert!(run(
		Root::PlayToClient,
		Version::V1_6_4,
		Version::V1_1,
		&swing(5, 1),
		&mut cx
	)
	.is_empty());
	assert!(run(
		Root::PlayToClient,
		Version::V1_6_4,
		Version::V1_3_2,
		&swing(5, 1),
		&mut cx
	)
	.is_empty());
	assert_eq!(
		run(
			Root::PlayToClient,
			Version::V1_6_4,
			Version::V1_4_2,
			&swing(5, 1),
			&mut cx
		),
		vec![swing(5, 1)]
	);
	let bed = |entity: i32| w(0x11).i32(entity).i8(0).i32(1).u8(64).i32(2).0;
	assert!(to_old(&bed(5), &mut cx).is_empty());
	assert_eq!(to_old(&bed(6), &mut cx), vec![bed(6)]);
	let collect = |collector: i32| w(0x16).i32(8).i32(collector).0;
	assert!(to_old(&collect(7), &mut cx).is_empty());
	assert_eq!(to_old(&collect(5), &mut cx), vec![collect(5)]);
	assert_eq!(to_old(&collect(42), &mut cx), vec![collect(42)]);
}

#[test]
fn effects_unknown_to_old_releases_are_dropped() {
	let effect = |id: i8| w(0x29).i32(42).i8(id).i8(0).i16(200);
	let remove = |id: i8| w(0x2a).i32(42).i8(id);
	for id in 20..=23 {
		assert!(
			client(Version::V1_6_4, Version::V1_0_1, &effect(id)).is_empty(),
			"{id}"
		);
		assert!(
			client(Version::V1_6_4, Version::V1_0_1, &remove(id)).is_empty(),
			"{id}"
		);
	}
	assert_eq!(
		client(Version::V1_6_4, Version::V1_0_1, &effect(19)),
		vec![effect(19).0]
	);
	assert_eq!(
		client(Version::V1_6_4, Version::V1_4_2, &effect(20)),
		vec![effect(20).0]
	);
	assert!(client(Version::V1_6_4, Version::V1_4_2, &effect(22)).is_empty());
	assert_eq!(
		client(Version::V1_6_4, Version::V1_6_1, &effect(22)),
		vec![effect(22).0]
	);
}

#[test]
fn modern_and_abstract_mobs_never_spawn_on_1_0() {
	for ty in [30, 48, 49, 67, 68, 101, 200] {
		assert!(
			client(Version::V1_6_4, Version::V1_0_1, &W(mob_1_6_4(5, ty))).is_empty(),
			"{ty}"
		);
	}
	for ty in [50, 63, 90, 97, 120] {
		assert_eq!(
			client(Version::V1_6_4, Version::V1_0_1, &W(mob_1_6_4(5, ty))).len(),
			1,
			"{ty}"
		);
	}
}

#[test]
fn players_with_entity_id_0_keep_the_player_metadata_layout() {
	let mut cx = Session::new();
	let absorption = w(0x28).i32(0).u8(0x71).f32(4.0).u8(0x7f).0;
	assert_eq!(
		run(
			Root::PlayToClient,
			Version::V1_6_4,
			Version::V1_0_1,
			&absorption,
			&mut cx
		),
		vec![absorption.clone()]
	);
	run(
		Root::PlayToClient,
		Version::V1_6_4,
		Version::V1_0_1,
		&login_1_6_4(0),
		&mut cx,
	);
	let down = run(
		Root::PlayToClient,
		Version::V1_6_4,
		Version::V1_0_1,
		&absorption,
		&mut cx,
	);
	assert_eq!(down, vec![w(0x28).i32(0).u8(0x11).i8(4).u8(0x7f).0]);
}

#[test]
fn health_below_one_heart_stays_alive_on_1_5_2() {
	let mut cx = Session::new();
	for (health, expected) in [
		(0.5f32, 1i16),
		(6.4, 6),
		(6.6, 7),
		(20.0, 20),
		(0.0, 0),
		(-3.0, 0),
	] {
		let packet = w(0x08).f32(health).i16(18).f32(2.5).0;
		let down = run(
			Root::PlayToClient,
			Version::V1_6_1,
			Version::V1_5_2,
			&packet,
			&mut cx,
		);
		assert_eq!(
			down,
			vec![w(0x08).i16(expected).i16(18).f32(2.5).0],
			"health {health}"
		);
	}
}

fn spawn_type(packets: &[Vec<u8>]) -> Vec<(u8, u8)> {
	packets.iter().map(|p| (p[0], p[5])).collect()
}

#[test]
fn mobs_missing_from_old_releases_arrive_as_stand_ins() {
	for (ty, from, to, with) in [
		(100, Version::V1_6_4, Version::V1_5_2, 90),
		(64, Version::V1_6_4, Version::V1_3_2, 56),
		(66, Version::V1_6_4, Version::V1_3_2, 61),
		(65, Version::V1_6_4, Version::V1_3_2, 93),
		(98, Version::V1_6_4, Version::V1_1, 95),
		(99, Version::V1_6_4, Version::V1_1, 97),
		(64, Version::V1_6_4, Version::V1_0_1, 56),
		(100, Version::V1_6_4, Version::V1_0_1, 90),
	] {
		let out = client(from, to, &W(mob_1_6_4(5, ty)));
		assert_eq!(spawn_type(&out), vec![(0x18, with)], "{ty} {to}");
	}
	for (ty, to) in [
		(100, Version::V1_6_1),
		(64, Version::V1_4_2),
		(66, Version::V1_4_2),
		(98, Version::V1_2_3),
	] {
		assert_eq!(
			spawn_type(&client(Version::V1_6_4, to, &W(mob_1_6_4(5, ty)))),
			vec![(0x18, ty)],
			"{ty} {to}"
		);
	}
}

#[test]
fn stand_ins_keep_only_metadata_their_own_class_registers() {
	let mut cx = Session::new();
	let to = |p: &[u8], cx: &mut Session| {
		run(Root::PlayToClient, Version::V1_6_4, Version::V1_0_1, p, cx)
	};
	to(&login_1_6_4(42), &mut cx);
	let mut wither = mob_1_6_4(5, 64);
	wither.pop();
	let wither = W(wither)
		.u8(0x00)
		.i8(0x20)
		.u8(0x66)
		.f32(300.0)
		.u8(0x51)
		.i32(9)
		.u8(0x7f)
		.0;
	let out = to(&wither, &mut cx);
	assert_eq!(out.len(), 1);
	assert_eq!(out[0][5], 56);
	assert!(out[0].ends_with(&[0x00, 0x20, 0x7f]), "{:?}", out[0]);
	let target = w(0x28).i32(5).u8(0x51).i32(9).u8(0x52).i32(9).0;
	let left = to(&W(target).u8(0x7f).0, &mut cx);
	assert!(left.is_empty(), "{left:?}");
	let flags = w(0x28).i32(5).u8(0x00).i8(1).u8(0x51).i32(9).u8(0x7f).0;
	assert_eq!(
		to(&flags, &mut cx),
		vec![w(0x28).i32(5).u8(0x00).i8(1).u8(0x7f).0]
	);
	let mut horse = mob_1_6_4(6, 100);
	horse.pop();
	let horse = W(horse).u8(0x50).i32(4).u8(0x4c).i32(-1000).u8(0x7f).0;
	let out = run(
		Root::PlayToClient,
		Version::V1_6_4,
		Version::V1_5_2,
		&horse,
		&mut cx,
	);
	assert_eq!(out[0][5], 90);
	assert!(
		out[0].ends_with(&[0x7f]) && !out[0].contains(&0x50),
		"{:?}",
		out[0]
	);
	let ocelot = mob_1_6_4(7, 98);
	run(
		Root::PlayToClient,
		Version::V1_6_4,
		Version::V1_1,
		&ocelot,
		&mut cx,
	);
	let tame = w(0x28).i32(7).u8(0x12).i8(2).u8(0x00).i8(0).u8(0x7f).0;
	assert_eq!(
		run(
			Root::PlayToClient,
			Version::V1_6_4,
			Version::V1_1,
			&tame,
			&mut cx
		),
		vec![w(0x28).i32(7).u8(0x00).i8(0).u8(0x7f).0]
	);
}

#[test]
fn objects_missing_from_old_releases_arrive_as_stand_ins_or_not_at_all() {
	let object = |ty: i8| {
		w(0x17)
			.i32(9)
			.i8(ty)
			.i32(0)
			.i32(2048)
			.i32(0)
			.i8(0)
			.i8(0)
			.i32(0)
	};
	assert_eq!(
		spawn_type(&client(Version::V1_6_4, Version::V1_3_2, &object(66))),
		vec![(0x17, 64)]
	);
	assert_eq!(
		spawn_type(&client(Version::V1_6_4, Version::V1_4_2, &object(66))),
		vec![(0x17, 66)]
	);
	assert_eq!(
		spawn_type(&client(Version::V1_6_4, Version::V1_1, &object(75))),
		vec![(0x17, 61)]
	);
	assert_eq!(
		spawn_type(&client(Version::V1_6_4, Version::V1_2_3, &object(75))),
		vec![(0x17, 75)]
	);
	for (ty, to) in [
		(71, Version::V1_3_2),
		(76, Version::V1_4_5),
		(77, Version::V1_5_2),
	] {
		assert!(
			client(Version::V1_6_4, to, &object(ty)).is_empty(),
			"{ty} {to}"
		);
	}
}

fn varint(mut v: u32) -> Vec<u8> {
	let mut out = Vec::new();
	loop {
		if v < 0x80 {
			out.push(v as u8);
			return out;
		}
		out.push(v as u8 & 0x7f | 0x80);
		v >>= 7;
	}
}

#[test]
fn netty_swings_and_wake_ups_only_reach_entities_that_can_take_them() {
	let mut cx = Session::new();
	let to = |p: Vec<u8>, cx: &mut Session| {
		run(Root::PlayToClient, Version::V1_7, Version::V1_6_4, &p, cx)
	};
	let login = [
		vec![0x01],
		42i32.to_be_bytes().to_vec(),
		vec![0, 0, 2, 20, 4],
		b"flat".to_vec(),
	]
	.concat();
	to(login, &mut cx);
	let minecart = [
		vec![0x0e],
		varint(7),
		vec![10],
		0i32.to_be_bytes().to_vec(),
		2048i32.to_be_bytes().to_vec(),
		0i32.to_be_bytes().to_vec(),
		vec![0, 0],
		0i32.to_be_bytes().to_vec(),
	]
	.concat();
	to(minecart, &mut cx);
	let animate = |id: u32, a: u8| [vec![0x0b], varint(id), vec![a]].concat();
	assert!(to(animate(7, 0), &mut cx).is_empty());
	assert_eq!(to(animate(8, 0), &mut cx), vec![w(0x12).i32(8).u8(1).0]);
	assert!(to(animate(8, 2), &mut cx).is_empty());
	assert_eq!(to(animate(42, 2), &mut cx), vec![w(0x12).i32(42).u8(3).0]);
	let collect = |collector: i32| {
		[
			vec![0x0d],
			9i32.to_be_bytes().to_vec(),
			collector.to_be_bytes().to_vec(),
		]
		.concat()
	};
	assert!(to(collect(7), &mut cx).is_empty());
	assert_eq!(to(collect(42), &mut cx), vec![w(0x16).i32(9).i32(42).0]);
	let bed = |id: i32| {
		[
			vec![0x0a],
			id.to_be_bytes().to_vec(),
			1i32.to_be_bytes().to_vec(),
			vec![64],
			2i32.to_be_bytes().to_vec(),
		]
		.concat()
	};
	assert!(to(bed(8), &mut cx).is_empty());
	assert_eq!(to(bed(42), &mut cx).len(), 1);
}

#[test]
fn strings_fit_the_limits_of_1_0_and_1_1() {
	let long = "x".repeat(150);
	assert_eq!(
		client(Version::V1_6_4, Version::V1_0_1, &w(0xff).str(&long)),
		vec![w(0xff).str(&long[..100]).0]
	);
	assert_eq!(
		client(Version::V1_6_4, Version::V1_1, &w(0xff).str(&long)),
		vec![w(0xff).str(&long).0]
	);
	let window = |title: &str| w(0x64).u8(1).u8(0).str(title).u8(27);
	assert_eq!(
		client(
			Version::V1_2_3,
			Version::V1_1,
			&window("Game Menu (Right Click)")
		),
		vec![window("Game Menu (Right").0]
	);
	assert_eq!(
		client(
			Version::V1_2_5,
			Version::V1_2_3,
			&window("Game Menu (Right Click)")
		),
		vec![window("Game Menu (Right Click)").0]
	);
}
