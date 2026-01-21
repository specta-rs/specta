use std::any::TypeId;

use specta::{
    Type, TypeCollection,
    datatype::{DataType, Reference},
};

#[derive(Type)]
struct GenericType<T>(T);

#[derive(Type)]
struct AnotherOne;

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

    let mut types = TypeCollection::default();

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
