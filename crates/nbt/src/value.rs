use crate::{Entry, List, ListItems, Payload, Tag};
use std::ops::{Deref, DerefMut};

impl Payload {
	pub fn kind(&self) -> u8 {
		match self {
			Payload::end => 0,
			Payload::byte(_) => 1,
			Payload::short(_) => 2,
			Payload::int(_) => 3,
			Payload::long(_) => 4,
			Payload::float(_) => 5,
			Payload::double(_) => 6,
			Payload::byteArray(_) => 7,
			Payload::string(_) => 8,
			Payload::list(_) => 9,
			Payload::compound(_) => 10,
			Payload::intArray(_) => 11,
			Payload::longArray(_) => 12,
		}
	}

	pub fn uuid(u: u128) -> Payload {
		Payload::intArray(vec![
			(u >> 96) as i32,
			(u >> 64) as i32,
			(u >> 32) as i32,
			u as i32,
		])
	}

	pub fn as_uuid(&self) -> Option<u128> {
		let Payload::intArray(v) = self else {
			return None;
		};
		let [a, b, c, d] = v.as_slice() else {
			return None;
		};
		Some(
			((*a as u32 as u128) << 96)
				| ((*b as u32 as u128) << 64)
				| ((*c as u32 as u128) << 32)
				| *d as u32 as u128,
		)
	}

	pub fn as_str(&self) -> Option<&str> {
		match self {
			Payload::string(s) => Some(s),
			_ => None,
		}
	}

	pub fn as_compound(&self) -> Option<&Vec<Entry>> {
		match self {
			Payload::compound(c) => Some(c),
			_ => None,
		}
	}

	pub fn as_compound_mut(&mut self) -> Option<&mut Vec<Entry>> {
		match self {
			Payload::compound(c) => Some(c),
			_ => None,
		}
	}

	pub fn into_compound(self) -> Option<Vec<Entry>> {
		match self {
			Payload::compound(c) => Some(c),
			_ => None,
		}
	}

	pub fn as_list(&self) -> Option<&List> {
		match self {
			Payload::list(l) => Some(l),
			_ => None,
		}
	}

	pub fn as_i64(&self) -> Option<i64> {
		match self {
			Payload::byte(v) => Some(*v as i64),
			Payload::short(v) => Some(*v as i64),
			Payload::int(v) => Some(*v as i64),
			Payload::long(v) => Some(*v),
			_ => None,
		}
	}

	pub fn as_f64(&self) -> Option<f64> {
		match self {
			Payload::float(v) => Some(*v as f64),
			Payload::double(v) => Some(*v),
			p => p.as_i64().map(|v| v as f64),
		}
	}

	pub fn to_i32(&self) -> Option<i32> {
		match self {
			Payload::float(v) => Some(*v as i32),
			Payload::double(v) => Some(*v as i32),
			p => p.as_i64().map(|v| v as i32),
		}
	}

	pub fn values(&self) -> Vec<Payload> {
		self.as_list().map(List::values).unwrap_or_default()
	}

	pub fn compounds(&self) -> &[Vec<Entry>] {
		self.as_list().map_or(&[], List::compounds)
	}

	pub fn compounds_mut(&mut self) -> Option<&mut Vec<Vec<Entry>>> {
		match self {
			Payload::list(l) => l.compounds_mut(),
			_ => None,
		}
	}

	pub fn field(&self, name: &str) -> Option<&Payload> {
		self.as_compound()?.field(name)
	}

	pub fn field_mut(&mut self, name: &str) -> Option<PayloadMut<'_>> {
		self.as_compound_mut()?.field_mut(name)
	}

	pub fn put(&mut self, name: &str, value: impl Into<Payload>) {
		if let Some(c) = self.as_compound_mut() {
			c.put(name, value);
		}
	}

	pub fn take(&mut self, name: &str) -> Option<Payload> {
		self.as_compound_mut()?.take(name)
	}
}

impl From<i8> for Payload {
	fn from(v: i8) -> Self {
		Payload::byte(v)
	}
}

impl From<i16> for Payload {
	fn from(v: i16) -> Self {
		Payload::short(v)
	}
}

impl From<i32> for Payload {
	fn from(v: i32) -> Self {
		Payload::int(v)
	}
}

impl From<i64> for Payload {
	fn from(v: i64) -> Self {
		Payload::long(v)
	}
}

impl From<f32> for Payload {
	fn from(v: f32) -> Self {
		Payload::float(v)
	}
}

impl From<f64> for Payload {
	fn from(v: f64) -> Self {
		Payload::double(v)
	}
}

impl From<&str> for Payload {
	fn from(v: &str) -> Self {
		Payload::string(v.to_owned())
	}
}

impl From<String> for Payload {
	fn from(v: String) -> Self {
		Payload::string(v)
	}
}

impl From<Vec<Entry>> for Payload {
	fn from(v: Vec<Entry>) -> Self {
		Payload::compound(v)
	}
}

impl From<List> for Payload {
	fn from(v: List) -> Self {
		Payload::list(v)
	}
}

impl From<ListItems> for Payload {
	fn from(v: ListItems) -> Self {
		Payload::list(List::new(v))
	}
}

pub struct PayloadMut<'a> {
	kind: &'a mut u8,
	value: &'a mut Payload,
}

impl Deref for PayloadMut<'_> {
	type Target = Payload;

	fn deref(&self) -> &Payload {
		self.value
	}
}

impl DerefMut for PayloadMut<'_> {
	fn deref_mut(&mut self) -> &mut Payload {
		self.value
	}
}

impl Drop for PayloadMut<'_> {
	fn drop(&mut self) {
		*self.kind = self.value.kind();
	}
}

impl Entry {
	pub fn new(name: impl Into<String>, value: impl Into<Payload>) -> Entry {
		let value = value.into();
		Entry {
			r#type: value.kind(),
			name: name.into(),
			value,
		}
	}

	pub fn set(&mut self, value: impl Into<Payload>) {
		self.value = value.into();
		self.r#type = self.value.kind();
	}

	pub fn value_mut(&mut self) -> PayloadMut<'_> {
		PayloadMut {
			kind: &mut self.r#type,
			value: &mut self.value,
		}
	}

	pub fn root(tag: &mut Option<Entry>) -> &mut Vec<Entry> {
		let e = tag.get_or_insert_with(|| Entry::new("", Vec::new()));
		if !matches!(e.value, Payload::compound(_)) {
			*e = Entry::new("", Vec::new());
		}
		match &mut e.value {
			Payload::compound(c) => c,
			_ => unreachable!(),
		}
	}
}

impl Tag {
	pub fn new(value: impl Into<Payload>) -> Tag {
		let value = value.into();
		Tag {
			r#type: value.kind(),
			value,
		}
	}

	pub fn set(&mut self, value: impl Into<Payload>) {
		self.value = value.into();
		self.r#type = self.value.kind();
	}

	pub fn value_mut(&mut self) -> PayloadMut<'_> {
		PayloadMut {
			kind: &mut self.r#type,
			value: &mut self.value,
		}
	}

	pub fn named(self, name: impl Into<String>) -> Entry {
		Entry::new(name, self.value)
	}
}

impl From<Entry> for Tag {
	fn from(e: Entry) -> Self {
		Tag::new(e.value)
	}
}

impl From<Tag> for Entry {
	fn from(t: Tag) -> Self {
		t.named("")
	}
}

impl ListItems {
	pub fn kind(&self) -> u8 {
		match self {
			ListItems::end(_) => 0,
			ListItems::byte(_) => 1,
			ListItems::short(_) => 2,
			ListItems::int(_) => 3,
			ListItems::long(_) => 4,
			ListItems::float(_) => 5,
			ListItems::double(_) => 6,
			ListItems::byteArray(_) => 7,
			ListItems::string(_) => 8,
			ListItems::list(_) => 9,
			ListItems::compound(_) => 10,
			ListItems::intArray(_) => 11,
			ListItems::longArray(_) => 12,
		}
	}

	pub fn len(&self) -> usize {
		match self {
			ListItems::end(_) => 0,
			ListItems::byte(v) => v.len(),
			ListItems::short(v) => v.len(),
			ListItems::int(v) => v.len(),
			ListItems::long(v) => v.len(),
			ListItems::float(v) => v.len(),
			ListItems::double(v) => v.len(),
			ListItems::byteArray(v) => v.len(),
			ListItems::string(v) => v.len(),
			ListItems::list(v) => v.len(),
			ListItems::compound(v) => v.len(),
			ListItems::intArray(v) => v.len(),
			ListItems::longArray(v) => v.len(),
		}
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

impl List {
	pub fn new(items: ListItems) -> List {
		List {
			r#type: items.kind(),
			items,
		}
	}

	pub fn of(values: Vec<Payload>) -> List {
		macro_rules! collect {
			($arm:ident) => {
				ListItems::$arm(
					values
						.into_iter()
						.filter_map(|v| {
							if let Payload::$arm(x) = v {
								Some(x)
							} else {
								None
							}
						})
						.collect(),
				)
			};
		}
		List::new(match values.first().map_or(0, Payload::kind) {
			1 => collect!(byte),
			2 => collect!(short),
			3 => collect!(int),
			4 => collect!(long),
			5 => collect!(float),
			6 => collect!(double),
			7 => collect!(byteArray),
			8 => collect!(string),
			9 => collect!(list),
			10 => collect!(compound),
			11 => collect!(intArray),
			12 => collect!(longArray),
			_ => ListItems::end(0),
		})
	}

	pub fn set(&mut self, items: ListItems) {
		*self = List::new(items);
	}

	pub fn len(&self) -> usize {
		self.items.len()
	}

	pub fn is_empty(&self) -> bool {
		self.items.is_empty()
	}

	pub fn values(&self) -> Vec<Payload> {
		self.clone().into_values()
	}

	pub fn into_values(self) -> Vec<Payload> {
		fn all<T>(v: Vec<T>, f: fn(T) -> Payload) -> Vec<Payload> {
			v.into_iter().map(f).collect()
		}
		match self.items {
			ListItems::end(_) => Vec::new(),
			ListItems::byte(v) => all(v, Payload::byte),
			ListItems::short(v) => all(v, Payload::short),
			ListItems::int(v) => all(v, Payload::int),
			ListItems::long(v) => all(v, Payload::long),
			ListItems::float(v) => all(v, Payload::float),
			ListItems::double(v) => all(v, Payload::double),
			ListItems::byteArray(v) => all(v, Payload::byteArray),
			ListItems::string(v) => all(v, Payload::string),
			ListItems::list(v) => all(v, Payload::list),
			ListItems::compound(v) => all(v, Payload::compound),
			ListItems::intArray(v) => all(v, Payload::intArray),
			ListItems::longArray(v) => all(v, Payload::longArray),
		}
	}

	pub fn strings(&self) -> Option<&[String]> {
		match &self.items {
			ListItems::string(s) => Some(s),
			_ => None,
		}
	}

	pub fn strings_mut(&mut self) -> Option<&mut Vec<String>> {
		match &mut self.items {
			ListItems::string(s) => Some(s),
			_ => None,
		}
	}

	pub fn compounds(&self) -> &[Vec<Entry>] {
		match &self.items {
			ListItems::compound(c) => c,
			_ => &[],
		}
	}

	pub fn compounds_mut(&mut self) -> Option<&mut Vec<Vec<Entry>>> {
		match &mut self.items {
			ListItems::compound(c) => Some(c),
			_ => None,
		}
	}
}

pub trait CompoundExt {
	fn field(&self, name: &str) -> Option<&Payload>;
	fn field_mut(&mut self, name: &str) -> Option<PayloadMut<'_>>;
	fn compound_mut(&mut self, name: &str) -> Option<&mut Vec<Entry>>;
	fn compounds_mut(&mut self, name: &str) -> Option<&mut Vec<Vec<Entry>>>;
	fn rename(&mut self, from: &str, to: &str);
}

impl CompoundExt for [Entry] {
	fn field(&self, name: &str) -> Option<&Payload> {
		self.iter().find(|e| e.name == name).map(|e| &e.value)
	}

	fn field_mut(&mut self, name: &str) -> Option<PayloadMut<'_>> {
		self.iter_mut()
			.find(|e| e.name == name)
			.map(Entry::value_mut)
	}

	fn compound_mut(&mut self, name: &str) -> Option<&mut Vec<Entry>> {
		self.iter_mut()
			.find(|e| e.name == name)?
			.value
			.as_compound_mut()
	}

	fn compounds_mut(&mut self, name: &str) -> Option<&mut Vec<Vec<Entry>>> {
		self.iter_mut()
			.find(|e| e.name == name)?
			.value
			.compounds_mut()
	}

	fn rename(&mut self, from: &str, to: &str) {
		if let Some(e) = self.iter_mut().find(|e| e.name == from) {
			e.name = to.to_owned();
		}
	}
}

pub trait CompoundVecExt {
	fn put(&mut self, name: &str, value: impl Into<Payload>);
	fn put_last(&mut self, name: &str, value: impl Into<Payload>);
	fn take(&mut self, name: &str) -> Option<Payload>;
	fn compound_or_insert(&mut self, name: &str) -> &mut Vec<Entry>;
}

impl CompoundVecExt for Vec<Entry> {
	fn put(&mut self, name: &str, value: impl Into<Payload>) {
		match self.iter_mut().find(|e| e.name == name) {
			Some(e) => e.set(value),
			None => self.push(Entry::new(name, value)),
		}
	}

	fn put_last(&mut self, name: &str, value: impl Into<Payload>) {
		self.take(name);
		self.push(Entry::new(name, value));
	}

	fn take(&mut self, name: &str) -> Option<Payload> {
		let at = self.iter().position(|e| e.name == name)?;
		Some(self.remove(at).value)
	}

	fn compound_or_insert(&mut self, name: &str) -> &mut Vec<Entry> {
		if !matches!(self.field(name), Some(Payload::compound(_))) {
			self.put(name, Vec::new());
		}
		self.compound_mut(name).expect("compound was just put")
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn sample() -> Vec<Entry> {
		vec![
			Entry::new("a", 1i32),
			Entry::new("b", "text"),
			Entry::new("c", vec![Entry::new("d", 2i8)]),
		]
	}

	#[test]
	fn constructors_set_the_type_byte() {
		let cases: [(Payload, u8); 13] = [
			(Payload::end, 0),
			(1i8.into(), 1),
			(1i16.into(), 2),
			(1i32.into(), 3),
			(1i64.into(), 4),
			(1f32.into(), 5),
			(1f64.into(), 6),
			(Payload::byteArray(vec![1]), 7),
			("s".into(), 8),
			(ListItems::int(vec![1]).into(), 9),
			(Vec::<Entry>::new().into(), 10),
			(Payload::intArray(vec![1]), 11),
			(Payload::longArray(vec![1]), 12),
		];
		for (value, kind) in cases {
			assert_eq!(Entry::new("x", value.clone()).r#type, kind);
			assert_eq!(Tag::new(value.clone()).r#type, kind);
			assert_eq!(value.kind(), kind);
		}
		assert_eq!(List::new(ListItems::compound(vec![])).r#type, 10);
		assert_eq!(List::of(vec![]).r#type, 0);
		assert_eq!(
			List::of(vec![Payload::string("a".into()), 1i32.into(), "b".into()]),
			List::new(ListItems::string(vec!["a".into(), "b".into()]))
		);
	}

	#[test]
	fn mutation_keeps_the_type_byte() {
		let mut c = sample();
		*c.field_mut("a").unwrap() = "now a string".into();
		assert_eq!(c[0].r#type, 8);
		c.put("b", 5i64);
		assert_eq!((c[1].r#type, &c[1].value), (4, &Payload::long(5)));
		if let Some(Payload::compound(inner)) = c.field_mut("c").as_deref_mut() {
			inner.put("d", 3i16);
		}
		assert_eq!(
			c.field("c").and_then(|p| p.field("d")),
			Some(&Payload::short(3))
		);
		let mut e = Entry::new("e", 1i32);
		e.set(Payload::double(1.5));
		assert_eq!(e.r#type, 6);
		*e.value_mut() = Payload::compound(vec![]);
		assert_eq!(e.r#type, 10);
		let mut t = Tag::new(1i32);
		*t.value_mut() = Payload::end;
		assert_eq!(t.r#type, 0);
		let mut l = List::new(ListItems::int(vec![1]));
		l.set(ListItems::string(vec![]));
		assert_eq!(l.r#type, 8);
	}

	#[test]
	fn put_keeps_order_and_put_last_moves() {
		let mut c = sample();
		c.put("a", 9i32);
		c.put("z", 0i32);
		assert_eq!(
			c.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
			["a", "b", "c", "z"]
		);
		c.put_last("a", 1i32);
		assert_eq!(
			c.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
			["b", "c", "z", "a"]
		);
		assert_eq!(c.take("b"), Some(Payload::string("text".into())));
		assert_eq!(c.take("b"), None);
		c.rename("z", "y");
		assert_eq!(c.field("y"), Some(&Payload::int(0)));
		c.compound_mut("c").unwrap().push(Entry::new("x", 1i32));
		assert!(c.compounds_mut("c").is_none());
		c.put("s", 1i8);
		c.compound_or_insert("s").push(Entry::new("in", 1i8));
		c.compound_or_insert("s").push(Entry::new("again", 2i8));
		c.compound_or_insert("new");
		assert_eq!(
			c.field("s"),
			Some(&Payload::compound(vec![
				Entry::new("in", 1i8),
				Entry::new("again", 2i8)
			]))
		);
		assert_eq!(c.last(), Some(&Entry::new("new", Vec::new())));
		c.put("l", ListItems::compound(vec![]));
		c.compounds_mut("l")
			.unwrap()
			.push(vec![Entry::new("n", 1i8)]);
		assert_eq!(
			c.field("l").map(Payload::compounds),
			Some(&[vec![Entry::new("n", 1i8)]][..])
		);
		assert_eq!(
			c.field("c").and_then(Payload::as_compound).map(Vec::len),
			Some(2)
		);
	}

	#[test]
	fn payload_accessors() {
		let mut p = Payload::compound(sample());
		assert_eq!(p.field("b").and_then(Payload::as_str), Some("text"));
		assert_eq!(p.field("a").and_then(Payload::as_i64), Some(1));
		p.put("f", Payload::float(2.9));
		assert_eq!(p.field("f").and_then(Payload::as_i64), None);
		assert_eq!(p.field("f").and_then(Payload::to_i32), Some(2));
		assert_eq!(p.field("f").and_then(Payload::as_f64), Some(2.9f32 as f64));
		assert_eq!(Payload::long(1 << 33 | 5).to_i32(), Some(5));
		assert_eq!(p.take("a"), Some(Payload::int(1)));
		let mut s = Payload::string("x".into());
		s.put("a", 1i32);
		assert_eq!(s.take("a"), None);
		let u = 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210u128;
		assert_eq!(Payload::uuid(u).as_uuid(), Some(u));
		assert_eq!(Payload::intArray(vec![1, 2]).as_uuid(), None);
	}

	#[test]
	fn list_views() {
		let mut p: Payload = ListItems::compound(vec![vec![Entry::new("a", 1i32)]]).into();
		assert_eq!(p.compounds().len(), 1);
		p.compounds_mut().unwrap().push(vec![]);
		assert_eq!(
			p.values(),
			vec![
				Payload::compound(vec![Entry::new("a", 1i32)]),
				Payload::compound(vec![])
			]
		);
		assert!(Payload::int(1).compounds().is_empty());
		let l = List::new(ListItems::string(vec!["a".into()]));
		assert_eq!(l.strings(), Some(&["a".to_owned()][..]));
		assert_eq!((l.len(), l.is_empty()), (1, false));
		assert!(List::new(ListItems::end(0)).into_values().is_empty());
	}

	#[test]
	fn root_creates_or_replaces_a_compound() {
		let mut tag = None;
		Entry::root(&mut tag).push(Entry::new("a", 1i32));
		assert_eq!(tag, Some(Entry::new("", vec![Entry::new("a", 1i32)])));
		let mut tag = Some(Entry::new("x", 1i32));
		assert!(Entry::root(&mut tag).is_empty());
		assert_eq!(tag, Some(Entry::new("", Vec::new())));
		let t: Tag = Entry::new("n", 1i8).into();
		assert_eq!(Entry::from(t), Entry::new("", 1i8));
	}
}
