use specta::{
    ResolvedTypes, Type, Types,
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
            .export(&ResolvedTypes::from_resolved_types(
                Types::default().register::<Account>(),
            ))
            .unwrap()
    )
}
