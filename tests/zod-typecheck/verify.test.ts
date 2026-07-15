import { expect, test } from "bun:test";
import { z } from "zod";
import {
  ContextualExternalWrapperSchema,
  ExternalEnumSchema,
  GenericSchema,
  GenericMapHolderSchema,
  MapOnlyExternalWrapperSchema,
  OptionalFlattenSchema,
  ProtoFieldSchema,
  RecursiveSchema,
  UntaggedMatchingFieldSchema,
  WireTypesSchema,
} from "./generated/bindings";

// Explicit schema type arguments must always be backed by runtime schema arguments.
// @ts-expect-error A custom generic cannot silently retain the default string schema.
GenericSchema<z.ZodNumber>();

test("generated schemas validate representative wire values", () => {
  expect(RecursiveSchema.safeParse({ children: [{ children: [] }] }).success).toBe(true);
  expect(GenericSchema().safeParse({ first: "one", second: "two" }).success).toBe(true);
  expect(WireTypesSchema.safeParse({
    character: "😀",
    integer_keys: { "-2": "value" },
    boolean_keys: { true: "value" },
    newtype_keys: { "42": "value" },
    boolean_newtype_keys: { false: "value" },
    enum_keys: { First: "value" },
    generic_finite_keys: { First: "value" },
    nested_generic_finite_keys: { First: "value" },
    remote_keys: { "42": "value" },
  }).success).toBe(true);
  expect(ExternalEnumSchema.safeParse({ Newtype: "value", Tuple: [1, true] }).success).toBe(false);
  expect(ContextualExternalWrapperSchema.safeParse({ kind: "Value", Unit: null }).success).toBe(true);
  expect(ContextualExternalWrapperSchema.safeParse({ kind: "Value", Newtype: 1 }).success).toBe(true);
  expect(ContextualExternalWrapperSchema.safeParse({ kind: "Value", Unit: null, Newtype: 1 }).success).toBe(false);
  expect(MapOnlyExternalWrapperSchema.safeParse({ kind: "Value", First: { value: 1 } }).success).toBe(true);
  expect(MapOnlyExternalWrapperSchema.safeParse({ kind: "Value", Second: { label: "ok" } }).success).toBe(true);
  expect(MapOnlyExternalWrapperSchema.safeParse({ kind: "Value", First: { value: 1 }, Second: { label: "no" } }).success).toBe(false);
  expect(UntaggedMatchingFieldSchema.safeParse({ Variant: "value", extra: true }).success).toBe(true);
  expect(UntaggedMatchingFieldSchema.safeParse({ extra: true }).success).toBe(true);
  expect(OptionalFlattenSchema.parse({ id: "id", inner: "kept" })).toEqual({ id: "id", inner: "kept" });
  expect(OptionalFlattenSchema.safeParse({ id: "id", inner: 1 }).success).toBe(false);
  expect(OptionalFlattenSchema.safeParse({ id: "id", unrelated: 1 }).success).toBe(true);
  expect(ProtoFieldSchema.safeParse(JSON.parse('{"__proto__":"value"}')).success).toBe(true);
  expect(ProtoFieldSchema.safeParse({}).success).toBe(false);
  expect(GenericMapHolderSchema.safeParse({
    booleans: { values: { true: "yes" } },
    integers: { values: { "-2": "value" } },
    finite: { values: { First: "value" } },
    chained: { marker: true, values: { false: "value" } },
  }).success).toBe(true);
  expect(GenericMapHolderSchema.safeParse({
    booleans: { values: { invalid: "no" } },
    integers: { values: { nope: "no" } },
    finite: { values: { Third: "no" } },
    chained: { marker: true, values: { invalid: "no" } },
  }).success).toBe(false);
});

test("generated schemas reject invalid primitive wire values", () => {
  expect(WireTypesSchema.safeParse({
    character: "too long",
    integer_keys: { invalid: "value" },
    boolean_keys: {},
    newtype_keys: {},
    boolean_newtype_keys: {},
    enum_keys: {},
    generic_finite_keys: {},
    nested_generic_finite_keys: {},
    remote_keys: {},
  }).success).toBe(false);

  expect(WireTypesSchema.safeParse({
    character: "x",
    integer_keys: { "2147483648": "out of range" },
    boolean_keys: {},
    newtype_keys: {},
    boolean_newtype_keys: {},
    enum_keys: {},
    generic_finite_keys: {},
    nested_generic_finite_keys: {},
    remote_keys: {},
  }).success).toBe(false);
});
