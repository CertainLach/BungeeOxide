pub fn as_i8(b: &[u8]) -> &[i8] {
	unsafe { std::slice::from_raw_parts(b.as_ptr().cast(), b.len()) }
}
