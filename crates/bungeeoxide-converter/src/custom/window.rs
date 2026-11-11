use std::collections::{BTreeMap, HashMap, HashSet};

use bungeeoxide_nbt::Payload;

use super::chunks::{
	Biomes, BlockEntity, Column, Heightmap, Light, Nibbles, Section, BLOCKS, NIBBLES,
};
use crate::gen::convert::Version;

const LAYER: usize = 256;
const LIGHT: usize = 18;
const RELATIVE: i8 = 0x1f;
const UNANSWERED: u32 = 100;
pub(crate) const SYNTHETIC: i32 = 0x4e00_0000;

pub(crate) trait Dialect: 'static {
	type Painting: Clone + Send + 'static;
	const AT: Version;
	fn air() -> u32;
	fn is_air(state: u32) -> bool;
	fn heightmaps(
		maps: &[Heightmap],
		_shift: i32,
		_height: i32,
		_window: i32,
		_floor: &[bool],
	) -> Vec<Heightmap> {
		maps.to_vec()
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Teleport {
	pub x: f64,
	pub y: f64,
	pub z: f64,
	pub yaw: f32,
	pub pitch: f32,
	pub flags: i8,
	pub id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Moved {
	pub id: i32,
	pub x: f64,
	pub y: f64,
	pub z: f64,
	pub yaw: i8,
	pub pitch: i8,
	pub ground: bool,
}

pub(crate) enum Emit<P> {
	Column(Column),
	Teleport(Teleport),
	Vehicle {
		x: f64,
		y: f64,
		z: f64,
		yaw: f32,
		pitch: f32,
	},
	Entity(Moved),
	Painting(i32, P),
	Spawn(i32, i32, i32),
}

struct Cached {
	sections: Vec<Option<Section>>,
	heightmaps: Option<Vec<Heightmap>>,
	biomes: Biomes,
	entities: Vec<BlockEntity>,
}

#[derive(Default)]
struct Slot {
	chunk: Option<Cached>,
	sky: Vec<Nibbles>,
	block: Vec<Nibbles>,
	shown: u16,
}

enum Tracked<P> {
	Entity {
		x: f64,
		y: f64,
		z: f64,
		yaw: i8,
		pitch: i8,
		ground: bool,
	},
	Painting(P),
}

pub(crate) struct Window<D: Dialect> {
	min_y: i32,
	height: i32,
	window: i32,
	stride: i32,
	keep: i32,
	enter: i32,
	view: i32,
	input: i32,
	settled: bool,
	awaiting: Option<i32>,
	unanswered: u32,
	pending: Vec<(i32, i32)>,
	synthetic: i32,
	player: (f64, f64, f64),
	reported: f64,
	spawn: Option<(i32, i32, i32)>,
	columns: HashMap<(i32, i32), Slot>,
	stale: HashSet<(i32, i32)>,
	loaded: HashSet<(i32, i32)>,
	entities: HashMap<i32, Tracked<D::Painting>>,
}

fn tuning(window: i32) -> (i32, i32, i32) {
	match window {
		..=128 => (32, 28, 32),
		_ => (128, 32, 64),
	}
}

impl<D: Dialect> Default for Window<D> {
	fn default() -> Self {
		let (stride, keep, enter) = tuning(256);
		Self {
			min_y: 0,
			height: 256,
			window: 256,
			stride,
			keep,
			enter,
			view: 0,
			input: 0,
			settled: false,
			awaiting: None,
			unanswered: 0,
			pending: Vec::new(),
			synthetic: 0,
			player: (0.0, 0.0, 0.0),
			reported: 0.0,
			spawn: None,
			columns: HashMap::new(),
			stale: HashSet::new(),
			loaded: HashSet::new(),
			entities: HashMap::new(),
		}
	}
}

impl<D: Dialect> Window<D> {
	pub(crate) fn reset(&mut self, min_y: i32, height: i32, window: i32) {
		let (stride, keep, enter) = tuning(window);
		*self = Self {
			min_y,
			height: height.max(16),
			window,
			stride,
			keep,
			enter,
			synthetic: self.synthetic,
			..Self::default()
		};
		self.view = self.preferred(f64::NAN, false);
		self.input = self.view;
	}

	pub(crate) fn view(&self) -> i32 {
		self.view
	}

	pub(crate) fn input(&self) -> i32 {
		self.input
	}

	pub(crate) fn window(&self) -> i32 {
		self.window
	}

	pub(crate) fn tall(&self) -> bool {
		self.height > self.window
	}

	pub(crate) fn inside(&self, y: i32) -> bool {
		(self.view..self.view + self.window).contains(&y)
	}

	pub(crate) fn shown(&self, x: i32, z: i32) -> bool {
		!self.stale.contains(&(x, z))
	}

	fn sections(&self) -> usize {
		(self.height / 16) as usize
	}

	fn shown_sections(&self) -> usize {
		(self.window / 16) as usize
	}

	fn floored(&self) -> bool {
		self.tall() && self.view > self.min_y
	}

	fn first_section(&self) -> usize {
		((self.view - self.min_y) / 16) as usize
	}

	pub(crate) fn bases(&self) -> Vec<i32> {
		let (low, high) = (self.min_y, self.min_y + self.height - self.window);
		if high <= low {
			return vec![low];
		}
		let anchor = if (low..=high).contains(&0) {
			low + (-low).div_euclid(16) * 16
		} else {
			low
		};
		let mut bases = vec![low, high];
		let mut b = anchor;
		while b > low {
			bases.push(b);
			b -= self.stride;
		}
		let mut b = anchor + self.stride;
		while b < high {
			bases.push(b);
			b += self.stride;
		}
		bases.sort_unstable();
		bases.dedup();
		bases
	}

	fn preferred(&self, y: f64, sticky: bool) -> i32 {
		let bases = self.bases();
		let identity = bases.contains(&0);
		if !y.is_finite() {
			return if identity { 0 } else { bases[0] };
		}
		let fits = |b: i32, margin: i32| {
			let bottom = if Some(&b) == bases.first() {
				f64::NEG_INFINITY
			} else {
				(b + margin) as f64
			};
			let top = if Some(&b) == bases.last() {
				f64::INFINITY
			} else {
				(b + self.window - margin) as f64
			};
			(bottom..=top).contains(&y)
		};
		if identity && fits(0, self.enter) {
			return 0;
		}
		if sticky && bases.contains(&self.view) && fits(self.view, self.keep) {
			return self.view;
		}
		if identity && fits(0, self.keep) {
			return 0;
		}
		let centre = |b: &i32| (y - (b + self.window / 2) as f64).abs();
		bases
			.iter()
			.copied()
			.min_by(|a, b| centre(a).total_cmp(&centre(b)))
			.unwrap_or(self.min_y)
	}

	fn select(&self, y: f64, sticky: bool) -> i32 {
		if !y.is_finite() {
			return self.view;
		}
		self.preferred(y, sticky && self.settled)
	}

	fn teleport_id(&mut self) -> i32 {
		self.synthetic = (self.synthetic + 1) & 0x00ff_ffff;
		SYNTHETIC | self.synthetic
	}

	fn input_y(&mut self, y: f64) -> f64 {
		if let Some(&(id, base)) = self.pending.last() {
			let (old, new) = (y + self.input as f64, y + base as f64);
			if synthetic(id) && (new - self.reported).abs() < (old - self.reported).abs() {
				self.input = base;
				self.pending.clear();
			}
		}
		self.reported = y + self.input as f64;
		self.reported
	}

	pub(crate) fn confirmed(&mut self, id: i32) -> bool {
		if let Some(at) = self.pending.iter().position(|(p, _)| *p == id) {
			self.input = self.pending[at].1;
			self.pending.drain(..=at);
		}
		if self.awaiting == Some(id) {
			self.awaiting = None;
		}
		synthetic(id)
	}

	pub(crate) fn client_moved(
		&mut self,
		x: f64,
		y: f64,
		z: f64,
		vehicle: Option<(f32, f32)>,
	) -> (f64, Vec<Emit<D::Painting>>) {
		let y = self.input_y(y);
		if self.awaiting.is_some() {
			self.unanswered += 1;
			if self.unanswered > UNANSWERED {
				self.awaiting = None;
			}
		}
		if !self.settled || self.awaiting.is_some() {
			return (y, vec![]);
		}
		self.player = (x, y, z);
		if !self.tall() {
			return (y, vec![]);
		}
		let target = self.select(y, true);
		if target == self.view {
			return (y, vec![]);
		}
		let id = self.teleport_id();
		let mut middle = vec![Emit::Teleport(Teleport {
			x: 0.0,
			y: (self.view - target) as f64,
			z: 0.0,
			yaw: 0.0,
			pitch: 0.0,
			flags: RELATIVE,
			id,
		})];
		if let Some((yaw, pitch)) = vehicle {
			middle.push(Emit::Vehicle {
				x,
				y: y - target as f64,
				z,
				yaw,
				pitch,
			});
		}
		let out = self.relocate(target, middle);
		self.pending.push((id, target));
		(y, out)
	}

	pub(crate) fn server_teleport(&mut self, t: Teleport) -> Vec<Emit<D::Painting>> {
		let at = |flag: i8, base: f64, value: f64| {
			if t.flags & flag != 0 {
				base + value
			} else {
				value
			}
		};
		let (x, y, z) = (
			at(1, self.player.0, t.x),
			at(2, self.player.1, t.y),
			at(4, self.player.2, t.z),
		);
		let near = (y - self.player.1).abs() <= self.keep as f64;
		self.player = (x, y, z);
		let target = if self.tall() {
			self.select(y, near)
		} else {
			self.view
		};
		self.settled = true;
		self.awaiting = Some(t.id);
		self.unanswered = 0;
		let moving = target != self.view;
		let position = Emit::Teleport(Teleport {
			y: if moving || t.flags & 2 == 0 {
				y - target as f64
			} else {
				t.y
			},
			flags: if moving { t.flags & !2 } else { t.flags },
			..t
		});
		let out = if moving {
			self.relocate(target, vec![position])
		} else {
			vec![position]
		};
		if self.view != self.input {
			self.pending.push((t.id, self.view));
		}
		out
	}

	pub(crate) fn vehicle_moved(&mut self, x: f64, y: f64, z: f64) -> f64 {
		self.player = (x, y, z);
		y - self.view as f64
	}

	pub(crate) fn spawn(&mut self, x: i32, y: i32, z: i32) -> i32 {
		self.spawn = Some((x, y, z));
		y - self.view
	}

	pub(crate) fn moved(
		&mut self,
		entity: i32,
		x: f64,
		y: f64,
		z: f64,
		look: Option<(i8, i8)>,
		ground: Option<bool>,
	) {
		if !self.tall() {
			return;
		}
		let e = self.entities.entry(entity).or_insert(Tracked::Entity {
			x,
			y,
			z,
			yaw: 0,
			pitch: 0,
			ground: false,
		});
		if let Tracked::Entity {
			x: ex,
			y: ey,
			z: ez,
			yaw,
			pitch,
			ground: on,
		} = e
		{
			(*ex, *ey, *ez) = (x, y, z);
			if let Some((a, b)) = look {
				(*yaw, *pitch) = (a, b);
			}
			if let Some(g) = ground {
				*on = g;
			}
		}
	}

	pub(crate) fn moved_by(
		&mut self,
		entity: i32,
		d: (f64, f64, f64),
		look: Option<(i8, i8)>,
		ground: bool,
	) {
		if let Some(Tracked::Entity { x, y, z, .. }) = self.entities.get(&entity) {
			let (x, y, z) = (x + d.0, y + d.1, z + d.2);
			self.moved(entity, x, y, z, look, Some(ground));
		}
	}

	pub(crate) fn looked(&mut self, entity: i32, look: (i8, i8), ground: bool) {
		if let Some(Tracked::Entity {
			yaw,
			pitch,
			ground: on,
			..
		}) = self.entities.get_mut(&entity)
		{
			(*yaw, *pitch, *on) = (look.0, look.1, ground);
		}
	}

	pub(crate) fn painting(&mut self, entity: i32, p: D::Painting) {
		if self.tall() {
			self.entities.insert(entity, Tracked::Painting(p));
		}
	}

	pub(crate) fn forget(&mut self, entity: i32) {
		self.entities.remove(&entity);
	}

	fn slot(&mut self, x: i32, z: i32) -> Option<&mut Slot> {
		if !self.tall() {
			return None;
		}
		let n = self.sections() + 2;
		let c = self.columns.entry((x, z)).or_default();
		c.sky.resize(n, Nibbles::Missing);
		c.block.resize(n, Nibbles::Missing);
		Some(c)
	}

	pub(crate) fn unload(&mut self, x: i32, z: i32) {
		self.columns.remove(&(x, z));
		self.stale.remove(&(x, z));
		self.loaded.remove(&(x, z));
	}

	pub(crate) fn forget_columns(&mut self) -> Vec<(i32, i32)> {
		let mut keys: Vec<(i32, i32)> = self.loaded.drain().collect();
		keys.sort_unstable();
		self.columns.clear();
		self.stale.clear();
		keys
	}

	pub(crate) fn column(&mut self, mut col: Column) -> Option<Column> {
		let (x, z) = (col.x, col.z);
		let light = col.light.take().map(|l| {
			let shown = self.render_light(&l.sky, &l.block, l.trust, false);
			if let Some(s) = self.slot(x, z) {
				merge(&mut s.sky, &l.sky);
				merge(&mut s.block, &l.block);
			}
			shown
		});
		if !col.full {
			return self.partial(col, light);
		}
		col.sections.resize(self.sections(), None);
		let cached = Cached {
			sections: col.sections,
			heightmaps: col.heightmaps,
			biomes: col.biomes,
			entities: col.entities,
		};
		let shown = self.columns.get(&(x, z)).map_or(0, |c| c.shown);
		let (mut out, shown) = self.render(x, z, &cached, shown);
		out.light = light;
		self.stale.remove(&(x, z));
		self.loaded.insert((x, z));
		if let Some(s) = self.slot(x, z) {
			s.chunk = Some(cached);
			s.shown = shown;
		}
		Some(out)
	}

	fn partial(&mut self, col: Column, light: Option<Light>) -> Option<Column> {
		let (x, z) = (col.x, col.z);
		let (s0, n) = (self.first_section(), self.shown_sections());
		let present: Vec<usize> = col
			.sections
			.iter()
			.enumerate()
			.filter(|(_, s)| s.is_some())
			.map(|(i, _)| i)
			.collect();
		let floored = self.floored();
		if let Some(Some(cached)) = self.columns.get_mut(&(x, z)).map(|s| s.chunk.as_mut()) {
			for (i, s) in col.sections.into_iter().enumerate() {
				if let (Some(s), Some(slot)) = (s, cached.sections.get_mut(i)) {
					*slot = Some(s);
				}
			}
			let wanted: Vec<usize> = (0..n)
				.filter(|k| {
					present.contains(&(s0 + k))
						|| (*k == 0 && floored && present.iter().any(|s| *s <= s0))
				})
				.collect();
			if wanted.is_empty() {
				return None;
			}
			let stale = self.stale.contains(&(x, z));
			let slot = self.columns.get(&(x, z))?;
			let (rendered, _) = self.render(x, z, slot.chunk.as_ref()?, 0);
			if stale {
				return None;
			}
			let mut out = Column::new(x, z, false, D::AT);
			out.sections = rendered
				.sections
				.into_iter()
				.enumerate()
				.map(|(k, s)| s.filter(|_| wanted.contains(&k)))
				.collect();
			out.light = light;
			if let Some(slot) = self.columns.get_mut(&(x, z)) {
				slot.shown |= wanted.iter().fold(0, |m, k| m | 1 << k);
			}
			return Some(out);
		}
		let mut out = Column::new(x, z, false, D::AT);
		out.sections = (0..n)
			.map(|k| col.sections.get(s0 + k).cloned().flatten())
			.collect();
		if out.sections.iter().all(Option::is_none) {
			return None;
		}
		out.light = light;
		Some(out)
	}

	pub(crate) fn light(&mut self, x: i32, z: i32, light: &Light) -> Light {
		let out = self.render_light(&light.sky, &light.block, light.trust, false);
		if let Some(s) = self.slot(x, z) {
			merge(&mut s.sky, &light.sky);
			merge(&mut s.block, &light.block);
		}
		out
	}

	fn state(&self, cached: &Cached, x: i32, y: i32, z: i32) -> u32 {
		match usize::try_from((y - self.min_y).div_euclid(16))
			.ok()
			.and_then(|s| cached.sections.get(s))
		{
			Some(Some(s)) => s.states[index(x, y, z)],
			_ => D::air(),
		}
	}

	fn set_block(&mut self, x: i32, y: i32, z: i32, state: u32) {
		let s = (y - self.min_y).div_euclid(16);
		let Some(Some(chunk)) = self
			.columns
			.get_mut(&(x >> 4, z >> 4))
			.map(|c| c.chunk.as_mut())
		else {
			return;
		};
		let Some(slot) = usize::try_from(s)
			.ok()
			.and_then(|s| chunk.sections.get_mut(s))
		else {
			return;
		};
		slot.get_or_insert_with(|| Section::filled(D::air(), None))
			.states[index(x, y, z)] = state;
	}

	fn floor_state(&self, cached: &Cached, x: i32, z: i32) -> u32 {
		(self.min_y..=self.view)
			.rev()
			.map(|y| self.state(cached, x, y, z))
			.find(|s| !D::is_air(*s))
			.unwrap_or_else(D::air)
	}

	fn visible(&self, x: i32, y: i32, z: i32, state: u32) -> Option<(i32, u32)> {
		if self.stale.contains(&(x >> 4, z >> 4)) {
			return None;
		}
		if self.floored() && y <= self.view {
			return match self
				.columns
				.get(&(x >> 4, z >> 4))
				.and_then(|c| c.chunk.as_ref())
			{
				Some(cached) => Some((0, self.floor_state(cached, x, z))),
				None => (y == self.view).then_some((0, state)),
			};
		}
		self.inside(y).then_some((y - self.view, state))
	}

	pub(crate) fn block_changed(
		&mut self,
		x: i32,
		y: i32,
		z: i32,
		state: u32,
	) -> Option<(i32, u32)> {
		self.set_block(x, y, z, state);
		self.visible(x, y, z, state)
	}

	pub(crate) fn blocks_changed(
		&mut self,
		changes: &[(i32, i32, i32, u32)],
	) -> Vec<(i32, i32, i32, u32)> {
		for &(x, y, z, state) in changes {
			self.set_block(x, y, z, state);
		}
		let mut shown: BTreeMap<(i32, i32, i32), u32> = BTreeMap::new();
		for &(x, y, z, state) in changes {
			if let Some((cy, state)) = self.visible(x, y, z, state) {
				shown.insert((cy, x, z), state);
			}
		}
		shown
			.into_iter()
			.map(|((y, x, z), s)| (x, y, z, s))
			.collect()
	}

	pub(crate) fn block_entity(
		&mut self,
		x: i32,
		y: i32,
		z: i32,
		entity: Option<BlockEntity>,
	) -> Option<(i32, Option<BlockEntity>)> {
		if let Some(Some(chunk)) = self
			.columns
			.get_mut(&(x >> 4, z >> 4))
			.map(|c| c.chunk.as_mut())
		{
			chunk.entities.retain(|e| (e.x, e.y, e.z) != (x, y, z));
			if let Some(e) = &entity {
				chunk.entities.push(e.clone());
			}
		}
		if !self.inside(y) || self.stale.contains(&(x >> 4, z >> 4)) {
			return None;
		}
		Some((y - self.view, entity.map(|e| shifted(e, self.view))))
	}

	fn render(&self, x: i32, z: i32, cached: &Cached, shown: u16) -> (Column, u16) {
		let (s0, n) = (self.first_section(), self.shown_sections());
		let air = D::air();
		let mut window: Vec<Option<Section>> = (0..n)
			.map(|k| cached.sections.get(s0 + k).cloned().flatten())
			.collect();
		let mut floor = [false; LAYER];
		if self.floored() {
			let mut states = window[0].as_ref().map(|s| s.states.clone());
			for (i, f) in floor.iter_mut().enumerate() {
				let current = states.as_ref().map_or(air, |s| s[i]);
				if !D::is_air(current) {
					continue;
				}
				let below =
					self.floor_state(cached, x * 16 + (i & 15) as i32, z * 16 + (i >> 4) as i32);
				if !D::is_air(below) {
					states.get_or_insert_with(|| vec![air; BLOCKS])[i] = below;
					*f = true;
				}
			}
			if floor.iter().any(|f| *f) {
				window[0] = states.map(|s| Section::new(s, None));
			}
		}
		let mut mask = 0u16;
		let sections = window
			.into_iter()
			.enumerate()
			.map(|(k, s)| match s {
				Some(s) => {
					mask |= 1 << k;
					Some(Section::new(s.states, None))
				}
				None if shown & (1 << k) != 0 => Some(Section::filled(air, Some(0))),
				None => None,
			})
			.collect();
		let biomes = match &cached.biomes {
			Biomes::Cells(c) => Biomes::Cells(
				(0..1024)
					.map(|i| c.get(s0 * 64 + i).or(c.last()).copied().unwrap_or(0))
					.collect(),
			),
			other => other.clone(),
		};
		let mut out = Column::new(x, z, true, D::AT);
		out.sections = sections;
		out.biomes = biomes;
		out.heightmaps = cached
			.heightmaps
			.as_ref()
			.map(|m| D::heightmaps(m, self.view - self.min_y, self.height, self.window, &floor));
		out.entities = cached
			.entities
			.iter()
			.filter(|e| self.inside(e.y))
			.map(|e| shifted(e.clone(), self.view))
			.collect();
		(out, mask)
	}

	fn render_light(&self, sky: &[Nibbles], block: &[Nibbles], trust: bool, fill: bool) -> Light {
		let s0 = self.first_section();
		let window = |arrays: &[Nibbles], open: bool| -> Vec<Nibbles> {
			(0..LIGHT)
				.map(|k| match arrays.get(s0 + k) {
					Some(Nibbles::Data(a)) => Nibbles::Data(a.clone()),
					Some(Nibbles::Empty) if fill => Nibbles::Data(vec![0; NIBBLES]),
					Some(Nibbles::Empty) => Nibbles::Empty,
					_ if fill && open => Nibbles::Data(sky_below(arrays, s0 + k)),
					_ if fill => Nibbles::Data(vec![0; NIBBLES]),
					_ => Nibbles::Missing,
				})
				.collect()
		};
		let sky = window(sky, true);
		let block = window(block, false);
		let has_sky = sky.iter().any(|n| !matches!(n, Nibbles::Missing));
		Light {
			sky,
			block,
			has_sky,
			trust,
		}
	}

	fn render_slot(&mut self, key: (i32, i32)) -> Option<Column> {
		let slot = self.columns.get(&key)?;
		let cached = slot.chunk.as_ref()?;
		let light = self.render_light(&slot.sky, &slot.block, true, true);
		let (mut col, shown) = self.render(key.0, key.1, cached, slot.shown);
		col.light = Some(light);
		if let Some(s) = self.columns.get_mut(&key) {
			s.shown = shown;
		}
		Some(col)
	}

	fn around(&self) -> impl Fn(&(i32, i32)) -> i32 {
		let at = (
			(self.player.0 / 16.0).floor() as i32,
			(self.player.2 / 16.0).floor() as i32,
		);
		move |(x, z): &(i32, i32)| (x - at.0).abs().max((z - at.1).abs())
	}

	fn relocate(&mut self, base: i32, middle: Vec<Emit<D::Painting>>) -> Vec<Emit<D::Painting>> {
		self.view = base;
		let distance = self.around();
		let mut near: Vec<(i32, i32)> = Vec::new();
		for (key, c) in &self.columns {
			if c.chunk.is_none() {
				continue;
			}
			if distance(key) <= 1 {
				near.push(*key);
			} else {
				self.stale.insert(*key);
			}
		}
		near.sort_by_key(|k| (-distance(k), *k));
		let mut out = Vec::new();
		for key in near {
			self.stale.remove(&key);
			out.extend(self.render_slot(key).map(Emit::Column));
		}
		out.extend(middle);
		let mut ids: Vec<i32> = self.entities.keys().copied().collect();
		ids.sort_unstable();
		for id in ids {
			match &self.entities[&id] {
				Tracked::Entity {
					x,
					y,
					z,
					yaw,
					pitch,
					ground,
				} => out.push(Emit::Entity(Moved {
					id,
					x: *x,
					y: y - base as f64,
					z: *z,
					yaw: *yaw,
					pitch: *pitch,
					ground: *ground,
				})),
				Tracked::Painting(p) => out.push(Emit::Painting(id, p.clone())),
			}
		}
		if let Some((x, y, z)) = self.spawn {
			out.push(Emit::Spawn(x, y - base, z));
		}
		out
	}

	pub(crate) fn backlog(&mut self, limit: usize) -> Vec<Emit<D::Painting>> {
		if self.stale.is_empty() {
			return vec![];
		}
		let distance = self.around();
		let mut keys: Vec<(i32, i32)> = self.stale.iter().copied().collect();
		keys.sort_by_key(|k| (distance(k), *k));
		let mut out = Vec::new();
		for key in keys.into_iter().take(limit) {
			self.stale.remove(&key);
			out.extend(self.render_slot(key).map(Emit::Column));
		}
		out
	}
}

fn index(x: i32, y: i32, z: i32) -> usize {
	(((y & 15) << 8) | ((z & 15) << 4) | (x & 15)) as usize
}

fn synthetic(id: i32) -> bool {
	id & !0x00ff_ffff == SYNTHETIC
}

fn shifted(mut e: BlockEntity, view: i32) -> BlockEntity {
	e.y -= view;
	if let Some(Payload::Compound(c)) = e.nbt.as_mut().map(|t| &mut t.value) {
		for entry in c.iter_mut().filter(|entry| entry.name == "y") {
			if let Payload::Int(y) = &mut entry.value {
				*y -= view;
			}
		}
	}
	e
}

fn sky_below(arrays: &[Nibbles], at: usize) -> Vec<u8> {
	let known = arrays
		.iter()
		.skip(at + 1)
		.find(|n| !matches!(n, Nibbles::Missing));
	match known {
		Some(Nibbles::Data(a)) if a.len() >= NIBBLES => a[..LAYER / 2].repeat(16),
		Some(_) => vec![0; NIBBLES],
		None => vec![0xff; NIBBLES],
	}
}

fn merge(into: &mut [Nibbles], from: &[Nibbles]) {
	for (a, b) in into.iter_mut().zip(from) {
		if !matches!(b, Nibbles::Missing) {
			a.clone_from(b);
		}
	}
}
