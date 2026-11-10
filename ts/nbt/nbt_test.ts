import { assertEquals, assertThrows } from "@std/assert";
import { Input, Output } from "./pds.ts";
import { mutf8_decode, mutf8_encode } from "./pds_native.ts";
import * as pds_Tag from "./gen/pds_Tag.ts";
import type { Tag } from "./mod.ts";

function roundtrip(bytes: number[]): Tag {
	const data = Uint8Array.from(bytes);
	const input = new Input(data);
	const tag = pds_Tag.deserialize(input);
	assertEquals(input.offset, data.length);
	const out = new Output();
	pds_Tag.serialize(tag, out);
	assertEquals(out.into_inner(), data);
	assertEquals(pds_Tag.size_of(tag), data.length);
	return tag;
}

const t = (s: string) => [...s].map((c) => c.charCodeAt(0));

Deno.test("compound with nested list and int array", () => {
	const tag = roundtrip([
		10,
		...[8, 0, 4, ...t("text"), 0, 2, ...t("hi")],
		...[9, 0, 5, ...t("extra"), 10, 0, 0, 0, 1, 0],
		...[11, 0, 1, ...t("n"), 0, 0, 0, 2, 0, 0, 0, 1, 0xff, 0xff, 0xff, 0xff],
		0,
	]);
	if (tag.value.kind !== "compound") throw new Error("not a compound");
	const [text, extra, ints] = tag.value.value;
	assertEquals(text.name, "text");
	assertEquals(text.value, { kind: "string", value: "hi" });
	assertEquals(extra.name, "extra");
	if (extra.value.kind !== "list") throw new Error("not a list");
	assertEquals(extra.value.value.items, { kind: "compound", value: [[]] });
	assertEquals(ints.value, { kind: "intArray", value: Int32Array.from([1, -1]) });
});

Deno.test("empty root is end", () => {
	assertEquals(roundtrip([0]).value, { kind: "end" });
});

Deno.test("typed arrays for numeric lists", () => {
	const tag = roundtrip([9, 6, 0, 0, 0, 1, 0x3f, 0xf0, 0, 0, 0, 0, 0, 0]);
	if (tag.value.kind !== "list") throw new Error("not a list");
	assertEquals(tag.value.value.items, { kind: "double", value: Float64Array.from([1]) });
});

Deno.test("modified utf-8", () => {
	const s = "a\u0000é\u{1f600}";
	const bytes = mutf8_encode(s);
	assertEquals([...bytes], [0x61, 0xc0, 0x80, 0xc3, 0xa9, 0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80]);
	assertEquals(mutf8_decode(bytes), s);
	assertThrows(() => mutf8_decode(Uint8Array.from([0xed, 0xa0, 0xbd])));
});
