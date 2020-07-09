use std::io::{Read, Result};

pub trait Packet: Sized {
	fn read<R: Read>(buf: &mut R) -> Result<Self>;
}
