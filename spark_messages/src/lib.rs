use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Test {
    pub wat: u32,
    pub version: f32,
}

