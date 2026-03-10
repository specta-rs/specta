use specta::{
    Type, TypeCollection,
    datatype::{DataType, Reference},
};
use specta_typescript::{Typescript, branded};

branded!(#[derive(Default)] struct AccountId(String) as "accountId");

#[derive(Type)]
pub struct Account {
    id: AccountId,
    name: String,
    data: specta_typescript::Any,
}

fn main() {
    println!(
        "{}",
        Typescript::default()
            .export(&TypeCollection::default().register::<Account>())
            .unwrap()
    )
}
