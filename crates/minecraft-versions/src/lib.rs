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

pub use session::{Outgoing, Session};

pub use gen::convert::{
	convert, index_of, inspect, legacy_version_of, reachable, version_of, Root, LEGACY_PROTOCOLS,
	PROTOCOLS, STEPS, VERSIONS,
};

pub const CONFIGURATION: &str = "1.20.2";

pub fn configuration_packets(cx: &mut Session) -> pds::convert::Result<Vec<Vec<u8>>> {
	custom::configuration_1_20_2(cx)
}
pub mod pds {
	pub use pds_runtime::*;
}
pub mod pds_native {
	pub use minecraft_natives::*;
	pub use pds_runtime::natives::{
		pds_bool, pds_f32, pds_f64, pds_i16, pds_i32, pds_i64, pds_i8, pds_u16, pds_u32, pds_u64,
		pds_u8, pds_void,
	};
}
