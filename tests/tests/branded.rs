// // Test for the branded! macro from specta-typescript
// // These tests verify that the macro generates valid Rust code that compiles

// use specta_typescript::branded;

// // Basic usage without custom name
// branded!(pub struct AccountId(String));

// // With custom TypeScript name
// branded!(pub struct UserId(u64) as "userId");

// // With attributes
// branded!(
//     #[derive(serde::Serialize)]
//     pub struct Token(String)
// );

// // With generic types
// branded!(pub struct Id<T>(T));

// // With multiple generic types
// branded!(pub struct Pair<A, B>((A, B)) as "pair");

// // With visibility modifiers
// branded!(pub(crate) struct InternalId(u64));
// branded!(struct PrivateId(String));

// // Without doc comment since macros can't use them
// branded!(pub struct OrderId(u64) as "orderId");

// #[test]
// fn test_branded_macro_compiles() {
//     // Just verify the macro generates valid code
//     let account_id = AccountId(String::from("acc_123"));
//     assert_eq!(account_id.0, "acc_123");

//     let user_id = UserId(42);
//     assert_eq!(user_id.0, 42);

//     let id = Id(String::from("test"));
//     assert_eq!(id.0, "test");

//     let pair = Pair((1, "test"));
//     assert_eq!(pair.0, (1, "test"));

//     let internal_id = InternalId(100);
//     assert_eq!(internal_id.0, 100);

//     let private_id = PrivateId(String::from("private"));
//     assert_eq!(private_id.0, "private");

//     let order_id = OrderId(999);
//     assert_eq!(order_id.0, 999);
// }

// // Verify that Type trait is implemented (will panic with todo! if called)
// #[test]
// #[should_panic(expected = "branded type implementation")]
// fn test_type_trait_is_implemented() {
//     use specta::{Type, TypeCollection};

//     let mut types = TypeCollection::default();
//     let _ = AccountId::definition(&mut types);
// }

// #[test]
// #[should_panic(expected = "branded type implementation")]
// fn test_type_trait_with_generics() {
//     use specta::{Type, TypeCollection};

//     let mut types = TypeCollection::default();
//     let _ = Id::<String>::definition(&mut types);
// }
