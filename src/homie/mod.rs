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

pub mod state;

use self::state::homie_node_to_state;
use crate::{
    homegraph::HomeGraphClient,
    types::user::{self, Homie},
};
use homie_controller::{Device, Event, HomieController, HomieEventLoop, Node, PollError};
use rumqttc::{ClientConfig, ConnectionError, MqttOptions, TlsConfiguration, Transport};
use std::{collections::HashMap, sync::Arc, time::Duration};
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

        if let Err(e) = home_graph_client
            .report_state(user_id, format!("{}/{}", device_id, node_id), state.clone())
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

/// Given a Homie device and node ID, looks up the corresponding Homie node (if any).
fn get_homie_node<'a>(
    devices: &'a HashMap<String, Device>,
    device_id: &str,
    node_id: &str,
) -> Option<&'a Node> {
    devices.get(device_id)?.nodes.get(node_id)
}
