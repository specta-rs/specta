//! The traversal contract of specta::Format.
//!
//! Every exporter formats through specta::Types::format, so these tests pin the
//! properties an exporter relies on rather than the behaviour of one exporter.

use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
};

use specta::{
    Format, FormatError, Type, Types,
    datatype::{DataType, Primitive},
};

#[derive(Type)]
#[specta(collect = false)]
struct Item {
    value: u32,
}

#[derive(Type)]
#[specta(collect = false)]
struct Other {
    name: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct Generic<T> {
    value: T,
}

/// A formatter whose collection mapping replaces the graph with one it borrows.
struct BorrowedAlternative {
    graph: Types,
}

impl Format for BorrowedAlternative {
    fn map_types<'a>(&'a self, _: &'a Types) -> Result<Cow<'a, Types>, FormatError> {
        Ok(Cow::Borrowed(&self.graph))
    }

    fn map_type<'a>(
        &'a self,
        _: &'a Types,
        dt: &'a DataType,
    ) -> Result<Cow<'a, DataType>, FormatError> {
        Ok(Cow::Borrowed(dt))
    }
}

/// Reproduced a silent divergence: some exporters discarded a borrowed graph
/// returned by map_types and exported the input instead.
#[test]
fn a_borrowed_replacement_graph_is_honoured() {
    let replacement = Types::default().register::<Other>();
    let formatted = Types::default()
        .format(&BorrowedAlternative { graph: replacement })
        .expect("formatting succeeds");

    assert!(
        formatted.iter_sorted().any(|ndt| ndt.name == "Other"),
        "the graph returned by map_types must be the graph that is rendered"
    );
}

/// Records every datatype handed to the per-node mapping.
struct Records {
    nodes: RefCell<Vec<DataType>>,
    calls: Cell<usize>,
}

impl Format for Records {
    fn map_types<'a>(&'a self, types: &'a Types) -> Result<Cow<'a, Types>, FormatError> {
        Ok(Cow::Borrowed(types))
    }

    fn map_type<'a>(
        &'a self,
        _: &'a Types,
        dt: &'a DataType,
    ) -> Result<Cow<'a, DataType>, FormatError> {
        self.calls.set(self.calls.get() + 1);
        self.nodes.borrow_mut().push(dt.clone());
        Ok(Cow::Borrowed(dt))
    }
}

#[test]
fn generic_placeholders_are_never_passed_to_the_formatter() {
    // Resolve a concrete instantiation so the declaration is registered.
    let types = Types::default().register::<Generic<u32>>();
    let records = Records {
        nodes: RefCell::default(),
        calls: Cell::new(0),
    };
    let _ = types.format(&records).expect("formatting succeeds");

    let nodes = records.nodes.into_inner();
    assert!(
        !nodes.iter().any(|dt| matches!(dt, DataType::Generic(_))),
        "a formatter must not receive an unsubstituted generic: {nodes:?}"
    );
}

/// A root can be an anonymous type. It must be formatted too, or exporters that
/// render a root without a declaration would render an unformatted shape.
#[test]
fn roots_are_formatted() {
    struct U32AsString;

    impl Format for U32AsString {
        fn map_types<'a>(&'a self, types: &'a Types) -> Result<Cow<'a, Types>, FormatError> {
            Ok(Cow::Borrowed(types))
        }

        fn map_type<'a>(
            &'a self,
            _: &'a Types,
            dt: &'a DataType,
        ) -> Result<Cow<'a, DataType>, FormatError> {
            Ok(match dt {
                DataType::Primitive(Primitive::u32) => {
                    Cow::Owned(DataType::Primitive(Primitive::str))
                }
                _ => Cow::Borrowed(dt),
            })
        }
    }

    let types = Types::default().register::<u32>();
    let formatted = types.format(&U32AsString).expect("formatting succeeds");

    assert_eq!(
        formatted.roots().next(),
        Some(&DataType::Primitive(Primitive::str))
    );
}

/// map_type must run exactly once per node: running it twice would let a
/// non-idempotent formatter (a rename, a wrapper) apply twice.
#[test]
fn the_formatter_runs_once_per_node() {
    let types = Types::default().register::<Item>();
    let records = Records {
        nodes: RefCell::default(),
        calls: Cell::new(0),
    };
    let _ = types.format(&records).expect("formatting succeeds");

    // The root reference, the Item declaration body, and its single field.
    assert_eq!(records.calls.get(), 3, "{:?}", records.nodes.borrow());
}

#[derive(Type)]
#[specta(collect = false)]
struct Leaves {
    values: Vec<u8>,
}

/// A formatter that clones every node but only rewrites one leaf must still
/// reach that leaf. An owned result equal to its input is unchanged.
#[test]
fn an_owned_unchanged_node_is_still_descended_into() {
    struct LeafRewrite;

    impl Format for LeafRewrite {
        fn map_types<'a>(&'a self, types: &'a Types) -> Result<Cow<'a, Types>, FormatError> {
            Ok(Cow::Owned(types.clone()))
        }

        fn map_type<'a>(
            &'a self,
            _: &'a Types,
            dt: &'a DataType,
        ) -> Result<Cow<'a, DataType>, FormatError> {
            Ok(Cow::Owned(match dt {
                DataType::Primitive(Primitive::u8) => DataType::Primitive(Primitive::str),
                _ => dt.clone(),
            }))
        }
    }

    let formatted = Types::default()
        .register::<Leaves>()
        .format(&LeafRewrite)
        .expect("formatting succeeds");

    let body = formatted
        .iter_sorted()
        .find(|ndt| ndt.name == "Leaves")
        .and_then(|ndt| ndt.ty.as_ref())
        .expect("Leaves is exported");
    let DataType::Struct(data) = body else {
        panic!("expected a struct, got {body:?}");
    };
    let specta::datatype::Fields::Named(named) = &data.fields else {
        panic!("expected named fields");
    };
    let field = named
        .fields
        .iter()
        .find(|(name, _)| name == "values")
        .and_then(|(_, field)| field.ty.as_ref())
        .expect("values field");
    let DataType::List(list) = inline_datatype(field) else {
        panic!("expected a list, got {field:?}");
    };
    assert_eq!(*list.ty, DataType::Primitive(Primitive::str));
}

/// A formatter that rewrites a whole subtree owns its result; the traversal
/// must not format the result again.
#[test]
fn an_owned_changed_node_is_not_formatted_again() {
    struct SubtreeRewrite;

    impl Format for SubtreeRewrite {
        fn map_types<'a>(&'a self, types: &'a Types) -> Result<Cow<'a, Types>, FormatError> {
            Ok(Cow::Owned(types.clone()))
        }

        fn map_type<'a>(
            &'a self,
            _: &'a Types,
            dt: &'a DataType,
        ) -> Result<Cow<'a, DataType>, FormatError> {
            let mut dt = dt.clone();
            rewrite_leaves(&mut dt);
            Ok(Cow::Owned(dt))
        }
    }

    fn rewrite_leaves(dt: &mut DataType) {
        match dt {
            DataType::Primitive(Primitive::u8) => *dt = DataType::Primitive(Primitive::str),
            DataType::Primitive(Primitive::str) => *dt = DataType::Primitive(Primitive::bool),
            DataType::Struct(data) => {
                if let specta::datatype::Fields::Named(named) = &mut data.fields {
                    for (_, field) in &mut named.fields {
                        if let Some(ty) = &mut field.ty {
                            rewrite_leaves(ty);
                        }
                    }
                }
            }
            DataType::List(list) => rewrite_leaves(&mut list.ty),
            _ => {}
        }
    }

    let formatted = Types::default()
        .register::<Leaves>()
        .format(&SubtreeRewrite)
        .expect("formatting succeeds");

    let body = formatted
        .iter_sorted()
        .find(|ndt| ndt.name == "Leaves")
        .and_then(|ndt| ndt.ty.as_ref())
        .expect("Leaves is exported");
    let DataType::Struct(data) = body else {
        panic!("expected a struct, got {body:?}");
    };
    let specta::datatype::Fields::Named(named) = &data.fields else {
        panic!("expected named fields");
    };
    let field = named
        .fields
        .iter()
        .find(|(name, _)| name == "values")
        .and_then(|(_, field)| field.ty.as_ref())
        .expect("values field");
    let DataType::List(list) = inline_datatype(field) else {
        panic!("expected a list, got {field:?}");
    };
    assert_eq!(
        *list.ty,
        DataType::Primitive(Primitive::str),
        "the formatter's rewritten subtree was formatted a second time"
    );
}

/// Unwraps the inline reference a passthrough container such as Vec produces.
fn inline_datatype(ty: &DataType) -> &DataType {
    if let DataType::Reference(specta::datatype::Reference::Named(reference)) = ty
        && let specta::datatype::NamedReferenceType::Inline { dt, .. } = &reference.inner
    {
        return dt;
    }
    ty
}
