export type ErrorKind = "NotEnoughData" | "TooLargeInteger" | "InvalidValue";

export class PdsError extends Error {
	constructor(readonly kind: ErrorKind, detail?: string) {
		super(detail ? `${kind}: ${detail}` : kind);
		this.name = "PdsError";
	}
}

export class Input {
	#data: Uint8Array;
	#offset = 0;
	#maskAt = -1;
	#mask = 0xff;

	constructor(data: Uint8Array) {
		this.#data = data;
	}

	get offset(): number {
		return this.#offset;
	}

	remaining(): number {
		return Math.max(0, this.#data.length - this.#offset);
	}

	mask_next(and: number): void {
		this.#maskAt = this.#offset;
		this.#mask = and;
	}

	peek(): number {
		if (this.#offset >= this.#data.length) {
			throw new PdsError("NotEnoughData");
		}
		const byte = this.#data[this.#offset];
		return this.#offset === this.#maskAt ? byte & this.#mask : byte;
	}

	read(): number {
		const byte = this.peek();
		this.#offset++;
		return byte;
	}

	skip(): void {
		this.#offset++;
	}

	rewind(offset: number): void {
		this.#offset = offset;
	}

	read_bytes(n: number): Uint8Array {
		if (n > this.remaining()) {
			throw new PdsError("NotEnoughData");
		}
		const out = this.#data.slice(this.#offset, this.#offset + n);
		if (n > 0 && this.#offset === this.#maskAt) {
			out[0] &= this.#mask;
		}
		this.#offset += n;
		return out;
	}

	read_rest(): Uint8Array {
		return this.read_bytes(this.remaining());
	}

	take(n: number): Input {
		return new Input(this.read_bytes(n));
	}
}

export class Output {
	#buf = new Uint8Array(64);
	#len = 0;

	get length(): number {
		return this.#len;
	}

	#reserve(n: number): void {
		if (this.#len + n <= this.#buf.length) {
			return;
		}
		let size = this.#buf.length * 2;
		while (size < this.#len + n) {
			size *= 2;
		}
		const next = new Uint8Array(size);
		next.set(this.#buf.subarray(0, this.#len));
		this.#buf = next;
	}

	write(bytes: ArrayLike<number>): void {
		this.#reserve(bytes.length);
		this.#buf.set(bytes, this.#len);
		this.#len += bytes.length;
	}

	write_byte(byte: number): void {
		this.#reserve(1);
		this.#buf[this.#len++] = byte & 0xff;
	}

	into_inner(): Uint8Array {
		return this.#buf.slice(0, this.#len);
	}
}

export function equal(a: unknown, b: unknown): boolean {
	if (a === b) {
		return true;
	}
	if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) {
		return false;
	}
	if (ArrayBuffer.isView(a) || Array.isArray(a)) {
		const x = a as ArrayLike<unknown>;
		const y = b as ArrayLike<unknown>;
		if (!(ArrayBuffer.isView(b) || Array.isArray(b)) || x.length !== y.length) {
			return false;
		}
		for (let i = 0; i < x.length; i++) {
			if (!equal(x[i], y[i])) {
				return false;
			}
		}
		return true;
	}
	const ka = Object.keys(a);
	const kb = Object.keys(b);
	if (ka.length !== kb.length) {
		return false;
	}
	const ra = a as Record<string, unknown>;
	const rb = b as Record<string, unknown>;
	return ka.every((k) => Object.hasOwn(rb, k) && equal(ra[k], rb[k]));
}

const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true });

export function utf8_encode(s: string): Uint8Array {
	return encoder.encode(s);
}

export function utf8_decode(bytes: Uint8Array): string {
	try {
		return decoder.decode(bytes);
	} catch {
		throw new PdsError("InvalidValue", "invalid utf-8");
	}
}

export interface Native<T> {
	serialize(v: T, output: Output): void;
	deserialize(input: Input): T;
	size_of(v: T): number;
}

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

export const pds_u8 = fixed<number>(1, (d) => d.getUint8(0), (d, v) => d.setUint8(0, v));
export const pds_i8 = fixed<number>(1, (d) => d.getInt8(0), (d, v) => d.setInt8(0, v));
export const pds_u16 = fixed<number>(2, (d) => d.getUint16(0), (d, v) => d.setUint16(0, v));
export const pds_i16 = fixed<number>(2, (d) => d.getInt16(0), (d, v) => d.setInt16(0, v));
export const pds_u32 = fixed<number>(4, (d) => d.getUint32(0), (d, v) => d.setUint32(0, v));
export const pds_i32 = fixed<number>(4, (d) => d.getInt32(0), (d, v) => d.setInt32(0, v));
export const pds_u64 = fixed<bigint>(8, (d) => d.getBigUint64(0), (d, v) => d.setBigUint64(0, v));
export const pds_i64 = fixed<bigint>(8, (d) => d.getBigInt64(0), (d, v) => d.setBigInt64(0, v));
export const pds_f32 = fixed<number>(4, (d) => d.getFloat32(0), (d, v) => d.setFloat32(0, v));
export const pds_f64 = fixed<number>(8, (d) => d.getFloat64(0), (d, v) => d.setFloat64(0, v));

export const pds_bool: Native<boolean> = {
	serialize(v: boolean, output: Output): void {
		output.write_byte(v ? 1 : 0);
	},
	deserialize(input: Input): boolean {
		return input.read() !== 0;
	},
	size_of(_v: boolean): number {
		return 1;
	},
};

export const pds_void: Native<null> = {
	serialize(_v: null, _output: Output): void {},
	deserialize(_input: Input): null {
		return null;
	},
	size_of(_v: null): number {
		return 0;
	},
};
