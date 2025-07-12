use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{
        Mutex,
        mpsc::{Receiver, channel},
    },
    task::spawn,
};
use uuid::Uuid;

use std::{path::PathBuf, process::Stdio, sync::Arc};

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ServiceConf {
    pub uuid: Uuid,
    pub name: String,
    pub command: String,
    pub args: Option<String>,
    pub directory: Option<PathBuf>,
    pub autostart: bool,
}
impl Default for ServiceConf {
    fn default() -> Self {
        Self::new()
    }
}
impl ServiceConf {
    /**
     *  Returns a new empty `ServiceConf`
     */
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            name: String::new(),
            command: String::new(),
            args: None,
            directory: None,
            autostart: false,
        }
    }
    /**
     *  Returns a new `ServiceConf` from parts.
     */
    pub fn from_parts(
        uuid: Uuid,
        name: String,
        command: String,
        args: Option<String>,
        directory: Option<PathBuf>,
        autostart: bool,
    ) -> Self {
        Self {
            uuid,
            name,
            command,
            args,
            directory,
            autostart,
        }
    }
    /**
     *  Returns a new `ServiceConf` from parts with new uuid.
     */
    pub fn new_from_parts(
        name: String,
        command: String,
        args: Option<String>,
        directory: Option<PathBuf>,
        autostart: bool,
    ) -> Self {
        Self {
            uuid: Uuid::new_v4(),
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
    conf: ServiceConf,
    proc: Option<Arc<Mutex<Child>>>,
    pub stdout: Option<Arc<Mutex<Receiver<String>>>>,
    pub stderr: Option<Arc<Mutex<Receiver<String>>>>,
}
impl Default for Service {
    fn default() -> Self {
        Self::new()
    }
}
impl Service {
    /**
     *  Returns a new empty `Service`
     */
    pub fn new() -> Self {
        Self {
            conf: ServiceConf::default(),
            proc: None,
            stdout: None,
            stderr: None,
        }
    }
    /**
     *  Returns a `Service` made from a `ServiceConf`.
     */
    pub fn from_conf(conf: &ServiceConf) -> Self {
        Self {
            conf: conf.clone(),
            proc: None,
            stdout: None,
            stderr: None,
        }
    }
    /**
     *  Returns the name of the service
     */
    pub async fn name(&self) -> &str {
        &self.conf.name
    }
    /**
     *  Uses `tokio::process::Command` to start the service.
     */
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.proc.is_some() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Process Already Exists",
            )));
        }
        let cmd = &self.conf.command;
        let mut proc = Command::new(cmd);
        proc.stdin(Stdio::piped());
        proc.stdout(Stdio::piped());
        proc.stderr(Stdio::piped());
        if let Some(a) = &self.conf.args {
            proc.args(a.split_whitespace());
        };
        if let Some(c) = &self.conf.directory {
            proc.current_dir(c);
        };
        let child = proc.spawn()?;
        self.proc = Some(Arc::new(Mutex::new(child)));
        Ok(())
    }
    /**
     *  Calls self.start(), then self.scan_stdout(), and finally self.scan_stderr()
     */
    #[inline]
    pub async fn start_with_output(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.start().await?;
        self.scan_stdout().await?;
        self.scan_stderr().await?;
        Ok(())
    }
    //TODO: process monitoring!
    /**
     *  Returns true when process is started and false when process is stopped.
     */
    pub async fn started(&self) -> bool {
        self.proc.is_some()
    }
    /**
     *  Invokes kill on the service process
     */
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(proc) = self.proc.clone() {
            let mut lock = proc.lock().await;
            lock.kill().await?;
            drop(lock);
            self.proc = None;
            self.stdout = None;
            self.stderr = None;
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No Process Associated with Service",
            )))
        }
    }
    /**
     *  Restarts service process
     */
    #[inline]
    pub async fn restart(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stop().await?;
        self.start().await?;
        Ok(())
    }
    /**
     *  Restarts service process
     */
    #[inline]
    pub async fn restart_with_output(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stop().await?;
        self.start_with_output().await?;
        Ok(())
    }
    /**
     *  Takes control of service process' stdout file handle and spawns a new task to continuously
     *  scan it.
     */
    pub async fn scan_stdout(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(proc) = self.proc.clone() {
            let mut lock = proc.lock().await;
            let stdout = if let Some(stdout) = lock.stdout.take() {
                stdout
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No stdout handle associated with process",
                )));
            };
            drop(lock);
            let (tx, rx) = channel(1024);
            let sname = self.conf.name.clone();
            let suuid = self.conf.uuid.clone();
            spawn(async move {
                let mut br = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = br.next_line().await {
                    println!("{} ({}) :: {}", &suuid, &sname, &line);
                    if let Err(_) = tx.send(line).await {
                        return;
                    };
                }
            });
            self.stdout = Some(Arc::new(Mutex::new(rx)));
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No Process Associated with Service",
            )))
        }
    }
    /**
     *  Takes control of service process' stderr file handle and spawns a new task to continuously
     *  scan it.
     */
    pub async fn scan_stderr(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(proc) = self.proc.clone() {
            let mut lock = proc.lock().await;
            let stderr = if let Some(stderr) = lock.stderr.take() {
                stderr
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No stderr handle associated with process",
                )));
            };
            drop(lock);
            let (tx, rx) = channel(1024);
            let sname = self.conf.name.clone();
            let suuid = self.conf.uuid.clone();
            spawn(async move {
                let mut br = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = br.next_line().await {
                    eprintln!("{} ({}) >< {}", &suuid, &sname, &line);
                    if let Err(_) = tx.send(line).await {
                        return;
                    };
                }
            });
            self.stderr = Some(Arc::new(Mutex::new(rx)));
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No Process Associated with Service",
            )))
        }
    }
    /**
     *  Writes to the service process' stdin, if it exists.
     */
    pub async fn write_stdin(&mut self, buf: String) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(proc) = self.proc.clone() {
            let mut lock = proc.lock().await;
            let stdin = if let Some(stdin) = lock.stdin.as_mut() {
                stdin
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No stdin handle associated with process",
                )));
            };
            stdin.write(&buf.as_bytes()).await?;
            stdin.flush().await?;
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No Process Associated with Service",
            )))
        }
    }
    /**
     *  Writes a line to the service process' stdin, if it exists.
     */
    #[inline]
    pub async fn writeln_stdin(&mut self, buf: String) -> Result<(), Box<dyn std::error::Error>> {
        self.write_stdin(format!("{}\n", buf)).await
    }
}
