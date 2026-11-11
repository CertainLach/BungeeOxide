const [manifest, dir] = Deno.args;
if (!manifest || !dir) {
	throw new Error("usage: workspace-crates.ts <workspace Cargo.toml> <dir of generated crates>");
}

const root = (await Deno.realPath(manifest)).replace(/\/[^/]+$/, "");
const crates = await Deno.realPath(dir);
if (!crates.startsWith(`${root}/`)) throw new Error(`${dir} is outside the workspace of ${manifest}`);
const rel = crates.slice(root.length + 1);

const versions = new Map<string, string>();
for await (const entry of Deno.readDir(crates)) {
	if (!entry.isDirectory) continue;
	const text = await Deno.readTextFile(`${crates}/${entry.name}/Cargo.toml`).catch(() => null);
	if (text === null) continue;
	const version = text.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
	if (!version) throw new Error(`${entry.name}/Cargo.toml has no version`);
	versions.set(entry.name, version);
}
const lines = (await Deno.readTextFile(manifest)).split("\n");
const section = (name: string) => {
	const start = lines.findIndex((l) => l.trim() === `[${name}]`);
	if (start < 0) throw new Error(`${manifest} has no [${name}]`);
	const end = lines.findIndex((l, i) => i > start && l.trim().startsWith("["));
	return [start, end < 0 ? lines.length : end];
};
const generated = [...versions.keys()].sort().map((n) => `${n} = { version = "${versions.get(n)}", path = "${rel}/${n}" }`);

const [header, end] = section("workspace.dependencies");
const deps = lines.slice(header + 1, end);
const ours = (l: string) => l.includes(`path = "${rel}/`);
const first = deps.findIndex(ours);
const kept = deps.filter((l) => !ours(l));
const at = first < 0 ? 0 : deps.slice(0, first).filter((l) => !ours(l)).length;
kept.splice(at, 0, ...generated);
lines.splice(header + 1, end - header - 1, ...kept);
await Deno.writeTextFile(manifest, lines.join("\n"));
console.log(`${manifest}: ${generated.length} workspace dependencies from ${rel}`);
