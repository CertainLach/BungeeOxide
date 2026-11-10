use super::remap::{self, remaps};
use super::rules::{blocks, entities, items, statistics, tags};
use super::shared::{self, same_bytes, text_down};
use super::slots::{self, untrusted_step};
use super::text;
use crate::gen::convert::{v1_21_5 as a, v1_21_6 as b, Version};
use crate::pds::convert::{Error, Result};
use crate::Session;
use nbt::Payload;

remaps!(Version::V1_21_5 => Version::V1_21_6 { late_sound_up_1_21_6: a::ItemSoundHolder::Ty = remap::sound; });
remaps!(Version::V1_21_6 => Version::V1_21_5 { late_sound_down_1_21_5: a::ItemSoundHolder::Ty = remap::sound; });

items!(a => b, Version::V1_21_5 => Version::V1_21_6 {
	slot: late_slot_item_up_1_21_6, hashed: late_hashed_item_up_1_21_6, trade1: late_trade1_up_1_21_6, trade2: late_trade2_up_1_21_6,
	display: late_display_item_up_1_21_6, recipes: late_recipe_items_up_1_21_6, stonecutter: late_stonecutter_up_1_21_6,
	requirements: late_requirements_up_1_21_6, repairable: late_repairable_up_1_21_6, pots: late_pots_up_1_21_6,
});
items!(b => a, Version::V1_21_6 => Version::V1_21_5 {
	slot: late_slot_item_down_1_21_5, hashed: late_hashed_item_down_1_21_5, trade1: late_trade1_down_1_21_5, trade2: late_trade2_down_1_21_5,
	display: late_display_item_down_1_21_5, recipes: late_recipe_items_down_1_21_5, stonecutter: late_stonecutter_down_1_21_5,
	requirements: late_requirements_down_1_21_5, repairable: late_repairable_down_1_21_5, pots: late_pots_down_1_21_5,
});

blocks!(a => b, Version::V1_21_5 => Version::V1_21_6 {
	block_change: late_block_change_up_1_21_6, multi_block: late_multi_block_up_1_21_6, block_action: late_block_action_up_1_21_6,
	world_event: late_world_event_up_1_21_6, chunk: late_chunk_up_1_21_6, tool: late_tool_up_1_21_6, predicate: late_predicate_up_1_21_6,
	particle: [late_particle_block_up_1_21_6, late_particle_marker_up_1_21_6, late_particle_dust_up_1_21_6, late_particle_pillar_up_1_21_6, late_particle_crumble_up_1_21_6],
	meta: [late_meta_state_up_1_21_6, late_meta_optional_state_up_1_21_6],
});
blocks!(b => a, Version::V1_21_6 => Version::V1_21_5 {
	block_change: late_block_change_down_1_21_5, multi_block: late_multi_block_down_1_21_5, block_action: late_block_action_down_1_21_5,
	world_event: late_world_event_down_1_21_5, chunk: late_chunk_down_1_21_5, tool: late_tool_down_1_21_5, predicate: late_predicate_down_1_21_5,
	particle: [late_particle_block_down_1_21_5, late_particle_marker_down_1_21_5, late_particle_dust_down_1_21_5, late_particle_pillar_down_1_21_5, late_particle_crumble_down_1_21_5],
	meta: [late_meta_state_down_1_21_5, late_meta_optional_state_down_1_21_5],
});

entities!(a => b, Version::V1_21_5 => Version::V1_21_6 { spawn: late_spawn_type_up_1_21_6, object: late_object_data_up_1_21_6, equippable: late_equip_entities_up_1_21_6, });
entities!(b => a, Version::V1_21_6 => Version::V1_21_5 { spawn: late_spawn_type_down_1_21_5, object: late_object_data_down_1_21_5, equippable: late_equip_entities_down_1_21_5, });

statistics!(a => b, Version::V1_21_5 => Version::V1_21_6 { late_statistics_up_1_21_6 });
statistics!(b => a, Version::V1_21_6 => Version::V1_21_5 { late_statistics_down_1_21_5 });

const TAG_RENAMES_UP: &[(&str, &str, &str)] = &[(
	"block",
	"plays_ambient_desert_block_sounds",
	"triggers_ambient_desert_dry_vegetation_block_sounds",
)];
const TAG_RENAMES_DOWN: &[(&str, &str, &str)] = &[(
	"block",
	"triggers_ambient_desert_dry_vegetation_block_sounds",
	"plays_ambient_desert_block_sounds",
)];

tags!(a => b, Version::V1_21_5 => Version::V1_21_6, TAG_RENAMES_UP, { config: late_config_tags_up_1_21_6, play: late_play_tags_up_1_21_6 });
tags!(b => a, Version::V1_21_6 => Version::V1_21_5, TAG_RENAMES_DOWN, { config: late_config_tags_down_1_21_5, play: late_play_tags_down_1_21_5 });

pub fn late_registry_data_up_1_21_6(
	mut p: a::configuration_toClient_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<b::configuration_toClient_packet::Ty>> {
	shared::count_biomes(&p, cx);
	shared::registry_entries(&mut p, "dimension_type", |_, e| {
		let effects = e
			.field("effects")
			.and_then(Payload::as_str)
			.unwrap_or_default();
		let effects = effects.strip_prefix("minecraft:").unwrap_or(effects);
		if !matches!(effects, "the_nether" | "the_end") && e.field("cloud_height").is_none() {
			e.put("cloud_height", Payload::int(192));
		}
	});
	Ok(vec![b::configuration_toClient_packet::Ty::registry_data(p)])
}

pub fn late_matchers_up_1_21_6(
	v: &a::ItemBlockPredicate::Ty,
	_cx: &mut Session,
) -> Result<b::DataComponentMatchers::Ty> {
	Ok(v.components.clone())
}

pub fn late_matchers_down_1_21_5(
	v: &b::ItemBlockPredicate::Ty,
	_cx: &mut Session,
) -> Result<a::DataComponentMatchers::Ty> {
	Ok(v.components.clone())
}

pub fn late_attribute_up_1_21_6(
	v: &a::SlotComponent__attribute_modifiers__items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(if v.typeId >= 8 {
		v.typeId + 1
	} else {
		v.typeId
	})
}

pub fn late_attribute_down_1_21_5(
	v: &b::SlotComponent__attribute_modifiers__items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	match v.typeId {
		8 | 33.. => Err(Error::Dropped),
		t if t > 8 => Ok(t - 1),
		t => Ok(t),
	}
}

pub fn late_attribute_display_up_1_21_6(
	_v: &a::SlotComponent__attribute_modifiers__items::Ty,
	_cx: &mut Session,
) -> Result<b::SlotComponent__attribute_modifiers__items__display::Ty> {
	Ok(b::SlotComponent__attribute_modifiers__items__display::Ty::default)
}

pub fn late_shearing_sound_up_1_21_6(
	_v: &a::SlotComponent__equippable::Ty,
	_cx: &mut Session,
) -> Result<b::ItemSoundHolder::Ty> {
	Ok(b::ItemSoundHolder::Ty::Id(
		remap::sound_id(Version::V1_21_6, "item.shears.snip").unwrap_or(0),
	))
}

same_bytes!(late_teams_up_1_21_6: a::play_toClient_packet_teams => b::play_toClient_packet_teams);
same_bytes!(late_teams_down_1_21_5: b::play_toClient_packet_teams => a::play_toClient_packet_teams);
same_bytes!(late_recipe_settings_up_1_21_6: a::play_toClient_packet_recipe_book_settings => b::play_toClient_packet_recipe_book_settings);
same_bytes!(late_recipe_settings_down_1_21_5: b::play_toClient_packet_recipe_book_settings => a::play_toClient_packet_recipe_book_settings);

pub fn late_heightmaps_up_1_21_6(
	v: &a::play_toClient_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<b::play_toClient_packet_map_chunk__heightmaps::Ty> {
	super::util::recode(
		&v.heightmaps,
		a::play_toClient_packet_map_chunk__heightmaps::serialize,
		b::play_toClient_packet_map_chunk__heightmaps::deserialize,
	)
}

pub fn late_heightmaps_down_1_21_5(
	v: &b::play_toClient_packet_map_chunk::Ty,
	_cx: &mut Session,
) -> Result<a::play_toClient_packet_map_chunk__heightmaps::Ty> {
	super::util::recode(
		&v.heightmaps,
		b::play_toClient_packet_map_chunk__heightmaps::serialize,
		a::play_toClient_packet_map_chunk__heightmaps::deserialize,
	)
}

const HANGING: &[&str] = &["item_frame", "glow_item_frame", "painting"];

pub fn late_meta_entity_up_1_21_6(
	p: &a::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, Version::V1_21_5, p.entityId);
	Ok(p.entityId)
}

pub fn late_meta_entity_down_1_21_5(
	p: &b::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<i32> {
	shared::meta_entity(cx, Version::V1_21_6, p.entityId);
	Ok(p.entityId)
}

pub fn late_meta_key_up_1_21_6(e: &a::entityMetadataEntry::Ty, cx: &mut Session) -> Result<u8> {
	Ok(if e.key >= 8 && shared::is(cx, HANGING) {
		e.key + 1
	} else {
		e.key
	})
}

pub fn late_meta_key_down_1_21_5(e: &b::entityMetadataEntry::Ty, cx: &mut Session) -> Result<u8> {
	if let Some(key) = shared::fallback_key(cx, Version::V1_21_6, Version::V1_21_5, e.key) {
		return key;
	}
	if !shared::is(cx, HANGING) || e.key < 8 {
		return Ok(e.key);
	}
	match e.key {
		8 => Err(Error::Dropped),
		k => Ok(k - 1),
	}
}

#[derive(Default)]
struct Sneak {
	player: i32,
	sneaking: bool,
}

pub fn late_login_up_1_21_6(
	p: &a::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<i32> {
	*cx.state::<Sneak>() = Sneak {
		player: p.entityId,
		sneaking: false,
	};
	Ok(p.entityId)
}

pub fn late_respawn_up_1_21_6(
	p: &a::play_toClient_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<u8> {
	cx.state::<Sneak>().sneaking = false;
	Ok(p.copyMetadata)
}

pub fn late_player_input_down_1_21_5(
	p: b::play_toServer_packet_player_input::Ty,
	cx: &mut Session,
) -> Result<Vec<a::play_toServer_packet::Ty>> {
	let state = cx.state::<Sneak>();
	let mut out = Vec::new();
	if p.shift != state.sneaking {
		state.sneaking = p.shift;
		use a::play_toServer_packet_entity_action__actionId::Ty as Action;
		out.push(a::play_toServer_packet::Ty::entity_action(
			a::play_toServer_packet_entity_action::Ty {
				entityId: state.player,
				actionId: if p.shift {
					Action::start_sneaking
				} else {
					Action::stop_sneaking
				},
				jumpBoost: 0,
			},
		));
	}
	out.push(a::play_toServer_packet::Ty::player_input(p));
	Ok(out)
}

pub fn late_change_gamemode_down_1_21_5(
	p: b::play_toServer_packet_change_gamemode::Ty,
	_cx: &mut Session,
) -> Result<Vec<a::play_toServer_packet::Ty>> {
	use b::play_toServer_packet_change_gamemode::Ty as Mode;
	let mode = match p {
		Mode::survival => "survival",
		Mode::creative => "creative",
		Mode::adventure => "adventure",
		Mode::spectator => "spectator",
		Mode::default(_) => return Ok(vec![]),
	};
	Ok(vec![a::play_toServer_packet::Ty::chat_command(format!(
		"gamemode {mode}"
	))])
}

untrusted_step!(late_untrusted_up_1_21_6: a => b, Version::V1_21_5 => Version::V1_21_6, slots::late_to_slot_1_21_5, slots::late_to_untrusted_1_21_6);
untrusted_step!(late_untrusted_down_1_21_5: b => a, Version::V1_21_6 => Version::V1_21_5, slots::late_to_slot_1_21_6, slots::late_to_untrusted_1_21_5);

pub fn late_registry_data_down_1_21_5(
	p: b::configuration_toClient_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<a::configuration_toClient_packet::Ty>> {
	shared::count_biomes(&p, cx);
	if p.id.strip_prefix("minecraft:").unwrap_or(&p.id) == "dialog" {
		return Ok(Vec::new());
	}
	Ok(vec![a::configuration_toClient_packet::Ty::registry_data(p)])
}

pub fn late_text_keep_up_1_21_6(v: nbt::Tag, _cx: &mut Session) -> Result<nbt::Tag> {
	Ok(v)
}

text_down!(b => a, text::undialog, {
	tag: late_undialog_down_1_21_5,
	links: [late_config_links_down_1_21_5, late_links_down_1_21_5],
	window: late_open_window_down_1_21_5,
	chat: late_player_chat_down_1_21_5,
	packets: [
		play_toClient_packet_system_chat: system_chat => late_system_chat_down_1_21_5,
		play_toClient_packet_playerlist_header: header => late_header_down_1_21_5,
		play_toClient_packet_death_combat_event: death => late_death_down_1_21_5,
		play_toClient_packet_profileless_chat: profileless => late_profileless_down_1_21_5,
		play_toClient_packet_scoreboard_objective: objective => late_objective_down_1_21_5,
		play_toClient_packet_scoreboard_score: score => late_score_down_1_21_5,
		play_toClient_packet_player_info: player_info => late_player_info_down_1_21_5,
		play_toClient_packet_boss_bar: boss_bar => late_boss_bar_down_1_21_5,
		play_toClient_packet_map: map => late_map_down_1_21_5,
		play_toClient_packet_tab_complete: tab_complete => late_tab_complete_down_1_21_5,
		play_toClient_packet_server_data: server_data => late_server_data_down_1_21_5,
		packet_common_add_resource_pack: resource_pack => late_resource_pack_down_1_21_5,
	],
	arms: [
		play_toClient_packet: action_bar => late_action_bar_down_1_21_5,
		play_toClient_packet: set_title_text => late_title_down_1_21_5,
		play_toClient_packet: set_title_subtitle => late_subtitle_down_1_21_5,
		play_toClient_packet: kick_disconnect => late_kick_down_1_21_5,
		configuration_toClient_packet: disconnect => late_config_disconnect_down_1_21_5,
	],
});

fn living(name: &str) -> bool {
	crate::registry::names(Version::V1_21_6, "entities")
		.and_then(|n| crate::registry::entity_metadata_keys(Version::V1_21_6, n.id(name)?))
		.is_some_and(|keys| keys.iter().any(|f| f == "living_entity_flags"))
}

pub fn late_spawn_entity_down_1_21_5(
	p: b::play_toClient_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<Vec<a::play_toClient_packet::Ty>> {
	let r#type = late_spawn_type_down_1_21_5(&p, cx)?;
	let objectData = late_object_data_down_1_21_5(&p, cx)?;
	let name = crate::registry::names(Version::V1_21_6, "entities")
		.and_then(|n| n.name(p.r#type.max(0) as u32))
		.unwrap_or_default();
	let v = p.velocity.clone();
	let moving = v.x != 0 || v.y != 0 || v.z != 0;
	let entityId = p.entityId;
	let spawn =
		a::play_toClient_packet::Ty::spawn_entity(a::play_toClient_packet_spawn_entity::Ty {
			r#type,
			objectData,
			..p
		});
	if !moving || living(name) {
		return Ok(vec![spawn]);
	}
	Ok(vec![
		spawn,
		a::play_toClient_packet::Ty::entity_velocity(a::play_toClient_packet_entity_velocity::Ty {
			entityId,
			velocity: v,
		}),
	])
}
