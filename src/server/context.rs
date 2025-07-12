use super::Config;
use salaryman::service::{Service, ServiceConf};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SalarymanService {
    pub config: ServiceConf,
    pub service: Arc<RwLock<Service>>,
}
impl SalarymanService {
    pub fn new() -> Self {
        Self {
            config: ServiceConf::new(),
            service: Arc::new(RwLock::new(Service::new())),
        }
    }
    pub fn from_parts(config: ServiceConf, service: Arc<RwLock<Service>>) -> Self {
        Self { config, service }
    }
}

pub struct SalarymanDContext {
    pub services: RwLock<Vec<Arc<SalarymanService>>>,
    pub save_file: PathBuf,
    pub config: Arc<RwLock<Config>>,
}
impl SalarymanDContext {
    pub fn new() -> Self {
        Self {
            services: RwLock::new(Vec::new()),
            save_file: PathBuf::from(""),
            config: Arc::new(RwLock::new(Config::new())),
        }
    }
    pub fn from_parts(
        services: RwLock<Vec<Arc<SalarymanService>>>,
        save_file: PathBuf,
        config: Arc<RwLock<Config>>,
    ) -> Self {
        Self {
            services,
            save_file,
            config,
        }
    }
}
