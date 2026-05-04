use specta::{
    Types,
    datatype::{DataType, Fields, NamedReferenceType, Reference},
};

/// Recursively replaces [`DataType`]s within a [`DataType`] structure from a set of remap rules.
///
/// `Remapper` is useful when a type should be represented differently for export
/// without changing the original Rust type or derive output. It performs [`DataType`]
/// equality checks while walking the [`DataType`] structure applying the transformations.
///
/// Rules are applied in the order they are registered. For each visited
/// [`DataType`], every matching rule is applied, with each rule seeing the
/// result of the previous matching rule.
///
/// # Examples
///
/// Remap `u32` to `str` and `i32` to `bool`:
///
/// ```rust
/// use specta::{Types, datatype::{DataType, Field, List, NamedDataType, Primitive, Struct}};
/// use specta_util::Remapper;
///
/// let remapper = Remapper::new()
///     .rule(Primitive::u32.into(), Primitive::str.into())
///     .rule(Primitive::i32.into(), Primitive::bool.into());
///
/// // For a single `DataType`
/// assert_eq!(
///     remapper.remap_dt(DataType::List(List::new(Primitive::u32.into()))),
///     DataType::List(List::new(Primitive::str.into()))
/// );
///
/// // For a whole collection of types
/// # #[allow(unused)]
/// let types: Types = remapper.remap_types(Types::default());
/// ```
#[derive(Debug, Clone, Default)]
pub struct Remapper {
    rules: Vec<(DataType, DataType)>,
}

impl Remapper {
    /// Creates a remapper with no rules.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a rule that replaces exact matches of `from` with `to`.
    ///
    /// Rules are checked in the order they are registered.
    pub fn rule(mut self, from: DataType, to: DataType) -> Self {
        self.rules.push((from, to));
        self
    }

    /// Applies the remap operation to a datatype, returning the remapped datatype.
    pub fn remap_dt(&self, mut dt: DataType) -> DataType {
        self.remap_internal(&mut dt);
        dt
    }

    /// Applies the remap operation to every datatype in a [`Types`] collection, returning the remapped collection.
    pub fn remap_types(&self, types: Types) -> Types {
        types.map(|mut ndt| {
            ndt.generics.to_mut().iter_mut().for_each(|generic| {
                if let Some(dt) = &mut generic.default {
                    self.remap_internal(dt);
                }
            });
            if let Some(dt) = &mut ndt.ty {
                self.remap_internal(dt);
            }
            ndt
        })
    }

    fn remap_internal(&self, dt: &mut DataType) {
        self.remap_rules(dt);

        match dt {
            DataType::Primitive(_) | DataType::Generic(_) => {}
            DataType::List(list) => self.remap_internal(&mut list.ty),
            DataType::Map(map) => {
                self.remap_internal(map.key_ty_mut());
                self.remap_internal(map.value_ty_mut());
            }
            DataType::Struct(s) => self.remap_fields(&mut s.fields),
            DataType::Enum(e) => {
                for (_, variant) in &mut e.variants {
                    self.remap_fields(&mut variant.fields);
                }
            }
            DataType::Tuple(tuple) => {
                for dt in &mut tuple.elements {
                    self.remap_internal(dt);
                }
            }
            DataType::Nullable(dt) => self.remap_internal(dt),
            DataType::Intersection(dts) => {
                for dt in dts {
                    self.remap_internal(dt);
                }
            }
            DataType::Reference(r) => self.remap_reference(r),
        }
    }

    fn remap_rules(&self, dt: &mut DataType) {
        for (from, to) in &self.rules {
            if *dt == *from {
                *dt = to.clone();
            }
        }
    }

    fn remap_fields(&self, fields: &mut Fields) {
        match fields {
            Fields::Unit => {}
            Fields::Unnamed(fields) => {
                for field in &mut fields.fields {
                    if let Some(dt) = &mut field.ty {
                        self.remap_internal(dt);
                    }
                }
            }
            Fields::Named(fields) => {
                for (_, field) in &mut fields.fields {
                    if let Some(dt) = &mut field.ty {
                        self.remap_internal(dt);
                    }
                }
            }
        }
    }

    fn remap_reference(&self, reference: &mut Reference) {
        let Reference::Named(reference) = reference else {
            return;
        };

        match &mut reference.inner {
            NamedReferenceType::Recursive => {}
            NamedReferenceType::Inline { dt, .. } => self.remap_internal(dt),
            NamedReferenceType::Reference { generics, .. } => {
                for (_, dt) in generics {
                    self.remap_internal(dt);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use specta::{
        Types,
        datatype::{DataType, Field, List, NamedDataType, Primitive, Struct, Tuple},
    };

    use super::Remapper;

    #[test]
    fn remaps_multiple_rules_in_one_crawl() {
        let dt = DataType::Tuple(Tuple::new(vec![
            Primitive::u32.into(),
            Primitive::i32.into(),
        ]));

        let remapped = Remapper::new()
            .rule(Primitive::u32.into(), Primitive::str.into())
            .rule(Primitive::i32.into(), Primitive::bool.into())
            .remap_dt(dt);

        assert_eq!(
            remapped,
            DataType::Tuple(Tuple::new(vec![
                Primitive::str.into(),
                Primitive::bool.into()
            ]))
        );
    }

    #[test]
    fn rules_are_piped_in_registration_order() {
        let remapped = Remapper::new()
            .rule(Primitive::u32.into(), Primitive::i32.into())
            .rule(Primitive::i32.into(), Primitive::bool.into())
            .remap_dt(Primitive::u32.into());

        assert_eq!(remapped, Primitive::bool.into());
    }

    #[test]
    fn replacement_is_recrawled() {
        let remapped = Remapper::new()
            .rule(
                Primitive::u32.into(),
                DataType::List(List::new(Primitive::i32.into())),
            )
            .rule(Primitive::i32.into(), Primitive::bool.into())
            .remap_dt(Primitive::u32.into());

        assert_eq!(remapped, DataType::List(List::new(Primitive::bool.into())));
    }

    #[test]
    fn remaps_named_type_bodies() {
        let mut types = Types::default();
        NamedDataType::new("User", &mut types, |_, ty| {
            ty.ty = Some(
                Struct::named()
                    .field("id", Field::new(Primitive::u32.into()))
                    .field("active", Field::new(Primitive::i32.into()))
                    .build(),
            );
        });

        let types = Remapper::new()
            .rule(Primitive::u32.into(), Primitive::str.into())
            .rule(Primitive::i32.into(), Primitive::bool.into())
            .remap_types(types);

        let debug = format!("{types:?}");
        assert!(debug.contains("Primitive(str)"));
        assert!(debug.contains("Primitive(bool)"));
    }
}
