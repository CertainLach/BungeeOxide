use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use bungeeoxide::{serve, Config};
use bungeeoxide_authlib::accounts::Accounts;
use bungeeoxide_converter::Version;
use clap::Parser;
use tokio::net::TcpListener;

#[derive(Parser)]
struct Opts {
	/// Address to accept players on
	#[arg(long, default_value = "127.0.0.1:25577")]
	listen: String,
	/// Backend server (host:port), running with online-mode=false unless players are mapped with --accounts
	#[arg(long)]
	server: String,
	/// Server list description, shown when the backend server does not answer the server list ping
	#[arg(long, default_value = "pds2 bridge")]
	motd: String,
	/// Seconds to reuse the backend's server list reply
	#[arg(long, default_value_t = 5.0)]
	status_cache: f64,
	/// Packets at or over this size are compressed towards the client
	#[arg(long, default_value_t = 256)]
	compression_threshold: usize,
	/// Minecraft version of the backend server
	#[arg(long, default_value = "1.8")]
	server_version: Version,
	/// Language files from `just languages`, to show translated text to old clients [default: target/minecraft-languages of the workspace, if present]
	#[arg(long)]
	languages: Option<PathBuf>,
	/// TOML file mapping players to Minecraft accounts the bridge logs in upstream with
	#[arg(long)]
	accounts: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt().without_time().init();
	let opts = Opts::parse();
	let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/minecraft-languages");
	match opts
		.languages
		.or_else(|| workspace.join("index.tsv").exists().then_some(workspace))
	{
		Some(dir) => {
			tracing::info!("translating text for old clients with {}", dir.display());
			bungeeoxide_converter::languages::load(dir);
		}
		None => tracing::info!(
			"no language files, old clients see translation keys (run just languages)"
		),
	}
	let accounts = match &opts.accounts {
		Some(path) => Accounts::load(path)?,
		None => Accounts::default(),
	};
	let listener = TcpListener::bind(&opts.listen).await?;
	tracing::info!(
		"listening on {}, forwarding to {}",
		opts.listen,
		opts.server
	);
	serve(
		listener,
		Arc::new(Config {
			target: opts.server,
			motd: opts.motd,
			threshold: opts.compression_threshold,
			server_version: opts.server_version,
			accounts,
			status_cache: Duration::try_from_secs_f64(opts.status_cache)?,
			status: Default::default(),
		}),
	)
	.await
}
