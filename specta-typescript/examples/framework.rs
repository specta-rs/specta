use std::path::PathBuf;

use specta::{
    Type, TypeCollection,
    datatype::{DataType, NamedDataTypeBuilder},
};
use specta_typescript::{Exporter, JSDoc, Layout, Typescript, primitives};
use specta_util::selection;

// #[derive(Type)]
// #[specta(rename = "bruh")]
// pub struct OneTwo {
//     a: String,
//     b: testing::Testing,
// }

/// Comment on `One`
#[derive(Type)]
pub struct One {
    /// Comment on `a`
    ///
    /// Another comment on `a`
    a: String,
    b: testing::Testing,
}

mod testing {
    use super::*;

    #[derive(Type)]
    pub struct Testing {
        c: char,
        another: AnotherNestedOne,
        #[specta(inline)]
        inlined: AnotherNestedOne,
    }

    #[derive(Type)]
    pub struct AnotherNestedOne {
        d: i8,
        e: inside_testing::InsideType,
        f: another_inside_testing::AnotherInsideType,
    }

    mod inside_testing {
        use specta::Type;

        #[derive(Type)]
        pub struct InsideType {
            d: i8,
        }
    }

    mod another_inside_testing {
        use specta::Type;

        #[derive(Type)]
        pub struct AnotherInsideType {
            d: i8,
        }
    }
}

#[derive(Type)]
pub struct MyChannel;

#[derive(Type)]
pub struct RecursiveMe {
    testing: Vec<RecursiveMe>,
    /// Inlined container comment
    inlined_container: InlinedContainer,
    inlined_container2: InlinedContainer,
}

/// JSDoc comment on inlined struct
#[derive(Type)]
#[specta(inline)]
pub struct InlinedContainer {
    /// JSDoc comment on `a`
    a: String,
    /// JSDoc comment on `b`
    ///
    /// Comment continued
    b: i32,
    #[specta(inline)]
    inline_in_inlined: AnotherOne,
}

#[derive(Type)]
pub struct AnotherOne {
    /// Hello world from really inlined.
    abc: String,
}

fn main() {
    let mut types = TypeCollection::default()
        .register::<One>()
        .register::<MyChannel>()
        .register::<RecursiveMe>();

    NamedDataTypeBuilder::new("VirtualOne", vec![], i32::definition(&mut types)).build(&mut types);
    let ndt = NamedDataTypeBuilder::new("VirtualTwo", vec![], i32::definition(&mut types))
        .module_path("")
        .build(&mut types);

    let r = ndt.reference(vec![]);
    // let r_inlined = ndt.reference(vec![]).inline(&mut types);

    NamedDataTypeBuilder::new("VirtualThree", vec![], r.into())
        .module_path("")
        .build(&mut types);

    let ndt = NamedDataTypeBuilder::new("AnotherOne", vec![], i32::definition(&mut types))
        .inline()
        .module_path("dontcreateme")
        .build(&mut types);

    // ndt.reference(vec![]); // TODO: This is required, I think, maybe be smarter???

    // TODO: Turn into unit tests
    {

        // NamedDataTypeBuilder::new("Testing", vec![], i32::definition(&mut types))
        //     .module_path("framework::testing")
        //     .build(&mut types);

        // {
        //     NamedDataTypeBuilder::new("testing", vec![], i32::definition(&mut types))
        //         .module_path("brother")
        //         .build(&mut types);

        //     NamedDataTypeBuilder::new(
        //         "ModuleAndTypeDuplicate",
        //         vec![],
        //         i32::definition(&mut types),
        //     )
        //     .module_path("brother::testing")
        //     .build(&mut types);
        // }

        // NamedDataTypeBuilder::new("framework", vec![], i32::definition(&mut types))
        //     .module_path("brother")
        //     .build(&mut types);
    }

    let dt = One::definition(&mut types);

    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("out");
    std::fs::create_dir_all(&base).unwrap();

    {
        let exporter = Exporter::from(Typescript::default());

        let exporter = exporter.framework_runtime(move |exporter| {
            // `TypeCollection`
            // exporter.types;

            // Can access any `Exporter` properties via `Deref`
            // exporter.bigint

            // `exporter.render_types()` allows rendering types within your runtime code,
            // if not called `Exporter` will append it.

            // `exporter.inline`, `exporter.reference` are helpers which passthrough the `TypeCollection` for you.

            Ok(format!(
                "// Runtime\nexport function testing(_: {}) {{}}",
                exporter.inline(&dt)?
            )
            .into())
        });
        // or
        // let exporter = exporter.framework_runtime(move |mut exporter| {
        //     Ok(format!(
        //         "// Runtime\nexport function testing(_: {}) {{}}\n\n// User Types{}",
        //         exporter.inline(&dt)?,
        //         exporter.render_types()?
        //     )
        //     .into())
        // });

        exporter
            .export_to(base.join("framework.ts"), &types)
            .unwrap();

        exporter
            .clone()
            .layout(Layout::ModulePrefixedName)
            .export_to(base.join("framework-prefixed.ts"), &types)
            .unwrap();

        exporter
            .clone()
            .layout(Layout::Namespaces)
            .export_to(base.join("framework-namespaces.ts"), &types)
            .unwrap();

        exporter
            .layout(Layout::Files)
            .export_to(base.join("framework-output"), &types)
            .unwrap();
    }

    {
        let exporter = Exporter::from(JSDoc::default());

        exporter
            .clone()
            .layout(Layout::Files)
            .export_to(base.join("framework-output-js"), &types)
            .unwrap();

        exporter
            .clone()
            .export_to(base.join("framework.js"), &types)
            .unwrap();

        exporter
            .clone()
            .layout(Layout::ModulePrefixedName)
            .export_to(base.join("framework-prefixed.js"), &types)
            .unwrap();
    }
}
