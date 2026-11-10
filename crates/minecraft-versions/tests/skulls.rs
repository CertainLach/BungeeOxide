use minecraft_versions::gen::convert::{convert, index_of, v1_15_2, v1_16, Root};
use minecraft_versions::pds::{Input, Output};
use minecraft_versions::Session;
use nbt::{Entry, Payload};

fn owner_id<'a>(root: &'a Entry, key: &str) -> &'a Entry {
	let Payload::compound(entries) = &root.value else {
		panic!("compound")
	};
	let owner = entries.iter().find(|e| e.name == key).expect("owner");
	let Payload::compound(owner) = &owner.value else {
		panic!("Owner compound")
	};
	owner.iter().find(|e| e.name == "Id").expect("Id")
}

#[test]
fn skull_owner_ids_keep_matching_nbt_types() {
	let uuid = "83973d76-06af-3046-8341-c8574af3577b";
	let tag = Entry::new(
		"",
		Payload::compound(vec![
			Entry::new("id", Payload::string("minecraft:skull".to_owned())),
			Entry::new("x", Payload::int(1)),
			Entry::new("y", Payload::int(64)),
			Entry::new("z", Payload::int(2)),
			Entry::new(
				"Owner",
				Payload::compound(vec![
					Entry::new("Id", Payload::string(uuid.to_owned())),
					Entry::new("Name", Payload::string("CertainLach".to_owned())),
				]),
			),
		]),
	);
	let p = v1_15_2::play_toClient_packet::Ty::tile_entity_data(
		v1_15_2::play_toClient_packet_tile_entity_data::Ty {
			location: v1_15_2::position::Ty { x: 1, y: 64, z: 2 },
			action: 4,
			nbtData: Some(tag),
		},
	);
	let mut out = Output::new();
	v1_15_2::play_toClient_packet::serialize(&p, &mut out).unwrap();
	let (old, new) = (index_of("1.15.2").unwrap(), index_of("1.16").unwrap());
	let mut cx = Session::new();
	let up = convert(Root::PlayToClient, old, new, &out.into_inner(), &mut cx).unwrap();
	let v1_16::play_toClient_packet::Ty::tile_entity_data(t) =
		v1_16::play_toClient_packet::deserialize(&mut Input::new(&up[0])).unwrap()
	else {
		panic!("expected tile_entity_data");
	};
	let id = owner_id(t.nbtData.as_ref().unwrap(), "SkullOwner");
	assert_eq!(id.r#type, 11);
	assert!(matches!(id.value, Payload::intArray(_)));
	let down = convert(Root::PlayToClient, new, old, &up[0], &mut cx).unwrap();
	let v1_15_2::play_toClient_packet::Ty::tile_entity_data(t) =
		v1_15_2::play_toClient_packet::deserialize(&mut Input::new(&down[0])).unwrap()
	else {
		panic!("expected tile_entity_data");
	};
	let id = owner_id(t.nbtData.as_ref().unwrap(), "Owner");
	assert_eq!(id.r#type, 8);
	assert!(matches!(&id.value, Payload::string(s) if s == uuid));
}
