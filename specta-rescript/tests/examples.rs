use specta_rescript::ReScript;

fn assert_export_eq(actual: String, golden: &str) {
    assert_eq!(actual.replace("\r\n", "\n"), golden.replace("\r\n", "\n"));
}

mod simple_usage {
    use super::{ReScript, assert_export_eq};
    include!("../examples/simple_usage.rs");

    #[test]
    fn default_export() {
        let golden = include_str!("../examples/generated/SimpleUsage.res");
        assert_export_eq(ReScript::default().export(&types()).unwrap(), golden);
    }
}

mod basic_types {
    use super::{ReScript, assert_export_eq};
    include!("../examples/basic_types.rs");

    #[test]
    fn export() {
        let golden = include_str!("../examples/generated/BasicTypes.res");
        assert_export_eq(
            ReScript::default()
                .without_serde()
                .export(&types())
                .unwrap(),
            golden,
        );
    }
}

mod variants {
    use super::{ReScript, assert_export_eq};
    include!("../examples/variants.rs");

    #[test]
    fn export() {
        let golden = include_str!("../examples/generated/Variants.res");
        assert_export_eq(
            ReScript::default()
                .without_serde()
                .export(&types())
                .unwrap(),
            golden,
        );
    }
}

mod generics {
    use super::{ReScript, assert_export_eq};
    include!("../examples/generics.rs");

    #[test]
    fn export() {
        let golden = include_str!("../examples/generated/Generics.res");
        assert_export_eq(
            ReScript::default()
                .without_serde()
                .export(&types())
                .unwrap(),
            golden,
        );
    }
}

mod result_types {
    use super::{ReScript, assert_export_eq};
    include!("../examples/result_types.rs");

    #[test]
    fn export() {
        let golden = include_str!("../examples/generated/ResultTypes.res");
        assert_export_eq(
            ReScript::default()
                .without_serde()
                .export(&types())
                .unwrap(),
            golden,
        );
    }
}

mod comments_example {
    use super::{ReScript, assert_export_eq};
    include!("../examples/comments_example.rs");

    #[test]
    fn export() {
        let golden = include_str!("../examples/generated/CommentsExample.res");
        assert_export_eq(ReScript::default().export(&types()).unwrap(), golden);
    }
}

mod comprehensive_demo {
    use super::{ReScript, assert_export_eq};
    include!("../examples/comprehensive_demo.rs");

    #[test]
    fn export() {
        let golden = include_str!("../examples/generated/ComprehensiveDemo.res");
        assert_export_eq(ReScript::default().export(&types()).unwrap(), golden);
    }
}

mod recursive {
    use super::assert_export_eq;

    include!("../examples/recursive.rs");

    #[test]
    fn export() {
        let golden = include_str!("../examples/generated/Recursive.res");
        assert_export_eq(
            ReScript::default()
                .without_serde()
                .export(&types())
                .unwrap(),
            golden,
        );
    }
}
