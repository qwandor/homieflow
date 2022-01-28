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
