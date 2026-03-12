use std::borrow::Cow;

use specta::{ResolvedTypes, Type, Types};
use specta_typescript::{
    BigIntExportBehavior, Branded, BrandedTypeExporter, Error, Typescript, branded,
};

branded!(pub struct UserId(String) as "UserId");

#[derive(Type)]
struct User {
    id: UserId,
}

fn ts_brand_impl(
    ctx: BrandedTypeExporter<'_>,
    branded: &Branded,
) -> Result<Cow<'static, str>, Error> {
    let datatype = ctx.inline(branded.ty())?;
    Ok(Cow::Owned(format!(
        "import(\"ts-brand\").Brand<{datatype}, \"{}\">",
        branded.brand().replace('"', "\\\"")
    )))
}

fn effect_brand_impl(
    ctx: BrandedTypeExporter<'_>,
    branded: &Branded,
) -> Result<Cow<'static, str>, Error> {
    let datatype = ctx.inline(branded.ty())?;
    Ok(Cow::Owned(format!(
        "{datatype} & import(\"effect\").Brand.Brand<\"{}\">",
        branded.brand().replace('"', "\\\"")
    )))
}

fn main() {
    let types = Types::default().register::<User>();
    let resolved_types = ResolvedTypes::from_resolved_types(types);

    let ts_brand = Typescript::default()
        .bigint(BigIntExportBehavior::Number)
        .branded_type_impl(ts_brand_impl)
        .export(&resolved_types)
        .unwrap();

    let effect_brand = Typescript::default()
        .bigint(BigIntExportBehavior::Number)
        .branded_type_impl(effect_brand_impl)
        .export(&resolved_types)
        .unwrap();

    println!("// ts-brand\n{ts_brand}");
    println!("// Effect Brand\n{effect_brand}");
}
