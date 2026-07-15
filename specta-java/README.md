# specta-java

Export [Specta](https://github.com/specta-rs/specta) type collections as Java 17 records, enums, and sealed interfaces.

```rust
use specta::{Type, Types};
use specta_java::Java;

#[derive(Type)]
struct User {
    name: String,
}

Java::default()
    .export_to(
        "Bindings.java",
        &Types::default().register::<User>(),
        specta_serde::Format,
    )?;
# Ok::<(), specta_java::Error>(())
```

The default flat-file layout nests generated types in a `Bindings` class. Use
`Layout::Files` with `export_to` to generate one public Java source file per type.
Flat-file output must be written to a file matching the configured class name
(for example, `Bindings.java`).

The crate generates Java data models, not serialization adapters. Legal formatted
field names are preserved, string enums expose their wire value through `value()`,
and names Java cannot represent are escaped deterministically. Applications should
configure their serializer for those escaped names and enum values. Rust tuples
used as fields become nested records; anonymous tuple positions that cannot be
named safely fall back to Java lists.
