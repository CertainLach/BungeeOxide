use super::text;
use crate::gen::convert::{v1_6_4, v1_7};
use crate::pds::convert::Result;
use crate::session::Session;

type Client164 = v1_6_4::play_to_client_packet::Ty;
type Client17 = v1_7::play_to_client_packet::Ty;

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
	play_to_client_packet,
	PlayerInfo,
	play_to_client_packet_player_info,
	[player_name, online, ping]
);
flat!(
	prenetty_scoreboard_objective_1_7,
	prenetty_scoreboard_objective_1_6_4,
	play_to_client_packet,
	ScoreboardObjective,
	play_to_client_packet_scoreboard_objective,
	[name, display_text, action]
);
flat!(
	prenetty_scoreboard_display_objective_1_7,
	prenetty_scoreboard_display_objective_1_6_4,
	play_to_client_packet,
	ScoreboardDisplayObjective,
	play_to_client_packet_scoreboard_display_objective,
	[position, name]
);
flat!(
	prenetty_spawn_entity_painting_1_7,
	prenetty_spawn_entity_painting_1_6_4,
	play_to_client_packet,
	SpawnEntityPainting,
	play_to_client_packet_spawn_entity_painting,
	[entity_id, title, location, direction]
);
flat!(
	prenetty_update_sign_1_7,
	prenetty_update_sign_1_6_4,
	play_to_client_packet,
	UpdateSign,
	play_to_client_packet_update_sign,
	[location, text1, text2, text3, text4]
);
flat!(
	prenetty_update_sign_server_1_7,
	prenetty_update_sign_server_1_6_4,
	play_to_server_packet,
	UpdateSign,
	play_to_server_packet_update_sign,
	[location, text1, text2, text3, text4]
);
flat!(
	prenetty_custom_payload_1_7,
	prenetty_custom_payload_1_6_4,
	play_to_client_packet,
	CustomPayload,
	play_to_client_packet_custom_payload,
	[channel, data]
);
flat!(
	prenetty_custom_payload_server_1_7,
	prenetty_custom_payload_server_1_6_4,
	play_to_server_packet,
	CustomPayload,
	play_to_server_packet_custom_payload,
	[channel, data]
);

pub fn prenetty_respawn_1_7(
	p: v1_6_4::play_to_client_packet_respawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client17>> {
	Ok(vec![Client17::Respawn(
		v1_7::play_to_client_packet_respawn::Ty {
			dimension: p.dimension,
			difficulty: p.difficulty,
			gamemode: p.gamemode,
			level_type: p.level_type,
		},
	)])
}

pub fn prenetty_respawn_1_6_4(
	p: v1_7::play_to_client_packet_respawn::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client164>> {
	Ok(vec![Client164::Respawn(
		v1_6_4::play_to_client_packet_respawn::Ty {
			dimension: p.dimension,
			difficulty: p.difficulty,
			gamemode: p.gamemode,
			world_height: 256,
			level_type: p.level_type,
		},
	)])
}

pub fn prenetty_open_window_1_7(
	p: v1_6_4::play_to_client_packet_open_window::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client17>> {
	use v1_6_4::play_to_client_packet_open_window_entity_id::Ty as Old;
	use v1_7::play_to_client_packet_open_window_entity_id::Ty as New;
	Ok(vec![Client17::OpenWindow(
		v1_7::play_to_client_packet_open_window::Ty {
			window_id: p.window_id,
			window_title: p.window_title,
			slot_count: p.slot_count,
			use_provided_title: p.use_provided_title,
			entity_id: match p.entity_id {
				Old::V11(id) => New::V11(id),
				Old::Default(t) => New::Default(t),
			},
		},
	)])
}

pub fn prenetty_open_window_1_6_4(
	p: v1_7::play_to_client_packet_open_window::Ty,
	_cx: &mut Session,
) -> Result<Vec<Client164>> {
	use v1_6_4::play_to_client_packet_open_window_entity_id::Ty as Old;
	use v1_7::play_to_client_packet_open_window_entity_id::Ty as New;
	Ok(vec![Client164::OpenWindow(
		v1_6_4::play_to_client_packet_open_window::Ty {
			window_id: p.window_id,
			window_title: text::truncate(&p.window_title, 32),
			slot_count: p.slot_count,
			use_provided_title: p.use_provided_title,
			entity_id: match p.entity_id {
				New::V11(id) => Old::V11(id),
				New::Default(t) => Old::Default(t),
			},
		},
	)])
}

macro_rules! score {
	($name:ident, $from:ident, $to:ident) => {
		pub fn $name(
			p: $from::play_to_client_packet_scoreboard_score::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_to_client_packet::Ty>> {
			use $from::play_to_client_packet_scoreboard_score_action::Ty as A;
			use $to::play_to_client_packet_scoreboard_score_action::Ty as B;
			Ok(vec![$to::play_to_client_packet::Ty::ScoreboardScore(
				$to::play_to_client_packet_scoreboard_score::Ty {
					item_name: p.item_name,
					action: match p.action {
						A::V1 => B::V1,
						A::Default(t, s) => B::Default(
							t,
							$to::play_to_client_packet_scoreboard_score_action_default::Ty {
								score_name: s.score_name,
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
			p: $from::play_to_client_packet_scoreboard_team::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_to_client_packet::Ty>> {
			use $from::play_to_client_packet_scoreboard_team_mode::Ty as A;
			use $to::play_to_client_packet_scoreboard_team_mode::Ty as B;
			let mode = match p.mode {
				A::V0(t) => B::V0($to::play_to_client_packet_scoreboard_team_mode_v0::Ty {
					name: t.name,
					prefix: t.prefix,
					suffix: t.suffix,
					friendly_fire: t.friendly_fire,
					players: t.players,
				}),
				A::V2(t) => B::V2($to::play_to_client_packet_scoreboard_team_mode_v2::Ty {
					name: t.name,
					prefix: t.prefix,
					suffix: t.suffix,
					friendly_fire: t.friendly_fire,
				}),
				A::V3(players) => B::V3(players),
				A::V4(players) => B::V4(players),
				A::Default(t) => B::Default(t),
			};
			Ok(vec![$to::play_to_client_packet::Ty::ScoreboardTeam(
				$to::play_to_client_packet_scoreboard_team::Ty { team: p.team, mode },
			)])
		}
	};
}

team!(prenetty_scoreboard_team_1_7, v1_6_4, v1_7);
team!(prenetty_scoreboard_team_1_6_4, v1_7, v1_6_4);

macro_rules! attributes {
	($name:ident, $from:ident, $to:ident) => {
		pub fn $name(
			p: $from::play_to_client_packet_update_attributes::Ty,
			_cx: &mut Session,
		) -> Result<Vec<$to::play_to_client_packet::Ty>> {
			let properties = p
				.properties
				.into_iter()
				.map(
					|a| $to::play_to_client_packet_update_attributes_properties_items::Ty {
						key: a.key,
						value: a.value,
						modifiers: a
							.modifiers
							.into_iter()
							.map(|m| {
								$to::play_to_client_packet_update_attributes_properties_items_modifiers_items::Ty {
																			uuid: m.uuid,
																			amount: m.amount,
																			operation: m.operation,
																		}
							})
							.collect(),
					},
				)
				.collect();
			Ok(vec![$to::play_to_client_packet::Ty::UpdateAttributes(
				$to::play_to_client_packet_update_attributes::Ty {
					entity_id: p.entity_id,
					properties,
				},
			)])
		}
	};
}

attributes!(prenetty_update_attributes_1_7, v1_6_4, v1_7);
attributes!(prenetty_update_attributes_1_6_4, v1_7, v1_6_4);
