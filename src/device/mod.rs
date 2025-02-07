use std::time::Duration;
use anyhow::{anyhow, bail, Result};
use byteorder::{ByteOrder, LittleEndian};
use log::debug;
use rusb::{DeviceDescriptor, DeviceHandle, GlobalContext};
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use crate::{PID_BEACN_MIC, VID_BEACN_MIC};
use crate::messages::{BeacnValue, GetId, BeacnParameter, Message};

// This is simply something to run in a thread, and have a back and forth with the device..
pub async fn spawn_device_handler(ready: oneshot::Sender<Result<()>>, mut receiver: mpsc::Receiver<(Message, oneshot::Sender<BeacnValue>)>) {
    // Firstly, we're going to locate, and connect to the device...
    debug!("Locating Beacn Mic");
    let (device, descriptor) = find_devices();

    debug!("Connecting to and configuring Device");
    let Ok(handle) = device.open() else {
        ready.send(Err(anyhow!("Unable to Open Device"))).expect("Broken Oneshot!");
        return;
    };

    if let Err(e) = handle.set_auto_detach_kernel_driver(true) {
        ready.send(Err(anyhow!(e))).expect("Broken Oneshot!");
        return;
    }

    if let Err(e) = handle.claim_interface(3) {
        ready.send(Err(anyhow!(e))).expect("Broken Oneshot!");
        return;
    }

    if let Err(e) = handle.set_alternate_setting(3, 1) {
        ready.send(Err(anyhow!(e))).expect("Broken Oneshot!");
        return;
    }

    debug!("Device Configured, Signalling Ready");
    ready.send(Ok(())).expect("Broken Oneshot!");

    loop {
        select! {
            Some((message, receiver)) = receiver.recv() => {
                match message {
                    Message::FETCH(param) => {
                        let mut request = [0; 4];
                        request[0] = param.get_id();
                        LittleEndian::write_u16(&mut request[1..3], param.get_child_id());
                        request[3] = 0xa3;

                        let response = param_lookup(&handle, request);
                        receiver.send(response).expect("Broken Response Oneshot");
                    }
                    Message::SET((param, value)) => {
                        let mut property = [0;4];
                        property[0] = param.get_id();
                        LittleEndian::write_u16(&mut property[1..3], param.get_child_id());
                        property[3] = 0xa4;

                        // We're defining the values and their lengths, so this should be safe.
                        let concat = [property, value].concat();
                        let request = concat.try_into().unwrap();

                        // Setters don't have responses, we should follow up with a fetch and
                        // confirm the result.. For now, return nothing.
                        let new_value = param_set(&handle, request);
                        receiver.send(new_value).expect("Broken Response Oneshot");
                    }
                    Message::QUIT => {
                        receiver.send([00,00,00,00]).expect("Broken Response Oneshot");
                        break;
                    }
                }
            }
        }
    }
}

fn find_devices() -> (rusb::Device<GlobalContext>, DeviceDescriptor) {
    if let Ok(devices) = rusb::devices() {
        for device in devices.iter() {
            if let Ok(descriptor) = device.device_descriptor() {
                let bus_number = device.bus_number();
                let address = device.address();

                if descriptor.vendor_id() == VID_BEACN_MIC && descriptor.product_id() == PID_BEACN_MIC {
                    debug!("Found Beacn Mic at address {}.{}", bus_number, address);
                    return (device, descriptor);
                }
            }
        }
    }
    panic!("Unable to Locate Device!");
}

fn param_lookup(handle: &DeviceHandle<GlobalContext>, request: [u8; 4]) -> BeacnValue {
    let timeout = Duration::from_secs(3);

    // Write out the command request
    handle.write_bulk(0x03, &request, timeout).expect("Unable to Write Message");

    // Grab the response into a buffer
    let mut buf = [0; 8];
    handle.read_bulk(0x83, &mut buf, timeout).expect("Unable to Read Message");

    // Validate the header...
    if buf[0..2] != request[0..2] || buf[3] != 0xa4 {
        panic!("Invalid Response Received");
    }

    <BeacnValue>::try_from(&buf[4..8]).expect("Buffer has shrunk itself?!")
}

fn param_set(handle: &DeviceHandle<GlobalContext>, request: [u8; 8]) -> [u8; 4] {
    let timeout = Duration::from_secs(3);

    // Write out the command request
    handle.write_bulk(0x03, &request, timeout).expect("Unable to Write Message");

    // Now read the value back out, and make sure it was changed..
    let mut lookup_request = [0; 4];
    lookup_request = request[0..4].try_into().unwrap();
    lookup_request[3] = 0xa3;

    let lookup_request = lookup_request.try_into().unwrap();
    let new_value = param_lookup(handle, lookup_request);

    // Compare the new response
    if new_value != request[4..8] {
        panic!("Value was not changed on the device!");
    }

    new_value
}