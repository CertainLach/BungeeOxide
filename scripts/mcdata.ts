import { hasPatch, patched, patchedText } from "./json-patch.ts";

const repo = "PrismarineJS/minecraft-data";

async function exists(path: string): Promise<boolean> {
	try {
		await Deno.stat(path);
		return true;
	} catch {
		return false;
	}
}

export async function latestCommit(): Promise<string> {
	const res = await fetch(`https://api.github.com/repos/${repo}/commits/master`, {
		headers: { accept: "application/vnd.github.sha" },
	});
	if (!res.ok) throw new Error(`can't resolve ${repo} master: ${res.status}`);
	return (await res.text()).trim();
}

export type Pin = { prefix: string; commit: string };

export async function readPins(file: string | undefined): Promise<Pin[]> {
	if (!file) return [];
	return (await Deno.readTextFile(file))
		.split("\n")
		.filter((l) => l.trim())
		.map((l) => {
			const [prefix, commit] = l.split("\t");
			if (!prefix || !/^[0-9a-f]{40}$/.test(commit ?? "")) throw new Error(`${file}: bad pin ${JSON.stringify(l)}`);
			return { prefix, commit };
		});
}

export class Source {
	constructor(readonly cache: string, readonly commit: string, readonly patches?: string, readonly pins: Pin[] = []) {}

	static async open(cache: string, commit: string, patches?: string, pins: Pin[] = []): Promise<Source> {
		await Deno.mkdir(cache, { recursive: true });
		const previous = (await exists(`${cache}/commit`)) ? (await Deno.readTextFile(`${cache}/commit`)).trim() : undefined;
		if (previous !== commit) {
			for (const stale of ["pc", "common", "dataPaths.json", "protocolVersions.json", "releases.tsv", "patched"]) {
				await Deno.remove(`${cache}/${stale}`, { recursive: true }).catch(() => {});
			}
			await Deno.writeTextFile(`${cache}/commit`, `${commit}\n`);
			if (previous) console.error(`minecraft-data moved ${previous} -> ${commit}`);
		}
		return new Source(cache, commit, patches, pins);
	}

	async rawText(path: string): Promise<string | undefined> {
		const pin = this.pins.find((p) => path.startsWith(`${p.prefix}/`));
		const commit = pin?.commit ?? this.commit;
		const file = pin ? `${this.cache}/pinned/${commit}/${path}` : `${this.cache}/${path}`;
		if (!(await exists(file))) {
			const res = await fetch(`https://raw.githubusercontent.com/${repo}/${commit}/data/${path}`);
			if (res.status === 404) {
				await res.body?.cancel();
				return undefined;
			}
			if (!res.ok) throw new Error(`${path}: ${res.status}`);
			await Deno.mkdir(file.slice(0, file.lastIndexOf("/")), { recursive: true });
			await Deno.writeTextFile(file, await res.text());
			console.error(pin ? `fetched ${path} at ${commit}` : `fetched ${path}`);
		}
		return await Deno.readTextFile(file);
	}

	async text(path: string): Promise<string> {
		const text = await patchedText(await this.rawText(path), this.patches, path);
		if (text === undefined) throw new Error(`minecraft-data has no ${path} and no patch creates it`);
		return text;
	}

	async json(path: string): Promise<unknown> {
		const text = await patchedText(await this.rawText(path), this.patches, path);
		const value = await patched(text === undefined ? undefined : JSON.parse(text), this.patches, path);
		if (value === undefined) throw new Error(`minecraft-data has no ${path} and no patch creates it`);
		return value;
	}

	async patchedFile(path: string): Promise<string> {
		const file = `${this.cache}/patched/${path}`;
		await Deno.mkdir(file.slice(0, file.lastIndexOf("/")), { recursive: true });
		await Deno.writeTextFile(file, JSON.stringify(await this.json(path), null, 2));
		return file;
	}

	async hasPatch(path: string): Promise<boolean> {
		return await hasPatch(this.patches, path);
	}
}
