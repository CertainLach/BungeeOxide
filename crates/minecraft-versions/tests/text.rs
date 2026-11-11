use minecraft_versions::gen::convert::packet_name;
use minecraft_versions::Version;
use minecraft_versions::{convert, Root, Session};

fn chat_1_8(json: &str) -> Vec<u8> {
	let mut out = vec![0x02];
	let mut len = json.len();
	loop {
		if len < 0x80 {
			out.push(len as u8);
			break;
		}
		out.push((len as u8 & 0x7f) | 0x80);
		len >>= 7;
	}
	out.extend_from_slice(json.as_bytes());
	out.push(0);
	out
}

fn contains(haystack: &[u8], needle: &str) -> bool {
	haystack
		.windows(needle.len())
		.any(|w| w == needle.as_bytes())
}

#[test]
fn legacy_hover_events_reach_26_1() {
	let json = r#"{"translate":"chat.type.text","with":[{"insertion":"CertainLach","clickEvent":{"action":"suggest_command","value":"/msg CertainLach "},"hoverEvent":{"action":"show_entity","value":{"text":"{name:\"CertainLach\",id:\"83973d76-06af-3046-8341-c8574af3577b\"}"}},"text":"CertainLach"},{"text":"the","hoverEvent":{"action":"show_item","value":{"text":"{id:\"minecraft:stone\",Count:3b}"}}}]}"#;
	let (from, to) = (Version::V1_8, Version::V26_1);
	let out = convert(
		Root::PlayToClient,
		from,
		to,
		&chat_1_8(json),
		&mut Session::new(),
	)
	.unwrap();
	assert_eq!(out.len(), 1);
	let p = &out[0];
	assert_eq!(packet_name(Root::PlayToClient, to, p), Some("system_chat"));
	for key in [
		"hover_event",
		"show_entity",
		"uuid",
		"83973d76-06af-3046-8341-c8574af3577b",
		"minecraft:player",
		"show_item",
		"minecraft:stone",
		"count",
	] {
		assert!(contains(p, key), "missing {key}");
	}
	assert!(!contains(p, "value"), "a legacy hover value is left");
}
