use specta_util::selection;

use crate::ts::inline_ref;

#[derive(Clone)]
#[allow(dead_code)]
struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub age: i32,
    pub password: String,
}

#[test]
fn test_selection_macros() {
    let user = User {
        id: 1,
        name: "Monty Beaumont".into(),
        email: "monty@otbeaumont.me".into(),
        age: 7,
        password: "password123".into(),
    };

    // Trailing comma
    selection!(user.clone(), { name, age, });
    selection!(vec![user.clone()], [{ name, age, }]);

    let s1 = selection!(user.clone(), { name, age });
    insta::assert_snapshot!(s1.name, @"Monty Beaumont");
    insta::assert_snapshot!(s1.age, @"7");
    insta::assert_snapshot!(inline_ref(&s1, &Default::default()).unwrap(), @"{ name: string; age: number }");

    let users = vec![user; 3];
    let s2 = selection!(users, [{ name, age }]);
    insta::assert_snapshot!(s2[0].name, @"Monty Beaumont");
    insta::assert_snapshot!(s2[0].age, @"7");
    insta::assert_snapshot!(inline_ref(&s2, &Default::default()).unwrap(), @"{ name: string; age: number }[]");
}
