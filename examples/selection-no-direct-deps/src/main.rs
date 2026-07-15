struct User {
    serde: String,
    specta: u32,
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
        },
        { serde, specta } as __private
    );
    assert_selection_traits(&selection);
    assert_eq!(selection.serde, "Ada");
    assert_eq!(selection.specta, 37);

    let selections = specta_util::selection!(
        [User {
            serde: "Grace".to_owned(),
            specta: 85,
        }],
        [{ serde, specta }] as __private
    );
    assert_selection_traits(&selections[0]);
    assert_eq!(selections[0].serde, "Grace");
    assert_eq!(selections[0].specta, 85);
}
