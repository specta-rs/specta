#[cfg(feature = "functions")]
mod test {
    use std::{borrow::Cow, fmt};

    use specta::{function, specta, ts::ExportConfig, Type};

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

    // TODO: Finish fixing these

    #[test]
    fn test_trailing_comma() {
        function::collect_functions![a];
        function::collect_functions![a,];
        function::collect_functions![a, b, c];
        function::collect_functions![a, b, c,];

        let (functions, types) = function::collect_functions![a, b, c, d, e::<i32>, f, g, h, i, k];
    }

    #[test]
    fn test_function_exporting() {
        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; a);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "a");
            assert_eq!(def.args.len(), 0);
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; b);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "b");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta::ts::datatype(&ExportConfig::default(), &def.args[0].1, type_map).unwrap(),
                "string"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; c);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "c");
            assert_eq!(def.args.len(), 3);
            assert_eq!(
                specta::ts::datatype(&ExportConfig::default(), &def.args[0].1, type_map).unwrap(),
                "string"
            );
            assert_eq!(
                specta::ts::datatype(&ExportConfig::default(), &def.args[1].1, type_map).unwrap(),
                "number"
            );
            assert_eq!(
                specta::ts::datatype(&ExportConfig::default(), &def.args[2].1, type_map).unwrap(),
                "boolean"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; d);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "d");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta::ts::datatype(&ExportConfig::default(), &def.args[0].1, type_map).unwrap(),
                "string"
            );
            assert_eq!(
                def.result
                    .and_then(|result| {
                        Some(
                            specta::ts::datatype(&ExportConfig::default(), &result, type_map)
                                .unwrap(),
                        )
                    })
                    .as_deref(),
                Some("number")
            );
        }

        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; e::<bool>);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "e");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta::ts::datatype(&ExportConfig::default(), &def.args[0].1, type_map).unwrap(),
                "boolean"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; f);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "f");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta::ts::datatype(&ExportConfig::default(), &def.args[0].1, type_map).unwrap(),
                "string"
            );
            assert_eq!(
                def.result
                    .and_then(|result| {
                        Some(
                            specta::ts::datatype(&ExportConfig::default(), &result, type_map)
                                .unwrap(),
                        )
                    })
                    .as_deref(),
                Some("number")
            );
        }

        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; g);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "g");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta::ts::datatype(&ExportConfig::default(), &def.args[0].1, type_map).unwrap(),
                "string"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; h);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "h");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta::ts::datatype(&ExportConfig::default(), &def.args[0].1, type_map).unwrap(),
                "string"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; i);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "i");
            assert_eq!(def.args.len(), 0);
            assert_eq!(
                def.result
                    .and_then(|result| {
                        Some(
                            specta::ts::datatype(&ExportConfig::default(), &result, type_map)
                                .unwrap(),
                        )
                    })
                    .as_deref(),
                Some("number")
            );
        }

        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; k);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "k");
            assert_eq!(def.args.len(), 0);
            assert_eq!(
                def.result
                    .and_then(|result| {
                        Some(
                            specta::ts::datatype(&ExportConfig::default(), &result, type_map)
                                .unwrap(),
                        )
                    })
                    .as_deref(),
                Some("string | number")
            );
        }

        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; l);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "l");
            assert_eq!(def.args.len(), 2);
            assert_eq!(
                specta::ts::datatype(&ExportConfig::default(), &def.args[0].1, type_map).unwrap(),
                "Demo"
            );
            assert_eq!(
                specta::ts::datatype(&ExportConfig::default(), &def.args[1].1, type_map).unwrap(),
                "[string, number]"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; m);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "m");
            assert_eq!(def.args.len(), 1);
            assert_eq!(
                specta::ts::datatype(&ExportConfig::default(), &def.args[0].1, type_map).unwrap(),
                "Demo"
            );
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; async_fn);
            assert_eq!(def.asyncness, true);
            assert_eq!(def.name, "async_fn");
            assert_eq!(def.args.len(), 0);
            assert_eq!(def.result, None);
        }

        {
            let mut type_map = &mut specta::TypeMap::default();
            let def: function::FunctionDataType = specta::fn_datatype!(type_map; with_docs);
            assert_eq!(def.asyncness, false);
            assert_eq!(def.name, "with_docs");
            assert_eq!(def.args.len(), 0);
            assert_eq!(def.result, None);
            assert_eq!(def.docs, Cow::Borrowed(" Testing Doc Comment"));
        }
    }
}
