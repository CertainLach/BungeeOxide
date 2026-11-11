use std::cell::RefCell;

use crate::pds::convert::{unconvertible, Result};

pub(super) struct Reader<'a> {
	data: &'a [u8],
	at: usize,
}

impl<'a> Reader<'a> {
	pub(super) fn new(data: &'a [u8]) -> Self {
		Self { data, at: 0 }
	}

	pub(super) fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
		let Some(out) = self
			.at
			.checked_add(n)
			.and_then(|end| self.data.get(self.at..end))
		else {
			return unconvertible("chunk data ends early");
		};
		self.at += n;
		Ok(out)
	}

	pub(super) fn u8(&mut self) -> Result<u8> {
		Ok(self.bytes(1)?[0])
	}

	pub(super) fn i16(&mut self) -> Result<i16> {
		Ok(i16::from_be_bytes(
			self.bytes(2)?.try_into().expect("two bytes"),
		))
	}

	pub(super) fn i32(&mut self) -> Result<i32> {
		Ok(i32::from_be_bytes(
			self.bytes(4)?.try_into().expect("four bytes"),
		))
	}

	pub(super) fn varint(&mut self) -> Result<i32> {
		let mut v = 0u32;
		for shift in (0..35).step_by(7) {
			let b = self.u8()?;
			v |= ((b & 0x7f) as u32) << shift;
			if b & 0x80 == 0 {
				return Ok(v as i32);
			}
		}
		unconvertible("varint in chunk data is too long")
	}

	pub(super) fn longs(&mut self, n: usize) -> Result<Vec<u64>> {
		let Some(len) = n.checked_mul(8) else {
			return unconvertible("long array is too long");
		};
		Ok(self
			.bytes(len)?
			.as_chunks::<8>()
			.0
			.iter()
			.map(|c| u64::from_be_bytes(*c))
			.collect())
	}

	pub(super) fn remaining(&self) -> usize {
		self.data.len() - self.at
	}

	pub(super) fn done(&self) -> bool {
		self.at >= self.data.len()
	}

	pub(super) fn rest(&self) -> &'a [u8] {
		&self.data[self.at.min(self.data.len())..]
	}
}

pub(super) fn put_varint(out: &mut Vec<u8>, v: i32) {
	let mut v = v as u32;
	while v >= 0x80 {
		out.push(v as u8 | 0x80);
		v >>= 7;
	}
	out.push(v as u8);
}

pub(super) fn put_longs(out: &mut Vec<u8>, longs: &[u64], length: bool) {
	if length {
		put_varint(out, longs.len() as i32);
	}
	out.reserve(longs.len() * 8);
	for l in longs {
		out.extend_from_slice(&l.to_be_bytes());
	}
}

pub(crate) fn bits_for(n: usize) -> u32 {
	match n {
		0 | 1 => 0,
		n => usize::BITS - (n - 1).leading_zeros(),
	}
}

pub(crate) fn unpack(longs: &[u64], bits: u32, count: usize, spanning: bool) -> Vec<u32> {
	let bits = bits.clamp(1, 32) as usize;
	let mask = if bits == 32 {
		u32::MAX as u64
	} else {
		(1u64 << bits) - 1
	};
	let word = |i: usize| longs.get(i).copied().unwrap_or(0);
	let mut out = Vec::with_capacity(count);
	if spanning {
		for i in 0..count {
			let bit = i * bits;
			let (w, off) = (bit / 64, bit % 64);
			let mut v = word(w) >> off;
			if off + bits > 64 {
				v |= word(w + 1) << (64 - off);
			}
			out.push((v & mask) as u32);
		}
	} else {
		let per = 64 / bits;
		for i in 0..count {
			out.push(((word(i / per) >> ((i % per) * bits)) & mask) as u32);
		}
	}
	out
}

pub(crate) fn pack(
	values: impl ExactSizeIterator<Item = u32>,
	bits: u32,
	spanning: bool,
) -> Vec<u64> {
	let bits = bits.clamp(1, 32) as usize;
	let mask = if bits == 32 {
		u32::MAX as u64
	} else {
		(1u64 << bits) - 1
	};
	if spanning {
		let mut out = vec![0u64; (values.len() * bits).div_ceil(64)];
		for (i, v) in values.enumerate() {
			let v = v as u64 & mask;
			let bit = i * bits;
			let (w, off) = (bit / 64, bit % 64);
			out[w] |= v << off;
			if off + bits > 64 {
				out[w + 1] |= v >> (64 - off);
			}
		}
		out
	} else {
		let per = 64 / bits;
		let mut out = vec![0u64; values.len().div_ceil(per)];
		for (i, v) in values.enumerate() {
			out[i / per] |= (v as u64 & mask) << ((i % per) * bits);
		}
		out
	}
}

pub(super) fn long_count(entries: usize, bits: u32, spanning: bool) -> usize {
	match (bits, spanning) {
		(0, _) => 0,
		(b, true) => (entries * b as usize).div_ceil(64),
		(b, false) => entries.div_ceil(64 / b as usize),
	}
}

pub(super) struct Palette {
	pub(super) values: Vec<u32>,
	pub(super) indices: Vec<u32>,
}

thread_local! {
	static SLOTS: RefCell<(u32, Vec<(u32, u32)>)> = const { RefCell::new((0, Vec::new())) };
}

pub(super) fn palette(states: &[u32], limit: usize) -> Option<Palette> {
	SLOTS.with(|slots| {
		let (generation, table) = &mut *slots.borrow_mut();
		*generation = generation.wrapping_add(1);
		if *generation == 0 {
			table.iter_mut().for_each(|e| *e = (0, 0));
			*generation = 1;
		}
		let generation = *generation;
		let mut values = Vec::new();
		let mut indices = Vec::with_capacity(states.len());
		for s in states {
			let at = *s as usize;
			if at >= table.len() {
				table.resize((at + 1).next_power_of_two().max(1 << 16), (0, 0));
			}
			let slot = &mut table[at];
			if slot.0 != generation {
				if values.len() == limit {
					return None;
				}
				*slot = (generation, values.len() as u32);
				values.push(*s);
			}
			indices.push(slot.1);
		}
		Some(Palette { values, indices })
	})
}
