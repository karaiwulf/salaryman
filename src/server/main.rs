mod context;
mod endpoints;

use clap::Parser;
use dropshot::{ApiDescription, ConfigDropshot, ConfigLogging, ConfigLoggingLevel, ServerBuilder};
use salaryman::service::{Service, ServiceConf};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::{fs::read_to_string, sync::RwLock};

use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use crate::context::{SalarymanDContext, SalarymanService};
use crate::endpoints::{
    endpoint_get_config, endpoint_get_config_save, endpoint_get_service, endpoint_get_services,
    endpoint_post_service, endpoint_post_stdin, endpoint_put_config, endpoint_restart_service,
    endpoint_start_service, endpoint_stop_service,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "config file override",
        default_value = "salaryman.toml"
    )]
    config: PathBuf,
    #[arg(
        short,
        long,
        value_name = "ADDR",
        help = "IP address to bind API to",
        default_value = "127.0.0.1"
    )]
    address: IpAddr,
    #[arg(
        short,
        long,
        value_name = "PORT",
        help = "TCP Port to bind API to",
        default_value = "3080"
    )]
    port: u16,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct User {
    pub username: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct Config {
    pub address: Option<IpAddr>,
    pub port: Option<u16>,
    pub user: Vec<User>,
    pub service: Vec<ServiceConf>,
}
impl Config {
    pub fn new() -> Self {
        Self {
            address: None,
            port: None,
            user: Vec::new(),
            service: Vec::new(),
        }
    }
}

async fn load_config(file: &PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    let s: String = match read_to_string(file).await {
        Ok(s) => s,
        Err(_) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "cannot find config file",
            )));
        }
    };
    match toml::from_str(s.as_str()) {
        Ok(c) => Ok(c),
        Err(_) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to parse config file",
        ))),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let conf: Config = load_config(&args.config).await?;
    let addr = if let Some(addr) = conf.address {
        addr
    } else {
        args.address
    };
    let port = if let Some(port) = conf.port {
        port
    } else {
        args.port
    };
    let bind = SocketAddr::new(addr, port);
    let services: RwLock<Vec<Arc<SalarymanService>>> = RwLock::new(Vec::new());
    for i in 0..conf.service.len() {
        let mut lock = services.write().await;
        lock.push(Arc::new(SalarymanService::from_parts(
            conf.service[i].clone(),
            Arc::new(RwLock::new(Service::from_conf(&conf.service[i]))),
        )));
        drop(lock);
    }
    let lock = services.write().await;
    for i in 0..lock.len() {
        if lock[i].config.autostart {
            let mut l = lock[i].service.write().await;
            l.start().await?;
            l.scan_stdout().await?;
            l.scan_stderr().await?;
            drop(l);
        }
    }
    drop(lock);
    let log_conf = ConfigLogging::StderrTerminal {
        level: ConfigLoggingLevel::Info,
    };
    let log = log_conf.to_logger("smd")?;
    let ctx = Arc::new(SalarymanDContext::from_parts(
        services,
        args.config,
        Arc::new(RwLock::new(conf)),
    ));
    let config = ConfigDropshot {
        bind_address: bind,
        ..Default::default()
    };
    let mut api = ApiDescription::new();
    api.register(endpoint_get_services)?;
    api.register(endpoint_get_service)?;
    api.register(endpoint_start_service)?;
    api.register(endpoint_stop_service)?;
    api.register(endpoint_restart_service)?;
    api.register(endpoint_post_stdin)?;
    api.register(endpoint_post_service)?;
    api.register(endpoint_get_config)?;
    api.register(endpoint_put_config)?;
    api.register(endpoint_get_config_save)?;
    api.openapi("Salaryman", semver::Version::new(1, 0, 0))
        .write(&mut std::io::stdout())?;
    let server = ServerBuilder::new(api, ctx.clone(), log)
        .config(config)
        .start()?;
    server.await?;
    Ok(())
}
