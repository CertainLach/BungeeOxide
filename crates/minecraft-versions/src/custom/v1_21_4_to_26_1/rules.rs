macro_rules! items {
	($a:ident => $b:ident, $from:path => $to:path {
		slot: $slot:ident, hashed: $hashed:ident, trade1: $trade1:ident, trade2: $trade2:ident,
		display: $display:ident, recipes: $recipes:ident, stonecutter: $stonecutter:ident,
		requirements: $requirements:ident, repairable: $repairable:ident, pots: $pots:ident,
	}) => {
		pub fn $slot(
			v: &$a::Slot__default::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::remap::item($from, $to, v.itemId)
		}

		pub fn $hashed(
			v: &$a::HashedSlot::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::remap::item($from, $to, v.itemId)
		}

		pub fn $trade1(
			v: &$a::play_toClient_packet_trade_list__trades__items__inputItem1::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::remap::item($from, $to, v.itemId)
		}

		pub fn $trade2(
			v: &$a::play_toClient_packet_trade_list__trades__items__inputItem2__items::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::remap::item($from, $to, v.itemId)
		}

		pub fn $display(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::play_toClient_SlotDisplay::Ty> {
			Ok($b::play_toClient_SlotDisplay::Ty::item(super::remap::item(
				$from, $to, v,
			)?))
		}

		pub fn $recipes(
			v: &$a::play_toClient_packet_declare_recipes__recipes__items::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<i32>> {
			super::remap::items($from, $to, v.items.clone())
		}

		pub fn $stonecutter(
			v: &$a::play_toClient_packet_declare_recipes__stoneCutterRecipes__items::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::IDSet::Ty> {
			super::remap::item_set($from, $to, v.input.clone())
		}

		pub fn $requirements(
			v: &$a::play_toClient_packet_recipe_book_add__entries__items__recipe::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Option<Vec<$b::IDSet::Ty>>> {
			v.craftingRequirements
				.clone()
				.map(|sets| {
					sets.into_iter()
						.map(|s| super::remap::item_set($from, $to, s))
						.collect()
				})
				.transpose()
		}

		pub fn $repairable(
			v: $a::IDSet::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::SlotComponent::Ty> {
			Ok($b::SlotComponent::Ty::repairable(super::remap::item_set(
				$from, $to, v,
			)?))
		}

		pub fn $pots(
			v: Vec<i32>,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::SlotComponent::Ty> {
			Ok($b::SlotComponent::Ty::pot_decorations(super::remap::items(
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
			v: &$a::play_toClient_packet_block_change::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::remap::state($from, $to, v.r#type)
		}

		pub fn $multi_block(
			v: &$a::play_toClient_packet_multi_block_change::Ty,
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
			v: &$a::play_toClient_packet_block_action::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::remap::block($from, $to, v.blockId)
		}

		pub fn $world_event(
			v: &$a::play_toClient_packet_world_event::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			match v.effectId {
				2001 | 3008 => super::remap::state($from, $to, v.data),
				_ => Ok(v.data),
			}
		}

		pub fn $chunk(
			v: &$a::play_toClient_packet_map_chunk::Ty,
			cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<u8>> {
			super::chunk::step($from, $to, &v.chunkData, super::world::biomes(cx))
		}

		pub fn $tool(
			v: &$a::SlotComponent__tool__rules__items::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::IDSet::Ty> {
			super::remap::block_set($from, $to, v.blocks.clone())
		}

		pub fn $predicate(
			v: &$a::ItemBlockPredicate::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Option<$b::ItemBlockPredicate__blockSet__items::Ty>> {
			Ok(v.blockSet.clone().map(|s| match s {
				$a::ItemBlockPredicate__blockSet__items::Ty::Named(n) => {
					$b::ItemBlockPredicate__blockSet__items::Ty::Named(n)
				}
				$a::ItemBlockPredicate__blockSet__items::Ty::Ids(ids) => {
					$b::ItemBlockPredicate__blockSet__items::Ty::Ids(
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
		) -> crate::pds::convert::Result<$b::Particle::Ty> {
			Ok($b::Particle::Ty::block(super::remap::state($from, $to, v)?))
		}

		pub fn $p_marker(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::Particle::Ty> {
			Ok($b::Particle::Ty::block_marker(super::remap::state(
				$from, $to, v,
			)?))
		}

		pub fn $p_dust(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::Particle::Ty> {
			Ok($b::Particle::Ty::falling_dust(super::remap::state(
				$from, $to, v,
			)?))
		}

		pub fn $p_pillar(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::Particle::Ty> {
			Ok($b::Particle::Ty::dust_pillar(super::remap::state(
				$from, $to, v,
			)?))
		}

		pub fn $p_crumble(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::Particle::Ty> {
			Ok($b::Particle::Ty::block_crumble(super::remap::state(
				$from, $to, v,
			)?))
		}

		pub fn $m_state(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::entityMetadataEntry__value::Ty> {
			Ok($b::entityMetadataEntry__value::Ty::block_state(
				super::remap::state($from, $to, v)?,
			))
		}

		pub fn $m_optional(
			v: i32,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<$b::entityMetadataEntry__value::Ty> {
			Ok($b::entityMetadataEntry__value::Ty::optional_block_state(
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
			v: &$a::play_toClient_packet_spawn_entity::Ty,
			cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			super::shared::spawned(cx, v.entityId, $from, v.r#type);
			super::remap::entity($from, $to, v.r#type)
		}

		pub fn $object(
			v: &$a::play_toClient_packet_spawn_entity::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<i32> {
			match crate::registry::names($from, "entities")
				.and_then(|n| n.name(v.r#type.max(0) as u32))
			{
				Some("falling_block") => super::remap::state($from, $to, v.objectData),
				_ => Ok(v.objectData),
			}
		}

		pub fn $equippable(
			v: &$a::SlotComponent__equippable::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Option<$b::IDSet::Ty>> {
			v.allowedEntities
				.clone()
				.map(|s| super::remap::entity_set($from, $to, s))
				.transpose()
		}
	};
}

macro_rules! statistics {
	($a:ident => $b:ident, $from:path => $to:path { $name:ident }) => {
		pub fn $name(
			v: Vec<$a::play_toClient_packet__statistics__items::Ty>,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<$b::play_toClient_packet::Ty>> {
			Ok(vec![$b::play_toClient_packet::Ty::statistics(
				super::ids::statistics($from, $to, v),
			)])
		}
	};
}

macro_rules! tags {
	($a:ident => $b:ident, $from:path => $to:path, $renames:expr, { config: $config:ident, play: $play:ident }) => {
		pub fn $config(
			v: Vec<$a::configuration_toClient_packet__tags__items::Ty>,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<$b::configuration_toClient_packet::Ty>> {
			Ok(vec![$b::configuration_toClient_packet::Ty::tags(
				super::tags::config($from, $to, $renames, v),
			)])
		}

		pub fn $play(
			v: Vec<$a::play_toClient_packet__tags__items::Ty>,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<$b::play_toClient_packet::Ty>> {
			Ok(vec![$b::play_toClient_packet::Ty::tags(super::tags::play(
				$from, $to, $renames, v,
			))])
		}
	};
}

macro_rules! hidden {
	($a:ident, $from:path => $to:path, $name:ident) => {
		pub fn $name(
			v: &$a::SlotComponent__tooltip_display::Ty,
			_cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<i32>> {
			Ok(v.hiddenComponents
				.iter()
				.filter_map(|id| super::components::remap($from, $to, *id))
				.collect())
		}
	};
}

pub(crate) use {blocks, entities, hidden, items, statistics, tags};
