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

fn sound_up(v: b::ItemSoundHolder::Ty) -> Result<b::ItemSoundHolder::Ty> {
	remap::sound(OLD, NEW, v)
}

fn sound_down(v: b::ItemSoundHolder::Ty) -> Result<b::ItemSoundHolder::Ty> {
	remap::sound(NEW, OLD, v)
}

fn effects(
	v: Vec<b::ItemConsumeEffect::Ty>,
	sound: impl Fn(b::ItemSoundHolder::Ty) -> Result<b::ItemSoundHolder::Ty>,
) -> Result<Vec<b::ItemConsumeEffect::Ty>> {
	v.into_iter()
		.map(|e| match e {
			b::ItemConsumeEffect::Ty::play_sound(s) => {
				Ok(b::ItemConsumeEffect::Ty::play_sound(sound(s)?))
			}
			e => Ok(e),
		})
		.collect()
}

fn text_up(t: nbt::Tag) -> nbt::Tag {
	text::up(t)
}

fn predicates_up(v: Vec<a::ItemBlockPredicate::Ty>) -> Result<Vec<b::ItemBlockPredicate::Ty>> {
	v.into_iter()
		.map(|p| {
			Ok(b::ItemBlockPredicate::Ty {
				blockSet: p.blockSet.map(|s| match s {
					a::ItemBlockPredicate__blockSet__items::Ty::Named(n) => {
						b::ItemBlockPredicate__blockSet__items::Ty::Named(n)
					}
					a::ItemBlockPredicate__blockSet__items::Ty::Ids(ids) => {
						b::ItemBlockPredicate__blockSet__items::Ty::Ids(
							ids.into_iter()
								.filter_map(|i| super::util::remap("blocks", OLD, NEW, i))
								.collect(),
						)
					}
				}),
				properties: p.properties,
				nbt: p.nbt,
				components: b::DataComponentMatchers::Ty {
					exactMatchers: Vec::new(),
					partialMatchers: Vec::new(),
				},
			})
		})
		.collect()
}

fn predicates_down(
	v: Vec<b::ItemBlockPredicate::Ty>,
	show: bool,
) -> a::SlotComponent__can_place_on::Ty {
	a::SlotComponent__can_place_on::Ty {
		predicates: v
			.into_iter()
			.map(|p| a::ItemBlockPredicate::Ty {
				blockSet: p.blockSet.map(|s| match s {
					b::ItemBlockPredicate__blockSet__items::Ty::Named(n) => {
						a::ItemBlockPredicate__blockSet__items::Ty::Named(n)
					}
					b::ItemBlockPredicate__blockSet__items::Ty::Ids(ids) => {
						a::ItemBlockPredicate__blockSet__items::Ty::Ids(
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
		showTooltip: show,
	}
}

fn material_up(m: a::SlotComponent__trim__material::Ty) -> b::SlotComponent__trim__material::Ty {
	match m {
		a::SlotComponent__trim__material::Ty::Id(i) => b::SlotComponent__trim__material::Ty::Id(i),
		a::SlotComponent__trim__material::Ty::Inline(m) => {
			b::SlotComponent__trim__material::Ty::Inline(b::ArmorTrimMaterial::Ty {
				assetBase: m.assetName,
				overrideArmorAssets: m
					.overrideArmorAssets
					.into_iter()
					.map(|o| b::ArmorTrimMaterial__overrideArmorAssets__items::Ty {
						key: o.key,
						value: o.value,
					})
					.collect(),
				description: text_up(m.description),
			})
		}
	}
}

fn pattern_up(p: a::SlotComponent__trim__pattern::Ty) -> b::SlotComponent__trim__pattern::Ty {
	match p {
		a::SlotComponent__trim__pattern::Ty::Id(i) => b::SlotComponent__trim__pattern::Ty::Id(i),
		a::SlotComponent__trim__pattern::Ty::Inline(p) => {
			b::SlotComponent__trim__pattern::Ty::Inline(b::ArmorTrimPattern::Ty {
				assetId: p.assetId,
				description: text_up(p.description),
				decal: p.decal,
			})
		}
	}
}

fn material_down(m: b::SlotComponent__trim__material::Ty) -> a::SlotComponent__trim__material::Ty {
	match m {
		b::SlotComponent__trim__material::Ty::Id(i) => a::SlotComponent__trim__material::Ty::Id(i),
		b::SlotComponent__trim__material::Ty::Inline(m) => {
			a::SlotComponent__trim__material::Ty::Inline(a::ArmorTrimMaterial::Ty {
				assetName: m.assetBase,
				ingredientId: 0,
				overrideArmorAssets: m
					.overrideArmorAssets
					.into_iter()
					.map(|o| a::ArmorTrimMaterial__overrideArmorAssets__items::Ty {
						key: o.key,
						value: o.value,
					})
					.collect(),
				description: m.description,
			})
		}
	}
}

fn pattern_down(p: b::SlotComponent__trim__pattern::Ty) -> a::SlotComponent__trim__pattern::Ty {
	match p {
		b::SlotComponent__trim__pattern::Ty::Id(i) => a::SlotComponent__trim__pattern::Ty::Id(i),
		b::SlotComponent__trim__pattern::Ty::Inline(p) => {
			a::SlotComponent__trim__pattern::Ty::Inline(a::ArmorTrimPattern::Ty {
				assetId: p.assetId,
				templateItemId: 0,
				description: p.description,
				decal: p.decal,
			})
		}
	}
}

fn bucket_up(mut tag: nbt::Tag, out: &mut Vec<b::SlotComponent::Ty>) -> nbt::Tag {
	if let Some(v) = tag
		.value
		.field("BucketVariantTag")
		.and_then(Payload::as_i64)
	{
		tag.value.take("BucketVariantTag");
		let v = v as i32;
		out.push(b::SlotComponent::Ty::tropical_fish_pattern(v & 0xffff));
		out.push(b::SlotComponent::Ty::tropical_fish_base_color(
			(v >> 16) & 0xff,
		));
		out.push(b::SlotComponent::Ty::tropical_fish_pattern_color(
			(v >> 24) & 0xff,
		));
	}
	tag
}

fn blocks_attacks() -> b::SlotComponent::Ty {
	b::SlotComponent::Ty::blocks_attacks(b::SlotComponent__blocks_attacks::Ty {
		blockDelaySeconds: 0.0,
		disableCooldownScale: 0.0,
		damageReductions: vec![
			b::SlotComponent__blocks_attacks__damageReductions__items::Ty {
				horizontalBlockingAngle: 90.0,
				r#type: None,
				base: -0.5,
				factor: 0.5,
			},
		],
		itemDamage: b::SlotComponent__blocks_attacks__itemDamage::Ty {
			threshold: 0.0,
			base: 0.0,
			factor: 0.0,
		},
		bypassedBy: None,
		blockSound: None,
		disableSound: None,
	})
}

fn retag_up(c: &a::SlotComponent::Ty) -> Result<Option<b::SlotComponent::Ty>> {
	components::retag(
		|t| components::remap(OLD, NEW, t),
		c,
		a::SlotComponent::serialize,
		b::SlotComponent::deserialize,
	)
}

fn retag_down(c: &b::SlotComponent::Ty) -> Result<Option<a::SlotComponent::Ty>> {
	components::retag(
		|t| components::remap(NEW, OLD, t),
		c,
		b::SlotComponent::serialize,
		a::SlotComponent::deserialize,
	)
}

fn type_up(t: &a::SlotComponentType::Ty) -> Result<Option<b::SlotComponentType::Ty>> {
	components::retag(
		|t| components::remap(OLD, NEW, t),
		t,
		a::SlotComponentType::serialize,
		b::SlotComponentType::deserialize,
	)
}

fn type_down(t: &b::SlotComponentType::Ty) -> Result<Option<a::SlotComponentType::Ty>> {
	components::retag(
		|t| components::remap(NEW, OLD, t),
		t,
		b::SlotComponentType::serialize,
		a::SlotComponentType::deserialize,
	)
}

pub fn slots_up(v: Vec<a::Slot::Ty>) -> Result<Vec<b::Slot::Ty>> {
	v.into_iter().map(slot_up).collect()
}

pub fn slot_up(s: a::Slot::Ty) -> Result<b::Slot::Ty> {
	let (count, d) = match s {
		a::Slot::Ty::v0 => return Ok(b::Slot::Ty::v0),
		a::Slot::Ty::default(count, d) => (count, d),
	};
	let itemId = remap::item(OLD, NEW, d.itemId)?;
	let item = item_name(NEW, itemId);
	let sword = item.ends_with("_sword");
	let mut out = Vec::with_capacity(d.components.len());
	let mut hidden = Vec::new();
	let mut hide_tooltip = false;
	for c in d.components {
		use a::SlotComponent::Ty as A;
		use b::SlotComponent::Ty as B;
		let converted = match c {
			A::hide_tooltip => {
				hide_tooltip = true;
				continue;
			}
			A::hide_additional_tooltip => {
				for name in HIDE_ADDITIONAL {
					hide(&mut hidden, name);
				}
				continue;
			}
			A::custom_name(t) => B::custom_name(text_up(t)),
			A::item_name(t) => B::item_name(text_up(t)),
			A::lore(lines) => B::lore(
				lines
					.into_iter()
					.map(|l| text_up(l.unwrap_or_else(|| Tag::new(Payload::from("")))))
					.collect(),
			),
			A::unbreakable(show) => {
				if !show {
					hide(&mut hidden, "unbreakable");
				}
				B::unbreakable
			}
			A::enchantments(e) => {
				if !e.showTooltip {
					hide(&mut hidden, "enchantments");
				}
				B::enchantments(
					e.enchantments
						.into_iter()
						.map(|x| b::SlotComponent__enchantments__items::Ty {
							id: x.id,
							level: x.level,
						})
						.collect(),
				)
			}
			A::stored_enchantments(e) => {
				if !e.showInTooltip {
					hide(&mut hidden, "stored_enchantments");
				}
				B::stored_enchantments(
					e.enchantments
						.into_iter()
						.map(|x| b::SlotComponent__stored_enchantments__items::Ty {
							id: x.id,
							level: x.level,
						})
						.collect(),
				)
			}
			A::can_place_on(p) => {
				if !p.showTooltip {
					hide(&mut hidden, "can_place_on");
				}
				B::can_place_on(predicates_up(p.predicates)?)
			}
			A::can_break(p) => {
				if !p.showTooltip {
					hide(&mut hidden, "can_break");
				}
				B::can_break(predicates_up(p.predicates)?)
			}
			A::attribute_modifiers(m) => {
				if !m.showTooltip {
					hide(&mut hidden, "attribute_modifiers");
				}
				B::attribute_modifiers(recode(
					&m.attributes,
					a::SlotComponent__attribute_modifiers__attributes::serialize,
					b::SlotComponent__attribute_modifiers::deserialize,
				)?)
			}
			A::dyed_color(c) => {
				if !c.showTooltip {
					hide(&mut hidden, "dyed_color");
				}
				B::dyed_color(c.color)
			}
			A::trim(t) => {
				if !t.showInTooltip {
					hide(&mut hidden, "trim");
				}
				B::trim(b::SlotComponent__trim::Ty {
					material: material_up(t.material),
					pattern: pattern_up(t.pattern),
				})
			}
			A::jukebox_playable(j) => {
				if !j.showInTooltip {
					hide(&mut hidden, "jukebox_playable");
				}
				let song: b::SlotComponent__jukebox_playable::Ty = recode(
					&j.song,
					a::SlotComponent__jukebox_playable__song::serialize,
					b::SlotComponent__jukebox_playable::deserialize,
				)?;
				B::jukebox_playable(match song {
					b::SlotComponent__jukebox_playable::Ty::r#true(
						b::SlotComponent__jukebox_playable__true::Ty::Inline(mut s),
					) => {
						s.soundEvent = sound_up(s.soundEvent)?;
						s.description = text_up(s.description);
						b::SlotComponent__jukebox_playable::Ty::r#true(
							b::SlotComponent__jukebox_playable__true::Ty::Inline(s),
						)
					}
					other => other,
				})
			}
			A::instrument(i) => B::instrument(b::SlotComponent__instrument::Ty::r#true(match i {
				a::SlotComponent__instrument::Ty::Id(id) => {
					b::SlotComponent__instrument__true::Ty::Id(id)
				}
				a::SlotComponent__instrument::Ty::Inline(mut d) => {
					d.soundEvent = sound_up(d.soundEvent)?;
					d.description = text_up(d.description);
					b::SlotComponent__instrument__true::Ty::Inline(d)
				}
			})),
			A::consumable(c)
				if sword
					&& matches!(
						c.animation,
						a::SlotComponent__consumable__animation::Ty::block
					) =>
			{
				blocks_attacks()
			}
			A::consumable(c) => {
				let mut c: b::SlotComponent__consumable::Ty = recode(
					&c,
					a::SlotComponent__consumable::serialize,
					b::SlotComponent__consumable::deserialize,
				)?;
				c.sound = sound_up(c.sound)?;
				c.effects = effects(c.effects, sound_up)?;
				B::consumable(c)
			}
			A::death_protection(e) => B::death_protection(effects(e, sound_up)?),
			A::tool(t) => {
				let creative = !(sword || matches!(item, "trident" | "mace" | "debug_stick"));
				B::tool(b::SlotComponent__tool::Ty {
					rules: t
						.rules
						.into_iter()
						.map(|r| {
							Ok(b::SlotComponent__tool__rules__items::Ty {
								blocks: remap::block_set(OLD, NEW, r.blocks)?,
								speed: r.speed,
								correctDropForBlocks: r.correctDropForBlocks,
							})
						})
						.collect::<Result<_>>()?,
					defaultMiningSpeed: t.defaultMiningSpeed,
					damagePerBlock: t.damagePerBlock,
					canDestroyBlocksInCreative: creative,
				})
			}
			A::equippable(e) => B::equippable(b::SlotComponent__equippable::Ty {
				slot: recode(
					&e.slot,
					a::SlotComponent__equippable__slot::serialize,
					b::SlotComponent__equippable__slot::deserialize,
				)?,
				sound: sound_up(e.sound)?,
				model: e.model,
				cameraOverlay: e.cameraOverlay,
				allowedEntities: e
					.allowedEntities
					.map(|s| remap::entity_set(OLD, NEW, s))
					.transpose()?,
				dispensable: e.dispensable,
				swappable: e.swappable,
				damageable: e.damageable,
				equipOnInteract: false,
			}),
			A::repairable(s) => B::repairable(remap::item_set(OLD, NEW, s)?),
			A::pot_decorations(v) => B::pot_decorations(remap::items(OLD, NEW, v)?),
			A::use_remainder(s) => B::use_remainder(slot_up(s)?),
			A::charged_projectiles(v) => B::charged_projectiles(slots_up(v)?),
			A::bundle_contents(v) => B::bundle_contents(slots_up(v)?),
			A::container(v) => B::container(slots_up(v)?),
			A::bucket_entity_data(t) => B::bucket_entity_data(bucket_up(t, &mut out)),
			A::written_book_content(w) => {
				let mut w: b::SlotComponent__written_book_content::Ty = recode(
					&w,
					a::SlotComponent__written_book_content::serialize,
					b::SlotComponent__written_book_content::deserialize,
				)?;
				for page in &mut w.pages {
					page.content = text_up(page.content.clone());
					page.filteredContent = page.filteredContent.take().map(text_up);
				}
				B::written_book_content(w)
			}
			other => match retag_up(&other)? {
				Some(c) => c,
				None => continue,
			},
		};
		out.push(converted);
	}
	if hide_tooltip || !hidden.is_empty() {
		out.push(b::SlotComponent::Ty::tooltip_display(
			b::SlotComponent__tooltip_display::Ty {
				hideTooltip: hide_tooltip,
				hiddenComponents: hidden,
			},
		));
	}
	let mut removes = Vec::new();
	for r in &d.removeComponents {
		removes.extend(type_up(r)?);
	}
	Ok(b::Slot::Ty::default(
		count,
		b::Slot__default::Ty {
			itemId,
			addedComponentCount: out.len() as i32,
			removedComponentCount: removes.len() as i32,
			components: out,
			removeComponents: removes,
		},
	))
}

pub fn slots_down(v: Vec<b::Slot::Ty>) -> Result<Vec<a::Slot::Ty>> {
	v.into_iter().map(slot_down).collect()
}

pub fn slot_down(s: b::Slot::Ty) -> Result<a::Slot::Ty> {
	let (count, d) = match s {
		b::Slot::Ty::v0 => return Ok(a::Slot::Ty::v0),
		b::Slot::Ty::default(count, d) => (count, d),
	};
	let itemId = remap::item(NEW, OLD, d.itemId)?;
	let display = d.components.iter().find_map(|c| match c {
		b::SlotComponent::Ty::tooltip_display(t) => Some(t.clone()),
		_ => None,
	});
	let shown = |name: &str| {
		display.as_ref().is_none_or(|t| {
			components::id(NEW, name).is_none_or(|id| !t.hiddenComponents.contains(&id))
		})
	};
	let mut out = Vec::with_capacity(d.components.len());
	let mut bucket: Option<(i32, i32, i32)> = None;
	for c in d.components {
		use a::SlotComponent::Ty as A;
		use b::SlotComponent::Ty as B;
		let converted = match c {
			B::tooltip_display(t) => {
				if t.hideTooltip {
					out.push(A::hide_tooltip);
				}
				let all = HIDE_ADDITIONAL
					.iter()
					.filter(|n| components::id(OLD, n).is_some())
					.all(|n| {
						components::id(NEW, n).is_some_and(|id| t.hiddenComponents.contains(&id))
					});
				if all {
					out.push(A::hide_additional_tooltip);
				}
				continue;
			}
			B::unbreakable => A::unbreakable(shown("unbreakable")),
			B::custom_name(t) => A::custom_name(text::down(t)),
			B::item_name(t) => A::item_name(text::down(t)),
			B::lore(lines) => A::lore(lines.into_iter().map(|l| Some(text::down(l))).collect()),
			B::written_book_content(w) => {
				let mut w: a::SlotComponent__written_book_content::Ty = recode(
					&w,
					b::SlotComponent__written_book_content::serialize,
					a::SlotComponent__written_book_content::deserialize,
				)?;
				for page in &mut w.pages {
					page.content = text::down(page.content.clone());
					page.filteredContent = page.filteredContent.take().map(text::down);
				}
				A::written_book_content(w)
			}
			B::enchantments(e) => A::enchantments(a::SlotComponent__enchantments::Ty {
				enchantments: e
					.into_iter()
					.map(
						|x| a::SlotComponent__enchantments__enchantments__items::Ty {
							id: x.id,
							level: x.level,
						},
					)
					.collect(),
				showTooltip: shown("enchantments"),
			}),
			B::stored_enchantments(e) => {
				A::stored_enchantments(a::SlotComponent__stored_enchantments::Ty {
					enchantments: e
						.into_iter()
						.map(
							|x| a::SlotComponent__stored_enchantments__enchantments__items::Ty {
								id: x.id,
								level: x.level,
							},
						)
						.collect(),
					showInTooltip: shown("stored_enchantments"),
				})
			}
			B::can_place_on(p) => A::can_place_on(predicates_down(p, shown("can_place_on"))),
			B::can_break(p) => {
				let p = predicates_down(p, shown("can_break"));
				A::can_break(a::SlotComponent__can_break::Ty {
					predicates: p.predicates,
					showTooltip: p.showTooltip,
				})
			}
			B::attribute_modifiers(m) => {
				let attributes = m
					.into_iter()
					.filter_map(|x| {
						recode(
							&x,
							b::SlotComponent__attribute_modifiers__items::serialize,
							a::SlotComponent__attribute_modifiers__attributes__items::deserialize,
						)
						.ok()
					})
					.collect();
				A::attribute_modifiers(a::SlotComponent__attribute_modifiers::Ty {
					attributes,
					showTooltip: shown("attribute_modifiers"),
				})
			}
			B::dyed_color(color) => A::dyed_color(a::SlotComponent__dyed_color::Ty {
				color,
				showTooltip: shown("dyed_color"),
			}),
			B::trim(t) => A::trim(a::SlotComponent__trim::Ty {
				material: material_down(t.material),
				pattern: pattern_down(t.pattern),
				showInTooltip: shown("trim"),
			}),
			B::jukebox_playable(j) => {
				let j = match j {
					b::SlotComponent__jukebox_playable::Ty::r#true(
						b::SlotComponent__jukebox_playable__true::Ty::Inline(mut s),
					) => {
						s.soundEvent = sound_down(s.soundEvent)?;
						s.description = text::down(s.description);
						b::SlotComponent__jukebox_playable::Ty::r#true(
							b::SlotComponent__jukebox_playable__true::Ty::Inline(s),
						)
					}
					other => other,
				};
				A::jukebox_playable(a::SlotComponent__jukebox_playable::Ty {
					song: recode(
						&j,
						b::SlotComponent__jukebox_playable::serialize,
						a::SlotComponent__jukebox_playable__song::deserialize,
					)?,
					showInTooltip: shown("jukebox_playable"),
				})
			}
			B::instrument(b::SlotComponent__instrument::Ty::r#true(i)) => A::instrument(match i {
				b::SlotComponent__instrument__true::Ty::Id(id) => {
					a::SlotComponent__instrument::Ty::Id(id)
				}
				b::SlotComponent__instrument__true::Ty::Inline(mut d) => {
					d.soundEvent = sound_down(d.soundEvent)?;
					d.description = text::down(d.description);
					a::SlotComponent__instrument::Ty::Inline(d)
				}
			}),
			B::instrument(_) => continue,
			B::consumable(mut c) => {
				c.sound = sound_down(c.sound)?;
				c.effects = effects(c.effects, sound_down)?;
				match recode(
					&c,
					b::SlotComponent__consumable::serialize,
					a::SlotComponent__consumable::deserialize,
				) {
					Ok(c) => A::consumable(c),
					Err(_) => continue,
				}
			}
			B::death_protection(e) => A::death_protection(effects(e, sound_down)?),
			B::tool(t) => A::tool(a::SlotComponent__tool::Ty {
				rules: t
					.rules
					.into_iter()
					.map(|r| {
						Ok(a::SlotComponent__tool__rules__items::Ty {
							blocks: remap::block_set(NEW, OLD, r.blocks)?,
							speed: r.speed,
							correctDropForBlocks: r.correctDropForBlocks,
						})
					})
					.collect::<Result<_>>()?,
				defaultMiningSpeed: t.defaultMiningSpeed,
				damagePerBlock: t.damagePerBlock,
			}),
			B::equippable(e) => A::equippable(a::SlotComponent__equippable::Ty {
				slot: match recode(
					&e.slot,
					b::SlotComponent__equippable__slot::serialize,
					a::SlotComponent__equippable__slot::deserialize,
				) {
					Ok(s) => s,
					Err(_) => continue,
				},
				sound: sound_down(e.sound)?,
				model: e.model,
				cameraOverlay: e.cameraOverlay,
				allowedEntities: e
					.allowedEntities
					.map(|s| remap::entity_set(NEW, OLD, s))
					.transpose()?,
				dispensable: e.dispensable,
				swappable: e.swappable,
				damageable: e.damageable,
			}),
			B::repairable(s) => A::repairable(remap::item_set(NEW, OLD, s)?),
			B::pot_decorations(v) => A::pot_decorations(remap::items(NEW, OLD, v)?),
			B::use_remainder(s) => A::use_remainder(slot_down(s)?),
			B::charged_projectiles(v) => A::charged_projectiles(slots_down(v)?),
			B::bundle_contents(v) => A::bundle_contents(slots_down(v)?),
			B::container(v) => A::container(slots_down(v)?),
			B::tropical_fish_pattern(p) => {
				let entry = bucket.get_or_insert((0, 0, 0));
				entry.0 = p;
				continue;
			}
			B::tropical_fish_base_color(c) => {
				bucket.get_or_insert((0, 0, 0)).1 = c;
				continue;
			}
			B::tropical_fish_pattern_color(c) => {
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
			.position(|c| matches!(c, a::SlotComponent::Ty::bucket_entity_data(_)));
		let mut tag = match at {
			Some(at) => match out.remove(at) {
				a::SlotComponent::Ty::bucket_entity_data(t) => t,
				_ => unreachable!("matched above"),
			},
			None => Tag::new(Payload::compound(vec![])),
		};
		tag.value.put("BucketVariantTag", Payload::int(packed));
		out.push(a::SlotComponent::Ty::bucket_entity_data(tag));
	}
	let mut removes = Vec::new();
	for r in &d.removeComponents {
		removes.extend(type_down(r)?);
	}
	Ok(a::Slot::Ty::default(
		count,
		a::Slot__default::Ty {
			itemId,
			addedComponentCount: out.len() as i32,
			removedComponentCount: removes.len() as i32,
			components: out,
			removeComponents: removes,
		},
	))
}

pub fn components_up(v: Vec<a::SlotComponent::Ty>) -> Result<Vec<b::SlotComponent::Ty>> {
	match slot_up(a::Slot::Ty::default(
		1,
		a::Slot__default::Ty {
			itemId: 0,
			addedComponentCount: v.len() as i32,
			removedComponentCount: 0,
			components: v,
			removeComponents: Vec::new(),
		},
	))? {
		b::Slot::Ty::default(_, d) => Ok(d.components),
		b::Slot::Ty::v0 => Err(Error::Dropped),
	}
}

pub fn components_down(v: Vec<b::SlotComponent::Ty>) -> Result<Vec<a::SlotComponent::Ty>> {
	match slot_down(b::Slot::Ty::default(
		1,
		b::Slot__default::Ty {
			itemId: 0,
			addedComponentCount: v.len() as i32,
			removedComponentCount: 0,
			components: v,
			removeComponents: Vec::new(),
		},
	))? {
		a::Slot::Ty::default(_, d) => Ok(d.components),
		a::Slot::Ty::v0 => Err(Error::Dropped),
	}
}
