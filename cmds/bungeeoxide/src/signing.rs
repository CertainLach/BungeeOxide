use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use bungeeoxide_converter::gen::convert::{packet_name, v26_1};
use bungeeoxide_converter::{convert, Root, Session};
use sha2::{Digest, Sha256};
use tracing::warn;

use crate::codec::{read_varint, write_varint};
use crate::decode;
use bungeeoxide_authlib::{now_millis, PlayerKeys};
use bungeeoxide_converter::Version;

const ACK_AFTER: i32 = 64;
const LAST_SEEN: usize = 20;
const LAST_SEEN_1_19_1: usize = 5;
const SEPARATOR: u8 = 70;

pub struct Chat {
	sv: Version,
	keys: Arc<PlayerKeys>,
	sender: u128,
	chain: Chain,
	split_commands: bool,
	checksums: bool,
	global_index: bool,
	last_time: i64,
	commands: Option<Tree>,
}

enum Chain {
	Plain,
	Linked {
		preceding: Option<Vec<u8>>,
		seen: Vec<(u128, Vec<u8>)>,
		unacknowledged: i32,
	},
	Session {
		id: u128,
		index: i32,
		seen: Box<LastSeen>,
		announced: bool,
		playing: bool,
	},
}

impl Chat {
	pub fn new(sv: Version, keys: Arc<PlayerKeys>, sender: u128) -> Option<Self> {
		let chain = if sv < Version::V1_19 {
			return None;
		} else if sv < Version::V1_19_2 {
			Chain::Plain
		} else if sv < Version::V1_19_3 {
			Chain::Linked {
				preceding: None,
				seen: Vec::new(),
				unacknowledged: 0,
			}
		} else {
			let id =
				(rand::random::<u128>() & !(0xf << 76) & !(0x3 << 62)) | (0x4 << 76) | (0x2 << 62);
			Chain::Session {
				id,
				index: 0,
				seen: Box::default(),
				announced: false,
				playing: false,
			}
		};
		Some(Self {
			sv,
			keys,
			sender,
			chain,
			split_commands: sv >= Version::V1_20_5,
			checksums: sv >= Version::V1_21_5,
			global_index: sv >= Version::V1_21_5,
			last_time: 0,
			commands: None,
		})
	}

	pub fn observe(&mut self, data: &[u8]) {
		if let Chain::Session { playing, .. } = &mut self.chain {
			*playing = true;
		}
		match packet_name(Root::PlayToClient, self.sv, data) {
			Some("player_chat") => {
				if self.received(data).is_none() {
					warn!("unreadable {} player_chat packet", self.sv);
				}
			}
			Some("declare_commands") => match Tree::from_packet(self.sv, data) {
				Ok(tree) => self.commands = Some(tree),
				Err(e) => warn!(
					"can't read the server's commands, command arguments stay unsigned: {e:#}"
				),
			},
			_ => {}
		}
	}

	fn received(&mut self, data: &[u8]) -> Option<()> {
		let mut r = Reader::packet(data)?;
		match &mut self.chain {
			Chain::Plain => {}
			Chain::Linked {
				seen,
				unacknowledged,
				..
			} => {
				if r.bool()? {
					r.byte_array()?;
				}
				let sender = r.uuid()?;
				let signature = r.byte_array()?;
				if sender != 0 && !signature.is_empty() {
					push_linked(seen, sender, signature);
					*unacknowledged += 1;
				}
			}
			Chain::Session { seen, .. } => {
				if self.global_index {
					r.varint()?;
				}
				r.uuid()?;
				r.varint()?;
				if r.bool()? {
					seen.add(r.take(256)?.to_vec());
				}
			}
		}
		Some(())
	}

	pub fn outgoing(&mut self, packets: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
		let mut out = Vec::new();
		if let Err(e) = self.pending(&mut out) {
			warn!("can't keep the chat session up: {e:#}");
		}
		for packet in packets {
			let rewritten = match packet_name(Root::PlayToServer, self.sv, &packet) {
				Some("chat_session_update" | "message_acknowledgement") => Ok(None),
				Some("chat_message") => first_string(&packet)
					.and_then(|m| self.message(&m))
					.map(Some),
				Some("chat_command" | "chat_command_signed") => first_string(&packet)
					.and_then(|c| self.command(&c))
					.map(Some),
				_ => Ok(Some(packet)),
			};
			match rewritten {
				Ok(Some(p)) => out.push(p),
				Ok(None) => {}
				Err(e) => warn!("dropped a chat packet that can't be signed: {e:#}"),
			}
		}
		out
	}

	fn pending(&mut self, out: &mut Vec<Vec<u8>>) -> Result<()> {
		let mut announce = None;
		let mut acknowledge = None;
		match &mut self.chain {
			Chain::Plain => {}
			Chain::Linked {
				seen,
				unacknowledged,
				..
			} => {
				if *unacknowledged > ACK_AFTER {
					*unacknowledged = 0;
					let mut ack = Vec::new();
					put_linked_seen(&mut ack, seen);
					ack.push(0);
					acknowledge = Some(ack);
				}
			}
			Chain::Session {
				id,
				seen,
				announced,
				playing,
				..
			} => {
				if *playing && !*announced {
					*announced = true;
					announce = Some(*id);
				}
				if seen.offset > ACK_AFTER {
					let mut ack = Vec::new();
					write_varint(&mut ack, seen.take_offset());
					acknowledge = Some(ack);
				}
			}
		}
		if let Some(id) = announce {
			let mut update = self.start("chat_session_update")?;
			update.extend_from_slice(&id.to_be_bytes());
			update.extend_from_slice(&self.keys.expires_at.to_be_bytes());
			put_bytes(&mut update, &self.keys.public);
			put_bytes(&mut update, &self.keys.signature);
			out.push(update);
		}
		if let Some(body) = acknowledge {
			let mut ack = self.start("message_acknowledgement")?;
			ack.extend_from_slice(&body);
			out.push(ack);
		}
		Ok(())
	}

	fn start(&self, name: &str) -> Result<Vec<u8>> {
		let id = packet_id(Root::PlayToServer, self.sv, name)
			.ok_or_else(|| anyhow!("{} has no {name} packet", self.sv))?;
		let mut out = Vec::new();
		write_varint(&mut out, id);
		Ok(out)
	}

	fn stamp(&mut self) -> (i64, i64) {
		let time = now_millis().max(self.last_time);
		self.last_time = time;
		let salt = loop {
			let salt: i64 = rand::random();
			if salt != 0 {
				break salt;
			}
		};
		(time, salt)
	}

	fn message(&mut self, text: &str) -> Result<Vec<u8>> {
		let mut out = self.start("chat_message")?;
		let (time, salt) = self.stamp();
		put_string(&mut out, text);
		out.extend_from_slice(&time.to_be_bytes());
		out.extend_from_slice(&salt.to_be_bytes());
		match &mut self.chain {
			Chain::Plain => {
				put_bytes(
					&mut out,
					&self
						.keys
						.sign(&plain_payload(self.sender, time, salt, text))?,
				);
				out.push(0);
			}
			Chain::Linked {
				preceding,
				seen,
				unacknowledged,
			} => {
				let signature = self.keys.sign(&linked_payload(
					self.sender,
					preceding.as_deref(),
					seen,
					time,
					salt,
					text,
				))?;
				put_bytes(&mut out, &signature);
				*preceding = Some(signature);
				out.push(0);
				put_linked_seen(&mut out, seen);
				out.push(0);
				*unacknowledged = 0;
			}
			Chain::Session {
				id, index, seen, ..
			} => {
				let update = seen.update();
				let signature = sized(self.keys.sign(&session_payload(
					self.sender,
					*id,
					*index,
					time,
					salt,
					text,
					&update.signatures,
				))?)?;
				*index += 1;
				out.push(1);
				out.extend_from_slice(&signature);
				write_varint(&mut out, update.offset);
				out.extend_from_slice(&update.acknowledged);
				if self.checksums {
					out.push(0);
				}
			}
		}
		Ok(out)
	}

	fn command(&mut self, command: &str) -> Result<Vec<u8>> {
		let arguments = self
			.commands
			.as_ref()
			.map(|t| t.signable(command))
			.unwrap_or_default();
		if self.split_commands && arguments.is_empty() {
			let mut out = self.start("chat_command")?;
			put_string(&mut out, command);
			return Ok(out);
		}
		let mut out = self.start(if self.split_commands {
			"chat_command_signed"
		} else {
			"chat_command"
		})?;
		let (time, salt) = self.stamp();
		put_string(&mut out, command);
		out.extend_from_slice(&time.to_be_bytes());
		out.extend_from_slice(&salt.to_be_bytes());
		write_varint(&mut out, arguments.len() as i32);
		match &mut self.chain {
			Chain::Plain => {
				for (name, value) in &arguments {
					put_string(&mut out, name);
					put_bytes(
						&mut out,
						&self
							.keys
							.sign(&plain_payload(self.sender, time, salt, value))?,
					);
				}
				out.push(0);
			}
			Chain::Linked {
				preceding,
				seen,
				unacknowledged,
			} => {
				for (name, value) in &arguments {
					let signature = self.keys.sign(&linked_payload(
						self.sender,
						preceding.as_deref(),
						seen,
						time,
						salt,
						value,
					))?;
					put_string(&mut out, name);
					put_bytes(&mut out, &signature);
					*preceding = Some(signature);
				}
				out.push(0);
				put_linked_seen(&mut out, seen);
				out.push(0);
				*unacknowledged = 0;
			}
			Chain::Session {
				id, index, seen, ..
			} => {
				let update = seen.update();
				for (name, value) in &arguments {
					let signature = sized(self.keys.sign(&session_payload(
						self.sender,
						*id,
						*index,
						time,
						salt,
						value,
						&update.signatures,
					))?)?;
					*index += 1;
					put_string(&mut out, name);
					out.extend_from_slice(&signature);
				}
				write_varint(&mut out, update.offset);
				out.extend_from_slice(&update.acknowledged);
				if self.split_commands && self.checksums {
					out.push(0);
				}
			}
		}
		Ok(out)
	}
}

fn sized(signature: Vec<u8>) -> Result<Vec<u8>> {
	if signature.len() != 256 {
		bail!(
			"the chat key makes {}-byte signatures, 1.19.3+ servers take 256",
			signature.len()
		);
	}
	Ok(signature)
}

fn packet_id(root: Root, version: Version, name: &str) -> Option<i32> {
	(0..256).find(|id| {
		let mut tag = Vec::new();
		write_varint(&mut tag, *id);
		packet_name(root, version, &tag) == Some(name)
	})
}

fn first_string(packet: &[u8]) -> Result<String> {
	Reader::packet(packet)
		.and_then(|mut r| r.string())
		.ok_or_else(|| anyhow!("unreadable chat packet"))
}

fn put_string(out: &mut Vec<u8>, text: &str) {
	put_bytes(out, text.as_bytes());
}

fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
	write_varint(out, bytes.len() as i32);
	out.extend_from_slice(bytes);
}

fn put_linked_seen(out: &mut Vec<u8>, seen: &[(u128, Vec<u8>)]) {
	write_varint(out, seen.len() as i32);
	for (sender, signature) in seen {
		out.extend_from_slice(&sender.to_be_bytes());
		put_bytes(out, signature);
	}
}

fn push_linked(seen: &mut Vec<(u128, Vec<u8>)>, sender: u128, signature: Vec<u8>) {
	if seen
		.first()
		.is_some_and(|(s, sig)| *s == sender && *sig == signature)
	{
		return;
	}
	if let Some(at) = seen.iter().position(|(s, _)| *s == sender) {
		seen.remove(at);
	}
	seen.insert(0, (sender, signature));
	seen.truncate(LAST_SEEN_1_19_1);
}

fn seconds(millis: i64) -> i64 {
	millis.div_euclid(1000)
}

pub fn plain_payload(sender: u128, time: i64, salt: i64, text: &str) -> Vec<u8> {
	let mut data = Vec::new();
	data.extend_from_slice(&salt.to_be_bytes());
	data.extend_from_slice(&sender.to_be_bytes());
	data.extend_from_slice(&seconds(time).to_be_bytes());
	let json = serde_json::json!({ "text": text })
		.to_string()
		.replace('\u{2028}', "\\u2028")
		.replace('\u{2029}', "\\u2029");
	data.extend_from_slice(json.as_bytes());
	data
}

pub fn linked_payload(
	sender: u128,
	preceding: Option<&[u8]>,
	seen: &[(u128, Vec<u8>)],
	time: i64,
	salt: i64,
	text: &str,
) -> Vec<u8> {
	let mut body = Vec::new();
	body.extend_from_slice(&salt.to_be_bytes());
	body.extend_from_slice(&seconds(time).to_be_bytes());
	body.extend_from_slice(text.as_bytes());
	body.push(SEPARATOR);
	for (who, signature) in seen {
		body.push(SEPARATOR);
		body.extend_from_slice(&who.to_be_bytes());
		body.extend_from_slice(signature);
	}
	let mut data = Vec::new();
	if let Some(preceding) = preceding {
		data.extend_from_slice(preceding);
	}
	data.extend_from_slice(&sender.to_be_bytes());
	data.extend_from_slice(&Sha256::digest(&body));
	data
}

pub fn session_payload(
	sender: u128,
	session: u128,
	index: i32,
	time: i64,
	salt: i64,
	text: &str,
	seen: &[Vec<u8>],
) -> Vec<u8> {
	let mut data = Vec::new();
	data.extend_from_slice(&1i32.to_be_bytes());
	data.extend_from_slice(&sender.to_be_bytes());
	data.extend_from_slice(&session.to_be_bytes());
	data.extend_from_slice(&index.to_be_bytes());
	data.extend_from_slice(&salt.to_be_bytes());
	data.extend_from_slice(&seconds(time).to_be_bytes());
	data.extend_from_slice(&(text.len() as i32).to_be_bytes());
	data.extend_from_slice(text.as_bytes());
	data.extend_from_slice(&(seen.len() as i32).to_be_bytes());
	for signature in seen {
		data.extend_from_slice(signature);
	}
	data
}

#[derive(Default)]
struct LastSeen {
	slots: [Option<Vec<u8>>; LAST_SEEN],
	tail: usize,
	offset: i32,
	last: Option<Vec<u8>>,
}

struct Update {
	offset: i32,
	acknowledged: [u8; 3],
	signatures: Vec<Vec<u8>>,
}

impl LastSeen {
	fn add(&mut self, signature: Vec<u8>) {
		if self.last.as_ref() == Some(&signature) {
			return;
		}
		self.last = Some(signature.clone());
		self.slots[self.tail] = Some(signature);
		self.tail = (self.tail + 1) % LAST_SEEN;
		self.offset += 1;
	}

	fn take_offset(&mut self) -> i32 {
		std::mem::take(&mut self.offset)
	}

	fn update(&mut self) -> Update {
		let offset = self.take_offset();
		let mut acknowledged = [0u8; 3];
		let mut signatures = Vec::new();
		for j in 0..LAST_SEEN {
			if let Some(signature) = &self.slots[(self.tail + j) % LAST_SEEN] {
				acknowledged[j / 8] |= 1 << (j % 8);
				signatures.push(signature.clone());
			}
		}
		Update {
			offset,
			acknowledged,
			signatures,
		}
	}
}

struct Reader<'a> {
	data: &'a [u8],
	at: usize,
}

impl<'a> Reader<'a> {
	fn packet(data: &'a [u8]) -> Option<Self> {
		let (_, at) = read_varint(data)?;
		Some(Self { data, at })
	}

	fn take(&mut self, n: usize) -> Option<&'a [u8]> {
		let out = self.data.get(self.at..self.at.checked_add(n)?)?;
		self.at += n;
		Some(out)
	}

	fn varint(&mut self) -> Option<i32> {
		let (value, n) = read_varint(self.data.get(self.at..)?)?;
		self.at += n;
		Some(value)
	}

	fn bool(&mut self) -> Option<bool> {
		Some(self.take(1)?[0] != 0)
	}

	fn uuid(&mut self) -> Option<u128> {
		Some(u128::from_be_bytes(self.take(16)?.try_into().ok()?))
	}

	fn byte_array(&mut self) -> Option<Vec<u8>> {
		let n = usize::try_from(self.varint()?).ok()?;
		Some(self.take(n)?.to_vec())
	}

	fn string(&mut self) -> Option<String> {
		String::from_utf8(self.byte_array()?).ok()
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Parser {
	Message,
	Greedy,
	Quotable,
	Words(usize),
	Word,
}

#[derive(Debug)]
enum Kind {
	Root,
	Literal(String),
	Argument(String, Parser),
}

#[derive(Debug)]
struct Node {
	kind: Kind,
	children: Vec<usize>,
	redirect: Option<usize>,
}

#[derive(Debug)]
pub struct Tree {
	nodes: Vec<Node>,
	root: usize,
}

impl Tree {
	fn from_packet(sv: Version, data: &[u8]) -> Result<Self> {
		use v26_1::play_to_client_packet::{deserialize, Ty};
		let anchor = Version::V26_1;
		let packets = if sv == anchor {
			vec![data.to_vec()]
		} else {
			convert(Root::PlayToClient, sv, anchor, data, &mut Session::new())
				.map_err(|e| anyhow!("{e}"))?
		};
		for packet in packets {
			if let Ty::DeclareCommands(commands) = decode(&packet, deserialize)? {
				return Self::new(commands);
			}
		}
		bail!("the command tree did not survive conversion to 26.1")
	}

	fn new(commands: v26_1::play_to_client_packet_declare_commands::Ty) -> Result<Self> {
		use v26_1::command_node_extra_node_data::Ty as Data;
		use v26_1::command_node_extra_node_data_v2_properties::Ty as P;
		use v26_1::command_node_extra_node_data_v2_properties_brigadier_string::Ty as Mode;
		use v26_1::command_node_redirect_node::Ty as Redirect;
		let count = commands.nodes.len();
		let index = |i: i32| usize::try_from(i).ok().filter(|i| *i < count);
		let mut nodes = Vec::with_capacity(count);
		for node in commands.nodes {
			let kind = match node.extra_node_data {
				Data::V1(name) => Kind::Literal(name),
				Data::V2(argument) => {
					let parser = match argument.properties {
						P::MinecraftMessage => Parser::Message,
						P::BrigadierString(Mode::GreedyPhrase) => Parser::Greedy,
						P::BrigadierString(Mode::QuotablePhrase) => Parser::Quotable,
						P::MinecraftVec3 | P::MinecraftBlockPos => Parser::Words(3),
						P::MinecraftVec2 | P::MinecraftColumnPos | P::MinecraftRotation => {
							Parser::Words(2)
						}
						_ => Parser::Word,
					};
					Kind::Argument(argument.name, parser)
				}
				_ => Kind::Root,
			};
			nodes.push(Node {
				kind,
				children: node.children.into_iter().filter_map(index).collect(),
				redirect: match node.redirect_node {
					Redirect::V1(target) => index(target),
					_ => None,
				},
			});
		}
		let root =
			index(commands.root_index).ok_or_else(|| anyhow!("the command tree has no root"))?;
		Ok(Self { nodes, root })
	}

	pub fn signable(&self, command: &str) -> Vec<(String, String)> {
		let mut found = Vec::new();
		if self.walk(self.root, command, 0, true, &mut found, 0) {
			found
		} else {
			Vec::new()
		}
	}

	fn walk(
		&self,
		node: usize,
		input: &str,
		at: usize,
		collect: bool,
		found: &mut Vec<(String, String)>,
		depth: usize,
	) -> bool {
		if at >= input.len() {
			return true;
		}
		if depth > 128 {
			return false;
		}
		let start = if node == self.root {
			at
		} else if input[at..].starts_with(' ') {
			at + 1
		} else {
			return false;
		};
		let (parent, collect) = match self.nodes[node].redirect {
			Some(target) => (target, collect && target != self.root),
			None => (node, collect),
		};
		let rest = &input[start..];
		let word = &rest[..rest.find(' ').unwrap_or(rest.len())];
		let children = &self.nodes[parent].children;
		if let Some(literal) = children
			.iter()
			.copied()
			.find(|c| matches!(&self.nodes[*c].kind, Kind::Literal(name) if name == word))
		{
			return self.walk(
				literal,
				input,
				start + word.len(),
				collect,
				found,
				depth + 1,
			);
		}
		for &child in children {
			let Kind::Argument(name, parser) = &self.nodes[child].kind else {
				continue;
			};
			let Some(end) = consume(*parser, input, start) else {
				continue;
			};
			let mark = found.len();
			if collect && *parser == Parser::Message {
				found.push((name.clone(), input[start..end].to_owned()));
			}
			if self.walk(child, input, end, collect, found, depth + 1) {
				return true;
			}
			found.truncate(mark);
		}
		false
	}
}

fn consume(parser: Parser, input: &str, start: usize) -> Option<usize> {
	let rest = &input[start..];
	if rest.is_empty() {
		return None;
	}
	match parser {
		Parser::Message | Parser::Greedy => Some(input.len()),
		Parser::Quotable if rest.starts_with(['"', '\'']) => quoted(rest).map(|n| start + n),
		Parser::Quotable | Parser::Word => Some(start + token(rest)),
		Parser::Words(n) => {
			let mut at = start;
			for i in 0..n {
				if i > 0 {
					at += 1;
					if !input[at - 1..].starts_with(' ') || at >= input.len() {
						return None;
					}
				}
				let len = token(&input[at..]);
				if len == 0 {
					return None;
				}
				at += len;
			}
			Some(at)
		}
	}
}

fn quoted(text: &str) -> Option<usize> {
	let quote = text.chars().next()?;
	let mut escaped = false;
	for (i, c) in text.char_indices().skip(1) {
		if escaped {
			escaped = false;
		} else if c == '\\' {
			escaped = true;
		} else if c == quote {
			return Some(i + 1);
		}
	}
	None
}

fn token(text: &str) -> usize {
	let mut depth = 0i32;
	let mut quote = None;
	let mut escaped = false;
	for (i, c) in text.char_indices() {
		if let Some(q) = quote {
			if escaped {
				escaped = false;
			} else if c == '\\' {
				escaped = true;
			} else if c == q {
				quote = None;
			}
			continue;
		}
		match c {
			'"' | '\'' => quote = Some(c),
			'[' | '{' | '(' => depth += 1,
			']' | '}' | ')' => depth -= 1,
			' ' if depth <= 0 => return i,
			_ => {}
		}
	}
	text.len()
}

#[cfg(test)]
mod tests {
	use super::*;
	use rsa::pkcs8::DecodePrivateKey;
	use rsa::RsaPrivateKey;

	const SENDER: u128 = 0x069a79f444e94726a5befca90e38aaf5;
	const SESSION: u128 = 0x853c80ef3c3749fdaa49938b674adae6;

	const TEST_KEY: &[&str] = &[
		"MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQDAGFXiOsXcAXds6IJ+xbKpWwiZIBlxb0ej7FerDFgF7W8r1NtW",
		"cDDDYhj2JDkIIagUdYi6kYXTj5bEARNiT9yGS7y7r0nOaz05EHsbCilHBE1tPyJd00g2Hg5oNHwEsnoq2SmAEiTN7ZWbUX7Ogk7O",
		"VgWpemvJCsgvpke0uczMV5P9FmDYjBroEW3i0in69UR+OGcpcTaNHmDayI2SsEFRK5N2X5RmXAj/TLdxJkKodecGVeI/5MPNbjj8",
		"7We+3J9AGMreltm565h+eS1PZeRsyHeOHe90VTur+wH8X1gwjPilCsPhqtlGBNstMER6n8aiQ1rgg4Cbo45pqLE43sfTAgMBAAEC",
		"ggEAJKK/hnT1FV+GOlQhxzHHQDtnaWTQ1WyGukGx+qJRC1hcMnzXFCYzdp/36MHBysY4eyZrXEGyx0WEWH9oBgD7KvW42ga1SdP0",
		"CjUVdXO7qZqtdZ9r238XL5MN5lvUJ0UuvYtLOey77jZYqRWi3JnHk9yeGdOKSP5wbX4GAPbKk1Hv5GJKGlMgZZ/GHEms4TXuVq6P",
		"bKJcc1RMwRe15xelmcNR0awSwqZKlT7XJt149fdMeUWsq/BTWVFTf68MASNkE+yotcwAkl5gE8fyVL8/i9s9qR3tZL2oTL7bjI+p",
		"qYjNK5nRvu0UXlCaByw0wu7Gm6EnJyeLNI3ibaT70eMOeQKBgQDRs0/tQKu5tNi4GIcO3nPw95CGqeVZW3EVLfjZOp/RRPWsAyqp",
		"oc/EQ1bR0yS/XyletfU1B3ZQvD3q2w2AXN83sl2tzWZ0OgXPVcpj20GAoylBWgEhdg5TctlTKWGyZ1jZwTrKvLXXvamkCrdX4b0R",
		"ZfIr2wY0VopNtAJQEgWKfwKBgQDqge6gZP7SM8/eu1NghqUYsK+oZSEh6pHedXsdiq7+CdtXxYEPFl76lGbdku7bjZ7RlejmsyhQ",
		"gF6jcCotOKuYrsCxKRjVxbh4pHPrn5JkAr/jzHFiJM1JTEIZbCsseBt3gXW0GyEjTH01NskMuuUC5MiTWs06+z1xtheSbufQrQKB",
		"gGmslQCYcHF7IyNSqOzjSUYLW7eg8kTfgdMMgbTmiBcrSypNxjgJpOvgi2Xdo00VJYHm7BS3F/aDhihODzW6l3o31mF5esHjY34i",
		"W4WKvi2XC3k1ZBB6r6l53O69kdAXqHzDlV8JThHUMCLMi6wrw916pFlZXH0Ph061Ad1b15L9AoGAbu7MDT/5c0JjQP/oby8Chm1+",
		"ivBb/glSu6NjfBQQt8e95QVfCd4RTARLrOFG8MFLseVyaT+zAHHZUwNKUF4QfoecS7IfN3Dy1M8mrv2raHgMghxCPqUeH2BdnF0Z",
		"B05B4ARzw6FZybLzP+sMrGL01quXXm2DzDTxWgtDl1ezfXkCgYAtmrS4gJi9mLUknIJ29tB29yBelK3uy9VuzmCUe2+L40I1D55/",
		"vF5ysKG/gqqw6gFiucPodp8Fb+z9GE4qjOsmMIG3f0g6iovKEmSdZAvDtCsbsDrSsr2f3mPiHZh4RKOY8iTmVDGZ8JK/Je7ZrDkt",
		"2i+8ArZ5hf2UdeosG4Z0hQ==",
	];

	fn test_keys() -> Arc<PlayerKeys> {
		let der = base64::Engine::decode(
			&base64::engine::general_purpose::STANDARD,
			TEST_KEY.concat(),
		)
		.unwrap();
		let private = RsaPrivateKey::from_pkcs8_der(&der).unwrap();
		Arc::new(PlayerKeys {
			public: rsa::pkcs8::EncodePublicKey::to_public_key_der(&private.to_public_key())
				.unwrap()
				.into_vec(),
			private,
			signature: vec![7; 512],
			legacy_signature: vec![8; 512],
			expires_at: 1_900_000_000_000,
			refresh_at: 1_900_000_000_000,
		})
	}

	fn hex(text: &str) -> Vec<u8> {
		let text: String = text.split_whitespace().collect();
		(0..text.len())
			.step_by(2)
			.map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
			.collect()
	}

	#[test]
	fn plain_payloads_follow_chat_session_1_19_0() {
		let payload = plain_payload(SENDER, 1_655_938_179_876, 0x0102030405060708, "a<b\"c");
		let mut expected =
			hex("0102030405060708 069a79f444e94726a5befca90e38aaf5 0000000062b39c83");
		expected.extend_from_slice(br#"{"text":"a<b\"c"}"#);
		assert_eq!(payload, expected);
	}

	#[test]
	fn linked_payloads_follow_chat_session_1_19_1() {
		let seen = vec![(SESSION, vec![0xaa, 0xbb])];
		let payload = linked_payload(
			SENDER,
			Some(&[0xee, 0xff]),
			&seen,
			1_655_938_179_876,
			5,
			"hi",
		);
		let mut body = hex("0000000000000005 0000000062b39c83");
		body.extend_from_slice(b"hi");
		body.push(70);
		body.push(70);
		body.extend_from_slice(&hex("853c80ef3c3749fdaa49938b674adae6 aabb"));
		let mut expected = hex("eeff 069a79f444e94726a5befca90e38aaf5");
		expected.extend_from_slice(&Sha256::digest(&body));
		assert_eq!(payload, expected);
		let first = linked_payload(SENDER, None, &[], 0, 0, "");
		assert_eq!(&first[..16], &SENDER.to_be_bytes());
		assert_eq!(first.len(), 48);
	}

	#[test]
	fn session_payloads_follow_chat_session_1_19_3() {
		let payload = session_payload(
			SENDER,
			SESSION,
			3,
			1_655_938_179_876,
			-1,
			"h\u{e9}",
			&[vec![1, 2], vec![3]],
		);
		let expected = hex(
			"00000001 069a79f444e94726a5befca90e38aaf5 853c80ef3c3749fdaa49938b674adae6 00000003
			ffffffffffffffff 0000000062b39c83 00000003 68c3a9 00000002 0102 03",
		);
		assert_eq!(payload, expected);
	}

	#[test]
	fn signatures_verify_with_the_public_key() {
		use rsa::pkcs8::DecodePublicKey;
		let keys = test_keys();
		let payload = session_payload(SENDER, SESSION, 0, 1, 2, "hello", &[]);
		let signature = keys.sign(&payload).unwrap();
		assert_eq!(signature.len(), 256);
		let public = rsa::RsaPublicKey::from_public_key_der(&keys.public).unwrap();
		public
			.verify(
				rsa::Pkcs1v15Sign::new::<Sha256>(),
				&Sha256::digest(&payload),
				&signature,
			)
			.unwrap();
	}

	#[derive(Default)]
	struct Validator {
		tracked: Vec<Option<(Vec<u8>, bool)>>,
		last: Option<Vec<u8>>,
	}

	impl Validator {
		fn new() -> Self {
			Self {
				tracked: vec![None; LAST_SEEN],
				last: None,
			}
		}

		fn add_pending(&mut self, signature: Vec<u8>) {
			if self.last.as_ref() != Some(&signature) {
				self.last = Some(signature.clone());
				self.tracked.push(Some((signature, true)));
			}
		}

		fn apply_offset(&mut self, offset: i32) -> bool {
			let room = self.tracked.len() - LAST_SEEN;
			if offset < 0 || offset as usize > room {
				return false;
			}
			self.tracked.drain(..offset as usize);
			true
		}

		fn apply(&mut self, update: &Update) -> Option<Vec<Vec<u8>>> {
			if !self.apply_offset(update.offset) {
				return None;
			}
			let mut out = Vec::new();
			for i in 0..LAST_SEEN {
				let set = update.acknowledged[i / 8] & (1 << (i % 8)) != 0;
				match (&mut self.tracked[i], set) {
					(Some((signature, pending)), true) => {
						*pending = false;
						out.push(signature.clone());
					}
					(None, true) => return None,
					(Some((_, false)), false) => return None,
					(slot, false) => *slot = None,
				}
			}
			Some(out)
		}
	}

	#[test]
	fn last_seen_updates_satisfy_the_server_validator() {
		let mut seen = LastSeen::default();
		let mut server = Validator::new();
		let mut state = 0x2545f4914f6cdd1du64;
		let mut next = || {
			state ^= state << 13;
			state ^= state >> 7;
			state ^= state << 17;
			state
		};
		for round in 0..400 {
			let incoming = next() % 7;
			for _ in 0..incoming {
				let signature = (next() % 50).to_be_bytes().to_vec();
				server.add_pending(signature.clone());
				seen.add(signature);
			}
			if round % 5 == 4 {
				let offset = seen.take_offset();
				assert!(server.apply_offset(offset), "round {round}");
			} else {
				let update = seen.update();
				assert_eq!(
					server.apply(&update).as_ref(),
					Some(&update.signatures),
					"round {round}"
				);
			}
		}
	}

	#[test]
	fn linked_last_seen_keeps_one_entry_per_sender() {
		let mut seen = Vec::new();
		for (sender, signature) in [(1, 1), (2, 2), (3, 3), (2, 4), (4, 5), (5, 6), (6, 7)] {
			push_linked(&mut seen, sender, vec![signature]);
		}
		let senders: Vec<u128> = seen.iter().map(|(s, _)| *s).collect();
		assert_eq!(senders, vec![6, 5, 4, 2, 3]);
		assert_eq!(seen[3].1, vec![4]);
	}

	fn tree() -> Tree {
		let literal = |name: &str, children: Vec<usize>, redirect: Option<usize>| Node {
			kind: Kind::Literal(name.to_owned()),
			children,
			redirect,
		};
		let argument = |name: &str, parser: Parser, children: Vec<usize>| Node {
			kind: Kind::Argument(name.to_owned(), parser),
			children,
			redirect: None,
		};
		Tree {
			root: 0,
			nodes: vec![
				Node {
					kind: Kind::Root,
					children: vec![1, 4, 5, 7, 9, 12],
					redirect: None,
				},
				literal("msg", vec![2], None),
				argument("targets", Parser::Word, vec![3]),
				argument("message", Parser::Message, vec![]),
				literal("tell", vec![], Some(1)),
				literal("say", vec![6], None),
				argument("message", Parser::Message, vec![]),
				literal("me", vec![8], None),
				argument("action", Parser::Message, vec![]),
				literal("execute", vec![10, 11], None),
				literal("run", vec![], Some(0)),
				literal("positioned", vec![13], None),
				literal("tp", vec![14], None),
				argument("pos", Parser::Words(3), vec![10, 11]),
				argument("destination", Parser::Words(3), vec![]),
			],
		}
	}

	#[test]
	fn signable_arguments_come_from_the_command_tree() {
		let tree = tree();
		let pairs = |v: &[(&str, &str)]| {
			v.iter()
				.map(|(a, b)| (a.to_string(), b.to_string()))
				.collect::<Vec<_>>()
		};
		assert_eq!(
			tree.signable("say hello world"),
			pairs(&[("message", "hello world")])
		);
		assert_eq!(
			tree.signable("msg Steve hi there"),
			pairs(&[("message", "hi there")])
		);
		assert_eq!(
			tree.signable("msg @a[name=\"a b\"] hi"),
			pairs(&[("message", "hi")])
		);
		assert_eq!(
			tree.signable("tell Steve psst"),
			pairs(&[("message", "psst")])
		);
		assert_eq!(tree.signable("me waves"), pairs(&[("action", "waves")]));
		assert_eq!(tree.signable("tp ~ ~1 ~"), pairs(&[]));
		assert_eq!(tree.signable("execute run say hi"), pairs(&[]));
		assert_eq!(
			tree.signable("execute positioned 1 2 3 run say hi"),
			pairs(&[])
		);
		assert_eq!(tree.signable("unknown thing"), pairs(&[]));
	}

	fn inspect(sv: Version, packet: &[u8]) -> String {
		bungeeoxide_converter::inspect(Root::PlayToServer, sv, packet)
			.unwrap_or_else(|e| panic!("{}: {e}", sv))
	}

	#[test]
	fn rewritten_packets_parse_in_every_signing_version() {
		let keys = test_keys();
		let say = "say hi";
		for &sv in Version::span(Version::V1_19..) {
			let mut chat = Chat::new(sv, keys.clone(), SENDER).unwrap();
			chat.commands = Some(tree());
			if let Chain::Session { seen, playing, .. } = &mut chat.chain {
				*playing = true;
				for i in 0..70u8 {
					seen.add(vec![i; 256]);
				}
			}
			if let Chain::Linked {
				seen,
				unacknowledged,
				..
			} = &mut chat.chain
			{
				push_linked(seen, 9, vec![1; 256]);
				*unacknowledged = 65;
			}
			let mut plain = chat.start("chat_message").unwrap();
			put_string(&mut plain, "hello");
			let out = chat.outgoing(vec![plain]);
			let names: Vec<String> = out.iter().map(|p| inspect(sv, p)).collect();
			let expected: &[&str] = match &chat.chain {
				Chain::Plain => &["chat_message"],
				Chain::Linked { .. } => &["message_acknowledgement", "chat_message"],
				Chain::Session { .. } => &[
					"chat_session_update",
					"message_acknowledgement",
					"chat_message",
				],
			};
			assert_eq!(names, expected, "{sv}");
			let signed = chat.command(say).unwrap();
			let name = if sv >= Version::V1_20_5 {
				"chat_command_signed"
			} else {
				"chat_command"
			};
			assert_eq!(inspect(sv, &signed), name, "{sv}");
			let unsigned = chat.command("tp ~ ~ ~").unwrap();
			assert_eq!(inspect(sv, &unsigned), "chat_command", "{sv}");
			assert_eq!(first_string(&unsigned).unwrap(), "tp ~ ~ ~");
		}
	}

	#[test]
	fn player_chat_signatures_are_tracked_in_every_session_version() {
		let keys = test_keys();
		for &sv in Version::span(Version::V1_19_3..) {
			let mut chat = Chat::new(sv, keys.clone(), SENDER).unwrap();
			let id = packet_id(Root::PlayToClient, sv, "player_chat").unwrap();
			let mut packet = Vec::new();
			write_varint(&mut packet, id);
			if sv >= Version::V1_21_5 {
				write_varint(&mut packet, 0);
			}
			packet.extend_from_slice(&SESSION.to_be_bytes());
			write_varint(&mut packet, 0);
			packet.push(1);
			packet.extend_from_slice(&[9; 256]);
			chat.observe(&packet);
			let Chain::Session { seen, .. } = &mut chat.chain else {
				panic!()
			};
			assert_eq!(seen.update().signatures, vec![vec![9; 256]], "{sv}");
		}
	}
}
