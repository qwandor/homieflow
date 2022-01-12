use chrono::Duration;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Eq, PartialEq, strum::Display, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[repr(u8)]
pub enum Client {
    Internal,
    GoogleHome,
}

impl Client {
    pub fn access_token_duration(&self) -> Duration {
        match *self {
            Self::Internal => Duration::minutes(10),
            Self::GoogleHome => Duration::minutes(10),
        }
    }
}
