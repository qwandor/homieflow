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

use crate::{json_prost::json_to_prost_struct, types::user};
use google_api_proto::google::home::graph::v1::{
    home_graph_api_service_client::HomeGraphApiServiceClient, ReportStateAndNotificationDevice,
    ReportStateAndNotificationRequest, RequestSyncDevicesRequest, StateAndNotificationPayload,
};
use google_authz::{Credentials, GoogleAuthz};
use google_smart_home::query::response;
use prost_types::{value::Kind, Struct, Value};
use serde_json::to_value;
use std::{collections::BTreeMap, error::Error, path::Path, sync::Arc};
use tokio::sync::Mutex;
use tonic::{transport::Channel, Status};

#[derive(Clone, Debug)]
pub struct HomeGraphClient(Arc<Mutex<HomeGraphApiServiceClient<GoogleAuthz<Channel>>>>);

impl HomeGraphClient {
    /// Connects to the Google Home Graph gRPC API server and returns a client which can make calls to
    /// the API.
    pub async fn connect(credentials_file: &Path) -> Result<Self, Box<dyn Error>> {
        let channel = Channel::from_static("https://homegraph.googleapis.com")
            .connect()
            .await?;
        let credentials = Credentials::builder()
            .json_file(credentials_file)
            .scopes(&["https://www.googleapis.com/auth/homegraph"])
            .build()
            .await?;
        let channel = GoogleAuthz::builder(channel)
            .credentials(credentials)
            .build()
            .await;
        Ok(Self(Arc::new(Mutex::new(HomeGraphApiServiceClient::new(
            channel,
        )))))
    }

    /// Reports state of the single device with the given ID for the given user.
    pub async fn report_state(
        &self,
        user_id: user::ID,
        device_id: String,
        state: response::State,
    ) -> Result<(), Status> {
        let mut fields = BTreeMap::new();
        fields.insert(
            device_id,
            Value {
                kind: Some(Kind::StructValue(query_state_to_report_state(state))),
            },
        );
        let request = ReportStateAndNotificationRequest {
            agent_user_id: user_id.to_string(),
            payload: Some(StateAndNotificationPayload {
                devices: Some(ReportStateAndNotificationDevice {
                    states: Some(Struct { fields }),
                    notifications: None,
                }),
            }),
            ..Default::default()
        };
        self.0
            .lock()
            .await
            .report_state_and_notification(request)
            .await?;

        Ok(())
    }

    /// Requests that Google make a SYNC intent, because devices have been added, removed or changed.
    pub async fn request_sync(&self, user_id: user::ID) -> Result<(), Status> {
        let request = RequestSyncDevicesRequest {
            agent_user_id: user_id.to_string(),
            r#async: false,
        };
        self.0.lock().await.request_sync_devices(request).await?;

        Ok(())
    }
}

fn query_state_to_report_state(state: response::State) -> Struct {
    if let Ok(serde_json::Value::Object(state_map)) = to_value(state) {
        json_to_prost_struct(state_map)
    } else {
        panic!("Failed to convert state to map.");
    }
}

#[cfg(test)]
mod tests {
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
}
