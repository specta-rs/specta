use specta::{
    Types,
    datatype::{DataType, Fields, NamedReferenceType, Reference},
};

/// Recursively replaces one [`DataType`] with another.
///
/// `Remap` is useful when a type should be represented differently for export
/// without changing the original Rust type or derive output. It performs exact
/// [`DataType`] equality checks and then walks into container types, structs,
/// enums, tuples, nullable values, intersections, named reference generic
/// arguments, inline references, and named type definitions in a [`Types`] map.
///
/// # Examples
///
/// Remap a single datatype:
///
/// ```rust
/// use specta::datatype::{DataType, List, Primitive};
/// use specta_util::Remap;
///
/// let remap = Remap::new(Primitive::u32.into(), Primitive::str.into());
///
/// assert_eq!(
///     remap.remap(DataType::List(List::new(Primitive::u32.into()))),
///     DataType::List(List::new(Primitive::str.into()))
/// );
/// ```
///
/// Remap every matching datatype inside a named type collection:
///
/// ```rust
/// use specta::{Types, datatype::{DataType, Field, NamedDataType, Primitive, Struct}};
/// use specta_util::Remap;
///
/// let mut types = Types::default();
/// NamedDataType::new("User", &mut types, |_, ty| {
///     ty.ty = Some(Struct::named()
///         .field("id", Field::new(Primitive::u32.into()))
///         .build());
/// });
///
/// let types = Remap::new(Primitive::u32.into(), Primitive::str.into())
///     .remap_types(types);
///
/// assert!(format!("{types:?}").contains("Primitive(str)"));
/// ```
#[derive(Debug, Clone)]
pub struct Remap {
    from: DataType,
    to: DataType,
}

impl Remap {
    /// Creates a remapper that replaces exact matches of `from` with `to`.
    pub fn new(from: DataType, to: DataType) -> Self {
        Self { from, to }
    }

    /// Applies the remap operation to a datatype, returning the remapped datatype.
    pub fn remap(&self, mut dt: DataType) -> DataType {
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
        if *dt == self.from {
            *dt = self.to.clone();
            return;
        }

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
