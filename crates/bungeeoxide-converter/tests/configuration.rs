use bungeeoxide_converter::gen::convert::packet_name;
use bungeeoxide_converter::{
	configuration_packets, convert, Root, Session, Version, CONFIGURATION,
};

fn references(data: &[u8]) -> Vec<String> {
	let marker = b"#minecraft:";
	let mut out = Vec::new();
	let mut i = 0;
	while i + marker.len() <= data.len() {
		if &data[i..i + marker.len()] != marker {
			i += 1;
			continue;
		}
		let start = i + marker.len();
		let end = data[start..]
			.iter()
			.position(|b| !(b.is_ascii_lowercase() || b.is_ascii_digit() || b"_/.-".contains(b)))
			.map_or(data.len(), |n| start + n);
		if i >= 2 && data[i - 2] == 0 && usize::from(data[i - 1]) == end - i {
			out.push(String::from_utf8_lossy(&data[start..end]).into_owned());
		}
		i = end;
	}
	out
}

fn declared(tags: &[u8], name: &str) -> bool {
	[format!("minecraft:{name}"), name.to_owned()]
		.iter()
		.any(|n| {
			let mut needle = vec![n.len() as u8];
			needle.extend_from_slice(n.as_bytes());
			tags.windows(needle.len()).any(|w| w == needle)
		})
}

#[test]
fn synthesized_registries_only_reference_sent_tags() {
	let base = CONFIGURATION;
	let mut problems = Vec::new();
	for &version in Version::span(base..) {
		let mut cx = Session::new();
		let mut packets = Vec::new();
		for p in configuration_packets(&mut cx).unwrap() {
			packets
				.extend(convert(Root::ConfigurationToClient, base, version, &p, &mut cx).unwrap());
		}
		let mut refs = Vec::new();
		let mut tags = Vec::new();
		for p in &packets {
			match packet_name(Root::ConfigurationToClient, version, p) {
				Some("registry_data") => refs.extend(references(p)),
				Some("tags") => tags.extend_from_slice(p),
				_ => {}
			}
		}
		refs.sort();
		refs.dedup();
		let missing: Vec<_> = refs.into_iter().filter(|r| !declared(&tags, r)).collect();
		if !missing.is_empty() {
			problems.push(format!("{}: {}", version, missing.join(" ")));
		}
	}
	assert!(
		problems.is_empty(),
		"registry data references tags never sent:\n{}",
		problems.join("\n")
	);
}
