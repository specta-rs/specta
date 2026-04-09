use std::borrow::Cow;

use specta::datatype::{
    DataType, Deprecated, Enum, Field, Fields, Generic, NamedDataType, NamedFields, NamedReference,
    Struct, Tuple, UnnamedFields, Variant,
};

pub(crate) trait SerdeCompatNamed {
    fn name(&self) -> &Cow<'static, str>;
    fn ty(&self) -> &DataType;
    fn generics(&self) -> &[Generic];
}

impl SerdeCompatNamed for NamedDataType {
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
    fn ty(&self) -> &DataType {
        &self.inner
    }
    fn generics(&self) -> &[Generic] {
        &self.generics
    }
}

pub(crate) trait SerdeCompatField {
    fn ty_mut(&mut self) -> Option<&mut DataType>;
    fn flatten(&self) -> bool;
    fn optional(&self) -> bool;
}

impl SerdeCompatField for Field {
    fn ty_mut(&mut self) -> Option<&mut DataType> {
        self.ty.as_mut()
    }
    fn flatten(&self) -> bool {
        self.flatten
    }
    fn optional(&self) -> bool {
        self.optional
    }
}

pub(crate) trait SerdeCompatStruct {
    fn fields(&self) -> &Fields;
    fn attributes(&self) -> &specta::datatype::Attributes;
}

impl SerdeCompatStruct for Struct {
    fn fields(&self) -> &Fields {
        &self.fields
    }
    fn attributes(&self) -> &specta::datatype::Attributes {
        &self.attributes
    }
}

pub(crate) trait SerdeCompatEnum {
    fn variants(&self) -> &[(Cow<'static, str>, Variant)];
    fn attributes(&self) -> &specta::datatype::Attributes;
}

impl SerdeCompatEnum for Enum {
    fn variants(&self) -> &[(Cow<'static, str>, Variant)] {
        &self.variants
    }
    fn attributes(&self) -> &specta::datatype::Attributes {
        &self.attributes
    }
}

pub(crate) trait SerdeCompatVariant {
    fn fields(&self) -> &Fields;
    fn attributes(&self) -> &specta::datatype::Attributes;
    fn type_overridden(&self) -> bool;
}

impl SerdeCompatVariant for Variant {
    fn fields(&self) -> &Fields {
        &self.fields
    }
    fn attributes(&self) -> &specta::datatype::Attributes {
        &self.attributes
    }
    fn type_overridden(&self) -> bool {
        self.type_overridden
    }
}

pub(crate) trait SerdeCompatUnnamed {
    fn fields(&self) -> &Vec<Field>;
}

impl SerdeCompatUnnamed for UnnamedFields {
    fn fields(&self) -> &Vec<Field> {
        &self.fields
    }
}

pub(crate) trait SerdeCompatNamedFields {
    fn fields(&self) -> &Vec<(Cow<'static, str>, Field)>;
}

impl SerdeCompatNamedFields for NamedFields {
    fn fields(&self) -> &Vec<(Cow<'static, str>, Field)> {
        &self.fields
    }
}

pub(crate) trait SerdeCompatTuple {
    fn elements(&self) -> &[DataType];
}

impl SerdeCompatTuple for Tuple {
    fn elements(&self) -> &[DataType] {
        &self.elements
    }
}

pub(crate) trait SerdeCompatReference {
    fn generics(&self) -> &[(specta::datatype::GenericReference, DataType)];
}

impl SerdeCompatReference for NamedReference {
    fn generics(&self) -> &[(specta::datatype::GenericReference, DataType)] {
        &self.generics
    }
}
