import mineflayer from "npm:mineflayer@4";

const [address, version, seconds = "20", spam = "0"] = Deno.args;
if (!address || !version) {
	throw new Error("usage: online-bot.ts <bridge host:port> <client version> [seconds] [extra chat lines]");
}
const [host, port] = address.split(":");
const username = `bot_${version.replaceAll(".", "_")}`.slice(0, 16);
const report: Record<string, unknown> = { version, username };
const bot = mineflayer.createBot({ host, port: Number(port), username, version, auth: "offline" });

const done = (code: number) => {
	console.log(JSON.stringify(report));
	Deno.exit(code);
};

bot.on("login", () => (report.login = bot.username));
bot.on("spawn", () => {
	report.spawn = true;
	const lines = [
		`hello from ${version}`,
		`/say said by ${version}`,
		`/me waves from ${version}`,
		`/msg ${bot.username} whisper from ${version}`,
		`/tell ${bot.username} tell from ${version}`,
		`/execute run say exec from ${version}`,
		"/proxy-ping",
		`second message from ${version}`,
		...Array.from({ length: Number(spam) }, (_, i) => (i % 3 === 2 ? `/say spam ${i}` : `spam ${i}`)),
	];
	const gap = Number(spam) > 0 ? 1100 : 700;
	lines.forEach((line, i) => setTimeout(() => bot.chat(line), 2000 + i * gap));
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
