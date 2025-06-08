use serde::{Serialize, Deserialize};

/// Remote (master) => light (slave)
#[derive(Serialize, Deserialize, Debug)]
pub struct Handshake {
    pub remote_mac: [u8; 6],
}

/// Light (slave) => remote (master)
#[derive(Serialize, Deserialize, Debug)]
pub struct HandshakeResponse {
    pub light_mac: [u8; 6],
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