use bungeeoxide_converter::gen::convert::{convert, v1_15_2, v1_16, Root};
use bungeeoxide_converter::pds::{Input, Output};
use bungeeoxide_converter::Session;
use bungeeoxide_converter::Version;
use bungeeoxide_nbt::{Entry, Payload};

fn owner_id<'a>(root: &'a Entry, key: &str) -> &'a Entry {
	let Payload::Compound(entries) = &root.value else {
		panic!("compound")
	};
	let owner = entries.iter().find(|e| e.name == key).expect("owner");
	let Payload::Compound(owner) = &owner.value else {
		panic!("Owner compound")
	};
	owner.iter().find(|e| e.name == "Id").expect("Id")
}

#[test]
fn skull_owner_ids_keep_matching_nbt_types() {
	let uuid = "83973d76-06af-3046-8341-c8574af3577b";
	let tag = Entry::new(
		"",
		Payload::Compound(vec![
			Entry::new("id", Payload::String("minecraft:skull".to_owned())),
			Entry::new("x", Payload::Int(1)),
			Entry::new("y", Payload::Int(64)),
			Entry::new("z", Payload::Int(2)),
			Entry::new(
				"Owner",
				Payload::Compound(vec![
					Entry::new("Id", Payload::String(uuid.to_owned())),
					Entry::new("Name", Payload::String("CertainLach".to_owned())),
				]),
			),
		]),
	);
	let p = v1_15_2::play_to_client_packet::Ty::TileEntityData(
		v1_15_2::play_to_client_packet_tile_entity_data::Ty {
			location: v1_15_2::position::Ty { x: 1, y: 64, z: 2 },
			action: 4,
			nbt_data: Some(tag),
		},
	);
	let mut out = Output::new();
	v1_15_2::play_to_client_packet::serialize(&p, &mut out).unwrap();
	let (old, new) = (Version::V1_15_2, Version::V1_16);
	let mut cx = Session::new();
	let up = convert(Root::PlayToClient, old, new, &out.into_inner(), &mut cx).unwrap();
	let v1_16::play_to_client_packet::Ty::TileEntityData(t) =
		v1_16::play_to_client_packet::deserialize(&mut Input::new(&up[0])).unwrap()
	else {
		panic!("expected tile_entity_data");
	};
	let id = owner_id(t.nbt_data.as_ref().unwrap(), "SkullOwner");
	assert_eq!(id.r#type, 11);
	assert!(matches!(id.value, Payload::IntArray(_)));
	let down = convert(Root::PlayToClient, new, old, &up[0], &mut cx).unwrap();
	let v1_15_2::play_to_client_packet::Ty::TileEntityData(t) =
		v1_15_2::play_to_client_packet::deserialize(&mut Input::new(&down[0])).unwrap()
	else {
		panic!("expected tile_entity_data");
	};
	let id = owner_id(t.nbt_data.as_ref().unwrap(), "Owner");
	assert_eq!(id.r#type, 8);
	assert!(matches!(&id.value, Payload::String(s) if s == uuid));
}
