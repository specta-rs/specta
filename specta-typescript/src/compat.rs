use std::borrow::Cow;

use specta::datatype::{
    DataType, Deprecated, Enum, Field, Fields, Generic, List, NamedDataType, NamedFields,
    NamedReference, Struct, Tuple, UnnamedFields, Variant,
};

pub(crate) trait TsCompat {
    fn name(&self) -> &Cow<'static, str>;
    fn docs(&self) -> &Cow<'static, str>;
    fn deprecated(&self) -> Option<&Deprecated>;
    fn module_path(&self) -> &Cow<'static, str>;
    fn generics(&self) -> &[Generic];
    fn ty(&self) -> &DataType;
}

impl TsCompat for NamedDataType {
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
    fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }
    fn deprecated(&self) -> Option<&Deprecated> {
        self.deprecated.as_ref()
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

pub(crate) trait TsFieldCompat {
    fn ty(&self) -> Option<&DataType>;
    fn ty_mut(&mut self) -> Option<&mut DataType>;
    fn inline(&self) -> bool;
    fn optional(&self) -> bool;
    fn flatten(&self) -> bool;
    fn docs(&self) -> &Cow<'static, str>;
    fn deprecated(&self) -> Option<&Deprecated>;
}

impl TsFieldCompat for Field {
    fn ty(&self) -> Option<&DataType> {
        self.ty.as_ref()
    }
    fn ty_mut(&mut self) -> Option<&mut DataType> {
        self.ty.as_mut()
    }
    fn inline(&self) -> bool {
        self.inline
    }
    fn optional(&self) -> bool {
        self.optional
    }
    fn flatten(&self) -> bool {
        self.flatten
    }
    fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }
    fn deprecated(&self) -> Option<&Deprecated> {
        self.deprecated.as_ref()
    }
}

pub(crate) trait TsVariantCompat {
    fn fields(&self) -> &Fields;
    fn fields_mut(&mut self) -> &mut Fields;
    fn skip(&self) -> bool;
    fn docs(&self) -> &Cow<'static, str>;
    fn deprecated(&self) -> Option<&Deprecated>;
}

impl TsVariantCompat for Variant {
    fn fields(&self) -> &Fields {
        &self.fields
    }
    fn fields_mut(&mut self) -> &mut Fields {
        &mut self.fields
    }
    fn skip(&self) -> bool {
        self.skip
    }
    fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }
    fn deprecated(&self) -> Option<&Deprecated> {
        self.deprecated.as_ref()
    }
}

pub(crate) trait TsStructCompat {
    fn fields(&self) -> &Fields;
    fn fields_mut(&mut self) -> &mut Fields;
}

impl TsStructCompat for Struct {
    fn fields(&self) -> &Fields {
        &self.fields
    }
    fn fields_mut(&mut self) -> &mut Fields {
        &mut self.fields
    }
}

pub(crate) trait TsEnumCompat {
    fn variants(&self) -> &[(Cow<'static, str>, Variant)];
    fn variants_mut(&mut self) -> &mut Vec<(Cow<'static, str>, Variant)>;
}

impl TsEnumCompat for Enum {
    fn variants(&self) -> &[(Cow<'static, str>, Variant)] {
        &self.variants
    }
    fn variants_mut(&mut self) -> &mut Vec<(Cow<'static, str>, Variant)> {
        &mut self.variants
    }
}

pub(crate) trait TsFieldsCompat<T> {
    fn fields(&self) -> &T;
    fn fields_mut(&mut self) -> &mut T;
}

impl TsFieldsCompat<Vec<Field>> for UnnamedFields {
    fn fields(&self) -> &Vec<Field> {
        &self.fields
    }
    fn fields_mut(&mut self) -> &mut Vec<Field> {
        &mut self.fields
    }
}

impl TsFieldsCompat<Vec<(Cow<'static, str>, Field)>> for NamedFields {
    fn fields(&self) -> &Vec<(Cow<'static, str>, Field)> {
        &self.fields
    }
    fn fields_mut(&mut self) -> &mut Vec<(Cow<'static, str>, Field)> {
        &mut self.fields
    }
}

pub(crate) trait TsListCompat {
    fn ty(&self) -> &DataType;
    fn ty_mut(&mut self) -> &mut DataType;
    fn length(&self) -> Option<usize>;
    fn set_ty(&mut self, ty: DataType);
}

impl TsListCompat for List {
    fn ty(&self) -> &DataType {
        &self.ty
    }
    fn ty_mut(&mut self) -> &mut DataType {
        &mut self.ty
    }
    fn length(&self) -> Option<usize> {
        self.length
    }
    fn set_ty(&mut self, ty: DataType) {
        self.ty = Box::new(ty);
    }
}

pub(crate) trait TsTupleCompat {
    fn elements(&self) -> &[DataType];
    fn elements_mut(&mut self) -> &mut Vec<DataType>;
}

impl TsTupleCompat for Tuple {
    fn elements(&self) -> &[DataType] {
        &self.elements
    }
    fn elements_mut(&mut self) -> &mut Vec<DataType> {
        &mut self.elements
    }
}

pub(crate) trait TsNamedReferenceCompat {
    fn generics(&self) -> &[(specta::datatype::GenericReference, DataType)];
}

impl TsNamedReferenceCompat for NamedReference {
    fn generics(&self) -> &[(specta::datatype::GenericReference, DataType)] {
        &self.generics
    }
}
