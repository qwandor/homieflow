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
use serde::Serialize;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("clerk: {0}")]
    Clerk(String),
    #[error("mailer: {0}")]
    Mailer(String),
    #[error("other: {0}")]
    Other(String),
    #[error("rendering template: {0}")]
    Template(String),
}
