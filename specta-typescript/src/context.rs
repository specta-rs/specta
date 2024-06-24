use std::{borrow::Cow, fmt};

use specta::ImplLocation;

use super::ExportConfig;

#[derive(Clone, Debug)]
pub(crate) enum PathItem {
    Type(Cow<'static, str>),
    TypeExtended(Cow<'static, str>, ImplLocation),
    Field(Cow<'static, str>),
    Variant(Cow<'static, str>),
}

#[derive(Clone)]
pub(crate) struct ExportContext<'a> {
    pub(crate) cfg: &'a ExportConfig,
    pub(crate) path: Vec<PathItem>,
    // `false` when inline'ing and `true` when exporting as named.
    pub(crate) is_export: bool,
}

impl ExportContext<'_> {
    pub(crate) fn with(&self, item: PathItem) -> Self {
        Self {
            path: self.path.iter().cloned().chain([item]).collect(),
            ..*self
        }
    }

    pub(crate) fn export_path(&self) -> ExportPath {
        ExportPath::new(&self.path)
    }
}

/// Represents the path of an error in the export tree.
/// This is designed to be opaque, meaning it's internal format and `Display` impl are subject to change at will.
pub struct ExportPath(String);

impl ExportPath {
    pub(crate) fn new(path: &[PathItem]) -> Self {
        let mut s = String::new();
        let mut path = path.iter().peekable();
        while let Some(item) = path.next() {
            s.push_str(match item {
                PathItem::Type(v) => v,
                PathItem::TypeExtended(_, loc) => loc.as_str(),
                PathItem::Field(v) => v,
                PathItem::Variant(v) => v,
            });

            if let Some(next) = path.peek() {
                s.push_str(match next {
                    PathItem::Type(_) => " -> ",
                    PathItem::TypeExtended(_, _) => " -> ",
                    PathItem::Field(_) => ".",
                    PathItem::Variant(_) => "::",
                });
            } else {
                break;
            }
        }

        Self(s)
    }

    #[doc(hidden)]
    pub fn new_unsafe(path: &str) -> Self {
        Self(path.to_string())
    }
}

impl PartialEq for ExportPath {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl fmt::Debug for ExportPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for ExportPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
