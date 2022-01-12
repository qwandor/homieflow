use serde::Deserialize;
use serde::Serialize;
use strum::EnumString;
use strum::EnumVariantNames;
use strum::IntoStaticStr;

/// Traits defines what functionality device supports
#[derive(
    Debug,
    Clone,
    Hash,
    Eq,
    PartialEq,
    strum::Display,
    IntoStaticStr,
    EnumString,
    Serialize,
    Deserialize,
)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Trait {
    OnOff,
    OpenClose,
}

/// Type of the device
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    strum::Display,
    EnumString,
    EnumVariantNames,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Type {
    Garage,
    Gate,
    Light,
}

#[derive(Debug, Clone, Eq, PartialEq, strum::Display, EnumVariantNames, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "command", content = "params", rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Command {
    OnOff(commands::OnOff),
    OpenClose(commands::OpenClose),
}

pub mod commands {
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub struct OnOff {
        pub on: bool,
    }

    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub struct OpenClose {
        pub open_percent: u8,
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, strum::Display)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum Error {
    /// Actually, <device(s)> <doesn't/don't> support that functionality.
    FunctionNotSupported,
    /// Device does not support sent parameters
    InvalidParameters,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, strum::Display)]
#[serde(tag = "status", content = "description")]
#[repr(u8)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Confirm that the command succeeded.
    Success,
    /// Target device is unable to perform the command.
    Error(Error),
}
