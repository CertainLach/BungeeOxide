use std::collections::HashMap;

use super::blocks::{
	new_blocks, new_items, old_blocks, old_items, raw_of_state, state_of_raw, FLAT, OLD,
};
use super::chunk::reset_world;
use super::entities::forget_flat;
use super::items::{legacy_of_item, slot_down, slot_up, trader_down, trader_up};
use super::text::{
	color_ordinal, json_to_legacy, last_color, legacy_to_json, strip_colors, truncate_legacy,
};
use super::wire::{put as put_value, put_string, put_varint, Reader};
use super::world::{item_of_legacy, sound_name_down};
use crate::gen::convert::{v1_12_2 as a, v1_13 as b, Root};
use crate::pds::convert::{Error, Result};
use crate::pds::Output;
use crate::session::Session;

type Client12 = a::play_to_client_packet::Ty;
type Client13 = b::play_to_client_packet::Ty;
type Server12 = a::play_to_server_packet::Ty;
type Server13 = b::play_to_server_packet::Ty;

#[derive(Default)]
struct Flat {
	dimension: i32,
	tab_transaction: i32,
	tab_input: String,
}

#[derive(Default)]
struct Legacy {
	dimension: i32,
	commands: Vec<String>,
	usernames: HashMap<u128, String>,
	request: Option<(i32, String, bool)>,
	counter: i32,
}

fn send<T>(
	cx: &mut Session,
	root: Root,
	version: crate::gen::convert::Version,
	packet: &T,
	f: fn(&T, &mut Output) -> crate::pds::Result<()>,
) {
	let mut o = Output::new();
	if f(packet, &mut o).is_ok() {
		cx.send(root, version, o.into_inner());
	}
}

const BLOCK_TAGS: &[(&str, &[&str])] = &[
	(
		"acacia_logs",
		&[
			"acacia_log",
			"stripped_acacia_log",
			"acacia_wood",
			"stripped_acacia_wood",
		],
	),
	("anvil", &["anvil", "chipped_anvil", "damaged_anvil"]),
	(
		"banners",
		&[
			"white_banner",
			"orange_banner",
			"magenta_banner",
			"light_blue_banner",
			"yellow_banner",
			"lime_banner",
			"pink_banner",
			"gray_banner",
			"light_gray_banner",
			"cyan_banner",
			"purple_banner",
			"blue_banner",
			"brown_banner",
			"green_banner",
			"red_banner",
			"black_banner",
			"white_wall_banner",
			"orange_wall_banner",
			"magenta_wall_banner",
			"light_blue_wall_banner",
			"yellow_wall_banner",
			"lime_wall_banner",
			"pink_wall_banner",
			"gray_wall_banner",
			"light_gray_wall_banner",
			"cyan_wall_banner",
			"purple_wall_banner",
			"blue_wall_banner",
			"brown_wall_banner",
			"green_wall_banner",
			"red_wall_banner",
			"black_wall_banner",
		],
	),
	(
		"birch_logs",
		&[
			"birch_log",
			"stripped_birch_log",
			"birch_wood",
			"stripped_birch_wood",
		],
	),
	(
		"buttons",
		&[
			"stone_button",
			"oak_button",
			"spruce_button",
			"birch_button",
			"jungle_button",
			"acacia_button",
			"dark_oak_button",
		],
	),
	(
		"carpets",
		&[
			"white_carpet",
			"orange_carpet",
			"magenta_carpet",
			"light_blue_carpet",
			"yellow_carpet",
			"lime_carpet",
			"pink_carpet",
			"gray_carpet",
			"light_gray_carpet",
			"cyan_carpet",
			"purple_carpet",
			"blue_carpet",
			"brown_carpet",
			"green_carpet",
			"red_carpet",
			"black_carpet",
		],
	),
	(
		"coral_blocks",
		&[
			"tube_coral_block",
			"brain_coral_block",
			"bubble_coral_block",
			"fire_coral_block",
			"horn_coral_block",
		],
	),
	(
		"coral_plants",
		&[
			"tube_coral",
			"brain_coral",
			"bubble_coral",
			"fire_coral",
			"horn_coral",
		],
	),
	(
		"corals",
		&[
			"tube_coral",
			"brain_coral",
			"bubble_coral",
			"fire_coral",
			"horn_coral",
			"tube_coral_fan",
			"brain_coral_fan",
			"bubble_coral_fan",
			"fire_coral_fan",
			"horn_coral_fan",
		],
	),
	(
		"dark_oak_logs",
		&[
			"dark_oak_log",
			"stripped_dark_oak_log",
			"dark_oak_wood",
			"stripped_dark_oak_wood",
		],
	),
	(
		"doors",
		&[
			"oak_door",
			"iron_door",
			"spruce_door",
			"birch_door",
			"jungle_door",
			"acacia_door",
			"dark_oak_door",
		],
	),
	(
		"enderman_holdable",
		&[
			"grass_block",
			"dirt",
			"coarse_dirt",
			"podzol",
			"sand",
			"red_sand",
			"gravel",
			"dandelion",
			"poppy",
			"blue_orchid",
			"allium",
			"azure_bluet",
			"red_tulip",
			"orange_tulip",
			"white_tulip",
			"pink_tulip",
			"oxeye_daisy",
			"brown_mushroom",
			"red_mushroom",
			"tnt",
			"cactus",
			"clay",
			"pumpkin",
			"netherrack",
			"carved_pumpkin",
			"melon",
			"mycelium",
		],
	),
	(
		"flower_pots",
		&[
			"flower_pot",
			"potted_oak_sapling",
			"potted_spruce_sapling",
			"potted_birch_sapling",
			"potted_jungle_sapling",
			"potted_acacia_sapling",
			"potted_dark_oak_sapling",
			"potted_fern",
			"potted_dandelion",
			"potted_poppy",
			"potted_blue_orchid",
			"potted_allium",
			"potted_azure_bluet",
			"potted_red_tulip",
			"potted_orange_tulip",
			"potted_white_tulip",
			"potted_pink_tulip",
			"potted_oxeye_daisy",
			"potted_red_mushroom",
			"potted_brown_mushroom",
			"potted_dead_bush",
			"potted_cactus",
		],
	),
	("ice", &["ice", "packed_ice", "frosted_ice", "blue_ice"]),
	(
		"jungle_logs",
		&[
			"jungle_log",
			"stripped_jungle_log",
			"jungle_wood",
			"stripped_jungle_wood",
		],
	),
	(
		"leaves",
		&[
			"oak_leaves",
			"spruce_leaves",
			"birch_leaves",
			"jungle_leaves",
			"acacia_leaves",
			"dark_oak_leaves",
		],
	),
	(
		"logs",
		&[
			"oak_log",
			"spruce_log",
			"birch_log",
			"jungle_log",
			"acacia_log",
			"dark_oak_log",
			"stripped_spruce_log",
			"stripped_birch_log",
			"stripped_jungle_log",
			"stripped_acacia_log",
			"stripped_dark_oak_log",
			"stripped_oak_log",
			"oak_wood",
			"spruce_wood",
			"birch_wood",
			"jungle_wood",
			"acacia_wood",
			"dark_oak_wood",
			"stripped_oak_wood",
			"stripped_spruce_wood",
			"stripped_birch_wood",
			"stripped_jungle_wood",
			"stripped_acacia_wood",
			"stripped_dark_oak_wood",
		],
	),
	(
		"oak_logs",
		&[
			"oak_log",
			"stripped_oak_log",
			"oak_wood",
			"stripped_oak_wood",
		],
	),
	(
		"planks",
		&[
			"oak_planks",
			"spruce_planks",
			"birch_planks",
			"jungle_planks",
			"acacia_planks",
			"dark_oak_planks",
		],
	),
	(
		"rails",
		&["powered_rail", "detector_rail", "rail", "activator_rail"],
	),
	("sand", &["sand", "red_sand"]),
	(
		"saplings",
		&[
			"oak_sapling",
			"spruce_sapling",
			"birch_sapling",
			"jungle_sapling",
			"acacia_sapling",
			"dark_oak_sapling",
		],
	),
	(
		"slabs",
		&[
			"prismarine_slab",
			"prismarine_brick_slab",
			"dark_prismarine_slab",
			"oak_slab",
			"spruce_slab",
			"birch_slab",
			"jungle_slab",
			"acacia_slab",
			"dark_oak_slab",
			"stone_slab",
			"sandstone_slab",
			"petrified_oak_slab",
			"cobblestone_slab",
			"brick_slab",
			"stone_brick_slab",
			"nether_brick_slab",
			"quartz_slab",
			"red_sandstone_slab",
			"purpur_slab",
		],
	),
	(
		"spruce_logs",
		&[
			"spruce_log",
			"stripped_spruce_log",
			"spruce_wood",
			"stripped_spruce_wood",
		],
	),
	(
		"stairs",
		&[
			"oak_stairs",
			"cobblestone_stairs",
			"brick_stairs",
			"stone_brick_stairs",
			"nether_brick_stairs",
			"sandstone_stairs",
			"spruce_stairs",
			"birch_stairs",
			"jungle_stairs",
			"quartz_stairs",
			"acacia_stairs",
			"dark_oak_stairs",
			"prismarine_stairs",
			"prismarine_brick_stairs",
			"dark_prismarine_stairs",
			"red_sandstone_stairs",
			"purpur_stairs",
		],
	),
	(
		"stone_bricks",
		&[
			"stone_bricks",
			"mossy_stone_bricks",
			"cracked_stone_bricks",
			"chiseled_stone_bricks",
		],
	),
	(
		"trapdoors",
		&[
			"oak_trapdoor",
			"spruce_trapdoor",
			"birch_trapdoor",
			"jungle_trapdoor",
			"acacia_trapdoor",
			"dark_oak_trapdoor",
			"iron_trapdoor",
		],
	),
	(
		"underwater_bonemeals",
		&[
			"seagrass",
			"tube_coral",
			"brain_coral",
			"bubble_coral",
			"fire_coral",
			"horn_coral",
			"tube_coral_wall_fan",
			"brain_coral_wall_fan",
			"bubble_coral_wall_fan",
			"fire_coral_wall_fan",
			"horn_coral_wall_fan",
			"tube_coral_fan",
			"brain_coral_fan",
			"bubble_coral_fan",
			"fire_coral_fan",
			"horn_coral_fan",
		],
	),
	("valid_spawn", &["grass_block", "podzol"]),
	(
		"wall_corals",
		&[
			"tube_coral_wall_fan",
			"brain_coral_wall_fan",
			"bubble_coral_wall_fan",
			"fire_coral_wall_fan",
			"horn_coral_wall_fan",
		],
	),
	(
		"wooden_buttons",
		&[
			"oak_button",
			"spruce_button",
			"birch_button",
			"jungle_button",
			"acacia_button",
			"dark_oak_button",
		],
	),
	(
		"wooden_doors",
		&[
			"oak_door",
			"spruce_door",
			"birch_door",
			"jungle_door",
			"acacia_door",
			"dark_oak_door",
		],
	),
	(
		"wooden_pressure_plates",
		&[
			"oak_pressure_plate",
			"spruce_pressure_plate",
			"birch_pressure_plate",
			"jungle_pressure_plate",
			"acacia_pressure_plate",
			"dark_oak_pressure_plate",
		],
	),
	(
		"wooden_slabs",
		&[
			"oak_slab",
			"spruce_slab",
			"birch_slab",
			"jungle_slab",
			"acacia_slab",
			"dark_oak_slab",
		],
	),
	(
		"wooden_stairs",
		&[
			"oak_stairs",
			"spruce_stairs",
			"birch_stairs",
			"jungle_stairs",
			"acacia_stairs",
			"dark_oak_stairs",
		],
	),
	(
		"wooden_trapdoors",
		&[
			"oak_trapdoor",
			"spruce_trapdoor",
			"birch_trapdoor",
			"jungle_trapdoor",
			"acacia_trapdoor",
			"dark_oak_trapdoor",
		],
	),
	(
		"wool",
		&[
			"white_wool",
			"orange_wool",
			"magenta_wool",
			"light_blue_wool",
			"yellow_wool",
			"lime_wool",
			"pink_wool",
			"gray_wool",
			"light_gray_wool",
			"cyan_wool",
			"purple_wool",
			"blue_wool",
			"brown_wool",
			"green_wool",
			"red_wool",
			"black_wool",
		],
	),
];

const ITEM_TAGS: &[(&str, &[&str])] = &[
	(
		"acacia_logs",
		&[
			"acacia_log",
			"stripped_acacia_log",
			"stripped_acacia_wood",
			"acacia_wood",
		],
	),
	("anvil", &["anvil", "chipped_anvil", "damaged_anvil"]),
	(
		"banners",
		&[
			"white_banner",
			"orange_banner",
			"magenta_banner",
			"light_blue_banner",
			"yellow_banner",
			"lime_banner",
			"pink_banner",
			"gray_banner",
			"light_gray_banner",
			"cyan_banner",
			"purple_banner",
			"blue_banner",
			"brown_banner",
			"green_banner",
			"red_banner",
			"black_banner",
		],
	),
	(
		"birch_logs",
		&[
			"birch_log",
			"stripped_birch_log",
			"stripped_birch_wood",
			"birch_wood",
		],
	),
	(
		"boats",
		&[
			"oak_boat",
			"spruce_boat",
			"birch_boat",
			"jungle_boat",
			"acacia_boat",
			"dark_oak_boat",
		],
	),
	(
		"buttons",
		&[
			"stone_button",
			"oak_button",
			"spruce_button",
			"birch_button",
			"jungle_button",
			"acacia_button",
			"dark_oak_button",
		],
	),
	(
		"carpets",
		&[
			"white_carpet",
			"orange_carpet",
			"magenta_carpet",
			"light_blue_carpet",
			"yellow_carpet",
			"lime_carpet",
			"pink_carpet",
			"gray_carpet",
			"light_gray_carpet",
			"cyan_carpet",
			"purple_carpet",
			"blue_carpet",
			"brown_carpet",
			"green_carpet",
			"red_carpet",
			"black_carpet",
		],
	),
	(
		"dark_oak_logs",
		&[
			"dark_oak_log",
			"stripped_dark_oak_log",
			"stripped_dark_oak_wood",
			"dark_oak_wood",
		],
	),
	(
		"doors",
		&[
			"iron_door",
			"oak_door",
			"spruce_door",
			"birch_door",
			"jungle_door",
			"acacia_door",
			"dark_oak_door",
		],
	),
	(
		"fishes",
		&[
			"cod",
			"salmon",
			"tropical_fish",
			"pufferfish",
			"cooked_cod",
			"cooked_salmon",
		],
	),
	(
		"jungle_logs",
		&[
			"jungle_log",
			"stripped_jungle_log",
			"stripped_jungle_wood",
			"jungle_wood",
		],
	),
	(
		"leaves",
		&[
			"oak_leaves",
			"spruce_leaves",
			"birch_leaves",
			"jungle_leaves",
			"acacia_leaves",
			"dark_oak_leaves",
		],
	),
	(
		"logs",
		&[
			"oak_log",
			"spruce_log",
			"birch_log",
			"jungle_log",
			"acacia_log",
			"dark_oak_log",
			"stripped_oak_log",
			"stripped_spruce_log",
			"stripped_birch_log",
			"stripped_jungle_log",
			"stripped_acacia_log",
			"stripped_dark_oak_log",
			"stripped_oak_wood",
			"stripped_spruce_wood",
			"stripped_birch_wood",
			"stripped_jungle_wood",
			"stripped_acacia_wood",
			"stripped_dark_oak_wood",
			"oak_wood",
			"spruce_wood",
			"birch_wood",
			"jungle_wood",
			"acacia_wood",
			"dark_oak_wood",
		],
	),
	(
		"oak_logs",
		&[
			"oak_log",
			"stripped_oak_log",
			"stripped_oak_wood",
			"oak_wood",
		],
	),
	(
		"planks",
		&[
			"oak_planks",
			"spruce_planks",
			"birch_planks",
			"jungle_planks",
			"acacia_planks",
			"dark_oak_planks",
		],
	),
	(
		"rails",
		&["powered_rail", "detector_rail", "rail", "activator_rail"],
	),
	("sand", &["sand", "red_sand"]),
	(
		"saplings",
		&[
			"oak_sapling",
			"spruce_sapling",
			"birch_sapling",
			"jungle_sapling",
			"acacia_sapling",
			"dark_oak_sapling",
		],
	),
	(
		"slabs",
		&[
			"oak_slab",
			"spruce_slab",
			"birch_slab",
			"jungle_slab",
			"acacia_slab",
			"dark_oak_slab",
			"stone_slab",
			"sandstone_slab",
			"petrified_oak_slab",
			"cobblestone_slab",
			"brick_slab",
			"stone_brick_slab",
			"nether_brick_slab",
			"quartz_slab",
			"red_sandstone_slab",
			"purpur_slab",
			"prismarine_slab",
			"prismarine_brick_slab",
			"dark_prismarine_slab",
		],
	),
	(
		"spruce_logs",
		&[
			"spruce_log",
			"stripped_spruce_log",
			"stripped_spruce_wood",
			"spruce_wood",
		],
	),
	(
		"stairs",
		&[
			"purpur_stairs",
			"oak_stairs",
			"cobblestone_stairs",
			"brick_stairs",
			"stone_brick_stairs",
			"nether_brick_stairs",
			"sandstone_stairs",
			"spruce_stairs",
			"birch_stairs",
			"jungle_stairs",
			"quartz_stairs",
			"acacia_stairs",
			"dark_oak_stairs",
			"prismarine_stairs",
			"prismarine_brick_stairs",
			"dark_prismarine_stairs",
			"red_sandstone_stairs",
		],
	),
	(
		"stone_bricks",
		&[
			"stone_bricks",
			"mossy_stone_bricks",
			"cracked_stone_bricks",
			"chiseled_stone_bricks",
		],
	),
	(
		"trapdoors",
		&[
			"oak_trapdoor",
			"spruce_trapdoor",
			"birch_trapdoor",
			"jungle_trapdoor",
			"acacia_trapdoor",
			"dark_oak_trapdoor",
			"iron_trapdoor",
		],
	),
	(
		"wooden_buttons",
		&[
			"oak_button",
			"spruce_button",
			"birch_button",
			"jungle_button",
			"acacia_button",
			"dark_oak_button",
		],
	),
	(
		"wooden_doors",
		&[
			"oak_door",
			"spruce_door",
			"birch_door",
			"jungle_door",
			"acacia_door",
			"dark_oak_door",
		],
	),
	(
		"wooden_pressure_plates",
		&[
			"oak_pressure_plate",
			"spruce_pressure_plate",
			"birch_pressure_plate",
			"jungle_pressure_plate",
			"acacia_pressure_plate",
			"dark_oak_pressure_plate",
		],
	),
	(
		"wooden_slabs",
		&[
			"oak_slab",
			"spruce_slab",
			"birch_slab",
			"jungle_slab",
			"acacia_slab",
			"dark_oak_slab",
		],
	),
	(
		"wooden_stairs",
		&[
			"oak_stairs",
			"spruce_stairs",
			"birch_stairs",
			"jungle_stairs",
			"acacia_stairs",
			"dark_oak_stairs",
		],
	),
	(
		"wooden_trapdoors",
		&[
			"oak_trapdoor",
			"spruce_trapdoor",
			"birch_trapdoor",
			"jungle_trapdoor",
			"acacia_trapdoor",
			"dark_oak_trapdoor",
		],
	),
	(
		"wool",
		&[
			"white_wool",
			"orange_wool",
			"magenta_wool",
			"light_blue_wool",
			"yellow_wool",
			"lime_wool",
			"pink_wool",
			"gray_wool",
			"light_gray_wool",
			"cyan_wool",
			"purple_wool",
			"blue_wool",
			"brown_wool",
			"green_wool",
			"red_wool",
			"black_wool",
		],
	),
];

const FLUID_TAGS: &[(&str, &[i32])] = &[("lava", &[3, 4]), ("water", &[1, 2])];

type Tag = b::play_to_client_packet_tags_block_tags_items::Ty;

fn tag(name: &str, entries: Vec<i32>) -> Tag {
	Tag {
		tag_name: format!("minecraft:{name}"),
		entries,
	}
}

fn tags() -> b::play_to_client_packet_tags::Ty {
	let blocks = new_blocks();
	let items = new_items();
	b::play_to_client_packet_tags::Ty {
		block_tags: BLOCK_TAGS
			.iter()
			.map(|(n, names)| {
				tag(
					n,
					names
						.iter()
						.filter_map(|b| blocks.block(b))
						.map(|b| b.id as i32)
						.collect(),
				)
			})
			.collect(),
		item_tags: ITEM_TAGS
			.iter()
			.map(|(n, names)| {
				tag(
					n,
					names
						.iter()
						.filter_map(|i| items.id(i))
						.map(|i| i as i32)
						.collect(),
				)
			})
			.collect(),
		fluid_tags: FLUID_TAGS
			.iter()
			.map(|(n, ids)| tag(n, ids.to_vec()))
			.collect(),
	}
}

fn commands() -> b::play_to_client_packet_declare_commands::Ty {
	let flags = |suggestions: u8, command: u8, kind: u8| b::command_node_flags::Ty {
		unused: 0,
		has_custom_suggestions: suggestions,
		has_redirect_node: 0,
		has_command: command,
		command_node_type: kind,
	};
	let root = b::command_node::Ty {
		flags: flags(0, 0, 0),
		children: vec![1],
		redirect_node: b::command_node_redirect_node::Ty::Default(0),
		extra_node_data: b::command_node_extra_node_data::Ty::V0,
	};
	let args = b::command_node::Ty {
		flags: flags(1, 1, 2),
		children: vec![],
		redirect_node: b::command_node_redirect_node::Ty::Default(0),
		extra_node_data: b::command_node_extra_node_data::Ty::V2(b::command_node_extra_node_data_v2::Ty {
			name: "args".to_owned(),
			parser: "brigadier:string".to_owned(),
			properties: b::command_node_extra_node_data_v2_properties::Ty::BrigadierString(
				b::command_node_extra_node_data_v2_properties_brigadier_string::Ty::GreedyPhrase,
			),
			suggestion_type: b::command_node_extra_node_data_v2_suggestion_type::Ty::V1("minecraft:ask_server".to_owned()),
		}),
	};
	b::play_to_client_packet_declare_commands::Ty {
		nodes: vec![root, args],
		root_index: 0,
	}
}

pub fn flattening_login_1_13(
	p: a::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	cx.state::<Flat>().dimension = p.dimension;
	reset_world(cx, p.dimension);
	forget_flat(cx);
	Ok(vec![
		Client13::Login(p),
		Client13::DeclareCommands(commands()),
		Client13::Tags(tags()),
	])
}

pub fn flattening_respawn_1_13(
	p: a::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let flat = cx.state::<Flat>();
	if flat.dimension != p.dimension {
		flat.dimension = p.dimension;
		reset_world(cx, p.dimension);
		forget_flat(cx);
	}
	Ok(vec![Client13::Respawn(p)])
}

pub fn flattening_login_1_12_2(
	p: b::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client12>> {
	let legacy = cx.state::<Legacy>();
	legacy.dimension = p.dimension;
	legacy.commands.clear();
	reset_world(cx, p.dimension);
	forget_flat(cx);
	Ok(vec![Client12::Login(p)])
}

pub fn flattening_respawn_1_12_2(
	p: b::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client12>> {
	let legacy = cx.state::<Legacy>();
	if legacy.dimension != p.dimension {
		legacy.dimension = p.dimension;
		reset_world(cx, p.dimension);
		forget_flat(cx);
	}
	Ok(vec![Client12::Respawn(p)])
}

pub fn flattening_login_plugin_request_1_12_2(
	p: b::login_to_client_packet_login_plugin_request::Ty,
	cx: &mut Session,
) -> Result<Vec<a::login_to_client_packet::Ty>> {
	let answer = b::login_to_server_packet::Ty::LoginPluginResponse(
		b::login_to_server_packet_login_plugin_response::Ty {
			message_id: p.message_id,
			data: None,
		},
	);
	send(
		cx,
		Root::LoginToServer,
		FLAT,
		&answer,
		b::login_to_server_packet::serialize,
	);
	Ok(vec![])
}

pub fn flattening_declare_commands_1_12_2(
	p: b::play_to_client_packet_declare_commands::Ty,
	cx: &mut Session,
) -> Result<Vec<Client12>> {
	let root = p.nodes.get(p.root_index.max(0) as usize);
	let commands = root
		.map(|r| {
			r.children
				.iter()
				.filter_map(|c| match &p.nodes.get(*c as usize)?.extra_node_data {
					b::command_node_extra_node_data::Ty::V1(name) => Some(format!("/{name}")),
					_ => None,
				})
				.collect()
		})
		.unwrap_or_default();
	cx.state::<Legacy>().commands = commands;
	Ok(vec![])
}

pub fn flattening_player_uuid_1_12_2(
	src: &b::play_to_client_packet_player_info_data_items::Ty,
	cx: &mut Session,
) -> Result<u128> {
	use b::play_to_client_packet_player_info_data_items_anon::Ty as Entry;
	let names = &mut cx.state::<Legacy>().usernames;
	match &src.anon {
		Entry::AddPlayer(add) => {
			names.insert(src.uuid, add.name.clone());
		}
		Entry::RemovePlayer => {
			names.remove(&src.uuid);
		}
		_ => {}
	}
	Ok(src.uuid)
}

pub fn flattening_server_info_1_13(
	p: String,
	_cx: &mut Session,
) -> Result<Vec<b::status_to_client_packet::Ty>> {
	Ok(vec![b::status_to_client_packet::Ty::ServerInfo(
		favicon_without_newlines(&p),
	)])
}

fn favicon_without_newlines(json: &str) -> String {
	let Some(key) = json.find("\"favicon\"") else {
		return json.to_owned();
	};
	let Some(open) = json[key + 9..].find('"').map(|i| key + 9 + i + 1) else {
		return json.to_owned();
	};
	let mut escaped = false;
	let end = json[open..]
		.char_indices()
		.find(|(_, c)| {
			let quote = *c == '"' && !escaped;
			escaped = *c == '\\' && !escaped;
			quote
		})
		.map_or(json.len(), |(i, _)| open + i);
	format!(
		"{}{}{}",
		&json[..open],
		json[open..end].replace("\\n", "").replace('\n', ""),
		&json[end..]
	)
}

const SOURCES: [&str; 10] = [
	"master", "music", "record", "weather", "block", "hostile", "neutral", "player", "ambient",
	"voice",
];

fn stop_sound_up(data: &[u8]) -> Result<b::play_to_client_packet_stop_sound::Ty> {
	use b::play_to_client_packet_stop_sound::Ty as Stop;
	let mut r = Reader::new(data);
	let source = r.string()?;
	let sound = r.string()?;
	let source =
		(!source.is_empty()).then(|| SOURCES.iter().position(|s| *s == source).unwrap_or(0) as i32);
	Ok(match (source, sound.is_empty()) {
		(None, true) => Stop::Default(0),
		(Some(s), true) => Stop::V1(s),
		(None, false) => Stop::V2(sound),
		(Some(source), false) => {
			Stop::V3(b::play_to_client_packet_stop_sound_v3::Ty { source, sound })
		}
	})
}

pub fn flattening_stop_sound_1_12_2(
	p: b::play_to_client_packet_stop_sound::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client12>> {
	use b::play_to_client_packet_stop_sound::Ty as Stop;
	let source = |s: i32| {
		SOURCES
			.get(s.max(0) as usize)
			.copied()
			.unwrap_or("master")
			.to_owned()
	};
	let (source, sound) = match p {
		Stop::V1(s) => (source(s), String::new()),
		Stop::V2(sound) => (String::new(), sound_name_down(&sound)),
		Stop::V3(v) => (source(v.source), sound_name_down(&v.sound)),
		Stop::Default(_) => (String::new(), String::new()),
	};
	let mut data = Vec::new();
	put_string(&mut data, &source);
	put_string(&mut data, &sound);
	Ok(vec![Client12::CustomPayload(
		a::play_to_client_packet_custom_payload::Ty {
			channel: "MC|StopSound".to_owned(),
			data,
		},
	)])
}

const CHANNELS: [(&str, &str); 12] = [
	("MC|TrList", "minecraft:trader_list"),
	("MC|Brand", "minecraft:brand"),
	("MC|BOpen", "minecraft:book_open"),
	("MC|DebugPath", "minecraft:debug/paths"),
	(
		"MC|DebugNeighborsUpdate",
		"minecraft:debug/neighbors_update",
	),
	("REGISTER", "minecraft:register"),
	("UNREGISTER", "minecraft:unregister"),
	("BungeeCord", "bungeecord:main"),
	("WDL|INIT", "wdl:init"),
	("WDL|CONTROL", "wdl:control"),
	("WDL|REQUEST", "wdl:request"),
	("FML|HS", "fml:hs"),
];

fn valid_key(s: &str) -> Option<String> {
	let (ns, path) = s.split_once(':').unwrap_or(("minecraft", s));
	let ok = |s: &str, slash: bool| {
		!s.is_empty()
			&& s.chars().all(|c| {
				matches!(c, 'a'..='z' | '0'..='9' | '_' | '.' | '-') || (slash && c == '/')
			})
	};
	(ok(ns, false) && ok(path, true)).then(|| format!("{ns}:{path}"))
}

fn channel_up(old: &str) -> Option<String> {
	if old == "bungeecord:main" {
		return None;
	}
	if old == "FML|MP" {
		return Some("fml:mp".to_owned());
	}
	match CHANNELS.iter().find(|(o, _)| *o == old) {
		Some((_, new)) => Some((*new).to_owned()),
		None => valid_key(old),
	}
}

fn channel_down(new: &str) -> Option<String> {
	let new = valid_key(new)?;
	if new == "fml:mp" {
		return Some("FML|MP".to_owned());
	}
	Some(match CHANNELS.iter().find(|(_, n)| *n == new) {
		Some((old, _)) => (*old).to_owned(),
		None => new.chars().take(20).collect(),
	})
}

fn channel_list(data: &[u8], f: fn(&str) -> Option<String>) -> Option<Vec<u8>> {
	let text = String::from_utf8_lossy(data);
	let names: Vec<String> = text
		.split('\0')
		.filter(|s| !s.is_empty())
		.filter_map(f)
		.collect();
	(!names.is_empty()).then(|| names.join("\0").into_bytes())
}

fn payload_up(channel: &str, data: Vec<u8>) -> Result<Option<(String, Vec<u8>)>> {
	let Some(new) = channel_up(channel) else {
		return Ok(None);
	};
	let data = match new.as_str() {
		"minecraft:register" | "minecraft:unregister" => match channel_list(&data, channel_up) {
			Some(d) => d,
			None => return Ok(None),
		},
		_ => data,
	};
	Ok(Some((new, data)))
}

fn payload_down(channel: &str, data: Vec<u8>) -> Result<Option<(String, Vec<u8>)>> {
	let Some(old) = channel_down(channel) else {
		return Ok(None);
	};
	let data = match old.as_str() {
		"REGISTER" | "UNREGISTER" => channel_list(&data, channel_down).unwrap_or_default(),
		_ => data,
	};
	Ok(Some((old, data)))
}

pub fn flattening_custom_payload_1_13(
	p: a::play_to_client_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client13>> {
	match p.channel.as_str() {
		"MC|StopSound" => return Ok(vec![Client13::StopSound(stop_sound_up(&p.data)?)]),
		"MC|TrList" => {
			return Ok(vec![Client13::CustomPayload(
				b::play_to_client_packet_custom_payload::Ty {
					channel: "minecraft:trader_list".to_owned(),
					data: trader_up(&p.data)?,
				},
			)])
		}
		_ => {}
	}
	Ok(payload_up(&p.channel, p.data)?
		.map(|(channel, data)| {
			Client13::CustomPayload(b::play_to_client_packet_custom_payload::Ty { channel, data })
		})
		.into_iter()
		.collect())
}

pub fn flattening_custom_payload_1_12_2(
	p: b::play_to_client_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client12>> {
	let data = match p.channel.as_str() {
		"minecraft:trader_list" => trader_down(&p.data)?,
		_ => p.data,
	};
	Ok(payload_down(&p.channel, data)?
		.map(|(channel, data)| {
			Client12::CustomPayload(a::play_to_client_packet_custom_payload::Ty { channel, data })
		})
		.into_iter()
		.collect())
}

fn block_position(r: &mut Reader) -> Result<b::position::Ty> {
	Ok(b::position::Ty {
		x: r.i32()?,
		y: r.i32()? as i16,
		z: r.i32()?,
	})
}

fn index_of(options: &[&str], value: &str) -> i32 {
	options
		.iter()
		.position(|o| *o == value)
		.unwrap_or(options.len() - 1) as i32
}

fn bit(r: &mut Reader, value: u8) -> Result<u8> {
	Ok(if r.u8()? != 0 { value } else { 0 })
}

fn server_payload_up(channel: &str, data: &[u8]) -> Result<Option<Server13>> {
	let mut r = Reader::new(data);
	Ok(Some(match channel {
		"MC|BSign" | "MC|BEdit" => Server13::EditBook(b::play_to_server_packet_edit_book::Ty {
			new_book: slot_up(r.with(a::slot::deserialize)?),
			signing: channel == "MC|BSign",
		}),
		"MC|ItemName" => Server13::NameItem(if r.remaining() > 0 {
			r.string()?
		} else {
			String::new()
		}),
		"MC|PickItem" => Server13::PickItem(r.varint()?),
		"MC|TrSel" => Server13::SelectTrade(r.i32()?),
		"MC|Beacon" => Server13::SetBeaconEffect(b::play_to_server_packet_set_beacon_effect::Ty {
			primary_effect: r.i32()?,
			secondary_effect: r.i32()?,
		}),
		"MC|AdvCmd" => {
			if r.u8()? != 1 {
				return Ok(None);
			}
			Server13::UpdateCommandBlockMinecart(
				b::play_to_server_packet_update_command_block_minecart::Ty {
					entity_id: r.i32()?,
					command: r.string()?,
					track_output: r.u8()? != 0,
				},
			)
		}
		"MC|AutoCmd" => {
			let location = block_position(&mut r)?;
			let command = r.string()?;
			let track = bit(&mut r, 1)?;
			let mode = index_of(&["SEQUENCE", "AUTO", "REDSTONE"], &r.string()?);
			let flags = track | bit(&mut r, 2)? | bit(&mut r, 4)?;
			Server13::UpdateCommandBlock(b::play_to_server_packet_update_command_block::Ty {
				location,
				command,
				mode,
				flags,
			})
		}
		"MC|Struct" => {
			let location = block_position(&mut r)?;
			let action = r.u8()? as i32 - 1;
			let mode = index_of(&["SAVE", "LOAD", "CORNER", "DATA"], &r.string()?);
			let name = r.string()?;
			let mut sizes = [0i8; 6];
			for s in &mut sizes {
				*s = r.i32()? as i8;
			}
			let mirror = index_of(&["NONE", "LEFT_RIGHT", "FRONT_BACK"], &r.string()?);
			let rotation = index_of(
				&[
					"NONE",
					"CLOCKWISE_90",
					"CLOCKWISE_180",
					"COUNTERCLOCKWISE_90",
				],
				&r.string()?,
			);
			let metadata = r.string()?;
			let flags = bit(&mut r, 1)? | bit(&mut r, 2)? | bit(&mut r, 4)?;
			let integrity = f32::from_bits(r.i32()? as u32);
			let seed = r.with(crate::pds_native::pds_varlong::deserialize)?;
			Server13::UpdateStructureBlock(b::play_to_server_packet_update_structure_block::Ty {
				location,
				action,
				mode,
				name,
				offset_x: sizes[0],
				offset_y: sizes[1],
				offset_z: sizes[2],
				size_x: sizes[3],
				size_y: sizes[4],
				size_z: sizes[5],
				mirror,
				rotation,
				metadata,
				integrity,
				seed,
				flags,
			})
		}
		_ => match payload_up(channel, data.to_vec())? {
			Some((channel, data)) => {
				Server13::CustomPayload(b::play_to_server_packet_custom_payload::Ty {
					channel,
					data,
				})
			}
			None => return Ok(None),
		},
	}))
}

pub fn flattening_server_custom_payload_1_13(
	p: a::play_to_server_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server13>> {
	Ok(server_payload_up(&p.channel, &p.data)?
		.into_iter()
		.collect())
}

pub fn flattening_server_custom_payload_1_12_2(
	p: b::play_to_server_packet_custom_payload::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server12>> {
	Ok(payload_down(&p.channel, p.data)?
		.map(|(channel, data)| {
			Server12::CustomPayload(a::play_to_server_packet_custom_payload::Ty { channel, data })
		})
		.into_iter()
		.collect())
}

fn server_payload(channel: &str, data: Vec<u8>) -> Result<Vec<Server12>> {
	Ok(vec![Server12::CustomPayload(
		a::play_to_server_packet_custom_payload::Ty {
			channel: channel.to_owned(),
			data,
		},
	)])
}

pub fn flattening_edit_book_1_12_2(
	p: b::play_to_server_packet_edit_book::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server12>> {
	let mut data = Vec::new();
	put_value(&mut data, &slot_down(p.new_book), a::slot::serialize)?;
	server_payload(if p.signing { "MC|BSign" } else { "MC|BEdit" }, data)
}

pub fn flattening_name_item_1_12_2(name: String, _cx: &mut Session) -> Result<Vec<Server12>> {
	let mut data = Vec::new();
	put_string(&mut data, &name);
	server_payload("MC|ItemName", data)
}

pub fn flattening_pick_item_1_12_2(slot: i32, _cx: &mut Session) -> Result<Vec<Server12>> {
	let mut data = Vec::new();
	put_varint(&mut data, slot);
	server_payload("MC|PickItem", data)
}

pub fn flattening_select_trade_1_12_2(slot: i32, _cx: &mut Session) -> Result<Vec<Server12>> {
	server_payload("MC|TrSel", slot.to_be_bytes().to_vec())
}

pub fn flattening_set_beacon_effect_1_12_2(
	p: b::play_to_server_packet_set_beacon_effect::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server12>> {
	let mut data = p.primary_effect.to_be_bytes().to_vec();
	data.extend(p.secondary_effect.to_be_bytes());
	server_payload("MC|Beacon", data)
}

fn position_ints(data: &mut Vec<u8>, p: &b::position::Ty) {
	data.extend(p.x.to_be_bytes());
	data.extend((p.y as i32).to_be_bytes());
	data.extend(p.z.to_be_bytes());
}

pub fn flattening_update_command_block_1_12_2(
	p: b::play_to_server_packet_update_command_block::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server12>> {
	let mut data = Vec::new();
	position_ints(&mut data, &p.location);
	put_string(&mut data, &p.command);
	data.push((p.flags & 1 != 0) as u8);
	put_string(
		&mut data,
		["SEQUENCE", "AUTO", "REDSTONE"][p.mode.clamp(0, 2) as usize],
	);
	data.push((p.flags & 2 != 0) as u8);
	data.push((p.flags & 4 != 0) as u8);
	server_payload("MC|AutoCmd", data)
}

pub fn flattening_update_command_block_minecart_1_12_2(
	p: b::play_to_server_packet_update_command_block_minecart::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server12>> {
	let mut data = vec![1];
	data.extend(p.entity_id.to_be_bytes());
	put_string(&mut data, &p.command);
	data.push(p.track_output as u8);
	server_payload("MC|AdvCmd", data)
}

pub fn flattening_update_structure_block_1_12_2(
	p: b::play_to_server_packet_update_structure_block::Ty,
	_cx: &mut Session,
) -> Result<Vec<Server12>> {
	let mut data = Vec::new();
	position_ints(&mut data, &p.location);
	data.push((p.action + 1) as u8);
	put_string(
		&mut data,
		["SAVE", "LOAD", "CORNER", "DATA"][p.mode.clamp(0, 3) as usize],
	);
	put_string(&mut data, &p.name);
	for v in [
		p.offset_x, p.offset_y, p.offset_z, p.size_x, p.size_y, p.size_z,
	] {
		data.extend((v as i32).to_be_bytes());
	}
	put_string(
		&mut data,
		["NONE", "LEFT_RIGHT", "FRONT_BACK"][p.mirror.clamp(0, 2) as usize],
	);
	put_string(
		&mut data,
		[
			"NONE",
			"CLOCKWISE_90",
			"CLOCKWISE_180",
			"COUNTERCLOCKWISE_90",
		][p.rotation.clamp(0, 3) as usize],
	);
	put_string(&mut data, &p.metadata);
	for bit in [1, 2, 4] {
		data.push((p.flags & bit != 0) as u8);
	}
	data.extend(p.integrity.to_be_bytes());
	let mut seed = p.seed as u64;
	loop {
		if seed & !0x7f == 0 {
			data.push(seed as u8);
			break;
		}
		data.push((seed as u8 & 0x7f) | 0x80);
		seed >>= 7;
	}
	server_payload("MC|Struct", data)
}

pub fn flattening_set_cooldown_1_13(
	p: a::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client13>> {
	Ok(item_of_legacy(p.item_id)
		.into_iter()
		.map(|item_id| {
			Client13::SetCooldown(b::play_to_client_packet_set_cooldown::Ty {
				item_id,
				cooldown_ticks: p.cooldown_ticks,
			})
		})
		.collect())
}

pub fn flattening_set_cooldown_1_12_2(
	p: b::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client12>> {
	let Some((id, _)) = legacy_of_item(p.item_id.max(0) as u32) else {
		return Ok(vec![]);
	};
	Ok(vec![Client12::SetCooldown(
		a::play_to_client_packet_set_cooldown::Ty {
			item_id: id as i32,
			cooldown_ticks: p.cooldown_ticks,
		},
	)])
}

const CUSTOM_STATS: [(&str, i32); 50] = [
	("leaveGame", 0),
	("playOneMinute", 1),
	("timeSinceDeath", 2),
	("sneakTime", 4),
	("walkOneCm", 5),
	("crouchOneCm", 6),
	("sprintOneCm", 7),
	("swimOneCm", 18),
	("fallOneCm", 9),
	("climbOneCm", 10),
	("flyOneCm", 11),
	("diveOneCm", 12),
	("minecartOneCm", 13),
	("boatOneCm", 14),
	("pigOneCm", 15),
	("horseOneCm", 16),
	("aviateOneCm", 17),
	("jump", 19),
	("drop", 20),
	("damageDealt", 21),
	("damageTaken", 22),
	("deaths", 23),
	("mobKills", 24),
	("animalsBred", 25),
	("playerKills", 26),
	("fishCaught", 27),
	("talkedToVillager", 28),
	("tradedWithVillager", 29),
	("cakeSlicesEaten", 30),
	("cauldronFilled", 31),
	("cauldronUsed", 32),
	("armorCleaned", 33),
	("bannerCleaned", 34),
	("brewingstandInteraction", 35),
	("beaconInteraction", 36),
	("dropperInspected", 37),
	("hopperInspected", 38),
	("dispenserInspected", 39),
	("noteblockPlayed", 40),
	("noteblockTuned", 41),
	("flowerPotted", 42),
	("trappedChestTriggered", 43),
	("enderchestOpened", 44),
	("itemEnchanted", 45),
	("recordPlayed", 46),
	("furnaceInteraction", 47),
	("craftingTableInteraction", 48),
	("chestOpened", 49),
	("sleepInBed", 50),
	("shulkerBoxOpened", 51),
];

const CATEGORIES: [&str; 8] = [
	"mineBlock",
	"craftItem",
	"useItem",
	"breakItem",
	"pickup",
	"drop",
	"killEntity",
	"entityKilledBy",
];

fn statistic_up(name: &str) -> Option<(i32, i32)> {
	let rest = name.strip_prefix("stat.")?;
	if let Some((_, id)) = CUSTOM_STATS.iter().find(|(n, _)| *n == rest) {
		return Some((8, *id));
	}
	let (category, object) = rest.split_once('.')?;
	let category = CATEGORIES.iter().position(|c| *c == category)? as i32;
	let object = object.strip_prefix("minecraft.").unwrap_or(object);
	let id = match category {
		0 => {
			let old = old_blocks().block(object)?.id;
			new_blocks().owner(state_of_raw(old << 4))?.id as i32
		}
		1..=5 => {
			let old = old_items().id(object)?;
			*item_of_legacy(old as i32).first()?
		}
		_ => return None,
	};
	Some((category, id))
}

fn statistic_down(category: i32, id: i32) -> Option<String> {
	if category == 8 {
		return CUSTOM_STATS
			.iter()
			.find(|(_, i)| *i == id)
			.map(|(n, _)| format!("stat.{n}"));
	}
	let name = match category {
		0 => {
			let raw = raw_of_state(new_blocks().list.get(id.max(0) as usize)?.default);
			old_blocks()
				.list
				.iter()
				.find(|b| b.id == raw >> 4)?
				.name
				.clone()
		}
		1..=5 => {
			let (old, _) = legacy_of_item(id.max(0) as u32)?;
			old_items().name(old as u32)?.to_owned()
		}
		_ => return None,
	};
	Some(format!(
		"stat.{}.minecraft.{name}",
		CATEGORIES[category as usize]
	))
}

pub fn flattening_statistics_1_13(
	p: Vec<a::play_to_client_packet_statistics_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<Client13>> {
	Ok(vec![Client13::Statistics(
		p.into_iter()
			.filter_map(|s| {
				let (category_id, statistic_id) = statistic_up(&s.name)?;
				Some(b::play_to_client_packet_statistics_items::Ty {
					category_id,
					statistic_id,
					value: s.value,
				})
			})
			.collect(),
	)])
}

pub fn flattening_statistics_1_12_2(
	p: Vec<b::play_to_client_packet_statistics_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<Client12>> {
	Ok(vec![Client12::Statistics(
		p.into_iter()
			.filter_map(|s| {
				Some(a::play_to_client_packet_statistics_items::Ty {
					name: statistic_down(s.category_id, s.statistic_id)?,
					value: s.value,
				})
			})
			.collect(),
	)])
}

fn utf16_len(s: &str) -> i32 {
	s.chars().map(char::len_utf16).sum::<usize>() as i32
}

pub fn flattening_tab_complete_1_13(
	matches: Vec<String>,
	cx: &mut Session,
) -> Result<Vec<Client13>> {
	let flat = cx.state::<Flat>();
	let input = &flat.tab_input;
	let (start, length) = match input.rfind(' ') {
		_ if input.is_empty() || input.ends_with(' ') => (utf16_len(input), 0),
		Some(space) => (
			utf16_len(&input[..space + 1]),
			utf16_len(&input[space + 1..]),
		),
		None => (0, utf16_len(input)),
	};
	let matches = matches
		.into_iter()
		.map(
			|m| b::play_to_client_packet_tab_complete_matches_items::Ty {
				r#match: match m.strip_prefix('/') {
					Some(rest) if start == 0 => rest.to_owned(),
					_ => m,
				},
				tooltip: None,
			},
		)
		.collect();
	Ok(vec![Client13::TabComplete(
		b::play_to_client_packet_tab_complete::Ty {
			transaction_id: flat.tab_transaction,
			start,
			length,
			matches,
		},
	)])
}

pub fn flattening_tab_complete_1_12_2(
	p: b::play_to_client_packet_tab_complete::Ty,
	cx: &mut Session,
) -> Result<Vec<Client12>> {
	let Some((id, request, assume)) = cx.state::<Legacy>().request.clone() else {
		return Ok(vec![]);
	};
	let last = request.rfind(' ').map_or(0, |i| i + 1);
	let accepted = id == p.transaction_id
		&& utf16_len(&request[..last]) == p.start
		&& p.length == utf16_len(&request[last..]);
	if !accepted {
		return Ok(vec![]);
	}
	let slash = if p.start == 0 && !assume { "/" } else { "" };
	Ok(vec![Client12::TabComplete(
		p.matches
			.into_iter()
			.map(|m| format!("{slash}{}", m.r#match))
			.collect(),
	)])
}

fn starts_with_ignore_case(value: &str, prefix: &str) -> bool {
	let (value, prefix): (Vec<u16>, Vec<u16>) = (
		value.encode_utf16().collect(),
		prefix.encode_utf16().collect(),
	);
	value.len() >= prefix.len()
		&& String::from_utf16_lossy(&value[..prefix.len()]).to_lowercase()
			== String::from_utf16_lossy(&prefix).to_lowercase()
}

pub fn flattening_server_tab_complete_1_12_2(
	p: b::play_to_server_packet_tab_complete::Ty,
	cx: &mut Session,
) -> Result<Vec<Server12>> {
	let flat = cx.state::<Flat>();
	flat.tab_transaction = p.transaction_id;
	flat.tab_input = p.text.clone();
	Ok(vec![Server12::TabComplete(
		a::play_to_server_packet_tab_complete::Ty {
			text: format!("/{}", p.text),
			assume_command: false,
			looked_at_block: None,
		},
	)])
}

pub fn flattening_server_tab_complete_1_13(
	p: a::play_to_server_packet_tab_complete::Ty,
	cx: &mut Session,
) -> Result<Vec<Server13>> {
	let legacy = cx.state::<Legacy>();
	let command = p.text;
	let assume = p.assume_command;
	let mut suggestions: Vec<String> = if !assume && !command.starts_with('/') {
		let buffer = &command[command.rfind(' ').map_or(0, |i| i + 1)..];
		legacy
			.usernames
			.values()
			.filter(|n| starts_with_ignore_case(n, buffer))
			.cloned()
			.collect()
	} else if !command.contains(' ') {
		legacy
			.commands
			.iter()
			.filter(|c| starts_with_ignore_case(c, &command))
			.cloned()
			.collect()
	} else {
		Vec::new()
	};
	if !suggestions.is_empty() {
		suggestions.sort();
		legacy.request = None;
		let answer = Client12::TabComplete(suggestions);
		send(
			cx,
			Root::PlayToClient,
			OLD,
			&answer,
			a::play_to_client_packet::serialize,
		);
		return Ok(vec![]);
	}
	let text = match command.strip_prefix('/') {
		Some(rest) if !assume => rest.to_owned(),
		_ => command,
	};
	legacy.counter = legacy.counter.wrapping_add(1);
	let transaction_id = legacy.counter;
	legacy.request = Some((transaction_id, text.clone(), assume));
	Ok(vec![Server13::TabComplete(
		b::play_to_server_packet_tab_complete::Ty {
			transaction_id,
			text,
		},
	)])
}

const TEAM_NAME_CODES: [(char, char); 22] = [
	('0', 'g'),
	('1', 'h'),
	('2', 'i'),
	('3', 'j'),
	('4', 'p'),
	('5', 'q'),
	('6', 's'),
	('7', 't'),
	('8', 'u'),
	('9', 'v'),
	('a', 'w'),
	('b', 'x'),
	('c', 'y'),
	('d', 'z'),
	('e', '!'),
	('f', '?'),
	('k', '#'),
	('l', '('),
	('m', ')'),
	('n', ':'),
	('o', ';'),
	('r', '/'),
];

fn member_name(name: String) -> String {
	if !strip_colors(&name).is_empty() {
		return name;
	}
	name.chars()
		.skip(1)
		.step_by(2)
		.map(|c| {
			let code = TEAM_NAME_CODES
				.iter()
				.find(|(from, _)| *from == c)
				.map_or(c, |(_, to)| *to);
			format!("\u{a7}{code}")
		})
		.collect()
}

pub fn flattening_score_name_1_13(
	src: &a::play_to_client_packet_scoreboard_score::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(member_name(src.item_name.clone()))
}

pub fn flattening_player_name_1_13(
	src: &a::play_to_client_packet_player_info_data_items_anon_add_player::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(member_name(src.name.clone()))
}

type Mode13 = b::play_to_client_packet_teams_mode::Ty;
type Mode12 = a::play_to_client_packet_teams_mode::Ty;

struct Team13 {
	name: String,
	formatting: i32,
	prefix: String,
	suffix: String,
}

fn team_up(name: &str, prefix: &str, suffix: &str) -> Team13 {
	let last = last_color(prefix);
	Team13 {
		name: legacy_to_json(name, false),
		formatting: color_ordinal(last),
		prefix: legacy_to_json(prefix, false),
		suffix: legacy_to_json(&format!("\u{a7}{last}{suffix}"), false),
	}
}

struct Team12 {
	name: String,
	prefix: String,
	suffix: String,
	color: i8,
}

fn team_down(name: &str, prefix: &str, suffix: &str, formatting: i32) -> Team12 {
	let color = if formatting == 21 {
		-1
	} else {
		formatting as i8
	};
	let code = if (0..=15).contains(&color) {
		char::from_digit(color as u32, 16).unwrap_or('r')
	} else {
		'r'
	};
	Team12 {
		name: truncate_legacy(&json_to_legacy(name), 'f', 32, false),
		prefix: truncate_legacy(
			&format!("{}\u{a7}{code}", json_to_legacy(prefix)),
			'f',
			16,
			true,
		),
		suffix: truncate_legacy(&json_to_legacy(suffix), '\0', 16, false),
		color,
	}
}

pub fn flattening_team_create_1_13(
	p: a::play_to_client_packet_teams_mode_v0::Ty,
	_cx: &mut Session,
) -> Result<Mode13> {
	let t = team_up(&p.name, &p.prefix, &p.suffix);
	Ok(Mode13::V0(b::play_to_client_packet_teams_mode_v0::Ty {
		name: t.name,
		friendly_fire: p.friendly_fire,
		name_tag_visibility: p.name_tag_visibility,
		collision_rule: p.collision_rule,
		formatting: t.formatting,
		prefix: t.prefix,
		suffix: t.suffix,
		players: p.players.into_iter().map(member_name).collect(),
	}))
}

pub fn flattening_team_update_1_13(
	p: a::play_to_client_packet_teams_mode_v2::Ty,
	_cx: &mut Session,
) -> Result<Mode13> {
	let t = team_up(&p.name, &p.prefix, &p.suffix);
	Ok(Mode13::V2(b::play_to_client_packet_teams_mode_v2::Ty {
		name: t.name,
		friendly_fire: p.friendly_fire,
		name_tag_visibility: p.name_tag_visibility,
		collision_rule: p.collision_rule,
		formatting: t.formatting,
		prefix: t.prefix,
		suffix: t.suffix,
	}))
}

pub fn flattening_team_join_1_13(players: Vec<String>, _cx: &mut Session) -> Result<Mode13> {
	Ok(Mode13::V3(players.into_iter().map(member_name).collect()))
}

pub fn flattening_team_leave_1_13(players: Vec<String>, _cx: &mut Session) -> Result<Mode13> {
	Ok(Mode13::V4(players.into_iter().map(member_name).collect()))
}

pub fn flattening_team_create_1_12_2(
	p: b::play_to_client_packet_teams_mode_v0::Ty,
	_cx: &mut Session,
) -> Result<Mode12> {
	let t = team_down(&p.name, &p.prefix, &p.suffix, p.formatting);
	Ok(Mode12::V0(a::play_to_client_packet_teams_mode_v0::Ty {
		name: t.name,
		prefix: t.prefix,
		suffix: t.suffix,
		friendly_fire: p.friendly_fire,
		name_tag_visibility: p.name_tag_visibility,
		collision_rule: p.collision_rule,
		color: t.color,
		players: p.players,
	}))
}

pub fn flattening_team_update_1_12_2(
	p: b::play_to_client_packet_teams_mode_v2::Ty,
	_cx: &mut Session,
) -> Result<Mode12> {
	let t = team_down(&p.name, &p.prefix, &p.suffix, p.formatting);
	Ok(Mode12::V2(a::play_to_client_packet_teams_mode_v2::Ty {
		name: t.name,
		prefix: t.prefix,
		suffix: t.suffix,
		friendly_fire: p.friendly_fire,
		name_tag_visibility: p.name_tag_visibility,
		collision_rule: p.collision_rule,
		color: t.color,
	}))
}

type Objective13 = b::play_to_client_packet_scoreboard_objective_action::Ty;
type Objective12 = a::play_to_client_packet_scoreboard_objective_action::Ty;

fn objective_type(t: &str) -> i32 {
	(t == "hearts") as i32
}

fn objective_type_down(t: i32) -> String {
	if t == 1 { "hearts" } else { "integer" }.to_owned()
}

pub fn flattening_objective_create_1_13(
	p: a::play_to_client_packet_scoreboard_objective_action_v0::Ty,
	_cx: &mut Session,
) -> Result<Objective13> {
	Ok(Objective13::V0(
		b::play_to_client_packet_scoreboard_objective_action_v0::Ty {
			display_text: legacy_to_json(&p.display_text, false),
			r#type: objective_type(&p.r#type),
		},
	))
}

pub fn flattening_objective_update_1_13(
	p: a::play_to_client_packet_scoreboard_objective_action_v2::Ty,
	_cx: &mut Session,
) -> Result<Objective13> {
	Ok(Objective13::V2(
		b::play_to_client_packet_scoreboard_objective_action_v2::Ty {
			display_text: legacy_to_json(&p.display_text, false),
			r#type: objective_type(&p.r#type),
		},
	))
}

pub fn flattening_objective_create_1_12_2(
	p: b::play_to_client_packet_scoreboard_objective_action_v0::Ty,
	_cx: &mut Session,
) -> Result<Objective12> {
	Ok(Objective12::V0(
		a::play_to_client_packet_scoreboard_objective_action_v0::Ty {
			display_text: truncate_legacy(&json_to_legacy(&p.display_text), 'f', 32, false),
			r#type: objective_type_down(p.r#type),
		},
	))
}

pub fn flattening_objective_update_1_12_2(
	p: b::play_to_client_packet_scoreboard_objective_action_v2::Ty,
	_cx: &mut Session,
) -> Result<Objective12> {
	Ok(Objective12::V2(
		a::play_to_client_packet_scoreboard_objective_action_v2::Ty {
			display_text: truncate_legacy(&json_to_legacy(&p.display_text), 'f', 32, false),
			r#type: objective_type_down(p.r#type),
		},
	))
}

const RECIPE: &str = "pds2:legacy_";

fn recipe_up(id: i32) -> String {
	format!("{RECIPE}{id}")
}

fn recipe_down(id: &str) -> Option<i32> {
	id.strip_prefix(RECIPE)?.parse().ok()
}

pub fn flattening_recipe_list_1_13(
	src: &a::play_to_client_packet_unlock_recipes::Ty,
	_cx: &mut Session,
) -> Result<Vec<String>> {
	Ok(src.recipes1.iter().copied().map(recipe_up).collect())
}

fn recipes_down(ids: &[String]) -> Result<Vec<i32>> {
	ids.iter()
		.map(|r| recipe_down(r).ok_or(Error::Dropped))
		.collect()
}

pub fn flattening_recipe_list_1_12_2(
	src: &b::play_to_client_packet_unlock_recipes::Ty,
	_cx: &mut Session,
) -> Result<Vec<i32>> {
	recipes_down(&src.recipes1)
}

pub fn flattening_recipe_init_1_13(
	ids: Vec<i32>,
	_cx: &mut Session,
) -> Result<b::play_to_client_packet_unlock_recipes_recipes2::Ty> {
	Ok(b::play_to_client_packet_unlock_recipes_recipes2::Ty::V0(
		ids.into_iter().map(recipe_up).collect(),
	))
}

pub fn flattening_recipe_init_1_12_2(
	ids: Vec<String>,
	_cx: &mut Session,
) -> Result<a::play_to_client_packet_unlock_recipes_recipes2::Ty> {
	Ok(a::play_to_client_packet_unlock_recipes_recipes2::Ty::V0(
		recipes_down(&ids)?,
	))
}

pub fn flattening_ghost_recipe_1_13(
	src: &a::play_to_client_packet_craft_recipe_response::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(recipe_up(src.recipe))
}

pub fn flattening_ghost_recipe_1_12_2(
	src: &b::play_to_client_packet_craft_recipe_response::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	recipe_down(&src.recipe).ok_or(Error::Dropped)
}

pub fn flattening_recipe_request_1_13(
	_src: &a::play_to_server_packet_craft_recipe_request::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Err(Error::Dropped)
}

pub fn flattening_recipe_request_1_12_2(
	src: &b::play_to_server_packet_craft_recipe_request::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	recipe_down(&src.recipe).ok_or(Error::Dropped)
}

pub fn flattening_recipe_seen_1_13(
	_id: i32,
	_cx: &mut Session,
) -> Result<b::play_to_server_packet_crafting_book_data::Ty> {
	Err(Error::Dropped)
}

pub fn flattening_book_settings_1_13(
	_v: a::play_to_server_packet_crafting_book_data_v1::Ty,
	_cx: &mut Session,
) -> Result<b::play_to_server_packet_crafting_book_data::Ty> {
	Err(Error::Dropped)
}

pub fn flattening_book_settings_1_12_2(
	v: b::play_to_server_packet_crafting_book_data_v1::Ty,
	_cx: &mut Session,
) -> Result<a::play_to_server_packet_crafting_book_data::Ty> {
	Ok(a::play_to_server_packet_crafting_book_data::Ty::V1(
		a::play_to_server_packet_crafting_book_data_v1::Ty {
			crafting_book_open: v.crafting_book_open,
			crafting_filter: v.crafting_filter,
		},
	))
}

pub fn flattening_recipe_seen_1_12_2(
	id: String,
	_cx: &mut Session,
) -> Result<a::play_to_server_packet_crafting_book_data::Ty> {
	recipe_down(&id)
		.map(a::play_to_server_packet_crafting_book_data::Ty::V0)
		.ok_or(Error::Dropped)
}
