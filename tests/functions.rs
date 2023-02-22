#[cfg(feature = "functions")]
mod test {
    use std::fmt;

    use specta::{functions, specta};

    #[specta]
    fn a() {}

    #[specta]
    fn b(demo: String) {}

    #[specta]
    fn c(a: String, b: i32, c: i64) {}

    #[specta]
    fn d(demo: String) -> i32 {
        42
    }

    #[specta]
    fn e<T: fmt::Debug>(window: T) {}

    #[test]
    fn test_function_export() {
        let (functions, types) = functions::collect_types![a, b, c, d, e::<i32>];

        // TODO: Asserts `functions` and `types` is correct
    }
}
