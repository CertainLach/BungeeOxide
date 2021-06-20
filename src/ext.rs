use crate::protocol::Packet;
use async_trait::async_trait;
use compress::zlib::Decoder;
use std::io::{BufReader, Cursor, Read, Write};
use thiserror::Error;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub struct Varint21 {
	pub ans: i32,
	pub size: u8,
}
impl Varint21 {
	fn read(mut read: impl Read) -> io::Result<Self> {
		let mut buf = [0];
		let mut ans = 0;
		let mut size = 0;
		for i in 0..=4 {
			read.read_exact(&mut buf)?;
			size += 1;
			ans |= ((buf[0] & 0b0111_1111) as i32) << (7 * i);
			if buf[0] & 0b1000_0000 == 0 {
				break;
			}
		}
		Ok(Self { ans, size })
	}
}

fn ensure_capacity(vec: &mut Vec<u8>, capacity: usize) {
	if vec.capacity() < capacity {
		vec.reserve(capacity - vec.capacity());
		// Safety: По факту нам без разницы, каким мусором там забито,
		// нам нужен лишь кусок этого буфера, и он гарантированно будет заполнен
		unsafe { vec.set_len(vec.capacity()) };
	}
}

#[derive(Error, Debug)]
pub enum CompressedError {
	#[error("miniz error #{0}")]
	MinizError(i32),
	#[error("user supplied bad packet")]
	BadInputPacket,
	#[error("io error")]
	IoError(#[from] io::Error),
}

/// Представляет собой обёртку над сжатыми/не сжатыми данными
pub enum MaybeCompressed<'t> {
	Decompressed {
		packet_id: i32,
		decompressed: Vec<u8>,
		compressed: &'t [u8],
	},
	Compressed {
		full_size: usize,
		compressed: &'t [u8],
	},
	PartiallyDecompressed {
		packet_id: i32,
		full_size: usize,
		decoder: compress::zlib::Decoder<Cursor<&'t [u8]>>,
	},
	Plain {
		packet_id: i32,
		data: &'t [u8],
	},
}
impl<'t> MaybeCompressed<'t> {
	pub fn id(&mut self) -> Result<i32, CompressedError> {
		match self {
			MaybeCompressed::Decompressed { packet_id, .. }
			| MaybeCompressed::Plain { packet_id, .. }
			| MaybeCompressed::PartiallyDecompressed { packet_id, .. } => Ok(*packet_id),
			MaybeCompressed::Compressed { .. } => {
				self.partially_decompress()?;
				self.id()
			}
		}
	}
	pub fn partially_decompress(&mut self) -> Result<(), CompressedError> {
		match self {
			&mut MaybeCompressed::Compressed {
				full_size,
				compressed,
			} => {
				let cursor = Cursor::new(compressed);
				let mut decoder = Decoder::new(cursor);
				let packet_id = Varint21::read(&mut decoder)?;

				*self = MaybeCompressed::PartiallyDecompressed {
					packet_id: packet_id.ans,
					decoder,
					full_size: full_size - packet_id.size as usize,
				};
				todo!()
			}
			_ => Ok(()),
		}
	}
	/// ID for partially uncompressed packets
	pub fn cheap_id(&self) -> Option<i32> {
		match self {
			MaybeCompressed::Decompressed { packet_id, .. }
			| MaybeCompressed::Plain { packet_id, .. }
			| MaybeCompressed::PartiallyDecompressed { packet_id, .. } => Some(*packet_id),
			_ => None,
		}
	}
	pub fn decode<T: Packet>(self) -> io::Result<T> {
		match self {
			MaybeCompressed::Decompressed { decompressed, .. } => {
				T::read(&mut Cursor::new(decompressed))
			}
			MaybeCompressed::Compressed { compressed, .. } => {
				T::read(&mut BufReader::new(Decoder::new(Cursor::new(compressed))))
			}
			MaybeCompressed::PartiallyDecompressed { decoder, .. } => {
				T::read(&mut BufReader::new(decoder))
			}
			MaybeCompressed::Plain { mut data, .. } => T::read(&mut data),
		}
	}
	pub async fn write<W: AsyncWrite + Unpin + Send>(
		self,
		compression_threshold: Option<i32>,
		buf: &mut W,
	) -> io::Result<()> {
		match self {
			// Fast path is available
			MaybeCompressed::Compressed {
				full_size,
				compressed,
			} if compression_threshold
				.map(|compression_threshold| compression_threshold as usize <= full_size)
				.unwrap_or(false) =>
			{
				let size_size = varint_size(full_size as i32);
				buf.write_varint(compressed.len() as i32 + size_size as i32)
					.await?;
				buf.write_varint(full_size as i32).await?;
				buf.write_all(compressed).await?;
			}
			MaybeCompressed::Decompressed {
				decompressed,
				compressed,
				..
			} if compression_threshold
				.map(|compression_threshold| compression_threshold as usize <= decompressed.len())
				.unwrap_or(false) =>
			{
				let size_size = varint_size(decompressed.len() as i32);
				buf.write_varint(compressed.len() as i32 + size_size as i32)
					.await?;
				buf.write_varint(decompressed.len() as i32).await?;
				buf.write_all(compressed).await?;
			}
			MaybeCompressed::PartiallyDecompressed {
				full_size, decoder, ..
			} if compression_threshold
				.map(|compression_threshold| compression_threshold as usize <= full_size)
				.unwrap_or(false) =>
			{
				let size_size = varint_size(full_size as i32);
				let compressed = decoder.unwrap().into_inner();
				buf.write_varint(compressed.len() as i32 + size_size as i32)
					.await?;
				buf.write_varint(full_size as i32).await?;
				buf.write_all(compressed).await?;
			}
			MaybeCompressed::Plain { packet_id, data }
				if compression_threshold
					.map(|compression_threshold| {
						compression_threshold as usize > data.len() + varint_size(packet_id)
					})
					.unwrap_or(true) =>
			{
				let id_len = varint_size(packet_id);
				if compression_threshold.is_some() {
					buf.write_varint(data.len() as i32 + id_len as i32 + 1)
						.await?;
					buf.write_varint(0).await?;
				} else {
					buf.write_varint(data.len() as i32 + id_len as i32).await?;
				}
				buf.write_varint(packet_id as i32).await?;
				buf.write_all(data).await?;
			}
			// Recompression needed
			_ => {
				todo!()
			}
		}
		Ok(())
	}
}

#[async_trait]
pub trait MinecraftAsyncReadExt: AsyncRead + Unpin + Send {
	async fn read_packet<'t>(
		&mut self,
		compression: Option<i32>,
		buf: &'t mut Vec<u8>,
	) -> io::Result<MaybeCompressed<'t>> {
		let packet_length = if compression.is_some() {
			let total_length = self.read_varint().await?.ans;
			assert!(total_length >= 1);
			let data_length = self.read_varint().await?;
			let total_length = total_length - data_length.size as i32;
			if data_length.ans != 0 {
				ensure_capacity(buf, total_length as usize);
				let buf = &mut buf[..total_length as usize];
				self.read_exact(buf).await?;
				return Ok(MaybeCompressed::Compressed {
					full_size: data_length.ans as usize,
					compressed: buf,
				});
			}
			total_length
		} else {
			let packet_length = self.read_varint().await?.ans;
			assert!(packet_length >= 1);
			packet_length
		};
		let Varint21 {
			ans: packet_id,
			size: packet_id_length,
		} = self.read_varint().await?;
		assert!(packet_id >= 0);
		let packet_length = packet_length - packet_id_length as i32;
		assert!(packet_length >= 0);

		ensure_capacity(buf, packet_length as usize);
		let buf = &mut buf[0..packet_length as usize];
		self.read_exact(buf).await?;
		let buf = &*buf;

		Ok(MaybeCompressed::Plain {
			packet_id,
			data: buf,
		})
	}
	async fn read_varint(&mut self) -> io::Result<Varint21> {
		let mut buf = [0];
		let mut ans = 0;
		let mut size = 0;
		for i in 0..=4 {
			self.read_exact(&mut buf).await?;
			size += 1;
			ans |= ((buf[0] & 0b0111_1111) as i32) << (7 * i);
			if buf[0] & 0b1000_0000 == 0 {
				break;
			}
		}
		Ok(Varint21 { ans, size })
	}

	async fn read_bytes(&mut self, limit: i32) -> io::Result<Vec<u8>> {
		let length = self.read_varint().await?.ans;
		if length > limit {
			panic!("Length exceeds limit"); // TODO don't panic
		}
		let mut buf = vec![0; length as usize];
		self.read_exact(&mut buf).await?;
		Ok(buf)
	}

	async fn read_string(&mut self, limit: i32) -> io::Result<String> {
		let bytes = self.read_bytes(limit).await?;
		Ok(String::from_utf8_lossy(&bytes).to_string())
	}
}
impl<T> MinecraftAsyncReadExt for T where T: AsyncRead + Unpin + Send {}

pub trait MinecraftReadExt: Read {
	fn read_varint(&mut self) -> io::Result<Varint21> {
		let mut buf = [0];
		let mut ans = 0;
		let mut size = 0;
		for i in 0..=4 {
			self.read_exact(&mut buf)?;
			size += 1;
			ans |= ((buf[0] & 0b0111_1111) as i32) << (7 * i);
			if buf[0] & 0b1000_0000 == 0 {
				break;
			}
		}
		Ok(Varint21 { ans, size })
	}
	fn read_bytes(&mut self, limit: i32) -> io::Result<Vec<u8>> {
		let length = self.read_varint().unwrap().ans;
		if length > limit {
			panic!("Length exceeds limit"); // TODO don't panic
		}
		let mut buf = vec![0; length as usize];
		self.read_exact(&mut buf).unwrap();
		Ok(buf)
	}

	fn read_string(&mut self, limit: i32) -> io::Result<String> {
		let bytes = self.read_bytes(limit).unwrap();
		Ok(String::from_utf8_lossy(&bytes).to_string())
	}
}
impl<T> MinecraftReadExt for T where T: Read {}

#[async_trait]
pub trait MinecraftAsyncWriteExt: AsyncWrite + Unpin {
	async fn write_packet<T: Packet + Sync>(
		&mut self,
		compression: Option<i32>,
		packet: &T,
	) -> io::Result<()> {
		println!("Compression {:?}", compression);
		self.write_packet_fn(T::ID, compression, |w| packet.write(w))
			.await
	}
	async fn write_packet_fn<H: Send + FnOnce(&mut Cursor<Vec<u8>>) -> io::Result<()>>(
		&mut self,
		packet_id: i32,
		compression: Option<i32>,
		data: H,
	) -> io::Result<()> {
		let out = {
			use crate::ext::MinecraftWriteExt;
			let mut writer = Cursor::new(Vec::new());
			MinecraftWriteExt::write_varint(&mut writer, packet_id)?;
			data(&mut writer)?;
			writer.into_inner()
		};
		{
			use tokio::io::AsyncWriteExt;
			if compression.is_some() {
				self.write_varint(out.len() as i32 + 1).await?;
				self.write_varint(0).await?;
			} else {
				self.write_varint(out.len() as i32).await?;
			}
			self.write_all(&out).await?;
			Ok(())
		}
	}
	async fn write_bytes_async(&mut self, buf: &[u8]) -> io::Result<()> {
		self.write_varint(buf.len() as i32).await?;
		self.write_all(&buf).await?;
		Ok(())
	}
	async fn write_varint(&mut self, mut value: i32) -> io::Result<()> {
		loop {
			let mut temp = value as u8 & 0b01111111;
			value >>= 7;
			if value != 0 {
				temp |= 0b10000000;
			}
			self.write_all(&[temp]).await?;
			if value == 0 {
				break;
			}
		}
		Ok(())
	}
}
impl<T> MinecraftAsyncWriteExt for T where T: AsyncWrite + Unpin {}

pub fn varint_size(mut value: i32) -> usize {
	let mut size = 0;
	loop {
		value >>= 7;
		size += 1;
		if value == 0 {
			break;
		}
	}
	size
}

pub trait MinecraftWriteExt: Write {
	fn write_varint(&mut self, mut value: i32) -> io::Result<()> {
		loop {
			let mut temp = value as u8 & 0b01111111;
			value >>= 7;
			if value != 0 {
				temp |= 0b10000000;
			}
			self.write_all(&[temp])?;
			if value == 0 {
				break;
			}
		}
		Ok(())
	}
	fn write_bytes(&mut self, buf: &[u8]) -> io::Result<()> {
		self.write_varint(buf.len() as i32)?;
		self.write_all(&buf)?;
		Ok(())
	}
	fn write_string(&mut self, str: &str) -> io::Result<()> {
		self.write_bytes(str.as_bytes())?;
		Ok(())
	}
}
impl<T> MinecraftWriteExt for T where T: Write {}
