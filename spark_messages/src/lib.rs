use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Test {
    pub wat: u32,
    pub version: f32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum Button {
    Button0,
    Button1,
    Button2,
    Button3,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ButtonSequence {
    pub buttons: smallvec::SmallVec<[Button; 5]>,
}