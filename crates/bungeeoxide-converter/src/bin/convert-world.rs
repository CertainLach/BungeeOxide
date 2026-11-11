use std::path::Path;
use std::process::ExitCode;

use bungeeoxide_converter::{world, Version};

fn main() -> ExitCode {
	let args: Vec<String> = std::env::args().skip(1).collect();
	let [src, dst] = args.as_slice() else {
		eprintln!("usage: convert-world <source world> <new world>");
		return ExitCode::from(2);
	};
	match world::convert_world(Path::new(src), Path::new(dst)) {
		Ok(report) => {
			println!(
				"{} chunks in {} regions converted to {}",
				report.chunks,
				report.regions,
				Version::LATEST
			);
			for (source, n) in &report.sources {
				println!("  {n} from {source}");
			}
			if !report.skipped.is_empty() {
				eprintln!("{} chunks skipped:", report.skipped.len());
				for why in report.skipped.iter().take(20) {
					eprintln!("  {why}");
				}
			}
			ExitCode::SUCCESS
		}
		Err(e) => {
			eprintln!("{e}");
			ExitCode::FAILURE
		}
	}
}
