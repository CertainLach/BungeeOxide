import { Input, type Native, Output, PdsError } from "./mod.ts";

export const VARINT = 0;
export const I64 = 1;
export const LEN = 2;
export const I32 = 5;

export function readVarint(input: Input): bigint {
	let out = 0n;
	for (let i = 0; i < 10; i++) {
		const b = input.read();
		out |= BigInt(b & 0x7f) << BigInt(7 * i);
		if ((b & 0x80) === 0) {
			return BigInt.asUintN(64, out);
		}
	}
	throw new PdsError("TooLargeInteger");
}

export function writeVarint(output: Output, value: bigint): void {
	let v = BigInt.asUintN(64, value);
	while (v >= 0x80n) {
		output.write_byte(Number(v & 0x7fn) | 0x80);
		v >>= 7n;
	}
	output.write_byte(Number(v));
}

export function varintSize(value: bigint): number {
	let v = BigInt.asUintN(64, value);
	let n = 1;
	while (v >= 0x80n) {
		v >>= 7n;
		n++;
	}
	return n;
}

export function readKey(input: Input): [number, number] {
	const key = readVarint(input);
	return [Number(key >> 3n), Number(key & 7n)];
}

export function writeKey(output: Output, number: number, wire: number): void {
	writeVarint(output, (BigInt(number) << 3n) | BigInt(wire));
}

export function keySize(number: number): number {
	return varintSize(BigInt(number) << 3n);
}

export function expect(wire: number, expected: number): void {
	if (wire !== expected) {
		throw new PdsError("InvalidValue", `wire type ${wire}, expected ${expected}`);
	}
}

export function len(input: Input): Input {
	return input.take(Number(readVarint(input)));
}

export function writeLen(output: Output, n: number): void {
	writeVarint(output, BigInt(n));
}

export function lenSize(n: number): number {
	return varintSize(BigInt(n)) + n;
}

export function skip(input: Input, wire: number): void {
	switch (wire) {
		case VARINT:
			readVarint(input);
			return;
		case I64:
			input.take(8);
			return;
		case LEN:
			len(input);
			return;
		case I32:
			input.take(4);
			return;
		default:
			throw new PdsError("InvalidValue", `wire type ${wire}`);
	}
}

function varint<T>(decode: (v: bigint) => T, encode: (v: T) => bigint): Native<T> {
	return {
		serialize(v: T, output: Output): void {
			writeVarint(output, encode(v));
		},
		deserialize(input: Input): T {
			return decode(readVarint(input));
		},
		size_of(v: T): number {
			return varintSize(encode(v));
		},
	};
}

export const pds_int32 = varint<number>((v) => Number(BigInt.asIntN(32, v)), (v) => BigInt(v));
export const pds_int64 = varint<bigint>((v) => BigInt.asIntN(64, v), (v) => v);
export const pds_uint32 = varint<number>((v) => Number(BigInt.asUintN(32, v)), (v) => BigInt(v));
export const pds_uint64 = varint<bigint>((v) => v, (v) => v);
export const pds_sint32 = varint<number>(
	(v) => {
		const n = Number(v & 0xffffffffn);
		return (n >>> 1) ^ -(n & 1);
	},
	(v) => BigInt(((v << 1) ^ (v >> 31)) >>> 0),
);
export const pds_sint64 = varint<bigint>(
	(v) => (v >> 1n) ^ -(v & 1n),
	(v) => BigInt.asUintN(64, (v << 1n) ^ (v >> 63n)),
);
export const pds_bool = varint<boolean>((v) => v !== 0n, (v) => (v ? 1n : 0n));

function fixed<T>(
	size: number,
	get: (view: DataView) => T,
	set: (view: DataView, v: T) => void,
): Native<T> {
	return {
		serialize(v: T, output: Output): void {
			const bytes = new Uint8Array(size);
			set(new DataView(bytes.buffer), v);
			output.write(bytes);
		},
		deserialize(input: Input): T {
			const bytes = input.read_bytes(size);
			return get(new DataView(bytes.buffer, bytes.byteOffset, size));
		},
		size_of(_v: T): number {
			return size;
		},
	};
}

export const pds_fixed32 = fixed<number>(4, (d) => d.getUint32(0, true), (d, v) => d.setUint32(0, v, true));
export const pds_fixed64 = fixed<bigint>(
	8,
	(d) => d.getBigUint64(0, true),
	(d, v) => d.setBigUint64(0, v, true),
);
export const pds_sfixed32 = fixed<number>(4, (d) => d.getInt32(0, true), (d, v) => d.setInt32(0, v, true));
export const pds_sfixed64 = fixed<bigint>(
	8,
	(d) => d.getBigInt64(0, true),
	(d, v) => d.setBigInt64(0, v, true),
);
export const pds_float = fixed<number>(4, (d) => d.getFloat32(0, true), (d, v) => d.setFloat32(0, v, true));
export const pds_double = fixed<number>(8, (d) => d.getFloat64(0, true), (d, v) => d.setFloat64(0, v, true));

const utf8 = new TextDecoder("utf-8", { fatal: true });
const encoder = new TextEncoder();

export const pds_string: Native<string> = {
	serialize(v: string, output: Output): void {
		output.write(encoder.encode(v));
	},
	deserialize(input: Input): string {
		try {
			return utf8.decode(input.read_rest());
		} catch {
			throw new PdsError("InvalidValue", "string is not utf-8");
		}
	},
	size_of(v: string): number {
		return encoder.encode(v).length;
	},
};

export const pds_bytes: Native<Uint8Array> = {
	serialize(v: Uint8Array, output: Output): void {
		output.write(v);
	},
	deserialize(input: Input): Uint8Array {
		return input.read_rest();
	},
	size_of(v: Uint8Array): number {
		return v.length;
	},
};
