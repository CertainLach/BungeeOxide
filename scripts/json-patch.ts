type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export type Operation =
	| { op: "add" | "replace" | "test"; path: string; value: Json }
	| { op: "remove"; path: string }
	| { op: "move" | "copy"; from: string; path: string };

function tokens(pointer: string): string[] {
	if (pointer === "") return [];
	if (!pointer.startsWith("/")) throw new Error(`bad JSON pointer ${JSON.stringify(pointer)}`);
	return pointer.slice(1).split("/").map((t) => t.replaceAll("~1", "/").replaceAll("~0", "~"));
}

function parent(doc: Json, pointer: string): [Json, string] {
	const path = tokens(pointer);
	const last = path.pop();
	if (last === undefined) throw new Error("the document root has no parent");
	let at = doc;
	for (const t of path) {
		const next = Array.isArray(at) ? at[Number(t)] : at !== null && typeof at === "object" ? at[t] : undefined;
		if (next === undefined) throw new Error(`${pointer}: ${t} does not exist`);
		at = next;
	}
	return [at, last];
}

function get(doc: Json, pointer: string): Json {
	if (pointer === "") return doc;
	const [at, key] = parent(doc, pointer);
	const value = Array.isArray(at) ? at[Number(key)] : (at as Record<string, Json>)[key];
	if (value === undefined) throw new Error(`${pointer} does not exist`);
	return value;
}

function index(list: Json[], key: string, append: boolean, pointer: string): number {
	if (append && key === "-") return list.length;
	const i = Number(key);
	if (!Number.isInteger(i) || i < 0 || i > list.length - (append ? 0 : 1)) throw new Error(`${pointer}: bad index ${key}`);
	return i;
}

function add(doc: Json, pointer: string, value: Json): Json {
	if (pointer === "") return value;
	const [at, key] = parent(doc, pointer);
	if (Array.isArray(at)) at.splice(index(at, key, true, pointer), 0, value);
	else if (at !== null && typeof at === "object") at[key] = value;
	else throw new Error(`${pointer}: parent is not a container`);
	return doc;
}

function remove(doc: Json, pointer: string): Json {
	const [at, key] = parent(doc, pointer);
	const old = get(doc, pointer);
	if (Array.isArray(at)) at.splice(index(at, key, false, pointer), 1);
	else delete (at as Record<string, Json>)[key];
	return old;
}

export function apply(doc: Json, ops: Operation[], name = "patch"): Json {
	ops.forEach((o, i) => {
		const where = `${name} #${i} (${o.op} ${o.path})`;
		try {
			switch (o.op) {
				case "add":
					doc = add(doc, o.path, structuredClone(o.value));
					break;
				case "remove":
					remove(doc, o.path);
					break;
				case "replace":
					if (o.path === "") doc = structuredClone(o.value);
					else {
						get(doc, o.path);
						remove(doc, o.path);
						doc = add(doc, o.path, structuredClone(o.value));
					}
					break;
				case "move": {
					const value = remove(doc, o.from);
					doc = add(doc, o.path, value);
					break;
				}
				case "copy":
					doc = add(doc, o.path, structuredClone(get(doc, o.from)));
					break;
				case "test":
					if (JSON.stringify(get(doc, o.path)) !== JSON.stringify(o.value)) {
						throw new Error(`expected ${JSON.stringify(o.value)}, found ${JSON.stringify(get(doc, o.path))}`);
					}
					break;
			}
		} catch (e) {
			throw new Error(`${where}: ${e instanceof Error ? e.message : e}`);
		}
	});
	return doc;
}

async function files(at: string, ext: string): Promise<string[]> {
	try {
		const info = await Deno.stat(at);
		if (info.isFile) return at.endsWith(".patch") === (ext === ".patch") ? [at] : [];
		const names: string[] = [];
		for await (const e of Deno.readDir(at)) if (e.isFile && e.name.endsWith(ext)) names.push(`${at}/${e.name}`);
		return names.sort();
	} catch (e) {
		if (e instanceof Deno.errors.NotFound) return [];
		throw e;
	}
}

export function applyDiff(text: string, diff: string, name = "patch"): string {
	const lines = text.split("\n");
	const hunks = diff.split("\n");
	let offset = 0;
	for (let i = 0; i < hunks.length; i++) {
		const head = hunks[i].match(/^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/);
		if (!head) continue;
		const before: string[] = [];
		const after: string[] = [];
		const [old, neu] = [Number(head[2] ?? 1), Number(head[4] ?? 1)];
		while ((before.length < old || after.length < neu) && ++i < hunks.length) {
			const line = hunks[i];
			if (line.startsWith("\\")) continue;
			if (line.startsWith("-")) before.push(line.slice(1));
			else if (line.startsWith("+")) after.push(line.slice(1));
			else {
				before.push(line.slice(1));
				after.push(line.slice(1));
			}
		}
		const start = Number(head[1]) - 1 + offset;
		const found = [start, ...Array.from({ length: lines.length }, (_, d) => [start - d - 1, start + d + 1]).flat()]
			.find((at) => at >= 0 && before.every((l, k) => lines[at + k] === l));
		if (found === undefined) throw new Error(`${name}: hunk ${head[0]} does not apply`);
		lines.splice(found, before.length, ...after);
		offset += after.length - before.length + (found - start);
	}
	return lines.join("\n");
}

export async function patchedText(raw: string | undefined, patches: string | undefined, path: string): Promise<string | undefined> {
	if (!patches) return raw;
	const found = await files(`${patches}/${path}`, ".patch");
	let text = raw;
	for (const file of found) text = applyDiff(text ?? "", await Deno.readTextFile(file), file);
	return text;
}

export async function patched(raw: Json | undefined, patches: string | undefined, path: string): Promise<Json | undefined> {
	if (!patches) return raw;
	const found = await files(`${patches}/${path}`, ".json");
	if (!found.length) return raw;
	let doc = structuredClone(raw ?? null);
	for (const file of found) doc = apply(doc, JSON.parse(await Deno.readTextFile(file)), file);
	return doc;
}

export async function hasPatch(patches: string | undefined, path: string): Promise<boolean> {
	if (!patches) return false;
	const at = `${patches}/${path}`;
	return (await files(at, ".json")).length + (await files(at, ".patch")).length > 0;
}
