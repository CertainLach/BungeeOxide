import mineflayer from "npm:mineflayer@4";

const [address = "127.0.0.1:25577", version = "1.8.8", seconds = "15", ...lines] = Deno.args;
const [host, port] = address.split(":");
const report: Record<string, unknown> = { version };
const bot = mineflayer.createBot({
	host,
	port: Number(port),
	username: `bot_${version.replaceAll(".", "_")}`.slice(0, 16),
	version,
	auth: "offline",
});

const done = (code: number) => {
	if (!report.spawn) report.packets = kinds;
	console.log(JSON.stringify(report));
	Deno.exit(code);
};

const kinds: Record<string, number> = {};
bot._client.on("packet", (_data: unknown, meta: { name: string }) => (kinds[meta.name] = (kinds[meta.name] ?? 0) + 1));
const worlds: string[] = [];
type Spawn = { worldState?: { name?: string }; worldName?: string; dimension?: unknown };
const world = (event: string, data: Spawn) =>
	worlds.push(`${event} ${data.worldState?.name ?? data.worldName ?? JSON.stringify(data.dimension)}`);
bot._client.on("login", (data: Spawn) => world("login", data));
bot._client.on("respawn", (data: Spawn) => world("respawn", data));
bot.on("login", () => (report.login = true));
let spawned = false;
bot.on("spawn", () => {
	report.spawn = true;
	report.worlds = worlds;
	if (spawned) return;
	spawned = true;
	report.position = bot.entity.position.toString();
	setTimeout(() => {
		report.below = bot.blockAt(bot.entity.position.offset(0, -1, 0))?.name ?? null;
		report.entities = Object.values(bot.entities).map((e) => e.name ?? e.type).sort();
		bot.chat("/proxy-ping");
		bot.chat(`hello from ${version}`);
		lines.forEach((line, i) => setTimeout(() => bot.chat(line), 5000 * (i + 1)));
	}, 3000);
});
bot.on("messagestr", (m: string) => ((report.chat ??= [] as string[]) as string[]).push(m));
bot.on("kicked", (reason: unknown) => {
	report.kicked = typeof reason === "string" ? reason : JSON.stringify(reason);
	done(1);
});
bot.on("error", (e: Error) => {
	report.error = String(e);
	done(1);
});
bot.on("end", (why: string) => {
	report.end = why;
	done(report.alive ? 0 : 1);
});
setTimeout(() => {
	report.alive = true;
	bot.quit();
	setTimeout(() => done(0), 500);
}, Number(seconds) * 1000);
