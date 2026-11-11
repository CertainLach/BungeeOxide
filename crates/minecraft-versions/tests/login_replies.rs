use minecraft_versions::gen::convert::{convert, packet_name, v26_1, Root};
use minecraft_versions::pds::Output;
use minecraft_versions::Session;
use minecraft_versions::Version;

fn encode(p: &v26_1::login_to_client_packet::Ty) -> Vec<u8> {
	let mut o = Output::new();
	v26_1::login_to_client_packet::serialize(p, &mut o).unwrap();
	o.into_inner()
}

fn replies(p: v26_1::login_to_client_packet::Ty) -> Vec<(Root, Version, Vec<u8>)> {
	let mut cx = Session::new();
	let out = convert(
		Root::LoginToClient,
		Version::V26_1,
		Version::V1_8,
		&encode(&p),
		&mut cx,
	)
	.unwrap();
	assert!(out.is_empty());
	cx.take_outgoing()
		.into_iter()
		.map(|o| (o.root, o.version, o.data))
		.collect()
}

fn to_server(root: Root, version: Version, data: &[u8]) -> Option<&'static str> {
	let sv = Version::V26_1;
	let mut cx = Session::new();
	let up = convert(root, version, sv, data, &mut cx).ok()?;
	packet_name(Root::LoginToServer, sv, up.first()?)
}

#[test]
fn login_plugin_requests_are_answered_for_old_logic() {
	let out = replies(v26_1::login_to_client_packet::Ty::LoginPluginRequest(
		v26_1::login_to_client_packet_login_plugin_request::Ty {
			message_id: 7,
			channel: "hypixel:probe".to_owned(),
			data: vec![1, 2, 3],
		},
	));
	assert_eq!(out.len(), 1);
	let (root, version, data) = &out[0];
	assert_eq!(*root, Root::LoginToServer);
	assert_eq!(
		to_server(*root, *version, data),
		Some("login_plugin_response")
	);
}

#[test]
fn cookie_requests_are_answered_for_old_logic() {
	let out = replies(v26_1::login_to_client_packet::Ty::CookieRequest(
		"minecraft:probe".to_owned(),
	));
	assert_eq!(out.len(), 1);
	let (root, version, data) = &out[0];
	assert_eq!(*root, Root::LoginToServer);
	assert_eq!(to_server(*root, *version, data), Some("cookie_response"));
}
