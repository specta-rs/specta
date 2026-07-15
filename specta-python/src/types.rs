use std::fmt::Debug;

use specta::{
    Type, Types,
    datatype::{DataType, Reference},
};

use crate::opaque;

macro_rules! python_type {
    ($name:ident, $opaque:ident, $doc:literal) => {
        #[doc = $doc]
        pub struct $name<T = ()>(T);

        impl<T> Type for $name<T> {
            fn definition(_: &mut Types) -> DataType {
                DataType::Reference(Reference::opaque(opaque::$opaque))
            }
        }

        impl<T: Debug> Debug for $name<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }

        impl<T: Clone> Clone for $name<T> {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }

        impl<T: Default> Default for $name<T> {
            fn default() -> Self {
                Self(T::default())
            }
        }

        #[cfg(feature = "serde")]
        #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
        impl<T: serde::Serialize> serde::Serialize for $name<T> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                T::serialize(&self.0, serializer)
            }
        }

        #[cfg(feature = "serde")]
        #[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
        impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for $name<T> {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                T::deserialize(deserializer).map(Self)
            }
        }
    };
}

python_type!(Any, Any, "Overrides a Rust type with Python `typing.Any`.");
python_type!(
    Unknown,
    Unknown,
    "Overrides a Rust type with Python `typing.Any`, representing an unknown value."
);
python_type!(
    Never,
    Never,
    "Overrides a Rust type with Python `typing.Never`."
);
