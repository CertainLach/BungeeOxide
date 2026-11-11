use bungeeoxide_converter::Version;
use std::time::{Duration, Instant};

use bungeeoxide_converter::gen::convert::{convert, inspect, step, Root};
use bungeeoxide_converter::Session;

fn chain(
	from: Version,
	to: Version,
	data: &[u8],
	cx: &mut Session,
) -> Result<Vec<Vec<u8>>, String> {
	let mut current = vec![data.to_vec()];
	let mut v = from;
	while v != to {
		let next = if to > v { v.next() } else { v.prev() }.expect("versions between endpoints");
		let mut out = Vec::new();
		for p in &current {
			out.extend(step(Root::PlayToClient, v, next, p, cx).map_err(|e| e.to_string())?);
		}
		current = out;
		v = next;
	}
	Ok(current)
}

struct Run {
	time: Duration,
	chunk_time: Duration,
	out: Vec<Vec<u8>>,
	failed: usize,
}

fn run(from: Version, to: Version, packets: &[Vec<u8>], virtual_path: bool) -> Run {
	let mut cx = Session::new();
	let mut r = Run {
		time: Duration::ZERO,
		chunk_time: Duration::ZERO,
		out: Vec::new(),
		failed: 0,
	};
	for p in packets {
		let chunk = inspect(Root::PlayToClient, from, p).is_ok_and(|n| n.starts_with("map_chunk"));
		let start = Instant::now();
		let result = match virtual_path {
			true => convert(Root::PlayToClient, from, to, p, &mut cx).map_err(|e| e.to_string()),
			false => chain(from, to, p, &mut cx),
		};
		let spent = start.elapsed();
		r.time += spent;
		if chunk {
			r.chunk_time += spent;
		}
		match result {
			Ok(out) => r.out.extend(out),
			Err(_) => r.failed += 1,
		}
	}
	r
}

fn main() -> anyhow::Result<()> {
	let mut args = std::env::args().skip(1);
	let path = args.next().expect("dump file");
	let version = |v: String| Version::of(&v).ok_or_else(|| anyhow::anyhow!("unknown version {v}"));
	let dump_version = args.next().map_or(Ok(Version::V1_8), version)?;
	let from = args.next().map_or(Ok(dump_version), version)?;
	let to = args.next().map_or(Ok(Version::V26_1), version)?;
	let output = args.next();
	let bytes = std::fs::read(path)?;
	let mut packets = Vec::new();
	let mut at = 0;
	while at + 4 <= bytes.len() {
		let len = u32::from_be_bytes(bytes[at..at + 4].try_into()?) as usize;
		packets.push(bytes[at + 4..at + 4 + len].to_vec());
		at += 4 + len;
	}
	if dump_version != from {
		let mut cx = Session::new();
		packets = packets
			.iter()
			.flat_map(|p| chain(dump_version, from, p, &mut cx).unwrap_or_default())
			.collect();
	}
	let chunks = packets
		.iter()
		.filter(|p| inspect(Root::PlayToClient, from, p).is_ok_and(|n| n.starts_with("map_chunk")))
		.count();
	let warm = run(from, to, &packets, true);
	let per_step = run(from, to, &packets, false);
	let carried = run(from, to, &packets, true);
	println!(
		"{} -> {}: {} packets, {chunks} chunk packets",
		from,
		to,
		packets.len()
	);
	println!(
		"  per-step: {:>10.2?} total, {:>10.2?} chunks, {} packets out, {} failed",
		per_step.time,
		per_step.chunk_time,
		per_step.out.len(),
		per_step.failed
	);
	println!(
		"  convert:  {:>10.2?} total, {:>10.2?} chunks, {} packets out, {} failed (first run {:.2?})",
		carried.time,
		carried.chunk_time,
		carried.out.len(),
		carried.failed,
		warm.time
	);
	if let Some(prefix) = output {
		let frame = |packets: &[Vec<u8>]| {
			packets
				.iter()
				.flat_map(|p| {
					(p.len() as u32)
						.to_be_bytes()
						.into_iter()
						.chain(p.iter().copied())
				})
				.collect::<Vec<u8>>()
		};
		std::fs::write(format!("{prefix}.step.bin"), frame(&per_step.out))?;
		std::fs::write(format!("{prefix}.convert.bin"), frame(&carried.out))?;
	}
	Ok(())
}
