use crate::ExportError;

// A smaller helper until this is stablised into the Rust standard library.
pub fn intersperse<T: Clone>(iter: impl Iterator<Item = T>, sep: T) -> impl Iterator<Item = T> {
    iter.enumerate().flat_map(move |(i, item)| {
        if i == 0 {
            vec![item]
        } else {
            vec![sep.clone(), item]
        }
    })
}
