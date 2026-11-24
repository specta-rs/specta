import { describe, expect, test } from "bun:test";
import { Schema } from "effect";
import {
  AliasedSchema,
  ChainedMapSchema,
  ComplexSchema,
  DefaultedKeyMapSchema,
  DefaultsSchema,
  FlattenedSchema,
  GenericMapSchema,
  GenericMapDefaultSchema,
  GenericSchema,
  MessageSchema,
  MutualASchema,
  NodeSchema,
  OptionalFieldsSchema,
  OptionalFlattenSchema,
  OptionalMapFlattenSchema,
  ProtoFieldSchema,
  UserSchema,
} from "./generated/bindings";
import { effect_typecheck as NamespacedSchemas } from "./generated/namespaces";
import { effect_typecheck_UserSchema as PrefixedUserSchema } from "./generated/prefixed";
import { UserSchema as FileUserSchema } from "./generated/files/effect_typecheck";

describe("specta-effect generated schemas", () => {
  test("decodes and encodes a struct with nullable fields", () => {
    const input = { id: 7, name: "Ada", nickname: null, active: true };
    const decoded = Schema.decodeUnknownSync(UserSchema)(input);
    expect(decoded).toEqual(input);
    expect(Schema.encodeSync(UserSchema)(decoded)).toEqual(input);

    type User = Schema.Schema.Type<typeof UserSchema>;
    const typed: User = decoded;
    expect(typed.id).toBe(7);
  });

  test("supports namespace, prefixed, and file layouts", () => {
    const input = { id: 8, name: "Grace", nickname: null, active: false };
    expect(Schema.decodeUnknownSync(NamespacedSchemas.UserSchema)(input)).toEqual(input);
    expect(Schema.decodeUnknownSync(PrefixedUserSchema)(input)).toEqual(input);
    expect(Schema.decodeUnknownSync(FileUserSchema)(input)).toEqual(input);
  });

  test("decodes exact optional keys", () => {
    expect(Schema.decodeUnknownSync(OptionalFieldsSchema)({ required: 1 })).toEqual({ required: 1 });
    expect(
      Schema.decodeUnknownSync(OptionalFieldsSchema)({ required: 1, nickname: null }),
    ).toEqual({ required: 1, nickname: null });
  });

  test("instantiates generic schemas", () => {
    const schema = GenericSchema(Schema.Number);
    const input = { value: 1, values: [2, 3] };
    expect(Schema.decodeUnknownSync(schema)(input)).toEqual(input);

    type GenericNumber = Schema.Schema.Type<typeof schema>;
    const typed: GenericNumber = input;
    expect(typed.values).toEqual([2, 3]);
  });

  test("applies chained generic defaults", () => {
    const schema = DefaultsSchema();
    const input = { first: "one", second: "two" };
    expect(Schema.decodeUnknownSync(schema)(input)).toEqual(input);

    type Defaults = Schema.Schema.Type<typeof schema>;
    const typed: Defaults = input;
    expect(typed.second).toBe("two");
  });

  test("decodes generic maps with default and explicit key schemas", () => {
    const defaultMap = GenericMapSchema();
    const decodedDefaultMap = Schema.decodeUnknownSync(defaultMap)({ values: { true: "yes" } });
    expect(decodedDefaultMap.values.true).toBe("yes");

    const integerMap = GenericMapSchema(Schema.Int, Schema.Int, Schema.String);
    const decodedMap = Schema.decodeUnknownSync(integerMap)({ values: { 1: "one" } });
    expect(decodedMap.values[1]).toBe("one");

    const chainedMap = ChainedMapSchema();
    const decodedChainedMap = Schema.decodeUnknownSync(chainedMap)({ marker: null, values: { false: "no" } });
    expect(decodedChainedMap.values.false).toBe("no");
  });

  test("decodes tagged generic enums", () => {
    const schema = MessageSchema(Schema.String);
    expect(Schema.decodeUnknownSync(schema)({ type: "hello", data: "Ada" })).toEqual({
      type: "hello",
      data: "Ada",
    });
    expect(Schema.decodeUnknownSync(schema)({ type: "ping" })).toEqual({
      type: "ping",
    });
    expect(
      Schema.decodeUnknownSync(schema)({ type: "goodbye", data: { reason: null } }),
    ).toEqual({ type: "goodbye", data: { reason: null } });
  });

  test("decodes a complex nested value", () => {
    const input = {
      user: { id: 1, name: "Ada", nickname: null, active: true },
      generic: { value: 2, values: [3] },
      defaulted: { first: "a", second: "b" },
      messages: [{ type: "hello", data: "hi" }],
      booleans: { true: [1, 2] },
      pair: [1, null],
      optional: { required: 1 },
      chained: { marker: null, values: { false: "no" } },
      default_map: { marker: true, value: { values: { true: "yes" } } },
    } as const;
    expect(Schema.decodeUnknownSync(ComplexSchema)(input)).toEqual(input);
  });

  test("accepts exactly one Serde field alias", () => {
    expect(Schema.is(AliasedSchema)({ value: "new", other: 1 })).toBe(true);
    expect(Schema.is(AliasedSchema)({ old_value: "old", other: 1 })).toBe(true);
    expect(Schema.is(AliasedSchema)({ value: "new", old_value: "old", other: 1 })).toBe(false);
    expect(Schema.is(AliasedSchema)({ other: 1 })).toBe(false);
  });

  test("resolves defaults on generic map-key wrappers", () => {
    const decoded = Schema.decodeUnknownSync(DefaultedKeyMapSchema)({
      values: { true: "yes" },
    });
    expect(decoded.values.true).toBe("yes");
  });

  test("rejects partial optional flattened payloads", () => {
    expect(Schema.is(OptionalFlattenSchema)({ id: 1 })).toBe(true);
    expect(Schema.is(OptionalFlattenSchema)({ id: 1, a: 1, b: 2 })).toBe(true);
    expect(Schema.is(OptionalFlattenSchema)({ id: 1, a: 1 })).toBe(false);
  });

  test("rejects invalid values in optional flattened maps", () => {
    expect(Schema.is(OptionalMapFlattenSchema)({ id: 1 })).toBe(true);
    expect(Schema.is(OptionalMapFlattenSchema)({ id: 1, x: "value" })).toBe(true);
    expect(Schema.is(OptionalMapFlattenSchema)({ id: 1, x: 1 })).toBe(false);
  });

  test("decodes mutually recursive schemas", () => {
    const input = { b: { a: { b: null } } };
    expect(Schema.is(MutualASchema)(input)).toBe(true);
  });

  test("instantiates generic schema factories in generic defaults", () => {
    const schema = GenericMapDefaultSchema();
    const input = { value: { values: { 1: 1 } } };
    expect(Schema.decodeUnknownSync(schema)(input)).toEqual(input);
  });

  test("decodes recursive schemas", () => {
    const input = { value: 1, children: [{ value: 2, children: [] }] };
    expect(Schema.decodeUnknownSync(NodeSchema)(input)).toEqual(input);
  });

  test("preserves __proto__ as an own property", () => {
    const input = JSON.parse('{"__proto__":"safe"}');
    const decoded = Schema.decodeUnknownSync(ProtoFieldSchema)(input);
    expect(Object.hasOwn(decoded, "__proto__")).toBe(true);
    expect(decoded.__proto__).toBe("safe");
  });

  test("decodes flattened maps without validating fixed fields twice", () => {
    const input = { id: 1, extra: "value" };
    const decoded = Schema.decodeUnknownSync(FlattenedSchema)(input);
    expect(JSON.stringify(decoded)).toBe(JSON.stringify(input));
  });
});
