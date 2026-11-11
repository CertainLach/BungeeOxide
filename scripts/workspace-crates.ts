const [manifest, dir] = Deno.args;
if (!manifest || !dir) {
	throw new Error("usage: workspace-crates.ts <workspace Cargo.toml> <dir of generated crates>");
}

const root = (await Deno.realPath(manifest)).replace(/\/[^/]+$/, "");
const crates = await Deno.realPath(dir);
if (!crates.startsWith(`${root}/`)) throw new Error(`${dir} is outside the workspace of ${manifest}`);
const rel = crates.slice(root.length + 1);

const names: string[] = [];
for await (const entry of Deno.readDir(crates)) {
	if (entry.isDirectory && (await Deno.stat(`${crates}/${entry.name}/Cargo.toml`).catch(() => null))) names.push(entry.name);
}
const generated = names.sort().map((n) => `${n} = { path = "${rel}/${n}" }`);

const lines = (await Deno.readTextFile(manifest)).split("\n");
const header = lines.findIndex((l) => l.trim() === "[workspace.dependencies]");
if (header < 0) throw new Error(`${manifest} has no [workspace.dependencies]`);
let end = lines.findIndex((l, i) => i > header && l.trim().startsWith("["));
if (end < 0) end = lines.length;

const section = lines.slice(header + 1, end);
const ours = (l: string) => l.includes(`path = "${rel}/`);
const first = section.findIndex(ours);
const kept = section.filter((l) => !ours(l));
const at = first < 0 ? 0 : section.slice(0, first).filter((l) => !ours(l)).length;
kept.splice(at, 0, ...generated);
lines.splice(header + 1, end - header - 1, ...kept);
await Deno.writeTextFile(manifest, lines.join("\n"));
console.log(`${manifest}: ${generated.length} workspace dependencies from ${rel}`);
