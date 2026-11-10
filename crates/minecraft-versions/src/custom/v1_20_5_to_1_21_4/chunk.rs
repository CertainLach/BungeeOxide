use super::{read_varint, write_varint};

fn bits_for(n: u32) -> u32 {
	32 - n.saturating_sub(1).leading_zeros()
}

fn longs(data: &[u8], at: &mut usize) -> Option<Vec<u64>> {
	let len = usize::try_from(read_varint(data, at)?).ok()?;
	let bytes = data.get(*at..*at + len.checked_mul(8)?)?;
	*at += bytes.len();
	Some(
		bytes
			.chunks_exact(8)
			.map(|b| u64::from_be_bytes(b.try_into().expect("8 bytes")))
			.collect(),
	)
}

fn write_longs(out: &mut Vec<u8>, data: &[u64]) {
	write_varint(out, data.len() as i32);
	for l in data {
		out.extend_from_slice(&l.to_be_bytes());
	}
}

fn unpack(data: &[u64], bits: u32, n: usize) -> Vec<u32> {
	let per = (64 / bits) as usize;
	let mask = (1u64 << bits) - 1;
	(0..n)
		.map(|i| {
			data.get(i / per)
				.map_or(0, |l| ((l >> ((i % per) as u32 * bits)) & mask) as u32)
		})
		.collect()
}

fn pack(values: &[u32], bits: u32) -> Vec<u64> {
	let per = (64 / bits) as usize;
	let mut out = vec![0u64; values.len().div_ceil(per)];
	for (i, v) in values.iter().enumerate() {
		out[i / per] |= (*v as u64) << ((i % per) as u32 * bits);
	}
	out
}

fn copy_biomes(data: &[u8], at: &mut usize, out: &mut Vec<u8>) -> Option<()> {
	let start = *at;
	let bits = *data.get(*at)?;
	*at += 1;
	let palette = match bits {
		0 => 1,
		1..=3 => read_varint(data, at)?,
		_ => 0,
	};
	for _ in 0..palette {
		read_varint(data, at)?;
	}
	longs(data, at)?;
	out.extend_from_slice(&data[start..*at]);
	Some(())
}

fn remap_blocks(
	data: &[u8],
	at: &mut usize,
	out: &mut Vec<u8>,
	map: &dyn Fn(i32) -> i32,
	direct: u32,
) -> Option<()> {
	let bits = *data.get(*at)?;
	*at += 1;
	match bits {
		0 => {
			let v = read_varint(data, at)?;
			longs(data, at)?;
			out.push(0);
			write_varint(out, map(v));
			write_varint(out, 0);
		}
		1..=8 => {
			let n = read_varint(data, at)?;
			let palette: Vec<i32> = (0..n)
				.map(|_| read_varint(data, at))
				.collect::<Option<_>>()?;
			let data = longs(data, at)?;
			out.push(bits);
			write_varint(out, n);
			for v in palette {
				write_varint(out, map(v));
			}
			write_longs(out, &data);
		}
		_ => {
			let values: Vec<u32> = unpack(&longs(data, at)?, bits as u32, 4096)
				.into_iter()
				.map(|v| map(v as i32) as u32)
				.collect();
			out.push(direct as u8);
			write_longs(out, &pack(&values, direct));
		}
	}
	Some(())
}

pub(super) fn remap_sections(
	data: &[u8],
	states: u32,
	map: &dyn Fn(i32) -> i32,
) -> Option<Vec<u8>> {
	let direct = bits_for(states);
	let mut out = Vec::with_capacity(data.len());
	let mut at = 0;
	while at < data.len() {
		out.extend_from_slice(data.get(at..at + 2)?);
		at += 2;
		remap_blocks(data, &mut at, &mut out, map, direct)?;
		copy_biomes(data, &mut at, &mut out)?;
	}
	Some(out)
}

#[cfg(test)]
pub(super) fn section(states: &[u32], bits: u8, biome: i32) -> Vec<u8> {
	let mut out = vec![0x10, 0x00];
	let mut palette: Vec<u32> = Vec::new();
	for s in states {
		if !palette.contains(s) {
			palette.push(*s);
		}
	}
	if bits <= 8 {
		out.push(bits);
		write_varint(&mut out, palette.len() as i32);
		for p in &palette {
			write_varint(&mut out, *p as i32);
		}
		let idx: Vec<u32> = states
			.iter()
			.map(|s| palette.iter().position(|p| p == s).unwrap() as u32)
			.collect();
		write_longs(&mut out, &pack(&idx, bits as u32));
	} else {
		out.push(bits);
		write_longs(&mut out, &pack(states, bits as u32));
	}
	out.push(0);
	write_varint(&mut out, biome);
	write_varint(&mut out, 0);
	out
}

#[cfg(test)]
pub(super) fn read_section(data: &[u8], at: &mut usize) -> (Vec<u32>, i32) {
	*at += 2;
	let bits = data[*at];
	*at += 1;
	let states = if bits <= 8 {
		let n = read_varint(data, at).unwrap();
		let palette: Vec<u32> = (0..n)
			.map(|_| read_varint(data, at).unwrap() as u32)
			.collect();
		let l = longs(data, at).unwrap();
		if bits == 0 {
			vec![palette[0]; 4096]
		} else {
			unpack(&l, bits as u32, 4096)
				.into_iter()
				.map(|i| palette[i as usize])
				.collect()
		}
	} else {
		unpack(&longs(data, at).unwrap(), bits as u32, 4096)
	};
	assert_eq!(data[*at], 0);
	*at += 1;
	let biome = read_varint(data, at).unwrap();
	assert_eq!(read_varint(data, at), Some(0));
	(states, biome)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn packing_round_trips() {
		let values: Vec<u32> = (0..4096).map(|i| (i * 7919) % 30000).collect();
		assert_eq!(unpack(&pack(&values, 15), 15, 4096), values);
	}

	#[test]
	fn sections_remap_palettes_and_direct_data() {
		let a: Vec<u32> = (0..4096).map(|i| (i % 3) as u32).collect();
		let b: Vec<u32> = (0..4096).map(|i| (i * 31 % 20000) as u32).collect();
		let mut data = section(&a, 4, 7);
		data.extend(section(&b, 15, 9));
		let out = remap_sections(&data, 30000, &|s| s + 1).unwrap();
		let mut at = 0;
		assert_eq!(
			read_section(&out, &mut at),
			(a.iter().map(|s| s + 1).collect(), 7)
		);
		assert_eq!(
			read_section(&out, &mut at),
			(b.iter().map(|s| s + 1).collect(), 9)
		);
		assert_eq!(at, out.len());
	}
}
