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
//! obvious structural type (`{ code: number; page: number; tags: [string, string] }`).
//! Interpolated expressions keep their *real* type: each rides a generic type
//! parameter inferred at the call site, so `ty!({ "user": some_user })` types `user`
//! as `some_user`'s own [`Type`](specta::Type). Complex expressions — field access,
//! method calls — work without parentheses.
//!
//! # Scope and caveats
//!
//! * Arrays type as a **tuple** of their elements (`[1, "x"]` → `[number, string]`),
//!   which stays honest for heterogeneous arrays.
//! * `null` maps to `()` (serializes as `null`, types as the unit/null type).
//! * A large literal expands to deeply nested items; bump `#![recursion_limit]` if
//!   you hit it.

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
    datatype::{DataType, Field, Struct, Tuple},
};

/// The empty tail of an object or array field list.
#[doc(hidden)]
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

/// A heterogeneous cons-list of array elements. Types as a tuple.
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
#[doc(hidden)]
pub struct Array<P>(pub P);

impl<P: SeqPart> Type for Array<P> {
    fn definition(types: &mut Types) -> DataType {
        let mut elements = Vec::new();
        <P as SeqPart>::extend_type(&mut elements, types);
        DataType::Tuple(Tuple::new(elements))
    }
}

impl<P: SeqPart> Serialize for Array<P> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(P::LEN))?;
        self.0.ser_elems(&mut seq)?;
        seq.end()
    }
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
    (@obj_val $key:literal [$($val:tt)*] , $($rest:tt)*) => {
        $crate::ty!(@node $key, $crate::ty!($($val)*), $crate::ty!(@obj $($rest)*))
    };
    (@obj_val $key:literal [$($val:tt)*] $next:tt $($rest:tt)*) => {
        $crate::ty!(@obj_val $key [$($val)* $next] $($rest)*)
    };
    (@obj_val $key:literal [$($val:tt)*]) => {
        $crate::ty!(@node $key, $crate::ty!($($val)*), $crate::ty::__macro::Nil)
    };
    (@node $key:literal, $head:expr, $tail:expr) => {{
        use $crate::ty::__macro::{Field, ObjPart, Serialize, SerializeMap, Type, Types};
        let head = $head;
        let tail = $tail;
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
        use $crate::ty::__macro::{DataType, SeqPart, Serialize, SerializeSeq, Type, Types};
        let head = $head;
        let tail = $tail;
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
    }};

    // ---- public leaves ----
    (null) => { () };
    ({}) => { $crate::ty::__macro::Object($crate::ty::__macro::Nil) };
    ({ $($obj:tt)+ }) => { $crate::ty::__macro::Object($crate::ty!(@obj $($obj)+)) };
    ([]) => { $crate::ty::__macro::Array($crate::ty::__macro::Nil) };
    ([ $($arr:tt)+ ]) => { $crate::ty::__macro::Array($crate::ty!(@arr $($arr)+)) };
    // a scalar literal or an interpolated expression (bare `a.b` / `f()` included)
    ($($other:tt)+) => { $($other)+ };
}

#[cfg(test)]
mod tests {
    use specta::{
        Type, Types,
        datatype::{DataType, Primitive},
    };

    fn def_of<T: Type>(_v: &T) -> DataType {
        T::definition(&mut Types::default())
    }
    fn json_of<T: serde::Serialize>(v: &T) -> String {
        serde_json::to_string(v).unwrap()
    }

    #[test]
    fn scalars_and_null() {
        assert_eq!(json_of(&crate::ty!(200)), "200");
        assert_eq!(json_of(&crate::ty!("hi")), r#""hi""#);
        assert_eq!(json_of(&crate::ty!(true)), "true");
        assert_eq!(json_of(&crate::ty!(null)), "null");
        assert_eq!(def_of(&crate::ty!(200)), Primitive::i32.into());
    }

    #[test]
    fn objects_and_nesting() {
        let v = crate::ty!({
            "code": 200,
            "meta": { "ok": true, "tag": "x" },
        });
        assert_eq!(json_of(&v), r#"{"code":200,"meta":{"ok":true,"tag":"x"}}"#);
        let d = format!("{:#?}", def_of(&v));
        assert!(matches!(def_of(&v), DataType::Struct(_)));
        assert!(d.contains("code") && d.contains("meta") && d.contains("ok") && d.contains("tag"));
    }

    #[test]
    fn arrays_type_as_tuples() {
        assert_eq!(json_of(&crate::ty!([1, 2, 3])), "[1,2,3]");
        assert_eq!(json_of(&crate::ty!([1, "x", true])), r#"[1,"x",true]"#);
        assert_eq!(json_of(&crate::ty!([[1], [2, 3]])), "[[1],[2,3]]");
        let d = format!("{:#?}", def_of(&crate::ty!([1, "x", true])));
        assert!(
            d.contains("Tuple") && d.contains("i32") && d.contains("str") && d.contains("bool")
        );
    }

    #[test]
    fn empties() {
        assert_eq!(json_of(&crate::ty!({})), "{}");
        assert_eq!(json_of(&crate::ty!([])), "[]");
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
}
