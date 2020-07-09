mod ext;

use byteorder::ReadBytesExt;
use byteorder::{BigEndian, ByteOrder};
use ext::*;
use std::{
	io::{Cursor, Read},
	ops::{Deref, DerefMut},
};
use tokio::io::{self, AsyncReadExt};
use tokio::net::{TcpListener, TcpStream};

trait Packet: Sized {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self>;
}
#[derive(Debug)]
enum State {
	Handshaking,
	Status,
	Login,
}
impl Packet for State {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		Ok(match buf.read_varint()?.0 {
			0 => Self::Handshaking,
			1 => Self::Status,
			2 => Self::Login,
			_ => todo!(),
		})
	}
}
#[derive(Debug)]
struct ServerHandshare {
	protocol: i32,
	address: String,
	port: i16,
	next_state: State,
}
impl Packet for ServerHandshare {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		Ok(ServerHandshare {
			protocol: buf.read_varint()?.0,
			address: buf.read_string()?,
			port: buf.read_i16::<BigEndian>()?,
			next_state: State::read(buf)?,
		})
	}
}
#[derive(Debug)]
struct StatusRequest;
impl Packet for StatusRequest {
    fn read<R: Read>(_buf: &mut R) -> io::Result<Self> {
		Ok(StatusRequest)
	}
}

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
				let mut cursor = std::io::Cursor::new(buf);
				self.handle(packet_id, &mut cursor).await?;
			}
		}
	}
	async fn handle(&mut self, packet_id: i32, data: &mut impl Read) -> io::Result<()> {
		match self.state {
			State::Handshaking => self.handle_handshaking(packet_id, data).await?,
			State::Status => self.handle_status(packet_id, data).await?,
			_ => todo!(),
		};
		Ok(())
	}
	async fn handle_handshaking(&mut self, packet_id: i32, data: &mut impl Read) -> io::Result<()> {
		match packet_id {
			0 => {
				let packet = ServerHandshare::read(data)?;
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
				state: State::Handshaking,
			};
			if let Err(e) = handle.process().await {
				println!("User error: {:?}", e);
			};
		});
	}
}
