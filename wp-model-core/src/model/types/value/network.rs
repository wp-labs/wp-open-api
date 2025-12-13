use std::{
    fmt::{Display, Formatter},
    net::IpAddr,
};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpAddrValue(pub IpAddr);

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainValue(pub String);

#[derive(PartialEq, Serialize, Deserialize, Debug, Clone, Eq)]
pub struct IpNetValue {
    addr: IpAddr,
    prefix_len: u8,
}

impl IpNetValue {
    pub fn new(addr: IpAddr, prefix_len: u8) -> Option<Self> {
        // Validate prefix length based on address family (v4: <=32, v6: <=128)
        let max_prefix = match addr {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        } as u8;
        if prefix_len > max_prefix {
            return None;
        }
        Some(Self { addr, prefix_len })
    }
}
impl Display for IpNetValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.addr, self.prefix_len)
    }
}
// Comparison impl moved to orion_exp adapters.

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct DomainT(pub String);
impl Display for DomainT {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct UrlValue(pub String);
impl Display for UrlValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct EmailT(pub String);
impl Display for EmailT {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
use serde::{Deserialize, Serialize};
