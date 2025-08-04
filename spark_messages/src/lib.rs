#![no_std]

use async_button::ButtonEvent;
use serde::{Serialize, Deserialize};

// /// Remote (master) => light (slave)
// #[derive(Serialize, Deserialize, Debug)]
// pub struct Handshake {
//     pub remote_mac: [u8; 6],
// }
//
// /// Light (slave) => remote (master)
// #[derive(Serialize, Deserialize, Debug)]
// pub struct HandshakeResponse {
//     pub light_mac: [u8; 6],
// }
//
// #[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
// pub enum Button {
//     Button0,
//     Button1,
//     Button2,
//     Button3,
// }
//
// #[derive(Serialize, Deserialize, Debug)]
// pub struct ButtonSequence {
//     pub buttons: smallvec::SmallVec<[Button; 5]>,
// }

pub const PROTOCOL_VERSION: u8 = 0;

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub protocol_version: u8,
    pub message_type: MessageType
}

#[derive(Serialize, Deserialize, Debug)]
#[non_exhaustive]
pub enum ButtonEventType {
    ShortPress {
        count: usize,
    },
    LongPress,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ButtonNumber {
    Button1,
    Button2,
    Button3,
    Button4,
}

#[derive(Serialize, Deserialize, Debug)]
#[non_exhaustive]
pub enum MessageType {
    ButtonEvent {
        button_number: ButtonNumber,
        event_type: ButtonEventType,
    }
}

impl From<ButtonEvent> for ButtonEventType {
    fn from(value: ButtonEvent) -> Self {
        match value {
            ButtonEvent::ShortPress { count } => ButtonEventType::ShortPress { count },
            ButtonEvent::LongPress => ButtonEventType::LongPress,
        }
    }
}