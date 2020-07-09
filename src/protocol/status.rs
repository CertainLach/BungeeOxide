use super::*;

#[derive(Debug)]
pub struct StatusRequest;
impl Packet for StatusRequest {
	const ID: i32 = 0;
}
impl PacketData for StatusRequest {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		Ok(StatusRequest)
	}
	fn write<W: std::io::Write>(&self, buf: &mut W) -> io::Result<()> {
		todo!()
	}
}

struct StatusResponse {
	response: String,
}
impl Packet for StatusResponse {
	const ID: i32 = 0;
}
impl PacketData for StatusResponse {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		Ok(StatusResponse {
			response: buf.read_string(32767)?,
		})
	}
	fn write<W: std::io::Write>(&self, buf: &mut W) -> io::Result<()> {
		todo!()
	}
}
