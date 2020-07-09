use async_trait::async_trait;
use std::ops::{Deref, DerefMut};
use tokio::io::{self, AsyncRead, AsyncReadExt};
use tokio::net::{TcpListener, TcpStream};

#[async_trait]
trait MinecraftAsyncReadExt: AsyncRead + Unpin {
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

struct UserHandle {
	stream: TcpStream,
}
impl Deref for UserHandle {
	type Target = TcpStream;
	fn deref(&self) -> &Self::Target {
		&self.stream
	}
}
impl DerefMut for UserHandle {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.stream
	}
}
impl UserHandle {
	// TODO: Return Err instead of panic on protocol error
	async fn process(&mut self) -> io::Result<()> {
		let mut buf = vec![0; 256];
		loop {
			let (packet_length, _) = self.read_varint().await?;
			assert!(packet_length >= 1);
			let (packet_id, packet_id_length) = self.read_varint().await?;
			assert!(packet_id >= 0);
			let packet_length = packet_length - packet_id_length as i32;
			assert!(packet_length >= 0);
			if packet_length >= 256 {
				todo!("pass to downstream directly");
			} else {
				let buf = &mut buf[0..packet_length as usize];
				self.read_exact(buf).await?;
				let buf = &*buf;
				println!("Received packet: {} {:?}", packet_id, buf);
			}
		}
	}
}

#[tokio::main(core_threads = 4, max_threads = 8)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut listener = TcpListener::bind("127.0.0.1:25565").await?;

	loop {
		let (stream, _) = listener.accept().await?;
		println!("Got connection: {:?}", stream);
		tokio::spawn(async move {
			let mut handle = UserHandle { stream };
			if let Err(e) = handle.process().await {
				println!("User error: {:?}", e);
			};
		});
	}
}
