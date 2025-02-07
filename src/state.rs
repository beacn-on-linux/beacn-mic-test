use crate::messages::{BeacnValue, MessageValue, RGB};
use crate::messages::led::LEDParameter;

#[derive(Default, Debug)]
pub struct DeviceState {
    pub(crate) led: LEDState,
}

impl DeviceState {
    pub fn set_led_param(&mut self, param: LEDParameter, value: BeacnValue) {
        self.led.set_param(param, value);
    }
}

#[derive(Default, Debug)]
pub struct LEDState {
    pub(crate) mode: u32,
    pub(crate) colour1: RGB,
    pub(crate) colour2: RGB,
    pub(crate) speed: i32,
    pub(crate) brightness: i32,
    pub(crate) meter_source: u32,
    pub(crate) meter_sensitivity: f32,
    pub(crate) mute_mode: u32,
    pub(crate) mute_colour: RGB,
    pub(crate) suspend_mode: u32,
    pub(crate) suspend_brightness: u32,
}

impl LEDState {
    fn set_param(&mut self, param: LEDParameter, value: BeacnValue) {
        match param {
            LEDParameter::Mode => self.mode = MessageValue::<u32>::from(value).0,
            LEDParameter::Colour1 => self.colour1 = MessageValue::<RGB>::from(value).0,
            LEDParameter::Colour2 => self.colour2 = MessageValue::<RGB>::from(value).0,
            LEDParameter::Speed => self.speed = MessageValue::<i32>::from(value).0,
            LEDParameter::Brightness => self.brightness = MessageValue::<i32>::from(value).0,
            LEDParameter::MeterSource => self.meter_source = MessageValue::<u32>::from(value).0,
            LEDParameter::MeterSensitivity => self.meter_sensitivity = MessageValue::<f32>::from(value).0,
            LEDParameter::MuteMode => self.mute_mode = MessageValue::<u32>::from(value).0,
            LEDParameter::MuteColour => self.mute_colour = MessageValue::<RGB>::from(value).0,
            LEDParameter::SuspendMode => self.suspend_mode = MessageValue::<u32>::from(value).0,
            LEDParameter::SuspendBrightness => self.suspend_brightness = MessageValue::<u32>::from(value).0,
        }
    }
}