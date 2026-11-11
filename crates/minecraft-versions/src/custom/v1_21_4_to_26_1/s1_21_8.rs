use super::remap::{self, remaps};
use super::rules::{items, statistics, tags};
use super::slots::{self, untrusted_step};
use crate::gen::convert::{v1_21_6 as a, v1_21_8 as b, Version};
use crate::pds::convert::Result;
use crate::Session;

remaps!(Version::V1_21_6 => Version::V1_21_8 { late_sound_up_1_21_8: a::item_sound_holder::Ty = remap::sound; });
remaps!(Version::V1_21_8 => Version::V1_21_6 { late_sound_down_1_21_6: a::item_sound_holder::Ty = remap::sound; });

items!(a => b, Version::V1_21_6 => Version::V1_21_8 {
	slot: late_slot_item_up_1_21_8, hashed: late_hashed_item_up_1_21_8, trade1: late_trade1_up_1_21_8, trade2: late_trade2_up_1_21_8,
	display: late_display_item_up_1_21_8, recipes: late_recipe_items_up_1_21_8, stonecutter: late_stonecutter_up_1_21_8,
	requirements: late_requirements_up_1_21_8, repairable: late_repairable_up_1_21_8, pots: late_pots_up_1_21_8,
});
items!(b => a, Version::V1_21_8 => Version::V1_21_6 {
	slot: late_slot_item_down_1_21_6, hashed: late_hashed_item_down_1_21_6, trade1: late_trade1_down_1_21_6, trade2: late_trade2_down_1_21_6,
	display: late_display_item_down_1_21_6, recipes: late_recipe_items_down_1_21_6, stonecutter: late_stonecutter_down_1_21_6,
	requirements: late_requirements_down_1_21_6, repairable: late_repairable_down_1_21_6, pots: late_pots_down_1_21_6,
});

statistics!(a => b, Version::V1_21_6 => Version::V1_21_8 { late_statistics_up_1_21_8 });
statistics!(b => a, Version::V1_21_8 => Version::V1_21_6 { late_statistics_down_1_21_6 });

tags!(a => b, Version::V1_21_6 => Version::V1_21_8, &[], { config: late_config_tags_up_1_21_8, play: late_play_tags_up_1_21_8 });
tags!(b => a, Version::V1_21_8 => Version::V1_21_6, &[], { config: late_config_tags_down_1_21_6, play: late_play_tags_down_1_21_6 });

pub fn late_matchers_up_1_21_8(
	v: &a::item_block_predicate::Ty,
	_cx: &mut Session,
) -> Result<b::data_component_matchers::Ty> {
	Ok(v.components.clone())
}

pub fn late_matchers_down_1_21_6(
	v: &b::item_block_predicate::Ty,
	_cx: &mut Session,
) -> Result<a::data_component_matchers::Ty> {
	Ok(v.components.clone())
}

untrusted_step!(late_untrusted_up_1_21_8: a => b, Version::V1_21_6 => Version::V1_21_8, slots::late_to_slot_1_21_6, slots::late_to_untrusted_1_21_8);
untrusted_step!(late_untrusted_down_1_21_6: b => a, Version::V1_21_8 => Version::V1_21_6, slots::late_to_slot_1_21_8, slots::late_to_untrusted_1_21_6);
