use minecraft_versions::gen::convert::packet_name;
use minecraft_versions::{convert, index_of, Root, Session};

fn dump() -> Vec<Vec<u8>> {
	let bytes = std::fs::read(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/tests/data/1.8-toClient.bin"
	))
	.unwrap();
	let mut out = Vec::new();
	let mut i = 0;
	while i + 4 <= bytes.len() {
		let len = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
		out.push(bytes[i + 4..i + 4 + len].to_vec());
		i += 4 + len;
	}
	out
}

#[test]
fn chunks_survive_1_9_and_back() {
	let (v8, v9) = (index_of("1.8").unwrap(), index_of("1.9").unwrap());
	let mut cx = Session::new();
	let mut checked = 0;
	for p in dump() {
		if !matches!(
			packet_name(Root::PlayToClient, v8, &p),
			Some("map_chunk" | "map_chunk_bulk")
		) {
			continue;
		}
		for up in convert(Root::PlayToClient, v8, v9, &p, &mut cx).unwrap() {
			let down = convert(Root::PlayToClient, v9, v8, &up, &mut cx).unwrap();
			assert_eq!(down.len(), 1);
			assert_eq!(
				packet_name(Root::PlayToClient, v8, &down[0]),
				Some("map_chunk")
			);
			let again = convert(Root::PlayToClient, v8, v9, &down[0], &mut cx).unwrap();
			assert_eq!(again, vec![up]);
			checked += 1;
		}
	}
	assert!(checked > 0);
}
