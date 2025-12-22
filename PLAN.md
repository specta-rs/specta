# Implementation Plan: Serde Attribute Support in Specta

## Overview
This document outlines the plan to implement missing Serde attributes in Specta to ensure type definitions accurately match runtime Serde serialization behavior.

## Architecture Changes Required

### 1. DataType Extensions
Add new metadata fields to core data structures:

```rust
// specta/src/datatype/fields.rs
pub struct Field {
    // Existing fields...
    pub(crate) optional: bool,
    pub(crate) flatten: bool,
    
    // NEW FIELDS
    pub(crate) aliases: Vec<Cow<'static, str>>,           // For #[serde(alias)]
    pub(crate) rename_serialize: Option<Cow<'static, str>>, // For split rename
    pub(crate) rename_deserialize: Option<Cow<'static, str>>,
    pub(crate) default_fn: Option<Cow<'static, str>>,     // For #[serde(default = "path")]
    pub(crate) skip_serializing: bool,                     // Distinct from skip
    pub(crate) skip_deserializing: bool,
    pub(crate) custom_serde: Option<SerdeOverride>,        // For with/serialize_with/deserialize_with
}

// New type for tracking custom serialization
pub enum SerdeOverride {
    SerializeWith(Cow<'static, str>),
    DeserializeWith(Cow<'static, str>),
    With(Cow<'static, str>),
}

// specta/src/datatype/enum.rs
pub struct EnumVariant {
    // Existing fields...
    pub(crate) skip: bool,
    
    // NEW FIELDS
    pub(crate) aliases: Vec<Cow<'static, str>>,
    pub(crate) rename_serialize: Option<Cow<'static, str>>,
    pub(crate) rename_deserialize: Option<Cow<'static, str>>,
    pub(crate) skip_serializing: bool,
    pub(crate) skip_deserializing: bool,
    pub(crate) custom_serde: Option<SerdeOverride>,
    pub(crate) is_other: bool,                             // For #[serde(other)]
    pub(crate) untagged: bool,                             // For per-variant #[serde(untagged)]
}

// specta/src/datatype/struct.rs
pub struct Struct {
    // Existing fields...
    pub(crate) fields: Fields,
    
    // NEW FIELDS
    pub(crate) deny_unknown_fields: bool,
    pub(crate) from_type: Option<Cow<'static, str>>,      // For #[serde(from)]
    pub(crate) try_from_type: Option<Cow<'static, str>>,  // For #[serde(try_from)]
    pub(crate) into_type: Option<Cow<'static, str>>,      // For #[serde(into)]
}

// specta/src/datatype/enum.rs
pub struct Enum {
    // Existing fields...
    pub(crate) repr: Option<EnumRepr>,
    
    // NEW FIELDS
    pub(crate) rename_all_fields: Option<Cow<'static, str>>, // For #[serde(rename_all_fields)]
    pub(crate) expecting: Option<Cow<'static, str>>,          // For #[serde(expecting)]
}

// New container-level split rename
pub struct NamedFields {
    // Existing fields...
    pub(crate) fields: Vec<(Cow<'static, str>, Field)>,
    
    // NEW FIELDS
    pub(crate) rename_all_serialize: Option<Cow<'static, str>>,
    pub(crate) rename_all_deserialize: Option<Cow<'static, str>>,
}
```

### 2. Macro Attribute Parsing Extensions

Update attribute parsing to capture new attributes:

```rust
// specta-macros/src/type/attr/field.rs
pub struct FieldAttr {
    // Existing...
    pub rename: Option<TokenStream>,
    pub skip: bool,
    
    // NEW
    pub aliases: Vec<String>,
    pub rename_serialize: Option<TokenStream>,
    pub rename_deserialize: Option<TokenStream>,
    pub default_fn: Option<TokenStream>,
    pub skip_serializing: bool,      // Split from skip
    pub skip_deserializing: bool,    // Split from skip
    pub serialize_with: Option<TokenStream>,
    pub deserialize_with: Option<TokenStream>,
    pub with: Option<TokenStream>,
    pub bound: Option<TokenStream>,
    pub borrow: Option<TokenStream>, // Store but potentially ignore
    pub getter: Option<TokenStream>, // For remote types
}

// specta-macros/src/type/attr/variant.rs
pub struct VariantAttr {
    // Existing...
    pub rename: Option<TokenStream>,
    pub skip: bool,
    
    // NEW
    pub aliases: Vec<String>,
    pub rename_serialize: Option<TokenStream>,
    pub rename_deserialize: Option<TokenStream>,
    pub skip_serializing: bool,
    pub skip_deserializing: bool,
    pub serialize_with: Option<TokenStream>,
    pub deserialize_with: Option<TokenStream>,
    pub with: Option<TokenStream>,
    pub bound: Option<TokenStream>,
    pub borrow: Option<TokenStream>,
    pub other: bool,
    pub untagged: bool,
}

// specta-macros/src/type/attr/container.rs
pub struct ContainerAttr {
    // Existing...
    pub rename: Option<TokenStream>,
    pub rename_all: Option<Inflection>,
    
    // NEW
    pub rename_serialize: Option<TokenStream>,
    pub rename_deserialize: Option<TokenStream>,
    pub rename_all_serialize: Option<Inflection>,
    pub rename_all_deserialize: Option<Inflection>,
    pub rename_all_fields: Option<Inflection>,
    pub rename_all_fields_serialize: Option<Inflection>,
    pub rename_all_fields_deserialize: Option<Inflection>,
    pub deny_unknown_fields: bool,
    pub bound: Option<TokenStream>,
    pub default: bool,
    pub default_fn: Option<TokenStream>,
    pub from: Option<TokenStream>,
    pub try_from: Option<TokenStream>,
    pub into: Option<TokenStream>,
    pub expecting: Option<TokenStream>,
    pub variant_identifier: bool,
    pub field_identifier: bool,
}
```

## Implementation Phases

### Phase 1: Core Data Structure Updates (Foundation)
**Priority: HIGH** - Required for all other work

1. **Update `Field` struct** (`specta/src/datatype/fields.rs`)
   - Add new fields for aliases, split renames, default_fn, skip variants, custom_serde
   - Add getters/setters with documentation
   - Update builders to support new fields

2. **Update `EnumVariant` struct** (`specta/src/datatype/enum.rs`)
   - Add variant-level metadata fields
   - Support for `other` and per-variant `untagged`

3. **Update `Struct` struct** (`specta/src/datatype/struct.rs`)
   - Add container-level metadata (deny_unknown_fields, from/try_from/into)

4. **Update `Enum` struct** (`specta/src/datatype/enum.rs`)
   - Add enum-level metadata (rename_all_fields, expecting)

5. **Add `SerdeOverride` enum** (`specta/src/datatype/mod.rs`)
   - New type to track custom serialization functions

**Testing:**
- Add unit tests for all new getters/setters
- Verify backward compatibility with existing code

---

### Phase 2: Macro Attribute Parsing
**Priority: HIGH** - Enables capturing attribute metadata

1. **Extend `FieldAttr`** (`specta-macros/src/type/attr/field.rs`)
   ```rust
   impl_parse! {
       FieldAttr(attr, out) {
           // Existing...
           "rename" => { /* existing */ },
           
           // NEW PARSERS
           "alias" => out.aliases.push(attr.parse_string()?),
           "rename" => {
               if let Some(nested) = attr.parse_nested()? {
                   // Handle rename(serialize = "...", deserialize = "...")
                   for item in nested {
                       match item.key.as_str() {
                           "serialize" => out.rename_serialize = Some(item.parse_string()?.to_token_stream()),
                           "deserialize" => out.rename_deserialize = Some(item.parse_string()?.to_token_stream()),
                       }
                   }
               } else {
                   // Simple rename = "..."
                   out.rename = Some(attr.parse_string()?.to_token_stream())
               }
           },
           "default" => {
               if let Some(path) = attr.try_parse_path()? {
                   out.default_fn = Some(path.to_token_stream());
                   out.optional = true;
               } else {
                   out.optional = attr.parse_bool().unwrap_or(true);
               }
           },
           "skip_serializing" => out.skip_serializing = true,
           "skip_deserializing" => out.skip_deserializing = true,
           "serialize_with" => out.serialize_with = Some(attr.parse_path()?.to_token_stream()),
           "deserialize_with" => out.deserialize_with = Some(attr.parse_path()?.to_token_stream()),
           "with" => out.with = Some(attr.parse_path()?.to_token_stream()),
           "bound" => out.bound = Some(attr.parse_string()?.to_token_stream()),
           "borrow" => out.borrow = Some(attr.parse_string_or_empty()?.to_token_stream()),
           "getter" => out.getter = Some(attr.parse_string()?.to_token_stream()),
       }
   }
   ```

2. **Extend `VariantAttr`** (`specta-macros/src/type/attr/variant.rs`)
   - Similar parsing logic for variant-level attributes
   - Add `other` and per-variant `untagged` support

3. **Extend `ContainerAttr`** (`specta-macros/src/type/attr/container.rs`)
   - Parse container-level attributes
   - Handle nested `rename(serialize = ..., deserialize = ...)`
   - Parse `rename_all_fields` with optional serialize/deserialize split

4. **Update `EnumAttr`** (`specta-macros/src/type/attr/enum.rs`)
   - Already has some logic, extend as needed

**Testing:**
- Add compile-time tests for all new attribute combinations
- Test error cases (invalid syntax, conflicting attributes)

---

### Phase 3: Macro Code Generation Updates
**Priority: HIGH** - Propagates parsed attributes to runtime

1. **Update `construct_field`** (`specta-macros/src/type/field.rs`)
   ```rust
   pub fn construct_field(
       crate_ref: &TokenStream,
       container_attrs: &ContainerAttr,
       field_attrs: FieldAttr,
       field_ty: &Type,
   ) -> TokenStream {
       // Existing logic...
       
       // NEW: Generate aliases
       let aliases = if !field_attrs.aliases.is_empty() {
           let aliases_vec = field_attrs.aliases.iter();
           quote!(vec![#(#aliases_vec.into()),*])
       } else {
           quote!(vec![])
       };
       
       // NEW: Handle split rename
       let rename_serialize = field_attrs.rename_serialize
           .or_else(|| field_attrs.rename.clone());
       let rename_deserialize = field_attrs.rename_deserialize
           .or_else(|| field_attrs.rename.clone());
       
       // NEW: Handle custom serde
       let custom_serde = if let Some(path) = field_attrs.with {
           quote!(Some(datatype::SerdeOverride::With(#path.into())))
       } else if let Some(path) = field_attrs.serialize_with {
           quote!(Some(datatype::SerdeOverride::SerializeWith(#path.into())))
       } else if let Some(path) = field_attrs.deserialize_with {
           quote!(Some(datatype::SerdeOverride::DeserializeWith(#path.into())))
       } else {
           quote!(None)
       };
       
       // Update field construction
       quote! {
           {
               let mut field = internal::construct::field(#ty_def);
               field.set_optional(#optional);
               field.set_flatten(#flatten);
               field.set_aliases(#aliases);
               field.set_rename_serialize(#rename_serialize);
               field.set_rename_deserialize(#rename_deserialize);
               field.set_default_fn(#default_fn);
               field.set_skip_serializing(#skip_serializing);
               field.set_skip_deserializing(#skip_deserializing);
               field.set_custom_serde(#custom_serde);
               // ... existing setters ...
               field
           }
       }
   }
   ```

2. **Update struct generation** (`specta-macros/src/type/struct.rs`)
   - Apply `rename_all_serialize` and `rename_all_deserialize`
   - Handle `deny_unknown_fields`
   - Handle `from`/`try_from`/`into` type conversions

3. **Update enum generation** (`specta-macros/src/type/enum.rs`)
   - Apply `rename_all_fields` to struct variants
   - Handle per-variant `untagged` and `other`
   - Generate variant aliases

**Testing:**
- Integration tests verifying generated code compiles
- Tests comparing generated types against expected output

---

### Phase 4: Validation Layer (`specta-serde`)
**Priority: HIGH** - Ensures correctness at runtime

1. **Extend validation rules** (`specta-serde/src/validate.rs`)
   ```rust
   // NEW validations:
   
   // 1. Validate custom serde usage
   fn validate_custom_serde(field: &Field) -> Result<(), Error> {
       if field.custom_serde().is_some() {
           // DECISION POINT: Should we warn, error, or allow?
           // For now, emit warning that type may not match runtime
           eprintln!("Warning: Field uses custom serialization which may not match exported type");
       }
       Ok(())
   }
   
   // 2. Validate from/try_from/into
   fn validate_type_conversion(s: &Struct) -> Result<(), Error> {
       if s.from_type.is_some() || s.try_from_type.is_some() || s.into_type.is_some() {
           // DECISION POINT: Should we require #[specta(type = ...)] override?
           return Err(Error::TypeConversionRequiresOverride);
       }
       Ok(())
   }
   
   // 3. Validate deny_unknown_fields + flatten
   fn validate_flatten_compat(s: &Struct) -> Result<(), Error> {
       if s.deny_unknown_fields {
           if let Fields::Named(fields) = &s.fields {
               for (_, field) in fields.fields() {
                   if field.flatten() {
                       return Err(Error::DenyUnknownFieldsWithFlatten);
                   }
               }
           }
       }
       Ok(())
   }
   
   // 4. Validate split renames consistency
   fn validate_split_rename(field: &Field) -> Result<(), Error> {
       if field.rename_serialize.is_some() && field.rename_deserialize.is_some() {
           if field.rename_serialize != field.rename_deserialize {
               // DECISION POINT: Warn or error?
               eprintln!("Warning: Field has different serialize/deserialize names");
           }
       }
       Ok(())
   }
   ```

2. **Add new error types** (`specta-serde/src/error.rs`)
   ```rust
   pub enum Error {
       // Existing...
       InvalidMapKey,
       InvalidInternallyTaggedEnum,
       
       // NEW
       DenyUnknownFieldsWithFlatten,
       TypeConversionRequiresOverride,
       ConflictingAttributes,
       InvalidSerdeOverride,
   }
   ```

**Testing:**
- Add tests for each validation rule
- Test error messages are clear and actionable

---

### Phase 5: Language Exporter Updates
**Priority: MEDIUM** - Makes attributes useful in generated code

#### TypeScript Exporter

1. **Handle aliases** (`specta-typescript/src/types.rs`)
   ```typescript
   // Current: { fieldName: string }
   // With aliases: { fieldName | alias1 | alias2: string }
   // OR store as JSDoc comment
   ```

2. **Handle split renames**
   - DECISION: Use serialize name for output types by default
   - Could generate separate input/output types if needed

3. **Handle `deny_unknown_fields`**
   - Could add JSDoc comment indicating strictness
   - Could affect how types are generated (exact types vs loose)

4. **Handle `other` variant**
   - DECISION: Should this be visible or hidden in TS types?
   - Could add as catch-all variant or omit

5. **Update JSDoc generation** (`specta-typescript/src/js_doc.rs`)
   - Include alias information
   - Document custom serde warnings
   - Show default function paths

#### Swift Exporter

1. **Handle aliases** (`specta-swift/src/swift.rs`)
   - Swift doesn't have union property names, store in comments

2. **Handle `deny_unknown_fields`**
   - Could affect Codable implementation

3. **Update documentation generation**

#### Other Exporters
- Similar updates for OpenAPI, Go, Kotlin, etc. as they mature

**Testing:**
- Integration tests comparing generated output
- Tests for each new attribute combination

---

### Phase 6: Documentation & Examples
**Priority: MEDIUM** - Critical for adoption

1. **Update main README** with new attribute support

2. **Add examples** for each new attribute:
   - `examples/serde_alias.rs`
   - `examples/serde_split_rename.rs`
   - `examples/serde_default_fn.rs`
   - `examples/serde_type_conversion.rs`
   - `examples/serde_custom.rs`

3. **Document decision points** and their rationale

4. **Add migration guide** for users

**Testing:**
- Ensure all examples compile and run
- Verify documentation accuracy

---

## Decision Points Requiring Clarification

### 1. Custom Serialization (`serialize_with`/`deserialize_with`/`with`)
**Issue:** These can completely change the runtime type representation.

**Options:**
- **A) Ignore silently** - Current behavior, but inaccurate types
- **B) Require `#[specta(type = ...)]` override** - Explicit but verbose
- **C) Emit warning** - Middle ground, alerts users
- **D) Error by default** - Safest but most restrictive

**Recommendation:** Option C + Option B
- Emit warning when detected
- Allow `#[specta(type = ...)]` to override and silence warning
- Store the function path in metadata for tooling

### 2. Type Conversions (`from`/`try_from`/`into`)
**Issue:** The serialized type differs from the struct definition.

**Options:**
- **A) Auto-detect converted type** - Complex, requires type resolution
- **B) Require manual `#[specta(type = ...)]`** - Simple but manual
- **C) Store conversion info** - Defer to validation layer

**Recommendation:** Option B + Store metadata
- Require explicit override: `#[specta(type = OtherType)]`
- Store from/into paths for potential future automation
- Provide clear error message with example

### 3. Aliases (`alias`)
**Issue:** Multiple valid names during deserialization.

**Options:**
- **A) TypeScript union keys** - `{ name | alias: string }` (invalid TS syntax)
- **B) JSDoc only** - Document but don't affect type
- **C) Ignore** - Not visible in types
- **D) Separate input types** - Generate input and output types

**Recommendation:** Option B (JSDoc) for TypeScript
```typescript
/**
 * @property fieldName - Also accepts: alias1, alias2
 */
export type MyType = {
  fieldName: string;
}
```

### 4. Split Rename (serialize vs deserialize)
**Issue:** Different names for input vs output.

**Options:**
- **A) Error** - Force consistency
- **B) Use serialize name** - For output-focused types
- **C) Generate separate types** - Input and Output variants
- **D) Store both** - Let exporters decide

**Recommendation:** Option D + Option B default
- Store both names in metadata
- TypeScript: Default to serialize name (output types)
- Provide flag to generate separate Input/Output types
- Emit warning about potential confusion

### 5. `default` vs `default = "path"`
**Issue:** Both indicate optional fields.

**Options:**
- **A) Both mark as optional** - Simple, works for most cases
- **B) Store function path** - Enables potential validation generation
- **C) Different handling** - Complex

**Recommendation:** Option A + Option B
- Both mark field as optional in generated types
- Store function path as metadata for future use
- Could enable generating validators with default logic

### 6. `bound` Attribute
**Issue:** Doesn't affect serialized representation.

**Options:**
- **A) Ignore completely** - Simplest
- **B) Validate consistency** - Complex
- **C) Store as metadata** - For future use

**Recommendation:** Option A
- Completely ignore `bound` - it's a compile-time concern
- Document this decision

### 7. `other` Variant
**Issue:** Catch-all for unknown enum values.

**Options:**
- **A) Include in type** - Explicit but may be confusing
- **B) Hide from type** - Cleaner but less accurate
- **C) Optional based on flag** - Flexible

**Recommendation:** Option C
- Add exporter flag `include_other_variants: bool`
- Default to false (hide)
- TypeScript: Could represent as `| { type: "__other__" }` when enabled

### 8. `borrow` and `getter`
**Issue:** Implementation details that don't affect logical types.

**Recommendation:** Ignore completely
- Parse and validate syntax for error checking
- Don't include in DataType representation
- Document as "supported but not exported"

### 9. `deny_unknown_fields`
**Issue:** Validation-only concern.

**Options:**
- **A) Ignore** - Not relevant to types
- **B) Store + validate** - Catch incompatible use with flatten
- **C) Add to JSDoc** - Document behavior

**Recommendation:** Option B + Option C
- Store and validate (error on use with flatten)
- Add JSDoc comment in TypeScript: `@strict - unknown fields not allowed`

### 10. `expecting` and `variant_identifier`/`field_identifier`
**Issue:** Deserialization-specific error messages and behaviors.

**Recommendation:** Store but don't export
- Parse and store as metadata
- Validate syntax
- Not relevant to generated types (deserialization detail)

---

## Implementation Order

### Sprint 1 (Foundation - 2-3 weeks)
1. Phase 1: Core data structure updates
2. Phase 2: Macro attribute parsing
3. Initial tests

### Sprint 2 (Code Generation - 2 weeks)
1. Phase 3: Macro code generation
2. Integration tests
3. Fix any discovered issues

### Sprint 3 (Validation - 1-2 weeks)
1. Phase 4: specta-serde validation
2. Error handling
3. Comprehensive test coverage

### Sprint 4 (Exporters - 2-3 weeks)
1. Phase 5: TypeScript exporter updates
2. Swift exporter updates
3. Integration tests with real-world examples

### Sprint 5 (Polish - 1 week)
1. Phase 6: Documentation and examples
2. Migration guide
3. Release preparation

**Total Estimated Timeline:** 8-11 weeks

---

## Testing Strategy

### Unit Tests
- Every new getter/setter in DataType structs
- Attribute parsing for all combinations
- Validation rules

### Integration Tests
- Real-world struct/enum combinations
- Comparison against expected TypeScript/Swift output
- Serde compatibility validation

### Compile-Time Tests
- Error messages for invalid attributes
- Conflicting attribute detection
- `tests/tests/macro/compile_error.rs` extensions

### Snapshot Tests
- TypeScript output snapshots
- Swift output snapshots
- Ensure no regressions

---

## Migration Path

### Backward Compatibility
- All new fields are additions, no breaking changes to existing API
- Existing code continues to work without modification
- New attributes are opt-in

### Deprecation Strategy
- No deprecations needed (purely additive)

### Version Strategy
- Minor version bump (0.x.y -> 0.x+1.0)
- Note in CHANGELOG all new attribute support

---

## Open Questions & Future Work

1. **Runtime validation generation**
   - Could `default = "path"` enable generating JS validators?
   - Could we generate Zod schemas from this metadata?

2. **Separate Input/Output types**
   - When serialize/deserialize differ significantly
   - Option to generate both `InputMyType` and `OutputMyType`

3. **Custom type overrides**
   - More flexible `#[specta(type = ...)]` syntax
   - Support for generic type parameters

4. **Better error messages**
   - Point to Serde docs for each attribute
   - Suggest fixes for common mistakes

5. **Performance optimization**
   - Ensure attribute parsing doesn't slow compile times
   - Optimize generated code size

---

## Success Criteria

1. ✅ All Serde container attributes parsed and stored
2. ✅ All Serde variant attributes parsed and stored  
3. ✅ All Serde field attributes parsed and stored
4. ✅ specta-serde validation catches incompatible combinations
5. ✅ TypeScript exporter handles new attributes appropriately
6. ✅ Swift exporter handles new attributes appropriately
7. ✅ Comprehensive test coverage (>90%)
8. ✅ Documentation complete with examples
9. ✅ No breaking changes to existing API
10. ✅ Real-world examples validate correctness

---

## Risk Assessment

### High Risk
- **Custom serialization handling** - Complex decision on how to handle
- **Type conversion attributes** - May require significant validation logic

### Medium Risk
- **Split rename support** - Potential confusion in exporters
- **Performance impact** - Adding many fields increases compile time

### Low Risk  
- **Alias support** - Straightforward metadata storage
- **Documentation** - Time-consuming but low technical risk

---

## Notes

- This plan assumes Specta's primary use case is generating **output types** (serialization-focused)
- For full input/output support, would need major changes to generate separate type variants
- Some attributes (like `borrow`) are intentionally ignored as implementation details
- The plan prioritizes accuracy of generated types over feature completeness
- Language exporter updates can be done incrementally as each language matures
