use specta::{
    Type, TypeCollection,
    datatype::collect,
    datatype::{Field, NamedDataType, NamedDataTypeBuilder, Struct},
};
use specta_typescript::{Exporter, Typescript};

use crate::testing::Testing;

#[derive(Type)]
pub struct One {
    a: String,
    b: testing::Testing,
}

mod testing {
    use super::*;

    #[derive(Type)]
    pub struct Testing {
        c: char,
    }
}

#[derive(Type)]
pub struct MyChannel;

fn main() {
    // TODO: `Channel` import???

    let mut types = TypeCollection::default()
        .register::<One>()
        .register::<MyChannel>();

    // TODO: Should this also print "Testing"
    // It doesn't because `One` is pre-resolved so it doesn't trigger a crawl.
    let result = specta::datatype::collect(|| {
        One::definition(&mut types);
    });
    println!(
        "{:?}",
        result.map(|ndt| ndt.name().clone()).collect::<Vec<_>>()
    );

    // // TODO: Make the imports work for this
    // NamedDataTypeBuilder::new(
    //     "VirtualType",
    //     vec![],
    //     Struct::named()
    //         .field("c", Field::new(Testing::definition(&mut types)))
    //         .build(),
    // )
    // .build(&mut types);

    // let todo: NamedDataType = todo!();

    // .framework_runtime("// Hello World")
    // .framework_runtime2(|| {
    //     Testing::definition(&mut types);
    //     // TODO
    //     "// Hello World".into()
    // })

    // TODO: Could this be replaced with `import("@tauri-apps/api/channel).Channel" by using opaque references???

    // TODO: `MyChannel` needs to be opaque so it doesn't have an implicit import.
    let channel_ndt = MyChannel::definition(&mut types);
    Exporter::from(Typescript::default())
        .framework_prelude(|ndts| {
            let result = FRAMEWORK_HEADER;
            for ndt in ndts {
                if ndt == channel_ndt {
                    result.push_str("import { Channel } from '@tauri-apps/api/channel';\n");
                }
            }

            result
        })
        .layout(specta_typescript::Layout::Files)
        .export_to("./framework_test", &types)
        .unwrap();
}
