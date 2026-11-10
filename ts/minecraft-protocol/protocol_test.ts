import { assertEquals } from "@std/assert";
import { Input, Output } from "./pds.ts";
import { lpVec3_to_f64, pds_anonOptionalNbt, pds_anonymousNbt, pds_lpVec3 } from "./pds_native.ts";
import * as gen from "./gen/mod.ts";

type Codec<T> = {
	deserialize(input: Input): T;
	serialize(v: T, output: Output): void;
	size_of(v: T): number;
};

function roundtrip<T>(codec: Codec<T>, bytes: ArrayLike<number>): T {
	const data = Uint8Array.from(bytes);
	const input = new Input(data);
	const v = codec.deserialize(input);
	assertEquals(input.offset, data.length);
	const out = new Output();
	codec.serialize(v, out);
	assertEquals(out.into_inner(), data);
	assertEquals(codec.size_of(v), data.length);
	return v;
}

function encode<T>(codec: Codec<T>, v: T): Uint8Array {
	const out = new Output();
	codec.serialize(v, out);
	return out.into_inner();
}

const hex = (s: string) => s.match(/../g)!.map((b) => parseInt(b, 16));

Deno.test("handshake", () => {
	const t = gen.pds_handshaking_toServer_packet_set_protocol;
	const v = { protocolVersion: 774, serverHost: "localhost", serverPort: 25565, nextState: 2 };
	const bytes = encode(t, v);
	assertEquals(bytes.length, t.size_of(v));
	assertEquals(roundtrip(t, bytes), v);
});

Deno.test("position bitfield", () => {
	const t = gen.pds_position;
	const v = { x: -123456, z: 654321, y: -42 };
	const bytes = encode(t, v);
	assertEquals(bytes.length, 8);
	assertEquals(roundtrip(t, bytes), v);
});

Deno.test("inline tag mapper", () => {
	const t = gen.pds_soundSource;
	assertEquals([...encode(t, { kind: "hostile" })], [5]);
	assertEquals(roundtrip(t, [5]), { kind: "hostile" });
});

Deno.test("merged mapper switch", () => {
	const t = gen.pds_SlotComponent_30;
	assertEquals([...encode(t, { kind: "hidden" })], [1]);
	assertEquals(roundtrip(t, [1]), { kind: "hidden" });
});

Deno.test("scalar tag merged into union", () => {
	const t = gen.pds_play_toClient_packet_hide_message;
	const v = { kind: "v0" as const, value: new Uint8Array(256).fill(7) };
	const bytes = encode(t, v);
	assertEquals([bytes[0], bytes.length], [0, 257]);
	assertEquals(roundtrip(t, bytes), v);
	assertEquals(roundtrip(t, [5]), { kind: "default", tag: 5 });
});

Deno.test("option tag derived from union", () => {
	const t = gen.pds_play_toClient_packet_scoreboard_score;
	const end = { type: 0, value: { kind: "end" as const } };
	const v = {
		itemName: "a",
		scoreName: "b",
		value: 3,
		display_name: null,
		styling: { kind: "v1" as const, value: end },
	};
	const bytes = encode(t, v);
	assertEquals([...bytes], [1, 97, 1, 98, 3, 0, 1, 1, 0]);
	assertEquals(roundtrip(t, bytes), v);
	const none = { ...v, styling: { kind: "default" as const, tag: null } };
	assertEquals([...encode(t, none)], [1, 97, 1, 98, 3, 0, 0]);
});

Deno.test("top bit set terminated equipment", () => {
	const t = gen.pds_play_toClient_packet_entity_equipment;
	const v = roundtrip(t, [7, 0x80, 0, 0x85, 0, 3, 0]);
	assertEquals(v.equipments.map((e) => e.slot), [0, 5, 3]);
});

Deno.test("entity metadata loop", () => {
	const t = gen.pds_play_toClient_packet_entity_metadata;
	const v = roundtrip(t, [1, 0, 0, 7, 0xff]);
	assertEquals(v.metadata.length, 1);
	assertEquals(v.metadata[0].key, 0);
});

Deno.test("lpVec3 real server payloads", () => {
	for (const h of ["00", "f9ff7ffeebed", "59e7800cebed", "51e880011541", "09e98000d8fd"]) {
		roundtrip(pds_lpVec3, hex(h));
	}
	const [x, y, z] = lpVec3_to_f64(roundtrip(pds_lpVec3, hex("f9ff7ffeebed")));
	assertEquals(Math.abs(y + 0.0784) < 0.001, true);
	assertEquals(Math.abs(x) < 1 && Math.abs(z) < 1, true);
});

Deno.test("nbt text component", () => {
	const t = (s: string) => [...s].map((c) => c.charCodeAt(0));
	const v = roundtrip(pds_anonymousNbt, [
		10,
		...[8, 0, 4, ...t("text"), 0, 2, ...t("hi")],
		0,
	]);
	if (v.value.kind !== "compound") throw new Error("not a compound");
	assertEquals(v.value.value[0].name, "text");
	assertEquals(roundtrip(pds_anonOptionalNbt, [0]).value, { kind: "end" });
});
