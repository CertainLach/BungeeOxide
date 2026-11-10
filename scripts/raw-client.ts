import mc from "npm:minecraft-protocol@1";

const [address = "127.0.0.1:25577", version = "1.7.10", seconds = "10", ...lines] = Deno.args;
const [host, port] = address.split(":");
const report: Record<string, unknown> = { version };
const kinds: Record<string, number> = {};
const client = mc.createClient({
	host,
	port: Number(port),
	username: `raw_${version.replaceAll(".", "_")}`.slice(0, 16),
	version,
	auth: "offline",
});

const done = (code: number) => {
	report.packets = kinds;
	console.log(JSON.stringify(report));
	Deno.exit(code);
};

client.on("packet", (data: Record<string, unknown>, meta: { name: string; state: string }) => {
	kinds[meta.name] = (kinds[meta.name] ?? 0) + 1;
	if (meta.name === "chat") ((report.chat ??= [] as unknown[]) as unknown[]).push(data.message);
	if (meta.name === "login" || meta.name === "respawn") {
		((report.worlds ??= [] as unknown[]) as unknown[]).push(`${meta.name} ${data.dimension}`);
	}
	if (meta.name === "position" && !report.position) {
		report.position = [data.x, data.y, data.z];
		client.write("chat", { message: "/proxy-ping" });
		client.write("chat", { message: `hello from ${version}` });
		lines.forEach((line, i) => setTimeout(() => client.write("chat", { message: line }), 5000 * (i + 1)));
	}
});
client.on("login", () => (report.login = true));
client.on("error", (e: Error) => {
	report.error = String(e).slice(0, 300);
	done(1);
});
client.on("end", (why: string) => {
	report.end = why;
	done(report.alive ? 0 : 1);
});
setTimeout(() => {
	report.alive = true;
	client.end("done");
}, Number(seconds) * 1000);
