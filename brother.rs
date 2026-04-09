#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
use specta::{Type, Types};
use specta_typescript::Typescript;
pub struct Testing<T>(T);
#[allow(non_camel_case_types)]
const _: () = {
    use std::borrow::Cow;
    use specta::datatype;
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
            static GENERICS: &[(datatype::GenericReference, Cow<'static, str>)] = &[
                (
                    specta::datatype::GenericReference::new::<PLACEHOLDER_T>(),
                    Cow::Borrowed("T"),
                ),
            ];
            datatype::DataType::Reference(
                datatype::NamedDataType::init_with_sentinel(
                    GENERICS,
                    <[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            (
                                specta::datatype::GenericReference::new::<PLACEHOLDER_T>(),
                                <T as specta::Type>::definition(types),
                            ),
                        ]),
                    ),
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
                                    builder
                                        .field_mut({
                                            let mut field = datatype::Field::default();
                                            field.set_optional(false);
                                            field.set_deprecated(None);
                                            field.set_docs("".into());
                                            field.set_inline(false);
                                            field.set_type_overridden(false);
                                            field.set_attributes(datatype::Attributes::default());
                                            if let Some(ty) = Some(
                                                <T as specta::Type>::definition(types),
                                            ) {
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
                                    _ => {
                                        ::core::panicking::panic_fmt(
                                            format_args!(
                                                "internal error: entered unreachable code: {0}",
                                                format_args!(
                                                    "specta derive generated non-container datatype",
                                                ),
                                            ),
                                        );
                                    }
                                }
                                e
                            }
                        });
                    },
                ),
            )
        }
    }
    const _: () = {
        #[allow(unsafe_code, non_snake_case)]
        #[allow(unused)]
        unsafe fn __push_specta_type_Testing() {
            #[allow(unsafe_code)]
            {
                #[link_section = "__DATA,__mod_init_func,mod_init_funcs"]
                #[used]
                #[allow(non_upper_case_globals, non_snake_case)]
                #[doc(hidden)]
                static f: extern "C" fn() -> ::ctor::__support::CtorRetType = {
                    #[allow(non_snake_case)]
                    extern "C" fn f() -> ::ctor::__support::CtorRetType {
                        unsafe {
                            __push_specta_type_Testing();
                        };
                        ::core::default::Default::default()
                    }
                    f
                };
            }
            {
                specta::collect::internal::register::<Testing<()>>();
            }
        }
    };
};
fn main() {
    {
        ::std::io::_print(
            format_args!(
                "{0}\n",
                Typescript::default()
                    .export(
                        &specta_serde::apply(Types::default().register::<Testing<i32>>())
                            .unwrap(),
                    )
                    .unwrap(),
            ),
        );
    };
}
