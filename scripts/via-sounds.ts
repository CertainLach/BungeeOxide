import { mapping, read } from "./via-data.ts";
import { Source } from "./mcdata.ts";

const [refs, cache, patches, renames, data] = Deno.args;
if (!refs || !cache || !patches || !renames || !data) {
	throw new Error("usage: via-sounds.ts <dir with Via checkouts> <minecraft-data cache> <patches dir> <renames.rs> <registry data dir>");
}

const viaData = `${refs}/ViaVersion/common/src/main/resources/assets/viaversion/data`;
const table = read(`${viaData}/identifier-table.nbt`) as unknown as { sounds: string[] };
const strip = (n: string) => n.replace(/^minecraft:/, "");
const via = (v: string) => mapping(read(`${viaData}/identifiers-${v}.nbt`).sounds as never).map((g) => strip(table.sounds[g]));

const commit = (await Deno.readTextFile("crates/bungeeoxide-converter/minecraft-data.commit")).trim();
const source = await Source.open(cache, commit);
const vanilla = async (v: string) => ((await source.json(`pc/${v}/sounds.json`)) as { id: number; name: string }[]).map((s) => s.name);

const [via12, vanilla1122] = [via("1.12"), await vanilla("1.12.2")];
if (via12.length !== vanilla1122.length) throw new Error("ViaVersion 1.12 and minecraft-data 1.12.2 sound lists differ in length");
const era = new Map(via12.map((n, i) => [n, vanilla1122[i]]));
const normalize = (names: string[]) => names.map((n) => era.get(n) ?? n);

const [via11, vanilla112] = [normalize(via("1.11")), await vanilla("1.11.2")];
const mismatch = via11.findIndex((n, i) => n !== vanilla112[i]);
if (mismatch >= 0 || via11.length !== vanilla112.length) {
	throw new Error(`normalized ViaVersion 1.11 differs from minecraft-data 1.11.2 at ${mismatch}: ${via11[mismatch]} / ${vanilla112[mismatch]}`);
}
const via94 = via("1.9.4");
const vanilla194 = await vanilla("1.9.4");
if (JSON.stringify(via94) !== JSON.stringify(vanilla194)) throw new Error("ViaVersion 1.9.4 differs from minecraft-data 1.9.4");

const write = async (path: string, ops: unknown[]) => {
	await Deno.mkdir(path.slice(0, path.lastIndexOf("/")), { recursive: true });
	await Deno.writeTextFile(path, JSON.stringify(ops, null, "\t") + "\n");
};
const lists: [string, string[]][] = [["1.8", via("1.8")], ["1.9", via("1.9")], ["1.10", normalize(via("1.10"))]];
for (const [v, names] of lists) {
	await write(`${patches}/pc/${v}/sounds.json/rewind.json`, [{ op: "add", path: "", value: names.map((name, id) => ({ id, name })) }]);
}
const paths: [string, string][] = [
	["1.8", "pc/1.8"], ["1.9", "pc/1.9"], ["1.9.2", "pc/1.9"], ["1.10", "pc/1.10"],
	["1.11", "pc/1.11.2"], ["1.12", "pc/1.12.2"], ["1.12.1", "pc/1.12.2"],
];
await write(`${patches}/dataPaths.json/rewind.json`, paths.map(([v, p]) => ({ op: "add", path: `/pc/${v}/sounds`, value: p })));

const down = read(`${refs}/ViaRewind/common/src/main/resources/assets/viarewind/data/mappings-1.9to1.8.nbt`).sounds as never;
const [names19, names18] = [via("1.9"), via("1.8")];
const entries: [string, string, string][] = [];
mapping(down).forEach((to, from) => {
	const [a, b] = [names19[from], names18[to]];
	if (a && b && to >= 0 && a !== b) entries.push(["1.8", a, b]);
});
const java = await Deno.readTextFile(`${refs}/ViaVersion/common/src/main/java/com/viaversion/viaversion/protocols/v1_8to1_9/data/SoundEffectMappings1_9.java`);
const seen = new Set<string>();
for (const [, old, neu] of java.matchAll(/^\s+[A-Z0-9_]+\("([^"]*)", "([^"]*)"/gm)) {
	if (!old || !neu || old === neu || seen.has(old)) continue;
	seen.add(old);
	entries.push(["1.9", old, neu]);
}

const backwards = `${refs}/ViaBackwards/common/src/main/resources/assets/viabackwards/data`;
const ours: Record<string, string> = { "1.21": "1.21.1", "1.21.2": "1.21.3", "1.21.7": "1.21.8" };
const rows = (await Deno.readTextFile(`${data}/versions.tsv`)).trim().split("\n").map((l) => l.split("\t"));
const order = [...new Set(rows.map(([v]) => v))];
const folder = new Map(rows.filter(([, k]) => k === "sounds").map(([v, , f]) => [v, f]));
const local = async (v: string) => {
	const f = folder.get(v);
	if (!f) return undefined;
	const lines = (await Deno.readTextFile(`${data}/${f}/sounds.txt`)).trim().split("\n").map((l) => l.split(" "));
	return lines.sort((a, b) => Number(a[0]) - Number(b[0])).map(([, n]) => n);
};
for await (const e of Deno.readDir(backwards)) {
	const m = e.name.match(/^mappings-(.+)to(.+)\.nbt$/);
	if (!m) continue;
	const sounds = read(`${backwards}/${e.name}`).sounds as never;
	if (!sounds) continue;
	const newer = ours[m[1]] ?? m[1];
	const at = order.indexOf(newer);
	if (at < 1) continue;
	const older = order[at - 1];
	const [a, b] = [await local(newer), await local(older)];
	const map = mapping(sounds);
	const size = (sounds as { size: number }).size, mapped = (sounds as { mappedSize: number }).mappedSize;
	if (!a || !b || a.length !== size || b.length !== mapped) {
		console.error(`skipping ${e.name}: ours ${newer} ${a?.length} / ${older} ${b?.length}, Via ${size} / ${mapped}`);
		continue;
	}
	const known = new Set(b);
	map.forEach((to, from) => {
		if (to < 0 || known.has(a[from]) || a[from] === b[to]) return;
		entries.push([older, a[from], b[to]]);
	});
}

const unique = [...new Map(entries.map((e) => [`${e[0]} ${e[1]}`, e])).values()];
const rs = (await Deno.readTextFile(renames)).split("\n");
const variant = (v: string) => `Version::V${v.replaceAll(".", "_")}`;
const generated = new Set(unique.map(([v, a, b]) => `(${variant(v)}, "${a}", "${b}"),`));
await Deno.writeTextFile(renames, rs.filter((l) => !generated.has(l.trim())).join("\n"));
const out = `use crate::gen::convert::Version;\n\npub const SOUNDS: &[(Version, &str, &str)] = &[\n${unique.map(([v, a, b]) => `\t(${variant(v)}, "${a}", "${b}"),`).join("\n")}\n];\n`;
await Deno.writeTextFile(renames.replace(/renames\.rs$/, "sounds.rs"), out);
console.log(`sound lists ${lists.map(([v, n]) => `${v}:${n.length}`).join(" ")}, ${unique.length} sound renames`);
