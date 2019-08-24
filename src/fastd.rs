#![warn(dead_code)]
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::iter::IntoIterator;
use std::vec::IntoIter;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FastdStatus {
    pub uptime: f64,
    pub interface: String,
    pub statistics: Statistics,
    pub peers: HashMap<String, Peer>,
}


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Peer {
    pub name: String,
    pub address: String,
    pub connection: Option<Connection>,
}


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statistics {
    pub rx: Traffic,
    pub rx_reordered: Traffic,
    pub tx: Traffic,
    pub tx_dropped: Traffic,
    pub tx_error: Traffic,
}

impl IntoIterator for Statistics {
	type Item = (String, Traffic);
	type IntoIter = IntoIter<Self::Item>;

	fn into_iter(self) -> Self::IntoIter {
		vec![
			("rx".to_owned(),           self.rx),
			("rx_reordered".to_owned(), self.rx_reordered),
			("tx".to_owned(),           self.tx),
			("tx_dropped".to_owned(),   self.tx_dropped),
			("tx_error".to_owned(),     self.tx_error),
		].into_iter()
	}
}


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    pub established: f64,
    pub method: String,
    pub statistics: Statistics,
    pub mac_addresses: Vec<String>,
}


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Traffic {
    pub packets: f64,
    pub bytes: f64,
}
