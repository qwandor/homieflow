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

#[cfg(test)]
mod tests {
    use super::*;

    use homie_controller::{Datatype, Property, State};
    use serde_json::json;

    #[test]
    fn light_with_brightness() {
        let on_property = Property {
            id: "on".to_string(),
            name: Some("On".to_string()),
            datatype: Some(Datatype::Boolean),
            settable: true,
            retained: true,
            unit: None,
            format: None,
            value: Some("true".to_string()),
        };
        let brightness_property = Property {
            id: "brightness".to_string(),
            name: Some("Brightness".to_string()),
            datatype: Some(Datatype::Integer),
            settable: true,
            retained: true,
            unit: None,
            format: Some("0:100".to_string()),
            value: Some("100".to_string()),
        };
        let node = Node {
            id: "node".to_string(),
            name: Some("Node name".to_string()),
            node_type: None,
            properties: property_set(vec![on_property, brightness_property]),
        };
        let device = Device {
            id: "device".to_string(),
            homie_version: "4.0".to_string(),
            name: Some("Device name".to_string()),
            state: State::Ready,
            implementation: None,
            nodes: node_set(vec![node]),
            extensions: vec![],
            local_ip: None,
            mac: None,
            firmware_name: None,
            firmware_version: None,
            stats_interval: None,
            stats_uptime: None,
            stats_signal: None,
            stats_cputemp: None,
            stats_cpuload: None,
            stats_battery: None,
            stats_freeheap: None,
            stats_supply: None,
        };

        assert_eq!(
            homie_node_to_google_home(&device, &device.nodes.get("node").unwrap()).unwrap(),
            PayloadDevice {
                id: "device/node".to_string(),
                device_type: GHomeDeviceType::Light,
                traits: vec![GHomeDeviceTrait::OnOff, GHomeDeviceTrait::Brightness],
                name: response::PayloadDeviceName {
                    default_names: None,
                    name: "Device name Node name".to_string(),
                    nicknames: Some(vec!["Node name".to_string()])
                },
                will_report_state: false,
                notification_supported_by_agent: false,
                room_hint: None,
                device_info: None,
                attributes: json!({}).as_object().unwrap().to_owned(),
                custom_data: None,
                other_device_ids: None,
            }
        );
    }

    #[test]
    fn light_with_color() {
        let on_property = Property {
            id: "on".to_string(),
            name: Some("On".to_string()),
            datatype: Some(Datatype::Boolean),
            settable: true,
            retained: true,
            unit: None,
            format: None,
            value: Some("true".to_string()),
        };
        let color_property = Property {
            id: "color".to_string(),
            name: Some("Colour".to_string()),
            datatype: Some(Datatype::Color),
            settable: true,
            retained: true,
            unit: None,
            format: Some("rgb".to_string()),
            value: Some("255,255,0".to_string()),
        };
        let node = Node {
            id: "node".to_string(),
            name: Some("Node name".to_string()),
            node_type: None,
            properties: property_set(vec![on_property, color_property]),
        };
        let device = Device {
            id: "device".to_string(),
            homie_version: "4.0".to_string(),
            name: Some("Device name".to_string()),
            state: State::Ready,
            implementation: None,
            nodes: node_set(vec![node]),
            extensions: vec![],
            local_ip: None,
            mac: None,
            firmware_name: None,
            firmware_version: None,
            stats_interval: None,
            stats_uptime: None,
            stats_signal: None,
            stats_cputemp: None,
            stats_cpuload: None,
            stats_battery: None,
            stats_freeheap: None,
            stats_supply: None,
        };

        assert_eq!(
            homie_node_to_google_home(&device, &device.nodes.get("node").unwrap()).unwrap(),
            PayloadDevice {
                id: "device/node".to_string(),
                device_type: GHomeDeviceType::Light,
                traits: vec![GHomeDeviceTrait::OnOff, GHomeDeviceTrait::ColorSetting],
                name: response::PayloadDeviceName {
                    default_names: None,
                    name: "Device name Node name".to_string(),
                    nicknames: Some(vec!["Node name".to_string()])
                },
                will_report_state: false,
                notification_supported_by_agent: false,
                room_hint: None,
                device_info: None,
                attributes: json!({"colorModel": "rgb"}).as_object().unwrap().to_owned(),
                custom_data: None,
                other_device_ids: None,
            }
        );
    }

    fn property_set(properties: Vec<Property>) -> HashMap<String, Property> {
        properties
            .into_iter()
            .map(|property| (property.id.clone(), property))
            .collect()
    }

    fn node_set(nodes: Vec<Node>) -> HashMap<String, Node> {
        nodes
            .into_iter()
            .map(|node| (node.id.clone(), node))
            .collect()
    }
}
