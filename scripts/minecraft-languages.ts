import { Source } from "./mcdata.ts";

const args = [...Deno.args];
const option = (name: string) => {
	const i = args.indexOf(name);
	return i < 0 ? undefined : args.splice(i, 2)[1];
};
const commit = option("--commit");
const patches = option("--patches");
const [mcdata, out, ...versions] = args;
if (!mcdata || !out || !commit) {
	throw new Error("usage: minecraft-languages.ts <minecraft-data cache> <out dir> --commit <sha> [--patches <dir>] <version>...");
}

const source = await Source.open(mcdata, commit, patches);
const dataPaths = ((await source.json("dataPaths.json")) as { pc: Record<string, Record<string, string>> }).pc;

await Deno.mkdir(out, { recursive: true });
const index: string[] = [];
const written = new Set<string>();
for (const version of versions) {
	const folder = dataPaths[version]?.language;
	if (!folder) continue;
	const name = folder.replace(/^pc\//, "");
	index.push(`${version}\t${name}`);
	if (written.has(name)) continue;
	written.add(name);
	const strings = (await source.json(`${folder}/language.json`)) as Record<string, string>;
	await Deno.writeTextFile(`${out}/${name}.json`, JSON.stringify(strings));
}
for await (const e of Deno.readDir(out)) {
	if (e.isFile && e.name.endsWith(".json") && !written.has(e.name.slice(0, -".json".length))) {
		await Deno.remove(`${out}/${e.name}`);
	}
}
await Deno.writeTextFile(`${out}/index.tsv`, index.join("\n") + "\n");
await Deno.writeTextFile(`${out}/commit`, `${commit}\n`);
console.error(`${written.size} languages for ${index.length} versions in ${out}`);
