import { crypto } from "jsr:@std/crypto@1";
import { encodeHex } from "jsr:@std/encoding@1/hex";

const [version, dir, port = "25610", world = "normal", ...ops] = Deno.args;
if (!version || !dir || !["normal", "flat"].includes(world)) {
	throw new Error("usage: server.ts <minecraft version> <dir> [port] [normal|flat] [op names...]");
}

const manifest = await (await fetch("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")).json();
const entry = manifest.versions.find((v: { id: string }) => v.id === version);
if (!entry) throw new Error(`Mojang has no version ${version}`);
const meta = await (await fetch(entry.url)).json();
const server = meta.downloads?.server;
if (!server) throw new Error(`${version} has no server download`);

await Deno.mkdir(dir, { recursive: true });
const jar = `${dir}/server.jar`;
const sha1 = async (bytes: Uint8Array) => encodeHex(await crypto.subtle.digest("SHA-1", bytes));
let have: Uint8Array | undefined;
try {
	have = await Deno.readFile(jar);
} catch {
	have = undefined;
}
if (!have || (await sha1(have)) !== server.sha1) {
	const bytes = new Uint8Array(await (await fetch(server.url)).arrayBuffer());
	if ((await sha1(bytes)) !== server.sha1) throw new Error(`${version} server.jar sha1 mismatch`);
	await Deno.writeFile(jar, bytes);
}

const levelType = meta.releaseTime < "2018-07-18"
	? world.toUpperCase().replace("NORMAL", "DEFAULT")
	: `minecraft\\:${world}`;
await Deno.writeTextFile(`${dir}/eula.txt`, "eula=true\n");
await Deno.writeTextFile(
	`${dir}/server.properties`,
	[
		"online-mode=false",
		`server-port=${port}`,
		`level-type=${levelType}`,
		`level-name=world-${world}`,
		"spawn-protection=0",
		"view-distance=8",
		"simulation-distance=6",
		"enforce-secure-profile=false",
		"generate-structures=false",
		"",
	].join("\n"),
);
const offline = async (name: string) => {
	const b = new Uint8Array(await crypto.subtle.digest("MD5", new TextEncoder().encode(`OfflinePlayer:${name}`)));
	b[6] = (b[6] & 0x0f) | 0x30;
	b[8] = (b[8] & 0x3f) | 0x80;
	const h = encodeHex(b);
	return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
};
const entries = await Promise.all(ops.map(async (name) => ({ uuid: await offline(name), name, level: 4, bypassesPlayerLimit: false })));
await Deno.writeTextFile(`${dir}/ops.json`, JSON.stringify(entries, null, 2) + "\n");
await Deno.writeTextFile(`${dir}/console.in`, "");
console.log(meta.javaVersion?.majorVersion ?? 8);
