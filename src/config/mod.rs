pub mod defaults;
pub mod server;

use regex::Regex;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    env::{self, VarError},
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
};
use tracing::Level;

pub trait Config: DeserializeOwned + Serialize {
    const DEFAULT_TOML: &'static str;
    const DEFAULT_FILE: &'static str;

    fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    fn write_defaults(path: impl AsRef<Path>) -> Result<(), Error> {
        let path = path.as_ref();
        if path.parent().is_none() || !path.parent().unwrap().exists() {
            let mut comps = path.components();
            comps.next_back();
            std::fs::create_dir_all(comps.as_path())?;
        }
        let mut file = std::fs::File::create(path)?;
        file.write_all(Self::DEFAULT_TOML.as_bytes())?;
        Ok(())
    }

    fn parse(s: &str) -> Result<Self, Error> {
        let re = Regex::new(r"\$\{([a-zA-Z_]+)\}").unwrap();
        let s = re.replace_all(s, |caps: &regex::Captures| {
            let (pos, name) = {
                let name_match = caps.get(1).unwrap();
                let pos = name_match.start();
                let name = name_match.as_str();
                (pos, name)
            };
            match env::var(name) {
                Ok(env) => env,
                Err(VarError::NotPresent) => panic!(
                    "environment variable named {} from configuration file at {} is not defined",
                    name,
                    pos
                ),
                Err(VarError::NotUnicode(_)) => panic!(
                    "environment variable named {} from configuration file at {} is not valid unicode",
                    name,
                    pos
                ),
            }
        });
        let config: Self = toml::from_str(&s)?;
        config.validate().map_err(Error::Validation)?;

        Ok(config)
    }

    fn read(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    fn default_path() -> PathBuf {
        xdg::BaseDirectories::with_prefix("homieflow")
            .unwrap()
            .get_config_home()
            .join(Self::DEFAULT_FILE)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")]
    IO(#[from] io::Error),
    #[error("toml deserialize: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("toml serialize: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("validation: {0}")]
    Validation(String),
}

pub fn init_logging(hide_timestamp: bool) {
    const LOG_ENV: &str = "HOMIEFLOW_LOG";

    let env_filter = match env::var(LOG_ENV) {
        Ok(env) => env,
        Err(VarError::NotPresent) => "info".to_string(),
        Err(VarError::NotUnicode(_)) => panic!(
            "{} environment variable is not valid unicode and can't be read",
            LOG_ENV
        ),
    };
    let level = Level::from_str(&env_filter)
        .unwrap_or_else(|err| panic!("invalid `{}` environment variable {}", LOG_ENV, err));

    if hide_timestamp {
        tracing_subscriber::fmt()
            .with_max_level(level)
            .without_time()
            .init()
    } else {
        tracing_subscriber::fmt().with_max_level(level).init()
    };
}
