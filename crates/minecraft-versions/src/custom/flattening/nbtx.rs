use nbt::{CompoundExt, CompoundVecExt, Entry, ListItems, Payload};

pub(super) fn root(c: Vec<Entry>) -> Option<Entry> {
	(!c.is_empty()).then(|| Entry::new("", c))
}

pub(super) fn children(tag: Option<Entry>) -> Vec<Entry> {
	tag.and_then(|e| e.value.into_compound())
		.unwrap_or_default()
}

pub(super) fn strings(p: &Payload) -> Vec<String> {
	match p.as_list().map(|l| &l.items) {
		Some(ListItems::String(s)) => s.clone(),
		Some(ListItems::Int(v)) => v.iter().map(|n| n.to_string()).collect(),
		_ => Vec::new(),
	}
}

pub(super) fn invert_colors(c: &mut Vec<Entry>) {
	if let Some(base) = c.field("Base").and_then(Payload::to_i32) {
		c.put("Base", Payload::Int(15 - base));
	}
	if let Some(patterns) = c.compounds_mut("Patterns") {
		for p in patterns {
			if let Some(color) = p.field("Color").and_then(Payload::to_i32) {
				p.put("Color", Payload::Int(15 - color));
			}
		}
	}
}
