use std::{
    any::Any,
    borrow::Cow,
    fmt,
    hash::{self, Hash},
    sync::Arc,
};

use crate::{
    Type, Types,
    datatype::{DataType, Reference},
};

/// A type-erased literal value stored inside an opaque [`Reference`].
#[derive(Clone)]
pub struct Literal(Arc<dyn LiteralType>);

/// Construct a literal [`DataType`] from a concrete value.
pub fn literal<T: LiteralType>(value: T) -> DataType {
    DataType::Reference(Reference::opaque(Literal::from(value)))
}

/// Trait used by type-erased literal values.
///
/// This sealed, allowing us to add new implementations in minor releases.
trait LiteralType: Any + Send + Sync + 'static {
    /// Returns the underlying datatype represented by this literal value.
    fn definition(&self, types: &mut Types) -> DataType;

    #[doc(hidden)]
    fn eq_dyn(&self, other: &dyn LiteralType) -> bool;

    #[doc(hidden)]
    fn hash_dyn(&self, state: &mut dyn hash::Hasher);

    #[doc(hidden)]
    fn fmt_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    #[doc(hidden)]
    fn as_any(&self) -> &dyn Any;
}

impl Literal {
    /// Returns the underlying datatype represented by this literal value.
    pub fn definition(&self, types: &mut Types) -> DataType {
        self.0.definition(types)
    }

    /// Attempt to downcast the stored literal to a concrete type.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref::<T>()
    }
}

impl<T: LiteralType> From<T> for Literal {
    fn from(value: T) -> Self {
        Self(Arc::new(value))
    }
}

impl fmt::Debug for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt_dyn(f)
    }
}

impl PartialEq for Literal {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_dyn(other.0.as_ref())
    }
}

impl Eq for Literal {}

impl hash::Hash for Literal {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.0.hash_dyn(state)
    }
}

macro_rules! impl_literal_type {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl LiteralType for $ty {
                fn definition(&self, types: &mut Types) -> DataType {
                    <$ty as Type>::definition(types)
                }

                fn eq_dyn(&self, other: &dyn LiteralType) -> bool {
                    other.as_any().downcast_ref::<Self>() == Some(self)
                }

                fn hash_dyn(&self, mut state: &mut dyn hash::Hasher) {
                    Hash::hash(self, &mut state);
                }

                fn fmt_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::Debug::fmt(self, f)
                }

                fn as_any(&self) -> &dyn Any {
                    self
                }
            }
        )+
    };
}

impl_literal_type!(
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    bool,
    char,
    &'static str,
    String,
    Cow<'static, str>,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum F32Key {
    NegInfinity,
    NegZero,
    Finite(u32),
    Infinity,
    NaN,
}

impl From<f32> for F32Key {
    fn from(value: f32) -> Self {
        if value.is_nan() {
            Self::NaN
        } else if value == f32::INFINITY {
            Self::Infinity
        } else if value == f32::NEG_INFINITY {
            Self::NegInfinity
        } else if value == 0.0 {
            if value.to_bits() == (-0.0f32).to_bits() {
                Self::NegZero
            } else {
                Self::Finite(0.0f32.to_bits())
            }
        } else {
            Self::Finite(value.to_bits())
        }
    }
}

impl LiteralType for f32 {
    fn definition(&self, types: &mut Types) -> DataType {
        <f32 as Type>::definition(types)
    }

    fn eq_dyn(&self, other: &dyn LiteralType) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| F32Key::from(*self) == F32Key::from(*other))
    }

    fn hash_dyn(&self, mut state: &mut dyn hash::Hasher) {
        F32Key::from(*self).hash(&mut state);
    }

    fn fmt_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum F64Key {
    NegInfinity,
    NegZero,
    Finite(u64),
    Infinity,
    NaN,
}

impl From<f64> for F64Key {
    fn from(value: f64) -> Self {
        if value.is_nan() {
            Self::NaN
        } else if value == f64::INFINITY {
            Self::Infinity
        } else if value == f64::NEG_INFINITY {
            Self::NegInfinity
        } else if value == 0.0 {
            if value.to_bits() == (-0.0f64).to_bits() {
                Self::NegZero
            } else {
                Self::Finite(0.0f64.to_bits())
            }
        } else {
            Self::Finite(value.to_bits())
        }
    }
}

impl LiteralType for f64 {
    fn definition(&self, types: &mut Types) -> DataType {
        <f64 as Type>::definition(types)
    }

    fn eq_dyn(&self, other: &dyn LiteralType) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| F64Key::from(*self) == F64Key::from(*other))
    }

    fn hash_dyn(&self, mut state: &mut dyn hash::Hasher) {
        F64Key::from(*self).hash(&mut state);
    }

    fn fmt_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
