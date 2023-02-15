use syn::Result;

use crate::utils::Attribute;

#[derive(Default)]
pub struct ContainerAttr {
    pub crate_name: Option<String>,
}

impl_parse! {
    ContainerAttr(attr, out) {
        "crate" => out.crate_name = out.crate_name.take().or(Some(attr.parse_string()?)),
    }
}

impl ContainerAttr {
    pub fn from_attrs(attrs: &mut Vec<Attribute>) -> Result<Self> {
        let mut result = Self::default();
        Self::try_from_attrs("specta", attrs, &mut result)?;
        Ok(result)
    }
}
