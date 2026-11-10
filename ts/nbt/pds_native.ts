import { Input, type Native, Output, PdsError } from "./pds.ts";

export { pds_f32, pds_f64, pds_i16, pds_i32, pds_i64, pds_i8, pds_u8, pds_void } from "./pds.ts";

export function mutf8_encode(s: string): Uint8Array {
	const out: number[] = [];
	for (let i = 0; i < s.length; i++) {
		const unit = s.charCodeAt(i);
		if (unit >= 0x0001 && unit <= 0x007f) {
			out.push(unit);
		} else if (unit <= 0x07ff) {
			out.push(0xc0 | (unit >> 6), 0x80 | (unit & 0x3f));
		} else {
			out.push(0xe0 | (unit >> 12), 0x80 | ((unit >> 6) & 0x3f), 0x80 | (unit & 0x3f));
		}
	}
	return Uint8Array.from(out);
}

export function mutf8_decode(b: Uint8Array): string {
	const cont = (i: number): number => {
		if (i >= b.length || (b[i] & 0xc0) !== 0x80) {
			throw new PdsError("InvalidValue", "invalid modified utf-8");
		}
		return b[i] & 0x3f;
	};
	const units: number[] = [];
	let i = 0;
	while (i < b.length) {
		const c = b[i];
		if ((c & 0x80) === 0) {
			units.push(c);
			i += 1;
		} else if ((c & 0xe0) === 0xc0) {
			units.push(((c & 0x1f) << 6) | cont(i + 1));
			i += 2;
		} else if ((c & 0xf0) === 0xe0) {
			units.push(((c & 0x0f) << 12) | (cont(i + 1) << 6) | cont(i + 2));
			i += 3;
		} else {
			throw new PdsError("InvalidValue", "invalid modified utf-8");
		}
	}
	let s = "";
	for (let j = 0; j < units.length; j += 0x2000) {
		s += String.fromCharCode(...units.slice(j, j + 0x2000));
	}
	if (!s.isWellFormed()) {
		throw new PdsError("InvalidValue", "unpaired surrogate");
	}
	return s;
}

export const pds_string: Native<string> = {
	serialize(v: string, output: Output): void {
		const bytes = mutf8_encode(v);
		if (bytes.length > 0xffff) {
			throw new PdsError("InvalidValue", "string too long");
		}
		output.write([bytes.length >> 8, bytes.length & 0xff]);
		output.write(bytes);
	},
	deserialize(input: Input): string {
		const len = (input.read() << 8) | input.read();
		return mutf8_decode(input.read_bytes(len));
	},
	size_of(v: string): number {
		return 2 + mutf8_encode(v).length;
	},
};
