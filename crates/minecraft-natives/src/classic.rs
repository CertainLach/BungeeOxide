use pds_bytes_runtime::{Error, Input, Output, Result};

pub const CP437: [char; 256] = {
	let high: [u16; 129] = [
		0x2302, 0x00C7, 0x00FC, 0x00E9, 0x00E2, 0x00E4, 0x00E0, 0x00E5, 0x00E7, 0x00EA, 0x00EB,
		0x00E8, 0x00EF, 0x00EE, 0x00EC, 0x00C4, 0x00C5, 0x00C9, 0x00E6, 0x00C6, 0x00F4, 0x00F6,
		0x00F2, 0x00FB, 0x00F9, 0x00FF, 0x00D6, 0x00DC, 0x00A2, 0x00A3, 0x00A5, 0x20A7, 0x0192,
		0x00E1, 0x00ED, 0x00F3, 0x00FA, 0x00F1, 0x00D1, 0x00AA, 0x00BA, 0x00BF, 0x2310, 0x00AC,
		0x00BD, 0x00BC, 0x00A1, 0x00AB, 0x00BB, 0x2591, 0x2592, 0x2593, 0x2502, 0x2524, 0x2561,
		0x2562, 0x2556, 0x2555, 0x2563, 0x2551, 0x2557, 0x255D, 0x255C, 0x255B, 0x2510, 0x2514,
		0x2534, 0x252C, 0x251C, 0x2500, 0x253C, 0x255E, 0x255F, 0x255A, 0x2554, 0x2569, 0x2566,
		0x2560, 0x2550, 0x256C, 0x2567, 0x2568, 0x2564, 0x2565, 0x2559, 0x2558, 0x2552, 0x2553,
		0x256B, 0x256A, 0x2518, 0x250C, 0x2588, 0x2584, 0x258C, 0x2590, 0x2580, 0x03B1, 0x00DF,
		0x0393, 0x03C0, 0x03A3, 0x03C3, 0x00B5, 0x03C4, 0x03A6, 0x0398, 0x03A9, 0x03B4, 0x221E,
		0x03C6, 0x03B5, 0x2229, 0x2261, 0x00B1, 0x2265, 0x2264, 0x2320, 0x2321, 0x00F7, 0x2248,
		0x00B0, 0x2219, 0x00B7, 0x221A, 0x207F, 0x00B2, 0x25A0, 0x00A0,
	];
	let low: [u16; 32] = [
		0x0000, 0x263A, 0x263B, 0x2665, 0x2666, 0x2663, 0x2660, 0x2022, 0x25D8, 0x25CB, 0x25D9,
		0x2642, 0x2640, 0x266A, 0x266B, 0x263C, 0x25BA, 0x25C4, 0x2195, 0x203C, 0x00B6, 0x00A7,
		0x25AC, 0x21A8, 0x2191, 0x2193, 0x2192, 0x2190, 0x221F, 0x2194, 0x25B2, 0x25BC,
	];
	let mut out = ['\0'; 256];
	let mut i = 0;
	while i < 256 {
		let code = if i < 32 {
			low[i] as u32
		} else if i < 127 {
			i as u32
		} else {
			high[i - 127] as u32
		};
		out[i] = match char::from_u32(code) {
			Some(c) => c,
			None => panic!("CP437 maps to scalar values"),
		};
		i += 1;
	}
	out
};

pub fn cp437_byte(c: char) -> Option<u8> {
	match c {
		' '..='~' => Some(c as u8),
		_ => CP437.iter().position(|x| *x == c).map(|i| i as u8),
	}
}

pub mod pds_classicString {
	use super::*;
	pub type Ty = String;
	pub const SIZE: usize = 64;
	pub fn serialize(_0: &Ty, output: &mut Output) -> Result<()> {
		let mut out = [b' '; SIZE];
		for (i, c) in _0.chars().enumerate() {
			*out.get_mut(i).ok_or(Error::InvalidValue)? =
				cp437_byte(c).ok_or(Error::InvalidValue)?;
		}
		output.write(&out);
		Ok(())
	}
	pub fn deserialize(input: &mut Input) -> Result<Ty> {
		let bytes: [u8; SIZE] = input.exact()?;
		let end = bytes.iter().rposition(|b| *b != b' ').map_or(0, |i| i + 1);
		Ok(bytes[..end].iter().map(|b| CP437[*b as usize]).collect())
	}
	pub fn size_of(_0: &Ty) -> usize {
		SIZE
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cp437_is_a_bijection() {
		for b in 0..=255u8 {
			assert_eq!(cp437_byte(CP437[b as usize]), Some(b));
		}
		assert_eq!(CP437[0x15], '\u{a7}');
		assert_eq!(cp437_byte('\u{2302}'), Some(0x7f));
	}

	#[test]
	fn classic_strings_are_padded_with_spaces() {
		use pds_classicString::*;
		let mut out = Output::new();
		serialize(&"&eHi \u{263a}".to_owned(), &mut out).unwrap();
		let bytes = out.into_inner();
		assert_eq!(bytes.len(), 64);
		assert_eq!(&bytes[..6], b"&eHi \x01");
		assert!(bytes[6..].iter().all(|b| *b == b' '));
		assert_eq!(
			deserialize(&mut Input::new(&bytes)).unwrap(),
			"&eHi \u{263a}"
		);
		assert!(serialize(&"x".repeat(65), &mut Output::new()).is_err());
		assert!(serialize(&"\u{e9}t\u{e9}".to_owned(), &mut Output::new()).is_ok());
		assert!(serialize(&"\u{1f600}".to_owned(), &mut Output::new()).is_err());
		assert_eq!(deserialize(&mut Input::new(&[b' '; 64])).unwrap(), "");
		assert!(deserialize(&mut Input::new(&[b' '; 63])).is_err());
	}
}
