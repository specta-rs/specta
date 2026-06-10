use serde::{Deserialize, Serialize};
use specta::{Type, Types};

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(bound(serialize = "T: serde::Serialize"))]
struct SerdeBoundNested<T>(T);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(bound = "T: serde::Serialize + serde::de::DeserializeOwned")]
struct SerdeBoundFlat<T>(T);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(bound(
    serialize = "T: serde::Serialize",
    deserialize = "T: serde::de::DeserializeOwned",
))]
struct SerdeBoundBoth<T>(T);

#[test]
fn serde_bound_nested() {
    let mut types = Types::default();
    let _ = SerdeBoundNested::<i32>::definition(&mut types);
}

#[test]
fn serde_bound_flat() {
    let mut types = Types::default();
    let _ = SerdeBoundFlat::<String>::definition(&mut types);
}

#[test]
fn serde_bound_serialize_and_deserialize() {
    let mut types = Types::default();
    let _ = SerdeBoundBoth::<String>::definition(&mut types);
}
