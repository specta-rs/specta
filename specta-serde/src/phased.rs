pub struct Phased<Serialize, Deserialize> {
    phantom: PhantomData<(Serialize, Deserialize)>,
}

pub trait Phased2 {
    type Serialize;
    type Deserialize;
}
impl<Serialize, Deserialize> Phased2 for Phased<Serialize, Deserialize> {
    type Serialize = Serialize;
    type Deserialize = Deserialize;
}
// TODO: Make this work. I think this is gonna be required.
// impl Type for Phased<Serialize, Deserialize> {}
