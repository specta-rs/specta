//! TODO: Move this somewhere else. Maybe out of core and maybe properly expose?
//!
//! TODO: Option to build types with generics???

use std::{borrow::Cow, fmt::Debug};

use crate::{datatype::{DeprecatedType, EnumRepr, EnumType, EnumVariant, Field, Fields, List, NamedFields, StructType, UnnamedFields}, DataType};

impl List {
    #[doc(hidden)] // TODO: Expose
    pub fn new(ty: DataType) -> Self {
        Self {
            ty: Box::new(ty),
            length: None,
            unique: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructBuilder<F = ()> {
    name: Cow<'static, str>,
    fields: F,
}

impl StructBuilder<()> {
    pub fn unit(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            fields: (),
        }
    }

    pub fn build(self) -> DataType {
            DataType::Struct(StructType {
                name: self.name,
                sid: None,
                generics: vec![],
                fields: Fields::Unit
            })
        }
}

impl StructBuilder<NamedFields> {
    pub fn named(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            fields: NamedFields { fields: Default::default(), tag: Default::default() }
        }
    }

    pub fn field(mut self, name: impl Into<Cow<'static, str>>, field: FieldBuilder) -> Self {
        self.fields.fields.push((name.into(), field.0));
        self
    }

    pub fn field_mut(&mut self, name: impl Into<Cow<'static, str>>, field: FieldBuilder) {
        self.fields.fields.push((name.into(), field.0));
    }

    pub fn build(self) -> DataType {
            DataType::Struct(StructType {
                name: self.name,
                sid: None,
                generics: vec![],
                fields: Fields::Named(self.fields),
            })
        }
}

impl StructBuilder<UnnamedFields> {
    pub fn unnamed(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            fields: UnnamedFields { fields: Default::default() },
        }
    }

    pub fn field(mut self, field: FieldBuilder) -> Self {
        self.fields.fields.push(field.0);
        self
    }

    pub fn field_mut(&mut self,field: FieldBuilder) {
        self.fields.fields.push(field.0);
    }

    pub fn build(self) -> DataType {
            DataType::Struct(StructType {
                name: self.name,
                sid: None,
                generics: vec![],
                fields: Fields::Unnamed(self.fields),
            })
        }
}

#[derive(Debug, Clone)]
pub struct FieldBuilder(Field);

impl FieldBuilder {
    pub fn new(ty: DataType) -> Self {
        Self(Field {
            optional: false,
            flatten: false,
            deprecated: None,
            docs: "".into(),
            ty: Some(ty),
        })
    }

    pub fn skip(mut self) -> Self {
        self.0.ty = None;
        self
    }

    pub fn set_skip(&mut self) {
        self.0.ty = None;
    }

    pub fn optional(mut self) -> Self {
        self.0.optional = true;
        self
    }

    pub fn set_optional(&mut self, optional: bool) {
        self.0.optional = optional;
    }

    pub fn flatten(mut self) -> Self {
        self.0.flatten = true;
        self
    }

    pub fn set_flatten(&mut self, flatten: bool) {
        self.0.flatten = flatten;
    }

    pub fn deprecated(mut self, reason: DeprecatedType) -> Self {
        self.0.deprecated = Some(reason);
        self
    }

    pub fn set_deprecated(&mut self, reason: DeprecatedType) {
        self.0.deprecated = Some(reason);
    }

    pub fn docs(mut self, docs: impl Into<Cow<'static, str>>) -> Self {
        self.0.docs = docs.into();
        self
    }

    pub fn set_docs(&mut self, docs: impl Into<Cow<'static, str>>) {
        self.0.docs = docs.into();
    }
}

pub struct EnumBuilder {
    name: Cow<'static, str>,
    variants: Vec<(Cow<'static, str>, EnumVariant)>,
}

impl EnumBuilder {
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            variants: vec![],
        }
    }

    // TODO: Configurable `repr`

    // pub fn variant(mut self, name: impl Into<Cow<'static, str>>, ty: DataType) -> Self {
    //     self.variants.push((name.into(), EnumVariant));
    //     self
    // }

    // pub fn variant_mut(&mut self, name: impl Into<Cow<'static, str>>, ty: DataType) {
    //     self.variants.push((name.into(), EnumVariant::new(ty)));
    // }

    pub fn build(self) -> DataType {
        DataType::Enum(EnumType {
            name: self.name,
            sid: None,
            skip_bigint_checks: false,
            repr: EnumRepr::External,
            generics: Default::default(),
            variants: self.variants,
        })
    }
}

pub struct VariantBuilder(EnumVariant);

impl VariantBuilder {
    pub fn unit() -> Self {
        Self(EnumVariant {
            skip: false,
            docs: "".into(),
            deprecated: None,
            fields: Fields::Unit
        })
    }

    pub fn named(name: impl Into<Cow<'static, str>>) -> Self {
        // Self(EnumVariant {
        //     skip: false,
        //     docs: "".into(),
        //     deprecated: None,
        //     fields: EnumVariants::Named(NamedFields { fields: (), tag: () })
        // })
        todo!();
    }
}
