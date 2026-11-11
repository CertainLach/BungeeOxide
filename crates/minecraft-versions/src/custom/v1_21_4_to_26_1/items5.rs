use super::components;
use super::remap;
use super::text;
use super::util::recode;
use crate::gen::convert::{v1_21_4 as a, v1_21_5 as b, Version};
use crate::pds::convert::{Error, Result};
use nbt::{Payload, Tag};

const OLD: Version = Version::V1_21_4;
const NEW: Version = Version::V1_21_5;

const HIDE_ADDITIONAL: &[&str] = &[
	"banner_patterns",
	"bees",
	"block_entity_data",
	"block_state",
	"bundle_contents",
	"charged_projectiles",
	"container",
	"container_loot",
	"firework_explosion",
	"fireworks",
	"instrument",
	"map_id",
	"painting_variant",
	"pot_decorations",
	"potion_contents",
	"tropical_fish_pattern",
	"written_book_content",
];

fn item_name(version: Version, id: i32) -> &'static str {
	crate::registry::names(version, "items")
		.and_then(|n| n.name(id.max(0) as u32))
		.unwrap_or("air")
}

fn hide(hidden: &mut Vec<i32>, name: &str) {
	if let Some(id) = components::id(NEW, name) {
		if !hidden.contains(&id) {
			hidden.push(id);
		}
	}
}

fn sound_up(v: b::item_sound_holder::Ty) -> Result<b::item_sound_holder::Ty> {
	remap::sound(OLD, NEW, v)
}

fn sound_down(v: b::item_sound_holder::Ty) -> Result<b::item_sound_holder::Ty> {
	remap::sound(NEW, OLD, v)
}

fn effects(
	v: Vec<b::item_consume_effect::Ty>,
	sound: impl Fn(b::item_sound_holder::Ty) -> Result<b::item_sound_holder::Ty>,
) -> Result<Vec<b::item_consume_effect::Ty>> {
	v.into_iter()
		.map(|e| match e {
			b::item_consume_effect::Ty::PlaySound(s) => {
				Ok(b::item_consume_effect::Ty::PlaySound(sound(s)?))
			}
			e => Ok(e),
		})
		.collect()
}

fn text_up(t: nbt::Tag) -> nbt::Tag {
	text::up(t)
}

fn predicates_up(v: Vec<a::item_block_predicate::Ty>) -> Result<Vec<b::item_block_predicate::Ty>> {
	v.into_iter()
		.map(|p| {
			Ok(b::item_block_predicate::Ty {
				block_set: p.block_set.map(|s| match s {
					a::item_block_predicate_block_set_items::Ty::Named(n) => {
						b::item_block_predicate_block_set_items::Ty::Named(n)
					}
					a::item_block_predicate_block_set_items::Ty::Ids(ids) => {
						b::item_block_predicate_block_set_items::Ty::Ids(
							ids.into_iter()
								.filter_map(|i| super::util::remap("blocks", OLD, NEW, i))
								.collect(),
						)
					}
				}),
				properties: p.properties,
				nbt: p.nbt,
				components: b::data_component_matchers::Ty {
					exact_matchers: Vec::new(),
					partial_matchers: Vec::new(),
				},
			})
		})
		.collect()
}

fn predicates_down(
	v: Vec<b::item_block_predicate::Ty>,
	show: bool,
) -> a::slot_component_can_place_on::Ty {
	a::slot_component_can_place_on::Ty {
		predicates: v
			.into_iter()
			.map(|p| a::item_block_predicate::Ty {
				block_set: p.block_set.map(|s| match s {
					b::item_block_predicate_block_set_items::Ty::Named(n) => {
						a::item_block_predicate_block_set_items::Ty::Named(n)
					}
					b::item_block_predicate_block_set_items::Ty::Ids(ids) => {
						a::item_block_predicate_block_set_items::Ty::Ids(
							ids.into_iter()
								.filter_map(|i| super::util::remap("blocks", NEW, OLD, i))
								.collect(),
						)
					}
				}),
				properties: p.properties,
				nbt: p.nbt,
			})
			.collect(),
		show_tooltip: show,
	}
}

fn material_up(m: a::slot_component_trim_material::Ty) -> b::slot_component_trim_material::Ty {
	match m {
		a::slot_component_trim_material::Ty::Id(i) => b::slot_component_trim_material::Ty::Id(i),
		a::slot_component_trim_material::Ty::Inline(m) => {
			b::slot_component_trim_material::Ty::Inline(b::armor_trim_material::Ty {
				asset_base: m.asset_name,
				override_armor_assets: m
					.override_armor_assets
					.into_iter()
					.map(|o| b::armor_trim_material_override_armor_assets_items::Ty {
						key: o.key,
						value: o.value,
					})
					.collect(),
				description: text_up(m.description),
			})
		}
	}
}

fn pattern_up(p: a::slot_component_trim_pattern::Ty) -> b::slot_component_trim_pattern::Ty {
	match p {
		a::slot_component_trim_pattern::Ty::Id(i) => b::slot_component_trim_pattern::Ty::Id(i),
		a::slot_component_trim_pattern::Ty::Inline(p) => {
			b::slot_component_trim_pattern::Ty::Inline(b::armor_trim_pattern::Ty {
				asset_id: p.asset_id,
				description: text_up(p.description),
				decal: p.decal,
			})
		}
	}
}

fn material_down(m: b::slot_component_trim_material::Ty) -> a::slot_component_trim_material::Ty {
	match m {
		b::slot_component_trim_material::Ty::Id(i) => a::slot_component_trim_material::Ty::Id(i),
		b::slot_component_trim_material::Ty::Inline(m) => {
			a::slot_component_trim_material::Ty::Inline(a::armor_trim_material::Ty {
				asset_name: m.asset_base,
				ingredient_id: 0,
				override_armor_assets: m
					.override_armor_assets
					.into_iter()
					.map(|o| a::armor_trim_material_override_armor_assets_items::Ty {
						key: o.key,
						value: o.value,
					})
					.collect(),
				description: m.description,
			})
		}
	}
}

fn pattern_down(p: b::slot_component_trim_pattern::Ty) -> a::slot_component_trim_pattern::Ty {
	match p {
		b::slot_component_trim_pattern::Ty::Id(i) => a::slot_component_trim_pattern::Ty::Id(i),
		b::slot_component_trim_pattern::Ty::Inline(p) => {
			a::slot_component_trim_pattern::Ty::Inline(a::armor_trim_pattern::Ty {
				asset_id: p.asset_id,
				template_item_id: 0,
				description: p.description,
				decal: p.decal,
			})
		}
	}
}

fn bucket_up(mut tag: nbt::Tag, out: &mut Vec<b::slot_component::Ty>) -> nbt::Tag {
	if let Some(v) = tag
		.value
		.field("BucketVariantTag")
		.and_then(Payload::as_i64)
	{
		tag.value.take("BucketVariantTag");
		let v = v as i32;
		out.push(b::slot_component::Ty::TropicalFishPattern(v & 0xffff));
		out.push(b::slot_component::Ty::TropicalFishBaseColor(
			(v >> 16) & 0xff,
		));
		out.push(b::slot_component::Ty::TropicalFishPatternColor(
			(v >> 24) & 0xff,
		));
	}
	tag
}

fn blocks_attacks() -> b::slot_component::Ty {
	b::slot_component::Ty::BlocksAttacks(b::slot_component_blocks_attacks::Ty {
		block_delay_seconds: 0.0,
		disable_cooldown_scale: 0.0,
		damage_reductions: vec![
			b::slot_component_blocks_attacks_damage_reductions_items::Ty {
				horizontal_blocking_angle: 90.0,
				r#type: None,
				base: -0.5,
				factor: 0.5,
			},
		],
		item_damage: b::slot_component_blocks_attacks_item_damage::Ty {
			threshold: 0.0,
			base: 0.0,
			factor: 0.0,
		},
		bypassed_by: None,
		block_sound: None,
		disable_sound: None,
	})
}

fn retag_up(c: &a::slot_component::Ty) -> Result<Option<b::slot_component::Ty>> {
	components::retag(
		|t| components::remap(OLD, NEW, t),
		c,
		a::slot_component::serialize,
		b::slot_component::deserialize,
	)
}

fn retag_down(c: &b::slot_component::Ty) -> Result<Option<a::slot_component::Ty>> {
	components::retag(
		|t| components::remap(NEW, OLD, t),
		c,
		b::slot_component::serialize,
		a::slot_component::deserialize,
	)
}

fn type_up(t: &a::slot_component_type::Ty) -> Result<Option<b::slot_component_type::Ty>> {
	components::retag(
		|t| components::remap(OLD, NEW, t),
		t,
		a::slot_component_type::serialize,
		b::slot_component_type::deserialize,
	)
}

fn type_down(t: &b::slot_component_type::Ty) -> Result<Option<a::slot_component_type::Ty>> {
	components::retag(
		|t| components::remap(NEW, OLD, t),
		t,
		b::slot_component_type::serialize,
		a::slot_component_type::deserialize,
	)
}

pub fn slots_up(v: Vec<a::slot::Ty>) -> Result<Vec<b::slot::Ty>> {
	v.into_iter().map(slot_up).collect()
}

pub fn slot_up(s: a::slot::Ty) -> Result<b::slot::Ty> {
	let (count, d) = match s {
		a::slot::Ty::V0 => return Ok(b::slot::Ty::V0),
		a::slot::Ty::Default(count, d) => (count, d),
	};
	let item_id = remap::item(OLD, NEW, d.item_id)?;
	let item = item_name(NEW, item_id);
	let sword = item.ends_with("_sword");
	let mut out = Vec::with_capacity(d.components.len());
	let mut hidden = Vec::new();
	let mut hide_tooltip = false;
	for c in d.components {
		use a::slot_component::Ty as A;
		use b::slot_component::Ty as B;
		let converted = match c {
			A::HideTooltip => {
				hide_tooltip = true;
				continue;
			}
			A::HideAdditionalTooltip => {
				for name in HIDE_ADDITIONAL {
					hide(&mut hidden, name);
				}
				continue;
			}
			A::CustomName(t) => B::CustomName(text_up(t)),
			A::ItemName(t) => B::ItemName(text_up(t)),
			A::Lore(lines) => B::Lore(
				lines
					.into_iter()
					.map(|l| text_up(l.unwrap_or_else(|| Tag::new(Payload::from("")))))
					.collect(),
			),
			A::Unbreakable(show) => {
				if !show {
					hide(&mut hidden, "unbreakable");
				}
				B::Unbreakable
			}
			A::Enchantments(e) => {
				if !e.show_tooltip {
					hide(&mut hidden, "enchantments");
				}
				B::Enchantments(
					e.enchantments
						.into_iter()
						.map(|x| b::slot_component_enchantments_items::Ty {
							id: x.id,
							level: x.level,
						})
						.collect(),
				)
			}
			A::StoredEnchantments(e) => {
				if !e.show_in_tooltip {
					hide(&mut hidden, "stored_enchantments");
				}
				B::StoredEnchantments(
					e.enchantments
						.into_iter()
						.map(|x| b::slot_component_stored_enchantments_items::Ty {
							id: x.id,
							level: x.level,
						})
						.collect(),
				)
			}
			A::CanPlaceOn(p) => {
				if !p.show_tooltip {
					hide(&mut hidden, "can_place_on");
				}
				B::CanPlaceOn(predicates_up(p.predicates)?)
			}
			A::CanBreak(p) => {
				if !p.show_tooltip {
					hide(&mut hidden, "can_break");
				}
				B::CanBreak(predicates_up(p.predicates)?)
			}
			A::AttributeModifiers(m) => {
				if !m.show_tooltip {
					hide(&mut hidden, "attribute_modifiers");
				}
				B::AttributeModifiers(recode(
					&m.attributes,
					a::slot_component_attribute_modifiers_attributes::serialize,
					b::slot_component_attribute_modifiers::deserialize,
				)?)
			}
			A::DyedColor(c) => {
				if !c.show_tooltip {
					hide(&mut hidden, "dyed_color");
				}
				B::DyedColor(c.color)
			}
			A::Trim(t) => {
				if !t.show_in_tooltip {
					hide(&mut hidden, "trim");
				}
				B::Trim(b::slot_component_trim::Ty {
					material: material_up(t.material),
					pattern: pattern_up(t.pattern),
				})
			}
			A::JukeboxPlayable(j) => {
				if !j.show_in_tooltip {
					hide(&mut hidden, "jukebox_playable");
				}
				let song: b::slot_component_jukebox_playable::Ty = recode(
					&j.song,
					a::slot_component_jukebox_playable_song::serialize,
					b::slot_component_jukebox_playable::deserialize,
				)?;
				B::JukeboxPlayable(match song {
					b::slot_component_jukebox_playable::Ty::True(
						b::slot_component_jukebox_playable_true::Ty::Inline(mut s),
					) => {
						s.sound_event = sound_up(s.sound_event)?;
						s.description = text_up(s.description);
						b::slot_component_jukebox_playable::Ty::True(
							b::slot_component_jukebox_playable_true::Ty::Inline(s),
						)
					}
					other => other,
				})
			}
			A::Instrument(i) => B::Instrument(b::slot_component_instrument::Ty::True(match i {
				a::slot_component_instrument::Ty::Id(id) => {
					b::slot_component_instrument_true::Ty::Id(id)
				}
				a::slot_component_instrument::Ty::Inline(mut d) => {
					d.sound_event = sound_up(d.sound_event)?;
					d.description = text_up(d.description);
					b::slot_component_instrument_true::Ty::Inline(d)
				}
			})),
			A::Consumable(c)
				if sword
					&& matches!(
						c.animation,
						a::slot_component_consumable_animation::Ty::Block
					) =>
			{
				blocks_attacks()
			}
			A::Consumable(c) => {
				let mut c: b::slot_component_consumable::Ty = recode(
					&c,
					a::slot_component_consumable::serialize,
					b::slot_component_consumable::deserialize,
				)?;
				c.sound = sound_up(c.sound)?;
				c.effects = effects(c.effects, sound_up)?;
				B::Consumable(c)
			}
			A::DeathProtection(e) => B::DeathProtection(effects(e, sound_up)?),
			A::Tool(t) => {
				let creative = !(sword || matches!(item, "trident" | "mace" | "debug_stick"));
				B::Tool(b::slot_component_tool::Ty {
					rules: t
						.rules
						.into_iter()
						.map(|r| {
							Ok(b::slot_component_tool_rules_items::Ty {
								blocks: remap::block_set(OLD, NEW, r.blocks)?,
								speed: r.speed,
								correct_drop_for_blocks: r.correct_drop_for_blocks,
							})
						})
						.collect::<Result<_>>()?,
					default_mining_speed: t.default_mining_speed,
					damage_per_block: t.damage_per_block,
					can_destroy_blocks_in_creative: creative,
				})
			}
			A::Equippable(e) => B::Equippable(b::slot_component_equippable::Ty {
				slot: recode(
					&e.slot,
					a::slot_component_equippable_slot::serialize,
					b::slot_component_equippable_slot::deserialize,
				)?,
				sound: sound_up(e.sound)?,
				model: e.model,
				camera_overlay: e.camera_overlay,
				allowed_entities: e
					.allowed_entities
					.map(|s| remap::entity_set(OLD, NEW, s))
					.transpose()?,
				dispensable: e.dispensable,
				swappable: e.swappable,
				damageable: e.damageable,
				equip_on_interact: false,
			}),
			A::Repairable(s) => B::Repairable(remap::item_set(OLD, NEW, s)?),
			A::PotDecorations(v) => B::PotDecorations(remap::items(OLD, NEW, v)?),
			A::UseRemainder(s) => B::UseRemainder(slot_up(s)?),
			A::ChargedProjectiles(v) => B::ChargedProjectiles(slots_up(v)?),
			A::BundleContents(v) => B::BundleContents(slots_up(v)?),
			A::Container(v) => B::Container(slots_up(v)?),
			A::BucketEntityData(t) => B::BucketEntityData(bucket_up(t, &mut out)),
			A::WrittenBookContent(w) => {
				let mut w: b::slot_component_written_book_content::Ty = recode(
					&w,
					a::slot_component_written_book_content::serialize,
					b::slot_component_written_book_content::deserialize,
				)?;
				for page in &mut w.pages {
					page.content = text_up(page.content.clone());
					page.filtered_content = page.filtered_content.take().map(text_up);
				}
				B::WrittenBookContent(w)
			}
			other => match retag_up(&other)? {
				Some(c) => c,
				None => continue,
			},
		};
		out.push(converted);
	}
	if hide_tooltip || !hidden.is_empty() {
		out.push(b::slot_component::Ty::TooltipDisplay(
			b::slot_component_tooltip_display::Ty {
				hide_tooltip: hide_tooltip,
				hidden_components: hidden,
			},
		));
	}
	let mut removes = Vec::new();
	for r in &d.remove_components {
		removes.extend(type_up(r)?);
	}
	Ok(b::slot::Ty::Default(
		count,
		b::slot_default::Ty {
			item_id,
			added_component_count: out.len() as i32,
			removed_component_count: removes.len() as i32,
			components: out,
			remove_components: removes,
		},
	))
}

pub fn slots_down(v: Vec<b::slot::Ty>) -> Result<Vec<a::slot::Ty>> {
	v.into_iter().map(slot_down).collect()
}

pub fn slot_down(s: b::slot::Ty) -> Result<a::slot::Ty> {
	let (count, d) = match s {
		b::slot::Ty::V0 => return Ok(a::slot::Ty::V0),
		b::slot::Ty::Default(count, d) => (count, d),
	};
	let item_id = remap::item(NEW, OLD, d.item_id)?;
	let display = d.components.iter().find_map(|c| match c {
		b::slot_component::Ty::TooltipDisplay(t) => Some(t.clone()),
		_ => None,
	});
	let shown = |name: &str| {
		display.as_ref().is_none_or(|t| {
			components::id(NEW, name).is_none_or(|id| !t.hidden_components.contains(&id))
		})
	};
	let mut out = Vec::with_capacity(d.components.len());
	let mut bucket: Option<(i32, i32, i32)> = None;
	for c in d.components {
		use a::slot_component::Ty as A;
		use b::slot_component::Ty as B;
		let converted = match c {
			B::TooltipDisplay(t) => {
				if t.hide_tooltip {
					out.push(A::HideTooltip);
				}
				let all = HIDE_ADDITIONAL
					.iter()
					.filter(|n| components::id(OLD, n).is_some())
					.all(|n| {
						components::id(NEW, n).is_some_and(|id| t.hidden_components.contains(&id))
					});
				if all {
					out.push(A::HideAdditionalTooltip);
				}
				continue;
			}
			B::Unbreakable => A::Unbreakable(shown("unbreakable")),
			B::CustomName(t) => A::CustomName(text::down(t)),
			B::ItemName(t) => A::ItemName(text::down(t)),
			B::Lore(lines) => A::Lore(lines.into_iter().map(|l| Some(text::down(l))).collect()),
			B::WrittenBookContent(w) => {
				let mut w: a::slot_component_written_book_content::Ty = recode(
					&w,
					b::slot_component_written_book_content::serialize,
					a::slot_component_written_book_content::deserialize,
				)?;
				for page in &mut w.pages {
					page.content = text::down(page.content.clone());
					page.filtered_content = page.filtered_content.take().map(text::down);
				}
				A::WrittenBookContent(w)
			}
			B::Enchantments(e) => A::Enchantments(a::slot_component_enchantments::Ty {
				enchantments: e
					.into_iter()
					.map(|x| a::slot_component_enchantments_enchantments_items::Ty {
						id: x.id,
						level: x.level,
					})
					.collect(),
				show_tooltip: shown("enchantments"),
			}),
			B::StoredEnchantments(e) => {
				A::StoredEnchantments(a::slot_component_stored_enchantments::Ty {
					enchantments: e
						.into_iter()
						.map(
							|x| a::slot_component_stored_enchantments_enchantments_items::Ty {
								id: x.id,
								level: x.level,
							},
						)
						.collect(),
					show_in_tooltip: shown("stored_enchantments"),
				})
			}
			B::CanPlaceOn(p) => A::CanPlaceOn(predicates_down(p, shown("can_place_on"))),
			B::CanBreak(p) => {
				let p = predicates_down(p, shown("can_break"));
				A::CanBreak(a::slot_component_can_break::Ty {
					predicates: p.predicates,
					show_tooltip: p.show_tooltip,
				})
			}
			B::AttributeModifiers(m) => {
				let attributes = m
					.into_iter()
					.filter_map(|x| {
						recode(
							&x,
							b::slot_component_attribute_modifiers_items::serialize,
							a::slot_component_attribute_modifiers_attributes_items::deserialize,
						)
						.ok()
					})
					.collect();
				A::AttributeModifiers(a::slot_component_attribute_modifiers::Ty {
					attributes,
					show_tooltip: shown("attribute_modifiers"),
				})
			}
			B::DyedColor(color) => A::DyedColor(a::slot_component_dyed_color::Ty {
				color,
				show_tooltip: shown("dyed_color"),
			}),
			B::Trim(t) => A::Trim(a::slot_component_trim::Ty {
				material: material_down(t.material),
				pattern: pattern_down(t.pattern),
				show_in_tooltip: shown("trim"),
			}),
			B::JukeboxPlayable(j) => {
				let j = match j {
					b::slot_component_jukebox_playable::Ty::True(
						b::slot_component_jukebox_playable_true::Ty::Inline(mut s),
					) => {
						s.sound_event = sound_down(s.sound_event)?;
						s.description = text::down(s.description);
						b::slot_component_jukebox_playable::Ty::True(
							b::slot_component_jukebox_playable_true::Ty::Inline(s),
						)
					}
					other => other,
				};
				A::JukeboxPlayable(a::slot_component_jukebox_playable::Ty {
					song: recode(
						&j,
						b::slot_component_jukebox_playable::serialize,
						a::slot_component_jukebox_playable_song::deserialize,
					)?,
					show_in_tooltip: shown("jukebox_playable"),
				})
			}
			B::Instrument(b::slot_component_instrument::Ty::True(i)) => A::Instrument(match i {
				b::slot_component_instrument_true::Ty::Id(id) => {
					a::slot_component_instrument::Ty::Id(id)
				}
				b::slot_component_instrument_true::Ty::Inline(mut d) => {
					d.sound_event = sound_down(d.sound_event)?;
					d.description = text::down(d.description);
					a::slot_component_instrument::Ty::Inline(d)
				}
			}),
			B::Instrument(_) => continue,
			B::Consumable(mut c) => {
				c.sound = sound_down(c.sound)?;
				c.effects = effects(c.effects, sound_down)?;
				match recode(
					&c,
					b::slot_component_consumable::serialize,
					a::slot_component_consumable::deserialize,
				) {
					Ok(c) => A::Consumable(c),
					Err(_) => continue,
				}
			}
			B::DeathProtection(e) => A::DeathProtection(effects(e, sound_down)?),
			B::Tool(t) => A::Tool(a::slot_component_tool::Ty {
				rules: t
					.rules
					.into_iter()
					.map(|r| {
						Ok(a::slot_component_tool_rules_items::Ty {
							blocks: remap::block_set(NEW, OLD, r.blocks)?,
							speed: r.speed,
							correct_drop_for_blocks: r.correct_drop_for_blocks,
						})
					})
					.collect::<Result<_>>()?,
				default_mining_speed: t.default_mining_speed,
				damage_per_block: t.damage_per_block,
			}),
			B::Equippable(e) => A::Equippable(a::slot_component_equippable::Ty {
				slot: match recode(
					&e.slot,
					b::slot_component_equippable_slot::serialize,
					a::slot_component_equippable_slot::deserialize,
				) {
					Ok(s) => s,
					Err(_) => continue,
				},
				sound: sound_down(e.sound)?,
				model: e.model,
				camera_overlay: e.camera_overlay,
				allowed_entities: e
					.allowed_entities
					.map(|s| remap::entity_set(NEW, OLD, s))
					.transpose()?,
				dispensable: e.dispensable,
				swappable: e.swappable,
				damageable: e.damageable,
			}),
			B::Repairable(s) => A::Repairable(remap::item_set(NEW, OLD, s)?),
			B::PotDecorations(v) => A::PotDecorations(remap::items(NEW, OLD, v)?),
			B::UseRemainder(s) => A::UseRemainder(slot_down(s)?),
			B::ChargedProjectiles(v) => A::ChargedProjectiles(slots_down(v)?),
			B::BundleContents(v) => A::BundleContents(slots_down(v)?),
			B::Container(v) => A::Container(slots_down(v)?),
			B::TropicalFishPattern(p) => {
				let entry = bucket.get_or_insert((0, 0, 0));
				entry.0 = p;
				continue;
			}
			B::TropicalFishBaseColor(c) => {
				bucket.get_or_insert((0, 0, 0)).1 = c;
				continue;
			}
			B::TropicalFishPatternColor(c) => {
				bucket.get_or_insert((0, 0, 0)).2 = c;
				continue;
			}
			other => match retag_down(&other)? {
				Some(c) => c,
				None => continue,
			},
		};
		out.push(converted);
	}
	if let Some((pattern, base, color)) = bucket {
		let packed = (color & 0xff) << 24 | (base & 0xff) << 16 | (pattern & 0xffff);
		let at = out
			.iter()
			.position(|c| matches!(c, a::slot_component::Ty::BucketEntityData(_)));
		let mut tag = match at {
			Some(at) => match out.remove(at) {
				a::slot_component::Ty::BucketEntityData(t) => t,
				_ => unreachable!("matched above"),
			},
			None => Tag::new(Payload::Compound(vec![])),
		};
		tag.value.put("BucketVariantTag", Payload::Int(packed));
		out.push(a::slot_component::Ty::BucketEntityData(tag));
	}
	let mut removes = Vec::new();
	for r in &d.remove_components {
		removes.extend(type_down(r)?);
	}
	Ok(a::slot::Ty::Default(
		count,
		a::slot_default::Ty {
			item_id,
			added_component_count: out.len() as i32,
			removed_component_count: removes.len() as i32,
			components: out,
			remove_components: removes,
		},
	))
}

pub fn components_up(v: Vec<a::slot_component::Ty>) -> Result<Vec<b::slot_component::Ty>> {
	match slot_up(a::slot::Ty::Default(
		1,
		a::slot_default::Ty {
			item_id: 0,
			added_component_count: v.len() as i32,
			removed_component_count: 0,
			components: v,
			remove_components: Vec::new(),
		},
	))? {
		b::slot::Ty::Default(_, d) => Ok(d.components),
		b::slot::Ty::V0 => Err(Error::Dropped),
	}
}

pub fn components_down(v: Vec<b::slot_component::Ty>) -> Result<Vec<a::slot_component::Ty>> {
	match slot_down(b::slot::Ty::Default(
		1,
		b::slot_default::Ty {
			item_id: 0,
			added_component_count: v.len() as i32,
			removed_component_count: 0,
			components: v,
			remove_components: Vec::new(),
		},
	))? {
		a::slot::Ty::Default(_, d) => Ok(d.components),
		a::slot::Ty::V0 => Err(Error::Dropped),
	}
}
