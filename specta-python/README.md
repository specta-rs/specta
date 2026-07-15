# specta-python

Export [Specta](https://github.com/specta-rs/specta) type collections as Python
3.13 type hints.

```rust
use specta::{Type, Types};
use specta_python::Python;

#[derive(Type)]
struct User {
    id: u64,
    name: String,
}

let types = Types::default().register::<User>();
Python::default()
    .export_to("bindings.py", &types, specta_serde::Format)
    .unwrap();
```

Generated records use `typing.TypedDict`, so their annotations describe the
serialized dictionary shape, including renamed and optional keys.

The generated syntax requires Python 3.13 so generic defaults can be preserved.
In the multi-file layout, cross-module imports are exposed through `TYPE_CHECKING`
and repeated after declarations at runtime. Deferring runtime imports keeps mutually
recursive modules importable while allowing lazy aliases, generic defaults, and
`typing.get_type_hints` to resolve after package initialization.

Python has no general intersection-type operator. Specta merges intersections
made entirely from record shapes. For an internally tagged map it emits the map
type (the narrowest representable Python supertype); other unrepresentable mixed
intersections return an exporter error.
