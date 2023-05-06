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

use super::defaults;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

use crate::types::permission;
use crate::types::room;
use crate::types::structure;
use crate::types::user;

use permission::Permission;
use room::Room;
use structure::Structure;
use url::Url;
use user::User;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Network configuration
    #[serde(default)]
    pub network: Network,
    /// Secret data
    pub secrets: Secrets,
    /// Path to the TLS configuration
    #[serde(default)]
    pub tls: Option<Tls>,
    /// Configuration of the Google 3rd party client
    #[serde(default)]
    pub google: Option<Google>,
    /// Configuration for login options
    #[serde(default)]
    pub logins: Logins,
    /// Structures
    #[serde(default)]
    pub structures: Vec<Structure>,
    /// Rooms
    #[serde(default)]
    pub rooms: Vec<Room>,
    /// Users
    #[serde(default)]
    pub users: Vec<User>,
    /// User -> Structure permission
    #[serde(default)]
    pub permissions: Vec<Permission>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Network {
    /// Server address
    #[serde(default = "defaults::server_listen_address")]
    pub address: std::net::IpAddr,
    /// Server port
    #[serde(default = "defaults::server_port")]
    pub port: u16,
    /// Base public URL of server, if different to the listen address and port.
    #[serde(default)]
    pub base_url: Option<Url>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Secrets {
    /// Key used to sign refresh tokens. Must be secret and should be fairly random.
    pub refresh_key: String,
    /// Key used to sign access tokens. Must be secret and should be fairly random.
    pub access_key: String,
    /// Key used to sign authorization codes. Must be secret and should be fairly random.
    pub authorization_code_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Tls {
    /// Server address
    #[serde(default = "defaults::server_listen_address")]
    pub address: std::net::IpAddr,
    /// Server port
    #[serde(default = "defaults::server_port")]
    pub port: u16,
    /// Path to the TLS certificate
    pub certificate: PathBuf,
    /// Path to the TLS private key
    pub private_key: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Google {
    /// OAuth2 Client ID identifying Google to your service
    pub client_id: String,
    /// OAuth2 Client Secret assigned to the Client ID which identifies Google to you
    pub client_secret: String,
    /// Google Project ID
    pub project_id: String,
    /// Credentials JSON file for Report State API.
    pub credentials_file: PathBuf,
    /// The minimum time between two calls to request sync.
    pub request_sync_rate_limit_seconds: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Logins {
    /// Configuration for Google login.
    pub google: Option<GoogleLogin>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GoogleLogin {
    /// OAuth2 Client ID identifying your service to Google.
    pub client_id: String,
}

impl super::Config for Config {
    const DEFAULT_TOML: &'static str = include_str!("../../default.toml");

    const DEFAULT_FILE: &'static str = "server.toml";

    fn validate(&self) -> Result<(), String> {
        for room in &self.rooms {
            if !self
                .structures
                .iter()
                .any(|structure| structure.id == room.structure_id)
            {
                return Err(format!(
                    "Couldn't find structure with id: {} for room: {}",
                    room.structure_id, room.id
                ));
            }
        }

        for permission in &self.permissions {
            if !self
                .structures
                .iter()
                .any(|structure| structure.id == permission.structure_id)
            {
                return Err(format!(
                    "Couldn't find structure with id: {} for permission: {:?}",
                    permission.structure_id, permission
                ));
            }
            if !self.users.iter().any(|user| user.id == permission.user_id) {
                return Err(format!(
                    "Couldn't find user with id: {} for permission: {:?}",
                    permission.user_id, permission
                ));
            }
        }

        Ok(())
    }
}

impl rand::distributions::Distribution<Secrets> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Secrets {
        let mut gen_secret = || {
            let mut bytes = [0; 32];
            rng.fill_bytes(&mut bytes);
            hex::encode(bytes)
        };
        Secrets {
            refresh_key: gen_secret(),
            access_key: gen_secret(),
            authorization_code_key: gen_secret(),
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        Self {
            address: defaults::server_listen_address(),
            port: defaults::server_port(),
            base_url: None,
        }
    }
}

impl Config {
    pub fn get_user(&self, user_id: &user::ID) -> Option<User> {
        self.users.iter().find(|user| user.id == *user_id).cloned()
    }

    pub fn get_user_by_email(&self, user_email: &str) -> Option<User> {
        self.users
            .iter()
            .find(|user| user.email == *user_email)
            .cloned()
    }

    pub fn get_room(&self, room_id: &room::ID) -> Option<Room> {
        self.rooms.iter().find(|room| room.id == *room_id).cloned()
    }

    pub fn get_base_url(&self) -> Url {
        self.network.base_url.clone().unwrap_or_else(|| {
            let (scheme, address, port) = if let Some(tls) = &self.tls {
                ("https", &tls.address, &tls.port)
            } else {
                ("http", &self.network.address, &self.network.port)
            };
            Url::parse(&format!("{}://{}:{}", scheme, address, port)).unwrap()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::Config as _;
    use super::*;

    use std::str::FromStr;
    use url::Url;

    use crate::types::permission;
    use crate::types::room;
    use crate::types::structure;
    use crate::types::user;

    use permission::Permission;
    use room::Room;
    use structure::Structure;
    use user::User;

    #[test]
    fn test_example() {
        let expected = Config {
            network: Network {
                address: std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                port: 1234,
                base_url: Some(Url::from_str("http://localhost:1234").unwrap()),
            },
            secrets: Secrets {
                refresh_key: String::from("some-refresh-key"),
                access_key: String::from("some-access-key"),
                authorization_code_key: String::from("some-authorization-code-key"),
            },
            tls: Some(Tls {
                certificate: PathBuf::from_str("/etc/certificate").unwrap(),
                private_key: PathBuf::from_str("/etc/private-key").unwrap(),
                address: std::net::IpAddr::V4(std::net::Ipv4Addr::new(1, 2, 3, 4)),
                port: 4321,
            }),
            google: Some(Google {
                client_id: String::from("google-client-id"),
                client_secret: String::from("google-client-secret"),
                project_id: String::from("google-project-id"),
                credentials_file: PathBuf::from_str("google-credentials.json").unwrap(),
                request_sync_rate_limit_seconds: 600,
            }),
            logins: Logins {
                google: Some(GoogleLogin {
                    client_id: String::from("google-login-client-id"),
                }),
            },
            structures: [Structure {
                id: structure::ID::from_str("bd7feab5033940e296ed7fcdc700ba65").unwrap(),
                name: String::from("Zukago"),
            }]
            .to_vec(),
            rooms: [Room {
                id: room::ID::from_str("baafebaa0708441782cf17470dd98392").unwrap(),
                structure_id: structure::ID::from_str("bd7feab5033940e296ed7fcdc700ba65").unwrap(),
                name: String::from("Bedroom"),
            }]
            .to_vec(),
            users: [User {
                id: user::ID::from_str("861ccceaa3e349138ce2498768dbfe09").unwrap(),
                email: String::from("root@gbaranski.com"),
                homie: None,
            }]
            .to_vec(),
            permissions: [Permission {
                structure_id: structure::ID::from_str("bd7feab5033940e296ed7fcdc700ba65").unwrap(),
                user_id: user::ID::from_str("861ccceaa3e349138ce2498768dbfe09").unwrap(),
                is_manager: true,
            }]
            .to_vec(),
        };
        std::env::set_var("REFRESH_KEY", &expected.secrets.refresh_key);
        std::env::set_var("ACCESS_KEY", &expected.secrets.access_key);
        std::env::set_var(
            "AUTHORIZATION_CODE_KEY",
            &expected.secrets.authorization_code_key,
        );
        println!(
            "--------------------\n\n Serialized: \n{}\n\n--------------------",
            toml::to_string(&expected).unwrap()
        );
        let config = Config::parse(include_str!("../../example.toml")).unwrap();
        assert_eq!(config, expected);
        crate::Config::validate(&config).unwrap();
    }
}
