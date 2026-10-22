// Type-level verification for the TypeScript exporter's map-key rendering.
//
// This file is compiled with `tsc --strict` (see tsconfig.json). Every map key
// must be a valid TypeScript `PropertyKey`, and a key or value of the wrong
// shape must be rejected. Regression coverage for
// https://github.com/specta-rs/specta/issues/594.
import type {
  GenericKeyMap,
  MaybeValidKey,
  WireMapTypes,
} from "./generated/bindings";
import type { LosslessMapTypes } from "./generated/lossless";

type FloatKeys = WireMapTypes["floats"];
type IntegerKeys = WireMapTypes["integers"];
type BooleanKeys = WireMapTypes["booleans"];
type NestedKeys = WireMapTypes["nested"];
type NamedWrapperKeys = WireMapTypes["named_wrapper"];

// Every map-key shape is a valid TypeScript property key.
const wire: WireMapTypes = {
  floats: { "1.5": "x" },
  integers: { "1": "x" },
  booleans: { true: "x", false: "y" },
  nested: { outer: { "2.5": "x" } },
  named_wrapper: { key: "x" },
};
void wire;

// Float keys drop the value-shaped `number | null` branch.
// @ts-expect-error a float-keyed map's values must be strings
const floatValue: FloatKeys = { "1.5": 1 };

// Integer keys keep their existing `number` rendering.
const integerKeys: IntegerKeys = { "1": "x" };
void integerKeys;
// @ts-expect-error a number-keyed map rejects non-numeric keys
const integerBadKey: IntegerKeys = { abc: "x" };

// Boolean keys are the stringified `"true" | "false"` literals.
const booleanKeys: BooleanKeys = { true: "x" };
void booleanKeys;
// @ts-expect-error a boolean-keyed map only accepts `true`/`false` keys
const booleanBadKey: BooleanKeys = { maybe: "x" };

// Nested maps route the inner key through the key path too.
const nestedKeys: NestedKeys = { outer: { "2.5": "x" } };
void nestedKeys;
// @ts-expect-error a nested map's inner values must be strings
const nestedBadValue: NestedKeys = { outer: { "2.5": 1 } };

// A concrete named wrapper keeps its own name (which resolves to `string`).
const namedWrapperKeys: NamedWrapperKeys = { key: "x" };
void namedWrapperKeys;
const namedWrapper: MaybeValidKey<string> = "x";
void namedWrapper;

// A generic key is locally constrained to `PropertyKey`.
const genericString: GenericKeyMap<string> = {
  generic: { a: "x" },
  generic_wrapper: { a: "x" },
};
void genericString;
const genericNumber: GenericKeyMap<number> = {
  generic: { 1: "x" },
  generic_wrapper: { 1: "x" },
};
void genericNumber;
// @ts-expect-error `symbol & PropertyKey` is `symbol`, not a string key
const genericSymbolBadKey: GenericKeyMap<symbol> = { generic: { a: "x" }, generic_wrapper: {} };

// The local `K & PropertyKey` constraint does not stop a non-key
// instantiation collapsing: `boolean & PropertyKey` is `never`, so the map is
// an empty object type and accepts any object. Documented so the limitation
// cannot change silently.
const genericBoolean: GenericKeyMap<boolean> = {
  generic: { a: "x" },
  generic_wrapper: {},
};
void genericBoolean;

// Wide-integer keys under `enable_lossless_bigints()` are lexical string keys,
// not `bigint`.
const lossless: LosslessMapTypes = {
  wide: { "18446744073709551615": "x" },
  floats: { "1.5": "x" },
};
void lossless;
// @ts-expect-error a wide-integer-keyed map's values must be strings
const losslessBadValue: LosslessMapTypes["wide"] = { "18446744073709551615": 1 };
