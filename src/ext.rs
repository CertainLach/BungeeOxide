use async_trait::async_trait;
use io::AsyncReadExt;
use std::io::Read;
use tokio::io::{self, AsyncRead};

#[async_trait]
pub trait MinecraftAsyncReadExt: AsyncRead + Unpin {
	async fn read_varint(&mut self) -> io::Result<(i32, u8)> {
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
		Ok((ans, size))
	}
}
impl<T> MinecraftAsyncReadExt for T where T: AsyncRead + Unpin {}
pub trait MinecraftReadExt: Read {
	fn read_varint(&mut self) -> io::Result<(i32, u8)> {
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
		Ok((ans, size))
	}
	fn read_buf(&mut self) -> io::Result<Vec<u8>> {
		let len = self.read_varint()?.0;
		let mut buf = vec![0; len as usize];
		self.read_exact(&mut buf)?;
		Ok(buf)
	}
	fn read_string(&mut self) -> io::Result<String> {
		Ok(String::from_utf8(self.read_buf()?).unwrap())
	}
}
impl<T> MinecraftReadExt for T where T: Read {}
