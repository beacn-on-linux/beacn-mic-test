mod messages;
mod device;
mod state;

use std::thread::sleep;
use std::time::Duration;
use anyhow::{anyhow, bail, Result};
use byteorder::{ByteOrder, LittleEndian};
use eframe::Frame;
use egui::{Context, Ui};
use log::{debug, LevelFilter};
use rusb::{Device, DeviceDescriptor, DeviceHandle, GlobalContext};
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};
use strum::IntoEnumIterator;
use tokio::sync::{mpsc, oneshot};
use tokio::task;
use tokio::task::{block_in_place, spawn_blocking};
use crate::device::spawn_device_handler;
use crate::messages::{BeacnValue, BeacnParameter, MessageValue, Message, RGB};
use crate::messages::led::LEDParameter;
use crate::messages::Message::SET;
use crate::state::DeviceState;

const VID_BEACN_MIC: u16 = 0x33ae;
const PID_BEACN_MIC: u16 = 0x0001;

#[tokio::main]
async fn main() -> Result<()> {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])?;

    let (ready_tx, ready_rx) = oneshot::channel();
    let (messenger_tx, messenger_rx) = mpsc::channel(30);

    debug!("Spawning Device Handler..");
    task::spawn(spawn_device_handler(ready_tx, messenger_rx));

    // If the setup errors out, bail out too.
    debug!("Waiting for Device Handler to Signal Ready");
    ready_rx.await??;
    debug!("Device Handler ready, attempting to load State from Device");

    let mut state = DeviceState::default();

    // Ok, lets load all the LED settings at once..
    debug!("Loading LED States");
    for parameter in LEDParameter::iter() {
        let message = Message::FETCH(BeacnParameter::LED(parameter));
        let (response_tx, response_rx) = oneshot::channel();

        messenger_tx.send((message, response_tx)).await?;
        let value = response_rx.await?;

        state.set_led_param(parameter, value);
    }
    debug!("Loading Complete, values discovered:");
    debug!("{:#?}", state);

    // Create an oneshot for send/receive message...
    debug!("Spawning UI..");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([460., 520.]),
        ..Default::default()
    };

    eframe::run_native(
        "Beacn Mic Configuration",
        options,
        Box::new(|cc| {
            Ok(Box::new(BeacnApp::new(state, messenger_tx.clone())))
        }),
    ).map_err(|e| anyhow!("Failed: {}", e))?;


    // Send a quit message.
    let (response_tx, response_rx) = oneshot::channel();
    messenger_tx.send((Message::QUIT, response_tx)).await?;
    response_rx.await?;

    Ok(())
}


struct BeacnApp {
    state: DeviceState,
    sender: mpsc::Sender<(Message, oneshot::Sender<BeacnValue>)>,

    // We need to extract the colours to eGUI values.
    colour1: [u8; 3],
    colour2: [u8; 3],
    mute_colour: [u8; 3],
}

impl BeacnApp {
    fn new(state: DeviceState, sender: mpsc::Sender<(Message, oneshot::Sender<BeacnValue>)>) -> Self {
        let colour1 = [state.led.colour1.red, state.led.colour1.green, state.led.colour1.blue];
        let colour2 = [state.led.colour2.red, state.led.colour2.green, state.led.colour2.blue];

        let mute_colour = [state.led.mute_colour.red, state.led.mute_colour.green, state.led.mute_colour.blue];

        Self {
            state,
            sender,
            colour1,
            colour2,
            mute_colour,
        }
    }

    // These are some common elements used in multiple pages..
    fn draw_primary_colour(&mut self, ui: &mut Ui) {
        ui.label("Primary Colour");
        if ui.color_edit_button_srgb(&mut self.colour1).changed() {
            let message = MessageValue::<RGB>(RGB {
                red: self.colour1[0],
                green: self.colour1[1],
                blue: self.colour1[2],
                alpha: 0,
            });
            let message = SET((BeacnParameter::LED(LEDParameter::Colour1), BeacnValue::from(message)));
            self.send_message(message);
        }
        ui.add_space(4.);
    }

    fn draw_secondary_colour(&mut self, ui: &mut Ui) {
        ui.label("Secondary Colour");
        if ui.color_edit_button_srgb(&mut self.colour2).changed() {
            let message = MessageValue::<RGB>(RGB {
                red: self.colour1[0],
                green: self.colour1[1],
                blue: self.colour1[2],
                alpha: 0,
            });
            let message = SET((BeacnParameter::LED(LEDParameter::Colour2), BeacnValue::from(message)));
            self.send_message(message);
        }
        ui.add_space(4.);
    }

    fn draw_speed_direction(&mut self, ui: &mut Ui) {
        ui.label("Speed and Direction");
        if ui.add(egui::Slider::new(&mut self.state.led.speed, -10..=10)).changed() {
            let message = MessageValue::<i32>(self.state.led.speed);
            let message = SET((BeacnParameter::LED(LEDParameter::Speed), BeacnValue::from(message)));
            self.send_message(message);
        };
        ui.add_space(4.);
    }

    fn draw_meter_sensitivity(&mut self, ui: &mut Ui) {
        ui.label("Meter Sensitivity");
        if ui.add(egui::Slider::new(&mut self.state.led.meter_sensitivity, 0.0..=10.0)).changed() {
            let message = MessageValue::<f32>(self.state.led.meter_sensitivity);
            let message = SET((BeacnParameter::LED(LEDParameter::MeterSensitivity), BeacnValue::from(message)));
            self.send_message(message);
        }
        ui.add_space(4.);
    }

    fn draw_meter_source(&mut self, ui: &mut Ui) {
        ui.label("Meter Source");
        egui::ComboBox::from_label("")
            .selected_text(
                match self.state.led.meter_source {
                    0 => "Microphone",
                    1 => "Headphones",
                    _ => "Unknown?",
                }
            )
            .show_ui(ui, |ui| {
                // TODO: Clean up the .changed code..
                if ui.selectable_value(&mut self.state.led.meter_source, 0, "Microphone").changed() {
                    let message = MessageValue::<u32>(self.state.led.meter_source);
                    let message = SET((BeacnParameter::LED(LEDParameter::MeterSource), BeacnValue::from(message)));
                    self.send_message(message);
                }
                if ui.selectable_value(&mut self.state.led.meter_source, 1, "Headphones").changed() {
                    let message = MessageValue::<u32>(self.state.led.meter_source);
                    let message = SET((BeacnParameter::LED(LEDParameter::MeterSource), BeacnValue::from(message)));
                    self.send_message(message);
                }
            });
        ui.add_space(4.);
    }

    fn draw_ring_brightness(&mut self, ui: &mut Ui) {
        ui.label("Ring Brightness");
        if ui.add(egui::Slider::new(&mut self.state.led.brightness, 0..=100)).changed() {
            let message = MessageValue::<i32>(self.state.led.brightness);
            let message = SET((BeacnParameter::LED(LEDParameter::Brightness), BeacnValue::from(message)));
            self.send_message(message);
        }
        ui.add_space(4.);
    }

    // This can be done better, there's no reason to duplicate code here between pages..
    fn draw_gradient_settings(&mut self, ui: &mut Ui) {
        self.draw_primary_colour(ui);
        self.draw_secondary_colour(ui);
        self.draw_speed_direction(ui);
        self.draw_ring_brightness(ui);
    }

    fn draw_solid_settings(&mut self, ui: &mut Ui) {
        self.draw_primary_colour(ui);
        self.draw_ring_brightness(ui);
    }

    fn draw_reactive_settings(&mut self, ui: &mut Ui) {
        ui.label("Behaviour");

        ui.vertical(|ui| {
            if ui.radio_value(&mut self.state.led.mode, 0x05, "Whole Ring Meter").changed() {
                self.set_mode(0x05);
            }
            if ui.radio_value(&mut self.state.led.mode, 0x06, "Bar Meter Up").changed() {
                self.set_mode(0x06);
            }
            if ui.radio_value(&mut self.state.led.mode, 0x07, "Bar Meter Down").changed() {
                self.set_mode(0x07);
            }
        });
        ui.add_space(4.);

        self.draw_primary_colour(ui);
        self.draw_secondary_colour(ui);
        self.draw_meter_sensitivity(ui);
        self.draw_ring_brightness(ui);
        self.draw_meter_source(ui);
    }

    fn draw_sparkle_settings(&mut self, ui: &mut Ui) {
        ui.label("Behaviour");

        ui.vertical(|ui| {
            if ui.radio_value(&mut self.state.led.mode, 0x0a, "Sparkle Random").changed() {
                self.set_mode(0x0a);
            }
            if ui.radio_value(&mut self.state.led.mode, 0x0b, "Sparkle Meter").changed() {
                self.set_mode(0x0b)
            }
        });
        ui.add_space(4.);

        self.draw_primary_colour(ui);
        self.draw_secondary_colour(ui);
        self.draw_meter_sensitivity(ui);
        self.draw_speed_direction(ui);
        self.draw_ring_brightness(ui);
        self.draw_meter_source(ui);
    }

    fn draw_spectrum_settings(&mut self, ui: &mut Ui) {
        self.draw_speed_direction(ui);
        self.draw_ring_brightness(ui);
    }

    fn set_mode(&mut self, mode: u32) {
        let value = MessageValue::<u32>(mode);
        let message = SET((BeacnParameter::LED(LEDParameter::Mode), BeacnValue::from(value)));
        self.send_message(message);
    }

    fn send_message(&self, message: Message) -> BeacnValue {
        let (response_tx, mut response_rx) = oneshot::channel();
        self.sender.try_send((message, response_tx)).expect("Failed to Send Message");

        // The reader is an async message, so we need to handle it in a sync way
        let millis_wait = Duration::from_millis(5);
        let max_wait = 1000 / 5;
        let mut count = 0;

        while count < max_wait {
            let response = response_rx.try_recv();
            match response {
                Ok(value) => return value,
                Err(e) => {
                    count += 1;
                    sleep(millis_wait);
                }
            }
        }
        panic!("Did not receive a response in time!");
    }
}

impl eframe::App for BeacnApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        // Ok, the panel order is important, as they define how they are 'stretched', because we want the
        // global settings to span the entire bottom, we need to do that first..
        egui::TopBottomPanel::bottom("global").exact_height(180.).resizable(false).show(ctx, |ui| {
            ui.heading("Other Lighting Options");
            egui::Grid::new("bottom_grid").num_columns(2).min_col_width(200.).show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label("When Muted");
                    ui.vertical(|ui| {
                        ui.radio_value(&mut self.state.led.mute_mode, 0, "Do Nothing");
                        ui.radio_value(&mut self.state.led.mute_mode, 1, "Turn LED ring to a solid colour");
                        ui.radio_value(&mut self.state.led.mute_mode, 2, "Turn off LED ring");
                    });
                    ui.add_space(4.);

                    ui.label("Colour");
                    ui.color_edit_button_srgb(&mut self.mute_colour);
                });

                ui.vertical(|ui| {
                    ui.label("When USB Is Suspended");
                    ui.vertical(|ui| {
                        ui.radio_value(&mut self.state.led.suspend_mode, 0, "Do Nothing");
                        ui.radio_value(&mut self.state.led.suspend_mode, 1, "Turn off LED ring");
                        ui.radio_value(&mut self.state.led.suspend_mode, 2, "Change the brightness to:");
                    });
                    ui.add_space(4.);
                    ui.add(egui::Slider::new(&mut self.state.led.suspend_brightness, 0..=100));
                });
                ui.end_row();
            });
        });

        // For the others, left first, then right.
        egui::SidePanel::left("mode").resizable(false).default_width(200.).show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("Lighting Style");

                // Our Match for Reactive / Sparkle depend on the active selection, so we'll match
                // directly if it's active, or have a 'default' if it's not.
                let reactive_match = match self.state.led.mode {
                    0x05..=0x07 => self.state.led.mode,
                    _ => 0x05
                };
                let sparkle_match = match self.state.led.mode {
                    0x0a..=0x0b => self.state.led.mode,
                    _ => 0x0a
                };

                if ui.selectable_value(&mut self.state.led.mode, 0x00, "Solid Colour").clicked() {
                    self.set_mode(0x00);
                }
                if ui.selectable_value(&mut self.state.led.mode, 0x03, "Gradient").clicked() {
                    self.set_mode(0x03);
                }
                if ui.selectable_value(&mut self.state.led.mode, reactive_match, "Reactive Meter").clicked() {
                    self.set_mode(reactive_match);
                }
                if ui.selectable_value(&mut self.state.led.mode, sparkle_match, "Solid Sparkle").clicked() {
                    self.set_mode(sparkle_match);
                }
                if ui.selectable_value(&mut self.state.led.mode, 0x01, "Spectrum Cycle").clicked() {
                    self.set_mode(0x01);
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state.led.mode as u8 {
                0x00 => self.draw_solid_settings(ui),
                0x01 => self.draw_spectrum_settings(ui),
                0x03 => self.draw_gradient_settings(ui),
                0x05..=0x07 => self.draw_reactive_settings(ui),
                0x0a..=0x0b => self.draw_sparkle_settings(ui),
                _ => {}
            }
        });
    }
}