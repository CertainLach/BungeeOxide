use crate::custom::chunks::padding;
use crate::gen::convert::Version;
use crate::pds::convert::{unconvertible, Result};

pub struct Reader<'a> {
	data: &'a [u8],
	at: usize,
}

impl<'a> Reader<'a> {
	pub fn new(data: &'a [u8]) -> Self {
		Self { data, at: 0 }
	}

	pub fn done(&self) -> bool {
		self.at >= self.data.len()
	}

	fn rest(&self) -> &'a [u8] {
		&self.data[self.at.min(self.data.len())..]
	}

	fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
		let Some(b) = self.data.get(self.at..self.at + n) else {
			return unconvertible("chunk data ends early");
		};
		self.at += n;
		Ok(b)
	}

	pub fn u8(&mut self) -> Result<u8> {
		Ok(self.bytes(1)?[0])
	}

	pub fn varint(&mut self) -> Result<u32> {
		let mut v = 0u32;
		for shift in (0..35).step_by(7) {
			let b = self.u8()?;
			v |= ((b & 0x7f) as u32) << shift;
			if b & 0x80 == 0 {
				return Ok(v);
			}
		}
		unconvertible("varint too long in chunk data")
	}

	pub fn long(&mut self) -> Result<u64> {
		Ok(u64::from_be_bytes(
			self.bytes(8)?.try_into().expect("8 bytes"),
		))
	}
}

pub fn varint(out: &mut Vec<u8>, mut v: u32) {
	loop {
		if v & !0x7f == 0 {
			out.push(v as u8);
			return;
		}
		out.push((v as u8 & 0x7f) | 0x80);
		v >>= 7;
	}
}

pub fn bits_for(count: u32) -> u8 {
	(32 - count.saturating_sub(1).leading_zeros()).max(1) as u8
}

fn unpack(longs: &[u64], bits: u8, n: usize) -> Vec<u32> {
	let per = 64 / bits as usize;
	let mask = (1u64 << bits) - 1;
	(0..n)
		.map(|i| {
			longs
				.get(i / per)
				.map_or(0, |l| ((l >> ((i % per) * bits as usize)) & mask) as u32)
		})
		.collect()
}

fn pack(values: &[u32], bits: u8) -> Vec<u64> {
	let per = 64 / bits as usize;
	let mut out = vec![0u64; values.len().div_ceil(per)];
	for (i, v) in values.iter().enumerate() {
		out[i / per] |= (*v as u64) << ((i % per) * bits as usize);
	}
	out
}

fn container(
	r: &mut Reader,
	out: &mut Vec<u8>,
	entries: usize,
	indirect_max: u8,
	global: Option<u8>,
	map: &dyn Fn(u32) -> u32,
) -> Result<()> {
	let bits = r.u8()?;
	if bits == 0 {
		let v = r.varint()?;
		let n = r.varint()?;
		for _ in 0..n {
			r.long()?;
		}
		out.push(0);
		varint(out, map(v));
		varint(out, 0);
		return Ok(());
	}
	if bits <= indirect_max {
		let len = r.varint()?;
		out.push(bits);
		varint(out, len);
		for _ in 0..len {
			let v = r.varint()?;
			varint(out, map(v));
		}
		let n = r.varint()?;
		varint(out, n);
		for _ in 0..n {
			out.extend_from_slice(&r.long()?.to_be_bytes());
		}
		return Ok(());
	}
	let n = r.varint()? as usize;
	let longs = (0..n).map(|_| r.long()).collect::<Result<Vec<_>>>()?;
	let values: Vec<u32> = unpack(&longs, bits, entries).into_iter().map(map).collect();
	let global = global.unwrap_or(bits);
	let packed = pack(&values, global);
	out.push(global);
	varint(out, packed.len() as u32);
	for l in packed {
		out.extend_from_slice(&l.to_be_bytes());
	}
	Ok(())
}

pub struct Remap<'a> {
	pub state: &'a dyn Fn(u32) -> u32,
	pub states: u32,
	pub biome: &'a dyn Fn(u32) -> u32,
	pub biomes: Option<u32>,
}

pub fn sections(data: &[u8], m: &Remap, version: Version, known: Option<usize>) -> Result<Vec<u8>> {
	let mut r = Reader::new(data);
	let mut out = Vec::with_capacity(data.len());
	let (mut read, mut single) = (0, 0);
	while !r.done() && !padding(version, r.rest(), read, single, known) {
		let count = r.bytes(2)?;
		out.extend_from_slice(count);
		single += usize::from(r.rest().first() == Some(&0));
		read += 1;
		container(&mut r, &mut out, 4096, 8, Some(bits_for(m.states)), m.state)?;
		container(&mut r, &mut out, 64, 3, m.biomes.map(bits_for), m.biome)?;
	}
	Ok(out)
}
