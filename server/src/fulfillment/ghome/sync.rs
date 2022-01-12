use std::collections::HashMap;

use crate::types::errors::ServerError;
use crate::types::user;
use crate::State;
use google_smart_home::device::Trait as GHomeDeviceTrait;
use google_smart_home::device::Type as GHomeDeviceType;
use google_smart_home::sync::response;
use google_smart_home::sync::response::PayloadDevice;
use homie_controller::ColorFormat;
use homie_controller::Device;
use homie_controller::Node;
use serde_json::Map;
use serde_json::Value;

#[tracing::instrument(name = "Sync", skip(state), err)]
pub async fn handle(state: State, user_id: user::ID) -> Result<response::Payload, ServerError> {
    if let Some(homie_controller) = state.homie_controllers.get(&user_id) {
        let devices = homie_devices_to_google_home(&homie_controller.devices());

        tracing::info!(
            "Synced {} devices: {}",
            devices.len(),
            serde_json::to_string_pretty(&devices).unwrap(),
        );

        Ok(response::Payload {
            agent_user_id: user_id.to_string(),
            error_code: None,
            debug_string: None,
            devices,
        })
    } else {
        Ok(response::Payload {
            agent_user_id: user_id.to_string(),
            error_code: Some("authFailure".to_string()),
            debug_string: Some("No such user".to_string()),
            devices: vec![],
        })
    }
}

fn homie_devices_to_google_home(devices: &HashMap<String, Device>) -> Vec<PayloadDevice> {
    let mut google_home_devices = vec![];
    for device in devices.values() {
        if device.state == homie_controller::State::Ready
            || device.state == homie_controller::State::Sleeping
        {
            for node in device.nodes.values() {
                if let Some(google_home_device) = homie_node_to_google_home(device, node) {
                    google_home_devices.push(google_home_device);
                }
            }
        }
    }
    google_home_devices
}

fn homie_node_to_google_home(device: &Device, node: &Node) -> Option<PayloadDevice> {
    let id = format!("{}/{}", device.id, node.id);
    let mut traits = vec![];
    let mut attributes = Map::new();
    let mut device_type = None;
    if node.properties.contains_key("on") {
        device_type = Some(GHomeDeviceType::Switch);
        traits.push(GHomeDeviceTrait::OnOff);
    }
    if node.properties.contains_key("brightness") {
        if node.properties.contains_key("on") {
            device_type = Some(GHomeDeviceType::Light);
        }
        traits.push(GHomeDeviceTrait::Brightness);
    }
    if let Some(color) = node.properties.get("color") {
        if let Ok(color_format) = color.color_format() {
            let color_model = match color_format {
                ColorFormat::Rgb => "rgb",
                ColorFormat::Hsv => "hsv",
            };
            device_type = Some(GHomeDeviceType::Light);
            traits.push(GHomeDeviceTrait::ColorSetting);
            attributes.insert(
                "colorModel".to_string(),
                Value::String(color_model.to_owned()),
            );
        }
    }
    if node.properties.contains_key("temperature") {
        device_type = Some(GHomeDeviceType::Thermostat);
        traits.push(GHomeDeviceTrait::TemperatureSetting);
        attributes.insert(
            "availableThermostatModes".to_string(),
            Value::Array(vec![Value::String("off".to_string())]),
        );
        attributes.insert(
            "thermostatTemperatureUnit".to_string(),
            Value::String("C".to_string()),
        );
        attributes.insert("queryOnlyTemperatureSetting".to_string(), Value::Bool(true));
    }

    let device_name = device.name.clone().unwrap_or_else(|| device.id.clone());
    let node_name = node.name.clone().unwrap_or_else(|| node.id.clone());
    Some(response::PayloadDevice {
        id,
        device_type: device_type?,
        traits,
        name: response::PayloadDeviceName {
            default_names: None,
            name: format!("{} {}", device_name, node_name),
            nicknames: Some(vec![node_name]),
        },
        device_info: None,
        will_report_state: false,
        notification_supported_by_agent: false,
        room_hint: None,
        attributes,
        custom_data: None,
        other_device_ids: None,
    })
}
