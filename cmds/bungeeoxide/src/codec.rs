use std::io::{self, Read, Write};

use bytes::{Buf, BufMut, BytesMut};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use tokio_util::codec::{Decoder, Encoder};

use crate::legacy::{handshake_len, packet_len, Cfb8, Frame, Legacy};

const MAX_FRAME: usize = (1 << 21) - 1;
const MAX_PACKET: usize = 1 << 23;

pub fn read_varint(buf: &[u8]) -> Option<(i32, usize)> {
	let mut out = 0u32;
	for (i, b) in buf.iter().take(5).enumerate() {
		out |= u32::from(b & 0x7f) << (7 * i);
		if b & 0x80 == 0 {
			return Some((out as i32, i + 1));
		}
	}
	None
}

pub fn write_varint(out: &mut Vec<u8>, v: i32) {
	let mut v = v as u32;
	loop {
		let b = (v & 0x7f) as u8;
		v >>= 7;
		if v == 0 {
			out.push(b);
			return;
		}
		out.push(b | 0x80);
	}
}

fn invalid(msg: impl Into<String>) -> io::Error {
	io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

pub struct Cipher {
	decrypt: Cfb8,
	encrypt: Cfb8,
	plain: BytesMut,
}

impl Cipher {
	pub fn new(secret: &[u8; 16]) -> Self {
		Self {
			decrypt: Cfb8::new(secret),
			encrypt: Cfb8::new(secret),
			plain: BytesMut::new(),
		}
	}
}

#[derive(Default)]
pub struct Codec {
	pub threshold: Option<usize>,
	pub legacy: Option<Legacy>,
	pub cipher: Option<Cipher>,
}

fn decode_legacy(legacy: &mut Legacy, src: &mut BytesMut) -> io::Result<Option<Vec<u8>>> {
	let Legacy {
		root,
		version,
		cipher,
		plain,
		need,
	} = legacy;
	let buf = match cipher {
		Some((decrypt, _)) => {
			if !src.is_empty() {
				let mut chunk = src.split();
				decrypt.decrypt(&mut chunk);
				plain.extend_from_slice(&chunk);
			}
			plain
		}
		None => src,
	};
	if buf.is_empty() || buf.len() < *need {
		return Ok(None);
	}
	let len = match version {
		Some(v) => packet_len(*root, *v, buf),
		None => handshake_len(buf),
	};
	match len.map_err(invalid)? {
		Frame::Complete(n) => {
			*need = 0;
			Ok(Some(buf.split_to(n).to_vec()))
		}
		Frame::Need(n) => {
			if n > MAX_PACKET {
				return Err(invalid(format!(
					"pre-netty packet of at least {n} bytes is too large"
				)));
			}
			*need = n;
			Ok(None)
		}
	}
}

impl Decoder for Codec {
	type Item = Vec<u8>;
	type Error = io::Error;

	fn decode(&mut self, src: &mut BytesMut) -> io::Result<Option<Vec<u8>>> {
		if let Some(legacy) = self.legacy.as_mut() {
			return decode_legacy(legacy, src);
		}
		let threshold = self.threshold;
		match self.cipher.as_mut() {
			Some(cipher) => {
				if !src.is_empty() {
					let mut chunk = src.split();
					cipher.decrypt.decrypt(&mut chunk);
					cipher.plain.extend_from_slice(&chunk);
				}
				decode_frame(threshold, &mut cipher.plain)
			}
			None => decode_frame(threshold, src),
		}
	}
}

fn decode_frame(threshold: Option<usize>, src: &mut BytesMut) -> io::Result<Option<Vec<u8>>> {
	if src.len() >= 5 && read_varint(&src[..5]).is_none() {
		return Err(invalid("frame length varint is too long"));
	}
	let Some((len, header)) = read_varint(src) else {
		return Ok(None);
	};
	let len = usize::try_from(len).map_err(|_| invalid("negative frame length"))?;
	if len > MAX_FRAME {
		return Err(invalid(format!("frame of {len} bytes is too large")));
	}
	if src.len() < header + len {
		src.reserve(header + len - src.len());
		return Ok(None);
	}
	src.advance(header);
	let frame = src.split_to(len);
	if threshold.is_none() {
		return Ok(Some(frame.to_vec()));
	}
	let (data_len, n) = read_varint(&frame).ok_or_else(|| invalid("bad data length"))?;
	let body = &frame[n..];
	if data_len == 0 {
		return Ok(Some(body.to_vec()));
	}
	let data_len = usize::try_from(data_len).map_err(|_| invalid("negative data length"))?;
	if data_len > MAX_PACKET {
		return Err(invalid(format!("packet of {data_len} bytes is too large")));
	}
	let mut out = vec![0; data_len];
	let mut z = ZlibDecoder::new(body);
	z.read_exact(&mut out)
		.map_err(|_| invalid("packet inflates to less than its declared size"))?;
	if z.read(&mut [0])? != 0 {
		return Err(invalid("packet inflates past its declared size"));
	}
	Ok(Some(out))
}

impl Encoder<Vec<u8>> for Codec {
	type Error = io::Error;

	fn encode(&mut self, data: Vec<u8>, dst: &mut BytesMut) -> io::Result<()> {
		if let Some(legacy) = self.legacy.as_mut() {
			let mut data = data;
			if let Some((_, encrypt)) = legacy.cipher.as_mut() {
				encrypt.encrypt(&mut data);
			}
			dst.put_slice(&data);
			return Ok(());
		}
		let mut payload = Vec::new();
		match self.threshold {
			Some(t) if data.len() >= t => {
				write_varint(&mut payload, data.len() as i32);
				let mut z = ZlibEncoder::new(&mut payload, Compression::default());
				z.write_all(&data)?;
				z.finish()?;
			}
			Some(_) => {
				payload.push(0);
				payload.extend_from_slice(&data);
			}
			None => payload = data,
		}
		let mut header = Vec::with_capacity(5);
		write_varint(&mut header, payload.len() as i32);
		dst.reserve(header.len() + payload.len());
		let start = dst.len();
		dst.put_slice(&header);
		dst.put_slice(&payload);
		if let Some(cipher) = self.cipher.as_mut() {
			cipher.encrypt.encrypt(&mut dst[start..]);
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn frames_roundtrip_with_and_without_compression_and_encryption() {
		for threshold in [None, Some(4), Some(1000)] {
			for encrypted in [false, true] {
				let mut codec = Codec {
					threshold,
					legacy: None,
					cipher: encrypted.then(|| Cipher::new(&[3; 16])),
				};
				let mut buf = BytesMut::new();
				let mut plain = BytesMut::new();
				let packets = vec![vec![1, 2, 3], vec![9; 300], vec![]];
				for p in &packets {
					codec.encode(p.clone(), &mut buf).unwrap();
					Codec {
						threshold,
						..Codec::default()
					}
					.encode(p.clone(), &mut plain)
					.unwrap();
				}
				assert_eq!(buf == plain, !encrypted);
				let tail = buf.split_off(buf.len() - 1);
				let mut got = Vec::new();
				while let Some(p) = codec.decode(&mut buf).unwrap() {
					got.push(p);
				}
				buf.unsplit(tail);
				while let Some(p) = codec.decode(&mut buf).unwrap() {
					got.push(p);
				}
				assert_eq!(got, packets, "{threshold:?} {encrypted}");
			}
		}
	}

	fn compressed(declared: i32, inflated: &[u8]) -> BytesMut {
		let mut payload = Vec::new();
		write_varint(&mut payload, declared);
		let mut z = ZlibEncoder::new(&mut payload, Compression::default());
		z.write_all(inflated).unwrap();
		z.finish().unwrap();
		let mut frame = Vec::new();
		write_varint(&mut frame, payload.len() as i32);
		frame.extend_from_slice(&payload);
		BytesMut::from(&frame[..])
	}

	fn decode_compressed(mut frame: BytesMut) -> io::Result<Option<Vec<u8>>> {
		Codec {
			threshold: Some(64),
			..Codec::default()
		}
		.decode(&mut frame)
	}

	#[test]
	fn compressed_packets_inflate_to_exactly_their_declared_size() {
		assert_eq!(
			decode_compressed(compressed(100, &[7; 100])).unwrap(),
			Some(vec![7; 100])
		);
		let bomb = compressed(100, &vec![0; 1 << 26]);
		assert!(bomb.len() < 1 << 17);
		assert!(decode_compressed(bomb).is_err());
		assert!(decode_compressed(compressed(100, &[7; 99])).is_err());
		assert!(decode_compressed(compressed(MAX_PACKET as i32 + 1, &[0])).is_err());
	}

	#[test]
	fn frames_past_the_vanilla_limit_are_rejected() {
		let mut header = Vec::new();
		write_varint(&mut header, MAX_FRAME as i32 + 1);
		let mut src = BytesMut::from(&header[..]);
		assert!(Codec::default().decode(&mut src).is_err());
	}

	#[test]
	fn legacy_packets_wait_for_the_bytes_they_are_known_to_need() {
		let mut handshake = vec![0x02, 0, 30];
		for unit in "a".repeat(30).encode_utf16() {
			handshake.extend_from_slice(&unit.to_be_bytes());
		}
		let mut codec = Codec {
			legacy: Some(Legacy::new(minecraft_versions::Root::PlayToServer, None)),
			..Codec::default()
		};
		let mut src = BytesMut::new();
		for (i, b) in handshake.iter().enumerate() {
			src.put_u8(*b);
			let got = codec.decode(&mut src).unwrap();
			if i + 1 < handshake.len() {
				assert_eq!(got, None);
				if i >= 2 {
					assert_eq!(codec.legacy.as_ref().unwrap().need, handshake.len());
				}
			} else {
				assert_eq!(got, Some(handshake.clone()));
			}
		}

		let mut chunk = vec![0x33];
		chunk.extend_from_slice(&[0; 13]);
		chunk.extend_from_slice(&i32::MAX.to_be_bytes());
		let mut codec = Codec {
			legacy: Some(Legacy::new(
				minecraft_versions::Root::PlayToClient,
				Some(crate::at("1.6.4")),
			)),
			..Codec::default()
		};
		assert!(codec.decode(&mut BytesMut::from(&chunk[..])).is_err());
	}
}
