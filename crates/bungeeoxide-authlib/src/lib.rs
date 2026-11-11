use std::fmt;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Result};
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use base64::Engine as _;
use md5::Md5;
use rsa::pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePublicKey};
use rsa::{Pkcs1v15Sign, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::Sha1;
use sha2::{Digest, Sha256};

pub mod accounts;

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
	pub fn new(value: impl Into<String>) -> Self {
		Self(value.into())
	}

	pub fn expose(&self) -> &str {
		&self.0
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

impl fmt::Debug for Secret {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("<redacted>")
	}
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Endpoints {
	pub session: String,
	pub services: String,
	pub microsoft: String,
	pub xbox_user: String,
	pub xbox_xsts: String,
}

impl Default for Endpoints {
	fn default() -> Self {
		Self {
			session: "https://sessionserver.mojang.com".to_owned(),
			services: "https://api.minecraftservices.com".to_owned(),
			microsoft: "https://login.microsoftonline.com".to_owned(),
			xbox_user: "https://user.auth.xboxlive.com".to_owned(),
			xbox_xsts: "https://xsts.auth.xboxlive.com".to_owned(),
		}
	}
}

impl Endpoints {
	fn url(base: &str, path: &str) -> String {
		format!("{}{path}", base.trim_end_matches('/'))
	}
}

pub fn offline_uuid_bits(name: &str) -> u128 {
	let mut b: [u8; 16] = Md5::digest(format!("OfflinePlayer:{name}")).into();
	b[6] = (b[6] & 0x0f) | 0x30;
	b[8] = (b[8] & 0x3f) | 0x80;
	u128::from_be_bytes(b)
}

pub fn offline_uuid(name: &str) -> String {
	hyphenated(offline_uuid_bits(name))
}

pub fn now_millis() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_millis() as i64)
		.unwrap_or_default()
}

pub fn now_secs() -> i64 {
	now_millis() / 1000
}

pub fn parse_uuid(text: &str) -> Option<u128> {
	let hex: String = text.chars().filter(|c| *c != '-').collect();
	if hex.len() != 32 {
		return None;
	}
	u128::from_str_radix(&hex, 16).ok()
}

pub fn hyphenated(uuid: u128) -> String {
	let hex = format!("{uuid:032x}");
	format!(
		"{}-{}-{}-{}-{}",
		&hex[0..8],
		&hex[8..12],
		&hex[12..16],
		&hex[16..20],
		&hex[20..32]
	)
}

pub fn server_hash(server_id: &str, secret: &[u8], public_key: &[u8]) -> String {
	let mut sha = Sha1::new();
	sha.update(server_id.as_bytes());
	sha.update(secret);
	sha.update(public_key);
	let mut digest: [u8; 20] = sha.finalize().into();
	let negative = digest[0] & 0x80 != 0;
	if negative {
		let mut carry = true;
		for b in digest.iter_mut().rev() {
			*b = !*b;
			if carry {
				let (sum, overflow) = b.overflowing_add(1);
				*b = sum;
				carry = overflow;
			}
		}
	}
	let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
	let hex = match hex.trim_start_matches('0') {
		"" => "0",
		trimmed => trimmed,
	};
	format!("{}{hex}", if negative { "-" } else { "" })
}

pub fn parse_time(text: &str) -> Option<i64> {
	let (date, time) = text.trim().split_once(['T', ' '])?;
	let mut date = date.split('-');
	let year: i64 = date.next()?.parse().ok()?;
	let month: i64 = date.next()?.parse().ok()?;
	let day: i64 = date.next()?.parse().ok()?;
	let (time, offset) = if let Some(time) = time.strip_suffix(['Z', 'z']) {
		(time, 0)
	} else if let Some(at) = time.rfind(['+', '-']) {
		let (time, zone) = time.split_at(at);
		let sign = if zone.starts_with('-') { -1 } else { 1 };
		let (h, m) = zone[1..].split_once(':').unwrap_or((&zone[1..], "0"));
		(
			time,
			sign * (h.parse::<i64>().ok()? * 60 + m.parse::<i64>().ok()?) * 60_000,
		)
	} else {
		(time, 0)
	};
	let (clock, fraction) = time.split_once('.').unwrap_or((time, ""));
	let mut clock = clock.split(':');
	let hour: i64 = clock.next()?.parse().ok()?;
	let minute: i64 = clock.next()?.parse().ok()?;
	let second: i64 = clock.next().unwrap_or("0").parse().ok()?;
	let millis: i64 = format!("{:0<3}", fraction.chars().take(3).collect::<String>())
		.parse()
		.ok()?;
	let y = if month <= 2 { year - 1 } else { year };
	let era = y.div_euclid(400);
	let yoe = y - era * 400;
	let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
	let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
	let days = era * 146_097 + doe - 719_468;
	Some(((days * 24 + hour) * 60 + minute) * 60_000 + second * 1000 + millis - offset)
}

fn agent() -> &'static ureq::Agent {
	static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
	AGENT.get_or_init(|| {
		ureq::Agent::config_builder()
			.http_status_as_error(false)
			.timeout_global(Some(Duration::from_secs(20)))
			.user_agent("pds2-bridge")
			.build()
			.into()
	})
}

struct Reply {
	status: u16,
	body: String,
}

impl Reply {
	fn ok(&self) -> bool {
		(200..300).contains(&self.status)
	}

	fn json(&self) -> Result<Value> {
		serde_json::from_str(&self.body)
			.map_err(|e| anyhow!("unreadable response ({}): {e}", self.status))
	}

	fn error(&self) -> String {
		let detail = serde_json::from_str::<Value>(&self.body)
			.ok()
			.and_then(|v| {
				[
					"errorMessage",
					"error_description",
					"error",
					"Message",
					"XErr",
					"path",
				]
				.iter()
				.find_map(|k| {
					v.get(*k).map(|e| {
						e.as_str()
							.map(str::to_owned)
							.unwrap_or_else(|| e.to_string())
					})
				})
			});
		let detail = detail.unwrap_or_else(|| self.body.chars().take(200).collect());
		format!("HTTP {}: {}", self.status, detail.trim())
	}
}

enum Body<'a> {
	Empty,
	Json(Value),
	Xbox(Value),
	Form(&'a [(&'a str, &'a str)]),
}

fn post(url: &str, body: Body<'_>, bearer: Option<&Secret>) -> Result<Reply> {
	let mut request = agent().post(url).header("Accept", "application/json");
	if let Some(token) = bearer {
		request = request.header("Authorization", format!("Bearer {}", token.expose()));
	}
	let response = match body {
		Body::Empty => request.send_empty(),
		Body::Json(value) => request
			.header("Content-Type", "application/json")
			.send(value.to_string()),
		Body::Xbox(value) => request
			.header("Content-Type", "application/json")
			.header("x-xbl-contract-version", "1")
			.send(value.to_string()),
		Body::Form(fields) => request.send_form(fields.iter().copied()),
	};
	reply(url, response)
}

fn get(url: &str, bearer: &Secret) -> Result<Reply> {
	let response = agent()
		.get(url)
		.header("Accept", "application/json")
		.header("Authorization", format!("Bearer {}", bearer.expose()))
		.call();
	reply(url, response)
}

fn reply(
	url: &str,
	response: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<Reply> {
	let mut response = response.map_err(|e| anyhow!("requesting {url}: {e}"))?;
	let status = response.status().as_u16();
	let body = response
		.body_mut()
		.read_to_string()
		.map_err(|e| anyhow!("reading {url}: {e}"))?;
	Ok(Reply { status, body })
}

fn text<'a>(value: &'a Value, path: &[&str]) -> Result<&'a str> {
	path.iter()
		.try_fold(value, |v, key| v.get(key))
		.and_then(Value::as_str)
		.ok_or_else(|| anyhow!("the response has no {}", path.join(".")))
}

fn base64(text: &str) -> Result<Vec<u8>> {
	let clean: String = text.chars().filter(|c| !c.is_whitespace()).collect();
	STANDARD
		.decode(&clean)
		.or_else(|_| STANDARD_NO_PAD.decode(clean.trim_end_matches('=')))
		.map_err(|e| anyhow!("bad base64: {e}"))
}

fn pem(text: &str) -> Result<Vec<u8>> {
	base64(
		&text
			.lines()
			.filter(|l| !l.starts_with("-----"))
			.collect::<String>(),
	)
}

pub enum Joined {
	Yes,
	InvalidToken,
}

pub fn join(
	endpoints: &Endpoints,
	token: &Secret,
	profile: u128,
	server_hash: &str,
) -> Result<Joined> {
	let body = json!({
		"accessToken": token.expose(),
		"selectedProfile": format!("{profile:032x}"),
		"serverId": server_hash,
	});
	let reply = post(
		&Endpoints::url(&endpoints.session, "/session/minecraft/join"),
		Body::Json(body),
		None,
	)?;
	if reply.ok() {
		return Ok(Joined::Yes);
	}
	let error = serde_json::from_str::<Value>(&reply.body).ok();
	let kind = error
		.as_ref()
		.and_then(|e| e.get("error"))
		.and_then(Value::as_str);
	if reply.status == 401 || kind == Some("ForbiddenOperationException") {
		return Ok(Joined::InvalidToken);
	}
	bail!("the session server refused the join, {}", reply.error())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameProfile {
	pub id: u128,
	pub name: String,
}

pub fn has_joined(
	endpoints: &Endpoints,
	username: &str,
	server_hash: &str,
) -> Result<Option<GameProfile>> {
	let url = Endpoints::url(&endpoints.session, "/session/minecraft/hasJoined");
	let response = agent()
		.get(&url)
		.header("Accept", "application/json")
		.query("username", username)
		.query("serverId", server_hash)
		.call();
	let reply = reply(&url, response)?;
	if !reply.ok() {
		bail!("the session server could not check the join, {}", reply.error());
	}
	if reply.status == 204 || reply.body.trim().is_empty() {
		return Ok(None);
	}
	let profile = reply.json()?;
	let id = parse_uuid(text(&profile, &["id"])?)
		.ok_or_else(|| anyhow!("the session server answered with a malformed profile id"))?;
	Ok(Some(GameProfile {
		id,
		name: text(&profile, &["name"])?.to_owned(),
	}))
}

pub struct PlayerKeys {
	pub private: RsaPrivateKey,
	pub public: Vec<u8>,
	pub signature: Vec<u8>,
	pub legacy_signature: Vec<u8>,
	pub expires_at: i64,
	pub refresh_at: i64,
}

impl fmt::Debug for PlayerKeys {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("PlayerKeys")
			.field("private", &"<redacted>")
			.field("expires_at", &self.expires_at)
			.field("refresh_at", &self.refresh_at)
			.finish_non_exhaustive()
	}
}

impl PlayerKeys {
	pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
		let digest = Sha256::digest(data);
		self.private
			.sign(Pkcs1v15Sign::new::<Sha256>(), &digest)
			.map_err(|e| anyhow!("signing with the chat key: {e}"))
	}

	pub fn parse(response: &Value) -> Result<Self> {
		let private = pem(text(response, &["keyPair", "privateKey"])?)?;
		let private = RsaPrivateKey::from_pkcs8_der(&private)
			.or_else(|_| RsaPrivateKey::from_pkcs1_der(&private))
			.map_err(|_| anyhow!("unreadable chat private key"))?;
		let public = pem(text(response, &["keyPair", "publicKey"])?)?;
		let public = match RsaPublicKey::from_public_key_der(&public) {
			Ok(_) => public,
			Err(_) => RsaPublicKey::from_pkcs1_der(&public)
				.map_err(|e| anyhow!("unreadable chat public key: {e}"))?
				.to_public_key_der()
				.map_err(|e| anyhow!("re-encoding the chat public key: {e}"))?
				.into_vec(),
		};
		let expires_at = parse_time(text(response, &["expiresAt"])?)
			.ok_or_else(|| anyhow!("unreadable expiresAt"))?;
		let refresh_at = text(response, &["refreshedAfter"])
			.ok()
			.and_then(parse_time)
			.unwrap_or(expires_at - 3_600_000);
		Ok(Self {
			private,
			public,
			signature: base64(text(response, &["publicKeySignatureV2"])?)?,
			legacy_signature: text(response, &["publicKeySignature"])
				.ok()
				.and_then(|s| base64(s).ok())
				.unwrap_or_default(),
			expires_at,
			refresh_at,
		})
	}
}

#[derive(Debug)]
pub struct Unauthorized;

impl fmt::Display for Unauthorized {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("Minecraft services rejected the access token")
	}
}

impl std::error::Error for Unauthorized {}

pub fn certificates(endpoints: &Endpoints, token: &Secret) -> Result<PlayerKeys> {
	let reply = post(
		&Endpoints::url(&endpoints.services, "/player/certificates"),
		Body::Empty,
		Some(token),
	)?;
	if reply.status == 401 {
		return Err(Unauthorized.into());
	}
	if !reply.ok() {
		bail!("fetching the chat signing key failed, {}", reply.error());
	}
	PlayerKeys::parse(&reply.json()?)
}

pub fn profile(endpoints: &Endpoints, token: &Secret) -> Result<(u128, String)> {
	let reply = get(
		&Endpoints::url(&endpoints.services, "/minecraft/profile"),
		token,
	)?;
	if !reply.ok() {
		bail!("fetching the Minecraft profile failed, {}", reply.error());
	}
	let profile = reply.json()?;
	let id =
		parse_uuid(text(&profile, &["id"])?).ok_or_else(|| anyhow!("the profile has a bad id"))?;
	Ok((id, text(&profile, &["name"])?.to_owned()))
}

pub struct Refreshed {
	pub access_token: Secret,
	pub expires_at: i64,
	pub refresh_token: Option<Secret>,
}

pub fn refresh(
	endpoints: &Endpoints,
	client_id: &str,
	refresh_token: &Secret,
) -> Result<Refreshed> {
	let form = [
		("client_id", client_id),
		("grant_type", "refresh_token"),
		("refresh_token", refresh_token.expose()),
		("scope", "XboxLive.SignIn XboxLive.offline_access"),
	];
	let reply = post(
		&Endpoints::url(&endpoints.microsoft, "/consumers/oauth2/v2.0/token"),
		Body::Form(&form),
		None,
	)?;
	if !reply.ok() {
		bail!("Microsoft refused the refresh token, {}", reply.error());
	}
	let msa = reply.json()?;
	let msa_token = Secret::new(text(&msa, &["access_token"])?);
	let next_refresh = msa
		.get("refresh_token")
		.and_then(Value::as_str)
		.map(Secret::new);
	let (user, _) = xbox(
		&Endpoints::url(&endpoints.xbox_user, "/user/authenticate"),
		json!({
			"Properties": {
				"AuthMethod": "RPS",
				"SiteName": "user.auth.xboxlive.com",
				"RpsTicket": format!("d={}", msa_token.expose()),
			},
			"RelyingParty": "http://auth.xboxlive.com",
			"TokenType": "JWT",
		}),
	)?;
	let (xsts, uhs) = xbox(
		&Endpoints::url(&endpoints.xbox_xsts, "/xsts/authorize"),
		json!({
			"Properties": { "SandboxId": "RETAIL", "UserTokens": [user.expose()] },
			"RelyingParty": "rp://api.minecraftservices.com/",
			"TokenType": "JWT",
		}),
	)?;
	let identity = format!("XBL3.0 x={uhs};{}", xsts.expose());
	let mut reply = post(
		&Endpoints::url(&endpoints.services, "/launcher/login"),
		Body::Json(json!({ "xtoken": identity, "platform": "PC_LAUNCHER" })),
		None,
	)?;
	if !reply.ok() {
		reply = post(
			&Endpoints::url(&endpoints.services, "/authentication/login_with_xbox"),
			Body::Json(json!({ "identityToken": identity })),
			None,
		)?;
	}
	if !reply.ok() {
		bail!("Minecraft refused the Xbox login, {}", reply.error());
	}
	let login = reply.json()?;
	let expires_in = login
		.get("expires_in")
		.and_then(Value::as_i64)
		.unwrap_or(86_400);
	Ok(Refreshed {
		access_token: Secret::new(text(&login, &["access_token"])?),
		expires_at: now_secs() + expires_in,
		refresh_token: next_refresh,
	})
}

fn xbox(url: &str, body: Value) -> Result<(Secret, String)> {
	let reply = post(url, Body::Xbox(body), None)?;
	if !reply.ok() {
		let hint = match serde_json::from_str::<Value>(&reply.body)
			.ok()
			.and_then(|v| v.get("XErr").and_then(Value::as_i64))
		{
			Some(2_148_916_233) => " (the account has no Xbox profile)",
			Some(2_148_916_235) => " (Xbox Live is unavailable in the account's country)",
			Some(2_148_916_238) => " (child account, add it to a family)",
			_ => "",
		};
		bail!("Xbox Live refused the login, {}{hint}", reply.error());
	}
	let value = reply.json()?;
	let token = Secret::new(text(&value, &["Token"])?);
	let uhs = value
		.pointer("/DisplayClaims/xui/0/uhs")
		.and_then(Value::as_str)
		.ok_or_else(|| anyhow!("the Xbox response has no user hash"))?
		.to_owned();
	Ok((token, uhs))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn server_hashes_match_the_known_vectors() {
		assert_eq!(
			server_hash("Notch", &[], &[]),
			"4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48"
		);
		assert_eq!(
			server_hash("jeb_", &[], &[]),
			"-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1"
		);
		assert_eq!(
			server_hash("simon", &[], &[]),
			"88e16a1019277b15d58faf0541e11910eb756f6"
		);
		assert_eq!(
			server_hash("", b"Not", b"ch"),
			"4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48"
		);
	}

	#[test]
	fn timestamps_parse_to_epoch_millis() {
		assert_eq!(parse_time("1970-01-01T00:00:00Z"), Some(0));
		assert_eq!(
			parse_time("2022-06-22T22:49:39.876498Z"),
			Some(1_655_938_179_876)
		);
		assert_eq!(
			parse_time("2022-06-22T22:49:39.8Z"),
			Some(1_655_938_179_800)
		);
		assert_eq!(
			parse_time("2022-06-23T00:49:39+02:00"),
			Some(1_655_938_179_000)
		);
		assert_eq!(parse_time("2000-02-29T12:00:00Z"), Some(951_825_600_000));
		assert_eq!(parse_time("yesterday"), None);
	}

	#[test]
	fn uuids_parse_with_and_without_dashes() {
		let id = 0x069a79f444e94726a5befca90e38aaf5u128;
		assert_eq!(parse_uuid("069a79f4-44e9-4726-a5be-fca90e38aaf5"), Some(id));
		assert_eq!(parse_uuid("069a79f444e94726a5befca90e38aaf5"), Some(id));
		assert_eq!(hyphenated(id), "069a79f4-44e9-4726-a5be-fca90e38aaf5");
		assert_eq!(parse_uuid("Notch"), None);
	}

	#[test]
	fn secrets_stay_out_of_debug_output() {
		let secret = Secret::new("eyJhbGciOiJIUzI1NiJ9.very-secret");
		assert_eq!(format!("{secret:?}"), "<redacted>");
		assert!(!format!("{:?}", Some(&secret)).contains("very-secret"));
	}
}
