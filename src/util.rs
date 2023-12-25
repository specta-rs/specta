use std::borrow::Cow;

use crate::DataType;

pub(crate) fn as_ref<'a, 'b>(c: &'a Cow<'b, [DataType]>) -> Cow<'a, [DataType]> {
    match c {
        Cow::Borrowed(c) => Cow::Borrowed(c),
        Cow::Owned(c) => Cow::Borrowed(&c[..]),
    }
}
