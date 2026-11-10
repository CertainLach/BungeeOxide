import mc from "npm:minecraft-protocol@1";
import prismarineChunk from "npm:prismarine-chunk@1";
import { Buffer } from "node:buffer";
import { Vec3 } from "npm:vec3@0";

const [file, version, other, ...rest] = Deno.args;
if (!file || !version) throw new Error("usage: check-dump.ts <dump> <minecraft version> [<dump to compare>] [--min-y N] [--height N]");
const option = (name: string, fallback: number) => {
	const i = rest.indexOf(name);
	return i >= 0 ? Number(rest[i + 1]) : fallback;
};
const minY = option("--min-y", 0);
const worldHeight = option("--height", 256);
const Chunk = prismarineChunk(version);
const [major, minor] = version.split(".").map(Number);
const at = (a: number, b: number) => major > a || (major === a && (minor ?? 0) >= b);

type Column = { key: string; blocks: number[]; light: number[]; biomes: number[] };

function read(path: string) {
	const bytes = Deno.readFileSync(path);
	const deserializer = mc.createDeserializer({ state: "play", isServer: false, version, customPackets: {} });
	const kinds: Record<string, number> = {};
	const failures: Record<string, string> = {};
	const blocks: Record<string, number> = {};
	const columns: Column[] = [];
	const lights = new Map<string, any>();
	let offset = 0;
	while (offset + 4 <= bytes.length) {
		const len = new DataView(bytes.buffer, offset, 4).getUint32(0);
		const data = Buffer.from(bytes.subarray(offset + 4, offset + 4 + len));
		offset += 4 + len;
		let parsed;
		try {
			parsed = deserializer.parsePacketBuffer(data);
		} catch (e) {
			failures[`id ${data[0]}`] = String(e).slice(0, 300);
			continue;
		}
		const { name, params } = parsed.data;
		kinds[name] = (kinds[name] ?? 0) + 1;
		if (parsed.metadata.size !== data.length) failures[name] = `${data.length - parsed.metadata.size} bytes left over`;
		if (name === "update_light") lights.set(`${params.chunkX},${params.chunkZ}`, params);
		if (name !== "map_chunk") continue;
		try {
			const chunk = new Chunk({ minY, worldHeight });
			const key = `${params.x},${params.z}`;
			if (at(1, 18)) {
				chunk.load(params.chunkData);
				chunk.loadParsedLight(params.skyLight, params.blockLight, params.skyLightMask, params.blockLightMask, params.emptySkyLightMask, params.emptyBlockLightMask);
			} else if (at(1, 17)) {
				chunk.load(params.chunkData, params.bitMap);
				chunk.loadBiomes(params.biomes);
				const l = lights.get(key);
				if (l) chunk.loadParsedLight(l.skyLight, l.blockLight, l.skyLightMask, l.blockLightMask, l.emptySkyLightMask, l.emptyBlockLightMask);
			} else if (at(1, 15)) {
				chunk.load(params.chunkData, params.bitMap);
				if (params.biomes) chunk.loadBiomes(params.biomes);
				const l = lights.get(key);
				if (l) chunk.loadLight(l.data, l.skyLightMask, l.blockLightMask, l.emptySkyLightMask, l.emptyBlockLightMask);
			} else if (at(1, 14)) {
				chunk.load(params.chunkData, params.bitMap, true, params.groundUp);
				const l = lights.get(key);
				if (l) chunk.loadLight(l.data, l.skyLightMask, l.blockLightMask, l.emptySkyLightMask, l.emptyBlockLightMask);
			} else {
				chunk.load(params.chunkData, params.bitMap, params.skylight ?? true, params.groundUp ?? true);
			}
			lights.delete(key);
			const column: Column = { key, blocks: [], light: [], biomes: [] };
			const pos = new Vec3(0, 0, 0);
			for (let y = minY; y < minY + worldHeight; y++) {
				for (let z = 0; z < 16; z++) {
					for (let x = 0; x < 16; x++) {
						pos.set(x, y, z);
						column.blocks.push(chunk.getBlockStateId(pos));
						column.light.push(chunk.getBlockLight(pos) << 4 | chunk.getSkyLight(pos));
						if ((x & 3) === 0 && (y & 3) === 0 && (z & 3) === 0) column.biomes.push(chunk.getBiome(pos));
					}
				}
			}
			for (let y = minY; y < minY + 8; y++) {
				const b = chunk.getBlock(new Vec3(0, y, 0));
				blocks[b.name] = (blocks[b.name] ?? 0) + 1;
			}
			columns.push(column);
		} catch (e) {
			failures.chunk_load = String(e).slice(0, 300);
		}
	}
	return { kinds, blocks, failures, columns };
}

const first = read(file);
const report: Record<string, unknown> = { version, kinds: first.kinds, blocks: first.blocks, failures: first.failures, columns: first.columns.length };
if (other) {
	const second = read(other);
	const differences: string[] = [];
	if (second.columns.length !== first.columns.length) differences.push(`${first.columns.length} columns vs ${second.columns.length}`);
	for (let i = 0; i < Math.min(first.columns.length, second.columns.length); i++) {
		const [a, b] = [first.columns[i], second.columns[i]];
		const diff = (what: string, x: number[], y: number[]) => {
			const n = x.findIndex((v, k) => v !== y[k]);
			if (n >= 0 || x.length !== y.length) differences.push(`column ${i} (${a.key} / ${b.key}) ${what} differ at ${n}: ${x[n]} vs ${y[n]}`);
		};
		diff("blocks", a.blocks, b.blocks);
		diff("light", a.light, b.light);
		diff("biomes", a.biomes, b.biomes);
	}
	report.compared = { other, columns: second.columns.length, failures: second.failures, differences: differences.slice(0, 10), differing: differences.length };
}
console.log(JSON.stringify(report, null, 1));
