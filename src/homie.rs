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
    types::user::{self, Homie},
};
use homie_controller::{
    Datatype, Device, Event, HomieController, HomieEventLoop, Node, PollError, Property,
};
use prost_types::{value::Kind, Value};
use rumqttc::{ClientConfig, ConnectionError, MqttOptions, TlsConfiguration, Transport};
use std::{
    collections::{BTreeMap, HashMap},
    ops::RangeInclusive,
    sync::Arc,
    time::Duration,
};
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
        let state = homie_node_to_state(node);

        if !state.is_empty() {
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

fn homie_node_to_state(node: &Node) -> BTreeMap<String, Value> {
    let mut state = BTreeMap::new();

    if let Some(on) = node.properties.get("on") {
        if let Ok(on) = on.value() {
            state.insert(
                "on".to_string(),
                Value {
                    kind: Some(Kind::BoolValue(on)),
                },
            );
        }
    }
    if let Some(brightness) = node.properties.get("brightness") {
        if let Some(brightness) = property_value_to_percentage(brightness) {
            state.insert(
                "brightness".to_string(),
                Value {
                    kind: Some(Kind::NumberValue(brightness.into())),
                },
            );
        }
    }
    if let Some(temperature) = node.properties.get("temperature") {
        if let Some(temperature) = property_value_to_number(temperature) {
            state.insert(
                "thermostatTemperatureAmbient".to_string(),
                Value {
                    kind: Some(Kind::NumberValue(temperature)),
                },
            );
        }
    }
    if let Some(humidity) = node.properties.get("humidity") {
        if let Some(humidity) = property_value_to_number(humidity) {
            state.insert(
                "thermostatHumidityAmbient".to_string(),
                Value {
                    kind: Some(Kind::NumberValue(humidity)),
                },
            );
        }
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
    use super::*;

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
}
