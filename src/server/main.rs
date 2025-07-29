use clap::Parser;
use salaryman::service::{Service, ServiceConf, ServiceState};
use serde::{Deserialize, Serialize};
use std::{
    fs::read_to_string,
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
};
use rayon::prelude::*;

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
        value_name = "SOCK",
        help = "UNIX socket to bind",
        default_value = "/tmp/salaryman.sock"
    )]
    socket: PathBuf,
}

pub enum ServiceReq {
    Create(ServiceConf),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub socket: Option<PathBuf>,
    pub service: Vec<ServiceConf>,
}
impl Config {
    pub fn new() -> Self {
        Self {
            socket: None,
            service: Vec::new(),
        }
    }
}

fn load_config(file: &PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    let s: String = match read_to_string(file) {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let conf: Config = load_config(&args.config)?;
    let _sockaddr = if let Some(sock) = conf.socket {
        sock
    } else {
        args.socket
    };
    let mut services: Vec<Service> = Vec::new();
    for service in conf.service {
        services.push(service.build()?);
    }
    loop {
        services.par_iter_mut()
            .for_each(|service| {
                match service.state().expect("unable to get service state") {
                    ServiceState::Failed => service.restart().expect("unable to restart service"),
                    ServiceState::Stopped => (),
                    _ => (),
                }
            });
        services.push(Service::new());
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
