use crate::Session;
use crate::generated::convert::{v26_3 as a, v26_4_snapshot_10 as b};
use crate::pds::convert::{Error, Result};

type Old = a::command_node_extra_node_data_v2_properties::Ty;
type New = b::command_node_extra_node_data_v2_properties::Ty;
type Mode = a::command_node_extra_node_data_v2_properties_brigadier_string::Ty;
type Suggestion = b::command_node_extra_node_data_v2_suggestion_type_v1::Ty;

const CHEST: i32 = 2;
const COMMAND_PREDICATE: i32 = 3;
const COMMAND_MODIFIER: i32 = 4;
const COMMAND_SLOT_SOURCE: i32 = 5;
const COMMAND_COMPUTE_DEFAULT: i32 = 6;

pub fn late_parser_string_up_26_4(v: Mode, _cx: &mut Session) -> Result<New> {
	match v {
		Mode::SingleWord => Ok(New::MinecraftWord),
		Mode::QuotablePhrase => Ok(New::MinecraftQuotedString),
		Mode::GreedyPhrase => Ok(New::MinecraftGreedyString),
		Mode::Default(mode) => Err(Error::Unconvertible(format!(
			"brigadier:string mode {mode}"
		))),
	}
}

pub fn late_parser_word_down_26_3(_v: (), _cx: &mut Session) -> Result<Old> {
	Ok(Old::BrigadierString(Mode::SingleWord))
}

pub fn late_parser_quoted_down_26_3(_v: (), _cx: &mut Session) -> Result<Old> {
	Ok(Old::BrigadierString(Mode::QuotablePhrase))
}

pub fn late_parser_greedy_down_26_3(_v: (), _cx: &mut Session) -> Result<Old> {
	Ok(Old::BrigadierString(Mode::GreedyPhrase))
}

pub fn late_parser_pattern_down_26_3(
	_v: b::command_node_extra_node_data_v2_properties_minecraft_pattern::Ty,
	_cx: &mut Session,
) -> Result<Old> {
	Ok(Old::BrigadierString(Mode::QuotablePhrase))
}

pub fn late_parser_resource_key_up_26_4(registry: String, _cx: &mut Session) -> Result<New> {
	Ok(New::MinecraftResource(registry))
}

pub fn late_parser_resource_or_tag_key_up_26_4(registry: String, _cx: &mut Session) -> Result<New> {
	Ok(New::MinecraftResourceOrTag(registry))
}

macro_rules! context_sets {
	($($up:ident, $down:ident, $arm:ident, $set:expr_2021;)*) => {
		$(
			pub fn $up(_v: (), _cx: &mut Session) -> Result<New> {
				Ok(New::$arm($set))
			}

			pub fn $down(_set: i32, _cx: &mut Session) -> Result<Old> {
				Ok(Old::$arm)
			}
		)*
	};
}

context_sets! {
	late_parser_float_provider_up_26_4, late_parser_float_provider_down_26_3, MinecraftContextFloatProvider, COMMAND_COMPUTE_DEFAULT;
	late_parser_int_provider_up_26_4, late_parser_int_provider_down_26_3, MinecraftContextIntProvider, COMMAND_COMPUTE_DEFAULT;
	late_parser_loot_modifier_up_26_4, late_parser_loot_modifier_down_26_3, MinecraftLootModifier, COMMAND_MODIFIER;
	late_parser_loot_predicate_up_26_4, late_parser_loot_predicate_down_26_3, MinecraftLootPredicate, COMMAND_PREDICATE;
	late_parser_loot_table_up_26_4, late_parser_loot_table_down_26_3, MinecraftLootTable, CHEST;
	late_parser_slot_source_up_26_4, late_parser_slot_source_down_26_3, MinecraftSlotSource, COMMAND_SLOT_SOURCE;
}

pub fn late_parser_score_holder_up_26_4(
	v: a::command_node_extra_node_data_v2_properties_minecraft_score_holder::Ty,
	_cx: &mut Session,
) -> Result<New> {
	Ok(New::MinecraftScoreHolder(v.allow_multiple != 0))
}

pub fn late_parser_score_holder_down_26_3(multiple: bool, _cx: &mut Session) -> Result<Old> {
	Ok(Old::MinecraftScoreHolder(
		a::command_node_extra_node_data_v2_properties_minecraft_score_holder::Ty {
			unused: 0,
			allow_multiple: multiple as u8,
		},
	))
}

pub fn late_parser_time_up_26_4(min: i32, _cx: &mut Session) -> Result<New> {
	Ok(New::MinecraftTime(
		b::command_node_extra_node_data_v2_properties_minecraft_time::Ty {
			flags: b::command_node_extra_node_data_v2_properties_minecraft_time_flags::Ty {
				unused: 0,
				max_present: 0,
				min_present: 1,
			},
			min: b::command_node_extra_node_data_v2_properties_minecraft_time_min::Ty::V1(min),
			max: b::command_node_extra_node_data_v2_properties_minecraft_time_max::Ty::Default(0),
		},
	))
}

pub fn late_parser_time_down_26_3(
	v: b::command_node_extra_node_data_v2_properties_minecraft_time::Ty,
	_cx: &mut Session,
) -> Result<Old> {
	Ok(Old::MinecraftTime(match v.min {
		b::command_node_extra_node_data_v2_properties_minecraft_time_min::Ty::V1(min) => min,
		b::command_node_extra_node_data_v2_properties_minecraft_time_min::Ty::Default(_) => {
			i32::MIN
		}
	}))
}

pub fn late_parser_vec2_up_26_4(_v: (), _cx: &mut Session) -> Result<New> {
	Ok(New::MinecraftVec2(true))
}

pub fn late_parser_vec2_down_26_3(_center: bool, _cx: &mut Session) -> Result<Old> {
	Ok(Old::MinecraftVec2)
}

pub fn late_parser_vec3_up_26_4(_v: (), _cx: &mut Session) -> Result<New> {
	Ok(New::MinecraftVec3(true))
}

pub fn late_parser_vec3_down_26_3(_center: bool, _cx: &mut Session) -> Result<Old> {
	Ok(Old::MinecraftVec3)
}

const ASK_SERVER: &str = "minecraft:ask_server";

pub fn late_suggestion_up_26_4(id: String, _cx: &mut Session) -> Result<Suggestion> {
	Ok(match id.strip_prefix("minecraft:").unwrap_or(&id) {
		"available_sounds" => Suggestion::AvailableSounds,
		"summonable_entities" => Suggestion::SummonableEntities,
		"post_effects" => Suggestion::PostEffects,
		_ => Suggestion::AskServer,
	})
}

pub fn late_suggestion_down_26_3(v: Suggestion, _cx: &mut Session) -> Result<String> {
	Ok(match v {
		Suggestion::AvailableSounds => "minecraft:available_sounds",
		Suggestion::SummonableEntities => "minecraft:summonable_entities",
		Suggestion::PostEffects => "minecraft:post_effects",
		Suggestion::AskServer | Suggestion::Literal(_) | Suggestion::Default(_) => ASK_SERVER,
	}
	.to_owned())
}
