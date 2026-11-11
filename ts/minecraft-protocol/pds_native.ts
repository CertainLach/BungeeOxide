import { Input, type Native, Output, PdsError } from "./pds.ts";
import * as pds_Tag from "../nbt/gen/Tag.ts";

export {
	pds_bool,
	pds_f32,
	pds_f64,
	pds_i16,
	pds_i32,
	pds_i64,
	pds_i8,
	pds_u16,
	pds_u32,
	pds_u64,
	pds_u8,
	pds_void,
} from "./pds.ts";

export const pds_varint: Native<number> = {
	serialize(v: number, output: Output): void {
		let value = v >>> 0;
		while (true) {
			const b = value & 0x7f;
			value >>>= 7;
			if (value === 0) {
				output.write_byte(b);
				return;
			}
			output.write_byte(b | 0x80);
		}
	},
	deserialize(input: Input): number {
		let out = 0;
		for (let shift = 0;; shift += 7) {
			if (shift >= 35) {
				throw new PdsError("TooLargeInteger");
			}
			const b = input.read();
			out |= (b & 0x7f) << shift;
			if ((b & 0x80) === 0) {
				return out | 0;
			}
		}
	},
	size_of(v: number): number {
		let size = 1;
		for (let value = (v >>> 0) >>> 7; value !== 0; value >>>= 7) {
			size++;
		}
		return size;
	},
};

export const pds_varlong: Native<bigint> = {
	serialize(v: bigint, output: Output): void {
		let value = BigInt.asUintN(64, v);
		while (true) {
			const b = Number(value & 0x7fn);
			value >>= 7n;
			if (value === 0n) {
				output.write_byte(b);
				return;
			}
			output.write_byte(b | 0x80);
		}
	},
	deserialize(input: Input): bigint {
		let out = 0n;
		for (let shift = 0n;; shift += 7n) {
			if (shift >= 70n) {
				throw new PdsError("TooLargeInteger");
			}
			const b = input.read();
			out |= BigInt(b & 0x7f) << shift;
			if ((b & 0x80) === 0) {
				return BigInt.asIntN(64, out);
			}
		}
	},
	size_of(v: bigint): number {
		let size = 1;
		for (let value = BigInt.asUintN(64, v) >> 7n; value !== 0n; value >>= 7n) {
			size++;
		}
		return size;
	},
};

export const pds_UUID: Native<bigint> = {
	serialize(v: bigint, output: Output): void {
		const bytes = new Uint8Array(16);
		const view = new DataView(bytes.buffer);
		const u = BigInt.asUintN(128, v);
		view.setBigUint64(0, u >> 64n);
		view.setBigUint64(8, BigInt.asUintN(64, u));
		output.write(bytes);
	},
	deserialize(input: Input): bigint {
		const bytes = input.read_bytes(16);
		const view = new DataView(bytes.buffer, bytes.byteOffset, 16);
		return (view.getBigUint64(0) << 64n) | view.getBigUint64(8);
	},
	size_of(_v: bigint): number {
		return 16;
	},
};

export type LpVec3 = { scale: number; x: number; y: number; z: number };

export function lpVec3_to_f64(v: LpVec3): [number, number, number] {
	const unpack = (q: number) => ((Math.min(q, 32766) * 2) / 32766 - 1) * v.scale;
	return [unpack(v.x), unpack(v.y), unpack(v.z)];
}

function lpVec3_high(v: LpVec3): number | null {
	return v.scale > 3 ? Math.floor(v.scale / 4) | 0 : null;
}

export const pds_lpVec3: Native<LpVec3> = {
	serialize(v: LpVec3, output: Output): void {
		if (v.scale === 0) {
			output.write_byte(0);
			return;
		}
		const high = lpVec3_high(v);
		const markers = (v.scale % 4) | (high === null ? 0 : 4);
		const packed = markers + (v.x & 0x7fff) * 0x8 + (v.y & 0x7fff) * 0x40000 +
			(v.z & 0x7fff) * 0x200000000;
		const bytes = new Uint8Array(6);
		bytes[0] = packed % 0x100;
		bytes[1] = Math.floor(packed / 0x100) % 0x100;
		new DataView(bytes.buffer).setUint32(2, Math.floor(packed / 0x10000) % 0x100000000);
		output.write(bytes);
		if (high !== null) {
			pds_varint.serialize(high, output);
		}
	},
	deserialize(input: Input): LpVec3 {
		const a = input.read();
		if (a === 0) {
			return { scale: 0, x: 0, y: 0, z: 0 };
		}
		const b = input.read();
		const rest = input.read_bytes(4);
		const c = new DataView(rest.buffer, rest.byteOffset, 4).getUint32(0);
		const packed = c * 0x10000 + b * 0x100 + a;
		let scale = a & 3;
		if ((a & 4) !== 0) {
			scale += (pds_varint.deserialize(input) >>> 0) * 4;
		}
		return {
			scale,
			x: Math.floor(packed / 0x8) % 0x8000,
			y: Math.floor(packed / 0x40000) % 0x8000,
			z: Math.floor(packed / 0x200000000) % 0x8000,
		};
	},
	size_of(v: LpVec3): number {
		if (v.scale === 0) {
			return 1;
		}
		const high = lpVec3_high(v);
		return high === null ? 6 : 6 + pds_varint.size_of(high);
	},
};

export const pds_restBuffer: Native<Uint8Array> = {
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

export const pds_anonymousNbt: Native<pds_Tag.Ty> = pds_Tag;
export const pds_anonOptionalNbt: Native<pds_Tag.Ty> = pds_Tag;
