use std::fmt;

use specta::{Type, function, specta};

/// Multiline
/// Docs
#[specta]
fn a() {}

#[specta]
fn b(demo: String) {}

#[specta]
fn c(a: String, b: i32, c: bool) {}

#[specta]
fn d(demo: String) -> i32 {
    42
}

#[specta]
fn e<T: fmt::Debug>(window: T) {}

// https://github.com/specta-rs/tauri-specta/issues/24
#[specta]
#[allow(unused_mut)]
fn f(mut demo: String) -> i32 {
    42
}

#[specta]
#[allow(unused_mut)]
fn g(x: std::string::String) {}

macro_rules! special_string {
    () => {
        String
    };
}

#[specta]
#[allow(unused_mut)]
fn h(demo: special_string!()) {}

#[specta]
fn i() -> Result<i32, f32> {
    Ok(42)
}

#[specta]
fn k() -> Result<String, f32> {
    Err(42.0)
}

#[derive(Type)]
#[specta(collect = false)]
pub struct Demo {
    pub demo: String,
}

#[specta]
fn l(Demo { demo }: Demo, (a, b): (String, u32)) {}

macro_rules! special_destructure {
    () => {
        Demo { demo }
    };
}

#[specta]
#[allow(unused_mut)]
fn m(special_destructure!(): Demo) {}

#[specta]
async fn async_fn() {}

/// Testing Doc Comment
#[specta]
fn with_docs() {}

#[specta]
pub fn public_function() {}

mod nested {
    use super::*;

    #[specta]
    pub fn nested() {}
}

#[specta]
fn raw(r#type: i32) {}

// TODO: Finish fixing these

#[test]
fn test_trailing_comma() {
    function::collect_functions![a];
    function::collect_functions![a,];
    function::collect_functions![a, b, c];
    function::collect_functions![a, b, c,];

    let (functions, types) =
        function::collect_functions![a, b, c, d, e::<i32>, f, g, h, i, k, nested::nested];
}

#[test]
fn test_function_exporting() {
    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![a](&type_map);
        insta::assert_snapshot!(def.asyncness, @"false");
        insta::assert_snapshot!(def.name, @"a");
        insta::assert_snapshot!(def.args.len(), @"0");
        insta::assert_snapshot!(format!("{:?}", def.result), @"None");
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![b](&type_map);
        insta::assert_snapshot!(def.asyncness, @"false");
        insta::assert_snapshot!(def.name, @"b");
        insta::assert_snapshot!(def.args.len(), @"1");
        insta::assert_snapshot!(
            specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                .unwrap(),
            @"string"
        );
        insta::assert_snapshot!(format!("{:?}", def.result), @"None");
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![c](&type_map);
        insta::assert_snapshot!(def.asyncness, @"false");
        insta::assert_snapshot!(def.name, @"c");
        insta::assert_snapshot!(def.args.len(), @"3");
        insta::assert_snapshot!(
            specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                .unwrap(),
            @"string"
        );
        insta::assert_snapshot!(
            specta_typescript::datatype(&ExportConfig::default(), &def.args[1].1, type_map)
                .unwrap(),
            @"number"
        );
        insta::assert_snapshot!(
            specta_typescript::datatype(&ExportConfig::default(), &def.args[2].1, type_map)
                .unwrap(),
            @"boolean"
        );
        insta::assert_snapshot!(format!("{:?}", def.result), @"None");
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![d](&type_map);
        insta::assert_snapshot!(def.asyncness, @"false");
        insta::assert_snapshot!(def.name, @"d");
        insta::assert_snapshot!(def.args.len(), @"1");
        insta::assert_snapshot!(
            specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                .unwrap(),
            @"string"
        );
        insta::assert_snapshot!(
            def.result
                .and_then(|result| {
                    Some(
                        specta_typescript::datatype(
                            &ExportConfig::default(),
                            &result,
                            type_map,
                        )
                        .unwrap(),
                    )
                })
                .as_deref()
                .unwrap_or("None"),
            @"number"
        );
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![e::<bool>](&type_map);
        insta::assert_snapshot!(def.asyncness, @"false");
        insta::assert_snapshot!(def.name, @"e");
        insta::assert_snapshot!(def.args.len(), @"1");
        insta::assert_snapshot!(
            specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                .unwrap(),
            @"boolean"
        );
        insta::assert_snapshot!(format!("{:?}", def.result), @"None");
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![f](&type_map);
        insta::assert_snapshot!(def.asyncness, @"false");
        insta::assert_snapshot!(def.name, @"f");
        insta::assert_snapshot!(def.args.len(), @"1");
        insta::assert_snapshot!(
            specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                .unwrap(),
            @"string"
        );
        insta::assert_snapshot!(
            def.result
                .and_then(|result| {
                    Some(
                        specta_typescript::datatype(
                            &ExportConfig::default(),
                            &result,
                            type_map,
                        )
                        .unwrap(),
                    )
                })
                .as_deref()
                .unwrap_or("None"),
            @"number"
        );
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![g](&type_map);
        insta::assert_snapshot!(def.asyncness, @"false");
        insta::assert_snapshot!(def.name, @"g");
        insta::assert_snapshot!(def.args.len(), @"1");
        insta::assert_snapshot!(
            specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                .unwrap(),
            @"string"
        );
        insta::assert_snapshot!(format!("{:?}", def.result), @"None");
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![h](&type_map);
        insta::assert_snapshot!(def.asyncness, @"false");
        insta::assert_snapshot!(def.name, @"h");
        insta::assert_snapshot!(def.args.len(), @"1");
        insta::assert_snapshot!(
            specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                .unwrap(),
            @"string"
        );
        insta::assert_snapshot!(format!("{:?}", def.result), @"None");
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![i](&type_map);
        insta::assert_snapshot!(def.asyncness, @"false");
        insta::assert_snapshot!(def.name, @"i");
        insta::assert_snapshot!(def.args.len(), @"0");
        insta::assert_snapshot!(
            def.result
                .and_then(|result| {
                    Some(
                        specta_typescript::datatype(
                            &ExportConfig::default(),
                            &result,
                            type_map,
                        )
                        .unwrap(),
                    )
                })
                .as_deref()
                .unwrap_or("None"),
            @"number"
        );
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![k](&type_map);
        insta::assert_snapshot!(def.asyncness, @"false");
        insta::assert_snapshot!(def.name, @"k");
        insta::assert_snapshot!(def.args.len(), @"0");
        insta::assert_snapshot!(
            def.result
                .and_then(|result| {
                    Some(
                        specta_typescript::datatype(
                            &ExportConfig::default(),
                            &result,
                            type_map,
                        )
                        .unwrap(),
                    )
                })
                .as_deref()
                .unwrap_or("None"),
            @"string | number"
        );
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![l](&type_map);
        insta::assert_snapshot!(def.asyncness, @"false");
        insta::assert_snapshot!(def.name, @"l");
        insta::assert_snapshot!(def.args.len(), @"2");
        insta::assert_snapshot!(
            specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                .unwrap(),
            @"Demo"
        );
        insta::assert_snapshot!(
            specta_typescript::datatype(&ExportConfig::default(), &def.args[1].1, type_map)
                .unwrap(),
            @"[string, number]"
        );
        insta::assert_snapshot!(format!("{:?}", def.result), @"None");
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![m](&type_map);
        insta::assert_snapshot!(def.asyncness, @"false");
        insta::assert_snapshot!(def.name, @"m");
        insta::assert_snapshot!(def.args.len(), @"1");
        insta::assert_snapshot!(
            specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                .unwrap(),
            @"Demo"
        );
        insta::assert_snapshot!(format!("{:?}", def.result), @"None");
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![async_fn](&type_map);
        insta::assert_snapshot!(def.asyncness, @"true");
        insta::assert_snapshot!(def.name, @"async_fn");
        insta::assert_snapshot!(def.args.len(), @"0");
        insta::assert_snapshot!(format!("{:?}", def.result), @"None");
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![with_docs](&type_map);
        insta::assert_snapshot!(def.asyncness, @"false");
        insta::assert_snapshot!(def.name, @"with_docs");
        insta::assert_snapshot!(def.args.len(), @"0");
        insta::assert_snapshot!(format!("{:?}", def.result), @"None");
        insta::assert_snapshot!(def.docs, @" Testing Doc Comment");
    }

    {
        let mut type_map = &mut specta::TypeCollection::default();
        let def: datatype::Function = specta::fn_datatype![raw](&type_map);
        insta::assert_snapshot!(def.args[0].0, @"type");
    }
}
