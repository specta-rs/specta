#![allow(deprecated)]

use specta::{
    Type,
    datatype::{DataType, Reference},
};

#[derive(Type)]
#[specta(collect = false)]
#[deprecated(since = "1.0.0", note = "use NewType")]
struct DeprecatedSince {
    value: String,
}

#[test]
fn structured_deprecated_since_is_recorded() {
    let mut types = specta::Types::default();
    let DataType::Reference(Reference::Named(reference)) = DeprecatedSince::definition(&mut types)
    else {
        panic!("derived named type should produce a reference");
    };
    let deprecated = types
        .get(&reference)
        .and_then(|ndt| ndt.deprecated.as_ref())
        .expect("deprecated metadata should be recorded");
    assert_eq!(deprecated.since.as_deref(), Some("1.0.0"));
    assert_eq!(deprecated.note.as_deref(), Some("use NewType"));
}
