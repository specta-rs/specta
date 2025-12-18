use specta::{Type, TypeCollection};
use specta_typescript::{Any, Layout, Typescript, primitives};

#[derive(Type)]
pub struct Testing {
    field: Any,
}

mod nested {
    #[derive(specta::Type)]
    pub struct Another {
        field: super::Testing,
        field2: super::another::Bruh,
    }
}

mod another {
    #[derive(specta::Type)]
    pub struct Bruh {
        field: u32,
        field2: bruh::Testing,
    }

    mod bruh {
        #[derive(specta::Type)]
        pub struct Testing {
            field: dev::Testing,
        }

        mod dev {
            #[derive(specta::Type)]
            pub struct Testing {
                field: u32,
            }
        }
    }
}

// /// An IPC channel.
// pub struct Channel<TSend> {
//     phantom: std::marker::PhantomData<TSend>,
// }

// const _: () = {
//     #[derive(specta::Type)]
//     #[specta(remote = Channel, rename = "TAURI_CHANNEL")]
//     #[allow(dead_code)]
//     struct Channel2<TSend>(std::marker::PhantomData<TSend>);
// };

fn main() {
    let ts = Typescript::default();

    // let r = ts.define("string & { _brand: 'a' }");
    // println!(
    //     "{:?}",
    //     primitives::inline(&ts, &Default::default(), &r.into())
    // );

    // TODO: Properly handle this with opaque types
    // println!("{:?}", primitives::inline(&Default::default(), &Default::default(), &DataType::String));

    ts.layout(Layout::Namespaces)
        .export_to(
            "demo.ts",
            &TypeCollection::default().register::<nested::Another>(),
        )
        .unwrap();

    // println!("PTR EQ: {:?}", std::ptr::eq(&ANY, &ANY));

    // println!(
    //     "definition: {:?}",
    //     Reference::opaque2(&ANY).ref_eq(&Reference::opaque2(&ANY))
    // );

    // match (
    //     Any::<()>::definition(&mut Default::default()),
    //     Any::<()>::definition(&mut Default::default()),
    // ) {
    //     (DataType::Reference(ref1), DataType::Reference(ref2)) => {
    //         println!(
    //             "Reference Tokens: {:?}, {:?} {:?}",
    //             ref1,
    //             ref2,
    //             ref1.ref_eq(&ref2)
    //         );
    //     }
    //     _ => {
    //         println!("Unexpected data types");
    //     }
    // }

    // println!(
    //     "{:?}",
    //     primitives::inline(
    //         &ts,
    //         &Default::default(),
    //         &Any::<()>::definition(&mut Default::default())
    //     )
    // )
}
