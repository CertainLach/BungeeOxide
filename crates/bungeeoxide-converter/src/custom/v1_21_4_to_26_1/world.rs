use crate::gen::convert::Version;
use crate::Session;
use std::collections::HashMap;

#[derive(Default)]
pub struct World {
	pub biomes: Option<u32>,
	pub entities: HashMap<i32, Vec<(Version, String)>>,
	pub player: Option<i32>,
	pub dimension: String,
}

pub fn world(cx: &mut Session) -> &mut World {
	cx.state::<World>()
}

pub fn biomes(cx: &mut Session) -> u32 {
	world(cx).biomes.unwrap_or(65)
}

pub fn spawned(cx: &mut Session, id: i32, version: Version, ty: i32) {
	if let Some(name) =
		crate::registry::names(version, "entities").and_then(|n| n.name(ty.max(0) as u32))
	{
		let names = world(cx).entities.entry(id).or_default();
		names.retain(|(v, _)| *v != version);
		names.push((version, name.to_owned()));
	}
}

pub fn kind(cx: &mut Session, version: Version, id: i32) -> Option<String> {
	let w = world(cx);
	let nearest = w.entities.get(&id).and_then(|names| {
		names
			.iter()
			.min_by_key(|(v, _)| v.index().abs_diff(version.index()))
	});
	nearest
		.map(|(_, n)| n.clone())
		.or_else(|| (w.player == Some(id)).then(|| "player".to_owned()))
}
