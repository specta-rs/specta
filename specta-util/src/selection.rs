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
    ( $s:ident, { $($fields:tt)+ } as $name:ident ) => {
        $crate::selection!(@build $name [$s] [] [] [] { $($fields)+ })
    };
    ( $s:expr, { $($fields:tt)+ } as $name:ident ) => {{
        let value = $s;
        $crate::selection!(@build $name [value] [] [] [] { $($fields)+ })
    }};
    ( $s:ident, { $($fields:tt)+ } ) => {
        $crate::selection!(@build Selection [$s] [] [] [] { $($fields)+ })
    };
    ( $s:expr, { $($fields:tt)+ } ) => {{
        let value = $s;
        $crate::selection!(@build Selection [value] [] [] [] { $($fields)+ })
    }};

    // `selection!(users, [{ ... }] as Name)` maps the selection over a list.
    ( $s:expr, [{ $($fields:tt)+ }] as $name:ident ) => {{
        let value = $s;
        value
            .into_iter()
            .map(|v| $crate::selection!(@build $name [v] [] [] [] { $($fields)+ }))
            .collect::<Vec<_>>()
    }};
    ( $s:expr, [{ $($fields:tt)+ }] ) => {{
        let value = $s;
        value
            .into_iter()
            .map(|v| $crate::selection!(@build Selection [v] [] [] [] { $($fields)+ }))
            .collect::<Vec<_>>()
    }};

    // ----------------------------------------------------------------------
    // Definition
    // ----------------------------------------------------------------------

    // Emit the module holding the selection type for a nested struct field.
    ( @module $field:ident { $($fields:tt)+ } ) => {
        pub mod $field {
            use super::{__sp, __sd, __u};

            $crate::selection!(@declare Selection [] [] { $($fields)+ });
        }
    };

    // Fast path: a body that is only bare field names needs no walk at all.
    ( @build $name:ident [$recv:tt] [] [] [] { $($field:ident),* $(,)? } ) => {{
        #[allow(non_camel_case_types)]
        mod selection {
            use $crate::__private::{serde as __sd, specta as __sp, unraw as __u};

            $crate::selection!(@type $name [] [$($field,)*]);
        }
        selection::$name { $($field: $recv.$field),* }
    }};

    // Declare the type *and* build the value in a single walk. Every field
    // becomes a type parameter so its type is inferred from the value (or the
    // original struct) at the call site. Nested fields still need their own
    // declaration pass (the type lives in an item, the value in an expression).
    ( @build $name:ident [$($recv:tt)*] [$($items:tt)*] [$($fields:ident,)*] [$($init:tt)*] { } ) => {{
        #[allow(non_camel_case_types)]
        mod selection {
            use $crate::__private::{serde as __sd, specta as __sp, unraw as __u};

            $crate::selection!(@type $name [$($items)*] [$($fields,)*]);
        }
        selection::$name { $($init)* }
    }};
    ( @build $name:ident [$($recv:tt)*] [$($items:tt)*] [$($fields:ident,)*] [$($init:tt)*]
        { $field:ident: { $($nested:tt)+ } $(, $($rest:tt)*)? } ) => {
        $crate::selection!(@build $name
            [$($recv)*]
            [$($items)* $crate::selection!(@module $field { $($nested)+ });]
            [$($fields,)* $field,]
            [$($init)* $field: $crate::selection!(@construct
                [$($recv)*.$field] [selection::$field] Selection [] { $($nested)+ }),]
            { $($($rest)*)? })
    };
    ( @build $name:ident [$($recv:tt)*] [$($items:tt)*] [$($fields:ident,)*] [$($init:tt)*]
        { $field:ident: [{ $($nested:tt)+ }] $(, $($rest:tt)*)? } ) => {
        $crate::selection!(@build $name
            [$($recv)*]
            [$($items)* $crate::selection!(@module $field { $($nested)+ });]
            [$($fields,)* $field,]
            [$($init)* $field: $($recv)*.$field
                .into_iter()
                .map(|v| $crate::selection!(@construct
                    [v] [selection::$field] Selection [] { $($nested)+ }))
                .collect::<Vec<_>>(),]
            { $($($rest)*)? })
    };
    ( @build $name:ident [$($recv:tt)*] [$($items:tt)*] [$($fields:ident,)*] [$($init:tt)*]
        { $field:ident: $value:expr $(, $($rest:tt)*)? } ) => {
        $crate::selection!(@build $name
            [$($recv)*] [$($items)*] [$($fields,)* $field,]
            [$($init)* $field: $value,]
            { $($($rest)*)? })
    };
    ( @build $name:ident [$($recv:tt)*] [$($items:tt)*] [$($fields:ident,)*] [$($init:tt)*]
        { $field:ident $(, $($rest:tt)*)? } ) => {
        $crate::selection!(@build $name
            [$($recv)*] [$($items)*] [$($fields,)* $field,]
            [$($init)* $field: $($recv)*.$field,]
            { $($($rest)*)? })
    };

    // ----------------------------------------------------------------------
    // Declaration only (nested modules)
    // ----------------------------------------------------------------------

    ( @declare $name:ident [] [] { $($field:ident),* $(,)? } ) => {
        $crate::selection!(@type $name [] [$($field,)*]);
    };
    ( @declare $name:ident [$($items:tt)*] [$($fields:ident,)*] { } ) => {
        $crate::selection!(@type $name [$($items)*] [$($fields,)*]);
    };
    ( @declare $name:ident [$($items:tt)*] [$($fields:ident,)*]
        { $field:ident: { $($nested:tt)+ } $(, $($rest:tt)*)? } ) => {
        $crate::selection!(@declare $name
            [$($items)* $crate::selection!(@module $field { $($nested)+ });]
            [$($fields,)* $field,]
            { $($($rest)*)? });
    };
    ( @declare $name:ident [$($items:tt)*] [$($fields:ident,)*]
        { $field:ident: [{ $($nested:tt)+ }] $(, $($rest:tt)*)? } ) => {
        $crate::selection!(@declare $name
            [$($items)* $crate::selection!(@module $field { $($nested)+ });]
            [$($fields,)* $field,]
            { $($($rest)*)? });
    };
    ( @declare $name:ident [$($items:tt)*] [$($fields:ident,)*]
        { $field:ident: $value:expr $(, $($rest:tt)*)? } ) => {
        $crate::selection!(@declare $name
            [$($items)*]
            [$($fields,)* $field,]
            { $($($rest)*)? });
    };
    ( @declare $name:ident [$($items:tt)*] [$($fields:ident,)*]
        { $field:ident $(, $($rest:tt)*)? } ) => {
        $crate::selection!(@declare $name
            [$($items)*]
            [$($fields,)* $field,]
            { $($($rest)*)? });
    };

    // ----------------------------------------------------------------------
    // Construction only (nested values)
    // ----------------------------------------------------------------------

    ( @construct [$recv:tt] [$($module:tt)*] $name:ident []
        { $($field:ident),* $(,)? } ) => {
        $($module)*::$name { $($field: $recv.$field),* }
    };
    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*] { } ) => {
        $($module)*::$name { $($init)* }
    };
    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*]
        { $field:ident: { $($nested:tt)+ } $(, $($rest:tt)*)? } ) => {
        $crate::selection!(@construct
            [$($recv)*] [$($module)*] $name
            [$($init)* $field: $crate::selection!(@construct
                [$($recv)*.$field] [$($module)*::$field] Selection [] { $($nested)+ }),]
            { $($($rest)*)? })
    };
    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*]
        { $field:ident: [{ $($nested:tt)+ }] $(, $($rest:tt)*)? } ) => {
        $crate::selection!(@construct
            [$($recv)*] [$($module)*] $name
            [$($init)* $field: $($recv)*.$field
                .into_iter()
                .map(|v| $crate::selection!(@construct
                    [v] [$($module)*::$field] Selection [] { $($nested)+ }))
                .collect::<Vec<_>>(),]
            { $($($rest)*)? })
    };
    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*]
        { $field:ident: $value:expr $(, $($rest:tt)*)? } ) => {
        $crate::selection!(@construct
            [$($recv)*] [$($module)*] $name [$($init)* $field: $value,]
            { $($($rest)*)? })
    };
    ( @construct [$($recv:tt)*] [$($module:tt)*] $name:ident [$($init:tt)*]
        { $field:ident $(, $($rest:tt)*)? } ) => {
        $crate::selection!(@construct
            [$($recv)*] [$($module)*] $name [$($init)* $field: $($recv)*.$field,]
            { $($($rest)*)? })
    };

    // ----------------------------------------------------------------------
    // The type
    // ----------------------------------------------------------------------

    // A selection is a private, non-recursive, always-inline named struct, so
    // describe it directly instead of going through the derive's named-type
    // protocol. Field types still resolve through their own `Type` impl, so
    // named field types stay named references, matching what the derive emits
    // for an inline type.
    ( @type $name:ident [$($items:tt)*] [$($fields:ident,)*] ) => {
        $($items)*

        pub struct $name<$($fields,)*> {
            $(pub $fields: $fields,)*
        }

        impl<$($fields: self::__sd::Serialize,)*> self::__sd::Serialize
            for $name<$($fields,)*>
        {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> ::core::result::Result<__S::Ok, __S::Error>
            where
                __S: self::__sd::Serializer,
            {
                let mut __state = self::__sd::Serializer::serialize_struct(
                    __serializer,
                    __u(stringify!($name)),
                    const { [$(stringify!($fields)),*].len() },
                )?;
                $(self::__sd::ser::SerializeStruct::serialize_field(
                    &mut __state,
                    __u(stringify!($fields)),
                    &self.$fields,
                )?;)*
                self::__sd::ser::SerializeStruct::end(__state)
            }
        }

        impl<$($fields: self::__sp::Type,)*> self::__sp::Type for $name<$($fields,)*>
        {
            fn definition(types: &mut self::__sp::Types) -> self::__sp::datatype::DataType {
                self::__sp::datatype::Struct::named()
                    $(.field(
                        __u(stringify!($fields)),
                        self::__sp::datatype::Field::new($fields::definition(types)),
                    ))*
                    .build()
            }
        }
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
    fn raw_identifier_fields_are_unraw_ed() {
        #[allow(dead_code)]
        struct Keywords {
            r#type: &'static str,
            r#match: u32,
        }

        let value = Keywords {
            r#type: "user",
            r#match: 1,
        };
        let selection = crate::selection!(value, { r#type, r#match });

        assert_eq!(
            serde_json::to_value(&selection).expect("selection should serialize"),
            serde_json::json!({ "type": "user", "match": 1 })
        );

        let mut types = specta::Types::default();
        fn datatype_of<T: specta::Type>(
            _: &T,
            types: &mut specta::Types,
        ) -> specta::datatype::DataType {
            T::definition(types)
        }
        fn field_names(datatype: &specta::datatype::DataType) -> Vec<&str> {
            match datatype {
                specta::datatype::DataType::Struct(struct_ty) => match &struct_ty.fields {
                    specta::datatype::Fields::Named(fields) => fields
                        .fields
                        .iter()
                        .map(|(name, _)| name.as_ref())
                        .collect(),
                    _ => Vec::new(),
                },
                _ => Vec::new(),
            }
        }

        let datatype = datatype_of(&selection, &mut types);
        assert_eq!(field_names(&datatype), ["type", "match"]);
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
