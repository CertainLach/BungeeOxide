use std::collections::BTreeMap;

use minecraft_versions::gen::convert::{index_of, inspect, step, Root, VERSIONS};
use minecraft_versions::Session;

fn main() -> anyhow::Result<()> {
	let mut args = std::env::args().skip(1);
	let path = args.next().expect("dump file");
	let from = args.next().unwrap_or_else(|| "1.8".to_owned());
	let to = args.next();
	let bytes = std::fs::read(path)?;
	let mut packets = Vec::new();
	let mut at = 0;
	while at + 4 <= bytes.len() {
		let len = u32::from_be_bytes(bytes[at..at + 4].try_into()?) as usize;
		packets.push(bytes[at + 4..at + 4 + len].to_vec());
		at += 4 + len;
	}
	let start = index_of(&from).ok_or_else(|| anyhow::anyhow!("unknown version {from}"))?;
	let end = match to {
		Some(v) => index_of(&v).ok_or_else(|| anyhow::anyhow!("unknown version {v}"))?,
		None => VERSIONS.len() - 1,
	};
	let mut cx = Session::new();
	let mut current: Vec<Vec<u8>> = packets;
	let mut version = start;
	while version != end {
		let next = if end > version {
			version + 1
		} else {
			version - 1
		};
		let mut out = Vec::new();
		let mut failed: BTreeMap<String, (usize, String)> = BTreeMap::new();
		let mut dropped: BTreeMap<String, usize> = BTreeMap::new();
		let mut spent: BTreeMap<String, std::time::Duration> = BTreeMap::new();
		for p in &current {
			let kind = inspect(Root::PlayToClient, version, p)
				.unwrap_or_else(|e| format!("undecodable ({e})"));
			let at = std::time::Instant::now();
			let result = step(Root::PlayToClient, version, next, p, &mut cx);
			*spent.entry(kind.clone()).or_default() += at.elapsed();
			match result {
				Ok(converted) if converted.is_empty() => *dropped.entry(kind).or_default() += 1,
				Ok(converted) => {
					for c in converted {
						match inspect(Root::PlayToClient, next, &c) {
							Ok(_) => out.push(c),
							Err(e) => {
								let entry = failed
									.entry(format!("{kind} decodes badly"))
									.or_insert((0, e));
								entry.0 += 1;
							}
						}
					}
				}
				Err(e) => {
					let entry = failed.entry(kind).or_insert((0, e.to_string()));
					entry.0 += 1;
				}
			}
		}
		let lost: usize = failed.values().map(|(n, _)| n).sum();
		println!(
			"{} -> {}: {} in, {} out, {} dropped, {lost} failed, {:.2?}",
			VERSIONS[version],
			VERSIONS[next],
			current.len(),
			out.len(),
			dropped.values().sum::<usize>(),
			spent.values().sum::<std::time::Duration>()
		);
		if spent.values().sum::<std::time::Duration>() > std::time::Duration::from_millis(50) {
			let mut slow: Vec<_> = spent.iter().collect();
			slow.sort_by(|a, b| b.1.cmp(a.1));
			for (kind, t) in slow.into_iter().take(2) {
				println!("    slow: {kind} {t:.2?}");
			}
		}
		for (kind, (n, why)) in &failed {
			println!("    {n:>4} {kind}: {why}");
		}
		current = out;
		version = next;
	}
	let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
	for p in &current {
		*kinds
			.entry(inspect(Root::PlayToClient, end, p).unwrap_or_default())
			.or_default() += 1;
	}
	println!("arrived at {}: {kinds:?}", VERSIONS[end]);
	Ok(())
}
