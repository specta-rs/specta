#[cfg(feature = "function")]
mod test {
    use std::{borrow::Cow, fmt};

    use specta::{datatype::Function, function, specta, ts::ExportConfig, Type};

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

    // https://github.com/oscartbeaumont/tauri-specta/issues/24
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
    #[specta(export = false)]
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
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "a");
            assert_eq!(def.args.len(), 0);
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![b](&type_map);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "b");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                    .unwrap(),
                "string"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![c](&type_map);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "c");
            assert_eq!(def.args.len(), 3);
            assert_eq!(
                specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                    .unwrap(),
                "string"
            );
            assert_eq!(
                specta_typescript::datatype(&ExportConfig::default(), &def.args[1].1, type_map)
                    .unwrap(),
                "number"
            );
            assert_eq!(
                specta_typescript::datatype(&ExportConfig::default(), &def.args[2].1, type_map)
                    .unwrap(),
                "boolean"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![d](&type_map);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "d");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                    .unwrap(),
                "string"
            );
            assert_eq!(
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
                    .as_deref(),
                Some("number")
            );
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![e::<bool>](&type_map);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "e");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                    .unwrap(),
                "boolean"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![f](&type_map);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "f");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                    .unwrap(),
                "string"
            );
            assert_eq!(
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
                    .as_deref(),
                Some("number")
            );
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![g](&type_map);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "g");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                    .unwrap(),
                "string"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![h](&type_map);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "h");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                    .unwrap(),
                "string"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![i](&type_map);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "i");
            assert_eq!(def.args.len(), 0);
            assert_eq!(
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
                    .as_deref(),
                Some("number")
            );
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![k](&type_map);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "k");
            assert_eq!(def.args.len(), 0);
            assert_eq!(
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
                    .as_deref(),
                Some("string | number")
            );
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![l](&type_map);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "l");
            assert_eq!(def.args.len(), 2);
            assert_eq!(
                specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                    .unwrap(),
                "Demo"
            );
            assert_eq!(
                specta_typescript::datatype(&ExportConfig::default(), &def.args[1].1, type_map)
                    .unwrap(),
                "[string, number]"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![m](&type_map);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "m");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta_typescript::datatype(&ExportConfig::default(), &def.args[0].1, type_map)
                    .unwrap(),
                "Demo"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![async_fn](&type_map);
            assert_eq!(def.asyncness, true);
            assert_eq!(def.name, "async_fn");
            assert_eq!(def.args.len(), 0);
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![with_docs](&type_map);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "with_docs");
            assert_eq!(def.args.len(), 0);
            assert_eq!(def.result, None);
            assert_eq!(def.docs, Cow::Borrowed(" Testing Doc Comment"));
        }

        {
            let mut type_map = &mut specta::TypeCollection::default();
            let def: datatype::Function = specta::fn_datatype![raw](&type_map);
            assert_eq!(def.args[0].0, "type");
        }
    }
}
