use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context as _, Result};
use bungeeoxide_authlib::accounts::{default_prism_file, home, Account, Source};
use bungeeoxide_authlib::{hyphenated, parse_uuid, Endpoints, Secret};
use serde::Deserialize;
use tracing::warn;

#[derive(Clone)]
pub struct User {
	pub username: String,
	pub uuid: Option<u128>,
	pub upstream: Option<Arc<Account>>,
	pub downstream: bool,
	pub allow_goto: bool,
}

impl User {
	pub fn guest(username: &str) -> Self {
		Self {
			username: username.to_owned(),
			uuid: None,
			upstream: None,
			downstream: false,
			allow_goto: false,
		}
	}

	pub fn admit(&self, uuid: Option<u128>) -> Result<()> {
		match (self.uuid, uuid) {
			(Some(wanted), Some(uuid)) if wanted != uuid => bail!(
				"{} is {}, not {}",
				self.username,
				hyphenated(wanted),
				hyphenated(uuid)
			),
			(Some(wanted), None) => bail!(
				"{} is {}, but the client did not tell its UUID",
				self.username,
				hyphenated(wanted)
			),
			_ => Ok(()),
		}
	}
}

pub struct Users {
	pub endpoints: Endpoints,
	pub users: Vec<User>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
	cache: Option<PathBuf>,
	#[serde(default)]
	endpoints: Endpoints,
	#[serde(default)]
	user: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
	username: String,
	uuid: Option<String>,
	upstream: Option<Upstream>,
	downstream: bool,
	allow_goto: bool,
}

#[derive(Deserialize)]
#[serde(tag = "source", rename_all = "lowercase", deny_unknown_fields)]
enum Upstream {
	#[serde(alias = "multimc")]
	Prism {
		file: Option<PathBuf>,
		profile: Option<String>,
		msa_client_id: Option<String>,
	},
	Token {
		access_token: Secret,
		uuid: Option<String>,
		name: Option<String>,
	},
	Process {
		profile: Option<String>,
	},
}

fn resolve(dir: &Path, path: &Path) -> PathBuf {
	match path.strip_prefix("~") {
		Ok(rest) => home().join(rest),
		Err(_) => dir.join(path),
	}
}

fn uuid(username: &str, text: &str) -> Result<u128> {
	parse_uuid(text).ok_or_else(|| anyhow!("user {username}: {text:?} is not a UUID"))
}

pub fn load(path: &Path) -> Result<Users> {
	let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
	let users = parse(&text, path).with_context(|| format!("reading {}", path.display()))?;
	let shared = fs::metadata(path)
		.map(|m| m.permissions().mode() & 0o077 != 0)
		.unwrap_or(false);
	if shared
		&& users
			.users
			.iter()
			.any(|u| u.upstream.as_ref().is_some_and(|a| a.has_tokens()))
	{
		warn!(
			"{} holds access tokens and other users can read it, chmod 600 it",
			path.display()
		);
	}
	Ok(users)
}

pub fn parse(text: &str, path: &Path) -> Result<Users> {
	let file: File = toml::from_str(text)?;
	let dir = path.parent().unwrap_or(Path::new("."));
	let cache = match &file.cache {
		Some(cache) => resolve(dir, cache),
		None => path.with_extension("tokens.json"),
	};
	let mut users: Vec<User> = Vec::new();
	for entry in file.user {
		let name = entry.username;
		if users.iter().any(|u| u.username.eq_ignore_ascii_case(&name)) {
			bail!("user {name} is listed twice");
		}
		let source = match entry.upstream {
			None => None,
			Some(Upstream::Prism {
				file,
				profile,
				msa_client_id,
			}) => Some(Source::Prism {
				file: file
					.as_deref()
					.map(|f| resolve(dir, f))
					.unwrap_or_else(default_prism_file),
				profile,
				client_id: msa_client_id,
			}),
			Some(Upstream::Token {
				access_token,
				uuid: profile,
				name: profile_name,
			}) => {
				if access_token.is_empty() {
					bail!("user {name} has an empty upstream access_token");
				}
				Some(Source::Token {
					token: access_token,
					uuid: profile.map(|u| uuid(&name, &u)).transpose()?,
					name: profile_name,
				})
			}
			Some(Upstream::Process { profile }) => Some(Source::Process { profile }),
		};
		users.push(User {
			uuid: entry.uuid.map(|u| uuid(&name, &u)).transpose()?,
			upstream: source
				.map(|s| Arc::new(Account::new(s, file.endpoints.clone(), Some(cache.clone())))),
			downstream: entry.downstream,
			allow_goto: entry.allow_goto,
			username: name,
		});
	}
	Ok(Users {
		endpoints: file.endpoints,
		users,
	})
}

pub fn find(users: &[User], username: &str) -> User {
	users
		.iter()
		.find(|u| u.username.eq_ignore_ascii_case(username))
		.cloned()
		.unwrap_or_else(|| User::guest(username))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn users_get_their_upstream_accounts_and_permissions() {
		let config = r#"
			[endpoints]
			session = "http://127.0.0.1:25661"

			[[user]]
			username = "Steve"
			uuid = "853c80ef-3c37-49fd-aa49-938b674adae6"
			downstream = true
			allow_goto = true
			upstream = { source = "prism", file = "fake-accounts.json", profile = "Notch" }

			[[user]]
			username = "Alex"
			downstream = false
			allow_goto = false

			[user.upstream]
			source = "token"
			access_token = "fake-token"
			uuid = "069a79f444e94726a5befca90e38aaf5"
			name = "Notch"

			[[user]]
			username = "Bob"
			downstream = false
			allow_goto = true
		"#;
		let parsed = parse(config, Path::new("/srv/bridge/users.toml")).unwrap();
		assert_eq!(parsed.endpoints.session, "http://127.0.0.1:25661");
		assert_eq!(parsed.endpoints.services, "https://api.minecraftservices.com");
		let steve = find(&parsed.users, "steve");
		assert_eq!(steve.uuid, Some(0x853c80ef3c3749fdaa49938b674adae6));
		assert!(steve.downstream && steve.allow_goto);
		assert_eq!(steve.upstream.as_ref().unwrap().label, "Notch");
		assert!(steve.admit(Some(0x853c80ef3c3749fdaa49938b674adae6)).is_ok());
		assert!(steve.admit(Some(1)).is_err());
		assert!(steve.admit(None).is_err());
		let alex = find(&parsed.users, "Alex");
		let credentials = alex.upstream.as_ref().unwrap().credentials(false).unwrap();
		assert_eq!(
			(credentials.uuid, credentials.name.as_str()),
			(0x069a79f444e94726a5befca90e38aaf5, "Notch")
		);
		assert!(!alex.allow_goto && alex.admit(None).is_ok());
		let bob = find(&parsed.users, "Bob");
		assert!(bob.upstream.is_none() && bob.allow_goto);
		let guest = find(&parsed.users, "Herobrine");
		assert!(guest.upstream.is_none() && !guest.downstream && !guest.allow_goto);
	}

	#[test]
	fn malformed_users_are_rejected() {
		let user = |rest: &str| {
			format!("[[user]]\nusername = \"a\"\ndownstream = false\nallow_goto = false\n{rest}")
		};
		for wrong in [
			user("upstream = { source = \"token\", access_token = \"t\", file = \"x\" }"),
			user("upstream = { source = \"magic\" }"),
			user("upstream = { source = \"token\", access_token = \"\" }"),
			user("uuid = \"not a uuid\""),
			format!("{}{}", user(""), user("")),
			"[[user]]\nusername = \"a\"\n".to_owned(),
		] {
			assert!(parse(&wrong, Path::new("a.toml")).is_err(), "{wrong}");
		}
	}
}
