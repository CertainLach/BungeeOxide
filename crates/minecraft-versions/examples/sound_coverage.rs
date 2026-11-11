use std::collections::BTreeMap;

use minecraft_versions::gen::convert::{
	convert, index_of, packet_name, v1_8, v26_1, Root, Version, VERSIONS,
};
use minecraft_versions::pds::Input;
use minecraft_versions::pds::Output;
use minecraft_versions::registry;
use minecraft_versions::Session;

fn main() {
	let to = std::env::args().nth(1).unwrap_or_else(|| "1.8".to_owned());
	let (top, bottom) = (
		index_of("26.1").unwrap(),
		index_of(&to).expect("a known version"),
	);
	let names = registry::names(Version::V26_1, "sounds").unwrap();
	let mut lost: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
	let mut arrived = 0;
	let mut unknown: Vec<(String, &str)> = Vec::new();
	let total = names.names.len();
	for (&id, name) in &names.names {
		let p = v26_1::play_to_client_packet::Ty::SoundEffect(
			v26_1::play_to_client_packet_sound_effect::Ty {
				sound: v26_1::item_sound_holder::Ty::Id(id as i32 - 1),
				sound_category: v26_1::sound_source::Ty::Hostile,
				x: 0,
				y: 512,
				z: 0,
				volume: 1.0,
				pitch: 1.0,
				seed: 0,
			},
		);
		let mut out = Output::new();
		v26_1::play_to_client_packet::serialize(&p, &mut out).unwrap();
		let mut packets = vec![out.into_inner()];
		let mut cx = Session::new();
		let mut at = top;
		while at > bottom && !packets.is_empty() {
			let mut next = Vec::new();
			for p in &packets {
				next.extend(
					convert(Root::PlayToClient, at, at - 1, p, &mut cx).unwrap_or_default(),
				);
			}
			packets = next
				.into_iter()
				.filter(|p| {
					packet_name(Root::PlayToClient, at - 1, p).is_some_and(|n| n.contains("sound"))
				})
				.collect();
			at -= 1;
		}
		if packets.is_empty() {
			lost.entry(at + 1).or_default().push(name);
		} else {
			arrived += 1;
			if bottom == index_of("1.8").unwrap() {
				for p in &packets {
					if let Ok(v1_8::play_to_client_packet::Ty::NamedSoundEffect(s)) =
						v1_8::play_to_client_packet::deserialize(&mut Input::new(p))
					{
						if registry::names(Version::V1_8, "sounds")
							.and_then(|n| n.id(&s.sound_name))
							.is_none()
						{
							unknown.push((s.sound_name, name));
						}
					}
				}
			}
		}
	}
	println!(
		"{arrived} of {total} 26.1 sounds reach {to}, {} of them under names {to} does not know",
		unknown.len()
	);
	for (got, from) in unknown.iter().take(12) {
		println!("  {from} arrives as {got}");
	}
	for (at, names) in lost {
		println!(
			"{:>4} lost at {} -> {}: {}",
			names.len(),
			VERSIONS[at],
			VERSIONS[at - 1],
			names.iter().take(6).copied().collect::<Vec<_>>().join(" ")
		);
	}
}
