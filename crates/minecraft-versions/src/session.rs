use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
	Player,
	Mob(u8),
	Object(u8),
}

#[derive(Debug)]
pub struct Outgoing {
	pub root: crate::gen::convert::Root,
	pub version: usize,
	pub data: Vec<u8>,
}

enum Payload {
	Bytes(Vec<u8>),
	Model(crate::custom::Virtual),
}

struct Queued {
	root: crate::gen::convert::Root,
	version: usize,
	payload: Payload,
}

#[derive(Default)]
pub struct Session {
	outgoing: Vec<Queued>,
	versions: Option<(usize, usize)>,
	teleport_id: i32,
	kinds: HashMap<i32, Kind>,
	vehicles: HashMap<i32, i32>,
	passengers: HashMap<i32, Vec<i32>>,
	states: HashMap<TypeId, Box<dyn Any + Send>>,
}

impl Session {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn set_versions(&mut self, server: usize, client: usize) {
		self.versions = Some((server, client));
	}

	pub fn server_version(&self) -> Option<usize> {
		self.versions.map(|(s, _)| s)
	}

	pub fn client_version(&self) -> Option<usize> {
		self.versions.map(|(_, c)| c)
	}

	pub fn next_teleport_id(&mut self) -> i32 {
		self.teleport_id = self.teleport_id.wrapping_add(1);
		self.teleport_id
	}

	pub fn spawned(&mut self, entity: i32, kind: Kind) {
		self.kinds.insert(entity, kind);
	}

	pub fn kind(&self, entity: i32) -> Kind {
		self.kinds.get(&entity).copied().unwrap_or(Kind::Player)
	}

	pub fn passengers(&self, vehicle: i32) -> Vec<i32> {
		self.passengers.get(&vehicle).cloned().unwrap_or_default()
	}

	pub fn mount(&mut self, passenger: i32, vehicle: i32) {
		self.vehicles.insert(passenger, vehicle);
		self.passengers.entry(vehicle).or_default().push(passenger);
	}

	pub fn dismount(&mut self, passenger: i32) -> Option<i32> {
		let vehicle = self.vehicles.remove(&passenger)?;
		if let Some(list) = self.passengers.get_mut(&vehicle) {
			list.retain(|p| *p != passenger);
		}
		Some(vehicle)
	}

	pub fn state<T: Default + Send + 'static>(&mut self) -> &mut T {
		self.states
			.entry(TypeId::of::<T>())
			.or_insert_with(|| Box::new(T::default()))
			.downcast_mut()
			.expect("states are keyed by their type")
	}

	pub fn send(&mut self, root: crate::gen::convert::Root, version: usize, data: Vec<u8>) {
		self.outgoing.push(Queued {
			root,
			version,
			payload: Payload::Bytes(data),
		});
	}

	pub fn send_model(
		&mut self,
		root: crate::gen::convert::Root,
		version: usize,
		model: crate::custom::Virtual,
	) {
		self.outgoing.push(Queued {
			root,
			version,
			payload: Payload::Model(model),
		});
	}

	pub fn take_outgoing(&mut self) -> Vec<Outgoing> {
		let mut out = Vec::new();
		for q in std::mem::take(&mut self.outgoing) {
			match q.payload {
				Payload::Bytes(data) => out.push(Outgoing {
					root: q.root,
					version: q.version,
					data,
				}),
				Payload::Model(model) => out.extend(crate::custom::chunks::deliver(
					q.root, q.version, model, self,
				)),
			}
		}
		out
	}

	pub fn reset(&mut self) {
		*self = Self {
			teleport_id: self.teleport_id,
			versions: self.versions,
			..Self::default()
		};
	}
}
