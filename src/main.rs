use clap::Parser;
use serde::{Deserialize, Serialize};

use tokio::fs::read_to_string;

use std::{net::IpAddr, path::PathBuf};

use salaryman::model::{Service, ServiceConf};

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

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    address: Option<IpAddr>,
    port: Option<u16>,
    service: Vec<ServiceConf>,
}
impl Config {
    fn new() -> Self {
        Self {
            address: None,
            port: None,
            service: Vec::new(),
        }
    }
}

async fn load_config(file: &PathBuf) -> Config {
    let s: String = match read_to_string(file).await {
        Ok(s) => s,
        Err(_) => String::new(),
    };
    match toml::from_str(s.as_str()) {
        Ok(c) => c,
        Err(_) => Config::new(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let conf: Config = load_config(&args.config).await;
    let mut services: Vec<Service> = Vec::new();
    for i in 0..conf.service.len() {
        services.push(Service::from_conf(&conf.service[i]));
        if conf.service[i].autostart {
            services[i].start().await?;
        }
    }
    let mut outs: Vec<(String, tokio::sync::mpsc::Receiver<String>)> = Vec::new();
    for i in 0..services.len() {
        if services[i].started().await {
            outs.push((services[i].name().await, services[i].scan_stdout().await?));
        }
    }
    for _i in 0..100 {
        for out in 0..outs.len() {
            if let Some(s) = outs[out].1.recv().await {
                println!("got line from {} :: {}", outs[out].0, s);
            }
        }
    }
    for mut service in services {
        match service.stop().await {
            Ok(_) => println!("lol it was killed"),
            Err(_) => println!("it either didn't exist, or failed to kill"),
        }
    }
    Ok(())
}
