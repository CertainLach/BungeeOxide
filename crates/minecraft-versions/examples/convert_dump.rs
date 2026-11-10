use minecraft_versions::gen::convert::{convert, index_of, Root};
use minecraft_versions::Session;

fn main() -> anyhow::Result<()> {
	let args: Vec<String> = std::env::args().skip(1).collect();
	let [input, from, to, output] = args.as_slice() else {
		anyhow::bail!("usage: convert_dump <dump> <from version> <to version> <output dump>");
	};
	let (from, to) = (
		index_of(from).ok_or_else(|| anyhow::anyhow!("unknown version {from}"))?,
		index_of(to).ok_or_else(|| anyhow::anyhow!("unknown version {to}"))?,
	);
	let bytes = std::fs::read(input)?;
	let mut cx = Session::new();
	let mut out = Vec::new();
	let mut at = 0;
	while at + 4 <= bytes.len() {
		let len = u32::from_be_bytes(bytes[at..at + 4].try_into()?) as usize;
		let data = &bytes[at + 4..at + 4 + len];
		at += 4 + len;
		match convert(Root::PlayToClient, from, to, data, &mut cx) {
			Ok(packets) => {
				for p in packets {
					out.extend((p.len() as u32).to_be_bytes());
					out.extend(p);
				}
			}
			Err(e) => eprintln!("packet {:#04x}: {e}", data[0]),
		}
	}
	std::fs::write(output, out)?;
	Ok(())
}
