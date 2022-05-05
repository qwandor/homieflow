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

use crate::{
    homegraph::{self, HomeGraphClient},
    json_prost::json_to_prost_struct,
    types::user::{self, Homie},
};
use google_smart_home::{
    device::commands::{ColorAbsolute, ColorValue},
    query::response::{self, Color},
};
use homie_controller::{
    ColorFormat, ColorHsv, ColorRgb, Datatype, Device, Event, HomieController, HomieEventLoop,
    Node, PollError, Property,
};
use prost_types::Struct;
use rumqttc::{ClientConfig, ConnectionError, MqttOptions, TlsConfiguration, Transport};
use serde_json::to_value;
use std::{collections::HashMap, ops::RangeInclusive, sync::Arc, time::Duration};
use tokio::{
    task::{self, JoinHandle},
    time::sleep,
};

const KEEP_ALIVE: Duration = Duration::from_secs(5);

pub fn get_mqtt_options(
    config: &Homie,
    tls_client_config: Option<Arc<ClientConfig>>,
) -> MqttOptions {
    let mut mqtt_options = MqttOptions::new(&config.client_id, &config.host, config.port);
    mqtt_options.set_keep_alive(KEEP_ALIVE);

    if let (Some(username), Some(password)) = (&config.username, &config.password) {
        mqtt_options.set_credentials(username, password);
    }

    if let Some(client_config) = tls_client_config {
        mqtt_options.set_transport(Transport::tls_with_config(TlsConfiguration::Rustls(
            client_config,
        )));
    }

    mqtt_options
}

pub fn spawn_homie_poller(
    controller: Arc<HomieController>,
    mut event_loop: HomieEventLoop,
    mut home_graph_client: Option<HomeGraphClient>,
    user_id: user::ID,
    reconnect_interval: Duration,
) -> JoinHandle<()> {
    task::spawn(async move {
        loop {
            match controller.poll(&mut event_loop).await {
                Ok(Some(event)) => {
                    handle_homie_event(controller.as_ref(), &mut home_graph_client, user_id, event)
                        .await;
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::error!(
                        "Failed to poll HomieController for base topic '{}': {}",
                        controller.base_topic(),
                        e
                    );
                    if let PollError::Connection(ConnectionError::Io(_)) = e {
                        sleep(reconnect_interval).await;
                    }
                }
            }
        }
    })
}

async fn handle_homie_event(
    controller: &HomieController,
    home_graph_client: &mut Option<HomeGraphClient>,
    user_id: user::ID,
    event: Event,
) {
    match event {
        Event::PropertyValueChanged {
            ref device_id,
            ref node_id,
            property_id: _,
            value: _,
            fresh: true,
        } => {
            if let Some(home_graph_client) = home_graph_client {
                node_state_changed(controller, home_graph_client, user_id, device_id, node_id)
                    .await;
            }
        }
        _ => tracing::trace!("Homie event {:?}", event),
    }
}

async fn node_state_changed(
    controller: &HomieController,
    home_graph_client: &mut HomeGraphClient,
    user_id: user::ID,
    device_id: &str,
    node_id: &str,
) {
    if let Some(node) = get_homie_node(&controller.devices(), device_id, node_id) {
        let state = query_state_to_report_state(homie_node_to_state(node));

        if !state.fields.is_empty() {
            if let Err(e) = homegraph::report_state(
                home_graph_client,
                user_id,
                format!("{}/{}", device_id, node_id),
                state.clone(),
            )
            .await
            {
                tracing::error!(
                    "Error reporting state of {}/{} {:?}: {:?}",
                    device_id,
                    node_id,
                    state,
                    e,
                );
            }
        }
    }
}

fn query_state_to_report_state(state: response::State) -> Struct {
    if let Ok(serde_json::Value::Object(state_map)) = to_value(state) {
        json_to_prost_struct(state_map)
    } else {
        panic!("Failed to convert state to map.");
    }
}

pub fn homie_node_to_state(node: &Node) -> response::State {
    let mut state = response::State {
        online: true,
        ..Default::default()
    };

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

    state
}

/// Given a Homie device and node ID, looks up the corresponding Homie node (if any).
fn get_homie_node<'a>(
    devices: &'a HashMap<String, Device>,
    device_id: &str,
    node_id: &str,
) -> Option<&'a Node> {
    devices.get(device_id)?.nodes.get(node_id)
}

/// Scales the value of the given property to a percentage.
pub fn property_value_to_percentage(property: &Property) -> Option<u8> {
    match property.datatype? {
        Datatype::Integer => {
            let value: i64 = property.value().ok()?;
            let range: RangeInclusive<i64> = property.range().ok()?;
            let percentage = (value - range.start()) * 100 / (range.end() - range.start());
            let percentage = cap(percentage, 0, 100);
            Some(percentage as u8)
        }
        Datatype::Float => {
            let value: f64 = property.value().ok()?;
            let range: RangeInclusive<f64> = property.range().ok()?;
            let percentage = (value - range.start()) * 100.0 / (range.end() - range.start());
            let percentage = cap(percentage, 0.0, 100.0);
            Some(percentage as u8)
        }
        _ => None,
    }
}

/// Converts a percentage to the appropriately scaled property value of the given property, if it has
/// a range specified.
pub fn percentage_to_property_value(property: &Property, percentage: u8) -> Option<String> {
    match property.datatype? {
        Datatype::Integer => {
            let range: RangeInclusive<i64> = property.range().ok()?;
            let value = range.start() + percentage as i64 * (range.end() - range.start()) / 100;
            Some(format!("{}", value))
        }
        Datatype::Float => {
            let range: RangeInclusive<f64> = property.range().ok()?;
            let value = range.start() + percentage as f64 * (range.end() - range.start()) / 100.0;
            Some(format!("{}", value))
        }
        _ => None,
    }
}

/// Converts the property value to a JSON number if it is an appropriate type.
pub fn property_value_to_number(property: &Property) -> Option<f64> {
    match property.datatype? {
        Datatype::Integer => {
            let value: i64 = property.value().ok()?;
            Some(value as f64)
        }
        Datatype::Float => {
            let value = property.value().ok()?;
            Some(value)
        }
        _ => None,
    }
}

/// Converts the value of the given property to a Google Home JSON color value, if it is the
/// appropriate type.
pub fn property_value_to_color(property: &Property) -> Option<Color> {
    let color_format = property.color_format().ok()?;
    let color_value = match color_format {
        ColorFormat::Rgb => {
            let rgb: ColorRgb = property.value().ok()?;
            let rgb_int = ((rgb.r as u32) << 16) + ((rgb.g as u32) << 8) + (rgb.b as u32);
            Color::SpectrumRgb(rgb_int)
        }
        ColorFormat::Hsv => {
            let hsv: ColorHsv = property.value().ok()?;
            Color::SpectrumHsv {
                hue: hsv.h.into(),
                saturation: hsv.s as f64 / 100.0,
                value: hsv.v as f64 / 100.0,
            }
        }
    };
    Some(color_value)
}

/// Converts a Google Home `ColorAbsolute` command to the appropriate value to set on the given
/// Homie property, if it is the appropriate format.
pub fn color_absolute_to_property_value(
    property: &Property,
    color_absolute: &ColorAbsolute,
) -> Option<String> {
    let color_format = property.color_format().ok()?;
    match color_format {
        ColorFormat::Rgb => {
            if let ColorValue::Rgb { spectrum_rgb } = color_absolute.color.value {
                let rgb = ColorRgb::new(
                    (spectrum_rgb >> 16) as u8,
                    (spectrum_rgb >> 8) as u8,
                    spectrum_rgb as u8,
                );
                return Some(rgb.to_string());
            }
        }
        ColorFormat::Hsv => {
            if let ColorValue::Hsv { spectrum_hsv } = &color_absolute.color.value {
                let hsv = ColorHsv::new(
                    spectrum_hsv.hue as u16,
                    (spectrum_hsv.saturation * 100.0) as u8,
                    (spectrum_hsv.value * 100.0) as u8,
                );
                return Some(hsv.to_string());
            }
        }
    }
    None
}

fn cap<N: Copy + PartialOrd>(value: N, min: N, max: N) -> N {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use google_smart_home::{
        device::commands::{Color, Hsv},
        query,
    };
    use prost_types::{value::Kind, Value};
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn convert_state() {
        let state = response::State {
            online: true,
            on: Some(true),
            brightness: Some(65),
            thermostat_temperature_ambient: Some(22.0),
            thermostat_humidity_ambient: Some(42.0),
            ..Default::default()
        };

        let mut map = BTreeMap::new();
        map.insert(
            "online".to_string(),
            Value {
                kind: Some(Kind::BoolValue(true)),
            },
        );
        map.insert(
            "on".to_string(),
            Value {
                kind: Some(Kind::BoolValue(true)),
            },
        );
        map.insert(
            "brightness".to_string(),
            Value {
                kind: Some(Kind::NumberValue(65.0)),
            },
        );
        map.insert(
            "thermostatTemperatureAmbient".to_string(),
            Value {
                kind: Some(Kind::NumberValue(22.0)),
            },
        );
        map.insert(
            "thermostatHumidityAmbient".to_string(),
            Value {
                kind: Some(Kind::NumberValue(42.0)),
            },
        );

        assert_eq!(query_state_to_report_state(state).fields, map);
    }

    #[test]
    fn percentage_integer() {
        let property = Property {
            id: "brightness".to_string(),
            name: Some("Brightness".to_string()),
            datatype: Some(Datatype::Integer),
            settable: true,
            retained: true,
            unit: None,
            format: Some("10:20".to_string()),
            value: Some("13".to_string()),
        };

        assert_eq!(property_value_to_percentage(&property), Some(30));
        assert_eq!(
            percentage_to_property_value(&property, 70),
            Some("17".to_string())
        );
    }

    #[test]
    fn percentage_float() {
        let property = Property {
            id: "brightness".to_string(),
            name: Some("Brightness".to_string()),
            datatype: Some(Datatype::Float),
            settable: true,
            retained: true,
            unit: None,
            format: Some("1.0:2.0".to_string()),
            value: Some("1.3".to_string()),
        };

        assert_eq!(property_value_to_percentage(&property), Some(30));
        assert_eq!(
            percentage_to_property_value(&property, 70),
            Some("1.7".to_string())
        );
    }

    #[test]
    fn number_integer() {
        let property = Property {
            id: "number".to_string(),
            name: Some("Number".to_string()),
            datatype: Some(Datatype::Integer),
            settable: true,
            retained: true,
            unit: None,
            format: None,
            value: Some("42".to_string()),
        };

        assert_eq!(property_value_to_number(&property), Some(42.0));
    }

    #[test]
    fn number_float() {
        let property = Property {
            id: "number".to_string(),
            name: Some("Number".to_string()),
            datatype: Some(Datatype::Float),
            settable: true,
            retained: true,
            unit: None,
            format: None,
            value: Some("42.2".to_string()),
        };

        assert_eq!(property_value_to_number(&property), Some(42.2));
    }

    #[test]
    fn color_rgb() {
        let property = Property {
            id: "color".to_string(),
            name: Some("Colour".to_string()),
            datatype: Some(Datatype::Color),
            settable: true,
            retained: true,
            unit: None,
            format: Some("rgb".to_string()),
            value: Some("17,34,51".to_string()),
        };

        assert_eq!(
            property_value_to_color(&property),
            Some(query::response::Color::SpectrumRgb(0x112233))
        );
        assert_eq!(
            color_absolute_to_property_value(
                &property,
                &ColorAbsolute {
                    color: Color {
                        name: None,
                        value: ColorValue::Rgb {
                            spectrum_rgb: 0x445566
                        }
                    }
                }
            ),
            Some("68,85,102".to_string())
        );
    }

    #[test]
    fn color_hsv() {
        let property = Property {
            id: "color".to_string(),
            name: Some("Colour".to_string()),
            datatype: Some(Datatype::Color),
            settable: true,
            retained: true,
            unit: None,
            format: Some("hsv".to_string()),
            value: Some("280,50,60".to_string()),
        };

        assert_eq!(
            property_value_to_color(&property),
            Some(query::response::Color::SpectrumHsv {
                hue: 280.0,
                saturation: 0.5,
                value: 0.6
            })
        );
        assert_eq!(
            color_absolute_to_property_value(
                &property,
                &ColorAbsolute {
                    color: Color {
                        name: None,
                        value: ColorValue::Hsv {
                            spectrum_hsv: Hsv {
                                hue: 290.0,
                                saturation: 0.2,
                                value: 0.3
                            }
                        }
                    }
                }
            ),
            Some("290,20,30".to_string())
        );
    }
}
