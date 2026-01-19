use specta::Type;

macro_rules! field_ty_macro {
    () => {
        String
    };
}

#[derive(Type)]
#[specta(collect = false)]
pub struct MacroStruct(field_ty_macro!());

#[derive(Type)]
#[specta(collect = false)]
pub struct MacroStruct2 {
    demo: field_ty_macro!(),
}

#[derive(Type)]
#[specta(collect = false)]
pub enum MacroEnum {
    Demo(field_ty_macro!()),
    Demo2 { demo2: field_ty_macro!() },
}

#[test]
fn test_macro_in_decls() {
    insta::assert_snapshot!(crate::ts::inline::<MacroStruct>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(crate::ts::inline::<MacroStruct2>(&Default::default()).unwrap(), @"{ demo: string }");
    insta::assert_snapshot!(crate::ts::inline::<MacroEnum>(&Default::default()).unwrap(), @"{ Demo: string } | { Demo2: { demo2: string } }");
}
