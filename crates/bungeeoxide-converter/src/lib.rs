#![allow(
	non_camel_case_types,
	non_snake_case,
	dead_code,
	unused_imports,
	unused_variables,
	unused_parens,
	unused_mut,
	unreachable_patterns,
	clippy::all
)]

pub mod custom;
pub mod gen;
pub mod languages;
pub mod registry;
pub mod session;
pub mod world;

pub use custom::chunks;

pub use session::{Outgoing, Session};

pub use gen::convert::{
	legacy_version_of, reachable, version_of, Root, Version, LEGACY_PROTOCOLS, PROTOCOLS, STEPS,
	VERSIONS,
};

pub const STACK_BUDGET: usize = 512 << 10;

pub fn convert(
	root: Root,
	from: Version,
	to: Version,
	data: &[u8],
	cx: &mut Session,
) -> pds::convert::Result<Vec<Vec<u8>>> {
	pds::stack::budget(STACK_BUDGET, || {
		gen::convert::convert(root, from, to, data, cx)
	})
}

pub fn inspect(root: Root, version: Version, data: &[u8]) -> Result<String, String> {
	pds::stack::budget(STACK_BUDGET, || gen::convert::inspect(root, version, data))
}

pub const CONFIGURATION: Version = Version::V1_20_2;

pub fn configuration_packets(cx: &mut Session) -> pds::convert::Result<Vec<Vec<u8>>> {
	custom::configuration_1_20_2(cx)
}
pub mod pds {
	pub use pds_bytes_runtime::*;
}
pub mod pds_native {
	pub use minecraft_natives::*;
	pub use pds_bytes_runtime::natives::{
		pds_bool, pds_f32, pds_f64, pds_i16, pds_i32, pds_i64, pds_i8, pds_u16, pds_u32, pds_u64,
		pds_u8, pds_void,
	};
}
