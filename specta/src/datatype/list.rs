use super::DataType;

/// A list of items. This will be a [`Vec`](https://doc.rust-lang.org/std/vec/struct.Vec.html) or similar types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct List {
    ty: Box<DataType>,
    length: Option<usize>,
    unique: bool,
}

impl List {
    /// Create a new list of a given type.
    pub fn new(ty: DataType) -> Self {
        Self {
            ty: Box::new(ty),
            length: None,
            unique: false,
        }
    }

    /// Get an immutable reference to the type of the elements in the list.
    pub fn ty(&self) -> &DataType {
        &self.ty
    }

    /// Get a mutable reference to the type of the elements in the list.
    pub fn ty_mut(&mut self) -> &mut DataType {
        &mut self.ty
    }

    /// Set the type of the elements in the list.
    pub fn set_ty(&mut self, ty: DataType) {
        *self.ty = ty;
    }

    /// Get the length of the list.
    ///
    /// Length is set for `[Type; N]` arrays.
    pub fn length(&self) -> Option<usize> {
        self.length
    }

    /// Set the length of the list.
    ///
    /// Length is set for `[Type; N]` arrays.
    pub fn set_length(&mut self, length: Option<usize>) {
        self.length = length;
    }

    /// Are each elements unique? Eg. `HashSet` or `BTreeSet`
    pub fn unique(&self) -> bool {
        self.unique
    }

    /// Set whether each element is unique.
    pub fn set_unique(&mut self, unique: bool) {
        self.unique = unique;
    }
}

impl From<List> for DataType {
    fn from(t: List) -> Self {
        Self::List(t)
    }
}
