use super::common::{
	self, block, block_state, decode, encode, item, meta_key, name_id, particle_id, particle_name,
	read_varint, varint, BLOCK_PARTICLES,
};
use super::items::{self, non_empty, to_components, to_legacy};
use super::step_1_20::{optional_state, record, spawn_object_data, tag_entries, world_event_data};
use crate::gen::convert::{v1_20_3 as old, v1_20_5 as new};
use crate::gen::convert::{Root, Version};
use crate::pds::convert::{unconvertible, Error, Result};
use crate::registry;
use crate::Session;
use nbt::{CompoundExt, CompoundVecExt, Entry, List, Payload, Tag};

const OLD: Version = Version::V1_20_3;
const NEW: Version = Version::V1_20_5;

type Client = new::play_toClient_packet::Ty;
type ClientOld = old::play_toClient_packet::Ty;
type Config = new::configuration_toClient_packet::Ty;
type ConfigOld = old::configuration_toClient_packet::Ty;
type Slot = new::Slot::Ty;

pub fn trails_slot_1_20_3_to_1_20_5(v: old::slot::Ty, _cx: &mut Session) -> Result<Slot> {
	Ok(to_components(v))
}

pub fn trails_item_1_20_3_to_1_20_5(v: old::slot::Ty, _cx: &mut Session) -> Result<Slot> {
	Ok(non_empty(v))
}

pub fn trails_slot_1_20_5_to_1_20_3(v: Slot, _cx: &mut Session) -> Result<old::slot::Ty> {
	Ok(to_legacy(v))
}

fn slot_bytes(b: &[u8]) -> Result<Vec<u8>> {
	encode(
		&non_empty(decode(b, |i| old::slot::deserialize(i))?),
		new::Slot::serialize,
	)
}

fn block_payload(from: Version, to: Version, payload: &[u8]) -> Result<Vec<u8>> {
	let (s, rest) = read_varint(payload)?;
	let mut out = Vec::new();
	varint(&mut out, block_state(from, to, s));
	out.extend_from_slice(rest);
	Ok(out)
}

fn up_payload(name: &str, color: i32, payload: &[u8]) -> Result<Vec<u8>> {
	Ok(match name {
		"entity_effect" => (color | (0xff << 24)).to_be_bytes().to_vec(),
		"dust_color_transition" if payload.len() == 28 => {
			[&payload[..12], &payload[16..28], &payload[12..16]].concat()
		}
		n if BLOCK_PARTICLES.contains(&n) => block_payload(OLD, NEW, payload)?,
		"item" => slot_bytes(payload)?,
		_ => payload.to_vec(),
	})
}

fn down_payload(name: &str, payload: &[u8]) -> Result<Vec<u8>> {
	Ok(match name {
		"entity_effect" | "ambient_entity_effect" => vec![],
		"dust_color_transition" if payload.len() == 28 => {
			[&payload[..12], &payload[24..28], &payload[12..24]].concat()
		}
		n if BLOCK_PARTICLES.contains(&n) => block_payload(NEW, OLD, payload)?,
		"item" => encode(
			&to_legacy(decode(payload, |i| new::Slot::deserialize(i))?),
			old::slot::serialize,
		)?,
		_ => payload.to_vec(),
	})
}

fn particle_target(from: Version, to: Version, id: i32) -> Result<(&'static str, i32)> {
	let name = particle_name(from, id).ok_or(Error::Dropped)?;
	let id = particle_id(to, name)
		.filter(|_| common::later(to, from))
		.or_else(|| particle_id(to, common::translate("particles", from, to, name)));
	let id = id.ok_or(Error::Dropped)?;
	Ok((particle_name(to, id).ok_or(Error::Dropped)?, id))
}

fn new_particle(id: i32, color: i32, payload: &[u8]) -> Result<new::Particle::Ty> {
	let (target, new_id) = particle_target(OLD, NEW, id)?;
	let mut out = Vec::new();
	varint(&mut out, new_id);
	out.extend(up_payload(target, color, payload)?);
	decode(&out, |i| new::Particle::deserialize(i))
}

fn old_particle(p: &new::Particle::Ty) -> Result<(i32, &'static str, Vec<u8>, Vec<u8>)> {
	let bytes = encode(p, new::Particle::serialize)?;
	let (id, payload) = read_varint(&bytes)?;
	let (target, old_id) = particle_target(NEW, OLD, id)?;
	Ok((
		old_id,
		target,
		down_payload(target, payload)?,
		payload.to_vec(),
	))
}

fn particle_parts(p: &old::Particle::Ty) -> Result<(i32, Vec<u8>)> {
	let bytes = encode(p, old::Particle::serialize)?;
	let (id, payload) = read_varint(&bytes)?;
	Ok((id, payload.to_vec()))
}

pub fn trails_particle_1_20_3_to_1_20_5(
	v: old::Particle::Ty,
	_cx: &mut Session,
) -> Result<new::Particle::Ty> {
	let (id, payload) = particle_parts(&v)?;
	new_particle(id, 0, &payload)
}

pub fn trails_particle_1_20_5_to_1_20_3(
	v: new::Particle::Ty,
	_cx: &mut Session,
) -> Result<old::Particle::Ty> {
	let (id, _, payload, _) = old_particle(&v)?;
	let mut b = Vec::new();
	varint(&mut b, id);
	b.extend(payload);
	decode(&b, |i| old::Particle::deserialize(i))
}

pub(super) fn sound_up(s: old::ItemSoundHolder::Ty) -> Result<new::ItemSoundHolder::Ty> {
	use new::ItemSoundHolder::Ty as H;
	Ok(match s {
		old::ItemSoundHolder::Ty::Id(id) => match common::sound(OLD, NEW, id)? {
			common::Sound::Id(id) => H::Id(id),
			common::Sound::Named(soundName) => H::Inline(new::ItemSoundEvent::Ty {
				soundName,
				fixedRange: None,
			}),
		},
		old::ItemSoundHolder::Ty::Inline(e) => H::Inline(new::ItemSoundEvent::Ty {
			soundName: e.soundName,
			fixedRange: e.fixedRange,
		}),
	})
}

pub(super) fn sound_down(s: new::ItemSoundHolder::Ty) -> Result<old::ItemSoundHolder::Ty> {
	use old::ItemSoundHolder::Ty as H;
	Ok(match s {
		new::ItemSoundHolder::Ty::Id(id) => match common::sound(NEW, OLD, id)? {
			common::Sound::Id(id) => H::Id(id),
			common::Sound::Named(soundName) => H::Inline(old::ItemSoundEvent::Ty {
				soundName,
				fixedRange: None,
			}),
		},
		new::ItemSoundHolder::Ty::Inline(e) => H::Inline(old::ItemSoundEvent::Ty {
			soundName: e.soundName,
			fixedRange: e.fixedRange,
		}),
	})
}

pub fn trails_sound_1_20_3_to_1_20_5(
	v: old::ItemSoundHolder::Ty,
	_cx: &mut Session,
) -> Result<new::ItemSoundHolder::Ty> {
	sound_up(v)
}

pub fn trails_sound_1_20_5_to_1_20_3(
	v: new::ItemSoundHolder::Ty,
	_cx: &mut Session,
) -> Result<old::ItemSoundHolder::Ty> {
	sound_down(v)
}

pub fn trails_wolf_variant_1_20_5_to_1_21_1(
	id: i32,
	_cx: &mut Session,
) -> Result<crate::gen::convert::v1_21_1::entityMetadataEntry__value::Ty> {
	use crate::gen::convert::v1_21_1 as next;
	Ok(next::entityMetadataEntry__value::Ty::wolf_variant(
		next::entityMetadataEntry__value__wolf_variant::Ty::Id(id),
	))
}

pub fn trails_wolf_variant_1_21_1_to_1_20_5(
	v: crate::gen::convert::v1_21_1::entityMetadataEntry__value__wolf_variant::Ty,
	_cx: &mut Session,
) -> Result<new::entityMetadataEntry__value::Ty> {
	match v {
		crate::gen::convert::v1_21_1::entityMetadataEntry__value__wolf_variant::Ty::Id(id) => {
			Ok(new::entityMetadataEntry__value::Ty::wolf_variant(id))
		}
		_ => Err(Error::Dropped),
	}
}

const META_PARTICLES: i32 = 18;

fn meta_type_up(t: i32) -> i32 {
	let mut t = t;
	for inserted in [18, 23, 28] {
		if t >= inserted {
			t += 1;
		}
	}
	t
}

fn meta_type_down(t: i32) -> Option<i32> {
	if [18, 23, 28].contains(&t) {
		return None;
	}
	Some(t - [18, 23, 28].iter().filter(|i| t > **i).count() as i32)
}

fn entry_bytes(key: u8, ty: i32, payload: &[u8]) -> Vec<u8> {
	let mut out = vec![key];
	varint(&mut out, ty);
	out.extend_from_slice(payload);
	out
}

fn keys(version: Version, kind: &str) -> Option<&'static [String]> {
	let id = crate::registry::names(version, "entities")?.id(kind)?;
	crate::registry::entity_metadata_keys(version, id)
}

fn effect_particles(color: i32) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	if color == 0 {
		varint(&mut out, 0);
		return Ok(out);
	}
	varint(&mut out, 1);
	let id = particle_id(NEW, "entity_effect").ok_or(Error::Dropped)?;
	varint(&mut out, id);
	out.extend((color | (0xff << 24)).to_be_bytes());
	Ok(out)
}

const CARPETS: [&str; 16] = [
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
];

fn carpet(entity: i32, color: i32) -> Client {
	let item = usize::try_from(color)
		.ok()
		.and_then(|c| CARPETS.get(c))
		.and_then(|n| crate::registry::names(NEW, "items")?.id(n))
		.map_or(Slot::v0, |id| {
			Slot::default(
				1,
				new::Slot__default::Ty {
					itemId: id as i32,
					addedComponentCount: 0,
					removedComponentCount: 0,
					components: vec![],
					removeComponents: vec![],
				},
			)
		});
	Client::entity_equipment(new::play_toClient_packet_entity_equipment::Ty {
		entityId: entity,
		equipments: vec![
			new::play_toClient_packet_entity_equipment__equipments__items::Ty { slot: 6, item },
		],
	})
}

fn meta_entry(key: u8, ty: i32, payload: &[u8]) -> Result<new::entityMetadataEntry::Ty> {
	decode(&entry_bytes(key, ty, payload), |i| {
		new::entityMetadataEntry::deserialize(i)
	})
}

fn varint_bytes(v: i32) -> Vec<u8> {
	let mut b = Vec::new();
	varint(&mut b, v);
	b
}

pub fn trails_entity_metadata_1_20_3_to_1_20_5(
	p: old::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	use old::entityMetadataItem::Ty as V;
	let kind = common::entity_kind(cx, p.entityId).unwrap_or_default();
	let target_keys = keys(NEW, &kind).unwrap_or_default();
	let source_keys = keys(OLD, &kind).unwrap_or_default();
	let name_of = |key: u8| source_keys.get(key as usize).map(String::as_str);
	let key_of = |name: &str| target_keys.iter().position(|k| k == name).map(|i| i as u8);
	let cloud_color = p
		.metadata
		.iter()
		.find_map(|e| match (name_of(e.key), &e.value) {
			(Some("color"), V::int(c)) if kind == "area_effect_cloud" => Some(*c),
			_ => None,
		});
	let mut extra = Vec::new();
	let mut out = Vec::with_capacity(p.metadata.len());
	let mut has_particle = false;
	for e in p.metadata {
		let source_type = read_varint(&encode(
			&e.r#type,
			old::entityMetadata__items__type::serialize,
		)?)?
		.0;
		let name = name_of(e.key);
		match (kind.as_str(), name, &e.value) {
			(_, Some("effect_color"), V::int(color)) if key_of("effect_particles").is_some() => {
				out.push(meta_entry(
					key_of("effect_particles").expect("checked"),
					META_PARTICLES,
					&effect_particles(*color)?,
				)?);
				continue;
			}
			("llama" | "trader_llama", Some("swag"), V::int(color)) => {
				extra.push(carpet(p.entityId, *color));
				continue;
			}
			("area_effect_cloud", Some("color"), _) => continue,
			_ => {}
		}
		let key = match name {
			Some(n) => match key_of(n) {
				Some(k) => k,
				None => continue,
			},
			None => match meta_key(cx, OLD, NEW, p.entityId, e.key, &[]) {
				Some(k) => k,
				None => continue,
			},
		};
		let payload = match e.value {
			V::item_stack(s) => {
				let slot = to_components(s);
				if matches!(slot, Slot::v0) && matches!(name, Some("item_stack")) {
					continue;
				}
				encode(&slot, new::Slot::serialize)?
			}
			V::block_state(s) => varint_bytes(block_state(OLD, NEW, s)),
			V::optional_block_state(s) => varint_bytes(optional_state(OLD, NEW, s)),
			V::int(color) if kind == "arrow" && name == Some("effect_color") => {
				varint_bytes(if color == -1 {
					-1
				} else {
					color | (0xff << 24)
				})
			}
			V::particle(particle) => {
				let (id, payload) = particle_parts(&particle)?;
				let Ok(particle) = new_particle(id, cloud_color.unwrap_or(0), &payload) else {
					continue;
				};
				has_particle = true;
				encode(&particle, new::Particle::serialize)?
			}
			v => encode(&v, old::entityMetadataItem::serialize)?,
		};
		out.push(meta_entry(key, meta_type_up(source_type), &payload)?);
	}
	if let (Some(color), false, Some(key)) = (cloud_color, has_particle, key_of("particle")) {
		if color != 0 {
			let particle = new_particle(
				particle_id(OLD, "entity_effect").ok_or(Error::Dropped)?,
				color,
				&[],
			)?;
			out.push(meta_entry(
				key,
				17,
				&encode(&particle, new::Particle::serialize)?,
			)?);
		}
	}
	let mut packets = vec![Client::entity_metadata(
		new::play_toClient_packet_entity_metadata::Ty {
			entityId: p.entityId,
			metadata: out,
		},
	)];
	packets.extend(extra);
	Ok(packets)
}

fn effect_color(particle: &new::Particle::Ty) -> Option<i32> {
	let b = encode(particle, new::Particle::serialize).ok()?;
	let (id, rest) = read_varint(&b).ok()?;
	(particle_name(NEW, id) == Some("entity_effect") && rest.len() == 4)
		.then(|| i32::from_be_bytes([0, rest[1], rest[2], rest[3]]))
}

pub fn trails_entity_metadata_1_20_5_to_1_20_3(
	p: new::play_toClient_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	use new::entityMetadataEntry__value::Ty as V;
	let kind = common::entity_kind(cx, p.entityId).unwrap_or_default();
	let source_keys = keys(NEW, &kind).unwrap_or_default();
	let target_keys = keys(OLD, &kind).unwrap_or_default();
	let key_of = |name: &str| target_keys.iter().position(|k| k == name).map(|i| i as u8);
	let mut out = Vec::with_capacity(p.metadata.len());
	for e in p.metadata {
		let name = source_keys.get(e.key as usize).map(String::as_str);
		let target = match name {
			Some("effect_particles") => "effect_color",
			Some(n) => n,
			None => "",
		};
		let key = match key_of(target) {
			Some(k) => k,
			None if name.is_none() => e.key,
			None => continue,
		};
		let bytes = encode(&e.value, new::entityMetadataEntry__value::serialize)?;
		let (source_type, payload) = read_varint(&bytes)?;
		let (ty, payload) = match e.value {
			V::particles(list) => (
				1,
				varint_bytes(list.iter().find_map(effect_color).unwrap_or(0)),
			),
			V::int(color) if kind == "arrow" && target == "effect_color" => (
				1,
				varint_bytes(if color == -1 { -1 } else { color & 0xff_ffff }),
			),
			V::item_stack(s) => (7, encode(&to_legacy(s), old::slot::serialize)?),
			V::block_state(s) => (14, varint_bytes(block_state(NEW, OLD, s))),
			V::optional_block_state(s) => (15, varint_bytes(optional_state(NEW, OLD, s))),
			V::particle(particle) => {
				let Ok((id, _, payload, _)) = old_particle(&particle) else {
					continue;
				};
				if kind == "area_effect_cloud" {
					if let (Some(color), Some(color_key)) =
						(effect_color(&particle), key_of("color"))
					{
						out.push(decode(
							&entry_bytes(color_key, 1, &varint_bytes(color)),
							|i| old::entityMetadata__items::deserialize(i),
						)?);
					}
				}
				let mut b = varint_bytes(id);
				b.extend(payload);
				(17, b)
			}
			V::wolf_variant(_) | V::armadillo_state(_) | V::particles(_) => continue,
			_ => match meta_type_down(source_type) {
				Some(t) => (t, payload.to_vec()),
				None => continue,
			},
		};
		out.push(decode(&entry_bytes(key, ty, &payload), |i| {
			old::entityMetadata__items::deserialize(i)
		})?);
	}
	Ok(vec![ClientOld::entity_metadata(
		old::play_toClient_packet_entity_metadata::Ty {
			entityId: p.entityId,
			metadata: out,
		},
	)])
}

fn vanilla_registries() -> Result<serde_json::Value> {
	Ok(super::codec::login(NEW)?["dimensionCodec"].clone())
}

fn vanilla_entries(registry: &serde_json::Value) -> Vec<(String, Payload)> {
	registry["entries"]
		.as_array()
		.map(|a| {
			a.iter()
				.filter_map(|e| {
					Some((
						e["key"].as_str()?.to_owned(),
						nbt::json::typed::read(&e["value"])?,
					))
				})
				.collect()
		})
		.unwrap_or_default()
}

const PARTICLE_KEYS: &[(&[&str], &str, &str)] = &[
	(
		&["block", "block_marker", "falling_dust", "dust_pillar"],
		"value",
		"block_state",
	),
	(&["item"], "value", "item"),
	(&["dust_color_transition"], "fromColor", "from_color"),
	(&["dust_color_transition"], "toColor", "to_color"),
	(&["entity_effect"], "value", "color"),
];

fn element(registry: &str, element: &mut Payload, up: bool) {
	let Payload::compound(c) = element else {
		return;
	};
	match registry {
		"dimension_type" => {
			let Some(light) = c.compound_mut("monster_spawn_light_level") else {
				return;
			};
			if up {
				if let Some(Payload::compound(value)) = light.take("value") {
					light.extend(value);
				}
			} else if let (Some(min), Some(max)) =
				(light.take("min_inclusive"), light.take("max_inclusive"))
			{
				light.put(
					"value",
					Payload::compound(vec![
						Entry::new("min_inclusive", min),
						Entry::new("max_inclusive", max),
					]),
				);
			}
		}
		"worldgen/biome" => {
			let Some(effects) = c.compound_mut("effects") else {
				return;
			};
			let Some(particle) = effects.compound_mut("particle") else {
				return;
			};
			let Some(options) = particle.compound_mut("options") else {
				return;
			};
			let kind = options
				.field("type")
				.and_then(Payload::as_str)
				.map(|t| strip(t).to_owned())
				.unwrap_or_default();
			for (kinds, old, new) in PARTICLE_KEYS {
				if kinds.contains(&kind.as_str()) {
					let (from, to) = if up { (old, new) } else { (new, old) };
					options.rename(from, to);
				}
			}
		}
		"trim_material" if !up => {
			if let Some(overrides) = c.compound_mut("override_armor_materials") {
				for e in overrides.iter_mut() {
					e.name = strip(&e.name).to_owned();
				}
			}
		}
		_ => {}
	}
}

fn remember(cx: &mut Session, registry: &str, keys: Vec<String>) {
	if registry == "dimension_type" {
		common::state(cx).dimensions = keys;
	}
}

fn strip(s: &str) -> &str {
	s.strip_prefix("minecraft:").unwrap_or(s)
}

fn registry_packet(id: &str, entries: Vec<(String, Payload)>) -> Config {
	Config::registry_data(new::configuration_toClient_packet_registry_data::Ty {
		id: id.to_owned(),
		entries: entries
			.into_iter()
			.map(|(key, value)| {
				new::configuration_toClient_packet_registry_data__entries__items::Ty {
					key,
					value: Some(Tag::new(value)),
				}
			})
			.collect(),
	})
}

fn vanilla_keys(registry: &str) -> Vec<String> {
	vanilla_registries()
		.ok()
		.map(|v| {
			vanilla_entries(&v[format!("minecraft:{registry}").as_str()])
				.into_iter()
				.map(|(k, _)| k)
				.collect()
		})
		.unwrap_or_default()
}

fn dimension_names(cx: &mut Session) -> Vec<String> {
	let known = common::state(cx).dimensions.clone();
	if known.is_empty() {
		vanilla_keys("dimension_type")
	} else {
		known
	}
}

pub fn trails_registry_data_1_20_3_to_1_20_5(codec: Tag, cx: &mut Session) -> Result<Vec<Config>> {
	let Payload::compound(root) = codec.value else {
		return unconvertible("registry data is not a compound");
	};
	let vanilla = vanilla_registries()?;
	let mut out = vec![Config::select_known_packs(vec![])];
	let mut seen = Vec::new();
	for registry in root {
		let id = registry.name.clone();
		let short = strip(&id).to_owned();
		let mut entries: Vec<(i32, String, Payload)> = registry
			.value
			.field("value")
			.map(Payload::values)
			.unwrap_or_default()
			.into_iter()
			.filter_map(|e| {
				let Payload::compound(mut c) = e else {
					return None;
				};
				let name = c.field("name").and_then(Payload::as_str)?.to_owned();
				let id = c.field("id").and_then(Payload::to_i32).unwrap_or(i32::MAX);
				let mut value = c.take("element")?;
				element(&short, &mut value, true);
				Some((id, name, value))
			})
			.collect();
		entries.sort_by_key(|(id, _, _)| *id);
		let mut entries: Vec<(String, Payload)> =
			entries.into_iter().map(|(_, n, e)| (n, e)).collect();
		if short == "damage_type" {
			for (key, value) in vanilla_entries(&vanilla["minecraft:damage_type"]) {
				if !entries.iter().any(|(k, _)| strip(k) == strip(&key)) {
					entries.push((key, value));
				}
			}
		}
		remember(cx, &short, entries.iter().map(|(k, _)| k.clone()).collect());
		seen.push(short);
		out.push(registry_packet(&id, entries));
	}
	if let Some(registries) = vanilla.as_object() {
		for (id, registry) in registries {
			if !seen.iter().any(|s| s == strip(id)) {
				out.push(registry_packet(id, vanilla_entries(registry)));
			}
		}
	}
	Ok(out)
}

pub fn trails_registry_data_1_20_5_to_1_20_3(
	p: new::configuration_toClient_packet_registry_data::Ty,
	cx: &mut Session,
) -> Result<Vec<ConfigOld>> {
	let vanilla = vanilla_registries()?;
	let fallback = vanilla_entries(&vanilla[p.id.as_str()]);
	let short = strip(&p.id).to_owned();
	remember(
		cx,
		&short,
		p.entries.iter().map(|e| e.key.clone()).collect(),
	);
	if ["wolf_variant", "banner_pattern"].contains(&short.as_str()) {
		return Ok(vec![]);
	}
	let elements: Vec<Payload> = p
		.entries
		.into_iter()
		.enumerate()
		.filter_map(|(i, e)| {
			let mut value = match e.value {
				Some(t) => t.value,
				None => fallback
					.iter()
					.find(|(k, _)| strip(k) == strip(&e.key))?
					.1
					.clone(),
			};
			element(&short, &mut value, false);
			Some(Payload::compound(vec![
				Entry::new("name", Payload::string(e.key)),
				Entry::new("id", Payload::int(i as i32)),
				Entry::new("element", value),
			]))
		})
		.collect();
	let registry = Payload::compound(vec![
		Entry::new("type", Payload::string(p.id.clone())),
		Entry::new("value", List::of(elements)),
	]);
	common::state(cx).pending.push(Entry::new(&p.id, registry));
	Ok(vec![])
}

pub fn trails_finish_configuration_1_20_5_to_1_20_3(
	p: new::configuration_toClient_packet_finish_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<ConfigOld>> {
	let pending = std::mem::take(&mut common::state(cx).pending);
	let mut out = Vec::new();
	if !pending.is_empty() {
		out.push(ConfigOld::registry_data(Tag::new(Payload::compound(
			pending,
		))));
	}
	out.push(ConfigOld::finish_configuration(p));
	Ok(out)
}

fn config_tags(
	from: Version,
	to: Version,
	mut tags: Vec<old::configuration_toClient_packet__tags__items::Ty>,
) -> Vec<old::configuration_toClient_packet__tags__items::Ty> {
	for registry in &mut tags {
		for t in &mut registry.tags {
			t.entries = tag_entries(&registry.tagType, from, to, std::mem::take(&mut t.entries));
		}
	}
	tags
}

arm!(
	trails_config_tags_1_20_3_to_1_20_5,
	Config,
	tags,
	Vec<old::configuration_toClient_packet__tags__items::Ty>,
	|p, _cx| config_tags(OLD, NEW, p)
);

arm!(
	trails_config_tags_1_20_5_to_1_20_3,
	ConfigOld,
	tags,
	Vec<new::configuration_toClient_packet__tags__items::Ty>,
	|p, _cx| config_tags(NEW, OLD, p)
);

arm!(
	trails_tags_1_20_3_to_1_20_5,
	Client,
	tags,
	Vec<old::play_toClient_packet__tags__items::Ty>,
	|p, _cx| super::step_1_20::remap_tags(OLD, NEW, p)
);

arm!(
	trails_tags_1_20_5_to_1_20_3,
	ClientOld,
	tags,
	Vec<new::play_toClient_packet__tags__items::Ty>,
	|p, _cx| super::step_1_20::remap_tags(NEW, OLD, p)
);

fn gamemode(g: u8) -> Result<new::play_toClient_SpawnInfo__gamemode::Ty> {
	decode(&[g], |i| {
		new::play_toClient_SpawnInfo__gamemode::deserialize(i)
	})
}

fn gamemode_byte(g: &new::play_toClient_SpawnInfo__gamemode::Ty) -> Result<u8> {
	Ok(
		encode(g, new::play_toClient_SpawnInfo__gamemode::serialize)?
			.first()
			.copied()
			.unwrap_or(0),
	)
}

fn dimension_id(cx: &mut Session, name: &str) -> i32 {
	let names = dimension_names(cx);
	names
		.iter()
		.position(|n| strip(n) == strip(name))
		.unwrap_or(0) as i32
}

fn dimension_name(cx: &mut Session, id: i32) -> String {
	dimension_names(cx)
		.get(id as usize)
		.cloned()
		.unwrap_or_else(|| "minecraft:overworld".to_owned())
}

const CREATIVE_BLOCK_RANGE: u128 = 0x736565d2_e1a7_403d_a3f8_1aeb3e302542;
const CREATIVE_ENTITY_RANGE: u128 = 0x98491ef6_97b1_4584_ae82_71a8cc85cf73;

fn range_attributes(entity: i32, creative: bool) -> Result<Client> {
	let attribute = |name: &str,
	                 base: f64,
	                 modifier: Option<(u128, f64)>|
	 -> Result<
		new::play_toClient_packet_entity_update_attributes__properties__items::Ty,
	> {
		let id = items::ATTRIBUTES
			.iter()
			.position(|a| *a == name)
			.ok_or(Error::Dropped)? as u8;
		Ok(new::play_toClient_packet_entity_update_attributes__properties__items::Ty {
			key: decode(&[id], |i| new::play_toClient_packet_entity_update_attributes__properties__items__key::deserialize(i))?,
			value: base,
			modifiers: modifier
				.map(|(uuid, amount)| new::play_toClient_packet_entity_update_attributes__properties__items__modifiers__items::Ty { uuid, amount, operation: 0 })
				.into_iter()
				.collect(),
		})
	};
	Ok(Client::entity_update_attributes(
		new::play_toClient_packet_entity_update_attributes::Ty {
			entityId: entity,
			properties: vec![
				attribute(
					"player.block_interaction_range",
					4.5,
					creative.then_some((CREATIVE_BLOCK_RANGE, 0.5)),
				)?,
				attribute(
					"player.entity_interaction_range",
					3.0,
					creative.then_some((CREATIVE_ENTITY_RANGE, 2.0)),
				)?,
			],
		},
	))
}

pub fn trails_login_1_20_3_to_1_20_5(
	p: old::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	let s = common::state(cx);
	s.player = Some(p.entityId);
	let secure = s.secure_chat.unwrap_or(false);
	let dimension = dimension_id(cx, &p.worldType);
	let creative = p.gameMode & 3 == 1;
	Ok(vec![
		Client::login(new::play_toClient_packet_login::Ty {
			entityId: p.entityId,
			isHardcore: p.isHardcore,
			worldNames: p.worldNames,
			maxPlayers: p.maxPlayers,
			viewDistance: p.viewDistance,
			simulationDistance: p.simulationDistance,
			reducedDebugInfo: p.reducedDebugInfo,
			enableRespawnScreen: p.enableRespawnScreen,
			doLimitedCrafting: p.doLimitedCrafting,
			worldState: new::play_toClient_SpawnInfo::Ty {
				dimension,
				name: p.worldName,
				hashedSeed: p.hashedSeed,
				gamemode: gamemode(p.gameMode)?,
				previousGamemode: p.previousGameMode as u8,
				isDebug: p.isDebug,
				isFlat: p.isFlat,
				death: p
					.death
					.map(|d| new::play_toClient_SpawnInfo__death__items::Ty {
						dimensionName: d.dimensionName,
						location: d.location,
					}),
				portalCooldown: p.portalCooldown,
			},
			enforcesSecureChat: secure,
		}),
		range_attributes(p.entityId, creative)?,
	])
}

pub fn trails_login_1_20_5_to_1_20_3(
	p: new::play_toClient_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let w = p.worldState;
	common::state(cx).player = Some(p.entityId);
	Ok(vec![ClientOld::login(
		old::play_toClient_packet_login::Ty {
			entityId: p.entityId,
			isHardcore: p.isHardcore,
			worldNames: p.worldNames,
			maxPlayers: p.maxPlayers,
			viewDistance: p.viewDistance,
			simulationDistance: p.simulationDistance,
			reducedDebugInfo: p.reducedDebugInfo,
			enableRespawnScreen: p.enableRespawnScreen,
			doLimitedCrafting: p.doLimitedCrafting,
			worldType: dimension_name(cx, w.dimension),
			worldName: w.name,
			hashedSeed: w.hashedSeed,
			gameMode: gamemode_byte(&w.gamemode)?,
			previousGameMode: w.previousGamemode as i8,
			isDebug: w.isDebug,
			isFlat: w.isFlat,
			death: w
				.death
				.map(|d| old::play_toClient_packet_login__death__items::Ty {
					dimensionName: d.dimensionName,
					location: d.location,
				}),
			portalCooldown: w.portalCooldown,
		},
	)])
}

pub fn trails_respawn_1_20_3_to_1_20_5(
	p: old::play_toClient_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	let dimension = dimension_id(cx, &p.dimension);
	let player = common::state(cx).player;
	let mut out = vec![Client::respawn(new::play_toClient_packet_respawn::Ty {
		worldState: new::play_toClient_SpawnInfo::Ty {
			dimension,
			name: p.worldName,
			hashedSeed: p.hashedSeed,
			gamemode: gamemode(p.gamemode as u8)?,
			previousGamemode: p.previousGamemode,
			isDebug: p.isDebug,
			isFlat: p.isFlat,
			death: p
				.death
				.map(|d| new::play_toClient_SpawnInfo__death__items::Ty {
					dimensionName: d.dimensionName,
					location: d.location,
				}),
			portalCooldown: p.portalCooldown,
		},
		copyMetadata: p.copyMetadata as u8,
	})];
	if let Some(player) = player {
		out.push(range_attributes(player, p.gamemode & 3 == 1)?);
	}
	Ok(out)
}

pub fn trails_respawn_1_20_5_to_1_20_3(
	p: new::play_toClient_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let w = p.worldState;
	Ok(vec![ClientOld::respawn(
		old::play_toClient_packet_respawn::Ty {
			dimension: dimension_name(cx, w.dimension),
			worldName: w.name,
			hashedSeed: w.hashedSeed,
			gamemode: gamemode_byte(&w.gamemode)? as i8,
			previousGamemode: w.previousGamemode,
			isDebug: w.isDebug,
			isFlat: w.isFlat,
			death: w
				.death
				.map(|d| old::play_toClient_packet_respawn__death__items::Ty {
					dimensionName: d.dimensionName,
					location: d.location,
				}),
			portalCooldown: w.portalCooldown,
			copyMetadata: p.copyMetadata != 0,
		},
	)])
}

pub fn trails_game_state_change_1_20_3_to_1_20_5(
	p: old::play_toClient_packet_game_state_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	let mode = matches!(
		p.reason,
		old::play_toClient_packet_game_state_change__reason::Ty::change_game_mode
	);
	let creative = (p.gameMode + 0.5).floor() as i32 == 1;
	let player = common::state(cx).player;
	let mut out = vec![Client::game_state_change(p)];
	if let (true, Some(player)) = (mode, player) {
		out.push(range_attributes(player, creative)?);
	}
	Ok(out)
}

pub fn trails_server_data_1_20_3_to_1_20_5(
	p: old::play_toClient_packet_server_data::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	common::state(cx).secure_chat = Some(p.enforcesSecureChat);
	Ok(vec![Client::server_data(
		new::play_toClient_packet_server_data::Ty {
			motd: p.motd,
			iconBytes: p.iconBytes,
		},
	)])
}

pub fn trails_secure_chat_1_20_5_to_1_20_3(
	_src: &new::play_toClient_packet_server_data::Ty,
	cx: &mut Session,
) -> Result<bool> {
	Ok(common::state(cx).secure_chat.unwrap_or(false))
}

pub fn trails_position_flags_1_20_3_to_1_20_5(
	v: i8,
	_cx: &mut Session,
) -> Result<new::play_toClient_PositionUpdateRelatives::Ty> {
	decode(&[v as u8 & 0x1f], |i| {
		new::play_toClient_PositionUpdateRelatives::deserialize(i)
	})
}

pub fn trails_position_flags_1_20_5_to_1_20_3(
	v: new::play_toClient_PositionUpdateRelatives::Ty,
	_cx: &mut Session,
) -> Result<i8> {
	Ok(
		encode(&v, new::play_toClient_PositionUpdateRelatives::serialize)?
			.first()
			.copied()
			.unwrap_or(0) as i8,
	)
}

pub fn trails_attribute_key_1_20_3_to_1_20_5(
	src: &old::play_toClient_packet_entity_update_attributes__properties__items::Ty,
	_cx: &mut Session,
) -> Result<new::play_toClient_packet_entity_update_attributes__properties__items__key::Ty> {
	let name = strip(&src.name);
	let name = if name == "horse.jump_strength" {
		"generic.jump_strength"
	} else {
		name
	};
	let id = items::ATTRIBUTES
		.iter()
		.position(|a| *a == name)
		.ok_or(Error::Dropped)? as u8;
	decode(&[id], |i| {
		new::play_toClient_packet_entity_update_attributes__properties__items__key::deserialize(i)
	})
}

pub fn trails_attribute_name_1_20_5_to_1_20_3(
	src: &new::play_toClient_packet_entity_update_attributes__properties__items::Ty,
	_cx: &mut Session,
) -> Result<String> {
	let id = read_varint(&encode(
		&src.key,
		new::play_toClient_packet_entity_update_attributes__properties__items__key::serialize,
	)?)?
	.0;
	let name = items::ATTRIBUTES.get(id as usize).ok_or(Error::Dropped)?;
	Ok(format!("minecraft:{name}"))
}

pub fn trails_entity_sound_1_20_3_to_1_20_5(
	src: &old::play_toClient_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<new::ItemSoundHolder::Ty> {
	use old::play_toClient_packet_entity_sound_effect__soundEvent::Ty as E;
	sound_up(match &src.soundEvent {
		E::v0(e) => old::ItemSoundHolder::Ty::Inline(old::ItemSoundEvent::Ty {
			soundName: e.resource.clone(),
			fixedRange: e.range,
		}),
		E::default(tag) => old::ItemSoundHolder::Ty::Id(tag - 1),
	})
}

pub fn trails_entity_sound_1_20_5_to_1_20_3(
	src: &new::play_toClient_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<old::play_toClient_packet_entity_sound_effect__soundEvent::Ty> {
	use old::play_toClient_packet_entity_sound_effect__soundEvent::Ty as E;
	Ok(match sound_down(src.sound.clone())? {
		old::ItemSoundHolder::Ty::Id(id) => E::default(id + 1),
		old::ItemSoundHolder::Ty::Inline(e) => E::v0(
			old::play_toClient_packet_entity_sound_effect__soundEvent__v0::Ty {
				resource: e.soundName,
				range: e.fixedRange,
			},
		),
	})
}

fn cost(s: Slot) -> Option<new::play_toClient_packet_trade_list__trades__items__inputItem1::Ty> {
	let Slot::default(count, d) = s else {
		return None;
	};
	let components: Vec<_> = d
		.components
		.into_iter()
		.filter(|c| !matches!(c, new::SlotComponent::Ty::custom_data(_)))
		.collect();
	Some(
		new::play_toClient_packet_trade_list__trades__items__inputItem1::Ty {
			itemId: d.itemId,
			itemCount: count as i32,
			addedComponentCount: components.len() as i32,
			components,
		},
	)
}

pub fn trails_trade_input_1_20_3_to_1_20_5(
	v: old::slot::Ty,
	_cx: &mut Session,
) -> Result<new::play_toClient_packet_trade_list__trades__items__inputItem1::Ty> {
	Ok(cost(non_empty(v)).expect("non_empty yields an item"))
}

pub fn trails_trade_input_2_1_20_3_to_1_20_5(
	v: old::slot::Ty,
	_cx: &mut Session,
) -> Result<Option<new::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty>> {
	Ok(cost(to_components(v)).map(|c| {
		new::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty {
			itemId: c.itemId,
			itemCount: c.itemCount,
			addedComponentCount: c.addedComponentCount,
			components: c.components,
		}
	}))
}

fn from_cost(itemId: i32, count: i32, components: Vec<new::SlotComponent::Ty>) -> old::slot::Ty {
	to_legacy(Slot::default(
		count as i8,
		new::Slot__default::Ty {
			itemId,
			addedComponentCount: components.len() as i32,
			removedComponentCount: 0,
			components,
			removeComponents: vec![],
		},
	))
}

pub fn trails_trade_input_1_20_5_to_1_20_3(
	v: new::play_toClient_packet_trade_list__trades__items__inputItem1::Ty,
	_cx: &mut Session,
) -> Result<old::slot::Ty> {
	Ok(from_cost(v.itemId, v.itemCount, v.components))
}

pub fn trails_trade_input_2_1_20_5_to_1_20_3(
	v: Option<new::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty>,
	_cx: &mut Session,
) -> Result<old::slot::Ty> {
	Ok(v.map_or(old::slot::Ty::r#false, |v| {
		from_cost(v.itemId, v.itemCount, v.components)
	}))
}

pub fn trails_world_particles_1_20_3_to_1_20_5(
	p: old::play_toClient_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	let payload = encode(&p.data, old::particleData::serialize)?;
	let (target, _) = particle_target(OLD, NEW, p.particleId)?;
	let color = match target {
		"entity_effect" if p.particleData == 0.0 => 0,
		"entity_effect" if p.particles == 0 => {
			let c = |v: f32| ((v * 255.0).round() as i32).clamp(0, 255);
			(c(p.offsetX) << 16) | (c(p.offsetY) << 8) | c(p.offsetZ)
		}
		_ => 0xffffff,
	};
	let particle = new_particle(p.particleId, color, &payload)?;
	Ok(vec![Client::world_particles(
		new::play_toClient_packet_world_particles::Ty {
			longDistance: p.longDistance,
			x: p.x,
			y: p.y,
			z: p.z,
			offsetX: p.offsetX,
			offsetY: p.offsetY,
			offsetZ: p.offsetZ,
			velocityOffset: p.particleData,
			amount: p.particles,
			particle,
		},
	)])
}

pub fn trails_world_particles_1_20_5_to_1_20_3(
	p: new::play_toClient_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let (id, name, payload, raw) = old_particle(&p.particle)?;
	let (mut offset, mut speed, mut count) = (
		[p.offsetX, p.offsetY, p.offsetZ],
		p.velocityOffset,
		p.amount,
	);
	if name == "entity_effect" && raw.len() == 4 {
		offset = [raw[1], raw[2], raw[3]].map(|c| c as f32 / 255.0);
		speed = 1.0;
		count = 0;
	}
	Ok(vec![ClientOld::world_particles(
		old::play_toClient_packet_world_particles::Ty {
			particleId: id,
			longDistance: p.longDistance,
			x: p.x,
			y: p.y,
			z: p.z,
			offsetX: offset[0],
			offsetY: offset[1],
			offsetZ: offset[2],
			particleData: speed,
			particles: count,
			data: decode(&payload, |i| {
				old::particleData::deserialize(i, old::particleData::Args { compareTo: &id })
			})?,
		},
	)])
}

fn block_entity_name(kind: i32) -> Option<&'static str> {
	match kind {
		15 => Some("skull"),
		19 => Some("banner"),
		_ => None,
	}
}

pub fn trails_chunk_entity_nbt_1_20_5(kind: i32, nbt: &mut Option<Tag>, up: bool) {
	match up {
		true => block_entity_up(kind, nbt),
		false => block_entity_down(kind, nbt),
	}
}

fn block_entity_up(kind: i32, nbt: &mut Option<Tag>) {
	if let (
		Some(name),
		Some(Tag {
			value: Payload::compound(c),
			..
		}),
	) = (block_entity_name(kind), nbt)
	{
		items::block_entity_1_20_5(name, c);
	}
}

fn block_entity_down(kind: i32, nbt: &mut Option<Tag>) {
	if let (
		Some(name),
		Some(Tag {
			value: Payload::compound(c),
			..
		}),
	) = (block_entity_name(kind), nbt.as_mut())
	{
		items::block_entity_1_20_3(name, c);
	}
	if matches!(nbt, Some(Tag { value: Payload::compound(c), .. }) if c.is_empty()) {
		*nbt = None;
	}
}

fn chunk_data(from: Version, to: Version, data: &[u8]) -> Result<Vec<u8>> {
	let table = common::state_map(from, to);
	let state = |s: u32| common::mapped(table, s as i32) as u32;
	let biome = |b: u32| b;
	super::chunk::sections(
		data,
		&super::chunk::Remap {
			state: &state,
			states: common::states(to),
			biome: &biome,
			biomes: None,
		},
		from.index(),
		None,
	)
}

arm!(
	trails_map_chunk_1_20_3_to_1_20_5,
	Client,
	map_chunk,
	old::play_toClient_packet_map_chunk::Ty,
	|mut p, _cx| {
		p.chunkData = chunk_data(OLD, NEW, &p.chunkData)?;
		for b in &mut p.blockEntities {
			block_entity_up(b.r#type, &mut b.nbtData);
		}
		p
	}
);

arm!(
	trails_map_chunk_1_20_5_to_1_20_3,
	ClientOld,
	map_chunk,
	new::play_toClient_packet_map_chunk::Ty,
	|mut p, _cx| {
		p.chunkData = chunk_data(NEW, OLD, &p.chunkData)?;
		for b in &mut p.blockEntities {
			block_entity_down(b.r#type, &mut b.nbtData);
		}
		p
	}
);

arm!(
	trails_tile_entity_data_1_20_3_to_1_20_5,
	Client,
	tile_entity_data,
	old::play_toClient_packet_tile_entity_data::Ty,
	|mut p, _cx| {
		block_entity_up(p.action, &mut p.nbtData);
		if p.nbtData.is_none() {
			p.nbtData = Some(Tag::new(Payload::compound(vec![])));
		}
		p
	}
);

arm!(
	trails_tile_entity_data_1_20_5_to_1_20_3,
	ClientOld,
	tile_entity_data,
	new::play_toClient_packet_tile_entity_data::Ty,
	|mut p, _cx| {
		block_entity_down(p.action, &mut p.nbtData);
		p
	}
);

arm!(
	trails_multi_block_change_1_20_3_to_1_20_5,
	Client,
	multi_block_change,
	old::play_toClient_packet_multi_block_change::Ty,
	|mut p, _cx| {
		p.records = p
			.records
			.into_iter()
			.map(|r| record(OLD, NEW, r as i64) as i32)
			.collect();
		p
	}
);

arm!(
	trails_multi_block_change_1_20_5_to_1_20_3,
	ClientOld,
	multi_block_change,
	new::play_toClient_packet_multi_block_change::Ty,
	|mut p, _cx| {
		p.records = p
			.records
			.into_iter()
			.map(|r| record(NEW, OLD, r as i64) as i32)
			.collect();
		p
	}
);

arm!(
	trails_block_change_1_20_3_to_1_20_5,
	Client,
	block_change,
	old::play_toClient_packet_block_change::Ty,
	|mut p, _cx| {
		p.r#type = block_state(OLD, NEW, p.r#type);
		p
	}
);

arm!(
	trails_block_change_1_20_5_to_1_20_3,
	ClientOld,
	block_change,
	new::play_toClient_packet_block_change::Ty,
	|mut p, _cx| {
		p.r#type = block_state(NEW, OLD, p.r#type);
		p
	}
);

arm!(
	trails_block_action_1_20_3_to_1_20_5,
	Client,
	block_action,
	old::play_toClient_packet_block_action::Ty,
	|mut p, _cx| {
		p.blockId = block(OLD, NEW, p.blockId).ok_or(Error::Dropped)?;
		p
	}
);

arm!(
	trails_block_action_1_20_5_to_1_20_3,
	ClientOld,
	block_action,
	new::play_toClient_packet_block_action::Ty,
	|mut p, _cx| {
		p.blockId = block(NEW, OLD, p.blockId).ok_or(Error::Dropped)?;
		p
	}
);

arm!(
	trails_world_event_1_20_3_to_1_20_5,
	Client,
	world_event,
	old::play_toClient_packet_world_event::Ty,
	|mut p, _cx| {
		p.data = world_event_data(OLD, NEW, p.effectId, p.data);
		p
	}
);

arm!(
	trails_world_event_1_20_5_to_1_20_3,
	ClientOld,
	world_event,
	new::play_toClient_packet_world_event::Ty,
	|mut p, _cx| {
		p.data = world_event_data(NEW, OLD, p.effectId, p.data);
		p
	}
);

macro_rules! spawn_entity {
	($name:ident, $root:ty, $src:ident, $from:expr, $to:expr) => {
		arm!(
			$name,
			$root,
			spawn_entity,
			$src::play_toClient_packet_spawn_entity::Ty,
			|mut p, cx| {
				common::spawned(cx, $from, p.entityId, p.r#type);
				let kind = crate::registry::names($from, "entities")
					.and_then(|n| n.name(p.r#type as u32))
					.unwrap_or_default();
				p.objectData = spawn_object_data($from, $to, kind, p.objectData);
				p.r#type = name_id("entities", $from, $to, p.r#type).ok_or(Error::Dropped)?;
				p
			}
		);
	};
}

spawn_entity!(trails_spawn_entity_1_20_3_to_1_20_5, Client, old, OLD, NEW);
spawn_entity!(
	trails_spawn_entity_1_20_5_to_1_20_3,
	ClientOld,
	new,
	NEW,
	OLD
);

arm!(
	trails_set_cooldown_1_20_3_to_1_20_5,
	Client,
	set_cooldown,
	old::play_toClient_packet_set_cooldown::Ty,
	|mut p, _cx| {
		p.itemID = item(OLD, NEW, p.itemID).ok_or(Error::Dropped)?;
		p
	}
);

arm!(
	trails_set_cooldown_1_20_5_to_1_20_3,
	ClientOld,
	set_cooldown,
	new::play_toClient_packet_set_cooldown::Ty,
	|mut p, _cx| {
		p.itemID = item(NEW, OLD, p.itemID).ok_or(Error::Dropped)?;
		p
	}
);

const RECIPE_TYPES: &[&str] = &[
	"crafting_shaped",
	"crafting_shapeless",
	"crafting_special_armordye",
	"crafting_special_bookcloning",
	"crafting_special_mapcloning",
	"crafting_special_mapextending",
	"crafting_special_firework_rocket",
	"crafting_special_firework_star",
	"crafting_special_firework_star_fade",
	"crafting_special_tippedarrow",
	"crafting_special_bannerduplicate",
	"crafting_special_banneraddpattern",
	"crafting_special_shielddecoration",
	"crafting_special_shulkerboxcoloring",
	"crafting_special_suspiciousstew",
	"crafting_special_repairitem",
	"smelting",
	"blasting",
	"smoking",
	"campfire_cooking",
	"stonecutting",
	"smithing_transform",
	"smithing_trim",
	"crafting_decorated_pot",
];

fn up_list(v: Vec<old::slot::Ty>) -> Vec<Slot> {
	v.into_iter()
		.map(to_components)
		.filter(|s| !matches!(s, Slot::v0))
		.collect()
}

fn down_list(v: Vec<Slot>) -> Vec<old::slot::Ty> {
	v.into_iter().map(to_legacy).collect()
}

fn smelting_up(s: old::minecraft_smelting_format::Ty) -> new::minecraft_smelting_format::Ty {
	new::minecraft_smelting_format::Ty {
		group: s.group,
		category: s.category,
		ingredient: up_list(s.ingredient),
		result: non_empty(s.result),
		experience: s.experience,
		cookTime: s.cookTime,
	}
}

fn smelting_down(s: new::minecraft_smelting_format::Ty) -> old::minecraft_smelting_format::Ty {
	old::minecraft_smelting_format::Ty {
		group: s.group,
		category: s.category,
		ingredient: down_list(s.ingredient),
		result: to_legacy(s.result),
		experience: s.experience,
		cookTime: s.cookTime,
	}
}

fn recipe_up(
	r: old::play_toClient_packet__declare_recipes__items::Ty,
) -> Result<Option<new::play_toClient_packet__declare_recipes__items::Ty>> {
	use new::play_toClient_packet__declare_recipes__items__data::Ty as N;
	use old::play_toClient_packet__declare_recipes__items__data::Ty as O;
	let Some(id) = RECIPE_TYPES.iter().position(|t| *t == strip(&r.r#type)) else {
		return Ok(None);
	};
	let data = match r.data {
		O::minecraft_crafting_shaped(s) => N::minecraft_crafting_shaped(new::play_toClient_packet__declare_recipes__items__data__minecraft_crafting_shaped::Ty {
			group: s.group,
			category: s.category,
			gridWidth: s.width,
			gridHeight: s.height,
			ingredients: s.ingredients.into_iter().map(|col| col.into_iter().map(up_list).collect()).collect(),
			result: non_empty(s.result),
			showNotification: s.showNotification,
		}),
		O::minecraft_crafting_shapeless(s) => N::minecraft_crafting_shapeless(new::play_toClient_packet__declare_recipes__items__data__minecraft_crafting_shapeless::Ty {
			group: s.group,
			category: s.category,
			ingredients: s.ingredients.into_iter().map(up_list).collect(),
			result: non_empty(s.result),
		}),
		O::minecraft_smelting(s) => N::minecraft_smelting(smelting_up(s)),
		O::minecraft_blasting(s) => N::minecraft_blasting(smelting_up(s)),
		O::minecraft_smoking(s) => N::minecraft_smoking(smelting_up(s)),
		O::minecraft_campfire_cooking(s) => N::minecraft_campfire_cooking(smelting_up(s)),
		O::minecraft_stonecutting(s) => N::minecraft_stonecutting(new::play_toClient_packet__declare_recipes__items__data__minecraft_stonecutting::Ty {
			group: s.group,
			ingredient: up_list(s.ingredient),
			result: non_empty(s.result),
		}),
		O::minecraft_smithing_transform(s) => N::minecraft_smithing_transform(new::play_toClient_packet__declare_recipes__items__data__minecraft_smithing_transform::Ty {
			template: up_list(s.template),
			base: up_list(s.base),
			addition: up_list(s.addition),
			result: non_empty(s.result),
		}),
		O::minecraft_smithing_trim(s) => N::minecraft_smithing_trim(new::play_toClient_packet__declare_recipes__items__data__minecraft_smithing_trim::Ty {
			template: up_list(s.template),
			base: up_list(s.base),
			addition: up_list(s.addition),
		}),
		O::default(_) => return Ok(None),
		simple => {
			let mut b = Vec::new();
			varint(&mut b, id as i32);
			b.extend(encode(&simple, old::play_toClient_packet__declare_recipes__items__data::serialize)?);
			decode(&b, |i| new::play_toClient_packet__declare_recipes__items__data::deserialize(i))?
		}
	};
	Ok(Some(
		new::play_toClient_packet__declare_recipes__items::Ty {
			name: r.recipeId,
			data,
		},
	))
}

fn recipe_down(
	r: new::play_toClient_packet__declare_recipes__items::Ty,
) -> Result<Option<old::play_toClient_packet__declare_recipes__items::Ty>> {
	use new::play_toClient_packet__declare_recipes__items__data::Ty as N;
	use old::play_toClient_packet__declare_recipes__items__data::Ty as O;
	let bytes = encode(
		&r.data,
		new::play_toClient_packet__declare_recipes__items__data::serialize,
	)?;
	let (id, payload) = read_varint(&bytes)?;
	let Some(kind) = RECIPE_TYPES
		.get(id as usize)
		.map(|t| format!("minecraft:{t}"))
	else {
		return Ok(None);
	};
	let data = match r.data {
		N::minecraft_crafting_shaped(s) => O::minecraft_crafting_shaped(old::play_toClient_packet__declare_recipes__items__data__minecraft_crafting_shaped::Ty {
			group: s.group,
			category: s.category,
			width: s.gridWidth,
			height: s.gridHeight,
			ingredients: s.ingredients.into_iter().map(|col| col.into_iter().map(down_list).collect()).collect(),
			result: to_legacy(s.result),
			showNotification: s.showNotification,
		}),
		N::minecraft_crafting_shapeless(s) => O::minecraft_crafting_shapeless(old::play_toClient_packet__declare_recipes__items__data__minecraft_crafting_shapeless::Ty {
			group: s.group,
			category: s.category,
			ingredients: s.ingredients.into_iter().map(down_list).collect(),
			result: to_legacy(s.result),
		}),
		N::minecraft_smelting(s) => O::minecraft_smelting(smelting_down(s)),
		N::minecraft_blasting(s) => O::minecraft_blasting(smelting_down(s)),
		N::minecraft_smoking(s) => O::minecraft_smoking(smelting_down(s)),
		N::minecraft_campfire_cooking(s) => O::minecraft_campfire_cooking(smelting_down(s)),
		N::minecraft_stonecutting(s) => O::minecraft_stonecutting(old::play_toClient_packet__declare_recipes__items__data__minecraft_stonecutting::Ty {
			group: s.group,
			ingredient: down_list(s.ingredient),
			result: to_legacy(s.result),
		}),
		N::minecraft_smithing_transform(s) => O::minecraft_smithing_transform(old::play_toClient_packet__declare_recipes__items__data__minecraft_smithing_transform::Ty {
			template: down_list(s.template),
			base: down_list(s.base),
			addition: down_list(s.addition),
			result: to_legacy(s.result),
		}),
		N::minecraft_smithing_trim(s) => O::minecraft_smithing_trim(old::play_toClient_packet__declare_recipes__items__data__minecraft_smithing_trim::Ty {
			template: down_list(s.template),
			base: down_list(s.base),
			addition: down_list(s.addition),
		}),
		N::default(_) => return Ok(None),
		_ => decode(payload, |i| old::play_toClient_packet__declare_recipes__items__data::deserialize(i, old::play_toClient_packet__declare_recipes__items__data::Args { r#type: &kind }))?,
	};
	Ok(Some(
		old::play_toClient_packet__declare_recipes__items::Ty {
			r#type: kind,
			recipeId: r.name,
			data,
		},
	))
}

pub fn trails_declare_recipes_1_20_3_to_1_20_5(
	p: Vec<old::play_toClient_packet__declare_recipes__items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	let recipes = p
		.into_iter()
		.map(recipe_up)
		.filter_map(Result::transpose)
		.collect::<Result<_>>()?;
	Ok(vec![Client::declare_recipes(recipes)])
}

pub fn trails_declare_recipes_1_20_5_to_1_20_3(
	p: Vec<new::play_toClient_packet__declare_recipes__items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let recipes = p
		.into_iter()
		.map(recipe_down)
		.filter_map(Result::transpose)
		.collect::<Result<_>>()?;
	Ok(vec![ClientOld::declare_recipes(recipes)])
}

pub fn trails_no_factor_1_20_5_to_1_20_3(
	_src: &new::play_toClient_packet_entity_effect::Ty,
	_cx: &mut Session,
) -> Result<Option<Tag>> {
	Ok(None)
}

const HORSES: &[&str] = &[
	"horse",
	"donkey",
	"mule",
	"zombie_horse",
	"skeleton_horse",
	"llama",
	"trader_llama",
	"camel",
];

arm!(
	trails_entity_equipment_1_20_3_to_1_20_5,
	Client,
	entity_equipment,
	old::play_toClient_packet_entity_equipment::Ty,
	|p, cx| {
		let horse =
			common::entity_kind(cx, p.entityId).is_some_and(|k| HORSES.contains(&k.as_str()));
		new::play_toClient_packet_entity_equipment::Ty {
			entityId: p.entityId,
			equipments: p
				.equipments
				.into_iter()
				.map(
					|e| new::play_toClient_packet_entity_equipment__equipments__items::Ty {
						slot: if horse && e.slot == 4 { 6 } else { e.slot },
						item: to_components(e.item),
					},
				)
				.collect(),
		}
	}
);

pub fn trails_chat_command_1_20_5_to_1_20_3(
	command: String,
	_cx: &mut Session,
) -> Result<Vec<old::play_toServer_packet::Ty>> {
	let now = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map_or(0, |d| d.as_millis() as i64);
	Ok(vec![old::play_toServer_packet::Ty::chat_command(
		old::play_toServer_packet_chat_command::Ty {
			command,
			timestamp: now,
			salt: 0,
			argumentSignatures: vec![],
			messageCount: 0,
			acknowledged: vec![0; 3],
		},
	)])
}

pub fn trails_chat_command_signed_1_20_5_to_1_20_3(
	p: new::play_toServer_packet_chat_command_signed::Ty,
	_cx: &mut Session,
) -> Result<Vec<old::play_toServer_packet::Ty>> {
	Ok(vec![old::play_toServer_packet::Ty::chat_command(
		old::play_toServer_packet_chat_command::Ty {
			command: p.command,
			timestamp: p.timestamp,
			salt: p.salt,
			argumentSignatures: p
				.argumentSignatures
				.into_iter()
				.map(
					|s| old::play_toServer_packet_chat_command__argumentSignatures__items::Ty {
						argumentName: s.argumentName,
						signature: s.signature,
					},
				)
				.collect(),
			messageCount: p.messageCount,
			acknowledged: p.acknowledged,
		},
	)])
}

pub fn trails_chat_command_1_20_3_to_1_20_5(
	p: old::play_toServer_packet_chat_command::Ty,
	_cx: &mut Session,
) -> Result<Vec<new::play_toServer_packet::Ty>> {
	Ok(vec![new::play_toServer_packet::Ty::chat_command_signed(
		new::play_toServer_packet_chat_command_signed::Ty {
			command: p.command,
			timestamp: p.timestamp,
			salt: p.salt,
			argumentSignatures: p
				.argumentSignatures
				.into_iter()
				.map(|s| {
					new::play_toServer_packet_chat_command_signed__argumentSignatures__items::Ty {
						argumentName: s.argumentName,
						signature: s.signature,
					}
				})
				.collect(),
			messageCount: p.messageCount,
			acknowledged: p.acknowledged,
		},
	)])
}

const LLAMAS: &[&str] = &["llama", "trader_llama"];

fn swag(entity: i32, item: &Slot) -> Result<ClientOld> {
	let color = match item {
		Slot::default(_, d) => registry::names(NEW, "items")
			.and_then(|n| n.name(d.itemId as u32))
			.and_then(|n| n.strip_suffix("_carpet"))
			.and_then(|c| items::DYES.iter().position(|d| *d == c))
			.map_or(-1, |i| i as i32),
		_ => -1,
	};
	let key = common::meta_index(OLD, "llama", "swag").ok_or(Error::Dropped)?;
	Ok(ClientOld::entity_metadata(
		old::play_toClient_packet_entity_metadata::Ty {
			entityId: entity,
			metadata: vec![decode(&entry_bytes(key, 1, &varint_bytes(color)), |i| {
				old::entityMetadata__items::deserialize(i)
			})?],
		},
	))
}

pub fn trails_entity_equipment_1_20_5_to_1_20_3(
	p: new::play_toClient_packet_entity_equipment::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let kind = common::entity_kind(cx, p.entityId).unwrap_or_default();
	let mut out = Vec::new();
	let mut equipments = Vec::new();
	for e in p.equipments {
		if e.slot == 6 && LLAMAS.contains(&kind.as_str()) {
			out.push(swag(p.entityId, &e.item)?);
			continue;
		}
		equipments.push(
			old::play_toClient_packet_entity_equipment__equipments__items::Ty {
				slot: if e.slot == 6 { 4 } else { e.slot },
				item: to_legacy(e.item),
			},
		);
	}
	if !equipments.is_empty() {
		out.insert(
			0,
			ClientOld::entity_equipment(old::play_toClient_packet_entity_equipment::Ty {
				entityId: p.entityId,
				equipments,
			}),
		);
	}
	Ok(out)
}

arm!(
	trails_open_horse_window_1_20_5_to_1_20_3,
	ClientOld,
	open_horse_window,
	new::play_toClient_packet_open_horse_window::Ty,
	|p, _cx| {
		old::play_toClient_packet_open_horse_window::Ty {
			windowId: p.windowId,
			nbSlots: p.nbSlots + 1,
			entityId: p.entityId,
		}
	}
);

pub fn trails_select_known_packs_1_20_5_to_1_20_3(
	_packs: Vec<new::configuration_toClient_packet__select_known_packs__items::Ty>,
	cx: &mut Session,
) -> Result<Vec<ConfigOld>> {
	common::reply(
		cx,
		Root::ConfigurationToServer,
		NEW,
		&new::configuration_toServer_packet::Ty::select_known_packs(vec![]),
		new::configuration_toServer_packet::serialize,
	)?;
	Ok(vec![])
}

fn cookie(key: String) -> new::packet_common_cookie_response::Ty {
	new::packet_common_cookie_response::Ty { key, value: None }
}

pub fn trails_login_cookie_request_1_20_5_to_1_20_3(
	key: String,
	cx: &mut Session,
) -> Result<Vec<old::login_toClient_packet::Ty>> {
	common::reply(
		cx,
		Root::LoginToServer,
		NEW,
		&new::login_toServer_packet::Ty::cookie_response(cookie(key)),
		new::login_toServer_packet::serialize,
	)?;
	Ok(vec![])
}

pub fn trails_config_cookie_request_1_20_5_to_1_20_3(
	key: String,
	cx: &mut Session,
) -> Result<Vec<ConfigOld>> {
	common::reply(
		cx,
		Root::ConfigurationToServer,
		NEW,
		&new::configuration_toServer_packet::Ty::cookie_response(cookie(key)),
		new::configuration_toServer_packet::serialize,
	)?;
	Ok(vec![])
}

pub fn trails_cookie_request_1_20_5_to_1_20_3(
	key: String,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	common::reply(
		cx,
		Root::PlayToServer,
		NEW,
		&new::play_toServer_packet::Ty::cookie_response(cookie(key)),
		new::play_toServer_packet::serialize,
	)?;
	Ok(vec![])
}
