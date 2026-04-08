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

impl Literal {
    /// Construct a literal [`DataType`] from a concrete value.
    ///
    /// T can be any of [`i8`], [`i16`], [`i32`], [`i64`], [`i128`], [`isize`], [`u8`], [`u16`], [`u32`], [`u64`], [`u128`], [`usize`], [`bool`], [`char`], [`&'static str`], [`String`], [`Cow<'static, str>`].
    pub fn new<T: LiteralType>(value: T) -> DataType {
        DataType::Reference(Reference::opaque(Literal::from(value)))
    }

    /// Returns the underlying datatype represented by this literal value.
    pub fn definition(&self, types: &mut Types) -> DataType {
        self.0.definition(types)
    }

    /// Attempt to downcast the stored literal to a concrete type.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.as_any().downcast_ref::<T>()
    }
}

/// Trait used by type-erased literal values.
///
/// Sealed so we can add implementations in minor releases
pub trait LiteralType: Any + Send + Sync + 'static {
    /// Returns the underlying datatype represented by this literal value.
    fn definition(&self, types: &mut Types) -> DataType;

    fn eq_dyn(&self, other: &dyn LiteralType) -> bool;

    fn hash_dyn(&self, state: &mut dyn hash::Hasher);

    fn fmt_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    fn as_any(&self) -> &dyn Any;
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
enum FloatKey<Bits> {
    NegInfinity,
    NegZero,
    Finite(Bits),
    Infinity,
    NaN,
}

trait FloatLiteral: Copy + PartialEq + Type + fmt::Debug + Send + Sync + 'static {
    type Bits: Copy + Eq + Hash;

    const INFINITY: Self;
    const NEG_INFINITY: Self;
    const ZERO_BITS: Self::Bits;
    const NEG_ZERO_BITS: Self::Bits;

    fn is_nan(self) -> bool;
    fn to_bits(self) -> Self::Bits;
}

impl<T: FloatLiteral> From<T> for FloatKey<T::Bits> {
    fn from(value: T) -> Self {
        if value.is_nan() {
            Self::NaN
        } else if value == T::INFINITY {
            Self::Infinity
        } else if value == T::NEG_INFINITY {
            Self::NegInfinity
        } else if value.to_bits() == T::NEG_ZERO_BITS {
            Self::NegZero
        } else if value.to_bits() == T::ZERO_BITS {
            Self::Finite(T::ZERO_BITS)
        } else {
            Self::Finite(value.to_bits())
        }
    }
}

macro_rules! impl_float_literal {
    ($ty:ty, $bits:ty, $zero:expr, $neg_zero:expr) => {
        impl FloatLiteral for $ty {
            type Bits = $bits;

            const INFINITY: Self = <$ty>::INFINITY;
            const NEG_INFINITY: Self = <$ty>::NEG_INFINITY;
            const ZERO_BITS: Self::Bits = $zero;
            const NEG_ZERO_BITS: Self::Bits = $neg_zero;

            fn is_nan(self) -> bool {
                self.is_nan()
            }

            fn to_bits(self) -> Self::Bits {
                self.to_bits()
            }
        }

        impl LiteralType for $ty {
            fn definition(&self, types: &mut Types) -> DataType {
                <$ty as Type>::definition(types)
            }

            fn eq_dyn(&self, other: &dyn LiteralType) -> bool {
                other
                    .as_any()
                    .downcast_ref::<Self>()
                    .is_some_and(|other| FloatKey::from(*self) == FloatKey::from(*other))
            }

            fn hash_dyn(&self, mut state: &mut dyn hash::Hasher) {
                FloatKey::from(*self).hash(&mut state);
            }

            fn fmt_dyn(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(self, f)
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

impl_float_literal!(f32, u32, 0.0f32.to_bits(), (-0.0f32).to_bits());
impl_float_literal!(f64, u64, 0.0f64.to_bits(), (-0.0f64).to_bits());

#[cfg(is_nightly)]
impl_float_literal!(f16, u16, 0.0f16.to_bits(), (-0.0f16).to_bits());

#[cfg(is_nightly)]
impl_float_literal!(f128, u128, 0.0f128.to_bits(), (-0.0f128).to_bits());
