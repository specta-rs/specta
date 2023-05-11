#[cfg(feature = "functions")]
mod test {
    use std::fmt;

    use specta::{functions, specta};

    /// Multiline
    /// Docs
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

    // https://github.com/oscartbeaumont/tauri-specta/issues/24
    #[specta]
    #[allow(unused_mut)]
    fn f(mut demo: String) -> i32 {
        42
    }

    #[specta]
    #[allow(unused_mut)]
    fn g(x: std::string::String) {}

    macro_rules! special_string {
        () => {
            String
        };
    }

    #[specta]
    #[allow(unused_mut)]
    fn h(demo: special_string!()) {}

    // TODO: Finish fixing these

    // #[derive(Type)]
    // pub struct Demo {
    //     pub demo: String,
    // }

    // #[specta]
    // #[allow(unused_mut)]
    // fn i(Demo { demo }: Demo) {}

    // macro_rules! special_destructure {
    //     () => {
    //         Demo { demo }
    //     };
    // }

    // #[specta]
    // #[allow(unused_mut)]
    // fn j(special_destructure!(): Demo) {}

    #[test]
    fn test_trailing_comma() {
        functions::collect_types![a, b, c].unwrap();
        functions::collect_types![a, b, c,].unwrap();
    }

    #[test]
    fn test_function_export() {
        let (functions, types) = functions::collect_types![a, b, c, d, e::<i32>, f, g, h].unwrap();

        assert_eq!(functions[0].docs, vec![" Multiline", " Docs"]);

        // TODO: Asserts `functions` and `types` is correct
    }
}
