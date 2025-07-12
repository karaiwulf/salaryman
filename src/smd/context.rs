use salaryman::service::{Service, ServiceConf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct SalarymanService {
    pub config: ServiceConf,
    pub service: Arc<Mutex<Service>>,
}
impl SalarymanService {
    pub fn new() -> Self {
        Self {
            config: ServiceConf::new(),
            service: Arc::new(Mutex::new(Service::new())),
        }
    }
    pub fn from_parts(config: ServiceConf, service: Arc<Mutex<Service>>) -> Self {
        Self { config, service }
    }
}

pub struct SalarymanDContext {
    pub services: Vec<Arc<SalarymanService>>,
}
impl SalarymanDContext {
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
        }
    }
    pub fn from_vec(services: Vec<Arc<SalarymanService>>) -> Self {
        Self { services }
    }
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
