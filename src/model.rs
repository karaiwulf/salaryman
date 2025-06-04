use serde::{Deserialize, Serialize};

use std::{
    fs::canonicalize,
    io::{Read, BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServiceConf {
    pub name: String,
    command: String,
    args: Option<String>,
    directory: Option<PathBuf>,
    pub autostart: bool,
}
impl ServiceConf {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            args: None,
            directory: None,
            autostart: false,
        }
    }
    pub fn from_parts(
        name: String,
        command: String,
        args: Option<String>,
        directory: Option<PathBuf>,
        autostart: bool,
    ) -> Self {
        Self {
            name,
            command,
            args,
            directory,
            autostart,
        }
    }
}

#[derive(Debug)]
pub struct Service {
    pub config: ServiceConf,
    process: Option<Mutex<Child>>,
}
impl Service {
    pub fn new() -> Self {
        Self {
            config: ServiceConf::new(),
            process: None,
        }
    }
    pub fn from_conf(config: ServiceConf) -> Self {
        Self {
            config,
            process: None,
        }
    }
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.process.is_some() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "process already exists!",
            )));
        }
        let command: &str = &self.config.command.as_str();
        let args: Vec<String> = if let Some(a) = &self.config.args {
            let mut v: Vec<String> = Vec::new();
            for arg in a.split_whitespace() {
                v.push(String::from(arg));
            }
            v
        } else {
            Vec::new()
        };
        if let Some(cwd) = &self.config.directory {
            let child = Command::new(command)
                .args(args)
                .current_dir(canonicalize(cwd)?)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            self.process = Some(Mutex::new(child));
        } else {
            let child = Command::new(command)
                .args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            self.process = Some(Mutex::new(child));
        };
        Ok(())
    }
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(process) = &self.process {
            if let Ok(mut process) = process.lock() {
                process.kill()?;
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::ResourceBusy,
                    "cannot acquire lock",
                )))
            }
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "process already exists!",
            )));
        };
        self.process = None;
        Ok(())
    }
    //TODO: this function needs to fork and do message passing via mpsc channels.
    pub fn stdout(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(process) = &mut self.process {
            if let Ok(mut process) = process.lock() {
                if let Some(stdout) = process.stdout.as_mut() {
                    let reader = BufReader::new(stdout);
                    Ok(String::from(reader.lines().filter_map(|line| line.ok()).collect::<String>()))
                } else {
                    Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::ResourceBusy,
                        "cannot acquire stdout",
                    )))
                }
            } else {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::ResourceBusy,
                    "cannot acquire lock",
                )))
            }
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "is the process started?",
            )))
        }
    }
    //TODO: this function needs to fork and do message passing via mpsc channels.
    //TODO: this function needs to use a bufreader instead
    pub fn stderr(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(process) = &mut self.process {
            if let Ok(mut process) = process.lock() {
                let mut s = String::new();
                if let Some(stderr) = process.stderr.as_mut() {
                    stderr.read_to_string(&mut s)?;
                }
                Ok(s)
            } else {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::ResourceBusy,
                    "cannot acquire lock",
                )))
            }
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "is the process started?",
            )))
        }
    }
}

