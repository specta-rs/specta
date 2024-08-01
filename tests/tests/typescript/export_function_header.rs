// #![allow(deprecated)]

// use specta::{datatype::Function, function::fn_datatype, specta, TypeMap};

// #[specta]
// fn a() {}
// #[specta]
// fn b() -> () {}
// #[specta]
// async fn c() {}
// #[specta]
// fn d() -> String {
//     "todo".into()
// }
// #[specta]
// fn e(a: String) {}
// #[specta]
// fn f(a: String, b: i32) {}
// #[specta]
// #[deprecated]
// fn g() {}

// #[test]
// fn test_export_function_header() {
//     assert(
//         fn_datatype!(a)(&mut TypeMap::default()),
//         Ok("export function a();"),
//     );
//     assert(
//         fn_datatype!(b)(&mut TypeMap::default()),
//         Ok("export function b(): null;"),
//     );
//     assert(
//         fn_datatype!(c)(&mut TypeMap::default()),
//         Ok("export async function c();"),
//     );
//     assert(
//         fn_datatype!(d)(&mut TypeMap::default()),
//         Ok("export function d(): string;"),
//     );
//     assert(
//         fn_datatype!(e)(&mut TypeMap::default()),
//         Ok("export function e(a: string);"),
//     );
//     assert(
//         fn_datatype!(f)(&mut TypeMap::default()),
//         Ok("export function f(a: string, b: number);"),
//     );
//     assert(
//         fn_datatype!(g)(&mut TypeMap::default()),
//         Ok("/**\n * @deprecated\n */\nexport function g();"),
//     );
// }

// #[track_caller]
// fn assert(dt: Function, result: specta_typescript::Result<&str>) {
//     match export_function_header(dt, &Default::default()) {
//         Ok(s) => assert_eq!(result, Ok(s.as_str())),
//         Err(e) => assert_eq!(result, Err(e)),
//     }
// }
