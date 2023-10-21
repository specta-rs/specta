#[cfg(feature = "cli")]
use specta::Type;

#[cfg_attr(feature = "cli", derive(Type))]
pub struct MyStruct {
    pub id: u32,
    pub name: String,
}

#[cfg_attr(feature = "cli", derive(Type))]
pub enum MyEnum {
    Variant1,
    Variant2,
}
