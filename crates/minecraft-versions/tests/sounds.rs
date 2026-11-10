use minecraft_versions::gen::convert::{
	convert, index_of, packet_name, v1_12_2, v1_8, Root, Version,
};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::registry;
use minecraft_versions::Session;

fn encode<T>(v: &T, ser: fn(&T, &mut Output) -> minecraft_versions::pds::Result<()>) -> Vec<u8> {
	let mut o = Output::new();
	ser(v, &mut o).unwrap();
	o.into_inner()
}

fn contains(haystack: &[u8], needle: &str) -> bool {
	haystack
		.windows(needle.len())
		.any(|w| w == needle.as_bytes())
}

#[test]
fn numeric_sounds_reach_1_8_by_name() {
	let id = registry::names(Version::V1_12_2, "sounds")
		.unwrap()
		.id("entity.zombie.ambient")
		.unwrap();
	let p = v1_12_2::play_toClient_packet::Ty::sound_effect(
		v1_12_2::play_toClient_packet_sound_effect::Ty {
			soundId: id as i32,
			soundCategory: 5,
			x: 80,
			y: 512,
			z: -40,
			volume: 1.0,
			pitch: 1.0,
		},
	);
	let data = encode(&p, v1_12_2::play_toClient_packet::serialize);
	let out = convert(
		Root::PlayToClient,
		index_of("1.12.2").unwrap(),
		index_of("1.8").unwrap(),
		&data,
		&mut Session::new(),
	)
	.unwrap();
	assert_eq!(out.len(), 1);
	let v1_8::play_toClient_packet::Ty::named_sound_effect(s) =
		v1_8::play_toClient_packet::deserialize(&mut Input::new(&out[0])).unwrap()
	else {
		panic!("expected a named sound");
	};
	assert_eq!(s.soundName, "mob.zombie.say");
	assert_eq!((s.x, s.y, s.z, s.pitch), (80, 512, -40, 63));
}

#[test]
fn sounds_without_a_1_8_equivalent_are_dropped() {
	let id = registry::names(Version::V1_12_2, "sounds")
		.unwrap()
		.id("item.elytra.flying")
		.unwrap();
	let p = v1_12_2::play_toClient_packet::Ty::sound_effect(
		v1_12_2::play_toClient_packet_sound_effect::Ty {
			soundId: id as i32,
			soundCategory: 7,
			x: 0,
			y: 0,
			z: 0,
			volume: 1.0,
			pitch: 1.0,
		},
	);
	let data = encode(&p, v1_12_2::play_toClient_packet::serialize);
	let out = convert(
		Root::PlayToClient,
		index_of("1.12.2").unwrap(),
		index_of("1.8").unwrap(),
		&data,
		&mut Session::new(),
	)
	.unwrap();
	assert!(out.is_empty());
}

#[test]
fn legacy_sound_names_reach_26_1() {
	let p = v1_8::play_toClient_packet::Ty::named_sound_effect(
		v1_8::play_toClient_packet_named_sound_effect::Ty {
			soundName: "mob.zombie.say".to_owned(),
			x: 80,
			y: 512,
			z: -40,
			volume: 1.0,
			pitch: 63,
		},
	);
	let data = encode(&p, v1_8::play_toClient_packet::serialize);
	let to = index_of("26.1").unwrap();
	let out = convert(
		Root::PlayToClient,
		index_of("1.8").unwrap(),
		to,
		&data,
		&mut Session::new(),
	)
	.unwrap();
	assert_eq!(out.len(), 1);
	assert_eq!(
		packet_name(Root::PlayToClient, to, &out[0]),
		Some("sound_effect")
	);
	assert!(!contains(&out[0], "mob.zombie.say"));
}
