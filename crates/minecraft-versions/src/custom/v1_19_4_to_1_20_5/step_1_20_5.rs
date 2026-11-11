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

type Client = new::play_to_client_packet::Ty;
type ClientOld = old::play_to_client_packet::Ty;
type Config = new::configuration_to_client_packet::Ty;
type ConfigOld = old::configuration_to_client_packet::Ty;
type Slot = new::slot::Ty;

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
		new::slot::serialize,
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
			&to_legacy(decode(payload, |i| new::slot::deserialize(i))?),
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

fn new_particle(id: i32, color: i32, payload: &[u8]) -> Result<new::particle::Ty> {
	let (target, new_id) = particle_target(OLD, NEW, id)?;
	let mut out = Vec::new();
	varint(&mut out, new_id);
	out.extend(up_payload(target, color, payload)?);
	decode(&out, |i| new::particle::deserialize(i))
}

fn old_particle(p: &new::particle::Ty) -> Result<(i32, &'static str, Vec<u8>, Vec<u8>)> {
	let bytes = encode(p, new::particle::serialize)?;
	let (id, payload) = read_varint(&bytes)?;
	let (target, old_id) = particle_target(NEW, OLD, id)?;
	Ok((
		old_id,
		target,
		down_payload(target, payload)?,
		payload.to_vec(),
	))
}

fn particle_parts(p: &old::particle::Ty) -> Result<(i32, Vec<u8>)> {
	let bytes = encode(p, old::particle::serialize)?;
	let (id, payload) = read_varint(&bytes)?;
	Ok((id, payload.to_vec()))
}

pub fn trails_particle_1_20_3_to_1_20_5(
	v: old::particle::Ty,
	_cx: &mut Session,
) -> Result<new::particle::Ty> {
	let (id, payload) = particle_parts(&v)?;
	new_particle(id, 0, &payload)
}

pub fn trails_particle_1_20_5_to_1_20_3(
	v: new::particle::Ty,
	_cx: &mut Session,
) -> Result<old::particle::Ty> {
	let (id, _, payload, _) = old_particle(&v)?;
	let mut b = Vec::new();
	varint(&mut b, id);
	b.extend(payload);
	decode(&b, |i| old::particle::deserialize(i))
}

pub(super) fn sound_up(s: old::item_sound_holder::Ty) -> Result<new::item_sound_holder::Ty> {
	use new::item_sound_holder::Ty as H;
	Ok(match s {
		old::item_sound_holder::Ty::Id(id) => match common::sound(OLD, NEW, id)? {
			common::Sound::Id(id) => H::Id(id),
			common::Sound::Named(sound_name) => H::Inline(new::item_sound_event::Ty {
				sound_name,
				fixed_range: None,
			}),
		},
		old::item_sound_holder::Ty::Inline(e) => H::Inline(new::item_sound_event::Ty {
			sound_name: e.sound_name,
			fixed_range: e.fixed_range,
		}),
	})
}

pub(super) fn sound_down(s: new::item_sound_holder::Ty) -> Result<old::item_sound_holder::Ty> {
	use old::item_sound_holder::Ty as H;
	Ok(match s {
		new::item_sound_holder::Ty::Id(id) => match common::sound(NEW, OLD, id)? {
			common::Sound::Id(id) => H::Id(id),
			common::Sound::Named(sound_name) => H::Inline(old::item_sound_event::Ty {
				sound_name,
				fixed_range: None,
			}),
		},
		new::item_sound_holder::Ty::Inline(e) => H::Inline(old::item_sound_event::Ty {
			sound_name: e.sound_name,
			fixed_range: e.fixed_range,
		}),
	})
}

pub fn trails_sound_1_20_3_to_1_20_5(
	v: old::item_sound_holder::Ty,
	_cx: &mut Session,
) -> Result<new::item_sound_holder::Ty> {
	sound_up(v)
}

pub fn trails_sound_1_20_5_to_1_20_3(
	v: new::item_sound_holder::Ty,
	_cx: &mut Session,
) -> Result<old::item_sound_holder::Ty> {
	sound_down(v)
}

pub fn trails_wolf_variant_1_20_5_to_1_21_1(
	id: i32,
	_cx: &mut Session,
) -> Result<crate::gen::convert::v1_21_1::entity_metadata_entry_value::Ty> {
	use crate::gen::convert::v1_21_1 as next;
	Ok(next::entity_metadata_entry_value::Ty::WolfVariant(
		next::entity_metadata_entry_value_wolf_variant::Ty::Id(id),
	))
}

pub fn trails_wolf_variant_1_21_1_to_1_20_5(
	v: crate::gen::convert::v1_21_1::entity_metadata_entry_value_wolf_variant::Ty,
	_cx: &mut Session,
) -> Result<new::entity_metadata_entry_value::Ty> {
	match v {
		crate::gen::convert::v1_21_1::entity_metadata_entry_value_wolf_variant::Ty::Id(id) => {
			Ok(new::entity_metadata_entry_value::Ty::WolfVariant(id))
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
		.map_or(Slot::V0, |id| {
			Slot::Default(
				1,
				new::slot_default::Ty {
					item_id: id as i32,
					added_component_count: 0,
					removed_component_count: 0,
					components: vec![],
					remove_components: vec![],
				},
			)
		});
	Client::EntityEquipment(new::play_to_client_packet_entity_equipment::Ty {
		entity_id: entity,
		equipments: vec![
			new::play_to_client_packet_entity_equipment_equipments_items::Ty { slot: 6, item },
		],
	})
}

fn meta_entry(key: u8, ty: i32, payload: &[u8]) -> Result<new::entity_metadata_entry::Ty> {
	decode(&entry_bytes(key, ty, payload), |i| {
		new::entity_metadata_entry::deserialize(i)
	})
}

fn varint_bytes(v: i32) -> Vec<u8> {
	let mut b = Vec::new();
	varint(&mut b, v);
	b
}

pub fn trails_entity_metadata_1_20_3_to_1_20_5(
	p: old::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	use old::entity_metadata_item::Ty as V;
	let kind = common::entity_kind(cx, p.entity_id).unwrap_or_default();
	let target_keys = keys(NEW, &kind).unwrap_or_default();
	let source_keys = keys(OLD, &kind).unwrap_or_default();
	let name_of = |key: u8| source_keys.get(key as usize).map(String::as_str);
	let key_of = |name: &str| target_keys.iter().position(|k| k == name).map(|i| i as u8);
	let cloud_color = p
		.metadata
		.iter()
		.find_map(|e| match (name_of(e.key), &e.value) {
			(Some("color"), V::Int(c)) if kind == "area_effect_cloud" => Some(*c),
			_ => None,
		});
	let mut extra = Vec::new();
	let mut out = Vec::with_capacity(p.metadata.len());
	let mut has_particle = false;
	for e in p.metadata {
		let source_type = read_varint(&encode(
			&e.r#type,
			old::entity_metadata_items_type::serialize,
		)?)?
		.0;
		let name = name_of(e.key);
		match (kind.as_str(), name, &e.value) {
			(_, Some("effect_color"), V::Int(color)) if key_of("effect_particles").is_some() => {
				out.push(meta_entry(
					key_of("effect_particles").expect("checked"),
					META_PARTICLES,
					&effect_particles(*color)?,
				)?);
				continue;
			}
			("llama" | "trader_llama", Some("swag"), V::Int(color)) => {
				extra.push(carpet(p.entity_id, *color));
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
			None => match meta_key(cx, OLD, NEW, p.entity_id, e.key, &[]) {
				Some(k) => k,
				None => continue,
			},
		};
		let payload = match e.value {
			V::ItemStack(s) => {
				let slot = to_components(s);
				if matches!(slot, Slot::V0) && matches!(name, Some("item_stack")) {
					continue;
				}
				encode(&slot, new::slot::serialize)?
			}
			V::BlockState(s) => varint_bytes(block_state(OLD, NEW, s)),
			V::OptionalBlockState(s) => varint_bytes(optional_state(OLD, NEW, s)),
			V::Int(color) if kind == "arrow" && name == Some("effect_color") => {
				varint_bytes(if color == -1 {
					-1
				} else {
					color | (0xff << 24)
				})
			}
			V::Particle(particle) => {
				let (id, payload) = particle_parts(&particle)?;
				let Ok(particle) = new_particle(id, cloud_color.unwrap_or(0), &payload) else {
					continue;
				};
				has_particle = true;
				encode(&particle, new::particle::serialize)?
			}
			v => encode(&v, old::entity_metadata_item::serialize)?,
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
				&encode(&particle, new::particle::serialize)?,
			)?);
		}
	}
	let mut packets = vec![Client::EntityMetadata(
		new::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
			metadata: out,
		},
	)];
	packets.extend(extra);
	Ok(packets)
}

fn effect_color(particle: &new::particle::Ty) -> Option<i32> {
	let b = encode(particle, new::particle::serialize).ok()?;
	let (id, rest) = read_varint(&b).ok()?;
	(particle_name(NEW, id) == Some("entity_effect") && rest.len() == 4)
		.then(|| i32::from_be_bytes([0, rest[1], rest[2], rest[3]]))
}

pub fn trails_entity_metadata_1_20_5_to_1_20_3(
	p: new::play_to_client_packet_entity_metadata::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	use new::entity_metadata_entry_value::Ty as V;
	let kind = common::entity_kind(cx, p.entity_id).unwrap_or_default();
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
		let bytes = encode(&e.value, new::entity_metadata_entry_value::serialize)?;
		let (source_type, payload) = read_varint(&bytes)?;
		let (ty, payload) = match e.value {
			V::Particles(list) => (
				1,
				varint_bytes(list.iter().find_map(effect_color).unwrap_or(0)),
			),
			V::Int(color) if kind == "arrow" && target == "effect_color" => (
				1,
				varint_bytes(if color == -1 { -1 } else { color & 0xff_ffff }),
			),
			V::ItemStack(s) => (7, encode(&to_legacy(s), old::slot::serialize)?),
			V::BlockState(s) => (14, varint_bytes(block_state(NEW, OLD, s))),
			V::OptionalBlockState(s) => (15, varint_bytes(optional_state(NEW, OLD, s))),
			V::Particle(particle) => {
				let Ok((id, _, payload, _)) = old_particle(&particle) else {
					continue;
				};
				if kind == "area_effect_cloud" {
					if let (Some(color), Some(color_key)) =
						(effect_color(&particle), key_of("color"))
					{
						out.push(decode(
							&entry_bytes(color_key, 1, &varint_bytes(color)),
							|i| old::entity_metadata_items::deserialize(i),
						)?);
					}
				}
				let mut b = varint_bytes(id);
				b.extend(payload);
				(17, b)
			}
			V::WolfVariant(_) | V::ArmadilloState(_) | V::Particles(_) => continue,
			_ => match meta_type_down(source_type) {
				Some(t) => (t, payload.to_vec()),
				None => continue,
			},
		};
		out.push(decode(&entry_bytes(key, ty, &payload), |i| {
			old::entity_metadata_items::deserialize(i)
		})?);
	}
	Ok(vec![ClientOld::EntityMetadata(
		old::play_to_client_packet_entity_metadata::Ty {
			entity_id: p.entity_id,
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
	let Payload::Compound(c) = element else {
		return;
	};
	match registry {
		"dimension_type" => {
			let Some(light) = c.compound_mut("monster_spawn_light_level") else {
				return;
			};
			if up {
				if let Some(Payload::Compound(value)) = light.take("value") {
					light.extend(value);
				}
			} else if let (Some(min), Some(max)) =
				(light.take("min_inclusive"), light.take("max_inclusive"))
			{
				light.put(
					"value",
					Payload::Compound(vec![
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
	Config::RegistryData(new::configuration_to_client_packet_registry_data::Ty {
		id: id.to_owned(),
		entries: entries
			.into_iter()
			.map(|(key, value)| {
				new::configuration_to_client_packet_registry_data_entries_items::Ty {
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
	let Payload::Compound(root) = codec.value else {
		return unconvertible("registry data is not a compound");
	};
	let vanilla = vanilla_registries()?;
	let mut out = vec![Config::SelectKnownPacks(vec![])];
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
				let Payload::Compound(mut c) = e else {
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
	p: new::configuration_to_client_packet_registry_data::Ty,
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
			Some(Payload::Compound(vec![
				Entry::new("name", Payload::String(e.key)),
				Entry::new("id", Payload::Int(i as i32)),
				Entry::new("element", value),
			]))
		})
		.collect();
	let registry = Payload::Compound(vec![
		Entry::new("type", Payload::String(p.id.clone())),
		Entry::new("value", List::of(elements)),
	]);
	common::state(cx).pending.push(Entry::new(&p.id, registry));
	Ok(vec![])
}

pub fn trails_finish_configuration_1_20_5_to_1_20_3(
	p: new::configuration_to_client_packet_finish_configuration::Ty,
	cx: &mut Session,
) -> Result<Vec<ConfigOld>> {
	let pending = std::mem::take(&mut common::state(cx).pending);
	let mut out = Vec::new();
	if !pending.is_empty() {
		out.push(ConfigOld::RegistryData(Tag::new(Payload::Compound(
			pending,
		))));
	}
	out.push(ConfigOld::FinishConfiguration(p));
	Ok(out)
}

fn config_tags(
	from: Version,
	to: Version,
	mut tags: Vec<old::configuration_to_client_packet_tags_items::Ty>,
) -> Vec<old::configuration_to_client_packet_tags_items::Ty> {
	for registry in &mut tags {
		for t in &mut registry.tags {
			t.entries = tag_entries(&registry.tag_type, from, to, std::mem::take(&mut t.entries));
		}
	}
	tags
}

arm!(
	trails_config_tags_1_20_3_to_1_20_5,
	Config,
	Tags,
	Vec<old::configuration_to_client_packet_tags_items::Ty>,
	|p, _cx| config_tags(OLD, NEW, p)
);

arm!(
	trails_config_tags_1_20_5_to_1_20_3,
	ConfigOld,
	Tags,
	Vec<new::configuration_to_client_packet_tags_items::Ty>,
	|p, _cx| config_tags(NEW, OLD, p)
);

arm!(
	trails_tags_1_20_3_to_1_20_5,
	Client,
	Tags,
	Vec<old::play_to_client_packet_tags_items::Ty>,
	|p, _cx| super::step_1_20::remap_tags(OLD, NEW, p)
);

arm!(
	trails_tags_1_20_5_to_1_20_3,
	ClientOld,
	Tags,
	Vec<new::play_to_client_packet_tags_items::Ty>,
	|p, _cx| super::step_1_20::remap_tags(NEW, OLD, p)
);

fn gamemode(g: u8) -> Result<new::play_to_client_spawn_info_gamemode::Ty> {
	decode(&[g], |i| {
		new::play_to_client_spawn_info_gamemode::deserialize(i)
	})
}

fn gamemode_byte(g: &new::play_to_client_spawn_info_gamemode::Ty) -> Result<u8> {
	Ok(
		encode(g, new::play_to_client_spawn_info_gamemode::serialize)?
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
		new::play_to_client_packet_entity_update_attributes_properties_items::Ty,
	> {
		let id = items::ATTRIBUTES
			.iter()
			.position(|a| *a == name)
			.ok_or(Error::Dropped)? as u8;
		Ok(new::play_to_client_packet_entity_update_attributes_properties_items::Ty {
			key: decode(&[id], |i| new::play_to_client_packet_entity_update_attributes_properties_items_key::deserialize(i))?,
			value: base,
			modifiers: modifier
				.map(|(uuid, amount)| new::play_to_client_packet_entity_update_attributes_properties_items_modifiers_items::Ty { uuid, amount, operation: 0 })
				.into_iter()
				.collect(),
		})
	};
	Ok(Client::EntityUpdateAttributes(
		new::play_to_client_packet_entity_update_attributes::Ty {
			entity_id: entity,
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
	p: old::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	let s = common::state(cx);
	s.player = Some(p.entity_id);
	let secure = s.secure_chat.unwrap_or(false);
	let dimension = dimension_id(cx, &p.world_type);
	let creative = p.game_mode & 3 == 1;
	Ok(vec![
		Client::Login(new::play_to_client_packet_login::Ty {
			entity_id: p.entity_id,
			is_hardcore: p.is_hardcore,
			world_names: p.world_names,
			max_players: p.max_players,
			view_distance: p.view_distance,
			simulation_distance: p.simulation_distance,
			reduced_debug_info: p.reduced_debug_info,
			enable_respawn_screen: p.enable_respawn_screen,
			do_limited_crafting: p.do_limited_crafting,
			world_state: new::play_to_client_spawn_info::Ty {
				dimension,
				name: p.world_name,
				hashed_seed: p.hashed_seed,
				gamemode: gamemode(p.game_mode)?,
				previous_gamemode: p.previous_game_mode as u8,
				is_debug: p.is_debug,
				is_flat: p.is_flat,
				death: p
					.death
					.map(|d| new::play_to_client_spawn_info_death_items::Ty {
						dimension_name: d.dimension_name,
						location: d.location,
					}),
				portal_cooldown: p.portal_cooldown,
			},
			enforces_secure_chat: secure,
		}),
		range_attributes(p.entity_id, creative)?,
	])
}

pub fn trails_login_1_20_5_to_1_20_3(
	p: new::play_to_client_packet_login::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let w = p.world_state;
	common::state(cx).player = Some(p.entity_id);
	Ok(vec![ClientOld::Login(
		old::play_to_client_packet_login::Ty {
			entity_id: p.entity_id,
			is_hardcore: p.is_hardcore,
			world_names: p.world_names,
			max_players: p.max_players,
			view_distance: p.view_distance,
			simulation_distance: p.simulation_distance,
			reduced_debug_info: p.reduced_debug_info,
			enable_respawn_screen: p.enable_respawn_screen,
			do_limited_crafting: p.do_limited_crafting,
			world_type: dimension_name(cx, w.dimension),
			world_name: w.name,
			hashed_seed: w.hashed_seed,
			game_mode: gamemode_byte(&w.gamemode)?,
			previous_game_mode: w.previous_gamemode as i8,
			is_debug: w.is_debug,
			is_flat: w.is_flat,
			death: w
				.death
				.map(|d| old::play_to_client_packet_login_death_items::Ty {
					dimension_name: d.dimension_name,
					location: d.location,
				}),
			portal_cooldown: w.portal_cooldown,
		},
	)])
}

pub fn trails_respawn_1_20_3_to_1_20_5(
	p: old::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	let dimension = dimension_id(cx, &p.dimension);
	let player = common::state(cx).player;
	let mut out = vec![Client::Respawn(new::play_to_client_packet_respawn::Ty {
		world_state: new::play_to_client_spawn_info::Ty {
			dimension,
			name: p.world_name,
			hashed_seed: p.hashed_seed,
			gamemode: gamemode(p.gamemode as u8)?,
			previous_gamemode: p.previous_gamemode,
			is_debug: p.is_debug,
			is_flat: p.is_flat,
			death: p
				.death
				.map(|d| new::play_to_client_spawn_info_death_items::Ty {
					dimension_name: d.dimension_name,
					location: d.location,
				}),
			portal_cooldown: p.portal_cooldown,
		},
		copy_metadata: p.copy_metadata as u8,
	})];
	if let Some(player) = player {
		out.push(range_attributes(player, p.gamemode & 3 == 1)?);
	}
	Ok(out)
}

pub fn trails_respawn_1_20_5_to_1_20_3(
	p: new::play_to_client_packet_respawn::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let w = p.world_state;
	Ok(vec![ClientOld::Respawn(
		old::play_to_client_packet_respawn::Ty {
			dimension: dimension_name(cx, w.dimension),
			world_name: w.name,
			hashed_seed: w.hashed_seed,
			gamemode: gamemode_byte(&w.gamemode)? as i8,
			previous_gamemode: w.previous_gamemode,
			is_debug: w.is_debug,
			is_flat: w.is_flat,
			death: w
				.death
				.map(|d| old::play_to_client_packet_respawn_death_items::Ty {
					dimension_name: d.dimension_name,
					location: d.location,
				}),
			portal_cooldown: w.portal_cooldown,
			copy_metadata: p.copy_metadata != 0,
		},
	)])
}

pub fn trails_game_state_change_1_20_3_to_1_20_5(
	p: old::play_to_client_packet_game_state_change::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	let mode = matches!(
		p.reason,
		old::play_to_client_packet_game_state_change_reason::Ty::ChangeGameMode
	);
	let creative = (p.game_mode + 0.5).floor() as i32 == 1;
	let player = common::state(cx).player;
	let mut out = vec![Client::GameStateChange(p)];
	if let (true, Some(player)) = (mode, player) {
		out.push(range_attributes(player, creative)?);
	}
	Ok(out)
}

pub fn trails_server_data_1_20_3_to_1_20_5(
	p: old::play_to_client_packet_server_data::Ty,
	cx: &mut Session,
) -> Result<Vec<Client>> {
	common::state(cx).secure_chat = Some(p.enforces_secure_chat);
	Ok(vec![Client::ServerData(
		new::play_to_client_packet_server_data::Ty {
			motd: p.motd,
			icon_bytes: p.icon_bytes,
		},
	)])
}

pub fn trails_secure_chat_1_20_5_to_1_20_3(
	_src: &new::play_to_client_packet_server_data::Ty,
	cx: &mut Session,
) -> Result<bool> {
	Ok(common::state(cx).secure_chat.unwrap_or(false))
}

pub fn trails_position_flags_1_20_3_to_1_20_5(
	v: i8,
	_cx: &mut Session,
) -> Result<new::play_to_client_position_update_relatives::Ty> {
	decode(&[v as u8 & 0x1f], |i| {
		new::play_to_client_position_update_relatives::deserialize(i)
	})
}

pub fn trails_position_flags_1_20_5_to_1_20_3(
	v: new::play_to_client_position_update_relatives::Ty,
	_cx: &mut Session,
) -> Result<i8> {
	Ok(
		encode(&v, new::play_to_client_position_update_relatives::serialize)?
			.first()
			.copied()
			.unwrap_or(0) as i8,
	)
}

pub fn trails_attribute_key_1_20_3_to_1_20_5(
	src: &old::play_to_client_packet_entity_update_attributes_properties_items::Ty,
	_cx: &mut Session,
) -> Result<new::play_to_client_packet_entity_update_attributes_properties_items_key::Ty> {
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
		new::play_to_client_packet_entity_update_attributes_properties_items_key::deserialize(i)
	})
}

pub fn trails_attribute_name_1_20_5_to_1_20_3(
	src: &new::play_to_client_packet_entity_update_attributes_properties_items::Ty,
	_cx: &mut Session,
) -> Result<String> {
	let id = read_varint(&encode(
		&src.key,
		new::play_to_client_packet_entity_update_attributes_properties_items_key::serialize,
	)?)?
	.0;
	let name = items::ATTRIBUTES.get(id as usize).ok_or(Error::Dropped)?;
	Ok(format!("minecraft:{name}"))
}

pub fn trails_entity_sound_1_20_3_to_1_20_5(
	src: &old::play_to_client_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<new::item_sound_holder::Ty> {
	use old::play_to_client_packet_entity_sound_effect_sound_event::Ty as E;
	sound_up(match &src.sound_event {
		E::V0(e) => old::item_sound_holder::Ty::Inline(old::item_sound_event::Ty {
			sound_name: e.resource.clone(),
			fixed_range: e.range,
		}),
		E::Default(tag) => old::item_sound_holder::Ty::Id(tag - 1),
	})
}

pub fn trails_entity_sound_1_20_5_to_1_20_3(
	src: &new::play_to_client_packet_entity_sound_effect::Ty,
	_cx: &mut Session,
) -> Result<old::play_to_client_packet_entity_sound_effect_sound_event::Ty> {
	use old::play_to_client_packet_entity_sound_effect_sound_event::Ty as E;
	Ok(match sound_down(src.sound.clone())? {
		old::item_sound_holder::Ty::Id(id) => E::Default(id + 1),
		old::item_sound_holder::Ty::Inline(e) => E::V0(
			old::play_to_client_packet_entity_sound_effect_sound_event_v0::Ty {
				resource: e.sound_name,
				range: e.fixed_range,
			},
		),
	})
}

fn cost(s: Slot) -> Option<new::play_to_client_packet_trade_list_trades_items_input_item1::Ty> {
	let Slot::Default(count, d) = s else {
		return None;
	};
	let components: Vec<_> = d
		.components
		.into_iter()
		.filter(|c| !matches!(c, new::slot_component::Ty::CustomData(_)))
		.collect();
	Some(
		new::play_to_client_packet_trade_list_trades_items_input_item1::Ty {
			item_id: d.item_id,
			item_count: count as i32,
			added_component_count: components.len() as i32,
			components,
		},
	)
}

pub fn trails_trade_input_1_20_3_to_1_20_5(
	v: old::slot::Ty,
	_cx: &mut Session,
) -> Result<new::play_to_client_packet_trade_list_trades_items_input_item1::Ty> {
	Ok(cost(non_empty(v)).expect("non_empty yields an item"))
}

pub fn trails_trade_input_2_1_20_3_to_1_20_5(
	v: old::slot::Ty,
	_cx: &mut Session,
) -> Result<Option<new::play_to_client_packet_trade_list_trades_items_input_item2_items::Ty>> {
	Ok(cost(to_components(v)).map(|c| {
		new::play_to_client_packet_trade_list_trades_items_input_item2_items::Ty {
			item_id: c.item_id,
			item_count: c.item_count,
			added_component_count: c.added_component_count,
			components: c.components,
		}
	}))
}

fn from_cost(item_id: i32, count: i32, components: Vec<new::slot_component::Ty>) -> old::slot::Ty {
	to_legacy(Slot::Default(
		count as i8,
		new::slot_default::Ty {
			item_id,
			added_component_count: components.len() as i32,
			removed_component_count: 0,
			components,
			remove_components: vec![],
		},
	))
}

pub fn trails_trade_input_1_20_5_to_1_20_3(
	v: new::play_to_client_packet_trade_list_trades_items_input_item1::Ty,
	_cx: &mut Session,
) -> Result<old::slot::Ty> {
	Ok(from_cost(v.item_id, v.item_count, v.components))
}

pub fn trails_trade_input_2_1_20_5_to_1_20_3(
	v: Option<new::play_to_client_packet_trade_list_trades_items_input_item2_items::Ty>,
	_cx: &mut Session,
) -> Result<old::slot::Ty> {
	Ok(v.map_or(old::slot::Ty::False, |v| {
		from_cost(v.item_id, v.item_count, v.components)
	}))
}

pub fn trails_world_particles_1_20_3_to_1_20_5(
	p: old::play_to_client_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	let payload = encode(&p.data, old::particle_data::serialize)?;
	let (target, _) = particle_target(OLD, NEW, p.particle_id)?;
	let color = match target {
		"entity_effect" if p.particle_data == 0.0 => 0,
		"entity_effect" if p.particles == 0 => {
			let c = |v: f32| ((v * 255.0).round() as i32).clamp(0, 255);
			(c(p.offset_x) << 16) | (c(p.offset_y) << 8) | c(p.offset_z)
		}
		_ => 0xffffff,
	};
	let particle = new_particle(p.particle_id, color, &payload)?;
	Ok(vec![Client::WorldParticles(
		new::play_to_client_packet_world_particles::Ty {
			long_distance: p.long_distance,
			x: p.x,
			y: p.y,
			z: p.z,
			offset_x: p.offset_x,
			offset_y: p.offset_y,
			offset_z: p.offset_z,
			velocity_offset: p.particle_data,
			amount: p.particles,
			particle,
		},
	)])
}

pub fn trails_world_particles_1_20_5_to_1_20_3(
	p: new::play_to_client_packet_world_particles::Ty,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let (id, name, payload, raw) = old_particle(&p.particle)?;
	let (mut offset, mut speed, mut count) = (
		[p.offset_x, p.offset_y, p.offset_z],
		p.velocity_offset,
		p.amount,
	);
	if name == "entity_effect" && raw.len() == 4 {
		offset = [raw[1], raw[2], raw[3]].map(|c| c as f32 / 255.0);
		speed = 1.0;
		count = 0;
	}
	Ok(vec![ClientOld::WorldParticles(
		old::play_to_client_packet_world_particles::Ty {
			particle_id: id,
			long_distance: p.long_distance,
			x: p.x,
			y: p.y,
			z: p.z,
			offset_x: offset[0],
			offset_y: offset[1],
			offset_z: offset[2],
			particle_data: speed,
			particles: count,
			data: decode(&payload, |i| {
				old::particle_data::deserialize(i, old::particle_data::Args { compare_to: &id })
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
			value: Payload::Compound(c),
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
			value: Payload::Compound(c),
			..
		}),
	) = (block_entity_name(kind), nbt.as_mut())
	{
		items::block_entity_1_20_3(name, c);
	}
	if matches!(nbt, Some(Tag { value: Payload::Compound(c), .. }) if c.is_empty()) {
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
		from,
		None,
	)
}

arm!(
	trails_map_chunk_1_20_3_to_1_20_5,
	Client,
	MapChunk,
	old::play_to_client_packet_map_chunk::Ty,
	|mut p, _cx| {
		p.chunk_data = chunk_data(OLD, NEW, &p.chunk_data)?;
		for b in &mut p.block_entities {
			block_entity_up(b.r#type, &mut b.nbt_data);
		}
		p
	}
);

arm!(
	trails_map_chunk_1_20_5_to_1_20_3,
	ClientOld,
	MapChunk,
	new::play_to_client_packet_map_chunk::Ty,
	|mut p, _cx| {
		p.chunk_data = chunk_data(NEW, OLD, &p.chunk_data)?;
		for b in &mut p.block_entities {
			block_entity_down(b.r#type, &mut b.nbt_data);
		}
		p
	}
);

arm!(
	trails_tile_entity_data_1_20_3_to_1_20_5,
	Client,
	TileEntityData,
	old::play_to_client_packet_tile_entity_data::Ty,
	|mut p, _cx| {
		block_entity_up(p.action, &mut p.nbt_data);
		if p.nbt_data.is_none() {
			p.nbt_data = Some(Tag::new(Payload::Compound(vec![])));
		}
		p
	}
);

arm!(
	trails_tile_entity_data_1_20_5_to_1_20_3,
	ClientOld,
	TileEntityData,
	new::play_to_client_packet_tile_entity_data::Ty,
	|mut p, _cx| {
		block_entity_down(p.action, &mut p.nbt_data);
		p
	}
);

arm!(
	trails_multi_block_change_1_20_3_to_1_20_5,
	Client,
	MultiBlockChange,
	old::play_to_client_packet_multi_block_change::Ty,
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
	MultiBlockChange,
	new::play_to_client_packet_multi_block_change::Ty,
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
	BlockChange,
	old::play_to_client_packet_block_change::Ty,
	|mut p, _cx| {
		p.r#type = block_state(OLD, NEW, p.r#type);
		p
	}
);

arm!(
	trails_block_change_1_20_5_to_1_20_3,
	ClientOld,
	BlockChange,
	new::play_to_client_packet_block_change::Ty,
	|mut p, _cx| {
		p.r#type = block_state(NEW, OLD, p.r#type);
		p
	}
);

arm!(
	trails_block_action_1_20_3_to_1_20_5,
	Client,
	BlockAction,
	old::play_to_client_packet_block_action::Ty,
	|mut p, _cx| {
		p.block_id = block(OLD, NEW, p.block_id).ok_or(Error::Dropped)?;
		p
	}
);

arm!(
	trails_block_action_1_20_5_to_1_20_3,
	ClientOld,
	BlockAction,
	new::play_to_client_packet_block_action::Ty,
	|mut p, _cx| {
		p.block_id = block(NEW, OLD, p.block_id).ok_or(Error::Dropped)?;
		p
	}
);

arm!(
	trails_world_event_1_20_3_to_1_20_5,
	Client,
	WorldEvent,
	old::play_to_client_packet_world_event::Ty,
	|mut p, _cx| {
		p.data = world_event_data(OLD, NEW, p.effect_id, p.data);
		p
	}
);

arm!(
	trails_world_event_1_20_5_to_1_20_3,
	ClientOld,
	WorldEvent,
	new::play_to_client_packet_world_event::Ty,
	|mut p, _cx| {
		p.data = world_event_data(NEW, OLD, p.effect_id, p.data);
		p
	}
);

macro_rules! spawn_entity {
	($name:ident, $root:ty, $src:ident, $from:expr, $to:expr) => {
		arm!(
			$name,
			$root,
			SpawnEntity,
			$src::play_to_client_packet_spawn_entity::Ty,
			|mut p, cx| {
				common::spawned(cx, $from, p.entity_id, p.r#type);
				let kind = crate::registry::names($from, "entities")
					.and_then(|n| n.name(p.r#type as u32))
					.unwrap_or_default();
				p.object_data = spawn_object_data($from, $to, kind, p.object_data);
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
	SetCooldown,
	old::play_to_client_packet_set_cooldown::Ty,
	|mut p, _cx| {
		p.item_id = item(OLD, NEW, p.item_id).ok_or(Error::Dropped)?;
		p
	}
);

arm!(
	trails_set_cooldown_1_20_5_to_1_20_3,
	ClientOld,
	SetCooldown,
	new::play_to_client_packet_set_cooldown::Ty,
	|mut p, _cx| {
		p.item_id = item(NEW, OLD, p.item_id).ok_or(Error::Dropped)?;
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
		.filter(|s| !matches!(s, Slot::V0))
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
		cook_time: s.cook_time,
	}
}

fn smelting_down(s: new::minecraft_smelting_format::Ty) -> old::minecraft_smelting_format::Ty {
	old::minecraft_smelting_format::Ty {
		group: s.group,
		category: s.category,
		ingredient: down_list(s.ingredient),
		result: to_legacy(s.result),
		experience: s.experience,
		cook_time: s.cook_time,
	}
}

fn recipe_up(
	r: old::play_to_client_packet_declare_recipes_items::Ty,
) -> Result<Option<new::play_to_client_packet_declare_recipes_items::Ty>> {
	use new::play_to_client_packet_declare_recipes_items_data::Ty as N;
	use old::play_to_client_packet_declare_recipes_items_data::Ty as O;
	let Some(id) = RECIPE_TYPES.iter().position(|t| *t == strip(&r.r#type)) else {
		return Ok(None);
	};
	let data = match r.data {
		O::MinecraftCraftingShaped(s) => N::MinecraftCraftingShaped(new::play_to_client_packet_declare_recipes_items_data_minecraft_crafting_shaped::Ty {
			group: s.group,
			category: s.category,
			grid_width: s.width,
			grid_height: s.height,
			ingredients: s.ingredients.into_iter().map(|col| col.into_iter().map(up_list).collect()).collect(),
			result: non_empty(s.result),
			show_notification: s.show_notification,
		}),
		O::MinecraftCraftingShapeless(s) => N::MinecraftCraftingShapeless(new::play_to_client_packet_declare_recipes_items_data_minecraft_crafting_shapeless::Ty {
			group: s.group,
			category: s.category,
			ingredients: s.ingredients.into_iter().map(up_list).collect(),
			result: non_empty(s.result),
		}),
		O::MinecraftSmelting(s) => N::MinecraftSmelting(smelting_up(s)),
		O::MinecraftBlasting(s) => N::MinecraftBlasting(smelting_up(s)),
		O::MinecraftSmoking(s) => N::MinecraftSmoking(smelting_up(s)),
		O::MinecraftCampfireCooking(s) => N::MinecraftCampfireCooking(smelting_up(s)),
		O::MinecraftStonecutting(s) => N::MinecraftStonecutting(new::play_to_client_packet_declare_recipes_items_data_minecraft_stonecutting::Ty {
			group: s.group,
			ingredient: up_list(s.ingredient),
			result: non_empty(s.result),
		}),
		O::MinecraftSmithingTransform(s) => N::MinecraftSmithingTransform(new::play_to_client_packet_declare_recipes_items_data_minecraft_smithing_transform::Ty {
			template: up_list(s.template),
			base: up_list(s.base),
			addition: up_list(s.addition),
			result: non_empty(s.result),
		}),
		O::MinecraftSmithingTrim(s) => N::MinecraftSmithingTrim(new::play_to_client_packet_declare_recipes_items_data_minecraft_smithing_trim::Ty {
			template: up_list(s.template),
			base: up_list(s.base),
			addition: up_list(s.addition),
		}),
		O::Default(_) => return Ok(None),
		simple => {
			let mut b = Vec::new();
			varint(&mut b, id as i32);
			b.extend(encode(&simple, old::play_to_client_packet_declare_recipes_items_data::serialize)?);
			decode(&b, |i| new::play_to_client_packet_declare_recipes_items_data::deserialize(i))?
		}
	};
	Ok(Some(new::play_to_client_packet_declare_recipes_items::Ty {
		name: r.recipe_id,
		data,
	}))
}

fn recipe_down(
	r: new::play_to_client_packet_declare_recipes_items::Ty,
) -> Result<Option<old::play_to_client_packet_declare_recipes_items::Ty>> {
	use new::play_to_client_packet_declare_recipes_items_data::Ty as N;
	use old::play_to_client_packet_declare_recipes_items_data::Ty as O;
	let bytes = encode(
		&r.data,
		new::play_to_client_packet_declare_recipes_items_data::serialize,
	)?;
	let (id, payload) = read_varint(&bytes)?;
	let Some(kind) = RECIPE_TYPES
		.get(id as usize)
		.map(|t| format!("minecraft:{t}"))
	else {
		return Ok(None);
	};
	let data = match r.data {
		N::MinecraftCraftingShaped(s) => O::MinecraftCraftingShaped(old::play_to_client_packet_declare_recipes_items_data_minecraft_crafting_shaped::Ty {
			group: s.group,
			category: s.category,
			width: s.grid_width,
			height: s.grid_height,
			ingredients: s.ingredients.into_iter().map(|col| col.into_iter().map(down_list).collect()).collect(),
			result: to_legacy(s.result),
			show_notification: s.show_notification,
		}),
		N::MinecraftCraftingShapeless(s) => O::MinecraftCraftingShapeless(old::play_to_client_packet_declare_recipes_items_data_minecraft_crafting_shapeless::Ty {
			group: s.group,
			category: s.category,
			ingredients: s.ingredients.into_iter().map(down_list).collect(),
			result: to_legacy(s.result),
		}),
		N::MinecraftSmelting(s) => O::MinecraftSmelting(smelting_down(s)),
		N::MinecraftBlasting(s) => O::MinecraftBlasting(smelting_down(s)),
		N::MinecraftSmoking(s) => O::MinecraftSmoking(smelting_down(s)),
		N::MinecraftCampfireCooking(s) => O::MinecraftCampfireCooking(smelting_down(s)),
		N::MinecraftStonecutting(s) => O::MinecraftStonecutting(old::play_to_client_packet_declare_recipes_items_data_minecraft_stonecutting::Ty {
			group: s.group,
			ingredient: down_list(s.ingredient),
			result: to_legacy(s.result),
		}),
		N::MinecraftSmithingTransform(s) => O::MinecraftSmithingTransform(old::play_to_client_packet_declare_recipes_items_data_minecraft_smithing_transform::Ty {
			template: down_list(s.template),
			base: down_list(s.base),
			addition: down_list(s.addition),
			result: to_legacy(s.result),
		}),
		N::MinecraftSmithingTrim(s) => O::MinecraftSmithingTrim(old::play_to_client_packet_declare_recipes_items_data_minecraft_smithing_trim::Ty {
			template: down_list(s.template),
			base: down_list(s.base),
			addition: down_list(s.addition),
		}),
		N::Default(_) => return Ok(None),
		_ => decode(payload, |i| old::play_to_client_packet_declare_recipes_items_data::deserialize(i, old::play_to_client_packet_declare_recipes_items_data::Args { r#type: &kind }))?,
	};
	Ok(Some(old::play_to_client_packet_declare_recipes_items::Ty {
		r#type: kind,
		recipe_id: r.name,
		data,
	}))
}

pub fn trails_declare_recipes_1_20_3_to_1_20_5(
	p: Vec<old::play_to_client_packet_declare_recipes_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<Client>> {
	let recipes = p
		.into_iter()
		.map(recipe_up)
		.filter_map(Result::transpose)
		.collect::<Result<_>>()?;
	Ok(vec![Client::DeclareRecipes(recipes)])
}

pub fn trails_declare_recipes_1_20_5_to_1_20_3(
	p: Vec<new::play_to_client_packet_declare_recipes_items::Ty>,
	_cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let recipes = p
		.into_iter()
		.map(recipe_down)
		.filter_map(Result::transpose)
		.collect::<Result<_>>()?;
	Ok(vec![ClientOld::DeclareRecipes(recipes)])
}

pub fn trails_no_factor_1_20_5_to_1_20_3(
	_src: &new::play_to_client_packet_entity_effect::Ty,
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
	EntityEquipment,
	old::play_to_client_packet_entity_equipment::Ty,
	|p, cx| {
		let horse =
			common::entity_kind(cx, p.entity_id).is_some_and(|k| HORSES.contains(&k.as_str()));
		new::play_to_client_packet_entity_equipment::Ty {
			entity_id: p.entity_id,
			equipments: p
				.equipments
				.into_iter()
				.map(
					|e| new::play_to_client_packet_entity_equipment_equipments_items::Ty {
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
) -> Result<Vec<old::play_to_server_packet::Ty>> {
	let now = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map_or(0, |d| d.as_millis() as i64);
	Ok(vec![old::play_to_server_packet::Ty::ChatCommand(
		old::play_to_server_packet_chat_command::Ty {
			command,
			timestamp: now,
			salt: 0,
			argument_signatures: vec![],
			message_count: 0,
			acknowledged: vec![0; 3],
		},
	)])
}

pub fn trails_chat_command_signed_1_20_5_to_1_20_3(
	p: new::play_to_server_packet_chat_command_signed::Ty,
	_cx: &mut Session,
) -> Result<Vec<old::play_to_server_packet::Ty>> {
	Ok(vec![old::play_to_server_packet::Ty::ChatCommand(
		old::play_to_server_packet_chat_command::Ty {
			command: p.command,
			timestamp: p.timestamp,
			salt: p.salt,
			argument_signatures: p
				.argument_signatures
				.into_iter()
				.map(
					|s| old::play_to_server_packet_chat_command_argument_signatures_items::Ty {
						argument_name: s.argument_name,
						signature: s.signature,
					},
				)
				.collect(),
			message_count: p.message_count,
			acknowledged: p.acknowledged,
		},
	)])
}

pub fn trails_chat_command_1_20_3_to_1_20_5(
	p: old::play_to_server_packet_chat_command::Ty,
	_cx: &mut Session,
) -> Result<Vec<new::play_to_server_packet::Ty>> {
	Ok(vec![new::play_to_server_packet::Ty::ChatCommandSigned(
		new::play_to_server_packet_chat_command_signed::Ty {
			command: p.command,
			timestamp: p.timestamp,
			salt: p.salt,
			argument_signatures: p
				.argument_signatures
				.into_iter()
				.map(|s| {
					new::play_to_server_packet_chat_command_signed_argument_signatures_items::Ty {
						argument_name: s.argument_name,
						signature: s.signature,
					}
				})
				.collect(),
			message_count: p.message_count,
			acknowledged: p.acknowledged,
		},
	)])
}

const LLAMAS: &[&str] = &["llama", "trader_llama"];

fn swag(entity: i32, item: &Slot) -> Result<ClientOld> {
	let color = match item {
		Slot::Default(_, d) => registry::names(NEW, "items")
			.and_then(|n| n.name(d.item_id as u32))
			.and_then(|n| n.strip_suffix("_carpet"))
			.and_then(|c| items::DYES.iter().position(|d| *d == c))
			.map_or(-1, |i| i as i32),
		_ => -1,
	};
	let key = common::meta_index(OLD, "llama", "swag").ok_or(Error::Dropped)?;
	Ok(ClientOld::EntityMetadata(
		old::play_to_client_packet_entity_metadata::Ty {
			entity_id: entity,
			metadata: vec![decode(&entry_bytes(key, 1, &varint_bytes(color)), |i| {
				old::entity_metadata_items::deserialize(i)
			})?],
		},
	))
}

pub fn trails_entity_equipment_1_20_5_to_1_20_3(
	p: new::play_to_client_packet_entity_equipment::Ty,
	cx: &mut Session,
) -> Result<Vec<ClientOld>> {
	let kind = common::entity_kind(cx, p.entity_id).unwrap_or_default();
	let mut out = Vec::new();
	let mut equipments = Vec::new();
	for e in p.equipments {
		if e.slot == 6 && LLAMAS.contains(&kind.as_str()) {
			out.push(swag(p.entity_id, &e.item)?);
			continue;
		}
		equipments.push(
			old::play_to_client_packet_entity_equipment_equipments_items::Ty {
				slot: if e.slot == 6 { 4 } else { e.slot },
				item: to_legacy(e.item),
			},
		);
	}
	if !equipments.is_empty() {
		out.insert(
			0,
			ClientOld::EntityEquipment(old::play_to_client_packet_entity_equipment::Ty {
				entity_id: p.entity_id,
				equipments,
			}),
		);
	}
	Ok(out)
}

arm!(
	trails_open_horse_window_1_20_5_to_1_20_3,
	ClientOld,
	OpenHorseWindow,
	new::play_to_client_packet_open_horse_window::Ty,
	|p, _cx| {
		old::play_to_client_packet_open_horse_window::Ty {
			window_id: p.window_id,
			nb_slots: p.nb_slots + 1,
			entity_id: p.entity_id,
		}
	}
);

pub fn trails_select_known_packs_1_20_5_to_1_20_3(
	_packs: Vec<new::configuration_to_client_packet_select_known_packs_items::Ty>,
	cx: &mut Session,
) -> Result<Vec<ConfigOld>> {
	common::reply(
		cx,
		Root::ConfigurationToServer,
		NEW,
		&new::configuration_to_server_packet::Ty::SelectKnownPacks(vec![]),
		new::configuration_to_server_packet::serialize,
	)?;
	Ok(vec![])
}

fn cookie(key: String) -> new::packet_common_cookie_response::Ty {
	new::packet_common_cookie_response::Ty { key, value: None }
}

pub fn trails_login_cookie_request_1_20_5_to_1_20_3(
	key: String,
	cx: &mut Session,
) -> Result<Vec<old::login_to_client_packet::Ty>> {
	common::reply(
		cx,
		Root::LoginToServer,
		NEW,
		&new::login_to_server_packet::Ty::CookieResponse(cookie(key)),
		new::login_to_server_packet::serialize,
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
		&new::configuration_to_server_packet::Ty::CookieResponse(cookie(key)),
		new::configuration_to_server_packet::serialize,
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
		&new::play_to_server_packet::Ty::CookieResponse(cookie(key)),
		new::play_to_server_packet::serialize,
	)?;
	Ok(vec![])
}
