macro_rules! arm {
	($name:ident, $root:ty, $arm:ident, $src:ty, |mut $p:ident, $cx:ident| $body:expr) => {
		pub fn $name(
			mut $p: $src,
			$cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<$root>> {
			Ok(vec![<$root>::$arm($body)])
		}
	};
	($name:ident, $root:ty, $arm:ident, $src:ty, |$p:ident, $cx:ident| $body:expr) => {
		pub fn $name(
			$p: $src,
			$cx: &mut crate::Session,
		) -> crate::pds::convert::Result<Vec<$root>> {
			Ok(vec![<$root>::$arm($body)])
		}
	};
}

mod chunk;
mod codec;
mod common;
mod hover;
mod items;
mod step_1_20;
mod step_1_20_2;
mod step_1_20_3;
mod step_1_20_5;
mod text;

pub use hover::{trails_text_1_20_3_to_1_20_5, trails_text_1_20_5_to_1_20_3};
pub use step_1_20::*;
pub use step_1_20_2::*;
pub use step_1_20_3::*;
pub use step_1_20_5::*;
