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

#[derive(PacketData)]
struct StatusResponse {
	response: String,
}
impl Packet for StatusResponse {
	const ID: i32 = 0;
}
