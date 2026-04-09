use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use specta::{
    Type, Types,
    datatype::{DataType, Literal, Reference},
};

fn hash_value<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn literals() {
    assert_eq!(Literal::from(42u32), Literal::from(42u32));
    assert_ne!(Literal::from(42u32), Literal::from(7u32));
    assert_ne!(Literal::from(42u32), Literal::from('a'));
    assert_eq!(
        hash_value(&Literal::from(42u32)),
        hash_value(&Literal::from(42u32))
    );

    assert_eq!(Literal::from(f32::NAN), Literal::from(f32::NAN));
    assert_ne!(Literal::from(0.0f32), Literal::from(-0.0f32));
    assert_eq!(Literal::from(f64::INFINITY), Literal::from(f64::INFINITY));
    assert_eq!(
        Literal::from(f64::NEG_INFINITY),
        Literal::from(f64::NEG_INFINITY)
    );
    assert_eq!(
        hash_value(&Literal::from(f64::NAN)),
        hash_value(&Literal::from(f64::NAN))
    );

    let literal = match Literal::new("hello") {
        DataType::Reference(Reference::Opaque(reference)) => reference,
        _ => panic!("expected opaque literal reference"),
    };

    let literal = literal
        .downcast_ref::<Literal>()
        .expect("expected stored literal");
    assert_eq!(literal.downcast_ref::<&'static str>(), Some(&"hello"));

    let mut types = Types::default();
    let expected_definition = <&'static str as Type>::definition(&mut types);
    assert_eq!(literal.definition(&mut types), expected_definition);

    assert_eq!(
        Reference::opaque(Literal::from(42u32)),
        Reference::opaque(Literal::from(42u32))
    );
    assert_ne!(
        Reference::opaque(Literal::from(0.0f64)),
        Reference::opaque(Literal::from(-0.0f64))
    );
    assert_eq!(
        Reference::opaque(Literal::from(f64::NAN)),
        Reference::opaque(Literal::from(f64::NAN))
    );

    #[cfg(is_nightly)]
    {
        assert_eq!(Literal::from(f16::NAN), Literal::from(f16::NAN));
        assert_ne!(Literal::from(0.0f16), Literal::from(-0.0f16));
        assert_eq!(Literal::from(f128::NAN), Literal::from(f128::NAN));
        assert_ne!(Literal::from(0.0f128), Literal::from(-0.0f128));
    }
}
