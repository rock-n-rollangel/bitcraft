import { beforeAll, describe, expect, it } from "vitest";
import { init, Schema, type SchemaDef } from "../src/index.js";

beforeAll(async () => { await init(); });

const def: SchemaDef = {
  fields: [
    { name: "len", kind: { type: "Scalar" }, signed: false, assemble: "ConcatMsb",
      fragments: [{ offset_bits: 0, len_bits: 8 }] },
    { name: "items",
      kind: { type: "Array", count: { from_field: "len" }, stride_bits: 8, offset_bits: 8 },
      signed: false, assemble: "ConcatMsb",
      fragments: [{ offset_bits: 0, len_bits: 8 }] },
  ],
};

describe("dynamic array count", () => {
  it("parses the element count from the packet", () => {
    const schema = Schema.compile(def);
    const parsed = schema.parse(new Uint8Array([0x03, 0x0a, 0x0b, 0x0c]));
    expect(parsed.len).toEqual({ kind: "u64", value: 3n });
    expect(parsed.items).toEqual({
      kind: "array",
      value: [
        { kind: "u64", value: 10n },
        { kind: "u64", value: 11n },
        { kind: "u64", value: 12n },
      ],
    });
  });

  it("serializes with the count derived from the array length", () => {
    const schema = Schema.compile(def);
    const bytes = schema.serialize({
      items: {
        kind: "array",
        value: [
          { kind: "u64", value: 10n },
          { kind: "u64", value: 11n },
        ],
      },
    });
    expect(bytes).toEqual(new Uint8Array([0x02, 0x0a, 0x0b]));
  });
});
