use std::{borrow::Cow, error, fmt, io, panic::Location, path::PathBuf};

use specta::datatype::{NamedDataType, OpaqueReference, RecursiveInlineType};

use crate::Layout;

/// The error type for the TypeScript exporter.
///
/// ## BigInt Forbidden
///
/// Specta refuses to export [usize], [isize], [i64], [u64], [i128], [u128] and [f128] to TypeScript. `JSON.parse` rounds integers past `Number.MAX_SAFE_INTEGER` (2^53 - 1) to the nearest double without an error, so `9007199254740993` reaches JavaScript as `9007199254740992`. The error names the field that holds one; pick the fix that matches its values:
///
/// | Values | Fix |
/// | --- | --- |
/// | fit in 32 bits | use `u32` or `i32` |
/// | must stay exact past 2^53, like snowflake ids or hashes | [send them as strings](#send-the-value-as-a-string) |
/// | may round past 2^53 in one field | mark that field `#[specta(type = specta_typescript::Number)]`, which records the accepted loss where reviewers see it |
/// | sit inside a `serde_json::Value` | [type its numbers as `number`](#serde_jsonvalue) |
/// | cross a Tauri command or event | [use Tauri Specta's builder options](#tauri-specta) |
/// | may round past 2^53 in any field | [remap every integer](#every-integer-at-once), as a last resort |
///
/// ### Send the value as a string
///
/// A string carries any integer exactly. Serialize the field with a codec that writes a string, and declare that wire type with `#[specta(type = String)]`:
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use specta::Type;
///
/// #[derive(Serialize, Deserialize, Type)]
/// pub struct Order {
///     #[serde(with = "string")]
///     #[specta(type = String)]
///     pub id: u64,
/// }
///
/// /// Writes a value with `Display` and reads it back with `FromStr`.
/// mod string {
///     use std::{fmt::Display, str::FromStr};
///
///     use serde::{Deserialize, Deserializer, Serializer, de::Error};
///
///     pub fn serialize<T: Display, S: Serializer>(value: &T, serializer: S) -> Result<S::Ok, S::Error> {
///         serializer.collect_str(value)
///     }
///
///     pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
///     where
///         T: FromStr<Err: Display>,
///         D: Deserializer<'de>,
///     {
///         String::deserialize(deserializer)?.parse().map_err(D::Error::custom)
///     }
/// }
/// # let types = specta::Types::default().register::<Order>();
/// # let output = specta_typescript::Typescript::default().export(&types, specta_serde::Format).unwrap();
/// # assert!(output.contains("id: string"));
/// ```
///
/// The frontend receives `id` as a `string`, and `BigInt(order.id)` restores the exact value.
///
/// ### `serde_json::Value`
///
/// `serde_json::Number` holds `i64` and `u64`, so any type containing a `serde_json::Value` hits this error at `serde_json::Value`. `JSON.parse` already turns those numbers into JavaScript numbers, so one remap rule can type them as `number` and leave every other integer guarded:
///
/// ```rust
/// use specta::{Type, Types};
/// use specta_util::Remapper;
///
/// #[derive(Type)]
/// pub struct Event {
///     pub payload: serde_json::Value,
/// }
///
/// let types = Remapper::new()
///     .rule(
///         <serde_json::Number as Type>::definition(&mut Types::default()),
///         <specta_typescript::Number as Type>::definition(&mut Types::default()),
///     )
///     .remap_types(Types::default().register::<Event>());
/// # let output = specta_typescript::Typescript::default().export(&types, specta_serde::Format).unwrap();
/// # assert!(output.contains("null | boolean | number | string"));
/// ```
///
/// Integers past 2^53 inside the value can still round. If the frontend treats the value as opaque, mark the field `#[specta(type = specta_typescript::Unknown)]` instead.
///
/// ### Tauri Specta
///
/// Tauri Specta's `Builder` has two options:
///
/// - `dangerously_cast_bigints_to_number()` exports them as `number` in every command and event, accepting the loss.
/// - `semantic_types` with [`semantic::Configuration::enable_lossless_bigints`](crate::semantic::Configuration::enable_lossless_bigints) types them as `bigint` and wraps values from Rust in `BigInt(value)`. The result is exact only for values that reach JavaScript as strings, because a JSON number past 2^53 may arrive already rounded ([#203](https://github.com/specta-rs/specta/issues/203#issuecomment-4387573925)).
///
/// ### Every integer at once
///
/// [`Remapper::dangerous_bigints_as_number`](https://docs.rs/specta-util/latest/specta_util/struct.Remapper.html#method.dangerous_bigints_as_number) exports every one of these integers in a collection as `number`. It accepts the loss for every field, including fields added later, so reach for it last ([#481](https://github.com/specta-rs/specta/issues/481)):
///
/// ```rust
/// # let types = specta::Types::default();
/// let types = specta_util::Remapper::new()
///     .dangerous_bigints_as_number()
///     .remap_types(types);
/// # let _ = types;
/// ```
///
/// ### Migrating from `BigIntExportBehavior`
///
/// Specta 2.0.0-rc.23 and earlier configured this with `Typescript::bigint(BigIntExportBehavior::…)`, which rc.24 removed:
///
/// | `BigIntExportBehavior` | Replacement |
/// | --- | --- |
/// | `Number` | [remap every integer](#every-integer-at-once), or Tauri Specta's `dangerously_cast_bigints_to_number()` |
/// | `String` | [send the value as a string](#send-the-value-as-a-string) |
/// | `BigInt` | `#[specta(type = specta_typescript::BigInt)]` on each field your runtime parses into `bigint`, or Tauri Specta's `semantic_types` |
/// | `Fail` | nothing: this error is the default |
#[non_exhaustive]
pub struct Error {
    kind: ErrorKind,
    named_datatype: Option<Box<NamedDataType>>,
    trace: Vec<ErrorTraceFrame>,
}

/// Additional TypeScript exporter context for an [`Error`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ErrorTraceFrame {
    /// The exporter was rendering a core-provided inline reference at `path` when the error occurred.
    Inlined {
        /// The named Rust type being inlined, if it could be resolved.
        named_datatype: Option<Box<NamedDataType>>,
        /// Field, variant, or variant-field path where the inline expansion occurred.
        path: String,
    },
}

type FrameworkSource = Box<dyn error::Error + Send + Sync + 'static>;
const BIGINT_DOCS_URL: &str =
    "https://docs.rs/specta-typescript/latest/specta_typescript/struct.Error.html#bigint-forbidden";

#[allow(dead_code)]
enum ErrorKind {
    /// A map key type cannot be represented as a valid Typescript index signature.
    ///
    /// Typescript map keys must resolve to string-like, number-like, symbol-like, or literal key
    /// types. Complex structural values cannot safely be represented as object keys.
    InvalidMapKey {
        path: String,
        reason: Cow<'static, str>,
    },
    /// Attempted to export a bigint type but the configuration forbids it.
    BigIntForbidden {
        path: String,
    },
    /// A type's name conflicts with a reserved keyword in Typescript.
    ForbiddenName {
        path: String,
        name: &'static str,
    },
    /// A type's name contains invalid characters or is not valid.
    InvalidName {
        path: String,
        name: Cow<'static, str>,
    },
    /// A type's name is empty and cannot be emitted as a Typescript type name.
    EmptyName {
        path: String,
    },
    /// Anonymous enum variants cannot be represented by the Typescript exporter.
    UnsupportedAnonymousEnumVariant {
        path: String,
        variant_kind: &'static str,
    },
    /// Detected multiple items within the same scope with the same name.
    /// Typescript doesn't support this so we error out.
    ///
    /// Using anything other than [Layout::FlatFile] should make this basically impossible.
    DuplicateTypeName {
        name: Cow<'static, str>,
        first: String,
        second: String,
    },
    /// An filesystem IO error.
    /// This is possible when using `Typescript::export_to` when writing to a file or formatting the file.
    Io(io::Error),
    /// Failed to read a directory while exporting files.
    ReadDir {
        path: PathBuf,
        source: io::Error,
    },
    /// Failed to inspect filesystem metadata while exporting files.
    Metadata {
        path: PathBuf,
        source: io::Error,
    },
    /// The files layout requires a directory, but the output path is an
    /// ordinary file. That file is left untouched rather than deleted.
    ExpectedDirectory {
        path: PathBuf,
    },
    /// Failed to remove a stale file while exporting files.
    RemoveFile {
        path: PathBuf,
        source: io::Error,
    },
    /// Failed to remove an empty directory while exporting files.
    RemoveDir {
        path: PathBuf,
        source: io::Error,
    },
    /// Failed to create an output directory while exporting files.
    CreateDir {
        path: PathBuf,
        source: io::Error,
    },
    /// Failed to write an output file while exporting files.
    WriteFile {
        path: PathBuf,
        source: io::Error,
    },
    /// Failed to read a generated file while exporting files.
    ReadFile {
        path: PathBuf,
        source: io::Error,
    },
    /// Found an opaque reference which the Typescript exporter doesn't know how to handle.
    /// You may be referencing a type which is not supported by the Typescript exporter.
    UnsupportedOpaqueReference {
        path: String,
        reference: OpaqueReference,
    },
    /// Found a named reference that cannot be resolved from the provided
    /// [`Types`](specta::Types).
    DanglingNamedReference {
        path: String,
        reference: String,
    },
    /// Found a recursive named reference marked by core inline resolution.
    InfiniteRecursiveInlineType {
        path: String,
        reference: String,
        cycle: RecursiveInlineType,
    },
    /// Reached the recursion limit while rendering an anonymous Typescript type.
    InlineRecursionLimitExceeded {
        path: String,
    },
    /// An error occurred in your exporter framework.
    Framework {
        message: Cow<'static, str>,
        source: FrameworkSource,
    },
    /// An error occurred in a format callback.
    Format {
        message: Cow<'static, str>,
        path: Option<String>,
        source: FrameworkSource,
    },
    /// The requested export layout is not supported by the current exporter configuration.
    ///
    /// Some layouts require the higher-level [`Exporter`](crate::Exporter) APIs so imports,
    /// file paths, and module boundaries can be coordinated correctly.
    ExportRequiresExportTo(Layout),
    JsdocNamespacesUnsupported,
    /// The type is part of a recursive type-alias cycle (e.g. through
    /// `#[serde(untagged)]` newtype variants) that TypeScript rejects
    /// (TS2456) and that the exporter could not collapse into an
    /// equivalent non-recursive alias.
    UnrepresentableAliasCycle {
        path: String,
        reason: Cow<'static, str>,
    },
    /// A runtime semantic transform is required beneath a construct whose
    /// wire shape cannot be discriminated without guessing (for example an
    /// overlapping `#[serde(untagged)]` enum). Emitting a transform here
    /// would risk the exported type and the runtime value diverging, so the
    /// request is rejected instead.
    UnsupportedSemanticTransform {
        reason: Cow<'static, str>,
    },
}

impl Error {
    fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            named_datatype: None,
            trace: Vec::new(),
        }
    }

    /// The named Rust type being exported when this error occurred, if known.
    pub fn named_datatype(&self) -> Option<&NamedDataType> {
        self.named_datatype.as_deref()
    }

    /// TypeScript exporter traversal context for this error.
    pub fn trace(&self) -> &[ErrorTraceFrame] {
        &self.trace
    }

    pub(crate) fn with_named_datatype(mut self, ndt: &NamedDataType) -> Self {
        self.named_datatype
            .get_or_insert_with(|| Box::new(ndt.clone()));
        self
    }

    pub(crate) fn with_inline_trace(
        mut self,
        ndt: Option<&NamedDataType>,
        path: impl Into<String>,
    ) -> Self {
        self.trace.push(ErrorTraceFrame::Inlined {
            named_datatype: ndt.map(|ndt| Box::new(ndt.clone())),
            path: path.into(),
        });
        self
    }

    pub(crate) fn invalid_map_key(
        path: impl Into<String>,
        reason: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(ErrorKind::InvalidMapKey {
            path: path.into(),
            reason: reason.into(),
        })
    }

    /// Construct an error for framework-specific logic.
    pub fn framework(
        message: impl Into<Cow<'static, str>>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::new(ErrorKind::Framework {
            message: message.into(),
            source: source.into(),
        })
    }

    /// Construct an error for custom format callbacks.
    pub(crate) fn format(
        message: impl Into<Cow<'static, str>>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::new(ErrorKind::Format {
            message: message.into(),
            path: None,
            source: source.into(),
        })
    }

    pub(crate) fn format_at(
        message: impl Into<Cow<'static, str>>,
        path: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::new(ErrorKind::Format {
            message: message.into(),
            path: Some(path.into()),
            source: source.into(),
        })
    }

    pub(crate) fn bigint_forbidden(path: String) -> Self {
        Self::new(ErrorKind::BigIntForbidden { path })
    }

    pub(crate) fn invalid_name(path: String, name: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::InvalidName {
            path,
            name: name.into(),
        })
    }

    pub(crate) fn empty_name(path: String) -> Self {
        Self::new(ErrorKind::EmptyName { path })
    }

    pub(crate) fn unsupported_anonymous_enum_variant(
        path: String,
        variant_kind: &'static str,
    ) -> Self {
        Self::new(ErrorKind::UnsupportedAnonymousEnumVariant { path, variant_kind })
    }

    pub(crate) fn forbidden_name(path: String, name: &'static str) -> Self {
        Self::new(ErrorKind::ForbiddenName { path, name })
    }

    pub(crate) fn duplicate_type_name(
        name: Cow<'static, str>,
        first: Location<'static>,
        second: Location<'static>,
    ) -> Self {
        Self::new(ErrorKind::DuplicateTypeName {
            name,
            first: format_location(first),
            second: format_location(second),
        })
    }

    pub(crate) fn read_dir(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::ReadDir { path, source })
    }

    pub(crate) fn metadata(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::Metadata { path, source })
    }

    pub(crate) fn expected_directory(path: PathBuf) -> Self {
        Self::new(ErrorKind::ExpectedDirectory { path })
    }

    pub(crate) fn remove_file(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::RemoveFile { path, source })
    }

    pub(crate) fn remove_dir(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::RemoveDir { path, source })
    }

    pub(crate) fn create_dir(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::CreateDir { path, source })
    }

    pub(crate) fn write_file(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::WriteFile { path, source })
    }

    pub(crate) fn read_file(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::ReadFile { path, source })
    }

    pub(crate) fn unsupported_opaque_reference(path: String, reference: OpaqueReference) -> Self {
        Self::new(ErrorKind::UnsupportedOpaqueReference { path, reference })
    }

    pub(crate) fn dangling_named_reference(path: String, reference: String) -> Self {
        Self::new(ErrorKind::DanglingNamedReference { path, reference })
    }

    pub(crate) fn infinite_recursive_inline_type(
        path: String,
        reference: String,
        cycle: RecursiveInlineType,
    ) -> Self {
        Self::new(ErrorKind::InfiniteRecursiveInlineType {
            path,
            reference,
            cycle,
        })
    }

    pub(crate) fn inline_recursion_limit_exceeded(path: String) -> Self {
        Self::new(ErrorKind::InlineRecursionLimitExceeded { path })
    }

    pub(crate) fn export_requires_export_to(layout: Layout) -> Self {
        Self::new(ErrorKind::ExportRequiresExportTo(layout))
    }

    pub(crate) fn jsdoc_namespaces_unsupported() -> Self {
        Self::new(ErrorKind::JsdocNamespacesUnsupported)
    }

    pub(crate) fn unrepresentable_alias_cycle(
        path: String,
        reason: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(ErrorKind::UnrepresentableAliasCycle {
            path,
            reason: reason.into(),
        })
    }

    pub(crate) fn unsupported_semantic_transform(reason: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::UnsupportedSemanticTransform {
            reason: reason.into(),
        })
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::new(ErrorKind::Io(error))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::InvalidMapKey { path, reason } => {
                write!(f, "Invalid map key at '{path}': {reason}")
            }
            ErrorKind::BigIntForbidden { path } => write!(
                f,
                "Attempted to export {path:?} but Specta forbids exporting BigInt-style types (usize, isize, i64, u64, i128, u128) to avoid precision loss. See {BIGINT_DOCS_URL} for a full explanation."
            ),
            ErrorKind::ForbiddenName { path, name } => write!(
                f,
                "Attempted to export {} but was unable to due to name {name:?} conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`",
                display_path(path)
            ),
            ErrorKind::InvalidName { path, name } => write!(
                f,
                "Attempted to export {} but was unable to due to name {name:?} containing an invalid character. Try renaming it or using `#[specta(rename = \"new name\")]`",
                display_path(path)
            ),
            ErrorKind::EmptyName { path } => write!(
                f,
                "Attempted to export {} but was unable to because the Typescript type name is empty. Try renaming it or using `#[specta(rename = \"new name\")]`",
                display_path(path)
            ),
            ErrorKind::UnsupportedAnonymousEnumVariant { path, variant_kind } => write!(
                f,
                "Attempted to export {} but anonymous {variant_kind} enum variants cannot be exported to Typescript. Try giving the variant a name or changing the enum representation.",
                display_path(path)
            ),
            ErrorKind::DuplicateTypeName {
                name,
                first,
                second,
            } => write!(
                f,
                "Detected multiple types with the same name: {name:?} at {first} and {second}"
            ),
            ErrorKind::Io(err) => write!(f, "IO error: {err}"),
            ErrorKind::ReadDir { path, source } => {
                write!(f, "Failed to read directory '{}': {source}", path.display())
            }
            ErrorKind::Metadata { path, source } => {
                write!(
                    f,
                    "Failed to read metadata for '{}': {source}",
                    path.display()
                )
            }
            ErrorKind::ExpectedDirectory { path } => write!(
                f,
                "The files layout requires '{}' to be a directory, but it is an existing file. Refusing to delete it.",
                path.display()
            ),
            ErrorKind::RemoveFile { path, source } => {
                write!(f, "Failed to remove file '{}': {source}", path.display())
            }
            ErrorKind::RemoveDir { path, source } => {
                write!(
                    f,
                    "Failed to remove directory '{}': {source}",
                    path.display()
                )
            }
            ErrorKind::CreateDir { path, source } => {
                write!(
                    f,
                    "Failed to create directory '{}': {source}",
                    path.display()
                )
            }
            ErrorKind::WriteFile { path, source } => {
                write!(f, "Failed to write file '{}': {source}", path.display())
            }
            ErrorKind::ReadFile { path, source } => {
                write!(f, "Failed to read file '{}': {source}", path.display())
            }
            ErrorKind::UnsupportedOpaqueReference { path, reference } => write!(
                f,
                "Found unsupported opaque reference '{}' at {}. It is not supported by the Typescript exporter.",
                reference.type_name(),
                display_path(path)
            ),
            ErrorKind::DanglingNamedReference { path, reference } => write!(
                f,
                "Found dangling named reference {reference} at {}. The referenced type is missing from the resolved type collection.",
                display_path(path)
            ),
            ErrorKind::InfiniteRecursiveInlineType {
                path,
                reference,
                cycle,
            } => {
                write!(
                    f,
                    "Found infinitely recursive inline named reference {reference} at {}. Recursive inline types cannot be expanded because they would produce an infinite Typescript type.",
                    display_path(path)
                )?;
                write!(f, "\nInline cycle:\n  {cycle:?}")?;
                Ok(())
            }
            ErrorKind::InlineRecursionLimitExceeded { path } if path.is_empty() => write!(
                f,
                "Type recursion limit exceeded while expanding the provided inline type. Recursive inline types cannot be expanded because they would produce an infinite Typescript type."
            ),
            ErrorKind::InlineRecursionLimitExceeded { path } => write!(
                f,
                "Type recursion limit exceeded while expanding an inline Typescript type at {}. Recursive inline types cannot be expanded because they would produce an infinite Typescript type.",
                display_path(path)
            ),
            ErrorKind::Framework { message, source } => {
                let source = source.to_string();
                if message.is_empty() && source.is_empty() {
                    write!(f, "Framework error")
                } else if source.is_empty() {
                    write!(f, "Framework error: {message}")
                } else {
                    write!(f, "Framework error: {message}: {source}")
                }
            }
            ErrorKind::Format {
                message,
                path,
                source,
            } => {
                let source = source.to_string();
                let location = path
                    .as_deref()
                    .filter(|path| !path.is_empty())
                    .map(|path| format!(" at {}", display_path(path)))
                    .unwrap_or_default();
                if message.is_empty() && source.is_empty() {
                    write!(f, "Format error{location}")
                } else if source.is_empty() {
                    write!(f, "Format error{location}: {message}")
                } else {
                    write!(f, "Format error{location}: {message}: {source}")
                }
            }
            ErrorKind::ExportRequiresExportTo(layout) => write!(
                f,
                "Unable to export layout {layout} as a single string. Use `Exporter::export_to` with a directory path for file-based exports."
            ),
            ErrorKind::JsdocNamespacesUnsupported => write!(
                f,
                "Unable to export JSDoc with the Namespaces layout. Disable JSDoc or use FlatFile, ModulePrefixedName, or Files layout."
            ),
            ErrorKind::UnrepresentableAliasCycle { path, reason } => write!(
                f,
                "Attempted to export {} but it is part of a recursive type-alias cycle that cannot be collapsed into valid TypeScript: {reason}. TypeScript rejects self-referential type aliases (TS2456); restructure the type so the recursion passes through an object, array, or tuple instead of a bare (untagged/transparent) reference.",
                display_path(path)
            ),
            ErrorKind::UnsupportedSemanticTransform { reason } => write!(
                f,
                "Unable to generate a runtime semantic transform: {reason}. The exported type would otherwise disagree with the value crossing the wire."
            ),
        }?;

        if let Some(ndt) = self.named_datatype() {
            write!(
                f,
                "\nRust type: {}::{} at {}",
                ndt.module_path,
                ndt.name,
                format_location(ndt.location)
            )?;
        }

        if !self.trace.is_empty() {
            write!(f, "\nWhile inlining:")?;
            for frame in self.trace.iter().rev() {
                match frame {
                    ErrorTraceFrame::Inlined {
                        named_datatype,
                        path,
                    } => {
                        write!(f, "\n  {path} -> ")?;
                        if let Some(ndt) = named_datatype.as_deref() {
                            write!(f, "{}::{}", ndt.module_path, ndt.name)?;
                        } else {
                            write!(f, "<unresolved named type>")?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.kind {
            ErrorKind::Io(error) => Some(error),
            ErrorKind::ReadDir { source, .. }
            | ErrorKind::Metadata { source, .. }
            | ErrorKind::RemoveFile { source, .. }
            | ErrorKind::RemoveDir { source, .. }
            | ErrorKind::CreateDir { source, .. }
            | ErrorKind::WriteFile { source, .. }
            | ErrorKind::ReadFile { source, .. } => Some(source),
            ErrorKind::Framework { source, .. } | ErrorKind::Format { source, .. } => {
                Some(source.as_ref())
            }
            _ => None,
        }
    }
}

fn format_location(location: Location<'static>) -> String {
    format!(
        "{}:{}:{}",
        location.file(),
        location.line(),
        location.column()
    )
}

fn display_path(path: &str) -> Cow<'_, str> {
    if path.is_empty() {
        Cow::Borrowed("<unknown path>")
    } else {
        Cow::Owned(format!("{path:?}"))
    }
}
