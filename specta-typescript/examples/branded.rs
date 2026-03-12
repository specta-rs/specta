use specta::{
    Type, Types,
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
            .export(&Types::default().register::<Account>())
            .unwrap()
    )
}
