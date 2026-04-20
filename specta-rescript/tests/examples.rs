use specta_rescript::ReScript;

mod simple_usage {
    use super::ReScript;
    include!("../examples/simple_usage.rs");

    #[test]
    fn default_export() {
        let golden = include_str!("../examples/generated/SimpleUsage.res");
        assert_eq!(ReScript::default().export(&types()).unwrap(), golden);
    }
}

mod basic_types {
    use super::ReScript;
    include!("../examples/basic_types.rs");

    #[test]
    fn export() {
        let golden = include_str!("../examples/generated/BasicTypes.res");
        assert_eq!(
            ReScript::default()
                .without_serde()
                .export(&types())
                .unwrap(),
            golden
        );
    }
}

mod variants {
    use super::ReScript;
    include!("../examples/variants.rs");

    #[test]
    fn export() {
        let golden = include_str!("../examples/generated/Variants.res");
        assert_eq!(
            ReScript::default()
                .without_serde()
                .export(&types())
                .unwrap(),
            golden
        );
    }
}

mod generics {
    use super::ReScript;
    include!("../examples/generics.rs");

    #[test]
    fn export() {
        let golden = include_str!("../examples/generated/Generics.res");
        assert_eq!(
            ReScript::default()
                .without_serde()
                .export(&types())
                .unwrap(),
            golden
        );
    }
}

mod result_types {
    use super::ReScript;
    include!("../examples/result_types.rs");

    #[test]
    fn export() {
        let golden = include_str!("../examples/generated/ResultTypes.res");
        assert_eq!(
            ReScript::default()
                .without_serde()
                .export(&types())
                .unwrap(),
            golden
        );
    }
}

mod comments_example {
    use super::ReScript;
    include!("../examples/comments_example.rs");

    #[test]
    fn export() {
        let golden = include_str!("../examples/generated/CommentsExample.res");
        assert_eq!(ReScript::default().export(&types()).unwrap(), golden);
    }
}

mod comprehensive_demo {
    use super::ReScript;
    include!("../examples/comprehensive_demo.rs");

    #[test]
    fn export() {
        let golden = include_str!("../examples/generated/ComprehensiveDemo.res");
        assert_eq!(ReScript::default().export(&types()).unwrap(), golden);
    }
}
