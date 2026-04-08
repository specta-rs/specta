use specta::{Type, Types};
use specta_typescript::Typescript;

// struct Demo<const N: usize = 42> {
//     data: [u32; N],
// }

// #[derive(Type)]
pub struct Testing<T = String>(T);

#[allow(non_camel_case_types)]
const _: () = {
    use specta::datatype;
    use std::borrow::Cow;
    #[automatically_derived]
    impl<T> specta::Type for Testing<T>
    where
        T: specta::Type,
    {
        fn definition(types: &mut specta::Types) -> datatype::DataType {
            pub struct PLACEHOLDER_T;
            impl specta::Type for PLACEHOLDER_T {
                fn definition(_: &mut specta::Types) -> datatype::DataType {
                    datatype::GenericReference::new::<Self>().into()
                }
            }
            static SENTINEL: &str = "generics::Testing";
            static GENERICS: &[(datatype::GenericReference, Cow<'static, str>)] = &[(
                specta::datatype::GenericReference::new::<PLACEHOLDER_T>(),
                Cow::Borrowed("T"),
            )];
            datatype::DataType::Reference(datatype::NamedDataType::init_with_sentinel(
                GENERICS,
                vec![(
                    specta::datatype::GenericReference::new::<PLACEHOLDER_T>(),
                    <T as specta::Type>::definition(types),
                )],
                false,
                types,
                SENTINEL,
                |types, ndt| {
                    ndt.set_name(Cow::Borrowed("Testing"));
                    ndt.set_docs(Cow::Borrowed(""));
                    ndt.set_deprecated(None);
                    ndt.set_module_path(Cow::Borrowed("generics"));
                    ndt.set_ty({
                        type T = PLACEHOLDER_T;
                        {
                            let mut e = {
                                let mut builder = datatype::Struct::unnamed();
                                builder.field_mut({
                                    let mut field = datatype::Field::default();
                                    field.set_optional(false);
                                    field.set_deprecated(None);
                                    field.set_docs("".into());
                                    field.set_inline(false);
                                    field.set_type_overridden(false);
                                    field.set_attributes(datatype::Attributes::default());
                                    if let Some(ty) = Some(<T as specta::Type>::definition(types)) {
                                        field.set_ty(ty);
                                    }
                                    field
                                });
                                builder.build()
                            };
                            match &mut e {
                                datatype::DataType::Struct(s) => {
                                    *s.attributes_mut() = datatype::Attributes::default();
                                }
                                datatype::DataType::Enum(en) => {
                                    *en.attributes_mut() = datatype::Attributes::default();
                                }
                                _ => unreachable!(),
                            }
                            e
                        }
                    });
                },
            ))
        }
    }
};

fn main() {
    println!(
        "{}",
        Typescript::default()
            .export(&specta_serde::apply(Types::default().register::<Testing>()).unwrap(),)
            .unwrap()
    );
}
