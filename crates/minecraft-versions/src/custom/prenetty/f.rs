use super::text;
use crate::gen::convert::{v1_6_4, v1_7};
use crate::pds::convert::Result;
use crate::session::Session;

type Client164 = v1_6_4::play_toClient_packet::Ty;
type Client17 = v1_7::play_toClient_packet::Ty;

macro_rules! flat {
	($up:ident, $down:ident, $root:ident, $arm:ident, $ty:ident, [$($f:ident),*]) => {
		pub fn $up(p: v1_6_4::$ty::Ty, _cx: &mut Session) -> Result<Vec<v1_7::$root::Ty>> {
			Ok(vec![v1_7::$root::Ty::$arm(v1_7::$ty::Ty { $($f: p.$f),* })])
		}

		pub fn $down(p: v1_7::$ty::Ty, _cx: &mut Session) -> Result<Vec<v1_6_4::$root::Ty>> {
			Ok(vec![v1_6_4::$root::Ty::$arm(v1_6_4::$ty::Ty { $($f: p.$f),* })])
		}
	};
}

flat!(
	prenetty_player_info_1_7,
	prenetty_player_info_1_6_4,
	play_toClient_packet,
	player_info,
	play_toClient_packet_player_info,
	[playerName, online, ping]
);
flat!(
	prenetty_scoreboard_objective_1_7,
	prenetty_scoreboard_objective_1_6_4,
	play_toClient_packet,
	scoreboard_objective,
	play_toClient_packet_scoreboard_objective,
	[name, displayText, action]
);
flat!(
	prenetty_scoreboard_display_objective_1_7,
	prenetty_scoreboard_display_objective_1_6_4,
	play_toClient_packet,
	scoreboard_display_objective,
	play_toClient_packet_scoreboard_display_objective,
	[position, name]
);
flat!(
	prenetty_spawn_entity_painting_1_7,
	prenetty_spawn_entity_painting_1_6_4,
	play_toClient_packet,
	spawn_entity_painting,
	play_toClient_packet_spawn_entity_painting,
	[entityId, title, location, direction]
);
flat!(
	prenetty_update_sign_1_7,
	prenetty_update_sign_1_6_4,
	play_toClient_packet,
	update_sign,
	play_toClient_packet_update_sign,
	[location, text1, text2, text3, text4]
);
flat!(
	prenetty_update_sign_server_1_7,
	prenetty_update_sign_server_1_6_4,
	play_toServer_packet,
	update_sign,
	play_toServer_packet_update_sign,
	[location, text1, text2, text3, text4]
);
flat!(
	prenetty_custom_payload_1_7,
	prenetty_custom_payload_1_6_4,
	play_toClient_packet,
	custom_payload,
	play_toClient_packet_custom_payload,
	[channel, data]
);
flat!(
	prenetty_custom_payload_server_1_7,
	prenetty_custom_payload_server_1_6_4,
	play_toServer_packet,
	custom_payload,
	play_toServer_packet_custom_payload,
	[channel, data]
);

pub fn prenetty_respawn_1_7(
	p: v1_6_4::play_toClient_packet_respawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client17>> {
	Ok(vec![Client17::respawn(
		v1_7::play_toClient_packet_respawn::Ty {
			dimension: p.dimension,
			difficulty: p.difficulty,
			gamemode: p.gamemode,
			levelType: p.levelType,
		},
	)])
}

pub fn prenetty_respawn_1_6_4(
	p: v1_7::play_toClient_packet_respawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client164>> {
	Ok(vec![Client164::respawn(
		v1_6_4::play_toClient_packet_respawn::Ty {
			dimension: p.dimension,
			difficulty: p.difficulty,
			gamemode: p.gamemode,
			worldHeight: 256,
			levelType: p.levelType,
		},
	)])
}

pub fn prenetty_open_window_1_7(
	p: v1_6_4::play_toClient_packet_open_window::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client17>> {
	use v1_6_4::play_toClient_packet_open_window__entityId::Ty as Old;
	use v1_7::play_toClient_packet_open_window__entityId::Ty as New;
	Ok(vec![Client17::open_window(
		v1_7::play_toClient_packet_open_window::Ty {
			windowId: p.windowId,
			windowTitle: p.windowTitle,
			slotCount: p.slotCount,
			useProvidedTitle: p.useProvidedTitle,
			entityId: match p.entityId {
				Old::v11(id) => New::v11(id),
				Old::default(t) => New::default(t),
			},
		},
	)])
}

pub fn prenetty_open_window_1_6_4(
	p: v1_7::play_toClient_packet_open_window::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client164>> {
	use v1_6_4::play_toClient_packet_open_window__entityId::Ty as Old;
	use v1_7::play_toClient_packet_open_window__entityId::Ty as New;
	Ok(vec![Client164::open_window(
		v1_6_4::play_toClient_packet_open_window::Ty {
			windowId: p.windowId,
			windowTitle: text::truncate(&p.windowTitle, 32),
			slotCount: p.slotCount,
			useProvidedTitle: p.useProvidedTitle,
			entityId: match p.entityId {
				New::v11(id) => Old::v11(id),
				New::default(t) => Old::default(t),
			},
		},
	)])
}

macro_rules! score {
	($name:ident, $from:ident, $to:ident) => {
		pub fn $name(
			p: $from::play_toClient_packet_scoreboard_score::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_toClient_packet::Ty>> {
			use $from::play_toClient_packet_scoreboard_score__action::Ty as A;
			use $to::play_toClient_packet_scoreboard_score__action::Ty as B;
			Ok(vec![$to::play_toClient_packet::Ty::scoreboard_score(
				$to::play_toClient_packet_scoreboard_score::Ty {
					itemName: p.itemName,
					action: match p.action {
						A::v1 => B::v1,
						A::default(t, s) => B::default(
							t,
							$to::play_toClient_packet_scoreboard_score__action__default::Ty {
								scoreName: s.scoreName,
								value: s.value,
							},
						),
					},
				},
			)])
		}
	};
}

score!(prenetty_scoreboard_score_1_7, v1_6_4, v1_7);
score!(prenetty_scoreboard_score_1_6_4, v1_7, v1_6_4);

macro_rules! team {
	($name:ident, $from:ident, $to:ident) => {
		pub fn $name(
			p: $from::play_toClient_packet_scoreboard_team::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_toClient_packet::Ty>> {
			use $from::play_toClient_packet_scoreboard_team__mode::Ty as A;
			use $to::play_toClient_packet_scoreboard_team__mode::Ty as B;
			let mode = match p.mode {
				A::v0(t) => B::v0($to::play_toClient_packet_scoreboard_team__mode__v0::Ty {
					name: t.name,
					prefix: t.prefix,
					suffix: t.suffix,
					friendlyFire: t.friendlyFire,
					players: t.players,
				}),
				A::v2(t) => B::v2($to::play_toClient_packet_scoreboard_team__mode__v2::Ty {
					name: t.name,
					prefix: t.prefix,
					suffix: t.suffix,
					friendlyFire: t.friendlyFire,
				}),
				A::v3(players) => B::v3(players),
				A::v4(players) => B::v4(players),
				A::default(t) => B::default(t),
			};
			Ok(vec![$to::play_toClient_packet::Ty::scoreboard_team(
				$to::play_toClient_packet_scoreboard_team::Ty { team: p.team, mode },
			)])
		}
	};
}

team!(prenetty_scoreboard_team_1_7, v1_6_4, v1_7);
team!(prenetty_scoreboard_team_1_6_4, v1_7, v1_6_4);

macro_rules! attributes {
	($name:ident, $from:ident, $to:ident) => {
		pub fn $name(
			p: $from::play_toClient_packet_update_attributes::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_toClient_packet::Ty>> {
			let properties = p
				.properties
				.into_iter()
				.map(
					|a| $to::play_toClient_packet_update_attributes__properties__items::Ty {
						key: a.key,
						value: a.value,
						modifiers: a
							.modifiers
							.into_iter()
							.map(|m| {
								$to::play_toClient_packet_update_attributes__properties__items__modifiers__items::Ty {
															uuid: m.uuid,
															amount: m.amount,
															operation: m.operation,
														}
							})
							.collect(),
					},
				)
				.collect();
			Ok(vec![$to::play_toClient_packet::Ty::update_attributes(
				$to::play_toClient_packet_update_attributes::Ty {
					entityId: p.entityId,
					properties,
				},
			)])
		}
	};
}

attributes!(prenetty_update_attributes_1_7, v1_6_4, v1_7);
attributes!(prenetty_update_attributes_1_6_4, v1_7, v1_6_4);
