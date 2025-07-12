use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct UpdateConf {
    pub address: Option<IpAddr>,
    pub port: Option<u16>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct StdinBuffer {
    pub stdin: String,
    pub endl: Option<bool>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct ServicePath {
    pub service_uuid: Uuid,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct NewService {
    pub name: Option<String>,
    pub command: Option<String>,
    pub args: Option<Option<String>>,
    pub directory: Option<Option<PathBuf>>,
    pub autostart: Option<bool>,
}
