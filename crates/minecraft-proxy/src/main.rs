use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io::{self, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use flate2::read::ZlibDecoder;
use minecraft_protocol::gen;
use minecraft_protocol::pds::{self, Input, Output};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum State {
	Handshaking,
	Status,
	Login,
	Configuration,
	Play,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Dir {
	C2s,
	S2c,
}
impl Dir {
	fn label(self) -> &'static str {
		match self {
			Dir::C2s => "c2s",
			Dir::S2c => "s2c",
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Outcome {
	Ok,
	Mismatch,
	Error,
	Panic,
}
impl Outcome {
	fn label(self) -> &'static str {
		match self {
			Outcome::Ok => "ok",
			Outcome::Mismatch => "MISMATCH",
			Outcome::Error => "ERROR",
			Outcome::Panic => "PANIC",
		}
	}
	fn index(self) -> usize {
		self as usize
	}
}

struct Report {
	name: String,
	outcome: Outcome,
	detail: String,
}

#[derive(Default)]
struct Effect {
	c2s: Option<State>,
	s2c: Option<State>,
	threshold: Option<i32>,
	opaque: bool,
}

struct Opts {
	listen: String,
	upstream: String,
	full: bool,
	failures_only: bool,
}

struct Conn {
	id: usize,
	c2s: Mutex<State>,
	s2c: Mutex<State>,
	threshold: AtomicI32,
	opaque: AtomicBool,
	stats: Mutex<BTreeMap<(Dir, State, String), [u64; 4]>>,
	print: Mutex<()>,
}

thread_local! {
	static QUIET: Cell<bool> = const { Cell::new(false) };
	static LAST_PANIC: RefCell<String> = const { RefCell::new(String::new()) };
}

fn guard<R>(f: impl FnOnce() -> R) -> Result<R, String> {
	QUIET.with(|q| q.set(true));
	let r = panic::catch_unwind(AssertUnwindSafe(f));
	QUIET.with(|q| q.set(false));
	r.map_err(|_| LAST_PANIC.with(|p| p.borrow().clone()))
}

fn install_panic_hook() {
	let default = panic::take_hook();
	panic::set_hook(Box::new(move |info| {
		if QUIET.with(|q| q.get()) {
			let msg = info
				.payload()
				.downcast_ref::<&str>()
				.map(|s| s.to_string())
				.or_else(|| info.payload().downcast_ref::<String>().cloned())
				.unwrap_or_else(|| "panic".to_owned());
			let at = info
				.location()
				.map(|l| format!(" at {}:{}", l.file(), l.line()))
				.unwrap_or_default();
			LAST_PANIC.with(|p| *p.borrow_mut() = format!("{msg}{at}"));
		} else {
			default(info);
		}
	}));
}

fn hex(b: &[u8]) -> String {
	b.iter()
		.map(|b| format!("{b:02x}"))
		.collect::<Vec<_>>()
		.join(" ")
}

fn diff(orig: &[u8], ours: &[u8]) -> String {
	let at = orig
		.iter()
		.zip(ours)
		.position(|(a, b)| a != b)
		.unwrap_or(orig.len().min(ours.len()));
	let window = |b: &[u8]| hex(&b[at.saturating_sub(4)..(at + 12).min(b.len())]);
	format!(
		"re-serialized {} bytes vs {} original, first difference at byte {at}: original [{}] ours [{}]",
		ours.len(),
		orig.len(),
		window(orig),
		window(ours)
	)
}

const MAX_FRAME: usize = (1 << 21) - 1;
const MAX_PACKET: usize = 1 << 23;

fn varint(buf: &[u8]) -> Option<(i32, usize)> {
	let mut v = 0u32;
	for (i, b) in buf.iter().take(5).enumerate() {
		v |= ((b & 0x7f) as u32) << (7 * i);
		if b & 0x80 == 0 {
			return Some((v as i32, i + 1));
		}
	}
	None
}

fn variant_name(debug: &str) -> String {
	debug
		.split(|c: char| c == '(' || c == '{' || c == ' ')
		.next()
		.unwrap_or("")
		.to_owned()
}

fn check<T: Debug>(
	data: &[u8],
	de: fn(&mut Input) -> pds::Result<T>,
	ser: fn(&T, &mut Output) -> pds::Result<()>,
	size: fn(&T) -> usize,
	opts: &Opts,
) -> (Option<T>, Report) {
	let id = varint(data).map(|(id, _)| id).unwrap_or(-1);
	let unknown = || format!("id {id:#04x}");
	let parsed = guard(|| {
		let mut input = Input::new(data);
		pds::stack::budget(512 << 10, || de(&mut input)).map(|v| (v, input.offset()))
	});
	let (value, consumed) = match parsed {
		Err(msg) => {
			return (
				None,
				Report {
					detail: format!("{}: deserialize panicked: {msg}", unknown()),
					name: unknown(),
					outcome: Outcome::Panic,
				},
			)
		}
		Ok(Err(e)) => {
			return (
				None,
				Report {
					detail: format!(
						"{}: deserialize failed: {e:?} ({} bytes: {})",
						unknown(),
						data.len(),
						hex(&data[..data.len().min(32)])
					),
					name: unknown(),
					outcome: Outcome::Error,
				},
			)
		}
		Ok(Ok(v)) => v,
	};
	let debug = format!("{value:?}");
	let name = variant_name(&debug);
	let mut problems = Vec::new();
	if consumed != data.len() {
		problems.push(format!(
			"consumed {consumed} of {} bytes, trailing [{}]",
			data.len(),
			hex(&data[consumed..data.len().min(consumed + 16)])
		));
	}
	match guard(|| {
		let mut out = Output::new();
		ser(&value, &mut out).map(|_| out.into_inner())
	}) {
		Err(msg) => problems.push(format!("serialize panicked: {msg}")),
		Ok(Err(e)) => problems.push(format!("serialize failed: {e:?}")),
		Ok(Ok(bytes)) if bytes != data => problems.push(diff(data, &bytes)),
		Ok(Ok(_)) => {}
	}
	match guard(|| size(&value)) {
		Err(msg) => problems.push(format!("size_of panicked: {msg}")),
		Ok(n) if n != data.len() => problems.push(format!("size_of {n} != {}", data.len())),
		Ok(_) => {}
	}
	let shown = if opts.full || debug.len() <= 300 {
		debug
	} else {
		let mut s: String = debug.chars().take(300).collect();
		s.push_str("...");
		s
	};
	let report = if problems.is_empty() {
		Report {
			name,
			outcome: Outcome::Ok,
			detail: shown,
		}
	} else {
		Report {
			detail: format!("{name}: {} | {shown}", problems.join("; ")),
			name,
			outcome: Outcome::Mismatch,
		}
	};
	(Some(value), report)
}

macro_rules! check {
	($m:ident, $data:expr, $opts:expr) => {
		check($data, $m::deserialize, $m::serialize, $m::size_of, $opts)
	};
}

fn inspect(dir: Dir, state: State, data: &[u8], opts: &Opts) -> (Report, Effect) {
	let mut fx = Effect::default();
	let report = match (dir, state) {
		(Dir::C2s, State::Handshaking) => {
			use gen::handshaking::to_server::packet as p;
			let (v, r) = check!(p, data, opts);
			if let Some(p::Ty::SetProtocol(h)) = v {
				let next = if h.next_state == 1 {
					State::Status
				} else {
					State::Login
				};
				fx.c2s = Some(next);
				fx.s2c = Some(next);
			}
			r
		}
		(Dir::S2c, State::Handshaking) => {
			use gen::handshaking::to_client::packet as p;
			check!(p, data, opts).1
		}
		(Dir::C2s, State::Status) => {
			use gen::status::to_server::packet as p;
			check!(p, data, opts).1
		}
		(Dir::S2c, State::Status) => {
			use gen::status::to_client::packet as p;
			check!(p, data, opts).1
		}
		(Dir::C2s, State::Login) => {
			use gen::login::to_server::packet as p;
			let (v, r) = check!(p, data, opts);
			if let Some(p::Ty::LoginAcknowledged(_)) = v {
				fx.c2s = Some(State::Configuration);
			}
			r
		}
		(Dir::S2c, State::Login) => {
			use gen::login::to_client::packet as p;
			let (v, r) = check!(p, data, opts);
			match v {
				Some(p::Ty::Success(_)) => fx.s2c = Some(State::Configuration),
				Some(p::Ty::Compress(t)) => fx.threshold = Some(t),
				Some(p::Ty::EncryptionBegin(_)) => fx.opaque = true,
				_ => {}
			}
			r
		}
		(Dir::C2s, State::Configuration) => {
			use gen::configuration::to_server::packet as p;
			let (v, r) = check!(p, data, opts);
			if let Some(p::Ty::FinishConfiguration(_)) = v {
				fx.c2s = Some(State::Play);
			}
			r
		}
		(Dir::S2c, State::Configuration) => {
			use gen::configuration::to_client::packet as p;
			let (v, r) = check!(p, data, opts);
			if let Some(p::Ty::FinishConfiguration(_)) = v {
				fx.s2c = Some(State::Play);
			}
			r
		}
		(Dir::C2s, State::Play) => {
			use gen::play::to_server::packet as p;
			let (v, r) = check!(p, data, opts);
			if let Some(p::Ty::ConfigurationAcknowledged(_)) = v {
				fx.c2s = Some(State::Configuration);
			}
			r
		}
		(Dir::S2c, State::Play) => {
			use gen::play::to_client::packet as p;
			let (v, r) = check!(p, data, opts);
			if let Some(p::Ty::StartConfiguration(_)) = v {
				fx.s2c = Some(State::Configuration);
			}
			r
		}
	};
	(report, fx)
}

fn read_frame(r: &mut impl Read) -> io::Result<Option<(Vec<u8>, usize)>> {
	let mut frame = Vec::new();
	let mut len = 0u32;
	for i in 0..5 {
		let mut b = [0u8];
		if r.read(&mut b)? == 0 {
			if i == 0 {
				return Ok(None);
			}
			return Err(io::ErrorKind::UnexpectedEof.into());
		}
		frame.push(b[0]);
		len |= ((b[0] & 0x7f) as u32) << (7 * i);
		if b[0] & 0x80 == 0 {
			if len as usize > MAX_FRAME {
				return Err(io::Error::new(
					io::ErrorKind::InvalidData,
					format!("frame of {len} bytes is too large"),
				));
			}
			let start = frame.len();
			frame.resize(start + len as usize, 0);
			r.read_exact(&mut frame[start..])?;
			return Ok(Some((frame, start)));
		}
	}
	Err(io::Error::new(
		io::ErrorKind::InvalidData,
		"frame length varint too long",
	))
}

fn packet_data(body: &[u8], threshold: i32) -> io::Result<Vec<u8>> {
	if threshold < 0 {
		return Ok(body.to_vec());
	}
	let (data_len, n) = varint(body)
		.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "bad data length"))?;
	let rest = &body[n..];
	if data_len == 0 {
		return Ok(rest.to_vec());
	}
	let invalid = |msg: String| io::Error::new(io::ErrorKind::InvalidData, msg);
	let data_len = usize::try_from(data_len)
		.ok()
		.filter(|n| *n <= MAX_PACKET)
		.ok_or_else(|| invalid(format!("data length {data_len} is out of range")))?;
	let mut out = vec![0; data_len];
	let mut z = ZlibDecoder::new(rest);
	z.read_exact(&mut out).map_err(|_| {
		invalid(format!(
			"inflates to less than the {data_len} bytes the header says"
		))
	})?;
	if z.read(&mut [0])? != 0 {
		return Err(invalid(format!(
			"inflates past the {data_len} bytes the header says"
		)));
	}
	Ok(out)
}

fn state_of(conn: &Conn, dir: Dir) -> State {
	*match dir {
		Dir::C2s => &conn.c2s,
		Dir::S2c => &conn.s2c,
	}
	.lock()
	.unwrap()
}

fn apply(conn: &Conn, fx: Effect) {
	if let Some(s) = fx.c2s {
		*conn.c2s.lock().unwrap() = s;
	}
	if let Some(s) = fx.s2c {
		*conn.s2c.lock().unwrap() = s;
	}
	if let Some(t) = fx.threshold {
		conn.threshold.store(t, Ordering::SeqCst);
	}
	if fx.opaque {
		conn.opaque.store(true, Ordering::SeqCst);
	}
}

fn log(conn: &Conn, line: String) {
	let _lock = conn.print.lock().unwrap();
	println!("#{} {line}", conn.id);
}

fn pump(conn: &Conn, dir: Dir, from: TcpStream, mut to: TcpStream, opts: &Opts) {
	let mut reader = BufReader::new(from);
	loop {
		let (frame, start) = match read_frame(&mut reader) {
			Ok(Some(f)) => f,
			Ok(None) => break,
			Err(e) => {
				log(conn, format!("{} read error: {e}", dir.label()));
				break;
			}
		};
		if !conn.opaque.load(Ordering::SeqCst) {
			let state = state_of(conn, dir);
			let threshold = conn.threshold.load(Ordering::SeqCst);
			match packet_data(&frame[start..], threshold) {
				Ok(data) => {
					let (report, fx) = inspect(dir, state, &data, opts);
					let encrypted = fx.opaque;
					apply(conn, fx);
					conn.stats
						.lock()
						.unwrap()
						.entry((dir, state, report.name.clone()))
						.or_default()[report.outcome.index()] += 1;
					if report.outcome != Outcome::Ok || !opts.failures_only {
						log(
							conn,
							format!(
								"{} {:<13} {:<8} {}",
								dir.label(),
								format!("{state:?}"),
								report.outcome.label(),
								report.detail
							),
						);
					}
					if encrypted {
						log(conn, "server requested encryption: online-mode traffic cannot be inspected, passing through".to_owned());
					}
				}
				Err(e) => log(conn, format!("{} frame decode error: {e}", dir.label())),
			}
		}
		if let Err(e) = to.write_all(&frame) {
			log(conn, format!("{} write error: {e}", dir.label()));
			break;
		}
	}
	let _ = to.shutdown(Shutdown::Both);
	let _ = reader.get_ref().shutdown(Shutdown::Both);
}

fn summary(conn: &Conn) {
	let stats = conn.stats.lock().unwrap();
	let _lock = conn.print.lock().unwrap();
	println!(
		"#{} summary: dir state packet ok/mismatch/error/panic",
		conn.id
	);
	for ((dir, state, name), [ok, mismatch, error, panic]) in stats.iter() {
		let flag = if mismatch + error + panic > 0 {
			"!"
		} else {
			" "
		};
		println!(
			"#{} {flag} {} {:<13} {:<45} {ok}/{mismatch}/{error}/{panic}",
			conn.id,
			dir.label(),
			format!("{state:?}"),
			name
		);
	}
}

fn handle(id: usize, client: TcpStream, opts: Arc<Opts>) -> io::Result<()> {
	let server = TcpStream::connect(&opts.upstream)?;
	client.set_nodelay(true)?;
	server.set_nodelay(true)?;
	let conn = Arc::new(Conn {
		id,
		c2s: Mutex::new(State::Handshaking),
		s2c: Mutex::new(State::Handshaking),
		threshold: AtomicI32::new(-1),
		opaque: AtomicBool::new(false),
		stats: Mutex::default(),
		print: Mutex::new(()),
	});
	log(
		&conn,
		format!("connected {} -> {}", client.peer_addr()?, opts.upstream),
	);
	let s2c = {
		let (conn, opts) = (conn.clone(), opts.clone());
		let (from, to) = (server.try_clone()?, client.try_clone()?);
		thread::spawn(move || pump(&conn, Dir::S2c, from, to, &opts))
	};
	pump(&conn, Dir::C2s, client, server, &opts);
	let _ = s2c.join();
	summary(&conn);
	Ok(())
}

fn parse_args() -> Result<Opts, String> {
	let mut positional = Vec::new();
	let mut full = false;
	let mut failures_only = false;
	for a in std::env::args().skip(1) {
		match a.as_str() {
			"--full" => full = true,
			"--failures" => failures_only = true,
			_ if a.starts_with("--") => return Err(format!("unknown flag {a}")),
			_ => positional.push(a),
		}
	}
	let [listen, upstream]: [String; 2] = positional
		.try_into()
		.map_err(|_| "expected <listen addr> <upstream addr>".to_owned())?;
	Ok(Opts {
		listen,
		upstream,
		full,
		failures_only,
	})
}

fn main() {
	let opts =
		match parse_args() {
			Ok(o) => Arc::new(o),
			Err(e) => {
				eprintln!("{e}\nusage: minecraft-proxy [--full] [--failures] <listen addr> <upstream addr>");
				std::process::exit(2);
			}
		};
	install_panic_hook();
	let listener = TcpListener::bind(&opts.listen).expect("bind listen address");
	println!(
		"listening on {}, forwarding to {}",
		opts.listen, opts.upstream
	);
	let next = AtomicUsize::new(1);
	for stream in listener.incoming() {
		let stream = match stream {
			Ok(s) => s,
			Err(e) => {
				eprintln!("accept: {e}");
				continue;
			}
		};
		let id = next.fetch_add(1, Ordering::SeqCst);
		let opts = opts.clone();
		thread::spawn(move || {
			if let Err(e) = handle(id, stream, opts) {
				eprintln!("#{id} {e}");
			}
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn opts() -> Opts {
		Opts {
			listen: String::new(),
			upstream: String::new(),
			full: false,
			failures_only: false,
		}
	}

	#[test]
	fn reports_panics_and_mismatches() {
		install_panic_hook();
		let (_, r) = check::<u8>(&[1], |_| todo!("nope"), |_, _| Ok(()), |_| 1, &opts());
		assert_eq!(r.outcome, Outcome::Panic);
		assert!(r.detail.contains("nope"), "{}", r.detail);

		let (_, r) = check::<u8>(
			&[1, 2],
			|i| i.read(),
			|v, o| {
				o.write(&[*v + 1]);
				Ok(())
			},
			|_| 1,
			&opts(),
		);
		assert_eq!(r.outcome, Outcome::Mismatch);
		assert!(r.detail.contains("consumed 1 of 2"), "{}", r.detail);
		assert!(
			r.detail.contains("first difference at byte 0"),
			"{}",
			r.detail
		);
		assert!(r.detail.contains("size_of 1 != 2"), "{}", r.detail);
	}
}
