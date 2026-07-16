import type { AliasHeavy, OptionalAlias } from "./generated/serde-aliases";

function readFirst<T extends AliasHeavy>(value: T) {
  return value.first ?? value.first_old;
}

const canonical: AliasHeavy = {
  first: "", second: "", third: "", fourth: "", fifth: "", sixth: "", seventh: "", eighth: "",
  ninth: "", tenth: "", eleventh: "", twelfth: "", thirteenth: "", fourteenth: "", fifteenth: "", sixteenth: "",
};
const aliases: AliasHeavy = {
  first_old: "", second_old: "", third_old: "", fourth_old: "", fifth_old: "", sixth_old: "", seventh_old: "", eighth_old: "",
  ninth_old: "", tenth_old: "", eleventh_old: "", twelfth_old: "", thirteenth_old: "", fourteenth_old: "", fifteenth_old: "", sixteenth_old: "",
};
// @ts-expect-error serde rejects duplicate spellings for one field.
const duplicate: AliasHeavy = { ...canonical, first_old: "" };
// @ts-expect-error a required field must use one accepted spelling.
const missing: AliasHeavy = {};

const absentOptional: OptionalAlias = {};
const canonicalOptional: OptionalAlias = { value: "" };
const aliasOptional: OptionalAlias = { value_old: "" };
// @ts-expect-error serde rejects duplicate spellings for an optional field too.
const duplicateOptional: OptionalAlias = { value: "", value_old: "" };

void readFirst(canonical);
void aliases;
void duplicate;
void missing;
void absentOptional;
void canonicalOptional;
void aliasOptional;
void duplicateOptional;
