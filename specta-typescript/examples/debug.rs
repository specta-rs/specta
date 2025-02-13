use std::{collections::HashMap, thread::sleep, time::Duration};

use specta::{Type, TypeCollection};

#[derive(Type)]
#[specta(export = false)]
struct Demo<A, B> {
    a: A,
    b: B,
}

type HalfGenericA<T> = Demo<T, bool>;

#[derive(Type)]
#[specta(export = false)]
struct Struct<T> {
    field: HalfGenericA<T>,
}

type MapC<B> = HashMap<String, Struct<B>>;

fn main() {
    // let mut types = TypeCollection::default();
    // println!("{:#?} {:#?}", MapC::<i32>::definition(&mut types), types);
    // let mut types = TypeCollection::default();
    // println!("{:#?} {:#?}", Struct::<i32>::definition(&mut types), types);

    loop {
        // println!(
        //     "{:#?}",
        //     specta_typescript::inline::<MapC<i32>>(&Default::default()) // "Partial<{ [key in string]: { field: Demo<number, boolean> } }>"
        // );

        // println!("{:?}", MapC::<i32>::definition(&mut Default::default()));

        let mut types = TypeCollection::default();

        MapC::<i32>::definition(&mut types);
        // println!("{:?}", types);

        // let ty = specta::datatype::inline(MapC::<i32>::definition(&mut types), &types);
        // println!("{ty:?}");

        sleep(Duration::from_secs(1));
    }

    // #[derive(Type)]
    // #[specta(export = false, transparent)]
    // pub struct Transparent<T>(T);

    // let mut types = TypeCollection::default();
    // let ty = Transparent::<Transparent<String>>::definition(&mut types);
    // println!("{:#?} \n {:#?}", ty, types);

    // println!(
    //     "{:?}",
    //     specta_typescript::inline::<Transparent<Transparent<String>>>(&Default::default())
    // );

    // println!(
    //     "{:?}",
    //     specta_typescript::inline::<ValidMaybeValidKey>(&Default::default())
    // );

    // println!(
    //     "{:?}",
    //     ValidMaybeValidKeyNested::definition(&mut Default::default())
    // );
    // println!(
    //     "{:?}",
    //     specta_typescript::inline::<ValidMaybeValidKeyNested>(&Default::default())
    // );

    // println!(
    //     "{:#?}",
    //     Generic1::<i32>::definition(&mut Default::default())
    // );
    // println!(
    //     "{:?}\n\n",
    //     specta_typescript::inline::<Generic1<i32>>(&Default::default())
    // );
    // println!(
    //     "{:?}\n\n",
    //     specta_typescript::export::<Generic1<i32>>(&Default::default())
    // );

    // println!("{:#?}", Recursive::definition(&mut Default::default()));

    // println!(
    //     "{:?}\n\n",
    //     specta_typescript::inline::<Todo>(&Default::default())
    // );
    // println!(
    //     "{:?}\n\n",
    //     specta_typescript::export::<HashMap<UnitVariants, ()>>(&Default::default())
    // );

    // Ok("{ tag: \"A\" } | ({ tag: \"B\" } & string) | { tag: \"C\"; a: string }")
}
