import { inflateSync } from "node:zlib";
import { Input, Output, PdsError } from "../minecraft-protocol/pds.ts";
import * as gen from "../minecraft-protocol/gen/mod.ts";

export type State = "Handshaking" | "Status" | "Login" | "Configuration" | "Play";
export type Dir = "c2s" | "s2c";
export type Outcome = "ok" | "MISMATCH" | "ERROR" | "PANIC";

export interface Report {
	name: string;
	outcome: Outcome;
	detail: string;
}

export interface Opts {
	listen: Deno.ListenOptions;
	upstream: Deno.ConnectOptions;
	full: boolean;
	failuresOnly: boolean;
	log: (line: string) => void;
}

interface Codec<T> {
	deserialize(input: Input): T;
	serialize(v: T, output: Output): void;
	size_of(v: T): number;
}

interface Effect {
	c2s?: State;
	s2c?: State;
	threshold?: number;
	opaque?: boolean;
}

interface Conn {
	id: number;
	c2s: State;
	s2c: State;
	threshold: number;
	opaque: boolean;
	closing: boolean;
	stats: Map<string, { dir: Dir; state: State; name: string; counts: number[] }>;
}

const OUTCOMES: Outcome[] = ["ok", "MISMATCH", "ERROR", "PANIC"];

function hex(b: Uint8Array): string {
	return [...b].map((x) => x.toString(16).padStart(2, "0")).join(" ");
}

function diff(orig: Uint8Array, ours: Uint8Array): string {
	let at = 0;
	const n = Math.min(orig.length, ours.length);
	while (at < n && orig[at] === ours[at]) {
		at++;
	}
	const window = (b: Uint8Array) =>
		hex(b.subarray(Math.max(0, at - 4), Math.min(b.length, at + 12)));
	return `re-serialized ${ours.length} bytes vs ${orig.length} original, first difference at byte ${at}: original [${
		window(orig)
	}] ours [${window(ours)}]`;
}

function varint(buf: Uint8Array): [number, number] | null {
	let v = 0;
	for (let i = 0; i < Math.min(5, buf.length); i++) {
		v |= (buf[i] & 0x7f) << (7 * i);
		if ((buf[i] & 0x80) === 0) {
			return [v | 0, i + 1];
		}
	}
	return null;
}

function describe(e: unknown): [Outcome, string] {
	if (e instanceof PdsError) {
		return ["ERROR", e.message];
	}
	if (e instanceof Error) {
		const at = e.stack?.split("\n").find((l) => l.trim().startsWith("at "))?.trim() ?? "";
		return ["PANIC", `${e.name}: ${e.message} ${at}`];
	}
	return ["PANIC", String(e)];
}

function show(v: unknown, full: boolean): string {
	const s = Deno.inspect(v, {
		depth: Infinity,
		compact: true,
		breakLength: Infinity,
		iterableLimit: full ? Infinity : 64,
		strAbbreviateSize: full ? Infinity : 200,
	});
	return full || s.length <= 300 ? s : `${s.slice(0, 300)}...`;
}

export function check<T>(
	data: Uint8Array,
	codec: Codec<T>,
	full: boolean,
): [T | undefined, Report] {
	const id = varint(data)?.[0] ?? -1;
	const unknown = `id 0x${id.toString(16).padStart(2, "0")}`;
	let value: T;
	let consumed: number;
	try {
		const input = new Input(data);
		value = codec.deserialize(input);
		consumed = input.offset;
	} catch (e) {
		const [outcome, msg] = describe(e);
		const extra = outcome === "ERROR"
			? ` (${data.length} bytes: ${hex(data.subarray(0, 32))})`
			: "";
		return [undefined, {
			name: unknown,
			outcome,
			detail: `${unknown}: deserialize failed: ${msg}${extra}`,
		}];
	}
	const name = (value as { kind?: string }).kind ?? unknown;
	const problems: string[] = [];
	if (consumed !== data.length) {
		problems.push(
			`consumed ${consumed} of ${data.length} bytes, trailing [${
				hex(data.subarray(consumed, consumed + 16))
			}]`,
		);
	}
	try {
		const out = new Output();
		codec.serialize(value, out);
		const bytes = out.into_inner();
		if (bytes.length !== data.length || bytes.some((b, i) => b !== data[i])) {
			problems.push(diff(data, bytes));
		}
	} catch (e) {
		problems.push(`serialize failed: ${describe(e)[1]}`);
	}
	try {
		const n = codec.size_of(value);
		if (n !== data.length) {
			problems.push(`size_of ${n} != ${data.length}`);
		}
	} catch (e) {
		problems.push(`size_of failed: ${describe(e)[1]}`);
	}
	const shown = show(value, full);
	if (problems.length === 0) {
		return [value, { name, outcome: "ok", detail: shown }];
	}
	return [value, {
		name,
		outcome: "MISMATCH",
		detail: `${name}: ${problems.join("; ")} | ${shown}`,
	}];
}

export function inspect(dir: Dir, state: State, data: Uint8Array, full: boolean): [Report, Effect] {
	const fx: Effect = {};
	switch (`${dir} ${state}`) {
		case "c2s Handshaking": {
			const [v, r] = check(data, gen.handshaking.toServer.packet, full);
			if (v?.kind === "set_protocol") {
				const next: State = v.value.nextState === 1 ? "Status" : "Login";
				fx.c2s = next;
				fx.s2c = next;
			}
			return [r, fx];
		}
		case "s2c Handshaking":
			return [check(data, gen.handshaking.toClient.packet, full)[1], fx];
		case "c2s Status":
			return [check(data, gen.status.toServer.packet, full)[1], fx];
		case "s2c Status":
			return [check(data, gen.status.toClient.packet, full)[1], fx];
		case "c2s Login": {
			const [v, r] = check(data, gen.login.toServer.packet, full);
			if (v?.kind === "login_acknowledged") {
				fx.c2s = "Configuration";
			}
			return [r, fx];
		}
		case "s2c Login": {
			const [v, r] = check(data, gen.login.toClient.packet, full);
			if (v?.kind === "success") {
				fx.s2c = "Configuration";
			} else if (v?.kind === "compress") {
				fx.threshold = v.value;
			} else if (v?.kind === "encryption_begin") {
				fx.opaque = true;
			}
			return [r, fx];
		}
		case "c2s Configuration": {
			const [v, r] = check(data, gen.configuration.toServer.packet, full);
			if (v?.kind === "finish_configuration") {
				fx.c2s = "Play";
			}
			return [r, fx];
		}
		case "s2c Configuration": {
			const [v, r] = check(data, gen.configuration.toClient.packet, full);
			if (v?.kind === "finish_configuration") {
				fx.s2c = "Play";
			}
			return [r, fx];
		}
		case "c2s Play": {
			const [v, r] = check(data, gen.play.toServer.packet, full);
			if (v?.kind === "configuration_acknowledged") {
				fx.c2s = "Configuration";
			}
			return [r, fx];
		}
		case "s2c Play": {
			const [v, r] = check(data, gen.play.toClient.packet, full);
			if (v?.kind === "start_configuration") {
				fx.s2c = "Configuration";
			}
			return [r, fx];
		}
	}
	throw new Error(`unreachable state ${dir} ${state}`);
}

class FrameReader {
	#conn: Deno.Conn;
	#buf = new Uint8Array(0);

	constructor(conn: Deno.Conn) {
		this.#conn = conn;
	}

	async #fill(n: number): Promise<boolean> {
		while (this.#buf.length < n) {
			const chunk = new Uint8Array(64 * 1024);
			const read = await this.#conn.read(chunk);
			if (read === null) {
				return false;
			}
			const next = new Uint8Array(this.#buf.length + read);
			next.set(this.#buf);
			next.set(chunk.subarray(0, read), this.#buf.length);
			this.#buf = next;
		}
		return true;
	}

	async next(): Promise<{ frame: Uint8Array; start: number } | null> {
		let len = 0;
		for (let i = 0;; i++) {
			if (i >= 5) {
				throw new Error("frame length varint too long");
			}
			if (!(await this.#fill(i + 1))) {
				if (i === 0) {
					return null;
				}
				throw new Error("unexpected end of stream");
			}
			const b = this.#buf[i];
			len |= (b & 0x7f) << (7 * i);
			if ((b & 0x80) === 0) {
				const start = i + 1;
				if (!(await this.#fill(start + len))) {
					throw new Error("unexpected end of stream");
				}
				const frame = this.#buf.slice(0, start + len);
				this.#buf = this.#buf.subarray(start + len);
				return { frame, start };
			}
		}
	}
}

function packetData(body: Uint8Array, threshold: number): Uint8Array {
	if (threshold < 0) {
		return body;
	}
	const header = varint(body);
	if (!header) {
		throw new Error("bad data length");
	}
	const [dataLen, n] = header;
	const rest = body.subarray(n);
	if (dataLen === 0) {
		return rest;
	}
	const out = new Uint8Array(inflateSync(rest));
	if (out.length !== dataLen) {
		throw new Error(`inflated ${out.length} bytes, header says ${dataLen}`);
	}
	return out;
}

async function writeAll(to: Deno.Conn, data: Uint8Array): Promise<void> {
	for (let n = 0; n < data.length;) {
		n += await to.write(data.subarray(n));
	}
}

function closeAll(...conns: Deno.Conn[]): void {
	for (const c of conns) {
		try {
			c.close();
		} catch {
			continue;
		}
	}
}

async function pump(
	conn: Conn,
	dir: Dir,
	from: Deno.Conn,
	to: Deno.Conn,
	opts: Opts,
): Promise<void> {
	const reader = new FrameReader(from);
	const log = (line: string) => opts.log(`#${conn.id} ${line}`);
	try {
		while (true) {
			const next = await reader.next();
			if (!next) {
				break;
			}
			if (!conn.opaque) {
				const state = conn[dir];
				try {
					const data = packetData(next.frame.subarray(next.start), conn.threshold);
					const [report, fx] = inspect(dir, state, data, opts.full);
					if (fx.c2s) conn.c2s = fx.c2s;
					if (fx.s2c) conn.s2c = fx.s2c;
					if (fx.threshold !== undefined) conn.threshold = fx.threshold;
					if (fx.opaque) conn.opaque = true;
					const key = `${dir}\0${state}\0${report.name}`;
					const entry = conn.stats.get(key) ??
						{ dir, state, name: report.name, counts: [0, 0, 0, 0] };
					entry.counts[OUTCOMES.indexOf(report.outcome)]++;
					conn.stats.set(key, entry);
					if (report.outcome !== "ok" || !opts.failuresOnly) {
						log(`${dir} ${state.padEnd(13)} ${report.outcome.padEnd(8)} ${report.detail}`);
					}
					if (fx.opaque) {
						log(
							"server requested encryption: online-mode traffic cannot be inspected, passing through",
						);
					}
				} catch (e) {
					log(`${dir} frame decode error: ${e instanceof Error ? e.message : e}`);
				}
			}
			await writeAll(to, next.frame);
		}
	} catch (e) {
		if (!conn.closing) {
			log(`${dir} stream error: ${e instanceof Error ? e.message : e}`);
		}
	} finally {
		conn.closing = true;
		closeAll(from, to);
	}
}

function summary(conn: Conn, opts: Opts): void {
	const order: State[] = ["Handshaking", "Status", "Login", "Configuration", "Play"];
	const rows = [...conn.stats.values()].sort((a, b) =>
		a.dir.localeCompare(b.dir) || order.indexOf(a.state) - order.indexOf(b.state) ||
		a.name.localeCompare(b.name)
	);
	opts.log(`#${conn.id} summary: dir state packet ok/mismatch/error/panic`);
	for (const r of rows) {
		const flag = r.counts.slice(1).some((n) => n > 0) ? "!" : " ";
		opts.log(
			`#${conn.id} ${flag} ${r.dir} ${r.state.padEnd(13)} ${r.name.padEnd(45)} ${
				r.counts.join("/")
			}`,
		);
	}
}

async function handle(id: number, client: Deno.Conn, opts: Opts): Promise<void> {
	let server: Deno.Conn;
	try {
		server = await Deno.connect(opts.upstream);
	} catch (e) {
		opts.log(`#${id} upstream connect failed: ${e instanceof Error ? e.message : e}`);
		closeAll(client);
		return;
	}
	const conn: Conn = {
		id,
		c2s: "Handshaking",
		s2c: "Handshaking",
		threshold: -1,
		opaque: false,
		closing: false,
		stats: new Map(),
	};
	const peer = client.remoteAddr as Deno.NetAddr;
	opts.log(
		`#${id} connected ${peer.hostname}:${peer.port} -> ${opts.upstream.hostname}:${opts.upstream.port}`,
	);
	await Promise.all([
		pump(conn, "c2s", client, server, opts),
		pump(conn, "s2c", server, client, opts),
	]);
	summary(conn, opts);
}

export function startProxy(opts: Opts): { listener: Deno.Listener; done: Promise<void> } {
	const listener = Deno.listen(opts.listen);
	const done = (async () => {
		let next = 1;
		const running: Promise<void>[] = [];
		try {
			for await (const client of listener) {
				running.push(handle(next++, client, opts));
			}
		} finally {
			await Promise.all(running);
		}
	})();
	return { listener, done };
}
