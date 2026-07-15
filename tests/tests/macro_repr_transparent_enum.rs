use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
#[repr(transparent)]
enum ReprTransparentEnum {
    Value(String),
}

#[test]
fn repr_transparent_enum_derives_type() {
    ReprTransparentEnum::definition(&mut specta::Types::default());
}
