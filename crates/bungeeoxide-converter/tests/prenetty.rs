use bungeeoxide_converter::minecraft;
use bungeeoxide_converter::pds::{Input, Output};
use bungeeoxide_converter::pds_native::pds_legacySlot::Item;

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
	($($root:ident)::+, $bytes:expr) => {{
		use minecraft::$($root)::+ as root;
		let bytes: Vec<u8> = $bytes.0;
		let mut input = Input::new(&bytes);
		let packet = root::deserialize(&mut input).unwrap();
		assert_eq!(input.remaining(), 0, "{packet:?}");
		let mut out = Output::new();
		root::serialize(&packet, &mut out).unwrap();
		assert_eq!(out.into_inner(), bytes, "{packet:?}");
		assert_eq!(root::size_of(&packet), bytes.len());
		packet
	}};
}

macro_rules! expect {
	($($seg:ident)::+, $bytes:expr) => {
		expect!(@split [] $($seg)+ ; $bytes)
	};
	(@split [$($root:ident)*] $arm:ident ; $bytes:expr) => {{
		use minecraft::$($root)::* as root;
		match round_trip!($($root)::*, $bytes) {
			root::Ty::$arm(p) => p,
			p => panic!("expected {}, got {p:?}", stringify!($arm)),
		}
	}};
	(@split [$($root:ident)*] $head:ident $($rest:ident)+ ; $bytes:expr) => {
		expect!(@split [$($root)* $head] $($rest)+ ; $bytes)
	};
}

macro_rules! absent {
	($($root:ident)::+, $($id:expr),+) => {{
		use minecraft::$($root)::+ as root;
		$(
			let bytes = [$id];
			let packet = root::deserialize(&mut Input::new(&bytes)).unwrap();
			assert!(matches!(packet, root::Ty::Default(id) if id == $id), "{packet:?}");
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
	($name:ident, $($client:ident)::+) => {
		#[test]
		fn $name() {
			let items = expect!(
				$($client)::+::WindowItems,
				w(0x68)
					.u8(0)
					.i16(3)
					.i16(-1)
					.then(slot(276, 1, 5, Some(GZIP)))
					.then(slot(1, 64, 0, None))
			);
			assert_eq!(items.window_id, 0);
			assert_eq!(debug(&items.items[0]), "_1");
			let minecraft::v1_3_2::Slot::Default(id, sword) = &items.items[1] else {
				panic!()
			};
			assert_eq!((*id, sword.item_count, sword.item_damage), (276, 1, 5));
			assert_eq!(sword.nbt_data.as_deref(), Some(GZIP));
			let minecraft::v1_3_2::Slot::Default(id, stone) = &items.items[2] else {
				panic!()
			};
			assert_eq!((*id, stone.item_count, &stone.nbt_data), (1, 64, &None));

			let set = expect!($($client)::+::SetSlot, w(0x67).i8(-1).i16(-1).i16(-1));
			assert_eq!(
				(set.window_id, set.slot, debug(&set.item).as_str()),
				(-1, -1, "_1")
			);
		}
	};
}

slots_with_nbt!(slots_1_3_2, v1_3_2::play::to_client::packet);
slots_with_nbt!(slots_1_4_2, v1_4_2::play::to_client::packet);
slots_with_nbt!(slots_1_4_7, v1_4_7::play::to_client::packet);
slots_with_nbt!(slots_1_5_2, v1_5_2::play::to_client::packet);
slots_with_nbt!(slots_1_6_4, v1_6_4::play::to_client::packet);

macro_rules! legacy_slots {
	($name:ident, $($client:ident)::+, $($server:ident)::+) => {
		#[test]
		fn $name() {
			let items = expect!(
				$($client)::+::WindowItems,
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
				$($server)::+::BlockPlace,
				w(0x0f).i32(10).u8(64).i32(-10).i8(1).i16(4).i8(12).i16(0)
			);
			assert_eq!(
				(place.location.y, place.direction, place.held_item.clone()),
				(64, 1, item(4, 12, 0, None))
			);

			let click = expect!(
				$($server)::+::WindowClick,
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
	v1_0_1::play::to_client::packet,
	v1_0_1::play::to_server::packet
);
legacy_slots!(
	slots_1_1,
	v1_1::play::to_client::packet,
	v1_1::play::to_server::packet
);
legacy_slots!(
	slots_1_2_3,
	v1_2_3::play::to_client::packet,
	v1_2_3::play::to_server::packet
);
legacy_slots!(
	slots_1_2_5,
	v1_2_5::play::to_client::packet,
	v1_2_5::play::to_server::packet
);

macro_rules! login_1_3 {
	($name:ident, $($client:ident)::+, $($server:ident)::+, $protocol:expr) => {
		#[test]
		fn $name() {
			let login = expect!(
				$($client)::+::Login,
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
					login.entity_id,
					login.level_type.as_str(),
					login.game_mode,
					login.dimension,
					login.difficulty,
					login.max_players
				),
				(17, "default", 1, -1, 2, 20)
			);
			let hs = expect!(
				$($server)::+::SetProtocol,
				w(0x02)
					.u8($protocol)
					.str("Notch")
					.str("localhost")
					.i32(25565)
			);
			assert_eq!(
				(
					hs.protocol_version,
					hs.username.as_str(),
					hs.server_host.as_str(),
					hs.server_port
				),
				($protocol, "Notch", "localhost", 25565)
			);
			let auth = expect!(
				$($client)::+::EncryptionBegin,
				w(0xfd)
					.str("-")
					.i16(3)
					.raw(&[1, 2, 3])
					.i16(4)
					.raw(&[9, 8, 7, 6])
			);
			assert_eq!(
				(
					auth.server_id.as_str(),
					auth.public_key.len(),
					auth.verify_token.len()
				),
				("-", 3, 4)
			);
			let key = expect!(
				$($server)::+::EncryptionBegin,
				w(0xfc).i16(2).raw(&[5, 5]).i16(0)
			);
			assert_eq!((key.shared_secret.len(), key.verify_token.len()), (2, 0));
			let done = expect!($($client)::+::SharedKey, w(0xfc).i16(0).i16(0));
			assert!(done.shared_secret.is_empty() && done.verify_token.is_empty());
			let respawn = expect!(
				$($client)::+::Respawn,
				w(0x09).i32(-1).u8(1).u8(0).i16(256).str("flat")
			);
			assert_eq!(
				(
					respawn.dimension,
					respawn.world_height,
					respawn.level_type.as_str()
				),
				(-1, 256, "flat")
			);
			let kick = expect!(
				$($client)::+::KickDisconnect,
				w(0xff).str("\u{a7}1\u{0}78\u{0}1.6.4\u{0}A Minecraft Server\u{0}0\u{0}20")
			);
			assert_eq!(kick.split('\u{0}').nth(2), Some("1.6.4"));
			let chat = expect!($($client)::+::Chat, w(0x03).str("\u{a7}e\u{e9}t\u{e9} \u{1f600}"));
			assert!(chat.ends_with('\u{1f600}'));
		}
	};
}

login_1_3!(
	login_1_3_2,
	v1_3_2::play::to_client::packet,
	v1_3_2::play::to_server::packet,
	39
);
login_1_3!(
	login_1_4_2,
	v1_4_2::play::to_client::packet,
	v1_4_2::play::to_server::packet,
	47
);
login_1_3!(
	login_1_4_5,
	v1_4_5::play::to_client::packet,
	v1_4_5::play::to_server::packet,
	49
);
login_1_3!(
	login_1_4_7,
	v1_4_7::play::to_client::packet,
	v1_4_7::play::to_server::packet,
	51
);
login_1_3!(
	login_1_5_1,
	v1_5_1::play::to_client::packet,
	v1_5_1::play::to_server::packet,
	60
);
login_1_3!(
	login_1_5_2,
	v1_5_2::play::to_client::packet,
	v1_5_2::play::to_server::packet,
	61
);
login_1_3!(
	login_1_6_1,
	v1_6_1::play::to_client::packet,
	v1_6_1::play::to_server::packet,
	73
);
login_1_3!(
	login_1_6_2,
	v1_6_2::play::to_client::packet,
	v1_6_2::play::to_server::packet,
	74
);
login_1_3!(
	login_1_6_4,
	v1_6_4::play::to_client::packet,
	v1_6_4::play::to_server::packet,
	78
);

macro_rules! chunks_1_3 {
	($name:ident, $($client:ident)::+, $bulk_meta:expr) => {
		#[test]
		fn $name() {
			let chunk = expect!(
				$($client)::+::MapChunk,
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
					chunk.ground_up,
					chunk.bit_map,
					chunk.add_bit_map
				),
				(-2, 7, true, 0b101, 0)
			);
			assert_eq!(chunk.compressed_chunk_data, CHUNK);

			let bulk = expect!(
				$($client)::+::MapChunkBulk,
				w(0x38).i16(2).i32(CHUNK.len() as i32).then($bulk_meta)
			);
			assert_eq!(bulk.meta.len(), 2);
			assert_eq!(
				(bulk.meta[1].x, bulk.meta[1].z, bulk.meta[1].bit_map),
				(1, -1, 0xffff)
			);

			let blocks = expect!(
				$($client)::+::MultiBlockChange,
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
			assert_eq!((r.horizontal_pos, r.y, r.block_id), (0x5a, 70, 0x2345));
			assert_eq!((blocks.records[1].block_id, blocks.data_length), (0x10, 8));
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

chunks_1_3!(chunks_1_3_2, v1_3_2::play::to_client::packet, bulk_tail());
chunks_1_3!(chunks_1_4_5, v1_4_5::play::to_client::packet, bulk_tail());
chunks_1_3!(
	chunks_1_4_7,
	v1_4_7::play::to_client::packet,
	bytes().bool(true).then(bulk_tail())
);
chunks_1_3!(
	chunks_1_6_4,
	v1_6_4::play::to_client::packet,
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
	let m = expect!(v1_6_4::play::to_client::packet::EntityMetadata, meta()).metadata;
	assert_eq!(m.len(), 6);
	assert_eq!((m[1].anon.r#type, m[1].anon.key), (4, 10));
	assert_eq!(debug(&m[1].value), "V4(\"Dinnerbone\")");
	assert!(debug(&m[2].value).starts_with("V5(Default(261,"));
	assert_eq!(debug(&m[3].value), "V5(_1)");
	assert_eq!(
		debug(&m[4].value),
		"V6(EntityMetadataItem2 { x: 1, y: 64, z: -3 })"
	);
	assert_eq!((m[5].anon.r#type, m[5].anon.key), (3, 6));
	expect!(v1_4_5::play::to_client::packet::EntityMetadata, meta());
	expect!(v1_5_2::play::to_client::packet::EntityMetadata, meta());

	let mob = expect!(
		v1_6_4::play::to_client::packet::SpawnEntityLiving,
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
			mob.head_pitch,
			mob.velocity.z,
			mob.metadata.len()
		),
		(50, 6, 9, 0)
	);
	let player = expect!(
		v1_6_4::play::to_client::packet::NamedEntitySpawn,
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
			player.player_name.as_str(),
			player.current_item,
			player.metadata.len()
		),
		("Notch", 276, 1)
	);
}

#[test]
fn metadata_without_nbt_1_4_2() {
	let m = expect!(
		v1_4_2::play::to_client::packet::EntityMetadata,
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
	assert!(debug(&m[0].value)
		.starts_with("V5(Default(264, EntityMetadataItem2 { item_count: 1, item_damage: 0 })"));
	assert_eq!(debug(&m[1].value), "V5(_1)");
	let map = expect!(
		v1_4_2::play::to_client::packet::Map,
		w(0x83).i16(358).i16(4).u8(3).raw(&[2, 0, 1])
	);
	assert_eq!(
		(map.item_type, map.item_damage, map.data.as_slice()),
		(358, 4, &[2u8, 0, 1][..])
	);
	let item = expect!(
		v1_4_2::play::to_client::packet::SpawnEntityItem,
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
	assert_eq!((item.entity_id, item.velocity_z), (7, 3));
}

macro_rules! metadata_b1_5 {
	($name:ident, $($client:ident)::+) => {
		#[test]
		fn $name() {
			let m = expect!(
				$($client)::+::EntityMetadata,
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
				"V5(EntityMetadataItem2 { id: -1, item_count: 0, item_damage: 0 })"
			);
			assert_eq!(debug(&m[1].value), "V4(\"x\")");
			let item = expect!(
				$($client)::+::SpawnEntityItem,
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
			assert_eq!((item.item.id, item.item.item_count, item.x), (264, 3, 1));
		}
	};
}

metadata_b1_5!(metadata_1_0_1, v1_0_1::play::to_client::packet);
metadata_b1_5!(metadata_1_2_5, v1_2_5::play::to_client::packet);
metadata_b1_5!(metadata_1_3_2, v1_3_2::play::to_client::packet);

#[test]
fn server_list_ping() {
	let host = bytes().u8(78).str("localhost").i32(25565).0;
	let stream = w(0xfe)
		.u8(1)
		.then(w(0xfa).str("MC|PingHost").i16(host.len() as i16).raw(&host))
		.0;
	let input = &mut Input::new(&stream);
	use minecraft::v1_6_4::play::to_server::packet as s;
	assert!(matches!(
		s::deserialize(input).unwrap(),
		s::Ty::LegacyServerListPing(1)
	));
	let s::Ty::CustomPayload(ping) = s::deserialize(input).unwrap() else {
		panic!()
	};
	assert_eq!(input.remaining(), 0);
	assert_eq!(
		(ping.channel.as_str(), ping.data.as_slice()),
		("MC|PingHost", host.as_slice())
	);

	expect!(
		v1_4_2::play::to_server::packet::LegacyServerListPing,
		w(0xfe).u8(1)
	);
	expect!(
		v1_3_2::play::to_server::packet::LegacyServerListPing,
		w(0xfe)
	);
	expect!(
		v1_0_1::play::to_server::packet::LegacyServerListPing,
		w(0xfe)
	);
}

#[test]
fn play_1_6_4() {
	use minecraft::v1_6_4::play::to_client::packet as c;
	let horse = expect!(
		v1_6_4::play::to_client::packet::OpenWindow,
		w(0x64).u8(3).u8(11).str("Horse").u8(2).bool(true).i32(99)
	);
	assert_eq!(
		(
			horse.window_title.as_str(),
			debug(&horse.entity_id).as_str()
		),
		("Horse", "V11(99)")
	);
	round_trip!(
		v1_6_4::play::to_client::packet,
		w(0x64).u8(1).u8(0).str("Chest").u8(27).bool(false)
	);

	let attrs = expect!(
		v1_6_4::play::to_client::packet::UpdateAttributes,
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
		v1_6_4::play::to_client::packet::Position,
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
		v1_6_4::play::to_server::packet::Position,
		w(0x0b).f64(1.5).f64(64.0).f64(65.62).f64(-3.5).bool(true)
	);
	assert_eq!((moved.y, moved.stance), (64.0, 65.62));

	let place = expect!(
		v1_6_4::play::to_server::packet::BlockPlace,
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
		(place.location.y, place.direction, place.cursor_z),
		(255, -1, 16)
	);

	let respawn = expect!(
		v1_6_4::play::to_server::packet::ClientCommand,
		w(0xcd).i8(1)
	);
	assert_eq!(debug(&respawn), "PerformRespawn");
	let action = expect!(
		v1_6_4::play::to_server::packet::EntityAction,
		w(0x13).i32(1).i8(6).i32(80)
	);
	assert_eq!(action.jump_boost, 80);
	let steer = expect!(
		v1_6_4::play::to_server::packet::SteerVehicle,
		w(0x1b).f32(0.5).f32(-1.0).bool(true).bool(false)
	);
	assert!(steer.jump && !steer.unmount);
	let sign = expect!(
		v1_6_4::play::to_client::packet::OpenSignEntity,
		w(0x85).i8(0).i32(1).i32(2).i32(3)
	);
	assert_eq!(sign.location.z, 3);
	let item = expect!(
		v1_6_4::play::to_client::packet::HeldItemSlot,
		w(0x10).i16(4)
	);
	assert_eq!(item, 4);
	assert!(matches!(
		round_trip!(v1_6_4::play::to_client::packet, w(0x00).i32(77)),
		c::Ty::KeepAlive(77)
	));
	absent!(v1_6_4::play::to_client::packet, 0x02, 0x15, 0x32);
	absent!(v1_6_4::play::to_server::packet, 0x1c);
}

#[test]
fn play_1_6_1() {
	let attrs = expect!(
		v1_6_1::play::to_client::packet::UpdateAttributes,
		w(0x2c).i32(5).i32(1).str("generic.maxHealth").f64(20.0)
	);
	assert_eq!(attrs.properties[0].value, 20.0);
	let abilities = expect!(
		v1_6_1::play::to_server::packet::Abilities,
		w(0xca).i8(0x0d).f32(0.05).f32(0.1)
	);
	assert_eq!(abilities.walking_speed, 0.1);
	absent!(v1_6_1::play::to_client::packet, 0x85);
}

#[test]
fn play_1_5() {
	for health in [
		expect!(
			v1_5_2::play::to_client::packet::UpdateHealth,
			w(0x08).i16(20).i16(18).f32(5.0)
		)
		.health,
		expect!(
			v1_5_1::play::to_client::packet::UpdateHealth,
			w(0x08).i16(20).i16(18).f32(5.0)
		)
		.health,
	] {
		assert_eq!(health, 20);
	}
	let window = expect!(
		v1_5_2::play::to_client::packet::OpenWindow,
		w(0x64).u8(1).u8(11).str("Chest").u8(27).bool(true)
	);
	assert_eq!(
		(window.inventory_type, window.use_provided_title),
		(11, true)
	);
	let abilities = expect!(
		v1_5_2::play::to_server::packet::Abilities,
		w(0xca).i8(0x0d).i8(12).i8(25)
	);
	assert_eq!(abilities.walking_speed, 25);
	let attach = expect!(
		v1_5_2::play::to_client::packet::AttachEntity,
		w(0x27).i32(1).i32(-1)
	);
	assert_eq!(attach.vehicle_id, -1);
	let stat = expect!(
		v1_5_2::play::to_client::packet::Statistics,
		w(0xc8).i32(1000).i8(1)
	);
	assert_eq!(stat.amount, 1);
	let score = expect!(
		v1_5_2::play::to_client::packet::ScoreboardScore,
		w(0xcf).str("Notch").i8(0).str("kills").i32(3)
	);
	assert_eq!(
		debug(&score.action),
		"Default(0, PacketScoreboardScore0 { score_name: \"kills\", value: 3 })"
	);
	let particles = expect!(
		v1_5_2::play::to_client::packet::WorldParticles,
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
	absent!(v1_5_2::play::to_client::packet, 0x2c);
	absent!(v1_5_2::play::to_server::packet, 0x1b);
	let action = expect!(
		v1_5_2::play::to_server::packet::EntityAction,
		w(0x13).i32(1).i8(1)
	);
	assert_eq!(action.action_id, 1);
}

#[test]
fn play_1_4() {
	let object = expect!(
		v1_4_7::play::to_client::packet::SpawnEntity,
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
	assert_eq!(
		(object.yaw, debug(&object.object_data).as_str()),
		(64, "V0")
	);
	let window = expect!(
		v1_4_7::play::to_client::packet::OpenWindow,
		w(0x64).u8(1).u8(0).str("Chest").u8(27)
	);
	assert_eq!(window.slot_count, 27);
	expect!(
		v1_4_7::play::to_client::packet::HeldItemSlot,
		w(0x10).i16(3)
	);
	absent!(v1_4_7::play::to_client::packet, 0x15, 0x3f, 0xce);

	let arrow = expect!(
		v1_4_5::play::to_client::packet::SpawnEntity,
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
	assert!(debug(&arrow.object_data).starts_with("Default(5,"));
	let item = expect!(
		v1_4_5::play::to_client::packet::SpawnEntityItem,
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
	assert_eq!((item.entity_id, item.velocity_y), (7, 2));
	absent!(v1_4_5::play::to_client::packet, 0x10);
	let event = expect!(
		v1_4_2::play::to_client::packet::WorldEvent,
		w(0x3d).i32(1013).i32(1).u8(64).i32(2).i32(0).bool(true)
	);
	assert!(event.global);
	let time = expect!(
		v1_4_2::play::to_client::packet::UpdateTime,
		w(0x04).i64(48000).i64(6000)
	);
	assert_eq!((time.age, time.time), (48000, 6000));
	let settings = expect!(
		v1_4_2::play::to_server::packet::Settings,
		w(0xcc).str("en_US").i8(0).i8(8).u8(2).bool(true)
	);
	assert!(settings.show_cape);
}

#[test]
fn play_1_3_2() {
	let time = expect!(
		v1_3_2::play::to_client::packet::UpdateTime,
		w(0x04).i64(6000)
	);
	assert_eq!(time.time, 6000);
	let event = expect!(
		v1_3_2::play::to_client::packet::WorldEvent,
		w(0x3d).i32(1001).i32(1).u8(64).i32(2).i32(0)
	);
	assert_eq!(event.effect_id, 1001);
	let settings = expect!(
		v1_3_2::play::to_server::packet::Settings,
		w(0xcc).str("en_US").i8(0).i8(8).u8(2)
	);
	assert_eq!(settings.locale, "en_US");
	let tile = expect!(
		v1_3_2::play::to_client::packet::TileEntityData,
		w(0x84)
			.i32(1)
			.i16(64)
			.i32(2)
			.u8(1)
			.i16(GZIP.len() as i16)
			.raw(GZIP)
	);
	assert_eq!(tile.nbt_data.as_deref(), Some(GZIP));
	let abilities = expect!(
		v1_3_2::play::to_client::packet::Abilities,
		w(0xca).i8(0x0d).i8(12).i8(25)
	);
	assert_eq!(abilities.flags, 0x0d);
	let spawn = expect!(
		v1_3_2::play::to_server::packet::ClientCommand,
		w(0xcd).i8(0)
	);
	assert_eq!(debug(&spawn), "InitialSpawn");
	absent!(v1_3_2::play::to_client::packet, 0x10, 0x3f, 0x02);
}

macro_rules! play_1_2 {
	($name:ident, $($client:ident)::+, $($server:ident)::+, $abilities:tt) => {
		#[test]
		fn $name() {
			let login = expect!(
				$($client)::+::Login,
				w(0x01).i32(17).str("").str("default").i32(1).i32(-1).u8(1).u8(0).u8(20)
			);
			assert_eq!((login.level_type.as_str(), login.game_mode, login.dimension), ("default", 1, -1));
			let request = expect!(
				$($server)::+::Login,
				w(0x01).i32(29).str("Notch").str("").i32(0).i32(0).u8(0).u8(0).u8(0)
			);
			assert_eq!((request.protocol_version, request.username.as_str()), (29, "Notch"));
			let hash = expect!($($client)::+::Handshake, w(0x02).str("-"));
			assert_eq!(hash.connection_hash, "-");
			let hello = expect!($($server)::+::SetProtocol, w(0x02).str("Notch;localhost:25565"));
			assert_eq!(hello.username_and_host, "Notch;localhost:25565");
			let chunk = expect!(
				$($client)::+::MapChunk,
				w(0x33).i32(1).i32(2).bool(true).u16(1).u16(0).i32(CHUNK.len() as i32).i32(0).raw(CHUNK)
			);
			assert_eq!((chunk.unused, chunk.compressed_chunk_data.as_slice()), (0, CHUNK));
			let alloc = expect!($($client)::+::PreChunk, w(0x32).i32(1).i32(2).bool(true));
			assert!(alloc.mode);
			let tile = expect!(
				$($client)::+::TileEntityData,
				w(0x84).i32(1).i16(64).i32(2).u8(1).i32(50).i32(0).i32(0)
			);
			assert_eq!((tile.action, tile.custom1), (1, 50));
			let destroy = expect!($($client)::+::EntityDestroy, w(0x1d).i32(5));
			assert_eq!(destroy, 5);
			let equipment = expect!($($client)::+::EntityEquipment, w(0x05).i32(1).i16(0).i16(267).i16(3));
			assert_eq!((equipment.item_id, equipment.item_damage), (267, 3));
			let player = expect!(
				$($client)::+::NamedEntitySpawn,
				w(0x14).i32(1).str("Notch").i32(0).i32(0).i32(0).i8(0).i8(0).i16(0)
			);
			assert_eq!(player.player_name, "Notch");
			let mob = expect!(
				$($client)::+::SpawnEntityLiving,
				w(0x18).i32(1).u8(50).i32(0).i32(0).i32(0).i8(1).i8(2).i8(3).u8(0x00).i8(0).u8(0x7f)
			);
			assert_eq!((mob.head_pitch, mob.metadata.len()), (3, 1));
			let explosion = expect!(
				$($client)::+::Explosion,
				w(0x3c).f64(0.5).f64(64.0).f64(0.5).f32(3.0).i32(1).i8(1).i8(0).i8(-1)
			);
			assert_eq!(explosion.affected_block_offsets[0].z, -1);
			let block = expect!($($client)::+::BlockChange, w(0x35).i32(1).u8(64).i32(2).u8(46).u8(0));
			assert_eq!(block.r#type, 46);
			let respawn = expect!($($server)::+::Respawn, w(0x09).i32(0).u8(1).u8(0).i16(256).str("default"));
			assert_eq!(respawn.world_height, 256);
			play_1_2!(@abilities $abilities, $($client)::+, $($server)::+);
			absent!($($client)::+, 0x10, 0x37, 0x38, 0x3e, 0xcb, 0xfc, 0xfd);
			absent!($($server)::+, 0xcb, 0xcc, 0xcd, 0xfc);
		}
	};
	(@abilities true, $($client:ident)::+, $($server:ident)::+) => {
		let abilities = expect!($($client)::+::Abilities, w(0xca).bool(false).bool(true).bool(true).bool(false));
		assert!(abilities.flying && abilities.can_fly && !abilities.instant_build);
		expect!($($server)::+::Abilities, w(0xca).bool(false).bool(false).bool(true).bool(true));
	};
	(@abilities false, $($client:ident)::+, $($server:ident)::+) => {
		absent!($($client)::+, 0xca);
		absent!($($server)::+, 0xca);
	};
}

play_1_2!(
	play_1_2_5,
	v1_2_5::play::to_client::packet,
	v1_2_5::play::to_server::packet,
	true
);
play_1_2!(
	play_1_2_3,
	v1_2_3::play::to_client::packet,
	v1_2_3::play::to_server::packet,
	false
);

#[test]
fn play_1_1() {
	let login = expect!(
		v1_1::play::to_client::packet::Login,
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
		(login.seed, login.level_type.as_str(), login.dimension),
		(-123, "default", 0)
	);
	let respawn = expect!(
		v1_1::play::to_server::packet::Respawn,
		w(0x09).i8(-1).u8(1).u8(0).i16(128).i64(-123).str("default")
	);
	assert_eq!((respawn.dimension, respawn.seed), (-1, -123));
	let chunk = expect!(
		v1_1::play::to_client::packet::MapChunk,
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
		(chunk.x, chunk.size_y, chunk.compressed_chunk_data.len()),
		(16, 127, CHUNK.len())
	);
	let blocks = expect!(
		v1_1::play::to_client::packet::MultiBlockChange,
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
		(blocks.block_ids.as_slice(), blocks.metadata.as_slice()),
		(&[1u8, 2][..], &[0u8, 3][..])
	);
	let brand = expect!(
		v1_1::play::to_server::packet::CustomPayload,
		w(0xfa).str("MC|Brand").i16(1).raw(b"x")
	);
	assert_eq!(brand.data, b"x");
	let hello = expect!(
		v1_1::play::to_server::packet::SetProtocol,
		w(0x02).str("Notch")
	);
	assert_eq!(hello.username, "Notch");
	absent!(v1_1::play::to_client::packet, 0x23, 0x84, 0xca);
	absent!(v1_1::play::to_server::packet, 0x1b);
}

#[test]
fn play_1_0_1() {
	let login = expect!(
		v1_0_1::play::to_client::packet::Login,
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
	assert_eq!((login.seed, login.game_mode, login.max_players), (5, 1, 8));
	let request = expect!(
		v1_0_1::play::to_server::packet::Login,
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
	assert_eq!(request.protocol_version, 22);
	let respawn = expect!(
		v1_0_1::play::to_client::packet::Respawn,
		w(0x09).i8(0).u8(1).u8(0).i16(128).i64(5)
	);
	assert_eq!(respawn.seed, 5);
	let stance = expect!(
		v1_0_1::play::to_server::packet::StanceUpdate,
		w(0x1b)
			.f32(0.0)
			.f32(1.0)
			.f32(2.0)
			.f32(3.0)
			.bool(false)
			.bool(true)
	);
	assert_eq!((stance.yaw, stance.unmount), (3.0, true));
	absent!(v1_0_1::play::to_client::packet, 0xfa, 0x23);
	absent!(v1_0_1::play::to_server::packet, 0xfa);
}
