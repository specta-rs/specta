import { expect, test } from "bun:test";
import { z } from "zod";
import {
  ExternalEnumSchema,
  GenericSchema,
  OptionalFlattenSchema,
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
    enum_keys: { First: "value" },
    generic_finite_keys: { First: "value" },
    nested_generic_finite_keys: { First: "value" },
    remote_keys: { "42": "value" },
  }).success).toBe(true);
  expect(ExternalEnumSchema.safeParse({ Newtype: "value", Tuple: [1, true] }).success).toBe(false);
  expect(UntaggedMatchingFieldSchema.safeParse({ Variant: "value", extra: true }).success).toBe(true);
  expect(UntaggedMatchingFieldSchema.safeParse({ extra: true }).success).toBe(true);
  expect(OptionalFlattenSchema.parse({ id: "id", inner: "kept" })).toEqual({ id: "id", inner: "kept" });
});

test("generated schemas reject invalid primitive wire values", () => {
  expect(WireTypesSchema.safeParse({
    character: "too long",
    integer_keys: { invalid: "value" },
    boolean_keys: {},
    newtype_keys: {},
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
    enum_keys: {},
    generic_finite_keys: {},
    nested_generic_finite_keys: {},
    remote_keys: {},
  }).success).toBe(false);
});
