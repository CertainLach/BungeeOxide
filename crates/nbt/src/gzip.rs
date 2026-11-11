use crate::gen::entry;
use crate::pds::{Input, Output};
use crate::Entry;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fmt;
use std::io::{self, Read, Write};

#[derive(Debug)]
pub enum Error {
	Io(io::Error),
	Nbt(crate::pds::Error),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Error::Io(e) => write!(f, "{e}"),
			Error::Nbt(e) => write!(f, "{e:?}"),
		}
	}
}

impl std::error::Error for Error {}

pub fn compress(data: &[u8]) -> io::Result<Vec<u8>> {
	let mut z = GzEncoder::new(Vec::new(), Compression::default());
	z.write_all(data)?;
	z.finish()
}

pub const MAX_INFLATED: usize = 2 << 20;

pub fn decompress(data: &[u8]) -> io::Result<Vec<u8>> {
	let mut out = Vec::new();
	GzDecoder::new(data)
		.take(MAX_INFLATED as u64 + 1)
		.read_to_end(&mut out)?;
	if out.len() > MAX_INFLATED {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			format!("gzip NBT inflates past {MAX_INFLATED} bytes"),
		));
	}
	Ok(out)
}

pub fn write(entry: &Entry) -> Result<Vec<u8>, Error> {
	let mut o = Output::new();
	entry::serialize(entry, &mut o).map_err(Error::Nbt)?;
	compress(&o.into_inner()).map_err(Error::Io)
}

pub fn read(data: &[u8]) -> Result<Entry, Error> {
	let raw = decompress(data).map_err(Error::Io)?;
	entry::deserialize(&mut Input::new(&raw)).map_err(Error::Nbt)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{ListItems, Payload};

	fn sample() -> Entry {
		Entry::new(
			"",
			vec![
				Entry::new("id", "MobSpawner"),
				Entry::new("Delay", 20i16),
				Entry::new(
					"ench",
					ListItems::Compound(vec![vec![
						Entry::new("id", 16i16),
						Entry::new("lvl", 2i16),
					]]),
				),
				Entry::new("big", Payload::LongArray(vec![1, -1])),
			],
		)
	}

	#[test]
	fn nbt_round_trips() {
		let gz = write(&sample()).unwrap();
		assert_eq!(&gz[..2], [0x1f, 0x8b]);
		assert_eq!(read(&gz).unwrap(), sample());
	}

	#[test]
	fn bytes_round_trip() {
		for data in [&b""[..], b"hello", &[0u8; 4096]] {
			assert_eq!(decompress(&compress(data).unwrap()).unwrap(), data);
		}
	}

	#[test]
	fn output_is_stable() {
		assert_eq!(compress(b"x").unwrap(), compress(b"x").unwrap());
		assert_eq!(
			&compress(b"x").unwrap()[..10],
			[0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff]
		);
	}

	#[test]
	fn bad_input_fails() {
		assert!(matches!(read(b"not gzip"), Err(Error::Io(_))));
		assert!(matches!(
			read(&compress(&[10, 0, 0, 99]).unwrap()),
			Err(Error::Nbt(_))
		));
		assert!(decompress(&compress(b"hello").unwrap()[..8]).is_err());
	}

	#[test]
	fn bombs_stop_at_the_inflate_limit() {
		let bomb = compress(&vec![0; MAX_INFLATED + 1]).unwrap();
		assert!(bomb.len() < 4096);
		assert!(decompress(&bomb).is_err());
		assert_eq!(
			decompress(&compress(&vec![0; MAX_INFLATED]).unwrap())
				.unwrap()
				.len(),
			MAX_INFLATED
		);
	}
}
