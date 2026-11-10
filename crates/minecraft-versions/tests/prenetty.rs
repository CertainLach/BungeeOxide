use minecraft_versions::gen;
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::pds_native::pds_legacySlot::Item;

struct W(Vec<u8>);

fn w(id: u8) -> W {
	W(vec![id])
}

fn bytes() -> W {
	W(vec![])
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
	fn f64(self, v: f64) -> Self {
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
	fn then(self, other: W) -> Self {
		self.raw(&other.0)
	}
}

macro_rules! round_trip {
	($root:ident, $bytes:expr) => {{
		let bytes: Vec<u8> = $bytes.0;
		let mut input = Input::new(&bytes);
		let packet = gen::$root::deserialize(&mut input).unwrap();
		assert_eq!(input.remaining(), 0, "{packet:?}");
		let mut out = Output::new();
		gen::$root::serialize(&packet, &mut out).unwrap();
		assert_eq!(out.into_inner(), bytes, "{packet:?}");
		assert_eq!(gen::$root::size_of(&packet), bytes.len());
		packet
	}};
}

macro_rules! expect {
	($root:ident :: $arm:ident, $bytes:expr) => {{
		match round_trip!($root, $bytes) {
			gen::$root::Ty::$arm(p) => p,
			p => panic!("expected {}, got {p:?}", stringify!($arm)),
		}
	}};
}

macro_rules! absent {
	($root:ident, $($id:expr),+) => {{
		$(
			let bytes = [$id];
			let packet = gen::$root::deserialize(&mut Input::new(&bytes)).unwrap();
			assert!(matches!(packet, gen::$root::Ty::default(id) if id == $id), "{packet:?}");
		)+
	}};
}

fn debug(v: &impl std::fmt::Debug) -> String {
	format!("{v:?}")
}

const GZIP: &[u8] = &[
	0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x03, 0x00,
];
const CHUNK: &[u8] = &[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01];

fn slot(id: i16, count: i8, damage: i16, nbt: Option<&[u8]>) -> W {
	let item = bytes().i16(id).i8(count).i16(damage);
	match nbt {
		Some(nbt) => item.i16(nbt.len() as i16).raw(nbt),
		None => item.i16(-1),
	}
}

macro_rules! slots_with_nbt {
	($name:ident, $client:ident) => {
		#[test]
		fn $name() {
			let items = expect!(
				$client::window_items,
				w(0x68)
					.u8(0)
					.i16(3)
					.i16(-1)
					.then(slot(276, 1, 5, Some(GZIP)))
					.then(slot(1, 64, 0, None))
			);
			assert_eq!(items.windowId, 0);
			assert_eq!(debug(&items.items[0]), "_1");
			let gen::pds_slot_v1_3_2::Ty::default(id, sword) = &items.items[1] else {
				panic!()
			};
			assert_eq!((*id, sword.itemCount, sword.itemDamage), (276, 1, 5));
			assert_eq!(sword.nbtData.as_deref(), Some(GZIP));
			let gen::pds_slot_v1_3_2::Ty::default(id, stone) = &items.items[2] else {
				panic!()
			};
			assert_eq!((*id, stone.itemCount, &stone.nbtData), (1, 64, &None));

			let set = expect!($client::set_slot, w(0x67).i8(-1).i16(-1).i16(-1));
			assert_eq!(
				(set.windowId, set.slot, debug(&set.item).as_str()),
				(-1, -1, "_1")
			);
		}
	};
}

slots_with_nbt!(slots_1_3_2, pds_v1_3_2_play_toClient_packet);
slots_with_nbt!(slots_1_4_2, pds_v1_4_2_play_toClient_packet);
slots_with_nbt!(slots_1_4_7, pds_v1_4_7_play_toClient_packet);
slots_with_nbt!(slots_1_5_2, pds_v1_5_2_play_toClient_packet);
slots_with_nbt!(slots_1_6_4, pds_v1_6_4_play_toClient_packet);

macro_rules! legacy_slots {
	($name:ident, $client:ident, $server:ident) => {
		#[test]
		fn $name() {
			let items = expect!(
				$client::window_items,
				w(0x68)
					.u8(0)
					.i16(4)
					.i16(-1)
					.then(slot(276, 1, 5, Some(GZIP)))
					.i16(1)
					.i8(64)
					.i16(0)
					.then(slot(261, 1, 0, None))
			);
			let item = |id, count, damage, nbt: Option<&[u8]>| {
				Some(Item {
					id,
					itemCount: count,
					itemDamage: damage,
					nbtData: nbt.map(<[u8]>::to_vec),
				})
			};
			assert_eq!(
				items.items,
				vec![
					None,
					item(276, 1, 5, Some(GZIP)),
					item(1, 64, 0, None),
					item(261, 1, 0, None)
				]
			);

			let place = expect!(
				$server::block_place,
				w(0x0f).i32(10).u8(64).i32(-10).i8(1).i16(4).i8(12).i16(0)
			);
			assert_eq!(
				(place.location.y, place.direction, place.heldItem.clone()),
				(64, 1, item(4, 12, 0, None))
			);

			let click = expect!(
				$server::window_click,
				w(0x66)
					.i8(0)
					.i16(36)
					.i8(0)
					.i16(7)
					.i8(1)
					.then(slot(359, 1, 30, None))
			);
			assert_eq!(
				(click.slot, click.mode, click.item),
				(36, 1, item(359, 1, 30, None))
			);
		}
	};
}

legacy_slots!(
	slots_1_0_1,
	pds_v1_0_1_play_toClient_packet,
	pds_v1_0_1_play_toServer_packet
);
legacy_slots!(
	slots_1_1,
	pds_v1_1_play_toClient_packet,
	pds_v1_1_play_toServer_packet
);
legacy_slots!(
	slots_1_2_3,
	pds_v1_2_3_play_toClient_packet,
	pds_v1_2_3_play_toServer_packet
);
legacy_slots!(
	slots_1_2_5,
	pds_v1_2_5_play_toClient_packet,
	pds_v1_2_5_play_toServer_packet
);

macro_rules! login_1_3 {
	($name:ident, $client:ident, $server:ident, $protocol:expr) => {
		#[test]
		fn $name() {
			let login = expect!(
				$client::login,
				w(0x01)
					.i32(17)
					.str("default")
					.u8(1)
					.i8(-1)
					.u8(2)
					.u8(0)
					.u8(20)
			);
			assert_eq!(
				(
					login.entityId,
					login.levelType.as_str(),
					login.gameMode,
					login.dimension,
					login.difficulty,
					login.maxPlayers
				),
				(17, "default", 1, -1, 2, 20)
			);
			let hs = expect!(
				$server::set_protocol,
				w(0x02)
					.u8($protocol)
					.str("Notch")
					.str("localhost")
					.i32(25565)
			);
			assert_eq!(
				(
					hs.protocolVersion,
					hs.username.as_str(),
					hs.serverHost.as_str(),
					hs.serverPort
				),
				($protocol, "Notch", "localhost", 25565)
			);
			let auth = expect!(
				$client::encryption_begin,
				w(0xfd)
					.str("-")
					.i16(3)
					.raw(&[1, 2, 3])
					.i16(4)
					.raw(&[9, 8, 7, 6])
			);
			assert_eq!(
				(
					auth.serverId.as_str(),
					auth.publicKey.len(),
					auth.verifyToken.len()
				),
				("-", 3, 4)
			);
			let key = expect!(
				$server::encryption_begin,
				w(0xfc).i16(2).raw(&[5, 5]).i16(0)
			);
			assert_eq!((key.sharedSecret.len(), key.verifyToken.len()), (2, 0));
			let done = expect!($client::shared_key, w(0xfc).i16(0).i16(0));
			assert!(done.sharedSecret.is_empty() && done.verifyToken.is_empty());
			let respawn = expect!(
				$client::respawn,
				w(0x09).i32(-1).u8(1).u8(0).i16(256).str("flat")
			);
			assert_eq!(
				(
					respawn.dimension,
					respawn.worldHeight,
					respawn.levelType.as_str()
				),
				(-1, 256, "flat")
			);
			let kick = expect!(
				$client::kick_disconnect,
				w(0xff).str("\u{a7}1\u{0}78\u{0}1.6.4\u{0}A Minecraft Server\u{0}0\u{0}20")
			);
			assert_eq!(kick.split('\u{0}').nth(2), Some("1.6.4"));
			let chat = expect!($client::chat, w(0x03).str("\u{a7}e\u{e9}t\u{e9} \u{1f600}"));
			assert!(chat.ends_with('\u{1f600}'));
		}
	};
}

login_1_3!(
	login_1_3_2,
	pds_v1_3_2_play_toClient_packet,
	pds_v1_3_2_play_toServer_packet,
	39
);
login_1_3!(
	login_1_4_2,
	pds_v1_4_2_play_toClient_packet,
	pds_v1_4_2_play_toServer_packet,
	47
);
login_1_3!(
	login_1_4_5,
	pds_v1_4_5_play_toClient_packet,
	pds_v1_4_5_play_toServer_packet,
	49
);
login_1_3!(
	login_1_4_7,
	pds_v1_4_7_play_toClient_packet,
	pds_v1_4_7_play_toServer_packet,
	51
);
login_1_3!(
	login_1_5_1,
	pds_v1_5_1_play_toClient_packet,
	pds_v1_5_1_play_toServer_packet,
	60
);
login_1_3!(
	login_1_5_2,
	pds_v1_5_2_play_toClient_packet,
	pds_v1_5_2_play_toServer_packet,
	61
);
login_1_3!(
	login_1_6_1,
	pds_v1_6_1_play_toClient_packet,
	pds_v1_6_1_play_toServer_packet,
	73
);
login_1_3!(
	login_1_6_2,
	pds_v1_6_2_play_toClient_packet,
	pds_v1_6_2_play_toServer_packet,
	74
);
login_1_3!(
	login_1_6_4,
	pds_v1_6_4_play_toClient_packet,
	pds_v1_6_4_play_toServer_packet,
	78
);

macro_rules! chunks_1_3 {
	($name:ident, $client:ident, $bulk_meta:expr) => {
		#[test]
		fn $name() {
			let chunk = expect!(
				$client::map_chunk,
				w(0x33)
					.i32(-2)
					.i32(7)
					.bool(true)
					.u16(0b101)
					.u16(0)
					.i32(CHUNK.len() as i32)
					.raw(CHUNK)
			);
			assert_eq!(
				(
					chunk.x,
					chunk.z,
					chunk.groundUp,
					chunk.bitMap,
					chunk.addBitMap
				),
				(-2, 7, true, 0b101, 0)
			);
			assert_eq!(chunk.compressedChunkData, CHUNK);

			let bulk = expect!(
				$client::map_chunk_bulk,
				w(0x38).i16(2).i32(CHUNK.len() as i32).then($bulk_meta)
			);
			assert_eq!(bulk.meta.len(), 2);
			assert_eq!(
				(bulk.meta[1].x, bulk.meta[1].z, bulk.meta[1].bitMap),
				(1, -1, 0xffff)
			);

			let blocks = expect!(
				$client::multi_block_change,
				w(0x34)
					.i32(3)
					.i32(-4)
					.i16(2)
					.i32(8)
					.u8(0x5a)
					.u8(70)
					.u8(0x23)
					.u8(0x45)
					.u8(0x00)
					.u8(0)
					.u8(0x00)
					.u8(0x10)
			);
			let r = &blocks.records[0];
			assert_eq!((r.horizontalPos, r.y, r.blockId), (0x5a, 70, 0x2345));
			assert_eq!((blocks.records[1].blockId, blocks.dataLength), (0x10, 8));
		}
	};
}

fn bulk_tail() -> W {
	bytes()
		.raw(CHUNK)
		.i32(0)
		.i32(0)
		.u16(1)
		.u16(0)
		.i32(1)
		.i32(-1)
		.u16(0xffff)
		.u16(0)
}

chunks_1_3!(chunks_1_3_2, pds_v1_3_2_play_toClient_packet, bulk_tail());
chunks_1_3!(chunks_1_4_5, pds_v1_4_5_play_toClient_packet, bulk_tail());
chunks_1_3!(
	chunks_1_4_7,
	pds_v1_4_7_play_toClient_packet,
	bytes().bool(true).then(bulk_tail())
);
chunks_1_3!(
	chunks_1_6_4,
	pds_v1_6_4_play_toClient_packet,
	bytes().bool(false).then(bulk_tail())
);

#[test]
fn metadata_with_nbt_slots_1_4_5_to_1_6_4() {
	fn meta() -> W {
		w(0x28)
			.i32(42)
			.u8(0x00)
			.i8(0x10)
			.u8(0x8a)
			.str("Dinnerbone")
			.u8(0xaa)
			.then(slot(261, 1, 0, Some(GZIP)))
			.u8(0xab)
			.i16(-1)
			.u8(0xc1)
			.i32(1)
			.i32(64)
			.i32(-3)
			.u8(0x66)
			.f32(20.0)
			.u8(0x7f)
	}
	let m = expect!(pds_v1_6_4_play_toClient_packet::entity_metadata, meta()).metadata;
	assert_eq!(m.len(), 6);
	assert_eq!((m[1].ANON.r#type, m[1].ANON.key), (4, 10));
	assert_eq!(debug(&m[1].value), "v4(\"Dinnerbone\")");
	assert!(debug(&m[2].value).starts_with("v5(default(261,"));
	assert_eq!(debug(&m[3].value), "v5(_1)");
	assert_eq!(debug(&m[4].value), "v6(Ty { x: 1, y: 64, z: -3 })");
	assert_eq!((m[5].ANON.r#type, m[5].ANON.key), (3, 6));
	expect!(pds_v1_4_5_play_toClient_packet::entity_metadata, meta());
	expect!(pds_v1_5_2_play_toClient_packet::entity_metadata, meta());

	let mob = expect!(
		pds_v1_6_4_play_toClient_packet::spawn_entity_living,
		w(0x18)
			.i32(9)
			.u8(50)
			.i32(1)
			.i32(2)
			.i32(3)
			.i8(4)
			.i8(5)
			.i8(6)
			.i16(7)
			.i16(8)
			.i16(9)
			.u8(0x7f)
	);
	assert_eq!(
		(
			mob.r#type,
			mob.headPitch,
			mob.velocity.z,
			mob.metadata.len()
		),
		(50, 6, 9, 0)
	);
	let player = expect!(
		pds_v1_6_4_play_toClient_packet::named_entity_spawn,
		w(0x14)
			.i32(1)
			.str("Notch")
			.i32(32)
			.i32(64)
			.i32(96)
			.i8(0)
			.i8(0)
			.i16(276)
			.u8(0x00)
			.i8(0)
			.u8(0x7f)
	);
	assert_eq!(
		(
			player.playerName.as_str(),
			player.currentItem,
			player.metadata.len()
		),
		("Notch", 276, 1)
	);
}

#[test]
fn metadata_without_nbt_1_4_2() {
	let m = expect!(
		pds_v1_4_2_play_toClient_packet::entity_metadata,
		w(0x28)
			.i32(1)
			.u8(0xaa)
			.i16(264)
			.i8(1)
			.i16(0)
			.u8(0xab)
			.i16(-1)
			.u8(0x7f)
	)
	.metadata;
	assert!(debug(&m[0].value).starts_with("v5(default(264, Ty { itemCount: 1, itemDamage: 0 })"));
	assert_eq!(debug(&m[1].value), "v5(_1)");
	let map = expect!(
		pds_v1_4_2_play_toClient_packet::map,
		w(0x83).i16(358).i16(4).u8(3).raw(&[2, 0, 1])
	);
	assert_eq!(
		(map.itemType, map.itemDamage, map.data.as_slice()),
		(358, 4, &[2u8, 0, 1][..])
	);
	let item = expect!(
		pds_v1_4_2_play_toClient_packet::spawn_entity_item,
		w(0x15)
			.i32(7)
			.then(slot(264, 3, 0, None))
			.i32(1)
			.i32(2)
			.i32(3)
			.i8(1)
			.i8(2)
			.i8(3)
	);
	assert_eq!((item.entityId, item.velocityZ), (7, 3));
}

macro_rules! metadata_b1_5 {
	($name:ident, $client:ident) => {
		#[test]
		fn $name() {
			let m = expect!(
				$client::entity_metadata,
				w(0x28)
					.i32(1)
					.u8(0xaa)
					.i16(-1)
					.i8(0)
					.i16(0)
					.u8(0x84)
					.str("x")
					.u8(0x7f)
			)
			.metadata;
			assert_eq!(
				debug(&m[0].value),
				"v5(Ty { id: -1, itemCount: 0, itemDamage: 0 })"
			);
			assert_eq!(debug(&m[1].value), "v4(\"x\")");
			let item = expect!(
				$client::spawn_entity_item,
				w(0x15)
					.i32(7)
					.i16(264)
					.i8(3)
					.i16(0)
					.i32(1)
					.i32(2)
					.i32(3)
					.i8(1)
					.i8(2)
					.i8(3)
			);
			assert_eq!((item.item.id, item.item.itemCount, item.x), (264, 3, 1));
		}
	};
}

metadata_b1_5!(metadata_1_0_1, pds_v1_0_1_play_toClient_packet);
metadata_b1_5!(metadata_1_2_5, pds_v1_2_5_play_toClient_packet);
metadata_b1_5!(metadata_1_3_2, pds_v1_3_2_play_toClient_packet);

#[test]
fn server_list_ping() {
	let host = bytes().u8(78).str("localhost").i32(25565).0;
	let stream = w(0xfe)
		.u8(1)
		.then(w(0xfa).str("MC|PingHost").i16(host.len() as i16).raw(&host))
		.0;
	let input = &mut Input::new(&stream);
	use gen::pds_v1_6_4_play_toServer_packet as s;
	assert!(matches!(
		s::deserialize(input).unwrap(),
		s::Ty::legacy_server_list_ping(1)
	));
	let s::Ty::custom_payload(ping) = s::deserialize(input).unwrap() else {
		panic!()
	};
	assert_eq!(input.remaining(), 0);
	assert_eq!(
		(ping.channel.as_str(), ping.data.as_slice()),
		("MC|PingHost", host.as_slice())
	);

	expect!(
		pds_v1_4_2_play_toServer_packet::legacy_server_list_ping,
		w(0xfe).u8(1)
	);
	expect!(
		pds_v1_3_2_play_toServer_packet::legacy_server_list_ping,
		w(0xfe)
	);
	expect!(
		pds_v1_0_1_play_toServer_packet::legacy_server_list_ping,
		w(0xfe)
	);
}

#[test]
fn play_1_6_4() {
	use gen::pds_v1_6_4_play_toClient_packet as c;
	let horse = expect!(
		pds_v1_6_4_play_toClient_packet::open_window,
		w(0x64).u8(3).u8(11).str("Horse").u8(2).bool(true).i32(99)
	);
	assert_eq!(
		(horse.windowTitle.as_str(), debug(&horse.entityId).as_str()),
		("Horse", "v11(99)")
	);
	round_trip!(
		pds_v1_6_4_play_toClient_packet,
		w(0x64).u8(1).u8(0).str("Chest").u8(27).bool(false)
	);

	let attrs = expect!(
		pds_v1_6_4_play_toClient_packet::update_attributes,
		w(0x2c)
			.i32(5)
			.i32(1)
			.str("generic.movementSpeed")
			.f64(0.1)
			.i16(1)
			.raw(&[0x66; 16])
			.f64(0.3)
			.i8(2)
	);
	let speed = &attrs.properties[0];
	assert_eq!(
		(speed.key.as_str(), speed.value, speed.modifiers.len()),
		("generic.movementSpeed", 0.1, 1)
	);
	assert_eq!(speed.modifiers[0].uuid, u128::from_be_bytes([0x66; 16]));

	let pos = expect!(
		pds_v1_6_4_play_toClient_packet::position,
		w(0x0d)
			.f64(1.5)
			.f64(65.62)
			.f64(64.0)
			.f64(-3.5)
			.f32(90.0)
			.f32(0.0)
			.bool(false)
	);
	assert_eq!((pos.y, pos.stance), (65.62, 64.0));
	let moved = expect!(
		pds_v1_6_4_play_toServer_packet::position,
		w(0x0b).f64(1.5).f64(64.0).f64(65.62).f64(-3.5).bool(true)
	);
	assert_eq!((moved.y, moved.stance), (64.0, 65.62));

	let place = expect!(
		pds_v1_6_4_play_toServer_packet::block_place,
		w(0x0f)
			.i32(-1)
			.u8(255)
			.i32(-1)
			.i8(-1)
			.then(slot(359, 1, 3, None))
			.i8(0)
			.i8(8)
			.i8(16)
	);
	assert_eq!(
		(place.location.y, place.direction, place.cursorZ),
		(255, -1, 16)
	);

	let respawn = expect!(
		pds_v1_6_4_play_toServer_packet::client_command,
		w(0xcd).i8(1)
	);
	assert_eq!(debug(&respawn), "perform_respawn");
	let action = expect!(
		pds_v1_6_4_play_toServer_packet::entity_action,
		w(0x13).i32(1).i8(6).i32(80)
	);
	assert_eq!(action.jumpBoost, 80);
	let steer = expect!(
		pds_v1_6_4_play_toServer_packet::steer_vehicle,
		w(0x1b).f32(0.5).f32(-1.0).bool(true).bool(false)
	);
	assert!(steer.jump && !steer.unmount);
	let sign = expect!(
		pds_v1_6_4_play_toClient_packet::open_sign_entity,
		w(0x85).i8(0).i32(1).i32(2).i32(3)
	);
	assert_eq!(sign.location.z, 3);
	let item = expect!(
		pds_v1_6_4_play_toClient_packet::held_item_slot,
		w(0x10).i16(4)
	);
	assert_eq!(item, 4);
	assert!(matches!(
		round_trip!(pds_v1_6_4_play_toClient_packet, w(0x00).i32(77)),
		c::Ty::keep_alive(77)
	));
	absent!(pds_v1_6_4_play_toClient_packet, 0x02, 0x15, 0x32);
	absent!(pds_v1_6_4_play_toServer_packet, 0x1c);
}

#[test]
fn play_1_6_1() {
	let attrs = expect!(
		pds_v1_6_1_play_toClient_packet::update_attributes,
		w(0x2c).i32(5).i32(1).str("generic.maxHealth").f64(20.0)
	);
	assert_eq!(attrs.properties[0].value, 20.0);
	let abilities = expect!(
		pds_v1_6_1_play_toServer_packet::abilities,
		w(0xca).i8(0x0d).f32(0.05).f32(0.1)
	);
	assert_eq!(abilities.walkingSpeed, 0.1);
	absent!(pds_v1_6_1_play_toClient_packet, 0x85);
}

#[test]
fn play_1_5() {
	for health in [
		expect!(
			pds_v1_5_2_play_toClient_packet::update_health,
			w(0x08).i16(20).i16(18).f32(5.0)
		)
		.health,
		expect!(
			pds_v1_5_1_play_toClient_packet::update_health,
			w(0x08).i16(20).i16(18).f32(5.0)
		)
		.health,
	] {
		assert_eq!(health, 20);
	}
	let window = expect!(
		pds_v1_5_2_play_toClient_packet::open_window,
		w(0x64).u8(1).u8(11).str("Chest").u8(27).bool(true)
	);
	assert_eq!((window.inventoryType, window.useProvidedTitle), (11, true));
	let abilities = expect!(
		pds_v1_5_2_play_toServer_packet::abilities,
		w(0xca).i8(0x0d).i8(12).i8(25)
	);
	assert_eq!(abilities.walkingSpeed, 25);
	let attach = expect!(
		pds_v1_5_2_play_toClient_packet::attach_entity,
		w(0x27).i32(1).i32(-1)
	);
	assert_eq!(attach.vehicleId, -1);
	let stat = expect!(
		pds_v1_5_2_play_toClient_packet::statistics,
		w(0xc8).i32(1000).i8(1)
	);
	assert_eq!(stat.amount, 1);
	let score = expect!(
		pds_v1_5_2_play_toClient_packet::scoreboard_score,
		w(0xcf).str("Notch").i8(0).str("kills").i32(3)
	);
	assert_eq!(
		debug(&score.action),
		"default(0, Ty { scoreName: \"kills\", value: 3 })"
	);
	let particles = expect!(
		pds_v1_5_2_play_toClient_packet::world_particles,
		w(0x3f)
			.str("heart")
			.f32(0.0)
			.f32(1.0)
			.f32(2.0)
			.f32(0.0)
			.f32(0.0)
			.f32(0.0)
			.f32(1.0)
			.i32(5)
	);
	assert_eq!(particles.particles, 5);
	absent!(pds_v1_5_2_play_toClient_packet, 0x2c);
	absent!(pds_v1_5_2_play_toServer_packet, 0x1b);
	let action = expect!(
		pds_v1_5_2_play_toServer_packet::entity_action,
		w(0x13).i32(1).i8(1)
	);
	assert_eq!(action.actionId, 1);
}

#[test]
fn play_1_4() {
	let object = expect!(
		pds_v1_4_7_play_toClient_packet::spawn_entity,
		w(0x17)
			.i32(9)
			.i8(10)
			.i32(1)
			.i32(2)
			.i32(3)
			.i8(0)
			.i8(64)
			.i32(0)
	);
	assert_eq!((object.yaw, debug(&object.objectData).as_str()), (64, "v0"));
	let window = expect!(
		pds_v1_4_7_play_toClient_packet::open_window,
		w(0x64).u8(1).u8(0).str("Chest").u8(27)
	);
	assert_eq!(window.slotCount, 27);
	expect!(
		pds_v1_4_7_play_toClient_packet::held_item_slot,
		w(0x10).i16(3)
	);
	absent!(pds_v1_4_7_play_toClient_packet, 0x15, 0x3f, 0xce);

	let arrow = expect!(
		pds_v1_4_5_play_toClient_packet::spawn_entity,
		w(0x17)
			.i32(9)
			.i8(60)
			.i32(1)
			.i32(2)
			.i32(3)
			.i32(5)
			.i16(1)
			.i16(2)
			.i16(3)
	);
	assert!(debug(&arrow.objectData).starts_with("default(5,"));
	let item = expect!(
		pds_v1_4_5_play_toClient_packet::spawn_entity_item,
		w(0x15)
			.i32(7)
			.then(slot(264, 3, 0, None))
			.i32(1)
			.i32(2)
			.i32(3)
			.i8(1)
			.i8(2)
			.i8(3)
	);
	assert_eq!((item.entityId, item.velocityY), (7, 2));
	absent!(pds_v1_4_5_play_toClient_packet, 0x10);
	let event = expect!(
		pds_v1_4_2_play_toClient_packet::world_event,
		w(0x3d).i32(1013).i32(1).u8(64).i32(2).i32(0).bool(true)
	);
	assert!(event.global);
	let time = expect!(
		pds_v1_4_2_play_toClient_packet::update_time,
		w(0x04).i64(48000).i64(6000)
	);
	assert_eq!((time.age, time.time), (48000, 6000));
	let settings = expect!(
		pds_v1_4_2_play_toServer_packet::settings,
		w(0xcc).str("en_US").i8(0).i8(8).u8(2).bool(true)
	);
	assert!(settings.showCape);
}

#[test]
fn play_1_3_2() {
	let time = expect!(
		pds_v1_3_2_play_toClient_packet::update_time,
		w(0x04).i64(6000)
	);
	assert_eq!(time.time, 6000);
	let event = expect!(
		pds_v1_3_2_play_toClient_packet::world_event,
		w(0x3d).i32(1001).i32(1).u8(64).i32(2).i32(0)
	);
	assert_eq!(event.effectId, 1001);
	let settings = expect!(
		pds_v1_3_2_play_toServer_packet::settings,
		w(0xcc).str("en_US").i8(0).i8(8).u8(2)
	);
	assert_eq!(settings.locale, "en_US");
	let tile = expect!(
		pds_v1_3_2_play_toClient_packet::tile_entity_data,
		w(0x84)
			.i32(1)
			.i16(64)
			.i32(2)
			.u8(1)
			.i16(GZIP.len() as i16)
			.raw(GZIP)
	);
	assert_eq!(tile.nbtData.as_deref(), Some(GZIP));
	let abilities = expect!(
		pds_v1_3_2_play_toClient_packet::abilities,
		w(0xca).i8(0x0d).i8(12).i8(25)
	);
	assert_eq!(abilities.flags, 0x0d);
	let spawn = expect!(
		pds_v1_3_2_play_toServer_packet::client_command,
		w(0xcd).i8(0)
	);
	assert_eq!(debug(&spawn), "initial_spawn");
	absent!(pds_v1_3_2_play_toClient_packet, 0x10, 0x3f, 0x02);
}

macro_rules! play_1_2 {
	($name:ident, $client:ident, $server:ident, $abilities:tt) => {
		#[test]
		fn $name() {
			let login = expect!(
				$client::login,
				w(0x01).i32(17).str("").str("default").i32(1).i32(-1).u8(1).u8(0).u8(20)
			);
			assert_eq!((login.levelType.as_str(), login.gameMode, login.dimension), ("default", 1, -1));
			let request = expect!(
				$server::login,
				w(0x01).i32(29).str("Notch").str("").i32(0).i32(0).u8(0).u8(0).u8(0)
			);
			assert_eq!((request.protocolVersion, request.username.as_str()), (29, "Notch"));
			let hash = expect!($client::handshake, w(0x02).str("-"));
			assert_eq!(hash.connectionHash, "-");
			let hello = expect!($server::set_protocol, w(0x02).str("Notch;localhost:25565"));
			assert_eq!(hello.usernameAndHost, "Notch;localhost:25565");
			let chunk = expect!(
				$client::map_chunk,
				w(0x33).i32(1).i32(2).bool(true).u16(1).u16(0).i32(CHUNK.len() as i32).i32(0).raw(CHUNK)
			);
			assert_eq!((chunk.unused, chunk.compressedChunkData.as_slice()), (0, CHUNK));
			let alloc = expect!($client::pre_chunk, w(0x32).i32(1).i32(2).bool(true));
			assert!(alloc.mode);
			let tile = expect!(
				$client::tile_entity_data,
				w(0x84).i32(1).i16(64).i32(2).u8(1).i32(50).i32(0).i32(0)
			);
			assert_eq!((tile.action, tile.custom1), (1, 50));
			let destroy = expect!($client::entity_destroy, w(0x1d).i32(5));
			assert_eq!(destroy, 5);
			let equipment = expect!($client::entity_equipment, w(0x05).i32(1).i16(0).i16(267).i16(3));
			assert_eq!((equipment.itemId, equipment.itemDamage), (267, 3));
			let player = expect!(
				$client::named_entity_spawn,
				w(0x14).i32(1).str("Notch").i32(0).i32(0).i32(0).i8(0).i8(0).i16(0)
			);
			assert_eq!(player.playerName, "Notch");
			let mob = expect!(
				$client::spawn_entity_living,
				w(0x18).i32(1).u8(50).i32(0).i32(0).i32(0).i8(1).i8(2).i8(3).u8(0x00).i8(0).u8(0x7f)
			);
			assert_eq!((mob.headPitch, mob.metadata.len()), (3, 1));
			let explosion = expect!(
				$client::explosion,
				w(0x3c).f64(0.5).f64(64.0).f64(0.5).f32(3.0).i32(1).i8(1).i8(0).i8(-1)
			);
			assert_eq!(explosion.affectedBlockOffsets[0].z, -1);
			let block = expect!($client::block_change, w(0x35).i32(1).u8(64).i32(2).u8(46).u8(0));
			assert_eq!(block.r#type, 46);
			let respawn = expect!($server::respawn, w(0x09).i32(0).u8(1).u8(0).i16(256).str("default"));
			assert_eq!(respawn.worldHeight, 256);
			play_1_2!(@abilities $abilities, $client, $server);
			absent!($client, 0x10, 0x37, 0x38, 0x3e, 0xcb, 0xfc, 0xfd);
			absent!($server, 0xcb, 0xcc, 0xcd, 0xfc);
		}
	};
	(@abilities true, $client:ident, $server:ident) => {
		let abilities = expect!($client::abilities, w(0xca).bool(false).bool(true).bool(true).bool(false));
		assert!(abilities.flying && abilities.canFly && !abilities.instantBuild);
		expect!($server::abilities, w(0xca).bool(false).bool(false).bool(true).bool(true));
	};
	(@abilities false, $client:ident, $server:ident) => {
		absent!($client, 0xca);
		absent!($server, 0xca);
	};
}

play_1_2!(
	play_1_2_5,
	pds_v1_2_5_play_toClient_packet,
	pds_v1_2_5_play_toServer_packet,
	true
);
play_1_2!(
	play_1_2_3,
	pds_v1_2_3_play_toClient_packet,
	pds_v1_2_3_play_toServer_packet,
	false
);

#[test]
fn play_1_1() {
	let login = expect!(
		pds_v1_1_play_toClient_packet::login,
		w(0x01)
			.i32(17)
			.str("")
			.i64(-123)
			.str("default")
			.i32(0)
			.i8(0)
			.u8(1)
			.u8(128)
			.u8(20)
	);
	assert_eq!(
		(login.seed, login.levelType.as_str(), login.dimension),
		(-123, "default", 0)
	);
	let respawn = expect!(
		pds_v1_1_play_toServer_packet::respawn,
		w(0x09).i8(-1).u8(1).u8(0).i16(128).i64(-123).str("default")
	);
	assert_eq!((respawn.dimension, respawn.seed), (-1, -123));
	let chunk = expect!(
		pds_v1_1_play_toClient_packet::map_chunk,
		w(0x33)
			.i32(16)
			.i16(0)
			.i32(32)
			.u8(15)
			.u8(127)
			.u8(15)
			.i32(CHUNK.len() as i32)
			.raw(CHUNK)
	);
	assert_eq!(
		(chunk.x, chunk.sizeY, chunk.compressedChunkData.len()),
		(16, 127, CHUNK.len())
	);
	let blocks = expect!(
		pds_v1_1_play_toClient_packet::multi_block_change,
		w(0x34)
			.i32(1)
			.i32(2)
			.i16(2)
			.u16(0x5a46)
			.u16(0x0001)
			.u8(1)
			.u8(2)
			.u8(0)
			.u8(3)
	);
	let c = &blocks.coordinates;
	assert_eq!((c[0].x, c[0].z, c[0].y, c[1].y), (5, 10, 0x46, 1));
	assert_eq!(
		(blocks.blockIds.as_slice(), blocks.metadata.as_slice()),
		(&[1u8, 2][..], &[0u8, 3][..])
	);
	let brand = expect!(
		pds_v1_1_play_toServer_packet::custom_payload,
		w(0xfa).str("MC|Brand").i16(1).raw(b"x")
	);
	assert_eq!(brand.data, b"x");
	let hello = expect!(
		pds_v1_1_play_toServer_packet::set_protocol,
		w(0x02).str("Notch")
	);
	assert_eq!(hello.username, "Notch");
	absent!(pds_v1_1_play_toClient_packet, 0x23, 0x84, 0xca);
	absent!(pds_v1_1_play_toServer_packet, 0x1b);
}

#[test]
fn play_1_0_1() {
	let login = expect!(
		pds_v1_0_1_play_toClient_packet::login,
		w(0x01)
			.i32(17)
			.str("")
			.i64(5)
			.i32(1)
			.i8(0)
			.u8(1)
			.u8(128)
			.u8(8)
	);
	assert_eq!((login.seed, login.gameMode, login.maxPlayers), (5, 1, 8));
	let request = expect!(
		pds_v1_0_1_play_toServer_packet::login,
		w(0x01)
			.i32(22)
			.str("Notch")
			.i64(0)
			.i32(0)
			.i8(0)
			.u8(0)
			.u8(0)
			.u8(0)
	);
	assert_eq!(request.protocolVersion, 22);
	let respawn = expect!(
		pds_v1_0_1_play_toClient_packet::respawn,
		w(0x09).i8(0).u8(1).u8(0).i16(128).i64(5)
	);
	assert_eq!(respawn.seed, 5);
	let stance = expect!(
		pds_v1_0_1_play_toServer_packet::stance_update,
		w(0x1b)
			.f32(0.0)
			.f32(1.0)
			.f32(2.0)
			.f32(3.0)
			.bool(false)
			.bool(true)
	);
	assert_eq!((stance.yaw, stance.unmount), (3.0, true));
	absent!(pds_v1_0_1_play_toClient_packet, 0xfa, 0x23);
	absent!(pds_v1_0_1_play_toServer_packet, 0xfa);
}
