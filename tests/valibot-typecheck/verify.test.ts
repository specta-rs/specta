import { expect, test } from "bun:test";
import * as v from "valibot";
import {
  ContextualExternalWrapperSchema,
  DangerousReferencedFlattenSchema,
  DefaultTupleSchema,
  EmptyObjectSchema,
  ExternalEnumSchema,
  GenericMapSchema,
  GenericSchema,
  GenericMapHolderSchema,
  MapOnlyExternalWrapperSchema,
  NarrowFloatsSchema,
  OptionalFlattenSchema,
  OptionalObjectSchema,
  ProtoFieldSchema,
  RecursiveSchema,
  ReferencedFlattenSchema,
  UntaggedMatchingFieldSchema,
  WireTypesSchema,
} from "./generated/bindings";
import {
  LowLevelInlineSchema,
  LowLevelRecordSchema,
  LowLevelReferenceSchema,
} from "./generated/low-level";
import { AdaptedManualNamespaceTypeSchema } from "./generated/namespaces-manual";

const transformingNumber = v.pipe(v.string(), v.transform(Number));
const TransformingGenericSchema = GenericSchema(transformingNumber);
const lowercaseKey = v.pipe(v.string(), v.transform((key) => key.toLowerCase()));
const LowercaseMapSchema = GenericMapSchema(v.string(), lowercaseKey);
const transformingInput: v.InferInput<typeof TransformingGenericSchema> = {
  first: "1",
  second: "2",
};
const transformingOutput: v.InferOutput<typeof TransformingGenericSchema> = {
  first: 1,
  second: 2,
};
// @ts-expect-error Transforming schemas accept strings, not their numeric outputs.
const invalidTransformingInput: v.InferInput<typeof TransformingGenericSchema> = transformingOutput;
void invalidTransformingInput;

// Adapted object schemas intentionally expose only generic schema metadata.
// @ts-expect-error Object combinators require a real Valibot ObjectSchema with entries metadata.
v.partial(EmptyObjectSchema);

const validWire = () => ({
  character: "x",
  floating: null,
  fixed_array: [1, 2],
  tuple: ["value", true],
  integer_keys: {},
  string_keys: {},
  boolean_keys: {},
  newtype_keys: {},
  boolean_newtype_keys: {},
  enum_keys: {},
  generic_finite_keys: {},
  nested_generic_finite_keys: {},
  remote_keys: {},
});

// Explicit schema type arguments must always be backed by runtime schema arguments.
// @ts-expect-error A custom generic cannot silently retain the default string schema.
GenericSchema<v.GenericSchema<number>>();

test("generated schemas validate representative wire values", () => {
  expect(v.safeParse(RecursiveSchema, { children: [{ children: [] }] }).success).toBe(true);
  expect(v.safeParse(GenericSchema(), { first: "one", second: "two" }).success).toBe(true);
  expect(v.safeParse(GenericSchema(), { first: "😀", second: "value" }).success).toBe(true);
  expect(v.safeParse(WireTypesSchema, {
    character: "😀",
    floating: null,
    fixed_array: [1, 2],
    tuple: ["value", true],
    integer_keys: { "-2": "value" },
    string_keys: {},
    boolean_keys: { true: "value" },
    newtype_keys: { "42": "value" },
    boolean_newtype_keys: { false: "value" },
    enum_keys: { First: "value" },
    generic_finite_keys: { First: "value" },
    nested_generic_finite_keys: { First: "value" },
    remote_keys: { "42": "value" },
  }).success).toBe(true);
  expect(v.parse(TransformingGenericSchema, transformingInput)).toEqual(transformingOutput);
  expect(v.parse(LowercaseMapSchema, { values: { UPPER: "first", upper: "last" } })).toEqual({
    values: { upper: "last" },
  });
  expect(v.safeParse(ExternalEnumSchema, { Newtype: "value", Tuple: [1, true] }).success).toBe(false);
  expect(v.safeParse(ExternalEnumSchema, { Tuple: [1, true, false] }).success).toBe(false);
  expect(v.safeParse(ContextualExternalWrapperSchema, { kind: "Value", Unit: null }).success).toBe(true);
  expect(v.safeParse(ContextualExternalWrapperSchema, { kind: "Value", Newtype: 1 }).success).toBe(true);
  expect(v.safeParse(ContextualExternalWrapperSchema, { kind: "Value", Unit: null, Newtype: 1 }).success).toBe(false);
  expect(v.safeParse(MapOnlyExternalWrapperSchema, { kind: "Value", First: { value: 1 } }).success).toBe(true);
  expect(v.safeParse(MapOnlyExternalWrapperSchema, { kind: "Value", Second: { label: "ok" } }).success).toBe(true);
  expect(v.safeParse(MapOnlyExternalWrapperSchema, { kind: "Value", First: { value: 1 }, Second: { label: "no" } }).success).toBe(false);
  expect(v.safeParse(UntaggedMatchingFieldSchema, { Variant: "value", extra: true }).success).toBe(true);
  expect(v.safeParse(UntaggedMatchingFieldSchema, { extra: true }).success).toBe(true);
  expect(v.parse(OptionalFlattenSchema, { id: "id", inner: "kept" })).toEqual({ id: "id", inner: "kept" });
  expect(v.safeParse(OptionalFlattenSchema, { id: "id", inner: 1 }).success).toBe(false);
  expect(v.safeParse(OptionalFlattenSchema, { id: "id", unrelated: 1 }).success).toBe(true);
  expect(v.parse(ProtoFieldSchema, JSON.parse('{"__proto__":"value"}'))).toEqual(
    JSON.parse('{"__proto__":"value"}'),
  );
  expect(v.safeParse(ProtoFieldSchema, {}).success).toBe(false);
  expect(v.safeParse(EmptyObjectSchema, []).success).toBe(false);
  expect(v.safeParse(EmptyObjectSchema, new Date()).success).toBe(false);
  expect(v.safeParse(OptionalObjectSchema, []).success).toBe(false);
  expect(v.safeParse(ReferencedFlattenSchema, { id: "id", First: { value: 1 } }).success).toBe(true);
  const dangerousFlatten = JSON.parse(
    '{"__proto__":"proto","constructor":"constructor","prototype":"prototype","First":{"value":1}}',
  );
  expect(v.parse(DangerousReferencedFlattenSchema, dangerousFlatten)).toEqual(dangerousFlatten);
  expect(v.parse(DefaultTupleSchema, [1])).toEqual([1]);
  expect(v.safeParse(DefaultTupleSchema, [1, undefined]).success).toBe(false);
  expect(v.safeParse(GenericMapHolderSchema, {
    booleans: { values: { true: "yes" } },
    integers: { values: { "-2": "value" } },
    finite: { values: { First: "value" } },
    chained: { marker: true, values: { false: "value" } },
  }).success).toBe(true);
  expect(v.safeParse(GenericMapHolderSchema, {
    booleans: { values: { invalid: "no" } },
    integers: { values: { nope: "no" } },
    finite: { values: { Third: "no" } },
    chained: { marker: true, values: { invalid: "no" } },
  }).success).toBe(false);
});

test("generated schemas reject invalid primitive wire values", () => {
  expect(v.safeParse(WireTypesSchema, {
    character: "too long",
    floating: null,
    fixed_array: [1, 2],
    tuple: ["value", true],
    integer_keys: { invalid: "value" },
    string_keys: {},
    boolean_keys: {},
    newtype_keys: {},
    boolean_newtype_keys: {},
    enum_keys: {},
    generic_finite_keys: {},
    nested_generic_finite_keys: {},
    remote_keys: {},
  }).success).toBe(false);

  expect(v.safeParse(WireTypesSchema, {
    character: "x",
    floating: null,
    fixed_array: [1, 2],
    tuple: ["value", true],
    integer_keys: { "2147483648": "out of range" },
    string_keys: {},
    boolean_keys: {},
    newtype_keys: {},
    boolean_newtype_keys: {},
    enum_keys: {},
    generic_finite_keys: {},
    nested_generic_finite_keys: {},
    remote_keys: {},
  }).success).toBe(false);

  expect(v.safeParse(WireTypesSchema, {
    character: "\ud800",
    floating: Infinity,
    fixed_array: [1, 2, 3],
    tuple: ["value", true, false],
    integer_keys: {},
    string_keys: [],
    boolean_keys: {},
    newtype_keys: {},
    boolean_newtype_keys: {},
    enum_keys: {},
    generic_finite_keys: {},
    nested_generic_finite_keys: {},
    remote_keys: {},
  }).success).toBe(false);

  expect(v.safeParse(WireTypesSchema, { ...validWire(), character: "\ud800" }).success).toBe(false);
  expect(v.safeParse(WireTypesSchema, { ...validWire(), character: "\udc00" }).success).toBe(false);
  expect(v.safeParse(GenericSchema(), { first: "\ud800", second: "value" }).success).toBe(false);
  expect(v.safeParse(GenericSchema(), { first: "\udc00", second: "value" }).success).toBe(false);
  expect(v.safeParse(WireTypesSchema, { ...validWire(), floating: Infinity }).success).toBe(false);
  expect(v.safeParse(NarrowFloatsSchema, { single: 3.4028235e38 }).success).toBe(true);
  expect(v.safeParse(NarrowFloatsSchema, { single: 3.4028236e38 }).success).toBe(false);
  expect(v.safeParse(WireTypesSchema, { ...validWire(), fixed_array: [1, 2, 3] }).success).toBe(false);
  expect(v.safeParse(WireTypesSchema, { ...validWire(), tuple: ["value", true, false] }).success).toBe(false);
  expect(v.safeParse(WireTypesSchema, { ...validWire(), string_keys: [] }).success).toBe(false);
  expect(
    v.safeParse(WireTypesSchema, { ...validWire(), string_keys: { ["\ud800"]: "value" } }).success,
  ).toBe(false);
  expect(
    v.safeParse(WireTypesSchema, { ...validWire(), string_keys: { ["\udc00"]: "value" } }).success,
  ).toBe(false);
  expect(
    v.safeParse(WireTypesSchema, { ...validWire(), string_keys: { ["😀"]: "value" } }).success,
  ).toBe(true);
});

test("record schemas preserve and validate prototype-sensitive keys", () => {
  const stringKeys = JSON.parse(
    '{"__proto__":"proto","constructor":"constructor","prototype":"prototype"}',
  );
  const wire = {
    character: "x",
    floating: null,
    fixed_array: [1, 2],
    tuple: ["value", true],
    integer_keys: {},
    string_keys: stringKeys,
    boolean_keys: {},
    newtype_keys: {},
    boolean_newtype_keys: {},
    enum_keys: {},
    generic_finite_keys: {},
    nested_generic_finite_keys: {},
    remote_keys: {},
  };
  expect(v.parse(WireTypesSchema, wire).string_keys).toEqual(stringKeys);

  for (const key of ["__proto__", "constructor", "prototype"]) {
    const invalid = JSON.parse(`{"${key}":1}`);
    expect(v.safeParse(WireTypesSchema, { ...wire, string_keys: invalid }).success).toBe(false);
  }
});

test("low-level schemas compose with the public runtime helpers", () => {
  const value = JSON.parse('{"__proto__":"proto","constructor":"constructor"}');
  expect(v.parse(LowLevelInlineSchema, value)).toEqual(value);
  expect(v.parse(LowLevelRecordSchema, value)).toEqual(value);
  expect(v.safeParse(LowLevelInlineSchema, { ["\ud800"]: "value" }).success).toBe(false);
  expect(v.safeParse(LowLevelReferenceSchema, {}).success).toBe(true);
  expect(v.safeParse(LowLevelReferenceSchema, []).success).toBe(false);
});

test("manual namespace exports are initialized before framework adapters", () => {
  expect(v.parse(AdaptedManualNamespaceTypeSchema, "value")).toBe("value");
});
