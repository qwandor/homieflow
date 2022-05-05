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

use crate::types::user::Homie;
use homie_controller::{Event, HomieController, HomieEventLoop, PollError, };
use rumqttc::{ClientConfig, ConnectionError, MqttOptions, TlsConfiguration, Transport};
use std::{sync::Arc, time::Duration};
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
    reconnect_interval: Duration,
) -> JoinHandle<()> {
    task::spawn(async move {
        loop {
            match controller.poll(&mut event_loop).await {
                Ok(Some(event)) => {
                    handle_homie_event(controller.as_ref(), event).await;
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

async fn handle_homie_event(_controller: &HomieController, event: Event) {
    match event {
        Event::PropertyValueChanged { .. } => {}
        _ => tracing::trace!("Homie event {:?}", event),
    }
}
