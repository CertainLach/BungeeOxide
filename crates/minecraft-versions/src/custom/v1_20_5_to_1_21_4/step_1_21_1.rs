use super::*;
use crate::gen::convert::{v1_20_5 as a, v1_21_1 as b};

const FROM: Version = Version::V1_20_5;
const TO: Version = Version::V1_21_1;

type ConfigA = a::configuration_to_client_packet::Ty;
type ConfigB = b::configuration_to_client_packet::Ty;
type ClientA = a::play_to_client_packet::Ty;
type ClientB = b::play_to_client_packet::Ty;
type ConfigTag = b::configuration_to_client_packet_tags_items::Ty;
type PlayTag = b::play_to_client_packet_tags_items::Ty;
type ConfigTags = Vec<ConfigTag>;
type PlayTags = Vec<PlayTag>;
type Painting = b::entity_metadata_entry_value_painting_variant::Ty;
type Meta = b::entity_metadata_entry_value::Ty;
type MetaA = a::entity_metadata_entry_value::Ty;

const ATTRIBUTES_1_20_5: &[&str] = &[
	"armor",
	"armor_toughness",
	"attack_damage",
	"attack_knockback",
	"attack_speed",
	"block_break_speed",
	"block_interaction_range",
	"entity_interaction_range",
	"fall_damage_multiplier",
	"flying_speed",
	"follow_range",
	"gravity",
	"jump_strength",
	"knockback_resistance",
	"luck",
	"max_absorption",
	"max_health",
	"movement_speed",
	"safe_fall_distance",
	"scale",
	"spawn_reinforcements",
	"step_height",
];

pub(super) const ATTRIBUTES_1_21_1: &[&str] = &[
	"armor",
	"armor_toughness",
	"attack_damage",
	"attack_knockback",
	"attack_speed",
	"block_break_speed",
	"block_interaction_range",
	"burning_time",
	"explosion_knockback_resistance",
	"entity_interaction_range",
	"fall_damage_multiplier",
	"flying_speed",
	"follow_range",
	"gravity",
	"jump_strength",
	"knockback_resistance",
	"luck",
	"max_absorption",
	"max_health",
	"mining_efficiency",
	"movement_efficiency",
	"movement_speed",
	"oxygen_bonus",
	"safe_fall_distance",
	"scale",
	"sneaking_speed",
	"spawn_reinforcements",
	"step_height",
	"submerged_mining_speed",
	"sweeping_damage_ratio",
	"water_movement_efficiency",
];

const PAINTINGS_1_20_5: &[(&str, i32, i32)] = &[
	("kebab", 1, 1),
	("aztec", 1, 1),
	("alban", 1, 1),
	("aztec2", 1, 1),
	("bomb", 1, 1),
	("plant", 1, 1),
	("wasteland", 1, 1),
	("pool", 2, 1),
	("courbet", 2, 1),
	("sea", 2, 1),
	("sunset", 2, 1),
	("creebet", 2, 1),
	("wanderer", 1, 2),
	("graham", 1, 2),
	("match", 2, 2),
	("bust", 2, 2),
	("stage", 2, 2),
	("void", 2, 2),
	("skull_and_roses", 2, 2),
	("wither", 2, 2),
	("fighters", 4, 2),
	("pointer", 4, 4),
	("pigscene", 4, 4),
	("burning_skull", 4, 4),
	("skeleton", 4, 3),
	("earth", 2, 2),
	("wind", 2, 2),
	("water", 2, 2),
	("fire", 2, 2),
	("donkey_kong", 4, 3),
];

const ENCHANTMENT_MENU: i32 = 13;

const MODIFIER_IDS: &[(u128, &str)] = &[
	(0xc1c727718b8eba4a_ace081a93c8928b2, "armor.body"),
	(0x7107de5e7ce84030_940e514c1f160890, "effect.slowness"),
	(
		0x60b1b7dbfffd4ad0_817cd6c6a93d8a45,
		"enchantment.aqua_affinity",
	),
	(0x49455a497ec545ba_b8863b90b23a1718, "attacking"),
	(0x020e0dfb87ae4653_9556831010e291a0, "attacking"),
	(
		0x55fced67e92a486e_9800b47f202c4386,
		"effect.minining_fatigue",
	),
	(
		0xb9716dbd50df4080_850e70347d24e687,
		"enchantment.soul_speed",
	),
	(
		0x92437d00c3a74f2e_8f6c1f21585d5dd0,
		"enchantment.swift_sneak",
	),
	(0x5cd17e52a79a43d3_a52990fde04b181e, "drinking"),
	(
		0x736565d2e1a7403d_a3f81aeb3e302542,
		"creative_mode_block_range",
	),
	(
		0x3ceb37c0db6246b5_bd02785457b01d96,
		"enchantment.efficiency",
	),
	(0x5d6f0ba2118646ac_b896c61c5cee99cc, "effect.health_boost"),
	(0x7e0292f2943448d5_a29f9583af7df27f, "covered"),
	(0xeae29cf0701e4ed6_883a96f798f3dab5, "effect.absorption"),
	(
		0x98491ef697b14584_ae8271a8cc85cf73,
		"creative_mode_entity_range",
	),
	(0xcc5af1422bd24215_b6362605aed11727, "effect.unluck"),
	(0xd8499b040e664726_ab2964469d734e0d, "armor.leggings"),
	(
		0x5d3d087bdebe4037_b53ed84f3ff51f17,
		"enchantment.sweeping_edge",
	),
	(0x91aeaa56376b4498_935b2f7f68070635, "effect.speed"),
	(
		0x87f46a96686f4796_b03522e16ee9e038,
		"enchantment.soul_speed",
	),
	(
		0x07a65791f64d4e79_86c7f83932f007ec,
		"enchantment.respiration",
	),
	(0x9e36292401de4ddd_a2b2d0f7a405a174, "suffocating"),
	(0x662a6b8dda3e4c1c_881396ea6097278d, "sprinting"),
	(0x9f3d476dc1184544_836564846904b48e, "armor.chestplate"),
	(
		0xb572ecd2ac0c4071_abde9594af072a37,
		"enchantment.fire_protection",
	),
	(0x648d70646a604f59_8abec2c23a6dd7a9, "effect.strength"),
	(0xfa233e1c41804865_b01bbcce9785aca3, "base_attack_speed"),
	(0xc0105bf3aef846b0_9ebc92943757ccbe, "effect.jump_boost"),
	(0x03c3c89d70374b42_869fb146bcb64d2e, "effect.luck"),
	(0x1eaf83ff72074596_b37ad7a07b3ec4ce, "powder_snow"),
	(0x845db27cc624495f_8c9f6020a9a58b6b, "armor.boots"),
	(0xaf8b6e3f33284c0a_aa365ba2bb9dbef3, "effect.haste"),
	(0x2ad3f246fee14e67_b88669fd380bb150, "armor.helmet"),
	(0xb9766b5995664402_bc1f2ee2a276d836, "baby"),
	(0x22653b89116e49dc_9b6b9971489b5be5, "effect.weakness"),
	(
		0x40a9968f5c664e2f_b7f42ec2f4b3e450,
		"enchantment.blast_protection",
	),
	(0x6555be7463b341f1_a24577833b3c2562, "evil"),
	(0x766bfa6411f311ea_8d71362b9e155667, "baby"),
	(
		0x11dc269a447646c0_aff39e17d7eb6801,
		"enchantment.depth_strider",
	),
	(0xcb3f55d3645c4f38_a4979c13a33db5cf, "base_attack_damage"),
];

const MODIFIER_NAMES: &[(&str, &str)] = &[
	("Random spawn bonus", "random_spawn_bonus"),
	("Random zombie-spawn bonus", "zombie_random_spawn_bonus"),
	("Leader zombie bonus", "leader_zombie_bonus"),
	(
		"Zombie reinforcement callee charge",
		"reinforcement_callee_charge",
	),
	(
		"Zombie reinforcement caller charge",
		"reinforcement_caller_charge",
	),
];

fn uuid_text(uuid: u128) -> String {
	let h = format!("{uuid:032x}");
	format!(
		"{}-{}-{}-{}-{}",
		&h[0..8],
		&h[8..12],
		&h[12..16],
		&h[16..20],
		&h[20..32]
	)
}

fn modifier_id(uuid: u128, name: Option<&str>) -> String {
	if let Some((_, id)) = MODIFIER_IDS.iter().find(|(u, _)| *u == uuid) {
		return format!("minecraft:{id}");
	}
	if let Some((_, id)) = name.and_then(|n| MODIFIER_NAMES.iter().find(|(old, _)| *old == n)) {
		return format!("minecraft:{id}");
	}
	format!("minecraft:{}", uuid_text(uuid))
}

fn modifier_uuid(id: &str) -> u128 {
	let id = strip(id);
	if let Some((uuid, _)) = MODIFIER_IDS.iter().find(|(_, i)| *i == id) {
		return *uuid;
	}
	let hex: String = id.chars().filter(|c| *c != '-').collect();
	if hex.len() == 32 {
		if let Ok(v) = u128::from_str_radix(&hex, 16) {
			return v;
		}
	}
	id.bytes()
		.fold(0x6c62272e07bb014262b821756295c58d, |h: u128, b| {
			(h ^ b as u128).wrapping_mul(0x1000000000000000000013b)
		})
}

fn modifier_name(id: &str) -> String {
	MODIFIER_NAMES
		.iter()
		.find(|(_, i)| *i == strip(id))
		.map_or_else(|| strip(id).to_owned(), |(n, _)| (*n).to_owned())
}

pub(super) fn attribute(from: &[&str], to: &[&str], id: i32) -> Result<i32> {
	let name = usize::try_from(id)
		.ok()
		.and_then(|i| from.get(i))
		.ok_or(Error::Dropped)?;
	to.iter()
		.position(|n| n == name)
		.map(|i| i as i32)
		.ok_or(Error::Dropped)
}

fn enchantment_up(cx: &mut Session, id: i32) -> Option<i32> {
	let name = registry::names(FROM, "enchantments")?.name(u32::try_from(id).ok()?)?;
	registry_index(cx, TO, "enchantment", name)
}

fn enchantment_down(cx: &mut Session, id: i32) -> Option<i32> {
	let key = registry_key(cx, TO, "enchantment", id)?;
	registry::names(FROM, "enchantments")?
		.id(&key)
		.map(|v| v as i32)
}

pub fn tricky_registry_data_1_21_1(p: RegistryData, cx: &mut Session) -> Result<Vec<ConfigB>> {
	Ok(vec![ConfigB::RegistryData(registry_up(
		cx,
		(FROM, TO),
		p,
		no_fix,
	))])
}

pub fn tricky_registry_data_down_1_20_5(p: RegistryData, cx: &mut Session) -> Result<Vec<ConfigA>> {
	Ok(vec![ConfigA::RegistryData(registry_down(
		cx,
		(TO, FROM),
		p,
		no_fix,
	)?)])
}

pub fn tricky_finish_configuration_1_21_1(
	p: a::configuration_to_client_packet_finish_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<ConfigB>> {
	let mut out: Vec<ConfigB> = missing_registries(cx, TO, no_fix)
		.into_iter()
		.map(ConfigB::RegistryData)
		.collect();
	out.push(ConfigB::FinishConfiguration(p));
	Ok(out)
}

pub fn tricky_feature_flags_1_21_1(p: Vec<String>, _cx: &mut Session) -> Result<Vec<ConfigB>> {
	Ok(vec![ConfigB::FeatureFlags(features(
		p,
		&[],
		&["update_1_21"],
	))])
}

pub fn tricky_feature_flags_down_1_20_5(p: Vec<String>, _cx: &mut Session) -> Result<Vec<ConfigA>> {
	Ok(vec![ConfigA::FeatureFlags(features(
		p,
		&["update_1_21"],
		&[],
	))])
}

fn required_tags(registry: &str) -> Vec<(&'static str, Vec<i32>)> {
	let id = |n: &str| {
		registry::names(TO, "enchantments")
			.and_then(|e| e.id(n))
			.map(|v| v as i32)
	};
	let empty = |names: &[&'static str]| names.iter().map(|n| (*n, vec![])).collect::<Vec<_>>();
	match strip(registry) {
		"enchantment" => [
			(
				"curse",
				["binding_curse", "vanishing_curse"]
					.iter()
					.filter_map(|n| id(n))
					.collect(),
			),
			(
				"prevents_decorated_pot_shattering",
				id("silk_touch").into_iter().collect(),
			),
		]
		.into_iter()
		.chain(empty(&[
			"double_trade_price",
			"in_enchanting_table",
			"non_treasure",
			"on_mob_spawn_equipment",
			"on_random_loot",
			"on_traded_equipment",
			"prevents_bee_spawns_when_mining",
			"prevents_ice_melting",
			"prevents_infested_spawns",
			"smelts_loot",
			"tooltip_order",
			"tradeable",
			"treasure",
			"exclusive_set/armor",
			"exclusive_set/boots",
			"exclusive_set/bow",
			"exclusive_set/crossbow",
			"exclusive_set/damage",
			"exclusive_set/mining",
			"exclusive_set/riptide",
		]))
		.collect(),
		"item" => empty(&[
			"enchantable/head_armor",
			"enchantable/chest_armor",
			"enchantable/leg_armor",
			"enchantable/foot_armor",
			"enchantable/armor",
			"enchantable/equippable",
			"enchantable/weapon",
			"enchantable/sword",
			"enchantable/sharp_weapon",
			"enchantable/fire_aspect",
			"enchantable/mace",
			"enchantable/trident",
			"enchantable/mining",
			"enchantable/mining_loot",
			"enchantable/bow",
			"enchantable/crossbow",
			"enchantable/fishing",
			"enchantable/durability",
			"enchantable/vanishing",
		]),
		"entity_type" => empty(&[
			"sensitive_to_bane_of_arthropods",
			"sensitive_to_impaling",
			"sensitive_to_smite",
			"arrows",
			"can_turn_in_boats",
			"deflects_projectiles",
			"immune_to_infested",
			"immune_to_oozing",
			"no_anger_from_wind_charge",
		]),
		"block" => empty(&["soul_speed_blocks", "blocks_wind_charge_explosions"]),
		_ => vec![],
	}
}

fn tags_up(registry: &str, tags: &mut Vec<TagList>) {
	remap_tags(FROM, TO, registry, tags);
	ensure_tags(tags, &required_tags(registry));
}

macro_rules! tag_groups {
	($groups:expr, $ty:ident, $registries:expr) => {{
		let mut groups = $groups;
		for g in &mut groups {
			tags_up(&g.tag_type, &mut g.tags);
		}
		for registry in $registries {
			if !groups.iter().any(|g| strip(&g.tag_type) == strip(registry)) {
				let mut tags = vec![];
				tags_up(registry, &mut tags);
				groups.push($ty {
					tag_type: registry.to_owned(),
					tags,
				});
			}
		}
		groups
	}};
}

pub fn tricky_config_tags_1_21_1(p: ConfigTags, _cx: &mut Session) -> Result<Vec<ConfigB>> {
	Ok(vec![ConfigB::Tags(tag_groups!(
		p,
		ConfigTag,
		[
			"minecraft:enchantment",
			"minecraft:item",
			"minecraft:entity_type",
			"minecraft:block"
		]
	))])
}

pub fn tricky_play_tags_1_21_1(p: PlayTags, _cx: &mut Session) -> Result<Vec<ClientB>> {
	Ok(vec![ClientB::Tags(tag_groups!(
		p,
		PlayTag,
		[
			"minecraft:enchantment",
			"minecraft:item",
			"minecraft:entity_type",
			"minecraft:block"
		]
	))])
}

pub fn tricky_config_tags_down_1_20_5(
	mut p: ConfigTags,
	_cx: &mut Session,
) -> Result<Vec<ConfigA>> {
	p.retain(|g| strip(&g.tag_type) != "enchantment");
	for g in &mut p {
		remap_tags(TO, FROM, &g.tag_type, &mut g.tags);
	}
	Ok(vec![ConfigA::Tags(p)])
}

pub fn tricky_play_tags_down_1_20_5(mut p: PlayTags, _cx: &mut Session) -> Result<Vec<ClientA>> {
	p.retain(|g| strip(&g.tag_type) != "enchantment");
	for g in &mut p {
		remap_tags(TO, FROM, &g.tag_type, &mut g.tags);
	}
	Ok(vec![ClientA::Tags(p)])
}

pub fn tricky_item_id_1_21_1(src: &a::slot_default::Ty, _cx: &mut Session) -> Result<i32> {
	Ok(item(FROM, TO, src.item_id))
}

pub fn tricky_item_id_down_1_20_5(src: &b::slot_default::Ty, _cx: &mut Session) -> Result<i32> {
	Ok(item(TO, FROM, src.item_id))
}

fn kept(c: &b::slot_component::Ty) -> bool {
	!matches!(c, b::slot_component::Ty::JukeboxPlayable(_))
}

pub fn tricky_added_components_down_1_20_5(
	src: &b::slot_default::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(src.components.iter().filter(|c| kept(c)).count() as i32)
}

pub fn tricky_removed_components_down_1_20_5(
	src: &b::slot_default::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(src
		.remove_components
		.iter()
		.filter(|c| !matches!(c, b::slot_component_type::Ty::JukeboxPlayable))
		.count() as i32)
}

pub fn tricky_trade_components_down_1_20_5(
	src: &b::play_to_client_packet_trade_list_trades_items_input_item1::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(src.components.iter().filter(|c| kept(c)).count() as i32)
}

pub fn tricky_trade_components2_down_1_20_5(
	src: &b::play_to_client_packet_trade_list_trades_items_input_item2_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(src.components.iter().filter(|c| kept(c)).count() as i32)
}

pub fn tricky_trade_item_1_21_1(
	src: &a::play_to_client_packet_trade_list_trades_items_input_item1::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item(FROM, TO, src.item_id))
}

pub fn tricky_trade_item_down_1_20_5(
	src: &b::play_to_client_packet_trade_list_trades_items_input_item1::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item(TO, FROM, src.item_id))
}

pub fn tricky_trade_item2_1_21_1(
	src: &a::play_to_client_packet_trade_list_trades_items_input_item2_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item(FROM, TO, src.item_id))
}

pub fn tricky_trade_item2_down_1_20_5(
	src: &b::play_to_client_packet_trade_list_trades_items_input_item2_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item(TO, FROM, src.item_id))
}

pub fn tricky_enchantment_1_21_1(
	src: &a::slot_component_enchantments_enchantments_items::Ty,
	cx: &mut Session,
) -> Result<i32> {
	enchantment_up(cx, src.id).ok_or(Error::Dropped)
}

pub fn tricky_enchantment_down_1_20_5(
	src: &b::slot_component_enchantments_enchantments_items::Ty,
	cx: &mut Session,
) -> Result<i32> {
	enchantment_down(cx, src.id).ok_or(Error::Dropped)
}

pub fn tricky_stored_enchantment_1_21_1(
	src: &a::slot_component_stored_enchantments_enchantments_items::Ty,
	cx: &mut Session,
) -> Result<i32> {
	enchantment_up(cx, src.id).ok_or(Error::Dropped)
}

pub fn tricky_stored_enchantment_down_1_20_5(
	src: &b::slot_component_stored_enchantments_enchantments_items::Ty,
	cx: &mut Session,
) -> Result<i32> {
	enchantment_down(cx, src.id).ok_or(Error::Dropped)
}

pub fn tricky_pot_decorations_1_21_1(
	p: Vec<i32>,
	_cx: &mut Session,
) -> Result<b::slot_component::Ty> {
	Ok(b::slot_component::Ty::PotDecorations(
		p.into_iter().map(|i| item(FROM, TO, i)).collect(),
	))
}

pub fn tricky_pot_decorations_down_1_20_5(
	p: Vec<i32>,
	_cx: &mut Session,
) -> Result<a::slot_component::Ty> {
	Ok(a::slot_component::Ty::PotDecorations(
		p.into_iter().map(|i| item(TO, FROM, i)).collect(),
	))
}

pub fn tricky_food_remainder_1_21_1(
	_src: &a::slot_component_food::Ty,
	_cx: &mut Session,
) -> Result<b::slot::Ty> {
	Ok(b::slot::Ty::V0)
}

pub fn tricky_potion_name_down_1_20_5(
	_src: &b::slot_component_potion_contents::Ty,
	_cx: &mut Session,
) -> Result<Option<String>> {
	Ok(None)
}

pub fn tricky_attribute_type_1_21_1(
	src: &a::slot_component_attribute_modifiers_attributes_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	attribute(ATTRIBUTES_1_20_5, ATTRIBUTES_1_21_1, src.type_id)
}

pub fn tricky_attribute_type_down_1_20_5(
	src: &b::slot_component_attribute_modifiers_attributes_items::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	attribute(ATTRIBUTES_1_21_1, ATTRIBUTES_1_20_5, src.type_id)
}

pub fn tricky_attribute_name_1_21_1(
	src: &a::slot_component_attribute_modifiers_attributes_items::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(modifier_id(src.uuid, Some(&src.name)))
}

pub fn tricky_attribute_name_down_1_20_5(
	src: &b::slot_component_attribute_modifiers_attributes_items::Ty,
	_cx: &mut Session,
) -> Result<String> {
	Ok(modifier_name(&src.name))
}

pub fn tricky_attribute_uuid_down_1_20_5(
	src: &b::slot_component_attribute_modifiers_attributes_items::Ty,
	_cx: &mut Session,
) -> Result<u128> {
	Ok(modifier_uuid(&src.name))
}

pub fn tricky_modifier_id_1_21_1(uuid: u128, _cx: &mut Session) -> Result<String> {
	Ok(modifier_id(uuid, None))
}

pub fn tricky_modifier_uuid_down_1_20_5(id: String, _cx: &mut Session) -> Result<u128> {
	Ok(modifier_uuid(&id))
}

fn painting_size(cx: &mut Session, id: i32) -> Option<(i32, i32)> {
	let tag = registry_entry(cx, TO, "painting_variant", id)?;
	let fields = tag.value.as_compound()?;
	let int = |name: &str| match fields.field(name)? {
		Payload::Int(v) => Some(*v),
		_ => None,
	};
	Some((int("width")?, int("height")?))
}

fn painting_down(cx: &mut Session, id: i32) -> i32 {
	let key = registry_key(cx, TO, "painting_variant", id);
	if let Some(at) = key.and_then(|k| PAINTINGS_1_20_5.iter().position(|(n, _, _)| *n == k)) {
		return at as i32;
	}
	let size = painting_size(cx, id);
	PAINTINGS_1_20_5
		.iter()
		.position(|(_, w, h)| Some((*w, *h)) == size)
		.unwrap_or(0) as i32
}

pub fn tricky_painting_1_21_1(id: i32, cx: &mut Session) -> Result<Meta> {
	let (name, _, _) = usize::try_from(id)
		.ok()
		.and_then(|i| PAINTINGS_1_20_5.get(i))
		.ok_or(Error::Dropped)?;
	Ok(Meta::PaintingVariant(Painting::Id(
		registry_index(cx, TO, "painting_variant", name).ok_or(Error::Dropped)?,
	)))
}

pub fn tricky_painting_down_1_20_5(p: Painting, cx: &mut Session) -> Result<MetaA> {
	let id = match p {
		Painting::Id(id) => painting_down(cx, id),
		Painting::Inline(v) => {
			let asset = v.asset_id.strip_prefix("minecraft:").unwrap_or(&v.asset_id);
			PAINTINGS_1_20_5
				.iter()
				.position(|(n, _, _)| *n == asset)
				.or_else(|| {
					PAINTINGS_1_20_5
						.iter()
						.position(|(_, w, h)| *w == v.width && *h == v.height)
				})
				.ok_or(Error::Dropped)? as i32
		}
	};
	Ok(MetaA::PaintingVariant(id))
}

pub fn tricky_chat_type_1_21_1(
	id: i32,
	_cx: &mut Session,
) -> Result<b::play_to_client_chat_types_holder::Ty> {
	Ok(b::play_to_client_chat_types_holder::Ty::Id(id))
}

fn decoration(chat: b::play_to_client_chat_type::Ty) -> Decoration {
	use b::play_to_client_chat_type_parameter_type::Ty as P;
	Decoration {
		key: chat.translation_key,
		parameters: chat
			.parameters
			.iter()
			.map(|p| match p {
				P::Sender => Parameter::Sender,
				P::Target => Parameter::Target,
				_ => Parameter::Content,
			})
			.collect(),
		style: chat.style.value.into_compound().unwrap_or_default(),
	}
}

fn registered(cx: &mut Session, chat: b::play_to_client_chat_type::Ty) -> i32 {
	let wanted = Some(decoration(chat));
	let mut id = 0;
	while registry_key(cx, FROM, "chat_type", id).is_some() {
		if registry_entry(cx, FROM, "chat_type", id).and_then(|t| ChatType::read(&t.value).chat)
			== wanted
		{
			return id;
		}
		id += 1;
	}
	registry_index(cx, FROM, "chat_type", "chat").unwrap_or(0)
}

fn system_chat(
	cx: &mut Session,
	chat: b::play_to_client_chat_type::Ty,
	sender: &Tag,
	target: Option<&Tag>,
	content: &Tag,
) -> Result<ClientA> {
	let mut content = decoration(chat).nbt(sender, target, content);
	localize(&mut content, TO, FROM, cx)?;
	Ok(ClientA::SystemChat(
		a::play_to_client_packet_system_chat::Ty {
			content,
			is_action_bar: false,
		},
	))
}

pub fn tricky_profileless_chat_down_1_20_5(
	p: b::play_to_client_packet_profileless_chat::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	Ok(vec![match p.r#type {
		b::play_to_client_chat_types_holder::Ty::Id(id) => {
			ClientA::ProfilelessChat(a::play_to_client_packet_profileless_chat::Ty {
				message: p.message,
				r#type: id,
				name: p.name,
				target: p.target,
			})
		}
		b::play_to_client_chat_types_holder::Ty::Inline(t) => {
			system_chat(cx, t.chat, &p.name, p.target.as_ref(), &p.message)?
		}
	}])
}

pub fn tricky_player_chat_down_1_20_5(
	p: b::play_to_client_packet_player_chat::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientA>> {
	let r#type = match p.r#type {
		b::play_to_client_chat_types_holder::Ty::Id(id) => id,
		b::play_to_client_chat_types_holder::Ty::Inline(t) if p.signature.is_none() => {
			let content = p.unsigned_chat_content.unwrap_or(Tag::new(p.plain_message));
			return Ok(vec![system_chat(
				cx,
				t.chat,
				&p.network_name,
				p.network_target_name.as_ref(),
				&content,
			)?]);
		}
		b::play_to_client_chat_types_holder::Ty::Inline(t) => registered(cx, t.chat),
	};
	use a::play_to_client_packet_player_chat_filter_type_mask::Ty as MaskA;
	use b::play_to_client_packet_player_chat_filter_type_mask::Ty as MaskB;
	Ok(vec![ClientA::PlayerChat(
		a::play_to_client_packet_player_chat::Ty {
			sender_uuid: p.sender_uuid,
			index: p.index,
			signature: p.signature,
			plain_message: p.plain_message,
			timestamp: p.timestamp,
			salt: p.salt,
			previous_messages: p.previous_messages,
			unsigned_chat_content: p.unsigned_chat_content,
			filter_type_mask: match p.filter_type_mask {
				MaskB::V2(mask) => MaskA::V2(mask),
				MaskB::Default(v) => MaskA::Default(v),
			},
			r#type,
			network_name: p.network_name,
			network_target_name: p.network_target_name,
		},
	)])
}

pub fn tricky_projectile_power_1_21_1(
	src: &a::play_to_client_packet_set_projectile_power::Ty,
	_cx: &mut Session,
) -> Result<f64> {
	let p = &src.power;
	Ok((p.x * p.x + p.y * p.y + p.z * p.z).sqrt())
}

const POWERED: &[&str] = &[
	"fireball",
	"small_fireball",
	"dragon_fireball",
	"wither_skull",
	"wind_charge",
	"breeze_wind_charge",
];

#[derive(Default)]
struct Projectiles(HashMap<i32, (f64, f64, f64)>);

fn motion(v: &b::vec3i16::Ty) -> (f64, f64, f64) {
	(
		v.x as f64 / 8000.0,
		v.y as f64 / 8000.0,
		v.z as f64 / 8000.0,
	)
}

pub fn tricky_spawn_entity_down_1_20_5(
	p: b::play_to_client_packet_spawn_entity::Ty,
	cx: &mut Session,
) -> Result<a::play_to_client_packet_spawn_entity::Ty> {
	if entity_name(TO, p.r#type).is_some_and(|n| POWERED.contains(&n)) {
		cx.state::<Projectiles>()
			.0
			.insert(p.entity_id, motion(&p.velocity));
	}
	Ok(p)
}

pub fn tricky_entity_velocity_down_1_20_5(
	p: b::play_to_client_packet_entity_velocity::Ty,
	cx: &mut Session,
) -> Result<a::play_to_client_packet_entity_velocity::Ty> {
	if let Some(v) = cx.state::<Projectiles>().0.get_mut(&p.entity_id) {
		*v = motion(&p.velocity);
	}
	Ok(p)
}

pub fn tricky_entity_destroy_down_1_20_5(p: Vec<i32>, cx: &mut Session) -> Result<Vec<ClientA>> {
	let projectiles = &mut cx.state::<Projectiles>().0;
	for id in &p {
		projectiles.remove(id);
	}
	Ok(vec![ClientA::EntityDestroy(p)])
}

pub fn tricky_projectile_power_down_1_20_5(
	src: &b::play_to_client_packet_set_projectile_power::Ty,
	cx: &mut Session,
) -> Result<a::vec3f64::Ty> {
	let (x, y, z) = cx
		.state::<Projectiles>()
		.0
		.get(&src.id)
		.copied()
		.ok_or(Error::Dropped)?;
	let length = (x * x + y * y + z * z).sqrt();
	if length < 1.0e-7 {
		return Err(Error::Dropped);
	}
	let scale = src.acceleration_power / length;
	Ok(a::vec3f64::Ty {
		x: x * scale,
		y: y * scale,
		z: z * scale,
	})
}

const DISCS: &str = "music_disc_";

pub fn tricky_world_event_1_21_1(
	src: &a::play_to_client_packet_world_event::Ty,
	cx: &mut Session,
) -> Result<i32> {
	if src.effect_id != 1010 {
		return Ok(src.data);
	}
	let name = registry::names(FROM, "items")
		.and_then(|n| n.name(u32::try_from(src.data).ok()?))
		.ok_or(Error::Dropped)?;
	let song = name.strip_prefix(DISCS).ok_or(Error::Dropped)?;
	registry_index(cx, TO, "jukebox_song", song).ok_or(Error::Dropped)
}

pub fn tricky_world_event_down_1_20_5(
	src: &b::play_to_client_packet_world_event::Ty,
	cx: &mut Session,
) -> Result<i32> {
	if src.effect_id != 1010 {
		return Ok(src.data);
	}
	let song = registry_key(cx, TO, "jukebox_song", src.data).ok_or(Error::Dropped)?;
	item_id(TO, &format!("{DISCS}{song}"))
		.map(|i| item(TO, FROM, i))
		.ok_or(Error::Dropped)
}

pub fn tricky_sound_1_21_1(holder: SoundHolder, _cx: &mut Session) -> Result<SoundHolder> {
	Ok(sound(FROM, TO, holder))
}

pub fn tricky_sound_down_1_20_5(holder: SoundHolder, _cx: &mut Session) -> Result<SoundHolder> {
	Ok(sound(TO, FROM, holder))
}

pub fn tricky_cooldown_item_1_21_1(
	src: &a::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item(FROM, TO, src.item_id))
}

pub fn tricky_cooldown_item_down_1_20_5(
	src: &b::play_to_client_packet_set_cooldown::Ty,
	_cx: &mut Session,
) -> Result<i32> {
	Ok(item(TO, FROM, src.item_id))
}

pub fn tricky_game_state_value_down_1_20_5(
	src: &b::play_to_client_packet_game_state_change::Ty,
	_cx: &mut Session,
) -> Result<f32> {
	Ok(match src.reason {
		b::play_to_client_packet_game_state_change_reason::Ty::WinGame => 1.0,
		_ => src.game_mode,
	})
}

#[derive(Default)]
struct Menus(HashMap<i32, i32>);

pub fn tricky_open_window_1_21_1(
	p: a::play_to_client_packet_open_window::Ty,
	cx: &mut Session,
) -> Result<b::play_to_client_packet_open_window::Ty> {
	cx.state::<Menus>().0.insert(p.window_id, p.inventory_type);
	Ok(p)
}

pub fn tricky_open_window_down_1_20_5(
	p: b::play_to_client_packet_open_window::Ty,
	cx: &mut Session,
) -> Result<a::play_to_client_packet_open_window::Ty> {
	cx.state::<Menus>().0.insert(p.window_id, p.inventory_type);
	Ok(p)
}

fn enchantment_hint(
	cx: &mut Session,
	window: u8,
	property: i16,
	value: i16,
	map: fn(&mut Session, i32) -> Option<i32>,
) -> i16 {
	let menu = cx.state::<Menus>().0.get(&(window as i32)).copied();
	if menu != Some(ENCHANTMENT_MENU) || !(4..=6).contains(&property) || value < 0 {
		return value;
	}
	map(cx, value as i32).map_or(-1, |v| v as i16)
}

pub fn tricky_window_property_1_21_1(
	src: &a::play_to_client_packet_craft_progress_bar::Ty,
	cx: &mut Session,
) -> Result<i16> {
	Ok(enchantment_hint(
		cx,
		src.window_id,
		src.property,
		src.value,
		enchantment_up,
	))
}

pub fn tricky_window_property_down_1_20_5(
	src: &b::play_to_client_packet_craft_progress_bar::Ty,
	cx: &mut Session,
) -> Result<i16> {
	Ok(enchantment_hint(
		cx,
		src.window_id,
		src.property,
		src.value,
		enchantment_down,
	))
}

#[derive(Default)]
struct Rotation {
	yaw: f32,
	pitch: f32,
}

pub fn tricky_look_1_21_1(
	p: a::play_to_server_packet_look::Ty,
	cx: &mut Session,
) -> Result<b::play_to_server_packet_look::Ty> {
	*cx.state::<Rotation>() = Rotation {
		yaw: p.yaw,
		pitch: p.pitch,
	};
	Ok(p)
}

pub fn tricky_position_look_1_21_1(
	p: a::play_to_server_packet_position_look::Ty,
	cx: &mut Session,
) -> Result<b::play_to_server_packet_position_look::Ty> {
	*cx.state::<Rotation>() = Rotation {
		yaw: p.yaw,
		pitch: p.pitch,
	};
	Ok(p)
}

pub fn tricky_use_item_rotation_1_21_1(
	_src: &a::play_to_server_packet_use_item::Ty,
	cx: &mut Session,
) -> Result<b::vec2f::Ty> {
	let r = cx.state::<Rotation>();
	Ok(b::vec2f::Ty {
		x: r.yaw,
		y: r.pitch,
	})
}
