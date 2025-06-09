use specta::Type;

use crate::ts::assert_ts_export;

// Some double-slash comment which is ignored
/// Some triple-slash comment
/// Some more triple-slash comment
#[derive(Type)]
#[specta(collect = false)]
pub struct CommentedStruct {
    // Some double-slash comment which is ignored
    /// Some triple-slash comment
    /// Some more triple-slash comment
    a: i32,
}

// Some double-slash comment which is ignored
/// Some triple-slash comment
/// Some more triple-slash comment
#[derive(Type)]
#[specta(collect = false)]
pub enum CommentedEnum {
    // Some double-slash comment which is ignored
    /// Some triple-slash comment
    /// Some more triple-slash comment
    A(i32),
    // Some double-slash comment which is ignored
    /// Some triple-slash comment
    /// Some more triple-slash comment
    B {
        // Some double-slash comment which is ignored
        /// Some triple-slash comment
        /// Some more triple-slash comment
        a: i32,
    },
}

/// Some single-line comment
#[derive(Type)]
#[specta(collect = false)]
pub enum SingleLineComment {
    /// Some single-line comment
    A(i32),
    /// Some single-line comment
    B {
        /// Some single-line comment
        a: i32,
    },
}

#[test]
fn comments() {
    assert_ts_export!(CommentedStruct, "/**\n * Some triple-slash comment\n * Some more triple-slash comment\n */\nexport type CommentedStruct = { \n/**\n * Some triple-slash comment\n * Some more triple-slash comment\n */\na: number };");
    assert_ts_export!(CommentedEnum, "/**\n * Some triple-slash comment\n * Some more triple-slash comment\n */\nexport type CommentedEnum = \n/**\n * Some triple-slash comment\n * Some more triple-slash comment\n */\n{ A: number } | \n/**\n * Some triple-slash comment\n * Some more triple-slash comment\n */\n{ B: { \n/**\n * Some triple-slash comment\n * Some more triple-slash comment\n */\na: number } };");
    assert_ts_export!(SingleLineComment, "/**\n * Some single-line comment\n */\nexport type SingleLineComment = \n/**\n * Some single-line comment\n */\n{ A: number } | \n/**\n * Some single-line comment\n */\n{ B: { \n/**\n * Some single-line comment\n */\na: number } };");
}
