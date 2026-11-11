use super::util::remap;
use crate::gen::convert::v1_21_4::play_to_client_packet_statistics_items as stat;
use crate::gen::convert::Version;

const CUSTOM_STATS: &[(Version, i32)] = &[(Version::V1_21_6, 17), (Version::V1_21_11, 22)];

fn custom_stat(from: Version, to: Version, id: i32) -> Option<i32> {
	match CUSTOM_STATS.iter().find(|(v, _)| *v == to || *v == from) {
		Some((v, at)) if *v == to => Some(if id >= *at { id + 1 } else { id }),
		Some((_, at)) => match id.cmp(at) {
			std::cmp::Ordering::Less => Some(id),
			std::cmp::Ordering::Equal => None,
			std::cmp::Ordering::Greater => Some(id - 1),
		},
		None => Some(id),
	}
}

pub fn statistics(from: Version, to: Version, v: Vec<stat::Ty>) -> Vec<stat::Ty> {
	v.into_iter()
		.filter_map(|s| {
			let id = match s.category_id {
				0 => remap("blocks", from, to, s.statistic_id),
				1..=5 => remap("items", from, to, s.statistic_id),
				6 | 7 => remap("entities", from, to, s.statistic_id),
				8 => custom_stat(from, to, s.statistic_id),
				_ => None,
			}?;
			Some(stat::Ty {
				statistic_id: id,
				..s
			})
		})
		.collect()
}
