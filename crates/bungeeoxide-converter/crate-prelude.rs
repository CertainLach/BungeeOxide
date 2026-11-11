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
use ::pds_bytes_runtime as pds;
mod pds_native {
	pub use bungeeoxide_pds_natives::*;
	pub use pds_bytes_runtime::natives::{
		pds_bool, pds_f32, pds_f64, pds_i16, pds_i32, pds_i64, pds_i8, pds_u16, pds_u32, pds_u64,
		pds_u8, pds_void,
	};
}
