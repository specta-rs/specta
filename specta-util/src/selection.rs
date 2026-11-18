/// Specta compatible selection of struct fields.
///
/// A selection creates an anonymous struct with the subset of fields you want. It
/// implements [`serde::Serialize`] and [`specta::Type`], so it can be returned from
/// an API without exposing the fields it leaves out.
///
/// ```
/// use specta_util::selection;
///
/// #[derive(Clone)]
/// struct Pet {
///     name: &'static str,
///     age: u32,
/// }
///
/// #[derive(Clone)]
/// struct User {
///     name: &'static str,
///     age: u32,
///     pets: Vec<Pet>,
/// }
///
/// let user = User {
///     name: "Monty",
///     age: 7,
///     pets: vec![Pet {
///         name: "Tux",
///         age: 2,
///     }],
/// };
/// let users = vec![user.clone()];
///
/// // Select a subset of fields.
/// let selection = selection!(user, { name, age });
/// assert_eq!(selection.name, "Monty");
/// assert_eq!(selection.age, 7);
///
/// // Add fields with arbitrary values.
/// let selection = selection!(user, {
///     __typename: "User",
///     name,
/// });
/// assert_eq!(selection.__typename, "User");
///
/// // Select nested structs and lists. Lists use `[{ ... }]`.
/// let selection = selection!(user, {
///     name,
///     pets: [{ name }],
/// });
/// assert_eq!(selection.pets[0].name, "Tux");
///
/// // Apply a selection to a list.
/// let selections = selection!(users, [{ name }]);
/// assert_eq!(selections[0].name, "Monty");
/// ```
#[macro_export]
macro_rules! selection {
    // ----------------------------------------------------------------------
    // Public entry points
    // ----------------------------------------------------------------------

    // `selection!(user, { ... } as Name)` keeps the partial moves of `user`.
    ( $s:ident, { $($fields:tt)+ } as $name:ident ) => {{
        $crate::selection!(@define $name { $($fields)+ });
        $crate::selection!(@construct [$s] [selection] $name [] { $($fields)+ })
    }};
    ( $s:expr, { $($fields:tt)+ } as $name:ident ) => {{
        $crate::selection!(@define $name { $($fields)+ });
        let value = $s;
        $crate::selection!(@construct [value] [selection] $name [] { $($fields)+ })
    }};
    ( $s:ident, { $($fields:tt)+ } ) => {
        $crate::selection!($s, { $($fields)+ } as Selection)
    };
    ( $s:expr, { $($fields:tt)+ } ) => {
        $crate::selection!($s, { $($fields)+ } as Selection)
    };

    // `selection!(users, [{ ... }] as Name)` maps the selection over a list.
    ( $s:expr, [{ $($fields:tt)+ }] as $name:ident ) => {{
        $crate::selection!(@define $name { $($fields)+ });
        let value = $s;
        value
            .into_iter()
            .map(|v| $crate::selection!(@construct [v] [selection] $name [] { $($fields)+ }))
            .collect::<Vec<_>>()
    }};
    ( $s:expr, [{ $($fields:tt)+ }] ) => {
        $crate::selection!($s, [{ $($fields)+ }] as Selection)
    };

    // ----------------------------------------------------------------------
    // Definitions
    // ----------------------------------------------------------------------

    ( @define $name:ident { $($fields:tt)+ } ) => {
        #[allow(non_camel_case_types)]
        mod selection {
            use $crate::__private::{serde as __specta_serde, specta as __specta};

            $crate::selection!(@declare $name [] [] [] { $($fields)+ });
        }
    };

    // Emit a module holding the selection type for a nested struct field.
    ( @module $field:ident { $($fields:tt)+ } ) => {
        pub mod $field {
            use super::{__specta, __specta_serde};

            $crate::selection!(@declare Selection [] [] [] { $($fields)+ });
        }
    };

    // Walk the fields once, emitting a module for each nested field and
    // accumulating the struct declaration. Every field becomes a type
    // parameter so its type is inferred from the value (or the original
    // struct) at the call site.
    ( @declare $name:ident [$($items:tt)*] [$($generics:tt)*] [$($fields:tt)*] { } ) => {
        $($items)*

        #[allow(non_camel_case_types)]
        #[derive(self::__specta_serde::Serialize, self::__specta::Type)]
        #[serde(crate = "self::__specta_serde")]
        #[specta(crate = self::__specta, inline)]
        pub struct $name<$($generics)*> {
            $($fields)*
        }
    };
    ( @declare $name:ident [$($items:tt)*] [$($generics:tt)*] [$($fields:tt)*]
        { $field:ident: { $($nested:tt)+ }, $($rest:tt)* } ) => {
        $crate::selection!(@declare $name
            [$($items)* $crate::selection!(@module $field { $($nested)+ });]
            [$($generics)* $field,]
            [$($fields)* pub $field: $field,]
            { $($rest)* });
    };
    ( @declare $name:ident [$($items:tt)*] [$($generics:tt)*] [$($fields:tt)*]
        { $field:ident: { $($nested:tt)+ } } ) => {
        $crate::selection!(@declare $name
            [$($items)* $crate::selection!(@module $field { $($nested)+ });]
            [$($generics)* $field,]
            [$($fields)* pub $field: $field,]
            { });
    };
    ( @declare $name:ident [$($items:tt)*] [$($generics:tt)*] [$($fields:tt)*]
        { $field:ident: [{ $($nested:tt)+ }], $($rest:tt)* } ) => {
        $crate::selection!(@declare $name
            [$($items)* $crate::selection!(@module $field { $($nested)+ });]
            [$($generics)* $field,]
            [$($fields)* pub $field: $field,]
            { $($rest)* });
    };
    ( @declare $name:ident [$($items:tt)*] [$($generics:tt)*] [$($fields:tt)*]
        { $field:ident: [{ $($nested:tt)+ }] } ) => {
        $crate::selection!(@declare $name
            [$($items)* $crate::selection!(@module $field { $($nested)+ });]
            [$($generics)* $field,]
            [$($fields)* pub $field: $field,]
            { });
    };
    ( @declare $name:ident [$($items:tt)*] [$($generics:tt)*] [$($fields:tt)*]
        { $field:ident: $value:expr, $($rest:tt)* } ) => {
        $crate::selection!(@declare $name
            [$($items)*]
            [$($generics)* $field,]
            [$($fields)* pub $field: $field,]
            { $($rest)* });
    };
    ( @declare $name:ident [$($items:tt)*] [$($generics:tt)*] [$($fields:tt)*]
        { $field:ident: $value:expr } ) => {
        $crate::selection!(@declare $name
            [$($items)*]
            [$($generics)* $field,]
            [$($fields)* pub $field: $field,]
            { });
    };
    ( @declare $name:ident [$($items:tt)*] [$($generics:tt)*] [$($fields:tt)*]
        { $field:ident, $($rest:tt)* } ) => {
        $crate::selection!(@declare $name
            [$($items)*]
            [$($generics)* $field,]
            [$($fields)* pub $field: $field,]
            { $($rest)* });
    };
    ( @declare $name:ident [$($items:tt)*] [$($generics:tt)*] [$($fields:tt)*]
        { $field:ident } ) => {
        $crate::selection!(@declare $name
            [$($items)*]
            [$($generics)* $field,]
            [$($fields)* pub $field: $field,]
            { });
    };

    // ----------------------------------------------------------------------
    // Construction
    // ----------------------------------------------------------------------

    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*] { } ) => {
        $($module)*::$name { $($init)* }
    };
    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*]
        { $field:ident: { $($nested:tt)+ }, $($rest:tt)* } ) => {
        $crate::selection!(@construct
            [$($recv)*] [$($module)*] $name
            [$($init)* $field: $crate::selection!(@construct
                [$($recv)*.$field] [$($module)*::$field] Selection [] { $($nested)+ }),]
            { $($rest)* })
    };
    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*]
        { $field:ident: { $($nested:tt)+ } } ) => {
        $crate::selection!(@construct
            [$($recv)*] [$($module)*] $name
            [$($init)* $field: $crate::selection!(@construct
                [$($recv)*.$field] [$($module)*::$field] Selection [] { $($nested)+ }),]
            { })
    };
    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*]
        { $field:ident: [{ $($nested:tt)+ }], $($rest:tt)* } ) => {
        $crate::selection!(@construct
            [$($recv)*] [$($module)*] $name
            [$($init)* $field: $($recv)*.$field
                .into_iter()
                .map(|v| $crate::selection!(@construct
                    [v] [$($module)*::$field] Selection [] { $($nested)+ }))
                .collect::<Vec<_>>(),]
            { $($rest)* })
    };
    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*]
        { $field:ident: [{ $($nested:tt)+ }] } ) => {
        $crate::selection!(@construct
            [$($recv)*] [$($module)*] $name
            [$($init)* $field: $($recv)*.$field
                .into_iter()
                .map(|v| $crate::selection!(@construct
                    [v] [$($module)*::$field] Selection [] { $($nested)+ }))
                .collect::<Vec<_>>(),]
            { })
    };
    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*]
        { $field:ident: $value:expr, $($rest:tt)* } ) => {
        $crate::selection!(@construct
            [$($recv)*] [$($module)*] $name [$($init)* $field: $value,]
            { $($rest)* })
    };
    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*]
        { $field:ident: $value:expr } ) => {
        $crate::selection!(@construct
            [$($recv)*] [$($module)*] $name [$($init)* $field: $value,]
            { })
    };
    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*]
        { $field:ident, $($rest:tt)* } ) => {
        $crate::selection!(@construct
            [$($recv)*] [$($module)*] $name [$($init)* $field: $($recv)*.$field,]
            { $($rest)* })
    };
    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*]
        { $field:ident } ) => {
        $crate::selection!(@construct
            [$($recv)*] [$($module)*] $name [$($init)* $field: $($recv)*.$field,]
            { })
    };
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    struct User {
        name: &'static str,
        age: u32,
    }

    struct Account {
        name: String,
        password: String,
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

    #[test]
    fn struct_identifier_preserves_unselected_fields() {
        let account = Account {
            name: "Ada".to_owned(),
            password: "correct horse".to_owned(),
        };
        let selection = crate::selection!(account, { name });

        assert_eq!(selection.name, "Ada");
        assert_eq!(account.password, "correct horse");
    }

    #[derive(Clone)]
    #[allow(dead_code)]
    struct Pet {
        name: &'static str,
        age: u32,
    }

    #[derive(Clone)]
    #[allow(dead_code)]
    struct Owner {
        name: &'static str,
        pets: Vec<Pet>,
    }

    fn owner() -> Owner {
        Owner {
            name: "Ada",
            pets: vec![
                Pet {
                    name: "Tux",
                    age: 2,
                },
                Pet {
                    name: "Pebble",
                    age: 4,
                },
            ],
        }
    }

    #[test]
    fn arbitrary_values() {
        let user = User {
            name: "Ada",
            age: 37,
        };
        let selection = crate::selection!(user, {
            __typename: "User",
            name,
        });

        assert_eq!(selection.__typename, "User");
        assert_eq!(selection.name, "Ada");
    }

    #[test]
    fn arbitrary_values_accept_expressions() {
        let user = User {
            name: "Ada",
            age: 37,
        };
        let selection = crate::selection!(user, {
            label: format!("{} ({})", user.name, user.age),
            is_adult: user.age >= 18,
            next_age: user.age + 1,
            name,
        });

        assert_eq!(selection.label, "Ada (37)");
        assert!(selection.is_adult);
        assert_eq!(selection.next_age, 38);
        assert_eq!(selection.name, "Ada");
    }

    #[test]
    fn arbitrary_values_accept_struct_literals() {
        #[derive(serde::Serialize, specta::Type)]
        struct Meta {
            kind: &'static str,
        }

        let selection = crate::selection!(owner(), {
            meta: Meta { kind: "owner" },
            name,
        });

        assert_eq!(selection.meta.kind, "owner");
        assert_eq!(selection.name, "Ada");
    }

    #[test]
    fn nested_struct() {
        let pet = Pet {
            name: "Tux",
            age: 2,
        };
        let selection = crate::selection!(pet, {
            name,
            kind: "linux",
        });

        assert_eq!(selection.name, "Tux");
        assert_eq!(selection.kind, "linux");
    }

    #[test]
    fn nested_list() {
        let selection = crate::selection!(owner(), {
            name,
            pets: [{ name, age }],
        });

        assert_eq!(selection.name, "Ada");
        assert_eq!(selection.pets.len(), 2);
        assert_eq!(selection.pets[0].name, "Tux");
        assert_eq!(selection.pets[0].age, 2);
        assert_eq!(selection.pets[1].name, "Pebble");
        assert_eq!(selection.pets[1].age, 4);
    }

    #[test]
    fn nested_nested() {
        #[allow(dead_code)]
        struct Address {
            city: &'static str,
            country: &'static str,
        }

        struct WithAddress {
            name: &'static str,
            address: Address,
        }

        let value = WithAddress {
            name: "Ada",
            address: Address {
                city: "London",
                country: "UK",
            },
        };
        let selection = crate::selection!(value, {
            name,
            address: {
                city,
                __typename: "Address",
            },
        });

        assert_eq!(selection.name, "Ada");
        assert_eq!(selection.address.city, "London");
        assert_eq!(selection.address.__typename, "Address");
    }

    #[test]
    fn list_of_nested_values() {
        let selection = crate::selection!([owner()], [{
            __typename: "Owner",
            pets: [{ name }],
        }]);

        assert_eq!(selection.len(), 1);
        assert_eq!(selection[0].__typename, "Owner");
        assert_eq!(selection[0].pets[0].name, "Tux");
    }

    #[test]
    fn named_selection_with_trailing_comma() {
        let selection = crate::selection!(owner(), {
            name,
            pets: [{ name }],
        } as OwnerSelection);

        assert_eq!(selection.name, "Ada");
        assert_eq!(selection.pets[0].name, "Tux");
    }

    #[test]
    fn fields_may_omit_the_trailing_comma() {
        let owner = owner();
        let selection = crate::selection!(owner, {
            name,
            pets: [{ name }]
        });
        assert_eq!(selection.pets[0].name, "Tux");

        let selection = crate::selection!(owner, {
            name,
            __typename: "Owner"
        });
        assert_eq!(selection.__typename, "Owner");

        #[allow(dead_code)]
        struct Address {
            city: &'static str,
            country: &'static str,
        }

        #[allow(dead_code)]
        struct WithAddress {
            name: &'static str,
            address: Address,
        }

        let value = WithAddress {
            name: "Ada",
            address: Address {
                city: "London",
                country: "UK",
            },
        };
        let selection = crate::selection!(value, {
            address: { city }
        });
        assert_eq!(selection.address.city, "London");
    }

    #[test]
    fn deeply_nested_lists() {
        #[allow(dead_code)]
        struct Toy {
            name: &'static str,
        }

        #[allow(dead_code)]
        struct Pet {
            name: &'static str,
            toys: Vec<Toy>,
        }

        #[allow(dead_code)]
        struct Owner {
            name: &'static str,
            pets: Vec<Pet>,
        }

        let owner = Owner {
            name: "Ada",
            pets: vec![Pet {
                name: "Tux",
                toys: vec![Toy { name: "Ball" }, Toy { name: "Rope" }],
            }],
        };
        let selection = crate::selection!(owner, {
            __typename: "Owner",
            pets: [{
                __typename: "Pet",
                name,
                toys: [{ name }],
            }],
        });

        assert_eq!(selection.__typename, "Owner");
        assert_eq!(selection.pets[0].__typename, "Pet");
        assert_eq!(selection.pets[0].name, "Tux");
        assert_eq!(selection.pets[0].toys[0].name, "Ball");
        assert_eq!(selection.pets[0].toys[1].name, "Rope");
    }

    #[test]
    fn fields_named_serde_and_specta_do_not_collide() {
        #[allow(dead_code)]
        struct Inner {
            serde: &'static str,
            specta: &'static str,
        }

        #[allow(dead_code)]
        struct Outer {
            serde: Inner,
            specta: Inner,
        }

        let value = Outer {
            serde: Inner {
                serde: "a",
                specta: "b",
            },
            specta: Inner {
                serde: "c",
                specta: "d",
            },
        };
        let selection = crate::selection!(value, {
            serde: { serde },
            specta: { specta },
        });

        assert_eq!(selection.serde.serde, "a");
        assert_eq!(selection.specta.specta, "d");
    }

    #[test]
    fn serializes_values_and_nested_fields() {
        let selection = crate::selection!(owner(), {
            __typename: "Owner",
            pets: [{ name }],
        });

        assert_eq!(
            serde_json::to_value(&selection).expect("selection should serialize"),
            serde_json::json!({
                "__typename": "Owner",
                "pets": [{ "name": "Tux" }, { "name": "Pebble" }],
            })
        );
    }
}
