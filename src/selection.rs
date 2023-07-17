/// Specta compatible selection of struct fields.
// TODO: better docs w/ example
#[macro_export]
macro_rules! selection {
    ( $s:expr, { $($n:ident),+ } ) => {{
        #[allow(non_camel_case_types)]
        mod selection {
            #[derive(serde::Serialize, $crate::Type)]
            #[specta(inline)]
            pub struct Selection<$($n,)*> {
                $(pub $n: $n),*
            }
        }
        use selection::Selection;
        #[allow(non_camel_case_types)]
        Selection { $($n: $s.$n,)* }
    }};
    ( $s:expr, [{ $($n:ident),+ }] ) => {{
        #[allow(non_camel_case_types)]
        mod selection {
            #[derive(serde::Serialize, $crate::Type)]
            #[specta(inline)]
            pub struct Selection<$($n,)*> {
                $(pub $n: $n,)*
            }
        }
        use selection::Selection;
        #[allow(non_camel_case_types)]
        $s.into_iter().map(|v| Selection { $($n: v.$n,)* }).collect::<Vec<_>>()
    }};
}
