use std::borrow::Cow;

use specta::{
    Format, FormatError, Type, Types,
    datatype::{DataType, Primitive},
};

struct U32AsString;

impl Format for U32AsString {
    fn map_types<'a>(&'a self, types: &'a Types) -> Result<Cow<'a, Types>, FormatError> {
        Ok(Cow::Borrowed(types))
    }

    fn map_type<'a>(
        &'a self,
        _: &'a Types,
        dt: &'a DataType,
    ) -> Result<Cow<'a, DataType>, FormatError> {
        if matches!(dt, DataType::Primitive(Primitive::u32)) {
            Ok(Cow::Owned(DataType::Primitive(Primitive::str)))
        } else {
            Ok(Cow::Borrowed(dt))
        }
    }
}

#[derive(Type)]
#[specta(collect = false)]
struct FormatConsistencyItem {
    value: u32,
}

#[test]
fn named_definitions_apply_map_type_consistently_across_exporters() {
    let types = || Types::default().register::<FormatConsistencyItem>();

    let typescript = specta_typescript::Typescript::default()
        .export(&types(), U32AsString)
        .expect("TypeScript export should succeed");
    assert!(typescript.contains("value: string"), "{typescript}");

    let zod = specta_zod::Zod::default()
        .export(&types(), U32AsString)
        .expect("Zod export should succeed");
    assert!(zod.contains("z.string()"), "{zod}");

    let effect = specta_effect::Effect::default()
        .export(&types(), U32AsString)
        .expect("Effect export should succeed");
    assert!(effect.contains("value: Schema.String"), "{effect}");

    let valibot = specta_valibot::Valibot::default()
        .export(&types(), U32AsString)
        .expect("Valibot export should succeed");
    assert!(valibot.contains("v.string()"), "{valibot}");

    let python = specta_python::Python::default()
        .export(&types(), U32AsString)
        .expect("Python export should succeed");
    assert!(python.contains("_specta_builtins.str"), "{python}");

    let go = specta_go::Go::default()
        .export(&types(), U32AsString)
        .expect("Go export should succeed");
    assert!(go.contains("Value string"), "{go}");

    let rust = specta_rust::Rust::default()
        .export(&types(), U32AsString)
        .expect("Rust export should succeed");
    assert!(rust.contains("::std::string::String"), "{rust}");
}
