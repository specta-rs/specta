use std::{borrow::Cow, collections::BTreeMap, marker::PhantomData};

use serde::{Deserialize, Serialize};
use specta::{
    Format, Type, Types,
    datatype::{DataType, Enum},
};
use specta_kotlin::{
    Error, IndentStyle, Kotlin, Layout, NamingConvention, Serialization, UnknownType,
};
use tempfile::TempDir;

struct IdentityFormat;

impl Format for IdentityFormat {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &'_ self,
        _: &Types,
        datatype: &DataType,
    ) -> Result<Cow<'_, DataType>, specta::FormatError> {
        Ok(Cow::Owned(datatype.clone()))
    }
}

/// Account information sent to the client.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Account<T> {
    /// Stable account identifier.
    id: u64,
    display_name: String,
    metadata: BTreeMap<String, Option<T>>,
    #[specta(optional)]
    nickname: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "snake_case")]
enum Event<T> {
    Started,
    Message(String),
    Progress { current: u32, total: u32 },
    Generic(T),
}

#[derive(Type)]
#[specta(collect = false)]
struct TupleRecord(u8, String, bool);

#[derive(Type)]
#[specta(collect = false)]
struct NewType(String);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct KotlinxRecord {
    name: String,
    maybe: Option<String>,
}

#[derive(Type)]
#[specta(collect = false)]
struct KotlinxChar {
    value: char,
}

#[derive(Type)]
#[specta(collect = false)]
struct Serializable {
    value: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct SerialName {
    value: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct EncodeDefault {
    value: Option<String>,
}

#[derive(Type)]
#[specta(collect = false)]
struct OptIn {
    value: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct JvmInline(String);

#[derive(Type)]
#[specta(type = Option<String>, collect = false)]
struct NullableAlias;

#[derive(Type)]
#[specta(collect = false)]
struct UsesNullableAlias {
    alias: NullableAlias,
}

#[derive(Type)]
#[specta(collect = false)]
struct KotlinxGenericProperty<T> {
    value: T,
}

#[derive(Type)]
#[specta(collect = false)]
struct KotlinxGenericNewtype<T>(T);

#[derive(Type)]
#[specta(inline, collect = false)]
struct InlineUnit;

#[derive(Type)]
#[specta(inline, collect = false)]
enum InlineUnitEnum {
    First,
    Second,
}

#[derive(Type)]
#[specta(collect = false)]
struct UsesInlineUnit {
    unit: InlineUnit,
}

#[derive(Type)]
#[specta(collect = false)]
struct UsesInlineUnitEnum {
    unit_enum: InlineUnitEnum,
}

#[derive(Type)]
#[specta(collect = false)]
struct KotlinGenericDefault<T = String> {
    value: T,
}

#[derive(Type)]
#[specta(collect = false)]
struct KotlinChainedGenericDefault<T = String, U = T> {
    first: T,
    second: U,
}

#[derive(Type)]
#[specta(collect = false)]
struct UsesKotlinGenericDefaults {
    default: KotlinGenericDefault,
    chained: KotlinChainedGenericDefault<i32>,
}

#[derive(Type)]
#[specta(collect = false)]
enum GenericSkippedVariant<T> {
    Tuple(#[specta(skip)] PhantomData<T>),
    Named {
        #[specta(skip)]
        marker: PhantomData<T>,
    },
    Value(T),
}

#[derive(Type, Serialize)]
#[specta(inline, collect = false)]
enum InlineLiteralPayload {
    Value,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
enum CarriesInlineLiteral {
    Value(InlineLiteralPayload),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum CarriesUntaggedInlineLiteral {
    Value(InlineLiteralPayload),
}

#[derive(Type)]
#[specta(collect = false)]
struct InnerPayload {
    value: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "context", content = "data", rename_all = "camelCase")]
enum ApiError {
    Auth(AuthApiError),
    RateLimited,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "reason", content = "params", rename_all = "camelCase")]
enum AuthApiError {
    CredentialsInvalid,
    SignInRemoved { cause: SignInRemovalCause },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "snake_case")]
enum SignInRemovalCause {
    Remote,
    AccountDeleted,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
enum RouteSessionType {
    Anonymous,
    SignupOnboarding,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum InternallyTagged {
    Empty,
    Named { cause: SignInRemovalCause },
    NewType(InternalPayload),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct InternalPayload {
    value: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedPayload {
    Unit,
    Text(String),
}

/// An all-unit enum with an internal tag still encodes as `{"kind": "..."}`, not a bare
/// string, so it needs the generated serializer rather than a Kotlinx string enum.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AllUnitInternal {
    Empty,
    Second,
}

/// Serde writes nullable payloads as `null` even when it also tolerates them being absent.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind", content = "data")]
enum AdjacentOptional {
    Value(Option<String>),
    Named { maybe: Option<String> },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum DefaultedFields {
    Named {
        #[serde(default)]
        defaulted: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        omitted: Option<u8>,
    },
}

/// A Kotlin keyword as a type name must still be able to own generated helper declarations.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "kind", content = "data")]
enum Package {
    A { value: i32 },
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(rename = "object", tag = "kind", content = "data")]
enum ReservedNameAdjacent {
    A { value: i32 },
}

#[derive(Type)]
#[specta(collect = false)]
enum SelfNamedRawVariant {
    Foo { foo: InnerPayload },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum KeywordEnum {
    #[serde(rename = "when")]
    When,
}

#[derive(Type)]
#[specta(collect = false)]
struct UsesEverything {
    account: Account<String>,
    event: Event<u8>,
    tuple: (String, i32),
    nested: Vec<Option<Account<bool>>>,
    keyword: KeywordEnum,
}

fn raw_types() -> Types {
    Types::default()
        .register::<Account<String>>()
        .register::<Event<String>>()
        .register::<TupleRecord>()
        .register::<NewType>()
        .register::<KeywordEnum>()
        .register::<UsesEverything>()
}

#[test]
fn kotlin_export_raw() {
    insta::assert_snapshot!(
        "kotlin-export-raw",
        Kotlin::default()
            .export(&raw_types(), IdentityFormat)
            .expect("Kotlin export should succeed")
    );
}

#[test]
fn kotlin_raw_enum_keeps_self_named_field() {
    let output = Kotlin::default()
        .export(
            &Types::default().register::<SelfNamedRawVariant>(),
            IdentityFormat,
        )
        .expect("raw enum should export");
    assert!(output.contains("val foo: InnerPayload"));
}

#[test]
fn kotlin_rejects_non_exportable_named_references() {
    let mut types = Types::default();
    let hidden = specta::datatype::NamedDataType::new("Hidden", &mut types, |_, _, _| {});
    specta::datatype::NamedDataType::new("UsesHidden", &mut types, |_, _, datatype| {
        datatype.ty = Some(
            specta::datatype::Struct::named()
                .field(
                    "value",
                    specta::datatype::Field::new(DataType::Reference(hidden.reference(vec![]))),
                )
                .build(),
        );
    });

    let error = Kotlin::default()
        .export(&types, IdentityFormat)
        .expect_err("references without an exported declaration must be rejected");
    assert!(
        error
            .to_string()
            .contains("does not have an exportable definition")
    );
}

#[test]
fn kotlin_rejects_recursive_aliases_and_preserves_literal_payloads() {
    let mut recursive = Types::default();
    specta::datatype::NamedDataType::new(
        "RecursiveAlias",
        &mut recursive,
        |_, self_ref, datatype| {
            datatype.ty = Some(DataType::Reference(self_ref.reference(vec![])));
        },
    );
    let error = Kotlin::default()
        .export(&recursive, IdentityFormat)
        .expect_err("recursive Kotlin aliases must be rejected");
    assert!(error.to_string().contains("recursive Kotlin typealiases"));

    let mut indirect = Types::default();
    let alias_a = specta::datatype::NamedDataType::new("AliasA", &mut indirect, |_, _, _| {});
    let alias_b =
        specta::datatype::NamedDataType::new("AliasB", &mut indirect, |_, _, datatype| {
            datatype.ty = Some(DataType::Reference(alias_a.reference(vec![])));
        });
    indirect.for_each_named_mut(|datatype| {
        if datatype.name == "AliasA" {
            datatype.ty = Some(DataType::Reference(alias_b.reference(vec![])));
        }
    });
    let error = Kotlin::default()
        .export(&indirect, IdentityFormat)
        .expect_err("indirect recursive Kotlin aliases must be rejected");
    assert!(error.to_string().contains("recursive Kotlin typealiases"));

    let output = Kotlin::default()
        .export(
            &Types::default().register::<CarriesInlineLiteral>(),
            specta_serde::Format,
        )
        .expect("a real inline literal payload must survive Serde normalization");
    assert!(output.contains("public data class Value("), "{output}");
    assert!(output.contains("public val value:"), "{output}");

    let output = Kotlin::default()
        .export(
            &Types::default().register::<CarriesUntaggedInlineLiteral>(),
            specta_serde::Format,
        )
        .expect("an untagged inline literal payload must survive Serde normalization");
    assert!(output.contains("public data class Value("), "{output}");
    assert!(output.contains("public val value:"), "{output}");
}

#[test]
fn kotlin_materializes_generic_defaults_and_keeps_empty_variants_generic() {
    let output = Kotlin::default()
        .export(
            &Types::default()
                .register::<UsesKotlinGenericDefaults>()
                .register::<GenericSkippedVariant<String>>(),
            IdentityFormat,
        )
        .expect("defaulted references and skipped generic variants should export");

    assert!(output.contains("val default: KotlinGenericDefault<kotlin.String>"));
    assert!(output.contains("val chained: KotlinChainedGenericDefault<kotlin.Int, kotlin.Int>"));
    assert!(
        output.contains("public class Tuple<T> : GenericSkippedVariant<T>"),
        "{output}"
    );
    assert!(
        output.contains("public class Named<T> : GenericSkippedVariant<T>"),
        "{output}"
    );
}

#[test]
fn kotlin_rejects_generics_that_shadow_their_declaration() {
    use specta::datatype::{GenericDefinition, NamedDataType, Struct};

    let mut types = Types::default();
    NamedDataType::new("Node", &mut types, |_, _, datatype| {
        datatype.generics = Cow::Owned(vec![GenericDefinition::new(Cow::Borrowed("Node"), None)]);
        datatype.ty = Some(DataType::Struct(Struct::unit()));
    });

    assert!(matches!(
        Kotlin::default().export(&types, IdentityFormat),
        Err(Error::DuplicateIdentifier { name, .. }) if name == "Node"
    ));
}

#[test]
fn kotlin_renames_enum_variants_that_shadow_parent_scope() {
    use specta::datatype::{Enum, GenericDefinition, NamedDataType, Variant};

    let mut types = Types::default();
    NamedDataType::new("Collision", &mut types, |_, _, datatype| {
        datatype.generics = Cow::Owned(vec![GenericDefinition::new(Cow::Borrowed("T"), None)]);
        let mut enm = Enum::default();
        for variant in ["Collision", "T", "TVariant"] {
            enm.variants
                .push((Cow::Owned(variant.to_owned()), Variant::unit()));
        }
        datatype.ty = Some(DataType::Enum(enm));
    });

    let output = Kotlin::default()
        .export(&types, IdentityFormat)
        .expect("variant collisions should be renamed without shadowing parent scope");
    assert!(output.contains("class CollisionVariant<T>"));
    assert!(output.contains("class TVariant<T>"));
    assert!(output.contains("class TVariantVariant<T>"));

    let mut root_types = Types::default();
    NamedDataType::new("RootVariants", &mut root_types, |_, _, datatype| {
        let mut enm = Enum::default();
        enm.variants.push((
            Cow::Borrowed("Kotlin"),
            Variant::unnamed()
                .field(specta::datatype::Field::new(DataType::Primitive(
                    specta::datatype::Primitive::str,
                )))
                .build(),
        ));
        datatype.ty = Some(DataType::Enum(enm));
    });
    let output = Kotlin::default()
        .naming(NamingConvention::SnakeCase)
        .export(&root_types, IdentityFormat)
        .expect("root namespace variant names should be renamed");
    assert!(output.contains("class kotlinVariant"));

    let mut keyword_types = Types::default();
    NamedDataType::new("KeywordVariants", &mut keyword_types, |_, _, datatype| {
        datatype.generics = Cow::Owned(vec![GenericDefinition::new(Cow::Borrowed("when"), None)]);
        let mut enm = Enum::default();
        enm.variants.push((Cow::Borrowed("When"), Variant::unit()));
        datatype.ty = Some(DataType::Enum(enm));
    });
    let output = Kotlin::default()
        .naming(NamingConvention::SnakeCase)
        .export(&keyword_types, IdentityFormat)
        .expect("escaped names should participate in variant collision allocation");
    assert!(output.contains("class whenVariant<`when`>"));

    let mut modifier_types = Types::default();
    NamedDataType::new("ModifierVariants", &mut modifier_types, |_, _, datatype| {
        datatype.generics = Cow::Owned(vec![GenericDefinition::new(Cow::Borrowed("out"), None)]);
        let mut enm = Enum::default();
        enm.variants.push((Cow::Borrowed("Out"), Variant::unit()));
        datatype.ty = Some(DataType::Enum(enm));
    });
    let output = Kotlin::default()
        .naming(NamingConvention::SnakeCase)
        .export(&modifier_types, IdentityFormat)
        .expect("generic modifiers should participate in semantic collision allocation");
    assert!(output.contains("class outVariant<`out`>"));
}

#[test]
fn kotlin_rejects_declarations_that_shadow_generic_modifiers() {
    use specta::datatype::{GenericDefinition, NamedDataType, Struct};

    for modifier in ["out", "reified"] {
        let mut types = Types::default();
        let declaration = if modifier == "out" { "Out" } else { modifier };
        NamedDataType::new(declaration, &mut types, |_, _, datatype| {
            datatype.generics = Cow::Owned(vec![GenericDefinition::new(
                Cow::Owned(modifier.to_owned()),
                None,
            )]);
            datatype.ty = Some(DataType::Struct(Struct::unit()));
        });

        let exporter = Kotlin::default().naming(if modifier == "out" {
            NamingConvention::SnakeCase
        } else {
            NamingConvention::Preserve
        });
        assert!(matches!(
            exporter.export(&types, IdentityFormat),
            Err(Error::DuplicateIdentifier { name, .. }) if name == modifier
        ));
    }
}

#[test]
fn kotlin_wraps_bare_generic_aliases_in_value_classes() {
    use specta::datatype::{Field, Generic, GenericDefinition, NamedDataType, Primitive, Struct};

    let mut types = Types::default();
    let generic = Generic::new(Cow::Borrowed("T"));
    let identity = NamedDataType::new("Identity", &mut types, |_, _, datatype| {
        datatype.generics = Cow::Owned(vec![GenericDefinition::new(Cow::Borrowed("T"), None)]);
        datatype.ty = Some(DataType::Generic(generic.clone()));
    });

    let output = Kotlin::default()
        .export(&types, IdentityFormat)
        .expect("bare generic aliases should use a legal Kotlin wrapper");
    assert!(output.contains("value class Identity<T>(public val value: T)"));

    let output = Kotlin::default()
        .mutable_properties(true)
        .export(&types, IdentityFormat)
        .expect("mutable bare generic aliases should use a data class");
    assert!(output.contains("data class Identity<T>(public var value: T)"));

    let error = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .mutable_properties(true)
        .export(&types, IdentityFormat)
        .expect_err("mutable generic wrappers cannot preserve scalar Kotlinx encoding");
    assert!(error.to_string().contains("generic wrappers"));

    NamedDataType::new("UsesIdentity", &mut types, |_, _, datatype| {
        datatype.ty = Some(
            Struct::named()
                .field(
                    "value",
                    Field::new(DataType::Reference(identity.reference(vec![(
                        generic.clone(),
                        DataType::Nullable(Box::new(DataType::Primitive(Primitive::str))),
                    )]))),
                )
                .build(),
        );
    });
    let output = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(&types, IdentityFormat)
        .expect("generic value wrappers are nominal even with nullable type arguments");
    assert!(output.contains("val value: Identity<kotlin.String?>"));
    assert!(!output.contains("val value: Identity<kotlin.String?> = null"));
}

#[test]
fn optional_nullable_fallbacks_do_not_get_duplicate_markers() {
    use specta::datatype::{Field, NamedDataType, Reference, Struct};

    #[derive(Hash, PartialEq, Eq)]
    struct UnsupportedOpaque;

    let mut types = Types::default();
    NamedDataType::new("OpaqueContainer", &mut types, |_, _, datatype| {
        let mut field = Field::new(Reference::opaque(UnsupportedOpaque).into());
        field.optional = true;
        datatype.ty = Some(Struct::named().field("value", field).build());
    });

    let output = Kotlin::default()
        .unknown_types(UnknownType::Any)
        .export(&types, IdentityFormat)
        .expect("optional opaque fallbacks should remain valid nullable Kotlin types");
    assert!(output.contains("val value: kotlin.Any? = null"));
    assert!(!output.contains("kotlin.Any??"));
}

#[test]
fn kotlin_rejects_generics_that_shadow_root_namespaces() {
    use specta::datatype::{GenericDefinition, NamedDataType, Struct};

    let mut types = Types::default();
    NamedDataType::new("GenericRoot", &mut types, |_, _, datatype| {
        datatype.generics = Cow::Owned(vec![GenericDefinition::new(Cow::Borrowed("kotlin"), None)]);
        datatype.ty = Some(DataType::Struct(Struct::unit()));
    });

    assert!(matches!(
        Kotlin::default().export(&types, IdentityFormat),
        Err(Error::ReservedNamespace { name, .. }) if name == "kotlin"
    ));
}

#[test]
fn kotlin_rejects_unknown_and_duplicate_generic_arguments() {
    use specta::datatype::{Field, Generic, GenericDefinition, NamedDataType, Primitive, Struct};

    let generic = Generic::new(Cow::Borrowed("T"));
    let cases = [
        (
            vec![(
                Generic::new(Cow::Borrowed("Unknown")),
                DataType::Primitive(Primitive::i32),
            )],
            "unknown generic argument",
        ),
        (
            vec![
                (generic.clone(), DataType::Primitive(Primitive::i32)),
                (generic.clone(), DataType::Primitive(Primitive::str)),
            ],
            "duplicate generic argument",
        ),
    ];

    for (arguments, expected) in cases {
        let mut types = Types::default();
        let target = NamedDataType::new("GenericTarget", &mut types, |_, _, datatype| {
            datatype.generics = Cow::Owned(vec![GenericDefinition::new(Cow::Borrowed("T"), None)]);
            datatype.ty = Some(
                Struct::named()
                    .field("value", Field::new(DataType::Generic(generic.clone())))
                    .build(),
            );
        });
        NamedDataType::new("GenericUser", &mut types, |_, _, datatype| {
            datatype.ty = Some(
                Struct::named()
                    .field(
                        "value",
                        Field::new(DataType::Reference(target.reference(arguments))),
                    )
                    .build(),
            );
        });

        let error = Kotlin::default()
            .export(&types, IdentityFormat)
            .expect_err("malformed generic arguments must be rejected");
        assert!(error.to_string().contains(expected), "{error}");
    }
}

#[test]
fn kotlin_escapes_generic_modifier_keywords() {
    use specta::datatype::{Field, Generic, GenericDefinition, NamedDataType, Struct};

    let out = Generic::new(Cow::Borrowed("out"));
    let reified = Generic::new(Cow::Borrowed("reified"));
    let mut types = Types::default();
    NamedDataType::new("ContextualGenerics", &mut types, |_, _, datatype| {
        datatype.generics = Cow::Owned(vec![
            GenericDefinition::new(Cow::Borrowed("out"), None),
            GenericDefinition::new(Cow::Borrowed("reified"), None),
        ]);
        datatype.ty = Some(
            Struct::named()
                .field("first", Field::new(DataType::Generic(out)))
                .field("second", Field::new(DataType::Generic(reified)))
                .build(),
        );
    });

    let output = Kotlin::default()
        .export(&types, IdentityFormat)
        .expect("contextual generic modifiers should be escaped");
    assert!(
        output.contains("ContextualGenerics<`out`, `reified`>"),
        "{output}"
    );
    assert!(output.contains("val first: `out`"), "{output}");
    assert!(output.contains("val second: `reified`"), "{output}");
}

#[test]
fn kotlin_export_serde() {
    insta::assert_snapshot!(
        "kotlin-export-serde",
        Kotlin::default()
            .package("dev.specta.generated")
            .export(&raw_types(), specta_serde::Format)
            .expect("Serde-formatted Kotlin export should succeed")
    );
}

/// The reporter's shapes from specta-rs/specta#630: adjacently tagged enums with unit and
/// struct variants, serde renames and an internally tagged enum whose newtype variant must be
/// flattened. `unknown_variants` adds the fallback a newer producer needs.
#[test]
fn kotlin_export_kotlinx_tagged_enums() {
    insta::assert_snapshot!(
        "kotlin-export-kotlinx-tagged-enums",
        Kotlin::default()
            .package("dev.specta.generated")
            .serialization(Serialization::Kotlinx)
            .unknown_variants(true)
            .export(
                &Types::default()
                    .register::<ApiError>()
                    .register::<AuthApiError>()
                    .register::<SignInRemovalCause>()
                    .register::<RouteSessionType>()
                    .register::<InternallyTagged>()
                    .register::<InternalPayload>(),
                specta_serde::Format
            )
            .expect("tagged enums should export with a generated Kotlinx serializer")
    );
}

#[test]
fn kotlin_unknown_variants_are_opt_in() {
    let types = Types::default().register::<ApiError>();

    let plain = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(&types, specta_serde::Format)
        .expect("a tagged enum exports without unknown variants");
    assert!(!plain.contains("data class Unknown"), "{plain}");
    assert!(
        plain.contains(
            "throw kotlinx.serialization.SerializationException(\"Unknown ApiError tag: \" + tag)"
        ),
        "{plain}"
    );

    let fallback = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .unknown_variants(true)
        .export(&types, specta_serde::Format)
        .expect("a tagged enum exports with unknown variants");
    assert!(
        fallback.contains("public data class Unknown(\n        public val tag: kotlin.String,\n        public val content: kotlinx.serialization.json.JsonElement? = null,"),
        "{fallback}"
    );
    assert!(
        fallback
            .contains("else -> ApiError.Unknown(tag, if (payload.isEmpty()) null else payload)"),
        "{fallback}"
    );
}

#[test]
fn kotlin_none_hoists_inline_payloads_and_drops_the_tag() {
    let output = Kotlin::default()
        .export(
            &Types::default()
                .register::<ApiError>()
                .register::<AuthApiError>()
                .register::<SignInRemovalCause>()
                .register::<RouteSessionType>(),
            specta_serde::Format,
        )
        .expect("plain declarations should model every payload field");

    // An adjacent struct payload is a named class instead of `Map<String, Any?>`.
    assert!(
        output.contains(
            "public data class AuthApiErrorSignInRemovedData(\n    public val cause: SignInRemovalCause\n)"
        ),
        "{output}"
    );
    assert!(
        output.contains("public val params: AuthApiErrorSignInRemovedData"),
        "{output}"
    );
    assert!(
        !output.contains("Map<kotlin.String, kotlin.Any?>"),
        "{output}"
    );

    // The variant name already carries the tag, so it is not repeated as a field.
    assert!(!output.contains("public val reason:"), "{output}");
    assert!(!output.contains("public val context:"), "{output}");

    // Serde's wire spelling is recorded for plain enums.
    assert!(
        output.contains(
            "public enum class SignInRemovalCause(public val wireValue: kotlin.String) {\n    Remote(\"remote\"),\n    AccountDeleted(\"account_deleted\")\n}"
        ),
        "{output}"
    );
    assert!(
        output.contains(
            "public enum class RouteSessionType(public val wireValue: kotlin.String) {\n    Anonymous(\"anonymous\"),\n    SignupOnboarding(\"signupOnboarding\")\n}"
        ),
        "{output}"
    );
}

#[test]
fn kotlin_kotlinx_tagged_unit_enums_keep_their_object_envelope() {
    let output = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<AllUnitInternal>(),
            specta_serde::Format,
        )
        .expect("an all-unit internally tagged enum still encodes as an object");

    // A Kotlinx string enum would write `"empty"`, while Serde writes `{"kind":"empty"}`.
    assert!(
        output.contains(
            "@kotlinx.serialization.Serializable(with = AllUnitInternalSerializer::class)"
        ),
        "{output}"
    );
    assert!(
        output.contains("public object AllUnitInternalSerializer"),
        "{output}"
    );
    assert!(
        output.contains("put(\"kind\", kotlinx.serialization.json.JsonPrimitive(\"empty\"))"),
        "{output}"
    );
    assert!(
        !output.contains("public enum class AllUnitInternal"),
        "{output}"
    );

    // Externally tagged all-unit enums are bare strings, so they stay Kotlinx string enums.
    let external = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<SignInRemovalCause>(),
            specta_serde::Format,
        )
        .expect("an externally tagged all-unit enum is a string enum");
    assert!(
        external.contains("public enum class SignInRemovalCause"),
        "{external}"
    );
}

#[test]
fn kotlin_kotlinx_keeps_defaulted_optionals_on_the_wire() {
    let output = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default()
                .register::<AdjacentOptional>()
                .register::<DefaultedFields>(),
            specta_serde::Format,
        )
        .expect("optional nullable payloads must stay encodable");

    // Serde writes a nil `Option` as `null`, even where it also accepts a missing key: a
    // nullable adjacent payload and a `#[serde(default)]` field both need `@EncodeDefault`.
    for property in [
        "@kotlinx.serialization.EncodeDefault\n        public val data: kotlin.String? = null",
        "@kotlinx.serialization.EncodeDefault\n    public val maybe: kotlin.String? = null",
        "@kotlinx.serialization.EncodeDefault\n        public val defaulted: kotlin.UByte? = null",
    ] {
        assert!(output.contains(property), "{output}");
    }

    // `skip_serializing_if` really may omit the key, so it keeps the Kotlin default.
    assert!(
        output.contains("\n        public val omitted: kotlin.UByte? = null"),
        "{output}"
    );
    assert!(
        !output.contains("@kotlinx.serialization.EncodeDefault\n        public val omitted"),
        "{output}"
    );
}

#[test]
fn kotlin_backticked_names_do_not_break_generated_declarations() {
    let kotlinx = Kotlin::default()
        .naming(NamingConvention::SnakeCase)
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<Package>(),
            specta_serde::Format,
        )
        .expect("a keyword type name must still be able to own a serializer");
    assert!(
        kotlinx.contains("public sealed interface `package`"),
        "{kotlinx}"
    );
    assert!(
        kotlinx.contains("public object packageSerializer"),
        "{kotlinx}"
    );
    assert!(kotlinx.contains("packageAData"), "{kotlinx}");

    let plain = Kotlin::default()
        .export(
            &Types::default().register::<ReservedNameAdjacent>(),
            specta_serde::Format,
        )
        .expect("a keyword type name must still be able to own a hoisted payload");
    assert!(
        plain.contains("public sealed interface `object`"),
        "{plain}"
    );
    assert!(plain.contains("public data class objectAData("), "{plain}");
}

#[test]
fn kotlin_kotlinx_rejects_representations_without_a_discriminator() {
    let untagged = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<UntaggedPayload>(),
            specta_serde::Format,
        )
        .expect_err("untagged enums have no tag to dispatch on");
    assert!(untagged.to_string().contains("untagged"), "{untagged}");

    let generic = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<Event<String>>(),
            specta_serde::Format,
        )
        .expect_err("a generic enum needs a per-instantiation serializer");
    assert!(generic.to_string().contains("generic enums"), "{generic}");
}

#[test]
fn kotlin_configuration() {
    let output = Kotlin::new()
        .header("// custom")
        .without_package()
        .indent(IndentStyle::Tabs)
        .naming(NamingConvention::SnakeCase)
        .serialization(Serialization::None)
        .mutable_properties(true)
        .with_raw("public const val generated = true")
        .export(
            &Types::default().register::<UsesEverything>(),
            IdentityFormat,
        )
        .expect("configured Kotlin export should succeed");

    assert!(output.starts_with("// custom"));
    assert!(output.contains("public data class uses_everything"));
    assert!(output.contains("public var account:"));
    assert!(!output.contains("@Serializable"));
    assert!(output.contains("public const val generated = true"));
}

#[test]
fn kotlinx_is_opt_in_and_rejects_incompatible_wire_shapes() {
    let supported = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<KotlinxRecord>(),
            specta_serde::Format,
        )
        .expect("plain records have compatible Kotlinx declarations");
    assert!(supported.contains("@kotlinx.serialization.Serializable"));
    assert!(supported.contains("@kotlinx.serialization.EncodeDefault"));
    assert!(supported.contains("val maybe: kotlin.String? = null"));

    let supported = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(&Types::default().register::<KotlinxChar>(), IdentityFormat)
        .expect("Rust char should use the wire-compatible Kotlinx string representation");
    assert!(supported.contains("val value: kotlin.String"));

    let supported = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<UsesNullableAlias>(),
            IdentityFormat,
        )
        .expect("nullable aliases should retain Kotlinx missing-field defaults");
    assert!(supported.contains("typealias NullableAlias = kotlin.String?"));
    assert!(supported.contains("@kotlinx.serialization.EncodeDefault"));
    assert!(supported.contains("val alias: NullableAlias = null"));

    let error = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<KotlinxGenericProperty<Option<String>>>(),
            IdentityFormat,
        )
        .expect_err("Kotlinx generic nullability depends on the instantiation");
    assert!(error.to_string().contains("unconstrained generic"));

    let supported = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<KotlinxGenericNewtype<Option<String>>>(),
            IdentityFormat,
        )
        .expect("generic Kotlinx value classes preserve scalar encoding");
    assert!(supported.contains("value class KotlinxGenericNewtype<T>"));
    assert!(supported.contains("public val value: T"));

    let supported = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default()
                .register::<Serializable>()
                .register::<SerialName>()
                .register::<EncodeDefault>()
                .register::<OptIn>()
                .register::<JvmInline>(),
            IdentityFormat,
        )
        .expect("generated names must not shadow fully qualified Kotlinx annotations");
    assert!(!supported.contains("import kotlinx.serialization"));
    assert!(!supported.contains("InternalSerializationApi"));
    assert!(supported.contains("@file:kotlin.OptIn"));
    assert!(supported.contains("@kotlinx.serialization.Serializable"));
    assert!(supported.contains("@kotlinx.serialization.EncodeDefault"));
    assert!(supported.contains("@kotlin.jvm.JvmInline"));

    let mut deprecated_types = Types::default().register::<OptIn>();
    deprecated_types.for_each_named_mut(|datatype| {
        datatype.deprecated = Some(specta::datatype::Deprecated::new());
    });
    let supported = Kotlin::default()
        .export(&deprecated_types, IdentityFormat)
        .expect("deprecation annotations should be namespace-safe");
    assert!(supported.contains("@kotlin.Deprecated"));

    let supported = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<KeywordEnum>(),
            specta_serde::Format,
        )
        .expect("unit-only enums encode exactly like a Kotlinx string enum");
    assert!(
        supported.contains("public enum class KeywordEnum"),
        "{supported}"
    );
    assert!(
        supported.contains("@kotlinx.serialization.SerialName(\"when\")"),
        "{supported}"
    );

    let supported = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default()
                .register::<SelfNamedRawVariant>()
                .register::<InnerPayload>(),
            specta_serde::Format,
        )
        .expect("payload enums require and receive a representation-aware custom serializer");
    assert!(
        supported.contains(
            "@kotlinx.serialization.Serializable(with = SelfNamedRawVariantSerializer::class)"
        ),
        "{supported}"
    );
    assert!(
        supported.contains("public object SelfNamedRawVariantSerializer"),
        "{supported}"
    );

    let mutable_newtype = Kotlin::default()
        .mutable_properties(true)
        .export(&Types::default().register::<NewType>(), IdentityFormat)
        .expect("mutable newtype should export as a regular data class");
    assert!(mutable_newtype.contains("data class NewType"));
    assert!(mutable_newtype.contains("public var field0: kotlin.String"));
    assert!(!mutable_newtype.contains("@JvmInline"));

    let error = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .mutable_properties(true)
        .export(&Types::default().register::<NewType>(), IdentityFormat)
        .expect_err("mutable Kotlinx newtypes cannot preserve scalar encoding");
    assert!(error.to_string().contains("mutable Kotlinx newtypes"));

    let error = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<UsesInlineUnit>(),
            IdentityFormat,
        )
        .expect_err("inline unit structs must not bypass Kotlinx wire-shape validation");
    assert!(error.to_string().contains("unit-struct null encoding"));

    let error = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<UsesInlineUnitEnum>(),
            IdentityFormat,
        )
        .expect_err("inline unit enums must not bypass Kotlinx wire-shape validation");
    assert!(error.to_string().contains("structural union"));
}

#[test]
fn kotlin_qualifies_builtins_that_generated_types_can_shadow() {
    let mut types = Types::default();
    specta::datatype::NamedDataType::new("String", &mut types, |_, _, datatype| {
        datatype.ty = Some(
            specta::datatype::Struct::named()
                .field(
                    "value",
                    specta::datatype::Field::new(DataType::Primitive(
                        specta::datatype::Primitive::str,
                    )),
                )
                .build(),
        );
    });

    let output = Kotlin::default()
        .export(&types, IdentityFormat)
        .expect("generated built-in names should not shadow Kotlin types");
    assert!(output.contains("data class String"));
    assert!(output.contains("val value: kotlin.String"));
}

#[test]
fn kotlin_rejects_names_that_shadow_root_namespaces() {
    for name in ["kotlin", "kotlinx", "java"] {
        let mut types = Types::default();
        specta::datatype::NamedDataType::new(name, &mut types, |_, _, datatype| {
            datatype.ty = Some(DataType::Struct(specta::datatype::Struct::unit()));
        });
        assert!(matches!(
            Kotlin::default().export(&types, IdentityFormat),
            Err(Error::ReservedNamespace { name: actual, .. }) if actual == name
        ));
    }

    assert!(matches!(
        Kotlin::default()
            .package("kotlin.generated")
            .export(&Types::default().register::<Account<String>>(), IdentityFormat),
        Err(Error::ReservedNamespace { name, .. }) if name == "kotlin"
    ));
}

#[test]
fn kotlin_files_layout() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&root).expect("temporary workspace directory should be creatable");
    let temp = TempDir::new_in(root).expect("temporary export directory should be creatable");
    let exporter = Kotlin::default()
        .layout(Layout::Files)
        .with_raw("public const val generated = true");

    assert!(matches!(
        exporter.export(&raw_types(), IdentityFormat),
        Err(Error::ExportRequiresExportTo(Layout::Files))
    ));
    exporter
        .export_to(temp.path(), &raw_types(), IdentityFormat)
        .expect("file layout should export");

    assert!(temp.path().join("Account.kt").is_file());
    assert!(temp.path().join("Event.kt").is_file());
    assert!(temp.path().join("Specta.kt").is_file());
    let account = std::fs::read_to_string(temp.path().join("Account.kt"))
        .expect("generated Kotlin file should be readable");
    assert!(account.contains("data class Account<T>"));
    assert!(!account.contains("data class Event<T>"));

    std::fs::write(temp.path().join("UserOwned.kt"), "// user owned")
        .expect("unrelated Kotlin file should be creatable");
    exporter
        .export_to(
            temp.path(),
            &Types::default().register::<Account<String>>(),
            IdentityFormat,
        )
        .expect("subsequent file export should clean stale generated files");
    assert!(!temp.path().join("Event.kt").exists());
    assert!(temp.path().join("UserOwned.kt").exists());
}

#[test]
fn kotlin_module_prefixed_layout_updates_declarations_and_references() {
    let output = Kotlin::default()
        .layout(Layout::ModulePrefixedName)
        .export(
            &Types::default()
                .register::<Account<String>>()
                .register::<UsesEverything>(),
            IdentityFormat,
        )
        .expect("module-prefixed layout should export");

    assert!(output.contains("data class TestKotlinAccount<T>"));
    assert!(output.contains("val account: TestKotlinAccount<kotlin.String>"));
}

#[test]
fn kotlin_export_to_creates_parent_directories() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&root).expect("temporary workspace directory should be creatable");
    let temp = TempDir::new_in(root).expect("temporary export directory should be creatable");
    let output = temp.path().join("nested/generated/Bindings.kt");

    Kotlin::default()
        .export_to(&output, &raw_types(), IdentityFormat)
        .expect("flat export should create parent directories");
    assert!(output.is_file());
}

#[test]
fn kotlin_rejects_unsafe_filenames_and_member_collisions() {
    let mut unsafe_types = Types::default();
    specta::datatype::NamedDataType::new("../../escape", &mut unsafe_types, |_, _, datatype| {
        datatype.ty = Some(DataType::Struct(specta::datatype::Struct::unit()));
    });
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&root).expect("temporary workspace directory should be creatable");
    let temp = TempDir::new_in(root).expect("temporary export directory should be creatable");
    assert!(matches!(
        Kotlin::default().layout(Layout::Files).export_to(
            temp.path(),
            &unsafe_types,
            IdentityFormat
        ),
        Err(Error::InvalidIdentifier { .. })
    ));

    let mut collision_types = Types::default();
    specta::datatype::NamedDataType::new("Collision", &mut collision_types, |_, _, datatype| {
        datatype.ty = Some(
            specta::datatype::Struct::named()
                .field(
                    "foo_bar",
                    specta::datatype::Field::new(DataType::Primitive(
                        specta::datatype::Primitive::bool,
                    )),
                )
                .field(
                    "fooBar",
                    specta::datatype::Field::new(DataType::Primitive(
                        specta::datatype::Primitive::bool,
                    )),
                )
                .build(),
        );
    });
    assert!(matches!(
        Kotlin::default()
            .naming(NamingConvention::CamelCase)
            .export(&collision_types, IdentityFormat),
        Err(Error::DuplicateIdentifier { .. })
    ));
}

#[test]
fn kotlin_exports_shared_type_corpus() {
    let (mut types, _) = crate::types();
    // `crate::types()` adds a synthetic `Primitives` record whose stringify-based field names
    // intentionally contain duplicates. It is useful to TypeScript's structural snapshots but
    // cannot be a Kotlin constructor, so exercise the actual collected named corpus here.
    types.for_each_named_mut(|datatype| {
        if datatype.name == "Primitives" {
            datatype.ty = None;
        }
    });
    let exporter = Kotlin::default().serialization(Serialization::None);
    exporter
        .export(&types, IdentityFormat)
        .expect("the raw shared datatype corpus should export to Kotlin");
    let error = exporter
        .export(&types, specta_serde::Format)
        .expect_err("the Serde corpus contains a recursive alias Kotlin must reject");
    assert!(error.to_string().contains("recursive Kotlin typealiases"));

    let (phased_types, _) = crate::types_phased();
    exporter
        .export(&phased_types, specta_serde::PhasesFormat)
        .expect("the phased Serde shared datatype corpus should export to Kotlin");
}

/// The datatype `ty!([])` builds: a list whose element type is uninhabited, which Specta
/// represents as an enum with no variants. `std::convert::Infallible` is the same shape.
struct Uninhabited;

impl Type for Uninhabited {
    fn definition(_: &mut Types) -> DataType {
        DataType::Enum(Enum::default())
    }
}

#[derive(Type)]
#[specta(collect = false)]
struct HoldsUninhabited {
    field: Vec<Uninhabited>,
}

/// `kotlin.Never` describes an uninhabited enum. The unit-only arm below it must not be
/// reached, since `all(..)` is vacuously true for an empty variant list and would report
/// it as a string enum.
#[test]
fn kotlin_renders_an_uninhabited_element_type_as_never() {
    let output = Kotlin::default()
        .export(
            &Types::default().register::<HoldsUninhabited>(),
            IdentityFormat,
        )
        .expect("an uninhabited element type should export");
    assert!(output.contains("kotlin.Never"), "{output}");
}
