use byteorder::{ByteOrder, LittleEndian};
use crate::messages::led::LEDParameter;

pub mod led;

pub type BeacnValue = [u8; 4];

pub trait GetId<T> {
    fn get_id(&self) -> T;
}


pub enum Message {
    QUIT,

    FETCH(BeacnParameter),
    SET((BeacnParameter, BeacnValue)),
}


pub enum BeacnParameter {
    LED(LEDParameter),
}

impl GetId<u8> for BeacnParameter {
    fn get_id(&self) -> u8 {
        match self {
            BeacnParameter::LED(_) => 0x01
        }
    }
}

impl BeacnParameter {
    pub fn get_child_id(&self) -> u16 {
        match self {
            BeacnParameter::LED(v) => v.get_id(),
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct RGB {
    pub(crate) red: u8,
    pub(crate) green: u8,
    pub(crate) blue: u8,
    pub(crate) alpha: u8,
}

pub struct MessageValue<T>(pub T);

impl From<BeacnValue> for MessageValue<RGB> {
    fn from(value: BeacnValue) -> Self {
        Self(
            RGB {
                red: value[2],
                green: value[1],
                blue: value[0],
                alpha: value[3],
            }
        )
    }
}

impl From<MessageValue<RGB>> for BeacnValue {
    fn from(value: MessageValue<RGB>) -> Self {
        // The format for this is ARGB, but little endian..
        [value.0.blue, value.0.green, value.0.red, 0]
    }
}

impl From<BeacnValue> for MessageValue<u32> {
    fn from(value: BeacnValue) -> Self {
        Self(LittleEndian::read_u32(&value))
    }
}

impl From<MessageValue<u32>> for BeacnValue {
    fn from(value: MessageValue<u32>) -> Self {
        let mut buf = [0; 4];
        LittleEndian::write_u32(&mut buf, value.0);

        buf
    }
}

impl From<BeacnValue> for MessageValue<i32> {
    fn from(value: BeacnValue) -> Self {
        Self(LittleEndian::read_i32(&value))
    }
}

impl From<MessageValue<i32>> for BeacnValue {
    fn from(value: MessageValue<i32>) -> Self {
        let mut buffer = [0; 4];
        LittleEndian::write_i32(&mut buffer, value.0);
        buffer
    }
}

impl From<BeacnValue> for MessageValue<f32> {
    fn from(value: BeacnValue) -> Self {
        Self(LittleEndian::read_f32(&value))
    }
}

impl From<MessageValue<f32>> for BeacnValue {
    fn from(value: MessageValue<f32>) -> Self {
        let mut buffer = [0; 4];
        LittleEndian::write_f32(&mut buffer, value.0);
        buffer
    }
}