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

use google_api_proto::google::home::graph::v1::home_graph_api_service_client::HomeGraphApiServiceClient;
use google_authz::{Credentials, GoogleAuthz};
use std::{error::Error, path::Path};
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
