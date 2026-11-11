use std::collections::BTreeMap;

use minecraft_versions::gen::v1_8::play::to_client::packet;
use minecraft_versions::pds::{Input, Output};

fn main() -> anyhow::Result<()> {
	let path = std::env::args().nth(1).expect("dump file");
	let bytes = std::fs::read(path)?;
	let mut at = 0;
	let mut ok: BTreeMap<String, usize> = BTreeMap::new();
	let mut bad = 0;
	while at + 4 <= bytes.len() {
		let len = u32::from_be_bytes(bytes[at..at + 4].try_into()?) as usize;
		let data = &bytes[at + 4..at + 4 + len];
		at += 4 + len;
		let decoded = packet::deserialize(&mut Input::new(data));
		let reencoded = decoded.as_ref().ok().map(|p| {
			let mut out = Output::new();
			packet::serialize(p, &mut out).map(|_| out.into_inner())
		});
		match (decoded, reencoded) {
			(Ok(p), Some(Ok(again))) if again == data => {
				let name = format!("{p:?}");
				*ok.entry(name.split('(').next().unwrap_or(&name).to_owned())
					.or_default() += 1;
			}
			(decoded, _) => {
				bad += 1;
				eprintln!("mismatch for packet {:#04x}: {:?}", data[0], decoded.err());
			}
		}
	}
	let total: usize = ok.values().sum();
	println!(
		"{total} packets of {} kinds re-encode identically, {bad} differ",
		ok.len()
	);
	Ok(())
}
