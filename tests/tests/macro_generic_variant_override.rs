use specta::{
    Type,
    datatype::{DataType, Reference},
};

#[derive(Type)]
#[specta(collect = false)]
enum VariantTypeOverride<T> {
    #[specta(type = Vec<T>)]
    Value(u32),
    #[specta(skip)]
    Marker(std::marker::PhantomData<T>),
}

#[test]
fn variant_type_override_registers_used_generic() {
    let mut types = specta::Types::default();
    let DataType::Reference(Reference::Named(reference)) =
        VariantTypeOverride::<String>::definition(&mut types)
    else {
        panic!("derived named type should produce a reference");
    };
    assert_eq!(
        types
            .get(&reference)
            .expect("type should be registered")
            .generics
            .iter()
            .map(|generic| generic.name.as_ref())
            .collect::<Vec<_>>(),
        ["T"]
    );
}
