use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use crate::messages::{GetId, RGB};


#[derive(EnumIter, Clone, Copy)]
pub enum LEDParameter {
    Mode,
    Colour1,
    Colour2,
    // Colour3, - Not called by the official App
    Speed,
    Brightness,
    MeterSource,
    MeterSensitivity,
    MuteMode,
    MuteColour,
    SuspendMode,
    SuspendBrightness,
}

impl GetId<u16> for LEDParameter {
    fn get_id(&self) -> u16 {
        match self {
            LEDParameter::Mode => 0,
            LEDParameter::Colour1 => 1,
            LEDParameter::Colour2 => 2,
            //LEDMessage::Colour3 => 3,
            LEDParameter::Speed => 4,
            LEDParameter::Brightness => 5,
            LEDParameter::MeterSource => 6,
            LEDParameter::MeterSensitivity => 7,
            LEDParameter::MuteMode => 8,
            LEDParameter::MuteColour => 9,
            LEDParameter::SuspendMode => 11,
            LEDParameter::SuspendBrightness => 12
        }
    }
}