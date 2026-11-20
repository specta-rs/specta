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

fn tauri_channel_reference(types: &mut Types, opaque: bool) -> Reference {
    use specta::datatype::NamedDataType;

    fn build(types: &mut Types, ndt: &mut NamedDataType, opaque: bool) {
        ndt.name = "TAURI_CHANNEL".into();
        ndt.module_path = "tauri::ipc::channel".into();
        ndt.ty = Some(if opaque {
            Reference::opaque("channel metadata").into()
        } else {
            String::definition(types)
        });
    }

    NamedDataType::init_with_sentinel(
        specta::datatype::NamedKey::label(if opaque {
            "opaque_channel"
        } else {
            "legacy_channel"
        }),
        &[],
        &[],
        false,
        false,
        0,
        types,
        if opaque {
            |types, ndt| build(types, ndt, true)
        } else {
            |types, ndt| build(types, ndt, false)
        },
        |_| Reference::opaque("channel metadata").into(),
    )
}

#[test]
fn tauri_channel_preserves_opaque_definition() {
    let mut types = Types::default();
    for _ in 0..2 {
        let Reference::Named(reference) = tauri_channel_reference(&mut types, true) else {
            panic!("expected a named reference");
        };
        assert_eq!(
            types.get(&reference).unwrap().ty,
            Some(Reference::opaque("channel metadata").into())
        );
    }
}

#[test]
fn tauri_channel_keeps_legacy_patch() {
    let mut types = Types::default();
    for _ in 0..2 {
        let Reference::Named(reference) = tauri_channel_reference(&mut types, false) else {
            panic!("expected a named reference");
        };
        assert!(types.get(&reference).unwrap().ty.is_none());
    }
}

#[test]
fn tauri_channel_inlines_opaque_definition() {
    let mut types = Types::default();
    for _ in 0..2 {
        let reference =
            specta::datatype::inline(&mut types, |types| tauri_channel_reference(types, true));
        let Reference::Named(reference) = reference else {
            panic!("expected an inline named reference");
        };
        let specta::datatype::NamedReferenceType::Inline { dt, .. } = reference.inner else {
            panic!("expected an inline opaque definition");
        };
        assert_eq!(*dt, Reference::opaque("channel metadata").into());
    }
}
