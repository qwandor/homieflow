// Copyright 2022 the homieflow authors.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

use super::homie::color_absolute_to_property_value;
use super::homie::get_homie_device_by_id;
use crate::homie::percentage_to_property_value;
use crate::types::errors::InternalError;
use crate::types::user;
use crate::State;
use google_smart_home::device::Command as GHomeCommand;
use google_smart_home::execute::request;
use google_smart_home::execute::request::PayloadCommandDevice;
use google_smart_home::execute::request::PayloadCommandExecution;
use google_smart_home::execute::response;
use homie_controller::Datatype;
use homie_controller::Device;
use homie_controller::HomieController;
use homie_controller::Node;
use homie_controller::Value;
use std::collections::HashMap;

#[tracing::instrument(name = "Execute", skip(state), err)]
pub async fn handle(
    state: State,
    user_id: user::ID,
    payload: &request::Payload,
) -> Result<response::Payload, InternalError> {
    let requests = payload
        .commands
        .iter()
        .flat_map(|cmd| cmd.execution.iter().zip(cmd.devices.iter()));

    if let Some(homie_controller) = state.homie_controllers.get(&user_id) {
        let commands =
            execute_homie_devices(homie_controller, &homie_controller.devices(), requests).await;
        Ok(response::Payload {
            error_code: None,
            debug_string: None,
            commands,
        })
    } else {
        Ok(response::Payload {
            error_code: Some("authFailure".to_string()),
            debug_string: Some("No such user".to_string()),
            commands: vec![],
        })
    }
}

async fn execute_homie_devices<'a>(
    controller: &HomieController,
    devices: &HashMap<String, Device>,
    requests: impl Iterator<Item = (&'a PayloadCommandExecution, &'a PayloadCommandDevice)>,
) -> Vec<response::PayloadCommand> {
    let mut responses = vec![];
    for (execution, device) in requests {
        responses.push(execute_homie_device(controller, devices, execution, device).await);
    }
    responses
}

async fn execute_homie_device(
    controller: &HomieController,
    devices: &HashMap<String, Device>,
    execution: &PayloadCommandExecution,
    command_device: &PayloadCommandDevice,
) -> response::PayloadCommand {
    let ids = vec![command_device.id.to_owned()];

    if let Some((device, node)) = get_homie_device_by_id(devices, &command_device.id) {
        // TODO: Check if device is offline?
        match &execution.command {
            GHomeCommand::OnOff(onoff) => {
                if let Some(on) = node.properties.get("on") {
                    if on.datatype == Some(Datatype::Boolean) {
                        return set_value(controller, device, node, "on", onoff.on, ids).await;
                    }
                }
            }
            GHomeCommand::BrightnessAbsolute(brightness_absolute) => {
                if let Some(brightness) = node.properties.get("brightness") {
                    if let Some(value) =
                        percentage_to_property_value(brightness, brightness_absolute.brightness)
                    {
                        return set_value(controller, device, node, "brightness", value, ids).await;
                    }
                }
            }
            GHomeCommand::ColorAbsolute(color_absolute) => {
                if let Some(color) = node.properties.get("color") {
                    if let Some(value) = color_absolute_to_property_value(color, color_absolute) {
                        return set_value(controller, device, node, "color", value, ids).await;
                    }
                }
            }
            _ => {}
        }
        command_error(ids, "actionNotAvailable")
    } else {
        command_error(ids, "deviceNotFound")
    }
}

async fn set_value(
    controller: &HomieController,
    device: &Device,
    node: &Node,
    property_id: &str,
    value: impl Value,
    ids: Vec<String>,
) -> response::PayloadCommand {
    if controller
        .set(&device.id, &node.id, property_id, value)
        .await
        .is_err()
    {
        command_error(ids, "transientError")
    } else {
        response::PayloadCommand {
            ids,
            status: response::PayloadCommandStatus::Pending,
            states: Default::default(),
            error_code: None,
        }
    }
}

fn command_error(ids: Vec<String>, error_code: &str) -> response::PayloadCommand {
    response::PayloadCommand {
        ids,
        status: response::PayloadCommandStatus::Error,
        states: Default::default(),
        error_code: Some(error_code.to_string()),
    }
}
