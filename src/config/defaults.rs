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

use std::net::IpAddr;
use std::net::Ipv4Addr;

pub const fn server_listen_address() -> IpAddr {
    IpAddr::V4(Ipv4Addr::LOCALHOST)
}

pub const fn server_port() -> u16 {
    6001
}

pub const fn server_port_tls() -> u16 {
    6002
}
