//! An experimental `ty!` macro that builds a value implementing [`specta::Type`]
//! from a `serde_json::json!`-style literal.
//!
//! Unlike [`serde_json::json!`], the value is not a `serde_json::Value` — it is a
//! purpose-built value that implements both [`serde::Serialize`] (so it round-trips
//! to *any* format, not just JSON) and [`specta::Type`] (so Specta can describe its
//! shape). No dependency on `serde_json`.
//!
//! ```
//! # use specta_util::ty;
//! let page = 2;
//! let _value = ty!({
//!     "code": 200,
//!     "page": page,          // interpolation: `page`'s type is captured
//!     "tags": ["serde", "specta"],
//! });
//! ```
//!
//! # What it captures
//!
//! Object keys and literal leaves are known at macro-expansion time and become the
//! obvious structural type (`{ code: number; page: number; tags: string[] }`).
//! Interpolated expressions keep their *real* type: each rides a generic type
//! parameter inferred at the call site, so `ty!({ "user": some_user })` types `user`
//! as `some_user`'s own [`Type`](specta::Type). Complex expressions — field access,
//! method calls — work without parentheses.
//!
//! # Arrays
//!
//! A bracket literal describes a JSON array, so it only becomes a list when every
//! element shares one type:
//!
//! * Elements all the same type → a list (`[1, 2, 3]` → `number[]`).
//! * Mixed element types → a tuple, the honest fixed-shape description
//!   (`[1, "x"]` → `[number, string]`).
//! * Empty → a list of the bottom type (`never[]`), because an empty array is an
//!   empty list rather than
//!   [`null`](https://www.typescriptlang.org/docs/handbook/2/everyday-types.html#null-and-undefined).
//!
//! Elements are compared as an exporter would render them, so a transparent alias
//! counts as the type it aliases: `ty!(["a", owned_string])` is `string[]` even
//! though `&str` and `String` are different Rust types.
//!
//! ```
//! # use specta_util::ty;
//! let tags: Vec<String> = vec!["serde".into(), "specta".into()];
//! let _list = ty!({ "tags": tags });       // tags: string[]
//! let _literal = ty!({ "pair": [1, 2] });  // pair: number[]
//! let _mixed = ty!({ "pair": [1, "x"] });  // pair: [number, string]
//! let _empty = ty!({ "pair": [] });        // pair: never[]
//! ```
//!
//! # Scope and caveats
//!
//! * `null` maps to `()` — it serializes as `null` and types as the unit type.
//! * Object keys must be string literals, must be unique, and cannot be interpolated
//!   expressions. A repeated key is emitted as a repeated field, which most exporters
//!   reject rather than resolve, so a duplicate is a mistake in the target language.
//! * A brace group is only read as an object literal when its first entry looks like
//!   `"key": value`; any other brace group is passed through as an expression, so a
//!   **bare block used as a value works**: `ty!({ "x": { 1 + 1 } })` is the block
//!   `{ 1 + 1 }`, and `ty!({ "x": {}})` is still an empty object. A missing `:` is
//!   read as a block, not as a malformed object.
//! * A value ends at a top-level `,`, so an expression containing one has to be bound
//!   to a variable or wrapped in a call first. This affects a turbofish with several
//!   parameters and a closure with several parameters — write `ty!({ "map": map })`
//!   rather than `ty!({ "map": HashMap::<K, V>::new() })`.
//! * An empty array is a list of Specta's bottom type. The exporters with a bottom
//!   type render it (TypeScript's `never[]`, Zod's `z.never()`, Python's `Never`,
//!   JSON Schema's `false`); a target without one may reject or widen it.
//! * A large literal expands to deeply nested items, so it is bounded by
//!   `#![recursion_limit]`: at the default of 128 a literal fits at most 31 entries
//!   (fields or array elements) before the compiler asks you to raise the limit.
//!
//! # Getting a [`specta::Type`] out
//!
//! The value's type is unnamed and unnameable, so [`Types::register`] cannot be
//! called on it directly. Infer it from a value instead:
//!
//! ```
//! # use specta_util::ty;
//! # use specta::{Type, Types, datatype::DataType};
//! let value = ty!({ "ok": true });
//!
//! fn definition_of<T: Type>(types: &mut Types, _: &T) -> DataType {
//!     T::definition(types)
//! }
//!
//! let definition = definition_of(&mut Types::default(), &value);
//! assert!(matches!(definition, DataType::Struct(_)));
//! ```

// The generated code reaches everything through this module, so a downstream
// `ty!` resolves regardless of what is (or isn't) imported at the call site.
#[doc(hidden)]
pub mod __macro {
    pub use serde::{
        Serialize, Serializer,
        ser::{SerializeMap, SerializeSeq},
    };
    pub use specta::{
        Type, Types,
        datatype::{DataType, Field, Struct, Tuple},
    };

    pub use super::{Array, Nil, ObjPart, Object, SeqPart};
}

use serde::{
    Serialize, Serializer,
    ser::{SerializeMap, SerializeSeq},
};
use specta::{
    Type, Types,
    datatype::{DataType, Enum, Field, List, NamedReferenceType, Reference, Struct, Tuple},
};

/// The empty tail of an object or array field list.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Nil;

/// A heterogeneous cons-list of object fields. Each value keeps its concrete type
/// so [`Type`] sees the real (possibly interpolated) type and [`Serialize`] stays
/// format-agnostic.
#[doc(hidden)]
pub trait ObjPart {
    const LEN: usize;
    fn extend_type(fields: &mut Vec<(&'static str, Field)>, types: &mut Types);
    fn ser_fields<M: SerializeMap>(&self, map: &mut M) -> Result<(), M::Error>;
}

impl ObjPart for Nil {
    const LEN: usize = 0;
    fn extend_type(_fields: &mut Vec<(&'static str, Field)>, _types: &mut Types) {}
    fn ser_fields<M: SerializeMap>(&self, _map: &mut M) -> Result<(), M::Error> {
        Ok(())
    }
}

/// Wraps an object's field list into a value implementing [`Type`] + [`Serialize`].
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Object<P>(pub P);

impl<P: ObjPart> Type for Object<P> {
    fn definition(types: &mut Types) -> DataType {
        let mut fields = Vec::new();
        <P as ObjPart>::extend_type(&mut fields, types);
        let mut builder = Struct::named();
        for (key, field) in fields {
            builder = builder.field(key, field);
        }
        builder.build()
    }
}

impl<P: ObjPart> Serialize for Object<P> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(P::LEN))?;
        self.0.ser_fields(&mut map)?;
        map.end()
    }
}

/// A heterogeneous cons-list of array elements.
#[doc(hidden)]
pub trait SeqPart {
    const LEN: usize;
    fn extend_type(elements: &mut Vec<DataType>, types: &mut Types);
    fn ser_elems<S: SerializeSeq>(&self, seq: &mut S) -> Result<(), S::Error>;
}

impl SeqPart for Nil {
    const LEN: usize = 0;
    fn extend_type(_elements: &mut Vec<DataType>, _types: &mut Types) {}
    fn ser_elems<S: SerializeSeq>(&self, _seq: &mut S) -> Result<(), S::Error> {
        Ok(())
    }
}

/// Wraps an array's element list into a value implementing [`Type`] + [`Serialize`].
///
/// A literal only becomes a [`List`] when every element shares one type, since a list
/// has a single element type. Mixed literals keep the tuple representation, which is
/// the accurate description of a fixed heterogeneous sequence.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Array<P>(pub P);

impl<P: SeqPart> Type for Array<P> {
    fn definition(types: &mut Types) -> DataType {
        let mut elements = Vec::new();
        <P as SeqPart>::extend_type(&mut elements, types);

        // An empty literal is an empty *list*. The unit type it would otherwise become
        // serialises as `null` and exports as `null`, which does not describe an array.
        if elements.is_empty() {
            return DataType::List(List::new(never()));
        }

        // A list has one element type, so the elements are compared as an exporter would
        // render them rather than by Rust type identity.
        let homogeneous = {
            let first = resolved(&elements[0]);
            elements
                .iter()
                .skip(1)
                .all(|element| resolved(element) == first)
        };

        if homogeneous {
            DataType::List(List::new(elements.remove(0)))
        } else {
            DataType::Tuple(Tuple::new(elements))
        }
    }
}

impl<P: SeqPart> Serialize for Array<P> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(P::LEN))?;
        self.0.ser_elems(&mut seq)?;
        seq.end()
    }
}

/// The element type of an empty array literal.
///
/// Specta has no dedicated bottom type, so an [`Enum`] with no variants stands in. It
/// is the representation exporters already render as "never" (TypeScript's `never`,
/// Zod's `z.never()`, Python's `Never`, JSON Schema's `false`), which is what an array
/// that may not hold anything is.
fn never() -> DataType {
    DataType::Enum(Enum::default())
}

/// The [`DataType`] an exporter would render for `dt`.
///
/// A named reference carrying an inline datatype — a transparent alias such as
/// [`String`], `smol_str::SmolStr` or `semver::Version`, all of which render as a
/// string — is unwrapped to the datatype it stands for. Comparing the resolved shape
/// keeps the list/tuple decision about the elements' *type* rather than about which
/// Rust type was written, so `ty!(["a", owned_string])` is still `string[]`.
fn resolved(dt: &DataType) -> &DataType {
    let mut current = dt;
    // Bounded: alias chains are short, and a self-referential one must not spin forever.
    for _ in 0..16 {
        let DataType::Reference(Reference::Named(named)) = current else {
            break;
        };
        let NamedReferenceType::Inline { dt, .. } = &named.inner else {
            break;
        };
        current = dt;
    }
    current
}

/// Build a value that implements [`specta::Type`] from a `json!`-style literal.
///
/// See the [module docs](crate::ty) for the supported subset and the interpolation
/// rules.
///
/// ```
/// # use specta_util::ty;
/// let _value = ty!({ "ok": true, "items": [1, 2, 3] });
/// ```
#[macro_export]
macro_rules! ty {
    // ---- object: peel `key :`, then accumulate value tokens up to a top-level comma ----
    (@obj) => { $crate::ty::__macro::Nil };
    (@obj $key:literal : $($rest:tt)*) => { $crate::ty!(@obj_val $key [] $($rest)*) };
    // Anything else is not an entry. The common cause is a value that was split on a
    // top-level comma, which leaves its second half here.
    (@obj $($unexpected:tt)+) => {
        compile_error!(
            "ty!: expected an object entry (`\"key\": value`) here. A value containing a \
             top-level comma — such as `HashMap::<K, V>::new()` or a multi-parameter closure \
             — is split at the comma; bind it to a variable or wrap it in a call first."
        )
    };
    (@obj_val $key:literal [] , $($rest:tt)*) => {
        compile_error!("ty!: missing a value for the preceding object key (`\"key\": value`)")
    };
    (@obj_val $key:literal [$($val:tt)*] , $($rest:tt)*) => {
        $crate::ty!(@node $key, $crate::ty!($($val)*), $crate::ty!(@obj $($rest)*))
    };
    (@obj_val $key:literal [$($val:tt)*] $next:tt $($rest:tt)*) => {
        $crate::ty!(@obj_val $key [$($val)* $next] $($rest)*)
    };
    (@obj_val $key:literal []) => {
        compile_error!("ty!: missing a value for the preceding object key (`\"key\": value`)")
    };
    (@obj_val $key:literal [$($val:tt)*]) => {
        $crate::ty!(@node $key, $crate::ty!($($val)*), $crate::ty::__macro::Nil)
    };
    (@node $key:literal, $head:expr, $tail:expr) => {{
        // The interpolated expressions are evaluated here, in a block with none of the
        // items below in scope. Otherwise a user type named `Type`, `Field`, `Node`,
        // ... would be shadowed by this expansion.
        let head = $head;
        let tail = $tail;
        {
            use $crate::ty::__macro::{Field, ObjPart, Serialize, SerializeMap, Type, Types};
            #[derive(Debug, Clone, PartialEq, Eq, Hash)]
            struct Node<H, T> {
                head: H,
                tail: T,
            }
            impl<H: Type + Serialize, T: ObjPart> ObjPart for Node<H, T> {
                const LEN: usize = 1 + <T as ObjPart>::LEN;
                fn extend_type(fields: &mut Vec<(&'static str, Field)>, types: &mut Types) {
                    fields.push(($key, Field::new(<H as Type>::definition(types))));
                    <T as ObjPart>::extend_type(fields, types);
                }
                fn ser_fields<M: SerializeMap>(&self, map: &mut M) -> Result<(), M::Error> {
                    map.serialize_entry($key, &self.head)?;
                    <T as ObjPart>::ser_fields(&self.tail, map)
                }
            }
            Node { head, tail }
        }
    }};

    // ---- array: accumulate each element's tokens up to a top-level comma ----
    (@arr) => { $crate::ty::__macro::Nil };
    (@arr $($rest:tt)+) => { $crate::ty!(@arr_val [] $($rest)+) };
    (@arr_val [$($val:tt)*] , $($rest:tt)*) => {
        $crate::ty!(@seq $crate::ty!($($val)*), $crate::ty!(@arr $($rest)*))
    };
    (@arr_val [$($val:tt)*] $next:tt $($rest:tt)*) => {
        $crate::ty!(@arr_val [$($val)* $next] $($rest)*)
    };
    (@arr_val [$($val:tt)*]) => {
        $crate::ty!(@seq $crate::ty!($($val)*), $crate::ty::__macro::Nil)
    };
    (@seq $head:expr, $tail:expr) => {{
        // See `@node` — the expressions are evaluated outside the item scope.
        let head = $head;
        let tail = $tail;
        {
            use $crate::ty::__macro::{DataType, SeqPart, Serialize, SerializeSeq, Type, Types};
            #[derive(Debug, Clone, PartialEq, Eq, Hash)]
            struct SeqNode<H, T> {
                head: H,
                tail: T,
            }
            impl<H: Type + Serialize, T: SeqPart> SeqPart for SeqNode<H, T> {
                const LEN: usize = 1 + <T as SeqPart>::LEN;
                fn extend_type(elements: &mut Vec<DataType>, types: &mut Types) {
                    elements.push(<H as Type>::definition(types));
                    <T as SeqPart>::extend_type(elements, types);
                }
                fn ser_elems<S: SerializeSeq>(&self, seq: &mut S) -> Result<(), S::Error> {
                    seq.serialize_element(&self.head)?;
                    <T as SeqPart>::ser_elems(&self.tail, seq)
                }
            }
            SeqNode { head, tail }
        }
    }};

    // ---- public leaves ----
    (null) => { () };
    ({}) => { $crate::ty::__macro::Object($crate::ty::__macro::Nil) };
    // Only a string literal followed by `:` can start an entry, so a brace group that
    // is not object-shaped — a block expression — falls through to the arm below.
    ({ $key:literal : $($obj:tt)* }) => {
        $crate::ty::__macro::Object($crate::ty!(@obj $key : $($obj)*))
    };
    ([]) => { $crate::ty::__macro::Array($crate::ty::__macro::Nil) };
    ([ $($arr:tt)+ ]) => { $crate::ty::__macro::Array($crate::ty!(@arr $($arr)+)) };
    // A single value never has a top-level comma, so finding one means the input was
    // meant to be two entries (or two array elements).
    ($first:expr , $($rest:tt)+) => {
        compile_error!(
            "ty! takes a single value; `,` only separates entries inside a `{{ }}` or `[ ]` literal"
        )
    };
    // a scalar literal or an interpolated expression (bare `a.b` / `f()` included)
    ($($other:tt)+) => { $($other)+ };
}

#[cfg(test)]
mod tests {
    use specta::{
        Type, Types,
        datatype::{DataType, Enum, Fields, List, NamedFields, Primitive, Tuple},
    };

    fn def_of<T: Type>(_v: &T) -> DataType {
        T::definition(&mut Types::default())
    }
    fn json_of<T: serde::Serialize>(v: &T) -> String {
        serde_json::to_string(v).expect("literal should serialize")
    }
    /// The element type of a [`DataType::List`], if it is one.
    fn list_element(dt: &DataType) -> Option<&DataType> {
        match dt {
            DataType::List(list) => Some(&list.ty),
            _ => None,
        }
    }
    /// The element types of a [`DataType::Tuple`], if it is one.
    fn tuple_elements(dt: &DataType) -> Option<&[DataType]> {
        match dt {
            DataType::Tuple(tuple) => Some(&tuple.elements),
            _ => None,
        }
    }
    /// The named fields of a [`DataType::Struct`], if it is one.
    fn named_fields(dt: &DataType) -> Option<&NamedFields> {
        match dt {
            DataType::Struct(s) => match &s.fields {
                Fields::Named(fields) => Some(fields),
                _ => None,
            },
            _ => None,
        }
    }

    #[test]
    fn scalars_and_null() {
        assert_eq!(json_of(&crate::ty!(200)), "200");
        assert_eq!(json_of(&crate::ty!("hi")), r#""hi""#);
        assert_eq!(json_of(&crate::ty!(true)), "true");
        assert_eq!(json_of(&crate::ty!(null)), "null");
        assert_eq!(def_of(&crate::ty!(200)), Primitive::i32.into());
        // `null` is the unit type, which is an empty tuple.
        assert_eq!(
            def_of(&crate::ty!(null)),
            DataType::Tuple(Tuple::new(vec![]))
        );
    }

    #[test]
    fn objects_and_nesting() {
        let v = crate::ty!({
            "code": 200,
            "meta": { "ok": true, "tag": "x" },
        });
        assert_eq!(json_of(&v), r#"{"code":200,"meta":{"ok":true,"tag":"x"}}"#);

        // Source order is preserved for both the type and the serialized value.
        let definition = def_of(&v);
        let fields = named_fields(&definition).expect("object literal should have named fields");
        let names = fields
            .fields
            .iter()
            .map(|(name, _)| &**name)
            .collect::<Vec<_>>();
        assert_eq!(names, ["code", "meta"]);
    }

    #[test]
    fn homogeneous_arrays_are_lists() {
        let three = crate::ty!([1, 2, 3]);
        let one = crate::ty!([1]);
        assert_eq!(
            list_element(&def_of(&three)),
            Some(&DataType::Primitive(Primitive::i32)),
        );
        assert_eq!(
            list_element(&def_of(&one)),
            Some(&DataType::Primitive(Primitive::i32)),
        );

        // Two structurally identical literals still share one element type.
        let nested = crate::ty!([[1], [2, 3]]);
        assert!(matches!(
            list_element(&def_of(&nested)),
            Some(DataType::List(_))
        ));

        // A transparent alias counts as the type it aliases, so a literal and a `String`
        // are one element type even though the Rust types differ.
        let owned: String = "a".into();
        let aliased = crate::ty!(["lit", owned.clone()]);
        assert_eq!(
            list_element(&def_of(&aliased)),
            Some(&DataType::Primitive(Primitive::str)),
        );
        // But genuinely different element types stay a tuple.
        assert!(tuple_elements(&def_of(&crate::ty!([1, "x"]))).is_some());
    }

    #[test]
    fn heterogeneous_arrays_are_tuples() {
        let v = crate::ty!([1, "x", true]);
        assert_eq!(json_of(&v), r#"[1,"x",true]"#);
        assert_eq!(
            tuple_elements(&def_of(&v)),
            Some(
                [
                    DataType::Primitive(Primitive::i32),
                    DataType::Primitive(Primitive::str),
                    DataType::Primitive(Primitive::bool),
                ]
                .as_slice()
            ),
        );
    }

    /// `[]` is an empty list, not the unit type — the unit type would export as `null`.
    #[test]
    fn empty_array_is_an_empty_list() {
        assert_eq!(json_of(&crate::ty!([])), "[]");

        // An empty enum is the bottom type Specta has, which exporters render as `never`.
        assert_eq!(
            list_element(&def_of(&crate::ty!([]))),
            Some(&DataType::Enum(Enum::default())),
        );
    }

    #[test]
    fn arrays_nested_in_objects() {
        let v = crate::ty!({
            "outer": [[1, 2], [3]],
            "after": 1,
            "last": [1, 2],
        });
        assert_eq!(
            json_of(&v),
            r#"{"outer":[[1,2],[3]],"after":1,"last":[1,2]}"#
        );

        // Each array became a list of numbers.
        let definition = def_of(&v);
        let fields = named_fields(&definition).expect("object literal should have named fields");
        let number_list = DataType::List(List::new(Primitive::i32.into()));
        let outer = fields
            .fields
            .iter()
            .map(|(name, field)| (&**name, field.ty.clone().expect("field should have a type")))
            .collect::<Vec<_>>();
        assert_eq!(outer[0], ("outer", DataType::List(List::new(number_list))));
        assert_eq!(outer[1], ("after", Primitive::i32.into()));
        assert_eq!(
            outer[2],
            ("last", DataType::List(List::new(Primitive::i32.into())))
        );
    }

    #[test]
    fn empties() {
        assert_eq!(json_of(&crate::ty!({})), "{}");
        assert_eq!(json_of(&crate::ty!([])), "[]");
        assert_eq!(
            json_of(&crate::ty!({ "a": {}, "b": [] })),
            r#"{"a":{},"b":[]}"#
        );

        assert!(matches!(def_of(&crate::ty!({})), DataType::Struct(_)));
    }

    #[test]
    fn trailing_commas_are_allowed() {
        assert_eq!(json_of(&crate::ty!([1, 2, 3,])), "[1,2,3]");
        assert_eq!(
            json_of(&crate::ty!({ "a": 1, "b": 2, })),
            r#"{"a":1,"b":2}"#
        );
        assert_eq!(json_of(&crate::ty!({ "a": [1, 2,], })), r#"{"a":[1,2]}"#);
    }

    /// A brace group that is not object-shaped is an expression, so a bare block works.
    #[test]
    fn bare_blocks_are_expressions() {
        let v = crate::ty!({
            "block": { let x = 2; x + 1 },
            "nested": { let y = 4; y * 2 },
        });
        assert_eq!(json_of(&v), r#"{"block":3,"nested":8}"#);
    }

    /// Building a `ty!` from another `ty!` keeps the inner type.
    #[test]
    fn nested_invocations() {
        let inner = crate::ty!({ "a": 1 });
        let v = crate::ty!({ "inner": inner, "b": 2 });
        assert_eq!(json_of(&v), r#"{"inner":{"a":1},"b":2}"#);
    }

    /// Keys are not restricted to identifiers — they are emitted verbatim.
    #[test]
    fn non_identifier_keys_are_preserved() {
        let v = crate::ty!({ "foo-bar": 1, "2x": 2 });
        assert_eq!(json_of(&v), r#"{"foo-bar":1,"2x":2}"#);

        let definition = def_of(&v);
        let fields = named_fields(&definition).expect("object literal should have named fields");
        assert_eq!(&*fields.fields[0].0, "foo-bar");
    }

    /// Interpolation of std-typed values (no derive needed): the value's own
    /// `Type` and `Serialize` flow through.
    #[test]
    fn interpolates_values_and_bare_exprs() {
        let name: String = "ada".into();
        let ids: Vec<u32> = vec![1, 2, 3];
        let flag = true;

        let v = crate::ty!({
            "name": name.clone(),
            "ids": ids,
            "ok": flag,
            "len": name.len(),          // bare method call, no parens
        });
        assert_eq!(
            json_of(&v),
            r#"{"name":"ada","ids":[1,2,3],"ok":true,"len":3}"#
        );
        // `ids` kept its List type, not a widened `any`
        let d = format!("{:#?}", def_of(&v));
        assert!(
            d.contains("List"),
            "interpolated Vec should keep its List type: {d}"
        );
    }

    /// A user-derived type interpolated: its real named type survives.
    #[cfg(feature = "serde")]
    #[test]
    fn interpolates_a_derived_type() {
        #[derive(serde::Serialize, specta::Type)]
        struct User {
            id: u32,
            name: String,
        }

        let user = User {
            id: 1,
            name: "ada".into(),
        };
        let v = crate::ty!({ "user": user });
        assert_eq!(json_of(&v), r#"{"user":{"id":1,"name":"ada"}}"#);
        let d = format!("{:#?}", def_of(&v));
        assert!(d.contains("User"), "interpolated User type was lost: {d}");
    }

    /// The generated value is a normal value.
    #[test]
    fn values_are_debug_clone_eq() {
        let v = crate::ty!({ "a": [1, 2], "b": "x" });
        assert_eq!(v.clone(), v);
        assert!(!format!("{v:?}").is_empty());
    }

    /// The expansion defines items (`Node`, and `use`s of `Type`/`Field`/...). Those
    /// must not be visible to the interpolated expressions, which resolve in the
    /// caller's scope.
    #[test]
    fn macro_internals_do_not_shadow_caller_items() {
        struct Node;
        impl Node {
            fn value(&self) -> u32 {
                7
            }
        }
        struct Field(u32);
        struct Types;

        let node = Node;
        let v = crate::ty!({
            "node": node.value(),
            "field": Field(1).0,
            "types": std::mem::size_of::<Types>(),
        });
        assert_eq!(
            json_of(&v),
            format!(
                r#"{{"node":7,"field":1,"types":{}}}"#,
                std::mem::size_of::<Types>()
            )
        );
    }
}
