const args = [...Deno.args];
const split = args.indexOf("--");
const [dir] = args.slice(0, split);
const cmd = args.slice(split + 1);
if (!dir || split < 0 || !cmd.length) {
	throw new Error("usage: migration-scaffold.ts <migrations dir> -- <converters --check command...>");
}

type Change = { up?: string; down?: string };
type Node = {
	whole: Change;
	ops: Map<string, { kind: string; name: string; change: Change }>;
	updated: Map<string, Node>;
};

const node = (): Node => ({ whole: {}, ops: new Map(), updated: new Map() });
const steps = new Map<string, Map<string, Node>>();
const originals = new Map<string, string>();
const both = { up: "unsupported", down: "unsupported" };

function at(step: string, base: string, path: string[]): Node {
	let bases = steps.get(step);
	if (!bases) steps.set(step, (bases = new Map()));
	let n = bases.get(base);
	if (!n) bases.set(base, (n = node()));
	for (const seg of path) {
		let next = n.updated.get(seg);
		if (!next) n.updated.set(seg, (next = node()));
		n = next;
	}
	return n;
}

function op(n: Node, kind: string, name: string, change: Change): boolean {
	const key = `${kind}(${name})`;
	const existing = n.ops.get(key);
	if (!existing) {
		n.ops.set(key, { kind, name, change: { ...change } });
		return true;
	}
	const before = JSON.stringify(existing.change);
	Object.assign(existing.change, change);
	return before !== JSON.stringify(existing.change);
}

function whole(n: Node, change: Change): boolean {
	const before = JSON.stringify(n.whole);
	Object.assign(n.whole, change);
	return before !== JSON.stringify(n.whole);
}

function remove(step: string, path: string): boolean {
	const [base, ...segs] = path.split("/");
	const bases = steps.get(step);
	if (!bases?.has(base)) return false;
	if (!segs.length) return bases.delete(base);
	let n = bases.get(base)!;
	for (const seg of segs.slice(0, -1)) {
		const next = n.updated.get(seg);
		if (!next) return false;
		n = next;
	}
	const last = segs[segs.length - 1];
	return /^\w+\(\w+\)$/.test(last) ? n.ops.delete(last) : n.updated.delete(last);
}

function absorb(line: string): "changed" | "same" | "unknown" {
	const parts = line.split(": ");
	if (parts.length < 2 || !/^[\d.]+ -> [\d.]+$/.test(parts[0])) return "same";
	const [step, where, ...rest] = parts;
	const unused = where.match(/^rule (.*) is not used$/);
	if (unused) return remove(step, unused[1]) ? "changed" : "unknown";
	const msg = rest.join(": ");
	const [base, ...segs] = where.split("/");
	const parent = segs.slice(0, -1);
	const last = segs[segs.length - 1];
	const listed = parent[parent.length - 1] === "items";
	const packets = base.endsWith("::packet") && segs.length === 1;
	const loose = packets || listed ? "drop" : "unsupported";
	let m: RegExpMatchArray | null;
	let changed: boolean;
	if ((m = msg.match(/^only in the newer version, add `added\((\w+), down: drop\)`/))) {
		changed = op(at(step, base, parent), "added", m[1], { down: loose });
	} else if ((m = msg.match(/^only in the older version, add `removed\((\w+), up: drop\)`/))) {
		changed = op(at(step, base, parent), "removed", m[1], { up: loose });
	} else if ((m = msg.match(/^only in the newer version, add `added\((\w+)(, up: \.\.\.)?\)`/))) {
		changed = op(at(step, base, parent), "added", m[1], { up: "unsupported" });
	} else if ((m = msg.match(/^only in the older version, add `removed\((\w+)(, down: \.\.\.)?\)`/))) {
		changed = op(at(step, base, parent), "removed", m[1], { down: "unsupported" });
	} else if (/ became .*, mention it with `converted`$/.test(msg) && last) {
		changed = op(at(step, base, parent), "converted", last, {});
	} else if (/^payload appeared or disappeared/.test(msg) && last) {
		changed = op(at(step, base, parent), "converted", last, both);
	} else if (/can't be converted automatically|the default arm|the tag type changed|became .*, mention it/.test(msg)) {
		changed = whole(at(step, base, segs), both);
	} else {
		return "unknown";
	}
	return changed ? "changed" : "same";
}

function render(n: Node, name: string, indent: string): string {
	const args = [name, ...Object.entries(n.whole).map(([k, v]) => `${k}: ${v}`)].join(", ");
	const inner: string[] = [];
	for (const o of n.ops.values()) {
		const a = [o.name, ...Object.entries(o.change).map(([k, v]) => `${k}: ${v}`)].join(", ");
		inner.push(`${indent}\t${o.kind}(${a});`);
	}
	for (const [seg, child] of n.updated) inner.push(render(child, seg, indent + "\t"));
	return inner.length ? `${indent}updated(${args}) {\n${inner.join("\n")}\n${indent}};` : `${indent}updated(${args});`;
}

async function write() {
	for (const [step, bases] of steps) {
		const file = `${dir}/${step.replace(" -> ", "-")}.pds`;
		if (!originals.has(file)) originals.set(file, await Deno.readTextFile(file).catch(() => ""));
		const blocks: string[] = [];
		for (const [base, n] of bases) {
			const path = base.split("::");
			const name = path.pop()!;
			let text = render(n, name, "\t".repeat(path.length));
			for (let i = path.length - 1; i >= 0; i--) {
				text = `${"\t".repeat(i)}namespace(${path[i]}) {\n${text}\n${"\t".repeat(i)}};`;
			}
			blocks.push(text);
		}
		const old = originals.get(file)!.trimEnd();
		await Deno.writeTextFile(file, [old, ...blocks].filter((b) => b).join("\n\n") + "\n");
	}
}

for (let round = 1; round <= 60; round++) {
	const out = await new Deno.Command(cmd[0], { args: cmd.slice(1), stdout: "piped", stderr: "piped", clearEnv: true }).output();
	if (out.success) Deno.exit(0);
	const lines = (new TextDecoder().decode(out.stdout) + new TextDecoder().decode(out.stderr)).split("\n");
	const results = lines.map((l) => [l, absorb(l)] as const);
	const unknown = results.filter(([, r]) => r === "unknown").map(([l]) => l);
	if (!results.some(([, r]) => r === "changed")) {
		for (const l of unknown.length ? unknown : lines) console.log(l);
		Deno.exit(1);
	}
	await write();
	console.error(`round ${round}: ${results.filter(([, r]) => r === "changed").length} rules touched`);
}
Deno.exit(1);
