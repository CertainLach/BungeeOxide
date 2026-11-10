use std::collections::BTreeMap;

use minecraft_versions::gen::convert::{convert, index_of, Root};
use minecraft_versions::gen::convert::{
	v1_12_2::play_toClient_packet as newest, v1_8::play_toClient_packet as oldest,
};
use minecraft_versions::pds::Input;
use minecraft_versions::Session;

fn kind(p: &impl std::fmt::Debug) -> String {
	let name = format!("{p:?}");
	name.split('(').next().unwrap_or(&name).to_owned()
}

fn main() -> anyhow::Result<()> {
	let path = std::env::args().nth(1).expect("dump file");
	let bytes = std::fs::read(path)?;
	let (first, last) = (index_of("1.8").unwrap(), index_of("1.12.2").unwrap());
	let mut cx = Session::new();
	let mut at = 0;
	let mut produced: BTreeMap<String, usize> = BTreeMap::new();
	let mut dropped: BTreeMap<String, usize> = BTreeMap::new();
	let mut failed: BTreeMap<String, Vec<String>> = BTreeMap::new();
	while at + 4 <= bytes.len() {
		let len = u32::from_be_bytes(bytes[at..at + 4].try_into()?) as usize;
		let data = &bytes[at + 4..at + 4 + len];
		at += 4 + len;
		let source = kind(
			&oldest::deserialize(&mut Input::new(data)).map_err(|e| anyhow::anyhow!("{e:?}"))?,
		);
		match convert(Root::PlayToClient, first, last, data, &mut cx) {
			Ok(out) if out.is_empty() => *dropped.entry(source).or_default() += 1,
			Ok(out) => {
				for p in out {
					let mut input = Input::new(&p);
					match newest::deserialize(&mut input) {
						Ok(decoded) if input.remaining() == 0 => {
							*produced.entry(kind(&decoded)).or_default() += 1
						}
						Ok(decoded) => failed.entry(source.clone()).or_default().push(format!(
							"{} bytes left after {}",
							input.remaining(),
							kind(&decoded)
						)),
						Err(e) => failed
							.entry(source.clone())
							.or_default()
							.push(format!("{e:?}")),
					}
				}
			}
			Err(e) => failed.entry(source).or_default().push(e.to_string()),
		}
	}
	println!("1.8 -> 1.12.2");
	println!("produced: {produced:?}");
	println!("dropped: {dropped:?}");
	for (k, errs) in &failed {
		println!("FAILED {k} x{}: {}", errs.len(), errs[0]);
	}
	Ok(())
}
