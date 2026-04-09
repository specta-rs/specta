use std::borrow::Cow;

use specta::datatype::{
    DataType, Deprecated, Enum, Field, Fields, Generic, List, NamedDataType, NamedFields,
    NamedReference, Struct, Tuple, UnnamedFields, Variant,
};

pub(crate) trait ZodNamedCompat {
    fn name(&self) -> &Cow<'static, str>;
    fn module_path(&self) -> &Cow<'static, str>;
    fn generics(&self) -> &[Generic];
    fn ty(&self) -> &DataType;
}

impl ZodNamedCompat for NamedDataType {
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
    fn module_path(&self) -> &Cow<'static, str> {
        &self.module_path
    }
    fn generics(&self) -> &[Generic] {
        &self.generics
    }
    fn ty(&self) -> &DataType {
        &self.inner
    }
}

pub(crate) trait ZodFieldCompat {
    fn ty(&self) -> Option<&DataType>;
    fn inline(&self) -> bool;
    fn flatten(&self) -> bool;
    fn optional(&self) -> bool;
}

impl ZodFieldCompat for Field {
    fn ty(&self) -> Option<&DataType> {
        self.ty.as_ref()
    }
    fn inline(&self) -> bool {
        self.inline
    }
    fn flatten(&self) -> bool {
        self.flatten
    }
    fn optional(&self) -> bool {
        self.optional
    }
}

pub(crate) trait ZodVariantCompat {
    fn fields(&self) -> &Fields;
    fn skip(&self) -> bool;
}

impl ZodVariantCompat for Variant {
    fn fields(&self) -> &Fields {
        &self.fields
    }
    fn skip(&self) -> bool {
        self.skip
    }
}

pub(crate) trait ZodStructCompat {
    fn fields(&self) -> &Fields;
}
impl ZodStructCompat for Struct {
    fn fields(&self) -> &Fields {
        &self.fields
    }
}

pub(crate) trait ZodEnumCompat {
    fn variants(&self) -> &[(Cow<'static, str>, Variant)];
}
impl ZodEnumCompat for Enum {
    fn variants(&self) -> &[(Cow<'static, str>, Variant)] {
        &self.variants
    }
}

pub(crate) trait ZodNamedFieldsCompat {
    fn fields(&self) -> &Vec<(Cow<'static, str>, Field)>;
}
impl ZodNamedFieldsCompat for NamedFields {
    fn fields(&self) -> &Vec<(Cow<'static, str>, Field)> {
        &self.fields
    }
}

pub(crate) trait ZodUnnamedFieldsCompat {
    fn fields(&self) -> &Vec<Field>;
}
impl ZodUnnamedFieldsCompat for UnnamedFields {
    fn fields(&self) -> &Vec<Field> {
        &self.fields
    }
}

pub(crate) trait ZodListCompat {
    fn ty(&self) -> &DataType;
    fn length(&self) -> Option<usize>;
}
impl ZodListCompat for List {
    fn ty(&self) -> &DataType {
        &self.ty
    }
    fn length(&self) -> Option<usize> {
        self.length
    }
}

pub(crate) trait ZodTupleCompat {
    fn elements(&self) -> &[DataType];
}
impl ZodTupleCompat for Tuple {
    fn elements(&self) -> &[DataType] {
        &self.elements
    }
}

pub(crate) trait ZodNamedReferenceCompat {
    fn generics(&self) -> &[(specta::datatype::GenericReference, DataType)];
}
impl ZodNamedReferenceCompat for NamedReference {
    fn generics(&self) -> &[(specta::datatype::GenericReference, DataType)] {
        &self.generics
    }
}
