#[derive(Clone)]
#[allow(dead_code)]
struct Pet {
    serde: String,
    specta: u32,
}

struct User {
    serde: String,
    specta: u32,
    pets: Vec<Pet>,
}

fn assert_selection_traits<T>(_: &T)
where
    T: specta_util::__private::serde::Serialize + specta_util::__private::specta::Type,
{
}

fn main() {
    let selection = specta_util::selection!(
        User {
            serde: "Ada".to_owned(),
            specta: 37,
            pets: vec![Pet {
                serde: "Tux".to_owned(),
                specta: 2,
            }],
        },
        { serde, specta } as __private
    );
    assert_selection_traits(&selection);
    assert_eq!(selection.serde, "Ada");
    assert_eq!(selection.specta, 37);

    // Values and nested selections resolve through `specta_util::__private` too.
    let user = User {
        serde: "Grace".to_owned(),
        specta: 85,
        pets: vec![Pet {
            serde: "Pebble".to_owned(),
            specta: 4,
        }],
    };
    let selections = specta_util::selection!(
        [user],
        [{
            __typename: "User",
            serde,
            pets: [{ serde }],
        }] as UserListSelection
    );
    assert_selection_traits(&selections[0]);
    assert_eq!(selections[0].__typename, "User");
    assert_eq!(selections[0].serde, "Grace");
    assert_eq!(selections[0].pets[0].serde, "Pebble");
}
