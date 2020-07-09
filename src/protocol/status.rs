use std::io::{Read, Result};
use protool::*;

struct StatusRequest;

impl Packet for StatusRequest {
    fn read<R: Read>(buf: &mut R) -> Result<Self> {  }
}

struct StatusResponse {
    response: String
}

impl Packet for StatusResponse {
    fn read<R: Read>(buf: &mut R) -> Result<Self> {
        response = buf.read_string(32767).unwrap();
    }
}