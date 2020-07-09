use super::*;

#[derive(Debug)]
pub struct StatusRequest;

impl Packet for StatusRequest {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		Ok(StatusRequest)
	}
}

struct StatusResponse {
	response: String,
}

impl Packet for StatusResponse {
	fn read<R: Read>(buf: &mut R) -> io::Result<Self> {
		Ok(StatusResponse {
			response: buf.read_string(32767)?,
		})
	}
}
