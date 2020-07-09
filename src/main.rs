mod ext;
mod protocol;

use ext::{MinecraftWriteExt, Varint21};
use std::{
	io::{Cursor, Read},
	ops::{Deref, DerefMut},
};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use protocol::{handshake::Handshake, State, status::StatusRequest, login::LoginStart, Packet};


struct UserHandle {
	stream: TcpStream,
	state: State,
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
		use ext::MinecraftAsyncReadExt;
		let mut buf = vec![0; 256];
		loop {
			let packet_length = self.read_varint().await?.ans;
			assert!(packet_length >= 1);
			let Varint21 {
				ans: packet_id,
				size: packet_id_length,
			} = self.read_varint().await?;
			assert!(packet_id >= 0);
			let packet_length = packet_length - packet_id_length as i32;
			assert!(packet_length >= 0);
			if packet_length >= 256 {
				todo!("pass to downstream directly");
			} else {
				let buf = &mut buf[0..packet_length as usize];
				self.read_exact(buf).await?;
				let buf = &*buf;
				let mut cursor = std::io::Cursor::new(buf);
				self.handle(packet_id, &mut cursor).await?;
			}
		}
	}
	async fn write_packet(
		&mut self,
		packet_id: i32,
		data: impl FnOnce(&mut Cursor<Vec<u8>>) -> io::Result<()>,
	) -> io::Result<()> {
		let out = {
			let mut writer = Cursor::new(Vec::new());
			writer.write_varint(packet_id)?;
			data(&mut writer)?;
			writer.into_inner()
		};
		{
			use ext::MinecraftAsyncWriteExt;
			self.write_varint(out.len() as i32).await?;
			self.write_all(&out).await?;
			Ok(())
		}
	}
	async fn handle(&mut self, packet_id: i32, data: &mut impl Read) -> io::Result<()> {
		match self.state {
			State::Handshaking => self.handle_handshaking(packet_id, data).await?,
			State::Status => self.handle_status(packet_id, data).await?,
			State::Login => self.handle_login(packet_id, data).await?,
			_ => todo!(),
		};
		Ok(())
	}
	async fn handle_handshaking(&mut self, packet_id: i32, data: &mut impl Read) -> io::Result<()> {
		match packet_id {
			0 => {
				let packet = Handshake::read(data)?;
				println!("Handshake: {:?}", packet);
				self.state = packet.next_state;
			}
			_ => todo!(),
		}
		Ok(())
	}
	async fn handle_status(&mut self, packet_id: i32, data: &mut impl Read) -> io::Result<()> {
		match packet_id {
			0 => {
				let packet = StatusRequest::read(data)?;
				println!("Request: {:?}", packet);
				self.write_packet(0, |c| {
					c.write_string("{\"players\":{\"max\":3,\"online\":4}}")?;
					Ok(())
				})
				.await?;
			}
			_ => todo!(),
		}
		Ok(())
	}
	async fn handle_login(&mut self, packet_id: i32, data: &mut impl Read) -> io::Result<()> {
		match packet_id {
			0 => {
				let packet = LoginStart::read(data)?;
				println!("Login: {:?}", packet);
				let name = packet.name;
				// TODO encryption request
				self.write_packet(2, |c| {
					c.write_string("a9213bf3-13a7-44e0-a456-db16b1c2b43f")?;
					c.write_string(name.as_str())?;
					Ok(())
				})
				.await?;
			}
			_ => todo!(),
		}
		Ok(())
	}
}

#[tokio::main(core_threads = 4, max_threads = 8)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut listener = TcpListener::bind("127.0.0.1:25566").await?;

	loop {
		let (stream, _) = listener.accept().await?;
		println!("Got connection: {:?}", stream);
		tokio::spawn(async move {
			let mut handle = UserHandle {
				stream,
				state: State::Handshaking
			};
			if let Err(e) = handle.process().await {
				println!("User error: {:?}", e);
			};
		});
	}
}
