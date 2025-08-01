use clap::Parser;
use rayon::prelude::*;
use salaryman::{Service, ServiceConf, ServiceState, SalarymanPacket};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    fs::{File, read_to_string},
    os::unix::net::UnixListener,
    path::PathBuf,
    sync::mpsc::{TryRecvError, channel},
};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "config file override",
        default_value = "None",
    )]
    config: Option<PathBuf>,
    #[arg(
        short,
        long,
        value_name = "SOCK",
        help = "UNIX socket to bind",
        default_value = "/tmp/salaryman.sock"
    )]
    socket: PathBuf,
}

pub enum InnerProtocol {
    Create(ServiceConf),
    Delete(Uuid),
    Start(Uuid),
    Stop(Uuid),
    Write((Uuid, String)),
    Scan,
    Quit,
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

fn _save_config(file: &PathBuf, conf: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::options().create(true).truncate(true).write(true).open(file)?;
    f.write(toml::to_string(conf)?.as_bytes())?;
    Ok(())
}

#[cfg(not(target_os = "windows"))] // We cannot build on Windows
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Config Handling
    let local_conf = PathBuf::from("salaryman.conf");
    let xdg_conf = PathBuf::from({
        if let Ok(xcd) = std::env::var("XDG_CONFIG_HOME") {
            xcd + "/smd/salaryman.toml"
        } else {
            if let Ok(homedir) = std::env::var("HOME") {
                homedir + "/.config/smd/salaryman.conf"
            } else {
                "/root/.config/smd/salaryman.conf".into()
            }
        }
    });
    let sys_conf = PathBuf::from("/etc/salaryman/smd/salaryman.conf");
    let confpaths = if let Some(path) = args.config.clone() {
        vec![path]
    } else {
        vec![local_conf, xdg_conf, sys_conf]
    };
    let get_conf = |confpaths, p: Option<PathBuf>| -> (PathBuf, Config) {
        for cfile in confpaths {
            if let Ok(conf) = load_config(&cfile) {
                return (cfile.to_owned(), conf);
            }
        }
        if let Some(cpath) = p {
            (cpath.to_owned(), Config::new())
        } else {
            (PathBuf::from("salaryman.toml"), Config::new())
        }
    };
    let (config_path, conf) = get_conf(confpaths, args.config.clone());

    let sockaddr = if let Some(sock) = conf.socket {
        sock
    } else {
        args.socket
    };
    let mut services: Vec<Service> = Vec::new();
    for service in conf.service {
        services.push(service.build()?);
    }
    // event loop sender
    let (etx, erx) = channel();
    // listener
    let (ltx, lrx) = channel();
    // main services event loop
    let (stx, srx) = channel();

    // for event loop sender to main services event
    let queue = stx.clone();
    let lkill = ltx.clone();
    std::thread::spawn(move || {
        loop { // event loop sender
            match erx.try_recv() {
                Err(TryRecvError::Disconnected) | Ok(true) => {
                    queue.send(InnerProtocol::Quit).unwrap_or_default();
                    lkill.send(true).unwrap_or_default();
                    return;
                },
                _ => (),
            }
            queue.send(InnerProtocol::Scan).unwrap_or_default();
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    });

    let ekill = etx.clone();
    let lkill = ltx.clone();
    std::thread::spawn(move || {
        // Main Services Event Loop
        loop {
            match srx.try_recv() {
                Err(TryRecvError::Disconnected) | Ok(InnerProtocol::Quit) => {
                    services.par_iter_mut()
                        .for_each(|service| {
                            if let Err(e) = service.stop() {
                                eprintln!("unable to start service {} with id {} due to error {e}", &service.name(), &service.uuid());
                            }
                        });
                    ekill.send(true).unwrap_or_default();
                    lkill.send(true).unwrap_or_default();
                    return;
                },
                Ok(InnerProtocol::Create(conf)) => if let Ok(s) = conf.build() { services.push(s); },
                Ok(InnerProtocol::Start(u)) => {
                    services.par_iter_mut()
                        .filter(|s| s.uuid() == &u)
                        .for_each(|s| {
                            if !s.started() {
                                if let Ok(_) = s.start() {
                                    return;
                                }
                            }
                        });
                },
                Ok(InnerProtocol::Stop(u)) => {
                    services.par_iter_mut()
                        .filter(|s| s.uuid() == &u)
                        .for_each(|s| {
                            if s.started() {
                                if let Ok(_) = s.stop() {
                                    return;
                                }
                            }
                        });
                },
                Ok(InnerProtocol::Write((u, buf))) => {
                    services.par_iter_mut()
                        .filter(|s| s.uuid() == &u)
                        .for_each(|s| {
                            if let Ok(_) = s.write_stdin(&buf) {
                                return;
                            }
                        });
                },
                Ok(InnerProtocol::Delete(u)) => {
                    if let Some(i) = services.par_iter_mut()
                        .position_first(|s| s.uuid() == &u) {
                        if let Err(e) = services[i].stop() {
                            eprintln!("unable to stop service {} with id {} due to error {e}", &services[i].name(), &services[i].uuid());
                        };
                        services.swap_remove(i);
                    }
                },
                Ok(InnerProtocol::Scan) => {
                    services.par_iter_mut()
                        .for_each(|service| {
                            match service.state() {
                                ServiceState::Failed => {
                                    if let Err(e) = service.restart() {
                                        eprintln!("unable to restart service {} with id {} due to error {e}", &service.name(), &service.uuid());
                                    }
                                },
                                _ => (),
                            }
                        });
                },
                _ => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }
    });
    let listen = UnixListener::bind(sockaddr)?;
    let queue = stx.clone();
    let ekill = etx.clone();
    for stream in listen.incoming() {
        match stream {
            Ok(mut stream) => {
                match lrx.try_recv() {
                    Err(TryRecvError::Disconnected) | Ok(true) => {
                        ekill.send(true).unwrap_or_default();
                        queue.send(InnerProtocol::Quit).unwrap_or_default();
                        return Ok(());
                    },
                    _ => (),
                }
                let squeue = queue.clone();
                std::thread::spawn(move || {
                    let mut buf = String::new();
                    if let Ok(len) = stream.read_to_string(&mut buf) {
                        if len > 0 {
                            let resp = match SalarymanPacket::deserialize(&buf.as_bytes()) {
                                Ok(SalarymanPacket::Create(sc)) => {
                                    squeue.send(InnerProtocol::Create(sc.to_owned())).unwrap_or_default();
                                    SalarymanPacket::response(&SalarymanPacket::Create(sc))
                                },
                                Ok(SalarymanPacket::Delete(u)) => {
                                    squeue.send(InnerProtocol::Delete(u.to_owned())).unwrap_or_default();
                                    SalarymanPacket::response(&SalarymanPacket::Delete(u))
                                },
                                Ok(SalarymanPacket::Start(u)) => {
                                    squeue.send(InnerProtocol::Start(u.to_owned())).unwrap_or_default();
                                    SalarymanPacket::response(&SalarymanPacket::Start(u))
                                },
                                Ok(SalarymanPacket::Stop(u)) => {
                                    squeue.send(InnerProtocol::Stop(u.to_owned())).unwrap_or_default();
                                    SalarymanPacket::response(&SalarymanPacket::Stop(u))
                                },
                                Ok(SalarymanPacket::Restart(u)) => {
                                    squeue.send(InnerProtocol::Stop(u.to_owned())).unwrap_or_default();
                                    squeue.send(InnerProtocol::Start(u.to_owned())).unwrap_or_default();
                                    SalarymanPacket::response(&SalarymanPacket::Restart(u))
                                },
                                Ok(SalarymanPacket::Quit) => {
                                    squeue.send(InnerProtocol::Quit).unwrap_or_default();
                                    SalarymanPacket::response(&SalarymanPacket::Quit)
                                },
                                Ok(b) => SalarymanPacket::response(&b),
                                Err(_) => SalarymanPacket::Invalid,
                            };
                            let resp = if let Ok(i) = SalarymanPacket::serialize(&resp) {
                                i
                            } else {
                                Vec::new()
                            };
                            if let Ok(len) = stream.write(&resp) {
                                if len > 0 {
                                    stream.flush().unwrap_or_default();
                                }
                            }
                        }
                    }
                });
            },
            Err(_) => {
                match lrx.try_recv() {
                    Err(TryRecvError::Disconnected) | Ok(true) => {
                        ekill.send(true).unwrap_or_default();
                        queue.send(InnerProtocol::Quit).unwrap_or_default();
                        return Ok(());
                    },
                    _ => (),
                }
            },
        }
    }
    Ok(())
}

