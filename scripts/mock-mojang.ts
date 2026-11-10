import { encodeBase64 } from "jsr:@std/encoding@1/base64";

const [port = "25661", ...specs] = Deno.args;
if (specs.length === 0) {
	throw new Error(
		"usage: [MOCK_MOJANG_KEY_FILE=<file for the services key>] mock-mojang.ts <port> <name>:<uuid>:<access token>:<refresh token> ...",
	);
}

type Account = { name: string; uuid: string; tokens: Set<string>; refresh: Set<string>; keys?: Record<string, string> };
const accounts: Account[] = specs.map((spec) => {
	const [name, uuid, token, refresh] = spec.split(":");
	return { name, uuid: uuid.replaceAll("-", ""), tokens: new Set([token]), refresh: new Set(refresh ? [refresh] : []) };
});
const joins = new Map<string, string>();
const msa = new Map<string, Account>();
const xbl = new Map<string, Account>();
const xsts = new Map<string, Account>();
const log = (line: string) => console.log(`${new Date().toISOString()} ${line}`);

const services = await crypto.subtle.generateKey(
	{ name: "RSASSA-PKCS1-v1_5", modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: "SHA-1" },
	true,
	["sign", "verify"],
);
const servicesDer = new Uint8Array(await crypto.subtle.exportKey("spki", services.publicKey));
const servicesPublic = encodeBase64(servicesDer);
const keyFile = Deno.env.get("MOCK_MOJANG_KEY_FILE");
if (keyFile) await Deno.writeFile(keyFile, servicesDer);

const random = (prefix: string) => `${prefix}-${crypto.randomUUID()}`;
const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
const pem = (label: string, der: Uint8Array) => `-----BEGIN ${label}-----\n${encodeBase64(der).match(/.{1,76}/g)!.join("\n")}\n-----END ${label}-----\n`;
const bearer = (req: Request) => {
	const token = req.headers.get("authorization")?.replace(/^Bearer /, "") ?? "";
	return accounts.find((a) => a.tokens.has(token));
};
const uuidBytes = (uuid: string) => Uint8Array.from(uuid.match(/../g)!.map((h) => parseInt(h, 16)));
const longBytes = (value: number) => {
	const out = new Uint8Array(8);
	new DataView(out.buffer).setBigInt64(0, BigInt(value));
	return out;
};
const concat = (...parts: Uint8Array[]) => {
	const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
	let at = 0;
	for (const p of parts) {
		out.set(p, at);
		at += p.length;
	}
	return out;
};
const sign = async (data: Uint8Array) => encodeBase64(new Uint8Array(await crypto.subtle.sign("RSASSA-PKCS1-v1_5", services.privateKey, data)));

async function certificates(account: Account) {
	if (!account.keys) {
		const pair = await crypto.subtle.generateKey(
			{ name: "RSASSA-PKCS1-v1_5", modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: "SHA-256" },
			true,
			["sign", "verify"],
		);
		const spki = new Uint8Array(await crypto.subtle.exportKey("spki", pair.publicKey));
		const pkcs8 = new Uint8Array(await crypto.subtle.exportKey("pkcs8", pair.privateKey));
		const expires = Date.now() + 48 * 3600_000;
		const publicPem = pem("RSA PUBLIC KEY", spki);
		account.keys = {
			privateKey: pem("RSA PRIVATE KEY", pkcs8),
			publicKey: publicPem,
			publicKeySignatureV2: await sign(concat(uuidBytes(account.uuid), longBytes(expires), spki)),
			publicKeySignature: await sign(new TextEncoder().encode(`${expires}${publicPem}`)),
			expiresAt: new Date(expires).toISOString().replace("Z", "000Z"),
			refreshedAfter: new Date(expires - 8 * 3600_000).toISOString(),
		};
	}
	const { privateKey, publicKey, publicKeySignature, publicKeySignatureV2, expiresAt, refreshedAfter } = account.keys;
	return { keyPair: { privateKey, publicKey }, publicKeySignature, publicKeySignatureV2, expiresAt, refreshedAfter };
}

function minecraftLogin(identity: string) {
	const token = identity.split(";")[1] ?? "";
	const account = xsts.get(token);
	if (!account) return json({ error: "UNAUTHORIZED", errorMessage: "bad xsts token" }, 401);
	const access = random(`mc-${account.name}`);
	account.tokens.add(access);
	return json({ username: account.uuid, access_token: access, token_type: "Bearer", expires_in: 86400 });
}

Deno.serve({ port: Number(port), hostname: "127.0.0.1", onListen: () => log(`mock Mojang on 127.0.0.1:${port}`) }, async (req) => {
	const url = new URL(req.url);
	const path = url.pathname;
	log(`${req.method} ${path}`);
	if (req.method === "POST" && path === "/session/minecraft/join") {
		const body = await req.json();
		const account = accounts.find((a) => a.tokens.has(body.accessToken));
		if (!account || account.uuid !== String(body.selectedProfile).replaceAll("-", "")) {
			log(`join refused for profile ${body.selectedProfile}`);
			return json({ error: "ForbiddenOperationException", errorMessage: "Invalid token" }, 403);
		}
		joins.set(account.name, body.serverId);
		log(`join ${account.name} ${body.serverId}`);
		return new Response(null, { status: 204 });
	}
	if (req.method === "GET" && path === "/session/minecraft/hasJoined") {
		const name = url.searchParams.get("username") ?? "";
		const account = accounts.find((a) => a.name === name);
		if (!account || joins.get(name) !== url.searchParams.get("serverId")) {
			log(`hasJoined ${name}: no`);
			return new Response(null, { status: 204 });
		}
		log(`hasJoined ${name}: yes`);
		return json({ id: account.uuid, name: account.name, properties: [], profileActions: [] });
	}
	if (req.method === "GET" && path.startsWith("/session/minecraft/profile/")) {
		const account = accounts.find((a) => a.uuid === path.split("/").pop()?.replaceAll("-", ""));
		return account ? json({ id: account.uuid, name: account.name, properties: [] }) : new Response(null, { status: 204 });
	}
	if (req.method === "GET" && path === "/publickeys") {
		return json({
			profilePropertyKeys: [{ publicKey: servicesPublic }],
			playerCertificateKeys: [{ publicKey: servicesPublic }],
			authenticationKeys: [],
		});
	}
	if (req.method === "POST" && path === "/player/certificates") {
		const account = bearer(req);
		if (!account) return json({ path, error: "UNAUTHORIZED" }, 401);
		log(`certificates for ${account.name}`);
		return json(await certificates(account));
	}
	if (req.method === "GET" && path === "/minecraft/profile") {
		const account = bearer(req);
		return account ? json({ id: account.uuid, name: account.name, skins: [], capes: [] }) : json({ path, error: "UNAUTHORIZED" }, 401);
	}
	if (req.method === "POST" && path === "/minecraft/profile/lookup/bulk/byname") {
		const names: string[] = await req.json();
		return json(accounts.filter((a) => names.some((n) => n.toLowerCase() === a.name.toLowerCase())).map((a) => ({ id: a.uuid, name: a.name })));
	}
	if (req.method === "GET" && path.startsWith("/minecraft/profile/lookup/name/")) {
		const name = path.split("/").pop()!.toLowerCase();
		const account = accounts.find((a) => a.name.toLowerCase() === name);
		return account ? json({ id: account.uuid, name: account.name }) : json({ path, errorMessage: "not found" }, 404);
	}
	if (req.method === "POST" && path === "/consumers/oauth2/v2.0/token") {
		const form = new URLSearchParams(await req.text());
		const account = accounts.find((a) => a.refresh.has(form.get("refresh_token") ?? ""));
		if (form.get("grant_type") !== "refresh_token" || !account) {
			log("msa refresh refused");
			return json({ error: "invalid_grant", error_description: "bad refresh token" }, 400);
		}
		const access = random("msa");
		const refresh = random(`refresh-${account.name}`);
		msa.set(access, account);
		account.refresh.add(refresh);
		log(`msa refresh for ${account.name} with client ${form.get("client_id")}`);
		return json({ token_type: "bearer", scope: form.get("scope"), expires_in: 3600, access_token: access, refresh_token: refresh });
	}
	if (req.method === "POST" && path === "/user/authenticate") {
		const body = await req.json();
		const account = msa.get(String(body.Properties?.RpsTicket ?? "").replace(/^d=/, ""));
		if (!account) return json({ XErr: 0 }, 401);
		const token = random("xbl");
		xbl.set(token, account);
		return json({ Token: token, DisplayClaims: { xui: [{ uhs: "4242" }] } });
	}
	if (req.method === "POST" && path === "/xsts/authorize") {
		const body = await req.json();
		const account = xbl.get(body.Properties?.UserTokens?.[0] ?? "");
		if (!account || body.RelyingParty !== "rp://api.minecraftservices.com/") return json({ XErr: 2148916233 }, 401);
		const token = random("xsts");
		xsts.set(token, account);
		return json({ Token: token, DisplayClaims: { xui: [{ uhs: "4242" }] } });
	}
	if (req.method === "POST" && path === "/launcher/login") {
		const body = await req.json();
		return minecraftLogin(String(body.xtoken ?? ""));
	}
	if (req.method === "POST" && path === "/authentication/login_with_xbox") {
		const body = await req.json();
		return minecraftLogin(String(body.identityToken ?? ""));
	}
	return json({ path, error: "NOT_FOUND" }, 404);
});
