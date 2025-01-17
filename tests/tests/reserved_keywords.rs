// use specta::Type;
// use specta_typescript::{ExportError, ExportPath, NamedLocation, Typescript};

// mod astruct {
//     use super::*;

//     // Typescript reserved type name
//     #[derive(Type)]
//     #[specta(export = false)]
//     #[allow(non_camel_case_types)]
//     pub struct r#enum {
//         a: String,
//     }
// }

// mod atuplestruct {
//     use super::*;

//     // Typescript reserved type name
//     #[derive(Type)]
//     #[specta(export = false)]
//     #[allow(non_camel_case_types)]
//     pub struct r#enum(String);
// }

// mod aenum {
//     use super::*;

//     // Typescript reserved type name
//     #[derive(Type)]
//     #[specta(export = false)]
//     #[allow(non_camel_case_types)]
//     pub enum r#enum {
//         A(String),
//     }
// }

// #[test]
// fn test_ts_reserved_keyworks() {
//     assert_eq!(
//         specta_typescript::export::<astruct::r#enum>(&Typescript::default()),
//         Err(ExportError::ForbiddenName(
//             NamedLocation::Type,
//             #[cfg(not(windows))]
//             ExportPath::new_unsafe("tests/tests/reserved_keywords.rs:8:14"),
//             #[cfg(windows)]
//             ExportPath::new_unsafe("tests\tests\reserved_keywords.rs:8:14"),
//             "enum"
//         ))
//     );
//     assert_eq!(
//         specta_typescript::export::<atuplestruct::r#enum>(&Typescript::default()),
//         Err(ExportError::ForbiddenName(
//             NamedLocation::Type,
//             #[cfg(not(windows))]
//             ExportPath::new_unsafe("tests/tests/reserved_keywords.rs:20:14"),
//             #[cfg(windows)]
//             ExportPath::new_unsafe("tests\tests\reserved_keywords.rs:20:14"),
//             "enum"
//         ))
//     );
//     assert_eq!(
//         specta_typescript::export::<aenum::r#enum>(&Typescript::default()),
//         Err(ExportError::ForbiddenName(
//             NamedLocation::Type,
//             #[cfg(not(windows))]
//             ExportPath::new_unsafe("tests/tests/reserved_keywords.rs:30:14"),
//             #[cfg(windows)]
//             ExportPath::new_unsafe("tests\tests\reserved_keywords.rs:32:14"),
//             "enum"
//         ))
//     );
// }
