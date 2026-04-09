use std::borrow::Cow;

use specta::datatype::{
    DataType, Deprecated, Enum, Field, Fields, Generic, List, NamedDataType, NamedFields,
    NamedReference, Struct, Tuple, UnnamedFields, Variant,
};

pub(crate) trait GoNamedCompat {
    fn docs(&self) -> &Cow<'static, str>;
    fn name(&self) -> &Cow<'static, str>;
    fn generics(&self) -> &[Generic];
    fn ty(&self) -> &DataType;
}

impl GoNamedCompat for NamedDataType {
    fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
    fn generics(&self) -> &[Generic] {
        &self.generics
    }
    fn ty(&self) -> &DataType {
        &self.inner
    }
}

pub(crate) trait GoStructCompat {
    fn fields(&self) -> &Fields;
    fn set_fields(&mut self, fields: Fields);
}

impl GoStructCompat for Struct {
    fn fields(&self) -> &Fields {
        &self.fields
    }
    fn set_fields(&mut self, fields: Fields) {
        self.fields = fields;
    }
}

pub(crate) trait GoEnumCompat {
    fn variants(&self) -> &[(Cow<'static, str>, Variant)];
}

impl GoEnumCompat for Enum {
    fn variants(&self) -> &[(Cow<'static, str>, Variant)] {
        &self.variants
    }
}

pub(crate) trait GoVariantCompat {
    fn docs(&self) -> &Cow<'static, str>;
    fn fields(&self) -> &Fields;
}

impl GoVariantCompat for Variant {
    fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }
    fn fields(&self) -> &Fields {
        &self.fields
    }
}

pub(crate) trait GoFieldCompat {
    fn docs(&self) -> &Cow<'static, str>;
    fn ty(&self) -> Option<&DataType>;
    fn optional(&self) -> bool;
}

impl GoFieldCompat for Field {
    fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }
    fn ty(&self) -> Option<&DataType> {
        self.ty.as_ref()
    }
    fn optional(&self) -> bool {
        self.optional
    }
}

pub(crate) trait GoNamedFieldsCompat {
    fn fields(&self) -> &Vec<(Cow<'static, str>, Field)>;
}

impl GoNamedFieldsCompat for NamedFields {
    fn fields(&self) -> &Vec<(Cow<'static, str>, Field)> {
        &self.fields
    }
}

pub(crate) trait GoUnnamedFieldsCompat {
    fn fields(&self) -> &Vec<Field>;
}

impl GoUnnamedFieldsCompat for UnnamedFields {
    fn fields(&self) -> &Vec<Field> {
        &self.fields
    }
}

pub(crate) trait GoListCompat {
    fn ty(&self) -> &DataType;
}

impl GoListCompat for List {
    fn ty(&self) -> &DataType {
        &self.ty
    }
}

pub(crate) trait GoTupleCompat {
    fn elements(&self) -> &[DataType];
}

impl GoTupleCompat for Tuple {
    fn elements(&self) -> &[DataType] {
        &self.elements
    }
}

pub(crate) trait GoNamedReferenceCompat {
    fn generics(&self) -> &[(specta::datatype::GenericReference, DataType)];
}

impl GoNamedReferenceCompat for NamedReference {
    fn generics(&self) -> &[(specta::datatype::GenericReference, DataType)] {
        &self.generics
    }
}
