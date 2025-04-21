use bincode::{Decode, Encode};
use serde::Serialize;
use serde::Deserialize;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[derive(Decode, Encode, Debug)]
pub struct Test {
    pub wat: u32,
    pub version: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
