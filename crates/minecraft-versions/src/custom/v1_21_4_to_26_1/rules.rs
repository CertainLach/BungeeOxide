macro_rules! items {
	($a:ident => $b:ident, $from:path => $to:path {
		slot: $slot:ident, hashed: $hashed:ident, trade1: $trade1:ident, trade2: $trade2:ident,
		display: $display:ident, recipes: $recipes:ident, stonecutter: $stonecutter:ident,
		requirements: $requirements:ident, repairable: $repairable:ident, pots: $pots:ident,
	}) => {
		pub fn $slot(
			v: &$a::slot_default::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::remap::item($from, $to, v.item_id)
		}

		pub fn $hashed(
			v: &$a::hashed_slot::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::remap::item($from, $to, v.item_id)
		}

		pub fn $trade1(
			v: &$a::play_to_client_packet_trade_list_trades_items_input_item1::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::remap::item($from, $to, v.item_id)
		}

		pub fn $trade2(
			v: &$a::play_to_client_packet_trade_list_trades_items_input_item2_items::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::remap::item($from, $to, v.item_id)
		}

		pub fn $display(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::play_to_client_slot_display::Ty> {
			Ok($b::play_to_client_slot_display::Ty::Item(
				super::remap::item($from, $to, v)?,
			))
		}

		pub fn $recipes(
			v: &$a::play_to_client_packet_declare_recipes_recipes_items::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<i32>> {
			super::remap::items($from, $to, v.items.clone())
		}

		pub fn $stonecutter(
			v: &$a::play_to_client_packet_declare_recipes_stone_cutter_recipes_items::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::id_set::Ty> {
			super::remap::item_set($from, $to, v.input.clone())
		}

		pub fn $requirements(
			v: &$a::play_to_client_packet_recipe_book_add_entries_items_recipe::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Option<Vec<$b::id_set::Ty>>> {
			v.crafting_requirements
				.clone()
				.map(|sets| {
					sets.into_iter()
						.map(|s| super::remap::item_set($from, $to, s))
						.collect()
				})
				.transpose()
		}

		pub fn $repairable(
			v: $a::id_set::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::slot_component::Ty> {
			Ok($b::slot_component::Ty::Repairable(super::remap::item_set(
				$from, $to, v,
			)?))
		}

		pub fn $pots(
			v: Vec<i32>,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::slot_component::Ty> {
			Ok($b::slot_component::Ty::PotDecorations(super::remap::items(
				$from, $to, v,
			)?))
		}
	};
}

macro_rules! blocks {
	($a:ident => $b:ident, $from:path => $to:path {
		block_change: $block_change:ident, multi_block: $multi_block:ident, block_action: $block_action:ident,
		world_event: $world_event:ident, chunk: $chunk:ident, tool: $tool:ident, predicate: $predicate:ident,
		particle: [$p_block:ident, $p_marker:ident, $p_dust:ident, $p_pillar:ident, $p_crumble:ident],
		meta: [$m_state:ident, $m_optional:ident],
	}) => {
		pub fn $block_change(
			v: &$a::play_to_client_packet_block_change::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::remap::state($from, $to, v.r#type)
		}

		pub fn $multi_block(
			v: &$a::play_to_client_packet_multi_block_change::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<i32>> {
			Ok(v.records
				.iter()
				.map(|r| {
					(crate::registry::remap_state($from, $to, (*r as u32) >> 12) as i32) << 12
						| (r & 0xfff)
				})
				.collect())
		}

		pub fn $block_action(
			v: &$a::play_to_client_packet_block_action::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::remap::block($from, $to, v.block_id)
		}

		pub fn $world_event(
			v: &$a::play_to_client_packet_world_event::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			match v.effect_id {
				2001 | 3008 => super::remap::state($from, $to, v.data),
				_ => Ok(v.data),
			}
		}

		pub fn $chunk(
			v: &$a::play_to_client_packet_map_chunk::Ty,
			cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<u8>> {
			super::chunk::step($from, $to, &v.chunk_data, super::world::biomes(cx))
		}

		pub fn $tool(
			v: &$a::slot_component_tool_rules_items::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::id_set::Ty> {
			super::remap::block_set($from, $to, v.blocks.clone())
		}

		pub fn $predicate(
			v: &$a::item_block_predicate::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Option<$b::item_block_predicate_block_set_items::Ty>> {
			Ok(v.block_set.clone().map(|s| match s {
				$a::item_block_predicate_block_set_items::Ty::Named(n) => {
					$b::item_block_predicate_block_set_items::Ty::Named(n)
				}
				$a::item_block_predicate_block_set_items::Ty::Ids(ids) => {
					$b::item_block_predicate_block_set_items::Ty::Ids(
						ids.into_iter()
							.filter_map(|i| super::util::remap("blocks", $from, $to, i))
							.collect(),
					)
				}
			}))
		}

		pub fn $p_block(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::particle::Ty> {
			Ok($b::particle::Ty::Block(super::remap::state($from, $to, v)?))
		}

		pub fn $p_marker(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::particle::Ty> {
			Ok($b::particle::Ty::BlockMarker(super::remap::state(
				$from, $to, v,
			)?))
		}

		pub fn $p_dust(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::particle::Ty> {
			Ok($b::particle::Ty::FallingDust(super::remap::state(
				$from, $to, v,
			)?))
		}

		pub fn $p_pillar(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::particle::Ty> {
			Ok($b::particle::Ty::DustPillar(super::remap::state(
				$from, $to, v,
			)?))
		}

		pub fn $p_crumble(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::particle::Ty> {
			Ok($b::particle::Ty::BlockCrumble(super::remap::state(
				$from, $to, v,
			)?))
		}

		pub fn $m_state(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::entity_metadata_entry_value::Ty> {
			Ok($b::entity_metadata_entry_value::Ty::BlockState(
				super::remap::state($from, $to, v)?,
			))
		}

		pub fn $m_optional(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::entity_metadata_entry_value::Ty> {
			Ok($b::entity_metadata_entry_value::Ty::OptionalBlockState(
				super::remap::state($from, $to, v)?,
			))
		}
	};
}

macro_rules! entities {
	($a:ident => $b:ident, $from:path => $to:path {
		spawn: $spawn:ident, object: $object:ident, equippable: $equippable:ident,
	}) => {
		pub fn $spawn(
			v: &$a::play_to_client_packet_spawn_entity::Ty,
			cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::shared::spawned(cx, v.entity_id, $from, v.r#type);
			super::remap::entity($from, $to, v.r#type)
		}

		pub fn $object(
			v: &$a::play_to_client_packet_spawn_entity::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			match crate::registry::names($from, "entities")
				.and_then(|n| n.name(v.r#type.max(0) as u32))
			{
				Some("falling_block") => super::remap::state($from, $to, v.object_data),
				_ => Ok(v.object_data),
			}
		}

		pub fn $equippable(
			v: &$a::slot_component_equippable::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Option<$b::id_set::Ty>> {
			v.allowed_entities
				.clone()
				.map(|s| super::remap::entity_set($from, $to, s))
				.transpose()
		}
	};
}

macro_rules! statistics {
	($a:ident => $b:ident, $from:path => $to:path { $name:ident }) => {
		pub fn $name(
			v: Vec<$a::play_to_client_packet_statistics_items::Ty>,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<$b::play_to_client_packet::Ty>> {
			Ok(vec![$b::play_to_client_packet::Ty::Statistics(
				super::ids::statistics($from, $to, v),
			)])
		}
	};
}

macro_rules! tags {
	($a:ident => $b:ident, $from:path => $to:path, $renames:expr, { config: $config:ident, play: $play:ident }) => {
		pub fn $config(
			v: Vec<$a::configuration_to_client_packet_tags_items::Ty>,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<$b::configuration_to_client_packet::Ty>> {
			Ok(vec![$b::configuration_to_client_packet::Ty::Tags(
				super::tags::config($from, $to, $renames, v),
			)])
		}

		pub fn $play(
			v: Vec<$a::play_to_client_packet_tags_items::Ty>,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<$b::play_to_client_packet::Ty>> {
			Ok(vec![$b::play_to_client_packet::Ty::Tags(
				super::tags::play($from, $to, $renames, v),
			)])
		}
	};
}

macro_rules! hidden {
	($a:ident, $from:path => $to:path, $name:ident) => {
		pub fn $name(
			v: &$a::slot_component_tooltip_display::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<i32>> {
			Ok(v.hidden_components
				.iter()
				.filter_map(|id| super::components::remap($from, $to, *id))
				.collect())
		}
	};
}

pub(crate) use {blocks, entities, hidden, items, statistics, tags};
