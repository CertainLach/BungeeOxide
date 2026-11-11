use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, warn};

use crate::{now_millis, now_secs, parse_uuid, Endpoints, Joined, PlayerKeys, Secret};

const MARGIN_SECS: i64 = 120;

pub struct Account {
	pub label: String,
	source: Source,
	endpoints: Endpoints,
	cache: Option<PathBuf>,
	state: Mutex<State>,
}

pub enum Source {
	Prism {
		file: PathBuf,
		profile: Option<String>,
		client_id: Option<String>,
	},
	Token {
		token: Secret,
		uuid: Option<u128>,
		name: Option<String>,
	},
	Process {
		profile: Option<String>,
	},
}

#[derive(Default)]
struct State {
	credentials: Option<Credentials>,
	keys: Option<Arc<PlayerKeys>>,
}

#[derive(Clone, Debug)]
pub struct Credentials {
	pub uuid: u128,
	pub name: String,
	pub token: Secret,
	pub expires: Option<i64>,
}

impl Credentials {
	fn fresh(&self) -> bool {
		self.expires.is_none_or(|e| e > now_secs() + MARGIN_SECS)
	}
}

pub fn home() -> PathBuf {
	std::env::var_os("HOME")
		.map(PathBuf::from)
		.unwrap_or_default()
}

pub fn default_prism_file() -> PathBuf {
	let data = std::env::var_os("XDG_DATA_HOME")
		.map(PathBuf::from)
		.unwrap_or_else(|| home().join(".local/share"));
	data.join("PrismLauncher/accounts.json")
}

impl Account {
	pub fn new(source: Source, endpoints: Endpoints, cache: Option<PathBuf>) -> Self {
		let label = match &source {
			Source::Prism {
				profile: Some(name), ..
			}
			| Source::Process {
				profile: Some(name),
			}
			| Source::Token {
				name: Some(name), ..
			} => name.clone(),
			Source::Prism { file, .. } => format!("the active account of {}", file.display()),
			Source::Token { .. } => "a configured access token".to_owned(),
			Source::Process { .. } => "a running game".to_owned(),
		};
		Self {
			label,
			source,
			endpoints,
			cache,
			state: Mutex::default(),
		}
	}

	pub fn has_tokens(&self) -> bool {
		matches!(self.source, Source::Token { .. })
	}

	pub fn endpoints(&self) -> &Endpoints {
		&self.endpoints
	}

	fn state(&self) -> std::sync::MutexGuard<'_, State> {
		self.state.lock().unwrap_or_else(|e| e.into_inner())
	}

	pub fn credentials(&self, force: bool) -> Result<Credentials> {
		let mut state = self.state();
		if !force {
			if let Some(credentials) = state.credentials.as_ref().filter(|c| c.fresh()) {
				return Ok(credentials.clone());
			}
		}
		let credentials = match &self.source {
			Source::Prism {
				file,
				profile,
				client_id,
			} => self.prism(file, profile.as_deref(), client_id.as_deref(), force)?,
			Source::Token { token, uuid, name } => {
				if force {
					bail!(
						"the session server rejected the access token configured for {}",
						self.label
					);
				}
				let (uuid, name) = match (uuid, name) {
					(Some(uuid), Some(name)) => (*uuid, name.clone()),
					_ => crate::profile(&self.endpoints, token)?,
				};
				Credentials {
					uuid,
					name,
					token: token.clone(),
					expires: None,
				}
			}
			Source::Process { profile } => running_game(profile.as_deref(), &self.endpoints)?,
		};
		if state.credentials.as_ref().map(|c| c.uuid) != Some(credentials.uuid) {
			state.keys = None;
		}
		state.credentials = Some(credentials.clone());
		Ok(credentials)
	}

	pub fn keys(&self, credentials: &Credentials) -> Result<(Credentials, Arc<PlayerKeys>)> {
		let now = now_millis();
		if let Some(keys) = self
			.state()
			.keys
			.as_ref()
			.filter(|k| now < k.refresh_at && now + 60_000 < k.expires_at)
		{
			return Ok((credentials.clone(), keys.clone()));
		}
		let (credentials, keys) = match crate::certificates(&self.endpoints, &credentials.token) {
			Err(e) if e.is::<crate::Unauthorized>() => {
				warn!(
					"Minecraft services rejected {}'s token, refreshing it",
					credentials.name
				);
				let credentials = self.credentials(true)?;
				let keys = crate::certificates(&self.endpoints, &credentials.token)?;
				(credentials, keys)
			}
			fetched => (credentials.clone(), fetched?),
		};
		let keys = Arc::new(keys);
		self.state().keys = Some(keys.clone());
		Ok((credentials, keys))
	}

	pub fn join(&self, credentials: &Credentials, server_hash: &str) -> Result<Credentials> {
		if let Joined::Yes = crate::join(
			&self.endpoints,
			&credentials.token,
			credentials.uuid,
			server_hash,
		)? {
			return Ok(credentials.clone());
		}
		warn!(
			"the session server rejected {}'s token, refreshing it",
			credentials.name
		);
		let credentials = self.credentials(true)?;
		match crate::join(
			&self.endpoints,
			&credentials.token,
			credentials.uuid,
			server_hash,
		)? {
			Joined::Yes => Ok(credentials),
			Joined::InvalidToken => bail!(
				"the session server rejected {}'s refreshed token",
				credentials.name
			),
		}
	}

	fn prism(
		&self,
		file: &Path,
		profile: Option<&str>,
		client_id: Option<&str>,
		force: bool,
	) -> Result<Credentials> {
		let text =
			fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
		let account = pick(prism_accounts(&text)?, profile)
			.with_context(|| format!("in {}", file.display()))?;
		let key = format!("{:032x}", account.id);
		let cached = self
			.cache
			.as_deref()
			.and_then(|path| load_cache(path).tokens.remove(&key));
		let credentials = |token: Secret, expires: i64| Credentials {
			uuid: account.id,
			name: account.name.clone(),
			token,
			expires: Some(expires),
		};
		if !force {
			let best = [
				account.token.clone().map(|t| (t, account.expires)),
				cached.as_ref().map(|c| (c.access_token.clone(), c.expires)),
			]
			.into_iter()
			.flatten()
			.max_by_key(|(_, expires)| *expires);
			if let Some((token, expires)) = best.filter(|(_, e)| *e > now_secs() + MARGIN_SECS) {
				return Ok(credentials(token, expires));
			}
		}
		let mut refresh_tokens = Vec::new();
		if let Some(cached) = &cached {
			if let Some(token) = &cached.refresh_token {
				refresh_tokens.push((token.clone(), cached.client_id.clone()));
			}
		}
		let mut unknown_client = false;
		if let Some(token) = &account.refresh {
			if !refresh_tokens.iter().any(|(t, _)| t == token) {
				match account.client_id.as_deref().or(client_id) {
					Some(id) => refresh_tokens.push((token.clone(), id.to_owned())),
					None => unknown_client = true,
				}
			}
		}
		if refresh_tokens.is_empty() && unknown_client {
			bail!(
				"{}'s Minecraft token has expired and {} has no msa-client-id for its refresh token, set msa_client_id on the account to the Microsoft client ID of the launcher",
				account.name,
				file.display()
			);
		}
		if refresh_tokens.is_empty() {
			bail!("{}'s Minecraft token has expired and there is no refresh token, log in with the launcher again", account.name);
		}
		let mut failure = None;
		for (refresh_token, client_id) in refresh_tokens {
			match crate::refresh(&self.endpoints, &client_id, &refresh_token) {
				Ok(refreshed) => {
					info!("refreshed {}'s Minecraft token", account.name);
					if let Some(path) = &self.cache {
						let entry = Cached {
							name: account.name.clone(),
							access_token: refreshed.access_token.clone(),
							expires: refreshed.expires_at,
							refresh_token: Some(refreshed.refresh_token.unwrap_or(refresh_token)),
							client_id,
						};
						if let Err(e) = store_cache(path, &key, entry) {
							warn!("can't write the token cache {}: {e:#}", path.display());
						}
					}
					return Ok(credentials(refreshed.access_token, refreshed.expires_at));
				}
				Err(e) => failure = Some(e),
			}
		}
		Err(failure
			.unwrap_or_else(|| anyhow!("no refresh token"))
			.context(format!("refreshing {}'s Minecraft token", account.name)))
	}
}

#[derive(Debug)]
pub struct PrismAccount {
	pub id: u128,
	pub name: String,
	pub token: Option<Secret>,
	pub expires: i64,
	pub refresh: Option<Secret>,
	pub client_id: Option<String>,
	pub active: bool,
}

pub fn prism_accounts(json: &str) -> Result<Vec<PrismAccount>> {
	let value: Value =
		serde_json::from_str(json).map_err(|e| anyhow!("not a launcher accounts.json: {e}"))?;
	let list = value
		.get("accounts")
		.and_then(Value::as_array)
		.ok_or_else(|| anyhow!("not a launcher accounts.json: no accounts list"))?;
	let text = |v: &Value, path: &str| {
		v.pointer(path)
			.and_then(Value::as_str)
			.filter(|s| !s.is_empty())
			.map(str::to_owned)
	};
	let mut out = Vec::new();
	for account in list {
		if account.get("type").and_then(Value::as_str) != Some("MSA") {
			continue;
		}
		let Some(id) = text(account, "/profile/id").and_then(|id| parse_uuid(&id)) else {
			continue;
		};
		out.push(PrismAccount {
			id,
			name: text(account, "/profile/name").unwrap_or_default(),
			token: text(account, "/ygg/token").map(Secret::new),
			expires: account
				.pointer("/ygg/exp")
				.and_then(Value::as_i64)
				.unwrap_or_default(),
			refresh: text(account, "/msa/refresh_token").map(Secret::new),
			client_id: text(account, "/msa-client-id"),
			active: account
				.get("active")
				.and_then(Value::as_bool)
				.unwrap_or_default(),
		});
	}
	Ok(out)
}

pub fn pick(accounts: Vec<PrismAccount>, profile: Option<&str>) -> Result<PrismAccount> {
	let names = accounts
		.iter()
		.map(|a| a.name.clone())
		.collect::<Vec<_>>()
		.join(", ");
	let index = match profile {
		Some(wanted) => {
			let uuid = parse_uuid(wanted);
			accounts
				.iter()
				.position(|a| a.name.eq_ignore_ascii_case(wanted) || Some(a.id) == uuid)
		}
		None => accounts
			.iter()
			.position(|a| a.active)
			.or_else(|| (accounts.len() == 1).then_some(0)),
	};
	let Some(index) = index else {
		match profile {
			Some(wanted) => bail!("no Microsoft account {wanted:?}, there are: {names}"),
			None => bail!("no active Microsoft account, set profile to one of: {names}"),
		}
	};
	Ok(accounts
		.into_iter()
		.nth(index)
		.expect("the index is in range"))
}

#[derive(Debug, PartialEq)]
pub struct GameArguments {
	pub token: Secret,
	pub uuid: Option<u128>,
	pub name: Option<String>,
}

pub fn game_arguments(args: &[String]) -> Option<GameArguments> {
	let value = |flag: &str| {
		args.iter()
			.position(|a| a == flag)
			.and_then(|i| args.get(i + 1))
			.cloned()
	};
	let token = value("--accessToken").filter(|t| !t.is_empty() && t != "0")?;
	Some(GameArguments {
		token: Secret::new(token),
		uuid: value("--uuid").and_then(|u| parse_uuid(&u)),
		name: value("--username"),
	})
}

fn running_game(profile: Option<&str>, endpoints: &Endpoints) -> Result<Credentials> {
	let mut found = Vec::new();
	for entry in fs::read_dir("/proc").context("listing /proc")?.flatten() {
		if !entry
			.file_name()
			.to_string_lossy()
			.bytes()
			.all(|b| b.is_ascii_digit())
		{
			continue;
		}
		let Ok(cmdline) = fs::read(entry.path().join("cmdline")) else {
			continue;
		};
		let args = cmdline
			.split(|b| *b == 0)
			.map(|a| String::from_utf8_lossy(a).into_owned())
			.collect::<Vec<_>>();
		if let Some(game) = game_arguments(&args) {
			found.push(game);
		}
	}
	let wanted = profile.map(|p| (p, parse_uuid(p)));
	let game = found
		.into_iter()
		.find(|g| match wanted {
			Some((name, uuid)) => {
				g.name
					.as_deref()
					.is_some_and(|n| n.eq_ignore_ascii_case(name))
					|| (uuid.is_some() && g.uuid == uuid)
			}
			None => true,
		})
		.ok_or_else(|| match profile {
			Some(p) => anyhow!("no running game of {p} with --accessToken on its command line"),
			None => anyhow!("no running game with --accessToken on its command line"),
		})?;
	let (uuid, name) = match (game.uuid, game.name) {
		(Some(uuid), Some(name)) => (uuid, name),
		_ => crate::profile(endpoints, &game.token)?,
	};
	Ok(Credentials {
		uuid,
		name,
		token: game.token,
		expires: None,
	})
}

#[derive(Default, Serialize, Deserialize)]
struct Cache {
	#[serde(default)]
	tokens: BTreeMap<String, Cached>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Cached {
	name: String,
	access_token: Secret,
	expires: i64,
	refresh_token: Option<Secret>,
	client_id: String,
}

static CACHE_LOCK: Mutex<()> = Mutex::new(());

fn load_cache(path: &Path) -> Cache {
	match fs::read_to_string(path) {
		Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
			warn!(
				"ignoring the unreadable token cache {}: {e}",
				path.display()
			);
			Cache::default()
		}),
		Err(_) => Cache::default(),
	}
}

fn store_cache(path: &Path, key: &str, entry: Cached) -> Result<()> {
	let _guard = CACHE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
	let mut cache = load_cache(path);
	cache.tokens.insert(key.to_owned(), entry);
	let text = serde_json::to_string_pretty(&cache)?;
	let temp = path.with_extension("tmp");
	let mut file = fs::OpenOptions::new()
		.write(true)
		.create(true)
		.truncate(true)
		.mode(0o600)
		.open(&temp)
		.with_context(|| format!("creating {}", temp.display()))?;
	file.set_permissions(fs::Permissions::from_mode(0o600))?;
	file.write_all(text.as_bytes())?;
	file.sync_all()?;
	fs::rename(&temp, path).with_context(|| format!("replacing {}", path.display()))?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	const PRISM: &str = r#"{
		"accounts": [
			{
				"type": "Offline",
				"profile": { "id": "00000000000000000000000000000001", "name": "Offline" },
				"ygg": { "token": "0" }
			},
			{
				"active": true,
				"type": "MSA",
				"msa-client-id": "fake-client-id",
				"msa": { "exp": 1700003600, "iat": 1700000000, "refresh_token": "M.fake-refresh", "token": "fake-msa" },
				"utoken": { "exp": 1700086400, "extra": { "uhs": "123" }, "token": "fake-user" },
				"xrp-mc": { "exp": 1700086400, "extra": { "uhs": "123" }, "token": "fake-xsts" },
				"ygg": { "exp": 1700086400, "iat": 1700000000, "token": "fake-minecraft-token" },
				"profile": {
					"id": "069a79f444e94726a5befca90e38aaf5",
					"name": "Notch",
					"capes": [],
					"skin": { "id": "x", "url": "", "variant": "CLASSIC" }
				},
				"entitlement": { "canPlayMinecraft": true, "ownsMinecraft": true }
			},
			{
				"type": "MSA",
				"msa": { "refresh_token": "M.other" },
				"ygg": { "exp": 1, "token": "old" },
				"profile": { "id": "853c80ef3c3749fdaa49938b674adae6", "name": "jeb_" }
			}
		],
		"formatVersion": 3
	}"#;

	#[test]
	fn prism_accounts_give_profile_tokens_and_refresh_tokens() {
		let accounts = prism_accounts(PRISM).unwrap();
		assert_eq!(accounts.len(), 2);
		let notch = &accounts[0];
		assert_eq!(notch.id, 0x069a79f444e94726a5befca90e38aaf5);
		assert_eq!(notch.name, "Notch");
		assert_eq!(notch.token, Some(Secret::new("fake-minecraft-token")));
		assert_eq!(notch.expires, 1_700_086_400);
		assert_eq!(notch.refresh, Some(Secret::new("M.fake-refresh")));
		assert_eq!(notch.client_id.as_deref(), Some("fake-client-id"));
		assert!(notch.active);
		assert_eq!(accounts[1].client_id, None);
		assert!(!format!("{accounts:?}").contains("fake-minecraft-token"));
		assert_eq!(
			pick(prism_accounts(PRISM).unwrap(), None).unwrap().name,
			"Notch"
		);
		assert_eq!(
			pick(prism_accounts(PRISM).unwrap(), Some("JEB_"))
				.unwrap()
				.name,
			"jeb_"
		);
		let by_uuid = pick(
			prism_accounts(PRISM).unwrap(),
			Some("853c80ef-3c37-49fd-aa49-938b674adae6"),
		)
		.unwrap();
		assert_eq!(by_uuid.name, "jeb_");
		assert!(pick(prism_accounts(PRISM).unwrap(), Some("Dinnerbone")).is_err());
		assert!(prism_accounts("{}").is_err());
	}

	#[test]
	fn game_command_lines_give_tokens() {
		let args: Vec<String> = [
			"java",
			"-Xmx2G",
			"net.minecraft.client.main.Main",
			"--username",
			"Notch",
			"--uuid",
			"069a79f444e94726a5befca90e38aaf5",
			"--accessToken",
			"fake-token",
			"--version",
			"1.21",
		]
		.iter()
		.map(|s| s.to_string())
		.collect();
		assert_eq!(
			game_arguments(&args),
			Some(GameArguments {
				token: Secret::new("fake-token"),
				uuid: Some(0x069a79f444e94726a5befca90e38aaf5),
				name: Some("Notch".to_owned()),
			})
		);
		assert_eq!(game_arguments(&args[..8]), None);
	}

	#[test]
	fn token_accounts_need_no_requests_when_they_know_the_profile() {
		let account = Account::new(
			Source::Token {
				token: Secret::new("fake-token"),
				uuid: Some(0x069a79f444e94726a5befca90e38aaf5),
				name: Some("Notch".to_owned()),
			},
			Endpoints::default(),
			None,
		);
		assert_eq!(account.label, "Notch");
		assert!(account.has_tokens());
		let credentials = account.credentials(false).unwrap();
		assert_eq!(
			(credentials.uuid, credentials.name.as_str()),
			(0x069a79f444e94726a5befca90e38aaf5, "Notch")
		);
		assert!(!format!("{credentials:?}").contains("fake-token"));
	}

	#[test]
	fn prism_tokens_are_used_until_they_expire_and_the_cache_wins_when_newer() {
		let dir = std::env::temp_dir().join(format!("pds2-bridge-accounts-{}", std::process::id()));
		fs::create_dir_all(&dir).unwrap();
		let future = now_secs() + 3600;
		let prism = PRISM.replace(
			"1700086400, \"iat\": 1700000000, \"token\": \"fake-minecraft-token\"",
			&format!("{future}, \"iat\": 1700000000, \"token\": \"fake-minecraft-token\""),
		);
		fs::write(dir.join("accounts.json"), &prism).unwrap();
		let steve = Account::new(
			Source::Prism {
				file: dir.join("accounts.json"),
				profile: None,
				client_id: None,
			},
			Endpoints::default(),
			Some(dir.join("bridge.tokens.json")),
		);
		let credentials = steve.credentials(false).unwrap();
		assert_eq!(credentials.token.expose(), "fake-minecraft-token");
		assert_eq!(credentials.name, "Notch");
		let cache = dir.join("bridge.tokens.json");
		let newer = Cached {
			name: "Notch".to_owned(),
			access_token: Secret::new("cached-token"),
			expires: future + 100,
			refresh_token: None,
			client_id: "fake-client-id".to_owned(),
		};
		store_cache(&cache, "069a79f444e94726a5befca90e38aaf5", newer).unwrap();
		assert_eq!(
			fs::metadata(&cache).unwrap().permissions().mode() & 0o777,
			0o600
		);
		assert_eq!(
			steve.credentials(false).unwrap().token.expose(),
			"fake-minecraft-token"
		);
		assert_eq!(
			steve
				.prism(&dir.join("accounts.json"), None, None, false)
				.unwrap()
				.token
				.expose(),
			"cached-token"
		);
		assert_eq!(
			fs::read_to_string(dir.join("accounts.json")).unwrap(),
			prism
		);
		let unknown = steve
			.prism(&dir.join("accounts.json"), Some("jeb_"), None, false)
			.unwrap_err();
		assert!(format!("{unknown:#}").contains("msa_client_id"), "{unknown:#}");
		fs::remove_dir_all(&dir).unwrap();
	}
}
