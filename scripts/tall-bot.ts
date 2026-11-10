import mineflayer from "npm:mineflayer@4";
import { Vec3 } from "npm:vec3@0.1";

const [address = "127.0.0.1:25631", version = "1.8.8", rconAddress = "127.0.0.1:25632", password = "tall", only = ""] = Deno.args;
const [host, port] = address.split(":");
const name = `tall_${version.replaceAll(".", "_")}`.slice(0, 16);
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

class Rcon {
	#conn: Deno.TcpConn;
	#id = 0;
	#buffer = new Uint8Array(0);
	#queue: Promise<unknown> = Promise.resolve();

	constructor(conn: Deno.TcpConn) {
		this.#conn = conn;
	}

	static async connect(address: string, password: string) {
		const [hostname, port] = address.split(":");
		const r = new Rcon(await Deno.connect({ hostname, port: Number(port) }));
		await r.#request(3, password);
		return r;
	}

	async #take(n: number) {
		while (this.#buffer.length < n) {
			const chunk = new Uint8Array(65536);
			const got = await this.#conn.read(chunk);
			if (got === null) throw new Error("rcon closed");
			const next = new Uint8Array(this.#buffer.length + got);
			next.set(this.#buffer);
			next.set(chunk.subarray(0, got), this.#buffer.length);
			this.#buffer = next;
		}
		const out = this.#buffer.slice(0, n);
		this.#buffer = this.#buffer.slice(n);
		return out;
	}

	async #request(type: number, body: string) {
		const id = ++this.#id;
		const text = new TextEncoder().encode(body);
		const packet = new Uint8Array(14 + text.length);
		const view = new DataView(packet.buffer);
		view.setInt32(0, 10 + text.length, true);
		view.setInt32(4, id, true);
		view.setInt32(8, type, true);
		packet.set(text, 12);
		let written = 0;
		while (written < packet.length) written += await this.#conn.write(packet.subarray(written));
		const length = new DataView((await this.#take(4)).buffer).getInt32(0, true);
		const rest = await this.#take(length);
		if (new DataView(rest.buffer).getInt32(0, true) === -1) throw new Error("rcon login failed");
		return new TextDecoder().decode(rest.subarray(8, rest.length - 2));
	}

	command(text: string): Promise<string> {
		const next = this.#queue.then(() => this.#request(2, text));
		this.#queue = next.catch(() => {});
		return next;
	}
}

const rcon = await Rcon.connect(rconAddress, password);
const report: Record<string, unknown> = { version, name };
const problems: string[] = [];
const log = (...parts: unknown[]) => console.error(`[${version}]`, ...parts);
const fail = (text: string) => {
	problems.push(text);
	log("PROBLEM", text);
};

const bot = mineflayer.createBot({ host, port: Number(port), username: name, version, auth: "offline" });
const forced: number[] = [];
bot.on("forcedMove", () => forced.push(bot.entity.position.y));
bot.on("kicked", (reason: unknown) => {
	report.kicked = typeof reason === "string" ? reason : JSON.stringify(reason);
	finish(1);
});
bot.on("error", (e: Error) => {
	report.error = String(e);
	finish(1);
});

function finish(code: number) {
	report.problems = problems;
	console.log(JSON.stringify(report));
	rcon.command(`gamemode creative ${name}`).finally(() => Deno.exit(code));
}

async function serverPos(): Promise<[number, number, number]> {
	const text = await rcon.command(`data get entity ${name} Pos`);
	const m = text.match(/\[(-?[\d.E-]+)d, (-?[\d.E-]+)d, (-?[\d.E-]+)d\]/);
	if (!m) throw new Error(`no position in ${text}`);
	return [Number(m[1]), Number(m[2]), Number(m[3])];
}

async function settle(ms = 1500) {
	let last = bot.entity.position.clone();
	let still = 0;
	for (let i = 0; i < 200 && still < ms; i += 1) {
		await sleep(100);
		const now = bot.entity.position;
		still = now.distanceTo(last) < 1e-3 ? still + 100 : 0;
		last = now.clone();
	}
}

const BASES = [-64, 0, 64];

async function where(label: string) {
	await settle();
	const s = await serverPos();
	const c = bot.entity.position;
	const offset = s[1] - c.y;
	const base = Math.round(offset);
	const entry = { label, server: s.map((v) => +v.toFixed(2)), client: [c.x, c.y, c.z].map((v) => +v.toFixed(2)), base };
	log("where", JSON.stringify(entry));
	if (Math.abs(s[0] - c.x) > 0.01 || Math.abs(s[2] - c.z) > 0.01) fail(`${label}: x/z differ ${JSON.stringify(entry)}`);
	if (Math.abs(offset - base) > 0.01 || !BASES.includes(base)) fail(`${label}: y offset ${offset} is not a window base`);
	if (c.y < 0 || c.y >= 256) fail(`${label}: client y ${c.y} outside 0..256`);
	checkLight(label, s[1]);
	return { s, base };
}

const MARKERS: [number, string][] = [
	[-63, "gold_block"],
	[-48, "diamond_block"],
	[-17, "emerald_block"],
	[-1, "lapis_block"],
	[0, "iron_block"],
	[1, "redstone_block"],
	[15, "coal_block"],
	[16, "gold_block"],
	[63, "diamond_block"],
	[64, "emerald_block"],
	[100, "lapis_block"],
	[191, "iron_block"],
	[192, "redstone_block"],
	[200, "coal_block"],
	[240, "gold_block"],
	[255, "diamond_block"],
	[256, "emerald_block"],
	[300, "lapis_block"],
	[319, "iron_block"],
];
let column: [number, number] = [0, 0];

async function placeMarkers(x: number, z: number) {
	column = [x, z];
	for (const [y] of MARKERS) await rcon.command(`setblock ${x} ${y} ${z} minecraft:glass`);
	await sleep(300);
	for (const [y, block] of MARKERS) {
		const out = await rcon.command(`setblock ${x} ${y} ${z} minecraft:${block}`);
		if (!out.startsWith("Changed")) log("setblock", y, out);
	}
}

async function checkMarkers(label: string, base: number) {
	await sleep(500);
	let seen = 0;
	const loaded = bot.blockAt(new Vec3(column[0], 64, column[1])) !== null;
	const near = Math.max(Math.abs(column[0] - bot.entity.position.x), Math.abs(column[1] - bot.entity.position.z)) < 64;
	if (!loaded) {
		if (near) fail(`${label}: the marker column next to the bot is not loaded`);
		log(label, "marker column not loaded");
		return;
	}
	for (const [y, block] of MARKERS) {
		const cy = y - base;
		if (cy < 0 || cy >= 256) continue;
		const got = bot.blockAt(new Vec3(column[0], cy, column[1]));
		if (got?.name !== block) fail(`${label}: marker ${block} at server y ${y} shows ${got?.name ?? "nothing"} at client y ${cy}`);
		else seen += 1;
	}
	log(label, `markers seen ${seen}`);
}

function solidAround(radius = 2) {
	const p = bot.entity.position.floored();
	let solid = 0, loaded = 0, lit = 0;
	for (let dx = -radius; dx <= radius; dx += 1) {
		for (let dy = -radius; dy <= radius; dy += 1) {
			for (let dz = -radius; dz <= radius; dz += 1) {
				const at = p.offset(dx, dy, dz);
				const b = bot.blockAt(at);
				if (b) loaded += 1;
				if (b && b.boundingBox === "block") {
					solid += 1;
					if (bot.world.getSkyLight(at) === 15) lit += 1;
				}
			}
		}
	}
	return { solid, loaded, lit };
}

function checkLight(label: string, serverY: number) {
	const around = solidAround(3);
	const head = bot.world.getSkyLight(bot.entity.position.floored().offset(0, 1, 0));
	log(label, "light", JSON.stringify({ ...around, head }));
	if (serverY < 30 && around.solid > 20 && around.lit > around.solid / 2) fail(`${label}: ${around.lit} of ${around.solid} solid blocks around are sky lit underground`);
	return around;
}

async function phase(label: string, run: () => Promise<void>) {
	if (only && !only.split(",").includes(label)) return;
	log("phase", label);
	const before = forced.length;
	try {
		await run();
	} catch (e) {
		fail(`${label}: ${e}`);
	}
	(report.phases ??= {} as Record<string, unknown>);
	(report.phases as Record<string, unknown>)[label] = { teleports: forced.slice(before) };
}

async function digDown(targetServerY: number) {
	let { s, base } = await where("dig start");
	const startClient = bot.entity.position.y;
	let travelled = 0;
	for (let i = 0; i < 600 && s[1] > targetServerY; i += 1) {
		const below = bot.blockAt(bot.entity.position.offset(0, -1, 0).floored());
		if (!below) {
			await sleep(200);
			continue;
		}
		if (below.boundingBox === "block" || below.name.includes("water") || below.name.includes("lava")) {
			if (below.name === "bedrock") break;
			if (below.boundingBox === "block") {
				try {
					await bot.dig(below, true);
				} catch (e) {
					log("dig failed", String(e));
					await sleep(200);
				}
			}
		}
		await sleep(150);
		const y = bot.entity.position.y;
		if (Math.abs(y - startClient - travelled) > 0) travelled = y - startClient;
		if (i % 10 === 0) s = await serverPos();
	}
	return await where("dig end");
}

async function flyBy(dy: number) {
	bot.creative.startFlying();
	let moved = 0;
	const step = 0.5 * Math.sign(dy);
	while (Math.abs(moved) < Math.abs(dy)) {
		bot.entity.velocity.set(0, 0, 0);
		bot.entity.position.y += step;
		moved += step;
		await sleep(50);
	}
	await sleep(500);
}

async function waitFor(event: string, ms: number) {
	return await Promise.race([
		new Promise((r) => bot.once(event, () => r(true))),
		sleep(ms).then(() => false),
	]);
}

bot.once("spawn", async () => {
	try {
		report.spawn = true;
		await rcon.command(`op ${name}`);
		await rcon.command(`gamemode creative ${name}`);
		await sleep(2000);
		const shift = () => Math.floor(Math.random() * 96) - 48;
		const home = (await serverPos()).map((v) => Math.floor(v) + 0.5 + shift());
		const surface = await rcon.command(`execute positioned ${home[0]} 0 ${home[2]} positioned over motion_blocking_no_leaves run tp ${name} ~ ~ ~`);
		log("surface", surface);
		await waitFor("forcedMove", 5000);
		await sleep(4000);
		let { s, base } = await where("spawn");
		if (base !== 0) fail(`spawn: expected identity window, got base ${base}`);
		const [sx, sy, sz] = s.map(Math.floor);
		await placeMarkers(sx + 2, sz + 2);
		await checkMarkers("spawn", base);
		report.spawnAround = solidAround();

		await phase("dig", async () => {
			({ s, base } = await digDown(-24));
			if (s[1] > 0) fail(`dig: only reached server y ${s[1]}`);
			if (s[1] < 16 && base !== -64) fail(`dig: expected the deep window at server y ${s[1]}, got base ${base}`);
			await checkMarkers("dig", base);
			const around = solidAround();
			if (around.solid < 20) fail(`dig: only ${around.solid} solid blocks around the bot`);
			report.digAround = around;
		});

		await phase("fly", async () => {
			const [fx, fy, fz] = (await serverPos()).map(Math.floor);
			await rcon.command(`fill ${fx} ${fy + 2} ${fz} ${fx} 319 ${fz} minecraft:air`);
			await rcon.command(`tp ${name} ${fx + 0.5} ${fy} ${fz + 0.5}`);
			await waitFor("forcedMove", 5000);
			await sleep(1000);
			const start = (await serverPos())[1];
			await flyBy(300 - start);
			({ s, base } = await where("fly"));
			if (s[1] < 290) fail(`fly: only reached server y ${s[1]}`);
			if (base !== 64) fail(`fly: expected the high window at server y ${s[1]}, got base ${base}`);
			await checkMarkers("fly", base);
		});

		await phase("fall", async () => {
			bot.creative.stopFlying();
			for (let i = 0; i < 100 && !bot.entity.onGround; i += 1) await sleep(200);
			({ s, base } = await where("fall"));
			await checkMarkers("fall", base);
			const below = bot.blockAt(bot.entity.position.offset(0, -0.5, 0).floored());
			if (!below || below.boundingBox !== "block") fail(`fall: standing on ${below?.name ?? "nothing"}`);
		});

		await phase("tp", async () => {
			const [x, z] = [column[0] - 2, column[1] - 2];
			await rcon.command(`fill ${x - 1} -42 ${z - 1} ${x + 1} -38 ${z + 1} minecraft:air`);
			await rcon.command(`fill ${x - 1} -43 ${z - 1} ${x + 1} -43 ${z + 1} minecraft:stone`);
			await rcon.command(`tp ${name} ${x + 0.5} -42 ${z + 0.5}`);
			await waitFor("forcedMove", 5000);
			({ s, base } = await where("tp deep"));
			if (base !== -64) fail(`tp: expected the deep window at -42, got ${base}`);
			await checkMarkers("tp deep", base);
			await rcon.command(`tp ${name} ${x + 0.5} 290 ${z + 0.5}`);
			await waitFor("forcedMove", 5000);
			bot.creative.startFlying();
			({ s, base } = await where("tp high"));
			if (base !== 64) fail(`tp: expected the high window at 290, got ${base}`);
			await checkMarkers("tp high", base);
			await rcon.command(`tp ${name} ${x + 0.5} 120 ${z + 0.5}`);
			await waitFor("forcedMove", 5000);
			({ s, base } = await where("tp middle"));
			if (base !== 0) fail(`tp: expected the identity window at 120, got ${base}`);
			await checkMarkers("tp middle", base);
		});

		await phase("respawn", async () => {
			await rcon.command(`tp ${name} ${column[0] - 2 + 0.5} -42 ${column[1] - 2 + 0.5}`);
			await waitFor("forcedMove", 5000);
			await where("before death");
			await rcon.command(`gamemode survival ${name}`);
			await rcon.command(`kill ${name}`);
			const respawned = await waitFor("spawn", 10000);
			if (!respawned) fail("respawn: no spawn event");
			await sleep(3000);
			({ s, base } = await where("respawn"));
			const fresh = s[1] < 32 ? -64 : s[1] > 224 ? 64 : 0;
			if (base !== fresh) fail(`respawn: expected window ${fresh} at server y ${s[1]}, got ${base}`);
			await checkMarkers("respawn", base);
			report.respawnAround = solidAround();
			await rcon.command(`gamemode creative ${name}`);
		});

		await phase("survival", async () => {
			const [x, z] = [column[0] - 2, column[1] - 2];
			await rcon.command(`tp ${name} ${x + 0.5} 190 ${z + 0.5}`);
			await waitFor("forcedMove", 5000);
			bot.creative.startFlying();
			await sleep(3000);
			await rcon.command(`fill ${x - 1} 216 ${z - 1} ${x + 1} 216 ${z + 1} minecraft:stone`);
			await rcon.command(`tp ${name} ${x + 0.5} 217 ${z + 0.5}`);
			await waitFor("forcedMove", 5000);
			bot.creative.stopFlying();
			await rcon.command(`gamemode survival ${name}`);
			({ s, base } = await where("survival platform"));
			if (base !== 0) fail(`survival: expected the identity window at 217, got ${base}`);
			await rcon.command(`give ${name} minecraft:stone 64`);
			await sleep(1000);
			const side = bot.blockAt(bot.entity.position.floored().offset(1, -1, 0));
			const held = bot.inventory.items().find((i) => i.name === "stone");
			if (held) await bot.equip(held, "hand");
			try {
				if (side) await bot.placeBlock(side, new Vec3(0, 1, 0));
				const placed = await rcon.command(`execute if block ${x + 1} 217 ${z} minecraft:stone`);
				if (!placed.includes("passed")) fail(`survival: the block next to the bot is not on the server: ${placed}`);
			} catch (e) {
				log("side place failed", String(e));
			}
			await rcon.command(`gamemode creative ${name}`);
			await rcon.command(`give ${name} minecraft:stone 64`);
			await sleep(1000);
			const stone = bot.inventory.items().find((i) => i.name === "stone");
			if (stone) await bot.equip(stone, "hand");
			let placed = 0;
			for (let i = 0; i < 14; i += 1) {
				const top = bot.entity.position.floored();
				const block = bot.blockAt(top.offset(0, -1, 0));
				await flyBy(1.5);
				if (block) {
					try {
						await bot.placeBlock(block, new Vec3(0, 1, 0));
						placed += 1;
					} catch (e) {
						log("place failed", String(e));
					}
				}
				bot.creative.stopFlying();
				for (let t = 0; t < 40 && !bot.entity.onGround; t += 1) await sleep(25);
				await sleep(150);
			}
			report.placed = placed;
			({ s, base } = await where("pillar"));
			if (s[1] > 225 && base !== 64) fail(`pillar: expected the high window at ${s[1]}, got ${base}`);
			if (s[1] <= 225) fail(`pillar: only reached ${s[1]}`);
			await checkMarkers("pillar", base);
			await rcon.command(`gamemode creative ${name}`);
		});

		await phase("nether", async () => {
			await rcon.command(`execute in minecraft:the_nether run tp ${name} 0.5 69 0.5`);
			await waitFor("spawn", 10000);
			bot.creative.startFlying();
			await sleep(3000);
			await rcon.command(`execute in minecraft:the_nether run fill -1 69 -1 1 72 1 minecraft:air`);
			await rcon.command(`execute in minecraft:the_nether run fill -1 68 -1 1 68 1 minecraft:netherrack`);
			await rcon.command(`execute in minecraft:the_nether run tp ${name} 0.5 69 0.5`);
			await waitFor("forcedMove", 5000);
			bot.creative.stopFlying();
			await sleep(3000);
			({ s, base } = await where("nether"));
			if (base !== 0) fail(`nether: expected no shift, got ${base}`);
			const below = bot.blockAt(bot.entity.position.offset(0, -0.5, 0).floored());
			if (below?.name !== "netherrack") fail(`nether: standing on ${below?.name ?? "nothing"}`);
			const [x, z] = [column[0] - 2, column[1] - 2];
			await rcon.command(`execute in minecraft:overworld run tp ${name} ${x + 0.5} -42 ${z + 0.5}`);
			await waitFor("spawn", 10000);
			await sleep(4000);
			({ s, base } = await where("back from the nether"));
			if (base !== -64) fail(`nether: expected the deep window at ${s[1]}, got ${base}`);
			await checkMarkers("back from the nether", base);
		});

		await phase("goto", async () => {
			const [x, z] = [column[0] - 2, column[1] - 2];
			await rcon.command(`tp ${name} ${x + 0.5} -42 ${z + 0.5}`);
			await waitFor("forcedMove", 5000);
			await where("before goto");
			bot.chat(`/proxy-goto ${Deno.env.get("TALL_GOTO") ?? "127.0.0.1:25630"}`);
			await waitFor("spawn", 10000);
			await sleep(4000);
			({ s, base } = await where("after goto"));
			if (base !== -64) fail(`goto: expected the deep window at ${s[1]}, got ${base}`);
			await checkMarkers("goto", base);
		});

		report.done = true;
		finish(problems.length ? 1 : 0);
	} catch (e) {
		report.error = String(e);
		finish(1);
	}
});

setTimeout(() => {
	report.timeout = true;
	finish(1);
}, 420_000);
