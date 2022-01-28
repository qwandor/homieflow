use super::homie::get_homie_device_by_id;
use super::homie::property_value_to_color;
use super::homie::property_value_to_number;
use super::homie::property_value_to_percentage;
use crate::types::errors::InternalError;
use crate::types::user;
use crate::State;
use google_smart_home::query::request;
use google_smart_home::query::response;
use homie_controller::Device;
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
            let mut state = response::State::default();
            state.online = true;

            if let Some(on) = node.properties.get("on") {
                state.on = on.value().ok();
            }
            if let Some(brightness) = node.properties.get("brightness") {
                state.brightness = property_value_to_percentage(brightness);
            }
            if let Some(color) = node.properties.get("color") {
                state.color = property_value_to_color(color);
            }
            if let Some(temperature) = node.properties.get("temperature") {
                state.thermostat_temperature_ambient = property_value_to_number(temperature);
            }
            if let Some(humidity) = node.properties.get("humidity") {
                state.thermostat_humidity_ambient = property_value_to_number(humidity);
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

#[cfg(test)]
mod tests {
    use super::*;

    use google_smart_home::query::response::Color;
    use homie_controller::{Datatype, Node, Property, State};

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
        let devices = device_set(vec![device]);

        let request_device = request::PayloadDevice {
            id: "device/node".to_string(),
            custom_data: None,
        };

        assert_eq!(
            get_homie_device(&devices, &request_device),
            response::PayloadDevice {
                status: response::PayloadDeviceStatus::Success,
                error_code: None,
                state: response::State {
                    online: true,
                    on: Some(true),
                    brightness: Some(100),
                    ..Default::default()
                },
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
        let devices = device_set(vec![device]);

        let request_device = request::PayloadDevice {
            id: "device/node".to_string(),
            custom_data: None,
        };

        assert_eq!(
            get_homie_device(&devices, &request_device),
            response::PayloadDevice {
                status: response::PayloadDeviceStatus::Success,
                error_code: None,
                state: response::State {
                    online: true,
                    on: Some(true),
                    color: Some(Color::SpectrumRgb(0xffff00)),
                    ..Default::default()
                },
            }
        );
    }

    #[test]
    fn temperature_sensor() {
        let temperature_property = Property {
            id: "temperature".to_string(),
            name: Some("Temperature".to_string()),
            datatype: Some(Datatype::Float),
            settable: true,
            retained: true,
            unit: Some("°C".to_string()),
            format: None,
            value: Some("21.3".to_string()),
        };
        let humidity_property = Property {
            id: "humidity".to_string(),
            name: Some("Humidity".to_string()),
            datatype: Some(Datatype::Integer),
            settable: true,
            retained: true,
            unit: Some("%".to_string()),
            format: Some("0:100".to_string()),
            value: Some("27".to_string()),
        };
        let node = Node {
            id: "node".to_string(),
            name: Some("Node name".to_string()),
            node_type: None,
            properties: property_set(vec![temperature_property, humidity_property]),
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
        let devices = device_set(vec![device]);

        let request_device = request::PayloadDevice {
            id: "device/node".to_string(),
            custom_data: None,
        };

        assert_eq!(
            get_homie_device(&devices, &request_device),
            response::PayloadDevice {
                status: response::PayloadDeviceStatus::Success,
                error_code: None,
                state: response::State {
                    online: true,
                    thermostat_temperature_ambient: Some(21.3),
                    thermostat_humidity_ambient: Some(27.0),
                    ..Default::default()
                },
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

    fn device_set(devices: Vec<Device>) -> HashMap<String, Device> {
        devices
            .into_iter()
            .map(|device| (device.id.clone(), device))
            .collect()
    }
}
