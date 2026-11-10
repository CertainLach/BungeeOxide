use minecraft_versions::gen::convert::{convert, index_of, packet_name, v26_1, Root};
use minecraft_versions::pds::Output;
use minecraft_versions::Session;

fn encode(p: &v26_1::login_toClient_packet::Ty) -> Vec<u8> {
	let mut o = Output::new();
	v26_1::login_toClient_packet::serialize(p, &mut o).unwrap();
	o.into_inner()
}

fn replies(p: v26_1::login_toClient_packet::Ty) -> Vec<(Root, usize, Vec<u8>)> {
	let mut cx = Session::new();
	let out = convert(
		Root::LoginToClient,
		index_of("26.1").unwrap(),
		index_of("1.8").unwrap(),
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

fn to_server(root: Root, version: usize, data: &[u8]) -> Option<&'static str> {
	let sv = index_of("26.1").unwrap();
	let mut cx = Session::new();
	let up = convert(root, version, sv, data, &mut cx).ok()?;
	packet_name(Root::LoginToServer, sv, up.first()?)
}

#[test]
fn login_plugin_requests_are_answered_for_old_logic() {
	let out = replies(v26_1::login_toClient_packet::Ty::login_plugin_request(
		v26_1::login_toClient_packet_login_plugin_request::Ty {
			messageId: 7,
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
	let out = replies(v26_1::login_toClient_packet::Ty::cookie_request(
		"minecraft:probe".to_owned(),
	));
	assert_eq!(out.len(), 1);
	let (root, version, data) = &out[0];
	assert_eq!(*root, Root::LoginToServer);
	assert_eq!(to_server(*root, *version, data), Some("cookie_response"));
}
