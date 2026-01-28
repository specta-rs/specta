use std::fmt;

use specta::{
    Type, TypeCollection,
    datatype::{DataType, Function, FunctionReturnType},
    function::{self, fn_datatype},
    specta,
};
use specta_typescript::{Typescript, primitives};

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
fn f(mut demo: String) -> i32 {
    42
}

#[specta]
fn g(x: std::string::String) {}

macro_rules! special_string {
    () => {
        String
    };
}

#[specta]
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

    function::collect_functions![a, b, c, d, e::<i32>, f, g, h, i, k, nested::nested];
}

#[test]
fn test_function_exporting() {
    {
        let mut types = TypeCollection::default();
        let def: Function = fn_datatype![a](&mut types);
        let ts = Typescript::new();
        insta::assert_snapshot!(def.asyncness(), @"false");
        insta::assert_snapshot!(def.name(), @"a");
        insta::assert_snapshot!(def.args().len(), @"0");
        insta::assert_snapshot!(format!("{:?}", def.result()), @"None");
    }

    {
        let mut types = TypeCollection::default();
        let def: Function = fn_datatype![b](&mut types);
        let ts = Typescript::new();
        insta::assert_snapshot!(def.asyncness(), @"false");
        insta::assert_snapshot!(def.name(), @"b");
        insta::assert_snapshot!(def.args().len(), @"1");
        insta::assert_snapshot!(
            match &def.args()[0].1 {
                DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                dt => primitives::inline(&ts, &types, dt).ok(),
            }.unwrap(),
            @"string"
        );
        insta::assert_snapshot!(format!("{:?}", def.result()), @"None");
    }

    {
        let mut types = TypeCollection::default();
        let def: Function = fn_datatype![c](&mut types);
        let ts = Typescript::new();
        insta::assert_snapshot!(def.asyncness(), @"false");
        insta::assert_snapshot!(def.name(), @"c");
        insta::assert_snapshot!(def.args().len(), @"3");
        insta::assert_snapshot!(
            match &def.args()[0].1 {
                DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                dt => primitives::inline(&ts, &types, dt).ok(),
            }.unwrap(),
            @"string"
        );
        insta::assert_snapshot!(
            match &def.args()[1].1 {
                DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                dt => primitives::inline(&ts, &types, dt).ok(),
            }.unwrap(),
            @"number"
        );
        insta::assert_snapshot!(
            match &def.args()[2].1 {
                DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                dt => primitives::inline(&ts, &types, dt).ok(),
            }.unwrap(),
            @"boolean"
        );
        insta::assert_snapshot!(format!("{:?}", def.result()), @"None");
    }

    {
        let mut types = TypeCollection::default();
        let def: Function = fn_datatype![d](&mut types);
        let ts = Typescript::new();
        insta::assert_snapshot!(def.asyncness(), @"false");
        insta::assert_snapshot!(def.name(), @"d");
        insta::assert_snapshot!(def.args().len(), @"1");
        insta::assert_snapshot!(
            match &def.args()[0].1 {
                DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                dt => primitives::inline(&ts, &types, dt).ok(),
            }.unwrap(),
            @"string"
        );
        insta::assert_snapshot!(
            def.result()
                .and_then(|result| match result {
                    FunctionReturnType::Value(dt) => match dt {
                        DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                        dt => primitives::inline(&ts, &types, dt).ok(),
                    }
                    FunctionReturnType::Result(ok, err) => {
                        let ok_str = match ok {
                            DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                            dt => primitives::inline(&ts, &types, dt).ok(),
                        }?;
                        let err_str = match err {
                            DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                            dt => primitives::inline(&ts, &types, dt).ok(),
                        }?;
                        let mut variants = vec![ok_str, err_str];
                        variants.dedup();
                        Some(variants.join(" | "))
                    }
                })
                .as_deref()
                .unwrap_or("None"),
            @"number"
        );
    }

    {
        let mut types = TypeCollection::default();
        let def: Function = fn_datatype![e::<bool>](&mut types);
        let ts = Typescript::new();
        insta::assert_snapshot!(def.asyncness(), @"false");
        insta::assert_snapshot!(def.name(), @"e");
        insta::assert_snapshot!(def.args().len(), @"1");
        insta::assert_snapshot!(
            match &def.args()[0].1 {
                DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                dt => primitives::inline(&ts, &types, dt).ok(),
            }.unwrap(),
            @"boolean"
        );
        insta::assert_snapshot!(format!("{:?}", def.result()), @"None");
    }

    {
        let mut types = TypeCollection::default();
        let def: Function = fn_datatype![f](&mut types);
        let ts = Typescript::new();
        insta::assert_snapshot!(def.asyncness(), @"false");
        insta::assert_snapshot!(def.name(), @"f");
        insta::assert_snapshot!(def.args().len(), @"1");
        insta::assert_snapshot!(
            match &def.args()[0].1 {
                DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                dt => primitives::inline(&ts, &types, dt).ok(),
            }.unwrap(),
            @"string"
        );
        insta::assert_snapshot!(
            def.result()
                .and_then(|result| match result {
                    FunctionReturnType::Value(dt) => match dt {
                        DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                        dt => primitives::inline(&ts, &types, dt).ok(),
                    }
                    FunctionReturnType::Result(ok, err) => {
                        let ok_str = match ok {
                            DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                            dt => primitives::inline(&ts, &types, dt).ok(),
                        }?;
                        let err_str = match err {
                            DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                            dt => primitives::inline(&ts, &types, dt).ok(),
                        }?;
                        let mut variants = vec![ok_str, err_str];
                        variants.dedup();
                        Some(variants.join(" | "))
                    }
                })
                .as_deref()
                .unwrap_or("None"),
            @"number"
        );
    }

    {
        let mut type_map = TypeCollection::default();
        let def: specta::datatype::Function = fn_datatype![g](&mut type_map);
        let ts = Typescript::new();
        insta::assert_snapshot!(def.asyncness(), @"false");
        insta::assert_snapshot!(def.name(), @"g");
        insta::assert_snapshot!(def.args().len(), @"1");
        insta::assert_snapshot!(
            match &def.args()[0].1 {
                DataType::Reference(r) => primitives::reference(&ts, &type_map, r).ok(),
                dt => primitives::inline(&ts, &type_map, dt).ok(),
            }.unwrap(),
            @"string"
        );
        insta::assert_snapshot!(format!("{:?}", def.result()), @"None");
    }

    {
        let mut type_map = TypeCollection::default();
        let def: specta::datatype::Function = fn_datatype![h](&mut type_map);
        let ts = Typescript::new();
        insta::assert_snapshot!(def.asyncness(), @"false");
        insta::assert_snapshot!(def.name(), @"h");
        insta::assert_snapshot!(def.args().len(), @"1");
        insta::assert_snapshot!(
            match &def.args()[0].1 {
                DataType::Reference(r) => primitives::reference(&ts, &type_map, r).ok(),
                dt => primitives::inline(&ts, &type_map, dt).ok(),
            }.unwrap(),
            @"string"
        );
        insta::assert_snapshot!(format!("{:?}", def.result()), @"None");
    }

    {
        let mut types = TypeCollection::default();
        let def: Function = fn_datatype![i](&mut types);
        let ts = Typescript::new();
        insta::assert_snapshot!(def.asyncness(), @"false");
        insta::assert_snapshot!(def.name(), @"i");
        insta::assert_snapshot!(def.args().len(), @"0");
        insta::assert_snapshot!(
            def.result()
                .and_then(|result| match result {
                    FunctionReturnType::Value(dt) => match dt {
                        DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                        dt => primitives::inline(&ts, &types, dt).ok(),
                    }
                    FunctionReturnType::Result(ok, err) => {
                        let ok_str = match ok {
                            DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                            dt => primitives::inline(&ts, &types, dt).ok(),
                        }?;
                        let err_str = match err {
                            DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                            dt => primitives::inline(&ts, &types, dt).ok(),
                        }?;
                        let mut variants = vec![ok_str, err_str];
                        variants.dedup();
                        Some(variants.join(" | "))
                    }
                })
                .as_deref()
                .unwrap_or("None"),
            @"number"
        );
    }

    {
        let mut types = TypeCollection::default();
        let def: Function = fn_datatype![k](&mut types);
        let ts = Typescript::new();
        insta::assert_snapshot!(def.asyncness(), @"false");
        insta::assert_snapshot!(def.name(), @"k");
        insta::assert_snapshot!(def.args().len(), @"0");
        insta::assert_snapshot!(
            def.result()
                .and_then(|result| match result {
                    FunctionReturnType::Value(dt) => match dt {
                        DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                        dt => primitives::inline(&ts, &types, dt).ok(),
                    }
                    FunctionReturnType::Result(ok, err) => {
                        let ok_str = match ok {
                            DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                            dt => primitives::inline(&ts, &types, dt).ok(),
                        }?;
                        let err_str = match err {
                            DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                            dt => primitives::inline(&ts, &types, dt).ok(),
                        }?;
                        let mut variants = vec![ok_str, err_str];
                        variants.dedup();
                        Some(variants.join(" | "))
                    }
                })
                .as_deref()
                .unwrap_or("None"),
            @"string | number"
        );
    }

    {
        let mut types = TypeCollection::default();
        let def: Function = fn_datatype![l](&mut types);
        let ts = Typescript::new();
        insta::assert_snapshot!(def.asyncness(), @"false");
        insta::assert_snapshot!(def.name(), @"l");
        insta::assert_snapshot!(def.args().len(), @"2");
        insta::assert_snapshot!(
            match &def.args()[0].1 {
                DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                dt => primitives::inline(&ts, &types, dt).ok(),
            }.unwrap(),
            @"Demo"
        );
        insta::assert_snapshot!(
            match &def.args()[1].1 {
                DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                dt => primitives::inline(&ts, &types, dt).ok(),
            }.unwrap(),
            @"[string, number]"
        );
        insta::assert_snapshot!(format!("{:?}", def.result()), @"None");
    }

    {
        let mut types = TypeCollection::default();
        let def: Function = fn_datatype![m](&mut types);
        let ts = Typescript::new();
        insta::assert_snapshot!(def.asyncness(), @"false");
        insta::assert_snapshot!(def.name(), @"m");
        insta::assert_snapshot!(def.args().len(), @"1");
        insta::assert_snapshot!(
            match &def.args()[0].1 {
                DataType::Reference(r) => primitives::reference(&ts, &types, r).ok(),
                dt => primitives::inline(&ts, &types, dt).ok(),
            }.unwrap(),
            @"Demo"
        );
        insta::assert_snapshot!(format!("{:?}", def.result()), @"None");
    }

    {
        let mut types = TypeCollection::default();
        let def: Function = fn_datatype![async_fn](&mut types);
        insta::assert_snapshot!(def.asyncness(), @"true");
        insta::assert_snapshot!(def.name(), @"async_fn");
        insta::assert_snapshot!(def.args().len(), @"0");
        insta::assert_snapshot!(format!("{:?}", def.result()), @"None");
    }

    {
        let mut types = TypeCollection::default();
        let def: Function = fn_datatype![with_docs](&mut types);
        insta::assert_snapshot!(def.asyncness(), @"false");
        insta::assert_snapshot!(def.name(), @"with_docs");
        insta::assert_snapshot!(def.args().len(), @"0");
        insta::assert_snapshot!(format!("{:?}", def.result()), @"None");
        insta::assert_snapshot!(def.docs(), @" Testing Doc Comment");
    }

    {
        let mut types = TypeCollection::default();
        let def: Function = fn_datatype![raw](&mut types);
        insta::assert_snapshot!(def.args()[0].0, @"type");
    }
}
