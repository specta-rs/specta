import type { AliasHeavy, CatalogEntry, CatalogResponse, OptionalAlias } from "./generated/serde-aliases";

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
const duplicateOptional: OptionalAlias = { value: "", value_old: "" };
// @ts-expect-error serde requires an object even when every field is optional.
const undefinedOptional: OptionalAlias = undefined;

void readFirst(canonical);
void aliases;
void duplicate;
void missing;
void absentOptional;
void canonicalOptional;
void aliasOptional;
void duplicateOptional;
void undefinedOptional;

declare const raw: CatalogResponse;

const mappedEntries = raw.entries.map(entry => ({
  id: entry.modelId ?? entry.model_id,
  inputCost: entry.modelPricing?.inputCost ?? entry.modelPricing?.input_cost,
  provider: entry.providerInfo?.displayName ?? entry.providerInfo?.display_name,
  tools: entry.modelCapability?.supportsTools ?? entry.modelCapability?.supports_tools,
  thinking: entry.modelThinking && "configured" in entry.modelThinking
    ? entry.modelThinking.configured?.effortLevels ?? entry.modelThinking.configured?.effort_levels
    : entry.modelThinking?.extensible.effort_levels,
  limits: entry.modelLimits?.maxInputTokens ?? entry.modelLimits?.max_input_tokens,
  label: entry.modelMetadata?.displayLabel ?? entry.modelMetadata?.display_label,
}));

function consumeEntry(entry: Pick<CatalogEntry, "modelId" | "modelPricing">) {
  return entry.modelPricing?.outputCost;
}

const structurallyAssigned: { modelId?: string | null } = raw.entries[0];
type EntryPresentation = Pick<CatalogEntry, "modelId" | "modelPricing" | "modelThinking" | "modelLimits" | "modelMetadata">;
const presentation: EntryPresentation = raw.entries[0];

type QueryResult<T> = { data: T };
function consumeQuery<T extends { entries: CatalogEntry[] }>(query: QueryResult<T>) {
  return query.data.entries.map(consumeEntry);
}

const queryResult: QueryResult<CatalogResponse> = { data: raw };
const queriedEntries = consumeQuery(queryResult);

void duplicateOptional;
void mappedEntries;
void structurallyAssigned;
void presentation;
void queriedEntries;
