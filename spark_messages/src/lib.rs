use serde::{Serialize, Deserialize};

/// Light (master) => remote (slave)
#[derive(Serialize, Deserialize, Debug)]
pub struct SparkI2cCommand {
    pub protocol_version: u8,
    pub kind: SparkI2cCommandKind,
}

#[derive(Serialize, Deserialize, Debug)]
#[non_exhaustive]
pub enum SparkI2cCommandKind {
    Handshake {
        light_mac: [u8; 6],
    }
}

/// Remote (slave) => light (master)
#[derive(Serialize, Deserialize, Debug)]
pub struct HandshakeCommandResponse {
    pub protocol_version: u8,
    pub remote_mac: [u8; 6],
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