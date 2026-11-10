import { startProxy } from "./proxy.ts";

function addr(s: string): { hostname: string; port: number } {
	const i = s.lastIndexOf(":");
	const port = Number(s.slice(i + 1));
	if (i < 0 || !Number.isInteger(port)) {
		throw new Error(`expected host:port, got ${s}`);
	}
	return { hostname: s.slice(0, i), port };
}

if (import.meta.main) {
	const flags = Deno.args.filter((a) => a.startsWith("--"));
	const positional = Deno.args.filter((a) => !a.startsWith("--"));
	const unknown = flags.filter((f) => f !== "--full" && f !== "--failures");
	if (positional.length !== 2 || unknown.length > 0) {
		console.error("usage: main.ts [--full] [--failures] <listen addr> <upstream addr>");
		Deno.exit(2);
	}
	const [listen, upstream] = positional;
	const { done } = startProxy({
		listen: addr(listen),
		upstream: addr(upstream),
		full: flags.includes("--full"),
		failuresOnly: flags.includes("--failures"),
		log: (line) => console.log(line),
	});
	console.log(`listening on ${listen}, forwarding to ${upstream}`);
	await done;
}
