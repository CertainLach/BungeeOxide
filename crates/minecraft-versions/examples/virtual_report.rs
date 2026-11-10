use std::time::{Duration, Instant};

use minecraft_versions::gen::convert::{convert, index_of, inspect, step, Root, VERSIONS};
use minecraft_versions::Session;

fn chain(from: usize, to: usize, data: &[u8], cx: &mut Session) -> Result<Vec<Vec<u8>>, String> {
	let mut current = vec![data.to_vec()];
	let mut v = from;
	while v != to {
		let next = if to > v { v + 1 } else { v - 1 };
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

fn run(from: usize, to: usize, packets: &[Vec<u8>], virtual_path: bool) -> Run {
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
	let dump_version = args.next().unwrap_or_else(|| "1.8".to_owned());
	let from = args.next().unwrap_or_else(|| dump_version.clone());
	let to = args.next().unwrap_or_else(|| "26.1".to_owned());
	let output = args.next();
	let version = |v: &str| index_of(v).ok_or_else(|| anyhow::anyhow!("unknown version {v}"));
	let (dump_version, from, to) = (version(&dump_version)?, version(&from)?, version(&to)?);
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
		VERSIONS[from],
		VERSIONS[to],
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
