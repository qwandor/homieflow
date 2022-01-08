use super::homie::get_homie_device_by_id;
use super::homie::property_value_to_color;
use super::homie::property_value_to_number;
use super::homie::property_value_to_percentage;
use crate::State;
use google_smart_home::query::request;
use google_smart_home::query::response;
use homie_controller::Device;
use houseflow_types::errors::InternalError;
use houseflow_types::user;
use serde_json::Map;
use serde_json::Value;
use std::collections::HashMap;

#[tracing::instrument(name = "Query", skip(state), err)]
pub async fn handle(
    state: State,
    user_id: user::ID,
    payload: &request::Payload,
) -> Result<response::Payload, InternalError> {
    if let Some(homie_controller) = state.homie_controllers.get(&user_id) {
        let devices = get_homie_devices(&homie_controller.devices(), &payload.devices);
        Ok(response::Payload {
            error_code: None,
            debug_string: None,
            devices,
        })
    } else {
        Ok(response::Payload {
            error_code: Some("authFailure".to_string()),
            debug_string: Some("No such user".to_string()),
            devices: HashMap::new(),
        })
    }
}

fn get_homie_devices(
    devices: &HashMap<String, Device>,
    request_devices: &[request::PayloadDevice],
) -> HashMap<String, response::PayloadDevice> {
    request_devices
        .iter()
        .map(|device| {
            let response = get_homie_device(devices, device);
            (device.id.to_owned(), response)
        })
        .collect()
}

fn get_homie_device(
    devices: &HashMap<String, Device>,
    request_device: &request::PayloadDevice,
) -> response::PayloadDevice {
    if let Some((device, node)) = get_homie_device_by_id(devices, &request_device.id) {
        if device.state == homie_controller::State::Ready
            || device.state == homie_controller::State::Sleeping
        {
            let mut state = Map::new();

            if let Some(on) = node.properties.get("on") {
                if let Ok(value) = on.value() {
                    state.insert("on".to_string(), Value::Bool(value));
                }
            }
            if let Some(brightness) = node.properties.get("brightness") {
                if let Some(percentage) = property_value_to_percentage(brightness) {
                    state.insert("brightness".to_string(), Value::Number(percentage.into()));
                }
            }
            if let Some(color) = node.properties.get("color") {
                if let Some(color_value) = property_value_to_color(color) {
                    state.insert("color".to_string(), Value::Object(color_value));
                }
            }
            if let Some(temperature) = node.properties.get("temperature") {
                if let Some(finite_number) = property_value_to_number(temperature) {
                    state.insert(
                        "thermostatTemperatureAmbient".to_string(),
                        Value::Number(finite_number),
                    );
                }
            }
            if let Some(humidity) = node.properties.get("humidity") {
                if let Some(finite_number) = property_value_to_number(humidity) {
                    state.insert(
                        "thermostatHumidityAmbient".to_string(),
                        Value::Number(finite_number),
                    );
                }
            }

            response::PayloadDevice {
                state,
                status: response::PayloadDeviceStatus::Success,
                error_code: None,
            }
        } else {
            response::PayloadDevice {
                state: Default::default(),
                status: response::PayloadDeviceStatus::Offline,
                error_code: Some("offline".to_string()),
            }
        }
    } else {
        response::PayloadDevice {
            status: response::PayloadDeviceStatus::Error,
            state: Default::default(),
            error_code: Some("deviceNotFound".to_string()),
        }
    }
}
