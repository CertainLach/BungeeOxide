
use async_trait::async_trait;
use std::io::{Write, Read};
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub struct Varint21 {
	pub ans: i32,
	pub size: u8,
}

#[async_trait]
pub trait MinecraftAsyncReadExt: AsyncRead + Unpin {
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

#[async_trait]
pub trait MinecraftAsyncWriteExt: AsyncWrite + Unpin {
	async fn write_varint(&mut self, mut value: i32) -> io::Result<()> {
		loop {
			let mut temp = value as u8 & 0b01111111;
			value >>= 7;
			if value != 0 {
				temp |= 0b10000000;
			}
			if value == 0 {
				break;
			}
			self.write_all(&[temp]).await?;
		}
		Ok(())
	}
}
impl<T> MinecraftAsyncWriteExt for T where T: AsyncWrite + Unpin {}

pub trait MinecraftWriteExt: Write {
	fn write_varint(&mut self, mut value: i32) -> io::Result<()> {
		loop {
			let mut temp = value as u8 & 0b01111111;
			value >>= 7;
			if value != 0 {
				temp |= 0b10000000;
			}
			if value == 0 {
				break;
			}
			self.write_all(&[temp])?;
		}
		Ok(())
	}
	fn write_buf(&mut self, buf: &[u8]) -> io::Result<()> {
		self.write_varint(buf.len() as i32)?;
		self.write_all(&buf)?;
		Ok(())
	}
	fn write_string(&mut self, str: &str) -> io::Result<()> {
		self.write_buf(str.as_bytes())?;
		Ok(())
	}
}
impl<T> MinecraftWriteExt for T where T: Write {}
