use serde::{
    Deserialize,
    Serialize,
};
use clap::Parser;

use tokio::fs::read_to_string;

use std::{
    fs::canonicalize,
    io::Read,
    process::{
        Child,
        Command,
        Stdio
    },
    path::PathBuf,
    net::IpAddr,
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

#[derive(Serialize, Deserialize)]
struct Service {
    name: String,
    command: String,
    args: String,
    directory: Option<PathBuf>,
}
impl Service {
    fn new() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            args: String::new(),
            directory: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
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

fn exec(image: &str, args: Vec<&str>, dir: Option<PathBuf>) -> Result<Child, Box<dyn std::error::Error>> {
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

async fn load_config(file: PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    let config: Config = toml::from_str(read_to_string(file).await?.as_str())?;
    Ok(config)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut conf: Config = Config::new();
    if let Some(file) = args.config.as_deref() {
        conf = load_config(PathBuf::from(file)).await?;
    } else {
        conf = load_config(PathBuf::from("salaryman.toml")).await?;
    }
    
    let mut child = exec("java", vec!["-jar", "minecraft_server.jar"], None)?;
    std::thread::sleep(std::time::Duration::from_secs(60));
    let mut buf: [u8; 512] = [0; 512];
    let mut ebuf: [u8; 512] = [0; 512];
    child.stdout.as_mut().unwrap().read(&mut buf[..])?;
    child.stderr.as_mut().unwrap().read(&mut ebuf[..])?;
    println!("{}", String::from_utf8_lossy(&buf));
    println!("{}", String::from_utf8_lossy(&ebuf));
    child.kill()?;
    Ok(())
}

