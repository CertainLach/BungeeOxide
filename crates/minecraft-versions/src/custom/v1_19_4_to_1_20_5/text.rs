use nbt::json::{parse_text, text_from_nbt, text_to_nbt};
use nbt::Tag;

pub fn to_nbt(json: &str) -> Tag {
	Tag::new(text_to_nbt(&parse_text(json)))
}

pub fn from_nbt(t: &Tag) -> String {
	text_from_nbt(&t.value).to_string()
}
