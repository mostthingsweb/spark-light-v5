use bincode::{Decode, Encode};

#[derive(Decode, Encode, Debug)]
pub struct Test {
    pub wat: u32,
    pub version: f32,
}
