use std::path::PathBuf;
use std::process::ExitCode;

use bungeeoxide_converter::{world, Version};
use clap::Parser;

/// Convert a Minecraft world to the latest supported version
#[derive(Parser)]
struct Opts {
	/// World to read: a McRegion (1.0, 1.1) or Anvil save
	source: PathBuf,
	/// Where to write the converted world, has to be missing or empty
	target: PathBuf,
}

fn main() -> ExitCode {
	let opts = Opts::parse();
	match world::convert_world(&opts.source, &opts.target) {
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
