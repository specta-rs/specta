/// Specta compatible selection of struct fields.
///
/// ```ignore
/// use specta_typescript::legacy::inline_ref;
/// use specta_util::selection;
///
/// #[derive(Clone)]
/// struct MyStruct {
///     name: String,
///     age: i32,
///     is_verified: bool,
///     password: String,
/// }
///
/// let person = MyStruct {
///     name: "Monty".into(),
///     age: 7,
///     is_verified: true,
///     password: "password".into(),
/// };
/// let people = vec![person.clone(), person.clone()];
///
/// // Selection creates an anonymous struct with the subset of fields you want.
/// assert_eq!(inline_ref(&selection!(person, {
///     name,
///     age
/// }), &Default::default()).unwrap(), "{ name: string; age: number }");
///
/// // You can apply the selection to an array.
/// assert_eq!(inline_ref(&selection!(people, [{
///     name,
///     age
/// }]), &Default::default()).unwrap(), "{ name: string; age: number }[]");
/// ```
// TODO: better docs w/ example
#[macro_export]
macro_rules! selection {
    ( $s:expr, { $($n:ident),+ $(,)? } as $name:ident ) => {{
        #[allow(non_camel_case_types)]
        mod selection {
            mod deps {
                pub use $crate::__private::{serde, specta};
            }

            pub mod definition {
                #[derive(super::deps::serde::Serialize, super::deps::specta::Type)]
                #[serde(crate = "super::deps::serde")]
                #[specta(crate = super::deps::specta, inline)]
                pub struct $name<$($n,)*> {
                    $(pub $n: $n),*
                }
            }
        }
        use selection::definition::$name;
        #[allow(non_camel_case_types)]
        let value = $s;
        $name { $($n: value.$n,)* }
    }};
    ( $s:expr, { $($n:ident),+ $(,)? } ) => {{
        $crate::selection!($s, { $($n),+ } as Selection)
    }};

    ( $s:expr, [{ $($n:ident),+ $(,)? }] as $name:ident ) => {{
        #[allow(non_camel_case_types)]
        mod selection {
            mod deps {
                pub use $crate::__private::{serde, specta};
            }

            pub mod definition {
                #[derive(super::deps::serde::Serialize, super::deps::specta::Type)]
                #[serde(crate = "super::deps::serde")]
                #[specta(crate = super::deps::specta, inline)]
                pub struct $name<$($n,)*> {
                    $(pub $n: $n),*
                }
            }
        }
        use selection::definition::$name;
        #[allow(non_camel_case_types)]
        $s.into_iter().map(|v| $name { $($n: v.$n,)* }).collect::<Vec<_>>()
    }};
    ( $s:expr, [{ $($n:ident),+ $(,)? }] ) => {{
        $crate::selection!($s, [{ $($n),+ }] as Selection)
    }};
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    struct User {
        name: &'static str,
        age: u32,
    }

    #[test]
    fn struct_input_is_evaluated_once() {
        let evaluations = Cell::new(0);
        let selection = crate::selection!(
            {
                evaluations.set(evaluations.get() + 1);
                User {
                    name: "Ada",
                    age: 37,
                }
            },
            { name, age }
        );

        assert_eq!(evaluations.get(), 1);
        assert_eq!(selection.name, "Ada");
        assert_eq!(selection.age, 37);
    }
}
