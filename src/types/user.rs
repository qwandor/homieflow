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

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use std::time::Duration;
use uuid::Uuid;

pub type ID = Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// Unique ID of the user
    pub id: ID,
    /// Email of the user
    pub email: String,
    /// Homie controller for the user.
    #[serde(default)]
    pub homie: Option<Homie>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Homie {
    /// The hostname of the MQTT broker.
    pub host: String,
    /// The port of the MQTT broker.
    pub port: u16,
    /// Whether to use TLS for the MQTT broker connection.
    #[serde(default)]
    pub use_tls: bool,
    /// The username with which to authenticate to the MQTT broker, if any.
    #[serde(default)]
    pub username: Option<String>,
    /// The password with which to authenticate to the MQTT broker, if any.
    #[serde(default)]
    pub password: Option<String>,
    /// The client ID to use for the MQTT connection.
    pub client_id: String,
    /// The Homie base MQTT topic.
    #[serde(default = "default_homie_prefix")]
    pub homie_prefix: String,
    #[serde(
        deserialize_with = "de_duration_seconds",
        rename = "reconnect-interval-seconds"
    )]
    pub reconnect_interval: Duration,
}

fn default_homie_prefix() -> String {
    "homie".to_string()
}

/// Deserialize an integer as a number of seconds.
fn de_duration_seconds<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
    let seconds = u64::deserialize(d)?;
    Ok(Duration::from_secs(seconds))
}
