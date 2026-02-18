use std::borrow::Cow;

use specta::{
    Type, TypeCollection,
    datatype::{DataType, Primitive},
};
use specta_typescript::{BigIntExportBehavior, Branded, Error, Typescript, branded};

branded!(pub struct UserId(String) as "UserId");

#[derive(Type)]
struct User {
    id: UserId,
}

fn ts_base_type(ty: &DataType) -> Result<&'static str, Error> {
    match ty {
        DataType::Primitive(Primitive::String | Primitive::char) => Ok("string"),
        DataType::Primitive(Primitive::bool) => Ok("boolean"),
        DataType::Primitive(_) => Ok("number"),
        other => Err(Error::Framework(Cow::Owned(format!(
            "example only supports primitive branded types, got {other:?}"
        )))),
    }
}

fn ts_brand_impl(branded: &Branded) -> Result<Cow<'static, str>, Error> {
    let base = ts_base_type(branded.ty())?;
    Ok(Cow::Owned(format!(
        "import(\"ts-brand\").Brand<{base}, \"{}\">",
        branded.brand().replace('"', "\\\"")
    )))
}

fn effect_brand_impl(branded: &Branded) -> Result<Cow<'static, str>, Error> {
    let base = ts_base_type(branded.ty())?;
    Ok(Cow::Owned(format!(
        "{base} & import(\"effect\").Brand.Brand<\"{}\">",
        branded.brand().replace('"', "\\\"")
    )))
}

fn main() {
    let types = TypeCollection::default().register::<User>();

    let ts_brand = Typescript::default()
        .bigint(BigIntExportBehavior::Number)
        .branded_type_impl(ts_brand_impl)
        .export(&types)
        .unwrap();

    let effect_brand = Typescript::default()
        .bigint(BigIntExportBehavior::Number)
        .branded_type_impl(effect_brand_impl)
        .export(&types)
        .unwrap();

    println!("// ts-brand\n{ts_brand}");
    println!("// Effect Brand\n{effect_brand}");
}
