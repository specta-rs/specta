use specta::{Type, Types};
struct UnboundedWire<T>(std::marker::PhantomData<T>);
impl<T> Type for UnboundedWire<T> {
    fn definition(types: &mut Types) -> specta::datatype::DataType {
        <String as Type>::definition(types)
    }
}
#[derive(Type)]
#[specta(bound = '', collect = false)]
struct NoInferredBound<T> {
    value: UnboundedWire<T>,
}
fn main() {}