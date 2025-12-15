use specta::{Type, TypeCollection};
use specta_typescript::{Any, Typescript, primitives};

#[derive(Type)]
struct Testing {
    field: Any,
}

fn main() {
    let mut ts = Typescript::default();

    let r = ts.define("string & { _brand: 'a' }");

    println!(
        "{:?}",
        primitives::inline(&ts, &Default::default(), &r.into())
    );

    // TODO: Properly handle this with opaque types
    // println!("{:?}", primitives::inline(&Default::default(), &Default::default(), &DataType::String));

    let s = ts
        .export(&TypeCollection::default().register::<Testing>())
        .unwrap();
    println!("{s:?}");

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
