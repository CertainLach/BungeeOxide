import { latestCommit, Source } from "./mcdata.ts";

const args = [...Deno.args];
const out = args.shift();
const option = (name: string) => {
	const i = args.indexOf(name);
	return i < 0 ? undefined : args.splice(i, 2)[1];
};
const flag = (name: string) => {
	const i = args.indexOf(name);
	return i >= 0 && args.splice(i, 1).length > 0;
};
if (!out) throw new Error("usage: minecraft-data.ts <out> [--commit <sha> | --latest] [--patches <dir>] [--releases] [pc/<version>...]");
const commit = option("--commit") ?? (flag("--latest") ? await latestCommit() : undefined);
if (!commit) throw new Error("pass --commit <sha> or --latest");
const patches = option("--patches");
const releases = flag("--releases");
const only = args;

const source = await Source.open(out, commit, patches);

type Paths = Record<string, Record<string, string>>;
type Protocol = { minecraftVersion: string; version: number; usesNetty: boolean; majorVersion: string };

const dataPaths = (await source.json("dataPaths.json")) as { pc: Paths };
const protocolVersions = (await source.json("pc/common/protocolVersions.json")) as Protocol[];

const release = /^\d+(\.\d+)+$/;
const order = (a: string, b: string) => {
	const [x, y] = [a.split(".").map(Number), b.split(".").map(Number)];
	for (let i = 0; i < Math.max(x.length, y.length); i++) {
		if ((x[i] ?? -1) !== (y[i] ?? -1)) return (x[i] ?? -1) - (y[i] ?? -1);
	}
	return 0;
};

const dirs = new Set<string>();
for (const v of Object.values(dataPaths.pc)) if (v.protocol) dirs.add(v.protocol);

const releaseDirs = [...dirs]
	.map((d) => d.slice("pc/".length))
	.filter((v) => release.test(v))
	.sort(order);
const games = protocolVersions.filter((p) => p.usesNetty && release.test(p.minecraftVersion));
const table = releaseDirs.map((dir) => {
	let protocols = new Set(
		games.filter((p) => dataPaths.pc[p.minecraftVersion]?.protocol === `pc/${dir}`).map((p) => p.version),
	);
	if (protocols.size === 0) {
		const major = games.filter((p) => p.majorVersion === dir).map((p) => p.version);
		if (major.length) protocols = new Set([Math.max(...major)]);
	}
	const served = games
		.filter((p) => protocols.has(p.version))
		.map((p) => p.minecraftVersion)
		.sort(order);
	return { dir, protocols: [...protocols].sort((a, b) => a - b), served };
});

const wanted = releases ? releaseDirs.map((d) => `pc/${d}`) : only.length ? only : [...dirs];
for (const p of wanted) {
	if (!dirs.has(p)) throw new Error(`minecraft-data has no protocol ${p}`);
	await source.patchedFile(`${p}/protocol.json`);
}

if (releases) {
	const lines = await Promise.all(
		table.map(async (t) => {
			const patched = (await source.hasPatch(`pc/${t.dir}/protocol.json`)) ? "patched" : "upstream";
			return `${t.dir}\t${t.protocols.join(",")}\t${t.served.join(", ")}\t${patched}`;
		}),
	);
	await Deno.writeTextFile(`${out}/releases.tsv`, lines.join("\n") + "\n");
}
