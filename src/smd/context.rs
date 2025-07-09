use super::Config;
use salaryman::service::Service;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SalarymanDContext {
    pub config: Config,
    pub service: Vec<Arc<Mutex<Service>>>,
}
impl SalarymanDContext {
    pub fn new() -> Self {
        Self {
            config: Config::new(),
            service: Vec::new(),
        }
    }
    pub fn from_parts(config: Config, service: Vec<Arc<Mutex<Service>>>) -> Self {
        Self { config, service }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct StdinBuffer {
    pub string: String,
}
