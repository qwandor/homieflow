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
    ratelimit::RateLimiter,
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
/// The minimum time between two calls to request sync.
const REQUEST_SYNC_RATE_LIMIT: Duration = Duration::from_secs(10);

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
    event_loop: HomieEventLoop,
    home_graph_client: Option<HomeGraphClient>,
    user_id: user::ID,
    reconnect_interval: Duration,
) -> JoinHandle<()> {
    task::spawn(homie_poller(
        controller,
        event_loop,
        home_graph_client,
        user_id,
        reconnect_interval,
    ))
}

async fn homie_poller(
    controller: Arc<HomieController>,
    mut event_loop: HomieEventLoop,
    mut home_graph_client: Option<HomeGraphClient>,
    user_id: user::ID,
    reconnect_interval: Duration,
) {
    let home_graph_client_clone = home_graph_client.clone();
    let request_sync = RateLimiter::new(REQUEST_SYNC_RATE_LIMIT, move || {
        Box::pin(request_sync(user_id, home_graph_client_clone.clone()))
    });

    loop {
        match controller.poll(&mut event_loop).await {
            Ok(Some(event)) => {
                handle_homie_event(
                    controller.as_ref(),
                    &request_sync,
                    &mut home_graph_client,
                    user_id,
                    event,
                )
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
}

async fn handle_homie_event(
    controller: &HomieController,
    request_sync: &RateLimiter,
    home_graph_client: &mut Option<HomeGraphClient>,
    user_id: user::ID,
    event: Event,
) {
    match event {
        Event::DeviceUpdated {
            device_id: _,
            has_required_attributes: true,
        }
        | Event::NodeUpdated {
            device_id: _,
            node_id: _,
            has_required_attributes: true,
        }
        | Event::PropertyUpdated {
            device_id: _,
            node_id: _,
            property_id: _,
            has_required_attributes: true,
        } => {
            tracing::trace!("Homie event {:?}, requesting sync.", event);
            request_sync.execute();
        }
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

async fn request_sync(user_id: user::ID, home_graph_client: Option<HomeGraphClient>) {
    if let Some(home_graph_client) = home_graph_client {
        if let Err(e) = home_graph_client.request_sync(user_id).await {
            tracing::error!("Error requesting sync for {}: {:?}", user_id, e);
        }
    }
}

async fn node_state_changed(
    controller: &HomieController,
    home_graph_client: &mut HomeGraphClient,
    user_id: user::ID,
    device_id: &str,
    node_id: &str,
) {
    if let Some((device, node)) = get_homie_node(&controller.devices(), device_id, node_id) {
        let online = device.state == homie_controller::State::Ready
            || device.state == homie_controller::State::Sleeping;
        let state = homie_node_to_state(node, online);

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
pub fn get_homie_node<'a>(
    devices: &'a HashMap<String, Device>,
    device_id: &str,
    node_id: &str,
) -> Option<(&'a Device, &'a Node)> {
    if let Some(device) = devices.get(device_id) {
        if let Some(node) = device.nodes.get(node_id) {
            return Some((device, node));
        }
    }
    None
}
