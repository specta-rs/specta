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
