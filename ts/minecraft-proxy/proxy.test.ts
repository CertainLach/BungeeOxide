import { assert, assertEquals, assertStringIncludes } from "@std/assert";
import { deflateSync } from "node:zlib";
import { Input, Output } from "../minecraft-protocol/pds.ts";
import * as gen from "../minecraft-protocol/gen/mod.ts";
import { check, startProxy } from "./proxy.ts";

const THRESHOLD = 64;

function ser<T>(v: T, codec: { serialize(v: T, o: Output): void }): Uint8Array {
	const out = new Output();
	codec.serialize(v, out);
	return out.into_inner();
}

function varint(v: number): number[] {
	const out: number[] = [];
	let value = v >>> 0;
	while (true) {
		const b = value & 0x7f;
		value >>>= 7;
		if (value === 0) {
			out.push(b);
			return out;
		}
		out.push(b | 0x80);
	}
}

function frame(data: Uint8Array, compressed: boolean): Uint8Array {
	let body: number[];
	if (!compressed) {
		body = [...data];
	} else if (data.length < THRESHOLD) {
		body = [...varint(0), ...data];
	} else {
		body = [...varint(data.length), ...deflateSync(data)];
	}
	return Uint8Array.from([...varint(body.length), ...body]);
}

class Frames {
	#buf: number[] = [];
	constructor(private conn: Deno.Conn) {}
	async skip(): Promise<void> {
		const need = async (n: number) => {
			while (this.#buf.length < n) {
				const chunk = new Uint8Array(4096);
				const r = await this.conn.read(chunk);
				if (r === null) throw new Error("eof");
				this.#buf.push(...chunk.subarray(0, r));
			}
		};
		let len = 0;
		let i = 0;
		for (;; i++) {
			await need(i + 1);
			len |= (this.#buf[i] & 0x7f) << (7 * i);
			if ((this.#buf[i] & 0x80) === 0) break;
		}
		await need(i + 1 + len);
		this.#buf.splice(0, i + 1 + len);
	}
}

async function fakeServer(listener: Deno.Listener): Promise<void> {
	const s = await listener.accept();
	const r = new Frames(s);
	await r.skip();
	await r.skip();
	const login = gen.login.toClient.packet;
	await s.write(frame(ser({ kind: "compress", value: THRESHOLD }, login), false));
	await s.write(
		frame(
			ser({ kind: "success", value: { uuid: 0x1234n, username: "pds", properties: [] } }, login),
			true,
		),
	);
	await r.skip();
	const config = gen.configuration.toClient.packet;
	const brand = {
		kind: "custom_payload" as const,
		value: { channel: "minecraft:brand", data: new Uint8Array(300).fill(7) },
	};
	await s.write(frame(ser(brand, config), true));
	await s.write(frame(ser({ kind: "finish_configuration", value: {} }, config), true));
	await r.skip();
	await s.write(frame(ser({ kind: "keep_alive", value: 42n }, gen.play.toClient.packet), true));
	const rest = new Uint8Array(16);
	while (await s.read(rest) !== null);
	s.close();
}

async function fakeClient(port: number): Promise<void> {
	const c = await Deno.connect({ hostname: "127.0.0.1", port });
	const r = new Frames(c);
	const handshake = {
		kind: "set_protocol" as const,
		value: { protocolVersion: 774, serverHost: "localhost", serverPort: 25565, nextState: 2 },
	};
	await c.write(frame(ser(handshake, gen.handshaking.toServer.packet), false));
	const login = gen.login.toServer.packet;
	await c.write(
		frame(
			ser({ kind: "login_start", value: { username: "pds", playerUUID: 0x1234n } }, login),
			false,
		),
	);
	await r.skip();
	await r.skip();
	await c.write(frame(ser({ kind: "login_acknowledged", value: {} }, login), true));
	await r.skip();
	await r.skip();
	await c.write(
		frame(
			ser({ kind: "finish_configuration", value: {} }, gen.configuration.toServer.packet),
			true,
		),
	);
	await r.skip();
	c.close();
}

Deno.test("proxies login through compression into play", async () => {
	const upstream = Deno.listen({ hostname: "127.0.0.1", port: 0 });
	const lines: string[] = [];
	const { listener, done } = startProxy({
		listen: { hostname: "127.0.0.1", port: 0 },
		upstream: { hostname: "127.0.0.1", port: (upstream.addr as Deno.NetAddr).port },
		full: false,
		failuresOnly: false,
		log: (line) => lines.push(line),
	});
	const server = fakeServer(upstream);
	await fakeClient((listener.addr as Deno.NetAddr).port);
	await server;
	for (let i = 0; i < 100 && !lines.some((l) => l.includes("summary")); i++) {
		await new Promise((resolve) => setTimeout(resolve, 10));
	}
	await new Promise((resolve) => setTimeout(resolve, 20));
	listener.close();
	upstream.close();
	await done;
	const log = lines.join("\n");
	console.log(log);
	for (const bad of ["MISMATCH", "ERROR", "PANIC", " error:", " ! "]) {
		assert(!log.includes(bad), `found ${bad} in proxy log`);
	}
	for (
		const [dir, state, packet] of [
			["c2s", "Handshaking", "set_protocol"],
			["c2s", "Login", "login_start"],
			["s2c", "Login", "compress"],
			["s2c", "Login", "success"],
			["c2s", "Login", "login_acknowledged"],
			["s2c", "Configuration", "custom_payload"],
			["s2c", "Configuration", "finish_configuration"],
			["c2s", "Configuration", "finish_configuration"],
			["s2c", "Play", "keep_alive"],
		]
	) {
		const needle = `${dir} ${state.padEnd(13)} ok       { kind: "${packet}"`;
		assert(lines.some((l) => l.includes(needle)), `missing ok ${dir} ${state} ${packet}`);
	}
});

Deno.test("reports panics and mismatches", () => {
	const [, panic] = check(Uint8Array.from([1]), {
		deserialize(_input: Input): number {
			throw new TypeError("nope");
		},
		serialize() {},
		size_of: () => 1,
	}, false);
	assertEquals(panic.outcome, "PANIC");
	assertStringIncludes(panic.detail, "nope");

	const [, mismatch] = check(Uint8Array.from([1, 2]), {
		deserialize: (input: Input) => input.read(),
		serialize: (v: number, o: Output) => o.write_byte(v + 1),
		size_of: () => 1,
	}, false);
	assertEquals(mismatch.outcome, "MISMATCH");
	assertStringIncludes(mismatch.detail, "consumed 1 of 2");
	assertStringIncludes(mismatch.detail, "first difference at byte 0");
	assertStringIncludes(mismatch.detail, "size_of 1 != 2");
});
