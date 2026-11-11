import { Source } from "./mcdata.ts";

const [mcdata, out, patches] = Deno.args;
if (!mcdata || !out) throw new Error("usage: minecraft-registries.ts <minecraft-data cache> <out dir> [patches dir]");

const kinds = ["blocks", "items", "entities", "particles", "biomes", "effects", "enchantments", "sounds", "loginPacket", "tags"];
const json = new Set(["loginPacket", "tags"]);
const commit = (await Deno.readTextFile(`${mcdata}/commit`)).trim();
const source = await Source.open(mcdata, commit, patches);
const dataPaths = ((await source.json("dataPaths.json")) as { pc: Record<string, Record<string, string>> }).pc;
const releases = (await Deno.readTextFile(`${mcdata}/releases.tsv`)).trim().split("\n").map((l) => l.split("\t")[0]);

async function fetchJson(path: string): Promise<unknown> {
	return await source.json(path);
}

type State = { name: string; type: string; num_values: number; values?: string[] };
type Block = { id: number; name: string; minStateId?: number; maxStateId?: number; defaultState?: number; states?: State[] };

function blocks(list: Block[]): string {
	return list
		.map((b) => {
			if (b.minStateId === undefined) return `${b.id} ${b.name}`;
			const props = (b.states ?? []).map((s) => {
				const values = s.type === "bool" ? ["true", "false"] : s.values ?? [...Array(s.num_values).keys()].map(String);
				return `${s.name}=${values.join(",")}`;
			});
			return [b.id, b.name, b.minStateId, b.defaultState, ...props].join(" ");
		})
		.join("\n");
}

type Named = { id: number; name: string; type?: string; metadataKeys?: string[] };
const named = (list: Named[]) => list.map((x) => `${x.id} ${x.name}`).join("\n");
const entities = (list: Named[]) =>
	list.map((x) => [x.id, x.name, x.type ?? "-", ...(x.metadataKeys ?? [])].join(" ")).join("\n");

const versions: string[] = [];
const written = new Set<string>();
for (const version of releases) {
	for (const kind of kinds) {
		const folder = dataPaths[version]?.[kind];
		if (!folder) continue;
		versions.push(`${version}\t${kind}\t${folder.replace(/^pc\//, "")}`);
		const target = `${out}/${folder.replace(/^pc\//, "")}/${kind}.${json.has(kind) ? "json" : "txt"}`;
		if (written.has(target)) continue;
		written.add(target);
		const data = await fetchJson(`${folder}/${kind}.json`);
		await Deno.mkdir(target.slice(0, target.lastIndexOf("/")), { recursive: true });
		const text =
			kind === "blocks"
				? blocks(data as Block[])
				: kind === "entities"
				? entities(data as Named[])
				: json.has(kind)
				? JSON.stringify(data)
				: named(data as Named[]);
		await Deno.writeTextFile(target, text + "\n");
	}
}
await Deno.writeTextFile(`${out}/versions.tsv`, versions.join("\n") + "\n");

type Protocol = { minecraftVersion: string; dataVersion?: number };
const protocols = (await fetchJson("pc/common/protocolVersions.json")) as Protocol[];
const dataVersions = releases.flatMap((version) => {
	const found = protocols.find((p) => p.minecraftVersion === version && p.dataVersion !== undefined);
	return found ? [`${version}\t${found.dataVersion}`] : [];
});
await Deno.writeTextFile(`${out}/data-versions.tsv`, dataVersions.join("\n") + "\n");

const legacy = (await fetchJson("pc/common/legacy.json")) as { blocks: Record<string, string>; items: Record<string, string> };
const legacyLines = [
	...Object.entries(legacy.blocks).map(([k, v]) => `block ${k} ${v}`),
	...Object.entries(legacy.items).map(([k, v]) => `item ${k} ${v}`),
];
await Deno.writeTextFile(`${out}/legacy.txt`, legacyLines.join("\n") + "\n");

const files = [...written].map((f) => f.slice(out.length + 1)).concat(["data-versions.tsv", "legacy.txt", "versions.tsv"]).sort();
const rust = [
	"pub const FILES: &[(&str, &str)] = &[",
	...files.map((f) => `\t(${JSON.stringify(f)}, include_str!(${JSON.stringify(`../../data/${f}`)})),`),
	"];",
	"",
].join("\n");
await Deno.writeTextFile(`${out}/../src/registry/files.rs`, rust);
