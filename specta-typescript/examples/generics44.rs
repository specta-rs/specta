use serde::Serialize;
use specta::{Type, Types};
use specta_typescript::Typescript;

mod arrays {
    use std::{convert::TryInto, marker::PhantomData};

    use serde::{
        Deserialize, Deserializer, Serialize, Serializer,
        de::{SeqAccess, Visitor},
        ser::SerializeTuple,
    };
    pub fn serialize<S: Serializer, T: Serialize, const N: usize>(
        data: &[T; N],
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        let mut s = ser.serialize_tuple(N)?;
        for item in data {
            s.serialize_element(item)?;
        }
        s.end()
    }

    struct ArrayVisitor<T, const N: usize>(PhantomData<T>);

    impl<'de, T, const N: usize> Visitor<'de> for ArrayVisitor<T, N>
    where
        T: Deserialize<'de>,
    {
        type Value = [T; N];

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str(&format!("an array of length {}", N))
        }

        #[inline]
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            // can be optimized using MaybeUninit
            let mut data = Vec::with_capacity(N);
            for _ in 0..N {
                match (seq.next_element())? {
                    Some(val) => data.push(val),
                    None => return Err(serde::de::Error::invalid_length(N, &self)),
                }
            }
            match data.try_into() {
                Ok(arr) => Ok(arr),
                Err(_) => unreachable!(),
            }
        }
    }
    pub fn deserialize<'de, D, T, const N: usize>(deserializer: D) -> Result<[T; N], D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        deserializer.deserialize_tuple(N, ArrayVisitor::<T, N>(PhantomData))
    }
}

#[derive(Type, Serialize)]
// #[specta(inline)] // TODO
struct Demo<const N: usize = 1> {
    #[serde(with = "arrays")]
    #[specta(type = [u32; N])]
    data: [u32; N], // becomes `number[]`
    a: [u8; 2], // becomes `number[]`
    #[specta(type = specta_util::FixedArray<3, u8>)]
    d: [u8; 3], // becomes `[number number]`
    e: Box<Demo<4>>,
}

#[derive(Type, Serialize)]
struct ContainsDemo {
    // #[serde(flatten)]
    // a: Demo,
    #[specta(inline)]
    b: Demo<2>,
    #[specta(inline)]
    c: Demo<3>,
    d: [u8; 2], // becomes `[number, number]`
}

fn main() {
    println!(
        "{}",
        Typescript::default()
            .export(&specta_serde::apply(Types::default().register::<ContainsDemo>()).unwrap(),)
            .unwrap()
    );
}
