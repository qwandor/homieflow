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

use crate::types::user;
use google_api_proto::google::home::graph::v1::{
    home_graph_api_service_client::HomeGraphApiServiceClient, ReportStateAndNotificationDevice,
    ReportStateAndNotificationRequest, StateAndNotificationPayload,
};
use google_authz::{Credentials, GoogleAuthz};
use prost_types::{value::Kind, Struct, Value};
use std::{collections::BTreeMap, error::Error, path::Path};
use tonic::transport::Channel;

pub type HomeGraphClient = HomeGraphApiServiceClient<GoogleAuthz<Channel>>;

/// Connects to the Google Home Graph gRPC API server and returns a client which can make calls to
/// the API.
pub async fn connect(credentials_file: &Path) -> Result<HomeGraphClient, Box<dyn Error>> {
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
    Ok(HomeGraphApiServiceClient::new(channel))
}

/// Reports state of the single device with the given ID for the given user.
pub async fn report_state(
    client: &mut HomeGraphClient,
    user_id: user::ID,
    device_id: String,
    state: BTreeMap<String, Value>,
) -> Result<(), Box<dyn Error>> {
    let mut fields = BTreeMap::new();
    fields.insert(
        device_id,
        Value {
            kind: Some(Kind::StructValue(Struct { fields: state })),
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
    client.report_state_and_notification(request).await?;

    Ok(())
}
