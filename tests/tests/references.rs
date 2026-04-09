use std::any::TypeId;

use specta::{
    Type, Types,
    datatype::{DataType, Reference},
};

#[derive(Type)]
#[specta(collect = false)]
struct GenericType<T>(T);

#[derive(Type)]
#[specta(collect = false)]
struct AnotherOne;

#[derive(Type)]
struct RecursivePoint {
    x: f64,
    y: f64,
}

#[derive(Type)]
enum RecursiveShape {
    Point(RecursivePoint),
    Nested { shapes: Vec<RecursiveShape> },
}

#[derive(Type)]
struct MutualA {
    b: Box<MutualB>,
}

#[derive(Type)]
struct MutualB {
    a: MutualA,
}

#[test]
fn references() {
    // Opaque references are compared by value
    assert_eq!(Reference::opaque(()), Reference::opaque(()));
    assert_eq!(Reference::opaque(true), Reference::opaque(true));
    assert_ne!(Reference::opaque(true), Reference::opaque(false));
    assert_ne!(Reference::opaque(42u32), Reference::opaque('a'));

    // Ensure opaque metadata can be extracted again
    {
        let r = match Reference::opaque(()) {
            Reference::Opaque(r) => r,
            _ => panic!("Expected an opaque reference"),
        };

        assert_eq!(r.type_id(), TypeId::of::<()>());
        assert_eq!(r.type_name(), "()");
        assert_eq!(r.downcast_ref(), Some(&()));
    }

    let mut types = Types::default();

    // Named references `PartialEq` are compared by type, generics, inline,
    // however `Reference::ty_eq` compares by just type.
    {
        let a = match GenericType::<()>::definition(&mut types) {
            DataType::Reference(r) => r,
            _ => panic!("Expected a reference type"),
        };
        let b = match GenericType::<()>::definition(&mut types) {
            DataType::Reference(r) => r,
            _ => panic!("Expected a reference type"),
        };
        assert_eq!(a, b);
        assert!(a.ty_eq(&b));
        assert!(b.ty_eq(&a));
    }

    {
        let a = match GenericType::<()>::definition(&mut types) {
            DataType::Reference(r) => r,
            _ => panic!("Expected a reference type"),
        };
        let b = match GenericType::<String>::definition(&mut types) {
            DataType::Reference(r) => r,
            _ => panic!("Expected a reference type"),
        };
        assert_ne!(a, b);
        assert!(a.ty_eq(&b));
        assert!(b.ty_eq(&a));
    }

    {
        let a = match GenericType::<()>::definition(&mut types) {
            DataType::Reference(r) => r,
            _ => panic!("Expected a reference type"),
        };
        let b = match AnotherOne::definition(&mut types) {
            DataType::Reference(r) => r,
            _ => panic!("Expected a reference type"),
        };
        assert_ne!(a, b);
        assert!(!a.ty_eq(&b));
        assert!(!b.ty_eq(&a));
    }
}

#[test]
fn recursive_named_types_restore_outer_sentinel() {
    let types = Types::default().register::<RecursiveShape>();
    let type_names = types
        .into_sorted_iter()
        .map(|ty| ty.name().as_ref().to_string())
        .collect::<Vec<_>>();

    assert!(type_names.iter().any(|name| name == "RecursivePoint"));
    assert!(type_names.iter().any(|name| name == "RecursiveShape"));
    assert_eq!(
        type_names
            .iter()
            .filter(|name| name.as_str() == "RecursiveShape")
            .count(),
        1
    );
}

#[test]
fn mutually_recursive_named_types_reuse_placeholders() {
    let types = Types::default().register::<MutualA>();
    let type_names = types
        .into_sorted_iter()
        .map(|ty| ty.name().as_ref().to_string())
        .collect::<Vec<_>>();

    assert!(type_names.iter().any(|name| name == "MutualA"));
    assert!(type_names.iter().any(|name| name == "MutualB"));
    assert_eq!(
        type_names
            .iter()
            .filter(|name| name.as_str() == "MutualA")
            .count(),
        1
    );
    assert_eq!(
        type_names
            .iter()
            .filter(|name| name.as_str() == "MutualB")
            .count(),
        1
    );
}
