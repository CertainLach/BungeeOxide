use std::io::Read;

use flate2::read::ZlibDecoder;
use minecraft_versions::custom::chunks::{encode_chunk, Biomes, Column, Light, Nibbles, Section};
use minecraft_versions::gen::convert::{convert, index_of, v1_0_1, v1_8, Root};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::Session;

type C8 = v1_8::play_toClient_packet::Ty;
type S8 = v1_8::play_toServer_packet::Ty;
type C0 = v1_0_1::play_toClient_packet::Ty;
type S0 = v1_0_1::play_toServer_packet::Ty;
type Cuboid = v1_0_1::play_toClient_packet_map_chunk::Ty;

const IDS: [u32; 16] = [1, 3, 4, 5, 7, 12, 13, 14, 15, 16, 17, 19, 20, 22, 24, 35];

fn at(version: &str) -> usize {
	index_of(version).unwrap()
}

fn session(versions: bool) -> Session {
	let mut cx = Session::new();
	if versions {
		cx.set_versions(at("1.8"), at("1.0.1"));
	}
	cx
}

fn decoded(packets: Vec<Vec<u8>>) -> Vec<C0> {
	packets
		.iter()
		.map(|p| v1_0_1::play_toClient_packet::deserialize(&mut Input::new(p)).unwrap())
		.collect()
}

fn down_bytes(bytes: &[u8], cx: &mut Session) -> Vec<C0> {
	decoded(convert(Root::PlayToClient, at("1.8"), at("1.0.1"), bytes, cx).unwrap())
}

fn down(p: C8, cx: &mut Session) -> Vec<C0> {
	let mut o = Output::new();
	v1_8::play_toClient_packet::serialize(&p, &mut o).unwrap();
	down_bytes(&o.into_inner(), cx)
}

fn up(p: S0, cx: &mut Session) -> (Vec<S8>, Vec<C0>) {
	let mut o = Output::new();
	v1_0_1::play_toServer_packet::serialize(&p, &mut o).unwrap();
	let sent = convert(
		Root::PlayToServer,
		at("1.0.1"),
		at("1.8"),
		&o.into_inner(),
		cx,
	)
	.unwrap();
	let sent = sent
		.iter()
		.map(|p| v1_8::play_toServer_packet::deserialize(&mut Input::new(p)).unwrap())
		.collect();
	let mut back = Vec::new();
	for o in cx.take_outgoing() {
		assert_eq!(o.root, Root::PlayToClient);
		back.extend(decoded(
			convert(o.root, o.version, at("1.0.1"), &o.data, cx).unwrap(),
		));
	}
	(sent, back)
}

fn column(x: i32, z: i32) -> Vec<u8> {
	let mut col = Column::new(x, z, true, at("1.8"));
	col.sections = IDS
		.iter()
		.map(|id| Some(Section::new(vec![id << 4; 4096], None)))
		.collect();
	let mut light = Light::new(16, true);
	for s in 0..16 {
		light.block[s + 1] = Nibbles::Data(vec![0; 2048]);
		light.sky[s + 1] = Nibbles::Data(vec![if s >= 12 { 0xff } else { 0 }; 2048]);
	}
	col.light = Some(light);
	col.biomes = Biomes::Flat(vec![1; 256]);
	encode_chunk(at("1.8"), &col).unwrap().remove(0)
}

fn login() -> C8 {
	C8::login(v1_8::play_toClient_packet_login::Ty {
		entityId: 7,
		gameMode: 1,
		dimension: 0,
		difficulty: 1,
		maxPlayers: 20,
		levelType: "default".to_owned(),
		reducedDebugInfo: false,
	})
}

fn teleport(y: f64) -> C8 {
	C8::position(v1_8::play_toClient_packet_position::Ty {
		x: 8.5,
		y,
		z: 8.5,
		yaw: 0.0,
		pitch: 0.0,
		flags: 0,
	})
}

fn block(x: i32, y: i32, z: i32, id: u32) -> C8 {
	C8::block_change(v1_8::play_toClient_packet_block_change::Ty {
		location: v1_8::position::Ty { x, y: y as i16, z },
		r#type: (id << 4) as i32,
	})
}

fn walk(y: f64) -> S0 {
	S0::position_look(v1_0_1::play_toServer_packet_position_look::Ty {
		x: 8.5,
		y,
		stance: y + 1.62,
		z: 8.5,
		yaw: 0.0,
		pitch: 0.0,
		onGround: true,
	})
}

fn ids(c: &Cuboid) -> Vec<u8> {
	let mut raw = Vec::new();
	ZlibDecoder::new(&c.compressedChunkData[..])
		.read_to_end(&mut raw)
		.unwrap();
	(0..=c.sizeY as usize).map(|y| raw[y]).collect()
}

fn one(v: Vec<C0>) -> C0 {
	assert_eq!(v.len(), 1, "{v:?}");
	v.into_iter().next().unwrap()
}

fn cuboid(out: &[C0], x: i32) -> Option<&Cuboid> {
	out.iter().find_map(|p| match p {
		C0::map_chunk(c) if c.x == x * 16 && c.z == 0 => Some(c),
		_ => None,
	})
}

fn feet(out: &[C0]) -> f64 {
	out.iter()
		.find_map(|p| match p {
			C0::position(p) => Some(p.stance),
			_ => None,
		})
		.expect("a position")
}

fn server_y(sent: &[S8]) -> f64 {
	match sent {
		[S8::position_look(p)] => p.y,
		[S8::position(p)] => p.y,
		s => panic!("{s:?}"),
	}
}

fn world(cx: &mut Session) {
	down(login(), cx);
	for x in [0, 1, 4] {
		down_bytes(&column(x, 0), cx);
	}
}

#[test]
fn old_clients_see_128_high_windows_of_256_high_worlds() {
	let mut cx = session(true);
	down(login(), &mut cx);
	let out = down_bytes(&column(0, 0), &mut cx);
	let c = cuboid(&out, 0).unwrap();
	assert_eq!((c.y, c.sizeY), (0, 127));
	let expected: Vec<u8> = (0..128).map(|y| IDS[y / 16] as u8).collect();
	assert_eq!(ids(c), expected);
	down_bytes(&column(1, 0), &mut cx);
	down_bytes(&column(4, 0), &mut cx);
	assert_eq!(feet(&down(teleport(70.0), &mut cx)), 70.0);
	let (sent, _) = up(walk(70.0), &mut cx);
	assert_eq!(server_y(&sent), 70.0);

	let out = down(teleport(200.0), &mut cx);
	assert_eq!(feet(&out), 72.0);
	let c = cuboid(&out, 0).unwrap();
	assert_eq!(
		ids(c),
		(128..256).map(|y| IDS[y / 16] as u8).collect::<Vec<u8>>()
	);
	assert!(cuboid(&out, 1).is_some() && cuboid(&out, 4).is_none());
	assert!(
		out.iter().position(|p| matches!(p, C0::map_chunk(_)))
			< out.iter().position(|p| matches!(p, C0::position(_)))
	);
	let (sent, back) = up(walk(72.0), &mut cx);
	assert_eq!(server_y(&sent), 200.0);
	assert!(back
		.iter()
		.any(|p| matches!(p, C0::map_chunk(c) if c.x == 64)));

	let C0::block_change(b) = one(down(block(3, 210, 4, 41), &mut cx)) else {
		panic!()
	};
	assert_eq!((b.location.y, b.r#type), (82, 41));
	let C0::block_change(b) = one(down(block(3, 100, 4, 0), &mut cx)) else {
		panic!()
	};
	assert_eq!((b.location.y, b.r#type), (0, IDS[8] as u8));
	let C0::block_change(b) = one(down(block(3, 128, 4, 0), &mut cx)) else {
		panic!()
	};
	assert_eq!((b.location.y, b.r#type), (0, IDS[7] as u8));

	for (client, server) in [(60.0, 188.0), (40.0, 168.0), (29.0, 157.0)] {
		let (sent, back) = up(walk(client), &mut cx);
		assert_eq!((server_y(&sent), back.len()), (server, 0));
	}
	let (sent, back) = up(walk(27.0), &mut cx);
	assert_eq!(server_y(&sent), 155.0);
	assert_eq!(feet(&back), 59.0);
	let c = cuboid(&back, 0).unwrap();
	assert_eq!(ids(c)[0], IDS[6] as u8);
	let (sent, _) = up(walk(59.0), &mut cx);
	assert_eq!(server_y(&sent), 155.0);
	let (sent, _) = up(walk(58.0), &mut cx);
	assert_eq!(server_y(&sent), 154.0);
	let dig = S0::block_dig(v1_0_1::play_toServer_packet_block_dig::Ty {
		status: 0,
		location: v1_0_1::position_ibi::Ty { x: 3, y: 57, z: 4 },
		face: 1,
	});
	let (sent, _) = up(dig, &mut cx);
	let [S8::block_dig(d)] = sent.as_slice() else {
		panic!("{sent:?}")
	};
	assert_eq!(d.location.y, 153);
}

#[test]
fn server_teleports_wait_for_the_echo() {
	let mut cx = session(true);
	world(&mut cx);
	down(teleport(70.0), &mut cx);
	up(walk(70.0), &mut cx);
	assert_eq!(feet(&down(teleport(10.0), &mut cx)), 10.0);
	let (sent, back) = up(walk(70.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (70.0, 0));
	let (sent, _) = up(walk(10.0), &mut cx);
	assert_eq!(server_y(&sent), 10.0);
	let out = down(teleport(240.0), &mut cx);
	assert_eq!(feet(&out), 112.0);
	let (sent, _) = up(walk(10.0), &mut cx);
	assert_eq!(server_y(&sent), 10.0);
	let (sent, _) = up(walk(112.0), &mut cx);
	assert_eq!(server_y(&sent), 240.0);
}

#[test]
fn without_versions_the_world_is_cut_as_before() {
	let mut cx = session(false);
	world(&mut cx);
	assert_eq!(feet(&down(teleport(200.0), &mut cx)), 200.0);
	assert!(down(block(3, 210, 4, 41), &mut cx).is_empty());
	let (sent, back) = up(walk(200.0), &mut cx);
	assert_eq!((server_y(&sent), back.len()), (200.0, 0));
}
