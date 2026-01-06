//! TODO: Move this somewhere else. Maybe out of core and maybe properly expose?
//!
//! TODO: Option to build types with generics???

use std::{borrow::Cow, fmt::Debug, panic::Location};

use crate::{
    DataType, TypeCollection,
    datatype::{
        ArcId, DeprecatedType, EnumVariant, Field, Fields, Generic, NamedDataType, NamedFields,
        Struct, UnnamedFields,
    },
};

#[derive(Debug, Clone)]
pub struct StructBuilder<F = ()> {
    pub(crate) fields: F,
}

impl StructBuilder<NamedFields> {
    pub fn field(mut self, name: impl Into<Cow<'static, str>>, field: Field) -> Self {
        self.fields.fields.push((name.into(), field));
        self
    }

    pub fn field_mut(&mut self, name: impl Into<Cow<'static, str>>, field: Field) {
        self.fields.fields.push((name.into(), field));
    }

    pub fn build(self) -> DataType {
        DataType::Struct(Struct {
            fields: Fields::Named(self.fields),
            attributes: Default::default(),
        })
    }
}

impl StructBuilder<UnnamedFields> {
    pub fn field(mut self, field: Field) -> Self {
        self.fields.fields.push(field);
        self
    }

    pub fn field_mut(&mut self, field: Field) {
        self.fields.fields.push(field);
    }

    // TODO: Allow configuring attributes???

    pub fn build(self) -> DataType {
        DataType::Struct(Struct {
            fields: Fields::Unnamed(self.fields),
            attributes: Default::default(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct VariantBuilder<V = ()> {
    pub(crate) v: EnumVariant,
    pub(crate) variant: V,
}

impl<T> VariantBuilder<T> {
    pub fn skip(mut self) -> Self {
        self.v.skip = true;
        self
    }

    pub fn docs(mut self, docs: Cow<'static, str>) -> Self {
        self.v.docs = docs;
        self
    }

    pub fn deprecated(mut self, reason: DeprecatedType) -> Self {
        self.v.deprecated = Some(reason);
        self
    }
}

impl VariantBuilder<NamedFields> {
    pub fn field(mut self, name: impl Into<Cow<'static, str>>, field: Field) -> Self {
        match &mut self.v.fields {
            Fields::Named(f) => f.fields.push((name.into(), field)),
            _ => unreachable!(),
        }
        self
    }

    pub fn field_mut(mut self, name: impl Into<Cow<'static, str>>, field: Field) -> Self {
        match &mut self.v.fields {
            Fields::Named(f) => f.fields.push((name.into(), field)),
            _ => unreachable!(),
        }
        self
    }

    pub fn build(mut self) -> EnumVariant {
        self.v.fields = Fields::Named(self.variant);
        self.v
    }
}

impl From<VariantBuilder<NamedFields>> for EnumVariant {
    fn from(val: VariantBuilder<NamedFields>) -> Self {
        val.build()
    }
}

impl VariantBuilder<UnnamedFields> {
    pub fn field(mut self, field: Field) -> Self {
        match &mut self.v.fields {
            Fields::Unnamed(f) => f.fields.push(field),
            _ => unreachable!(),
        }
        self
    }

    pub fn field_mut(mut self, field: Field) -> Self {
        match &mut self.v.fields {
            Fields::Unnamed(f) => f.fields.push(field),
            _ => unreachable!(),
        }
        self
    }

    pub fn build(mut self) -> EnumVariant {
        self.v.fields = Fields::Unnamed(self.variant);
        self.v
    }
}

impl From<VariantBuilder<UnnamedFields>> for EnumVariant {
    fn from(val: VariantBuilder<UnnamedFields>) -> Self {
        val.build()
    }
}

#[derive(Debug, Clone)]
pub struct NamedDataTypeBuilder {
    pub(crate) name: Cow<'static, str>,
    pub(crate) docs: Cow<'static, str>,
    pub(crate) deprecated: Option<DeprecatedType>,
    pub(crate) module_path: Cow<'static, str>,
    pub(crate) generics: Vec<Generic>,
    pub(crate) inner: DataType,
}

impl NamedDataTypeBuilder {
    pub fn new(name: impl Into<Cow<'static, str>>, generics: Vec<Generic>, dt: DataType) -> Self {
        Self {
            name: name.into(),
            docs: Cow::Borrowed(""),
            deprecated: None,
            module_path: Cow::Borrowed("virtual"),
            generics,
            inner: dt,
        }
    }

    /// Set the module path that this type was defined in.
    ///
    /// The value for this is usually determined by [`module_path`](std::module_path). It's important you keep this in the form `edge::edge::edge::node` or `node`.
    pub fn module_path(mut self, module_path: impl Into<Cow<'static, str>>) -> Self {
        self.module_path = module_path.into();
        self
    }

    pub fn docs(mut self, docs: impl Into<Cow<'static, str>>) -> Self {
        self.docs = docs.into();
        self
    }

    pub fn deprecated(mut self, deprecated: DeprecatedType) -> Self {
        self.deprecated = Some(deprecated);
        self
    }

    #[track_caller]
    pub fn build(self, types: &mut TypeCollection) -> NamedDataType {
        let ndt = NamedDataType {
            id: ArcId::Dynamic(Default::default()),
            name: self.name,
            docs: self.docs,
            deprecated: self.deprecated,
            module_path: self.module_path,
            location: Location::caller().to_owned(),
            generics: self.generics,
            inner: self.inner,
        };

        types.0.insert(ndt.id.clone(), Some(ndt.clone()));
        ndt
    }
}
