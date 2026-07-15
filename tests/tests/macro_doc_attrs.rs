use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
#[doc(hidden)]
struct DocHiddenContainer {
    #[doc(hidden)]
    value: String,
}

#[derive(Type)]
#[specta(collect = false)]
#[doc = concat!("generated", " docs")]
struct GeneratedDocsContainer {
    value: String,
}

#[derive(Type)]
#[specta(collect = false)]
#[doc = include_str!("../../README.md")]
struct IncludedDocsContainer {
    value: String,
}

#[derive(Type)]
#[specta(collect = false)]
enum DocHiddenVariant {
    #[doc(hidden)]
    Value(#[doc = concat!("generated", " field docs")] String),
}

#[test]
fn non_literal_doc_attributes_are_ignored() {
    let mut types = specta::Types::default();
    DocHiddenContainer::definition(&mut types);
    GeneratedDocsContainer::definition(&mut types);
    IncludedDocsContainer::definition(&mut types);
    DocHiddenVariant::definition(&mut types);
}
