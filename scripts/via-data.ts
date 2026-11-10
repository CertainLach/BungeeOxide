type Bytes = { bytes: number[] };
type Nbt = number | string | Bytes | number[] | Nbt[] | { [key: string]: Nbt };
type Compound = { [key: string]: Nbt };

export function read(file: string): Compound {
	const bytes = Deno.readFileSync(file);
	const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
	let at = 0;
	const int = () => {
		const v = view.getInt32(at);
		at += 4;
		return v;
	};
	const str = () => {
		const n = view.getUint16(at);
		at += 2;
		const s = new TextDecoder().decode(bytes.subarray(at, at + n));
		at += n;
		return s;
	};
	const payload = (t: number): Nbt => {
		switch (t) {
			case 1:
				return view.getInt8(at++);
			case 2:
				at += 2;
				return view.getInt16(at - 2);
			case 3:
				return int();
			case 4:
				at += 8;
				return Number(view.getBigInt64(at - 8));
			case 5:
				at += 4;
				return view.getFloat32(at - 4);
			case 6:
				at += 8;
				return view.getFloat64(at - 8);
			case 7: {
				const n = int();
				at += n;
				return { bytes: Array.from(bytes.subarray(at - n, at)) };
			}
			case 8:
				return str();
			case 9: {
				const et = bytes[at++];
				return Array.from({ length: int() }, () => payload(et));
			}
			case 10: {
				const o: Compound = {};
				for (let ct = bytes[at++]; ct !== 0; ct = bytes[at++]) {
					const name = str();
					o[name] = payload(ct);
				}
				return o;
			}
			case 11:
				return Array.from({ length: int() }, int);
		}
		throw new Error(`${file}: nbt type ${t}`);
	};
	const t = bytes[at++];
	str();
	return payload(t) as Compound;
}

export function varints(bytes: number[]): number[] {
	const out: number[] = [];
	let v = 0, shift = 0;
	for (const b of bytes) {
		v |= (b & 0x7f) << shift;
		shift += 7;
		if ((b & 0x80) === 0) {
			out.push(v >>> 0);
			v = 0;
			shift = 0;
		}
	}
	return out;
}

const zigzag = (v: number) => (v >>> 1) ^ -(v & 1);

export function mapping(tag: Compound): number[] {
	const size = tag.size as number;
	const values = varints((tag.val as Bytes | undefined)?.bytes ?? []);
	switch (tag.id as number) {
		case 3:
			return Array.from({ length: size }, (_, i) => i);
		case 0: {
			let prev = 0;
			return values.slice(0, size).map((v) => (prev += zigzag(v)));
		}
	}
	const pairs: [number, number][] = [];
	for (let i = 0, prevAt = -1, prevValue = 0; i + 1 < values.length; i += 2) {
		prevAt += 1 + values[i];
		prevValue += zigzag(values[i + 1]);
		pairs.push([prevAt, prevValue]);
	}
	const result = Array.from({ length: size }, (_, i) => i);
	if (tag.id === 1) {
		pairs.forEach(([from, to], i) => {
			const end = i + 1 < pairs.length ? pairs[i + 1][0] : size;
			for (let id = from; id < end; id++) result[id] = to++;
		});
		return result;
	}
	if (tag.id === 2) {
		for (const [changed, value] of pairs) result[changed] = value;
		return result;
	}
	throw new Error(`unknown mapping strategy ${tag.id}`);
}

function ranges(bytes: number[]): number[] {
	const values = varints(bytes);
	const ids: number[] = [];
	for (let i = 0, prevEnd = 0; i + 1 < values.length; i += 2) {
		const start = prevEnd + values[i];
		const end = start + values[i + 1];
		for (let id = start; id <= end; id++) ids.push(id);
		prevEnd = end + 1;
	}
	return ids;
}

const [via, mode, target, out, filter] = Deno.args;
const keep = new RegExp(filter ?? "");
const table = (import.meta.main && via ? read(`${via}/identifier-table.nbt`) : {}) as { [key: string]: string[] };
const registries: Record<string, string> = { block: "blocks", item: "items", entity_type: "entities" };
const fluids = ["empty", "flowing_water", "water", "flowing_lava", "lava"];

function names(version: string, type: string): string[] | undefined {
	if (type === "fluid") return fluids;
	const key = registries[type];
	const ids = key ? (read(`${via}/identifiers-${version}.nbt`)[key] as Compound | undefined) : undefined;
	return ids && mapping(ids).map((g) => table[key][g].replace(/^minecraft:/, ""));
}

const steps = [
	["1.12", "1.13"], ["1.13", "1.13.2"], ["1.13.2", "1.14"], ["1.14", "1.15"], ["1.15", "1.16"], ["1.16", "1.16.2"],
	["1.16.2", "1.17"], ["1.17", "1.18"], ["1.18", "1.19"], ["1.19", "1.19.3"], ["1.19.3", "1.19.4"], ["1.19.4", "1.20"],
	["1.20", "1.20.2"], ["1.20.2", "1.20.3"], ["1.20.3", "1.20.5"],
];

function tags(version: string): Record<string, Record<string, string[]>> {
	const out: Record<string, Record<string, string[]>> = {};
	const last = steps.findIndex(([, to]) => to === version);
	if (last < 0) throw new Error(`no ViaVersion mappings reach ${version}`);
	for (const [from, to] of steps.slice(0, last + 1)) {
		const data = read(`${via}/mappings-${from}to${to}.nbt`);
		const sources: [string, Compound][] = [["block_tags", "block"], ["item_tags", "item"], ["fluid_tags", "fluid"]]
			.filter(([key]) => data[key])
			.map(([key, type]) => [type, data[key] as Compound]);
		sources.push(...Object.entries((data.tags ?? {}) as Compound).map(([type, v]) => [type, v as Compound] as [string, Compound]));
		for (const [type, list] of sources) {
			const known = names(to, type);
			if (!known) continue;
			for (const [tag, value] of Object.entries(list)) {
				if (!keep.test(`${type}/${tag.replace(/^minecraft:/, "")}`)) continue;
				const ids = Array.isArray(value) ? (value as number[]) : ranges((value as Bytes).bytes);
				(out[type] ??= {})[tag.replace(/^minecraft:/, "")] = [...new Set(ids.map((id) => known[id]).filter(Boolean))].sort();
			}
		}
	}
	return Object.fromEntries(
		Object.keys(out).sort().map((type) => [type, Object.fromEntries(Object.keys(out[type]).sort().map((t) => [t, out[type][t]]))]),
	);
}

function registry(version: string, key: string): { id: number; name: string }[] {
	const ids = read(`${via}/identifiers-${version}.nbt`)[key] as Compound | undefined;
	if (!ids) throw new Error(`ViaVersion has no ${key} for ${version}`);
	return mapping(ids).map((g, id) => ({ id, name: table[key][g].replace(/^minecraft:/, "") }));
}

if (import.meta.main) {
	if (!via || !mode || !target || !out) {
		throw new Error("usage: via-data.ts <ViaVersion data dir> <tags | registry key> <version> <patch file creating the data file> [type/tag regex]");
	}
	const value = mode === "tags" ? tags(target) : registry(target, mode);
	await Deno.mkdir(out.slice(0, out.lastIndexOf("/")), { recursive: true });
	await Deno.writeTextFile(out, JSON.stringify([{ op: "add", path: "", value }], null, "\t") + "\n");
}
