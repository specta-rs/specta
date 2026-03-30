#[derive(Clone, Copy)]
pub enum Scope {
    Container,
    Field,
    Variant,
}

impl Scope {
    fn as_str(self) -> &'static str {
        match self {
            Scope::Container => "container",
            Scope::Field => "field",
            Scope::Variant => "variant",
        }
    }
}

pub fn migration_hint(scope: Scope, key: &str) -> Option<String> {
    let replacement = match (scope, key) {
        (Scope::Container, "rename") | (Scope::Field, "rename") | (Scope::Variant, "rename") => {
            Some("#[serde(rename = \"...\")]")
        }
        (Scope::Container, "rename_all") | (Scope::Variant, "rename_all") => {
            Some("#[serde(rename_all = \"...\")]")
        }
        (Scope::Container, "tag") => Some("#[serde(tag = \"...\")]"),
        (Scope::Container, "content") => Some("#[serde(content = \"...\")]"),
        (Scope::Container, "untagged") => Some("#[serde(untagged)]"),
        (Scope::Field, "flatten") => Some("#[serde(flatten)]"),
        (Scope::Field, "skip_serializing") | (Scope::Variant, "skip_serializing") => {
            Some("#[serde(skip_serializing)]")
        }
        (Scope::Field, "skip_deserializing") | (Scope::Variant, "skip_deserializing") => {
            Some("#[serde(skip_deserializing)]")
        }
        (Scope::Field, "skip_serializing_if") => Some("#[serde(skip_serializing_if = \"...\")]"),
        _ => None,
    }?;

    Some(format!(
        "specta: `#[specta({key} ...)]` is no longer supported on {scope}s. Use `{replacement}` instead.",
        scope = scope.as_str()
    ))
}
