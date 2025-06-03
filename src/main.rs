use clap::Parser;
use serde::{Deserialize, Serialize};

use tokio::fs::read_to_string;
use tokio::fs::write;

use std::{
    fs::canonicalize,
    io::Read,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    path::PathBuf,
    process::{Child, Command, Stdio},
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    #[arg(short, long, value_name = "ADDR")]
    address: Option<IpAddr>,
    #[arg(short, long, value_name = "PORT")]
    port: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Service {
    name: String,
    command: String,
    args: Option<String>,
    directory: Option<PathBuf>,
    autostart: bool,
}
impl Service {
    fn new() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            args: None,
            directory: None,
            autostart: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    address: Option<IpAddr>,
    port: Option<u16>,
    service: Vec<Service>,
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

fn exec(
    image: &str,
    args: Vec<&str>,
    dir: Option<PathBuf>,
) -> Result<Child, Box<dyn std::error::Error>> {
    if let Some(cwd) = dir {
        let child = Command::new(image)
            .args(args)
            .current_dir(canonicalize(cwd)?)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(child)
    } else {
        let child = Command::new(image)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(child)
    }
}

async fn load_config(file: PathBuf) -> Config {
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
    let conf: Config = load_config(PathBuf::from("salaryman.toml")).await;

    println!("{conf:?}");

    /*
        let mut child = exec("java", vec!["-jar", "minecraft_server.jar"], None)?;
        std::thread::sleep(std::time::Duration::from_secs(60));
        let mut buf: [u8; 512] = [0; 512];
        let mut ebuf: [u8; 512] = [0; 512];
        child.stdout.as_mut().unwrap().read(&mut buf[..])?;
        child.stderr.as_mut().unwrap().read(&mut ebuf[..])?;
        println!("{}", String::from_utf8_lossy(&buf));
        println!("{}", String::from_utf8_lossy(&ebuf));
        child.kill()?;
    */
    Ok(())
}
