use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
};
use uuid::Uuid;

pub enum ServiceState {
    Running,
    Failed,
    Stopped,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServiceConf {
    uuid: Uuid,
    name: String,
    command: String,
    args: Option<String>,
    directory: Option<PathBuf>,
    autostart: bool,
}
impl Default for ServiceConf {
    fn default() -> Self {
        Self::new()
    }
}
impl ServiceConf {
    /// Returns a new empty `ServiceConf`
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
    /// Returns a new `ServiceConf` from parts.
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
    /// Returns a new `ServiceConf` from parts with new uuid.
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
    /// Returns the `uuid::Uuid` associated with the service config
    pub fn get_uuid(&self) -> &Uuid {
        &self.uuid
    }
    /// Returns the name of the described service
    pub fn get_name(&self) -> &str {
        &self.name
    }
    /// Returns the command of the described service
    pub fn get_command(&self) -> &str {
        &self.command
    }
    /// Returns the args of the described service
    pub fn get_args(&self) -> &Option<String> {
        &self.args
    }
    /// Returns the work directory of the described service
    pub fn get_work_dir(&self) -> &Option<PathBuf> {
        &self.directory
    }
    /// Returns the autostart status of the described service
    pub fn get_autostart(&self) -> bool {
        self.autostart
    }
    /// Sets the name of the described service
    pub fn name(&mut self, name: &str) -> &mut Self {
        self.name = String::from(name);
        self
    }
    /// Sets the command of the described service
    pub fn command(&mut self, command: &str) -> &mut Self {
        self.command = String::from(command);
        self
    }
    /// Sets the args of the described service
    pub fn args(&mut self, args: &Option<String>) -> &mut Self {
        self.args = args.clone();
        self
    }
    /// Sets the work directory of the described service
    pub fn work_dir(&mut self, work_dir: &Option<PathBuf>) -> &mut Self {
        self.directory = work_dir.clone();
        self
    }
    /// Sets the autostart value of the described service
    pub fn autostart(&mut self, autostart: bool) -> &mut Self {
        self.autostart = autostart;
        self
    }
    /// Builds a Service from this object
    #[inline]
    pub fn build(&self) -> Result<Service, Box<dyn std::error::Error>> {
        Service::from_conf(&self)
    }
}

#[derive(Debug)]
pub struct Service {
    conf: ServiceConf,
    proc: Option<Child>,
    pub outpath: Option<PathBuf>,
    pub errpath: Option<PathBuf>,
}
impl Default for Service {
    fn default() -> Self {
        Self::new()
    }
}
impl<'a> Service {
    /// Returns a new empty `Service`
    pub fn new() -> Self {
        Self {
            conf: ServiceConf::default(),
            proc: None,
            outpath: None,
            errpath: None,
        }
    }
    /// Returns a `Service` made from a `ServiceConf`.
    pub fn from_conf(conf: &ServiceConf) -> Result<Self, Box<dyn std::error::Error>> {
        let mut service = Self {
            conf: conf.to_owned(),
            proc: None,
            outpath: None,
            errpath: None,
        };
        if conf.get_autostart() {
            service.start()?;
        }
        Ok(service)
    }
    /// Gets the ServiceConf associated with the service
    #[inline]
    pub fn config(&self) -> &ServiceConf {
        &self.conf
    }
    /// Returns the name of the service
    #[inline]
    pub fn name(&self) -> &str {
        &self.config().get_name()
    }
    #[inline]
    fn create_dirs(&self) -> Result<(), Box<dyn std::error::Error>> {
        match std::fs::create_dir("./logs") {
            Ok(_) => (),
            Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(Box::new(e)),
        }
        match std::fs::create_dir(format!("./logs/{}", &self.config().get_uuid())) {
            Ok(_) => (),
            Err(ref e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(Box::new(e)),
        }
        Ok(())
    }
    /// Uses `tokio::process::Command` to start the service.
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.proc.is_some() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Process Already Exists",
            )));
        }
        self.create_dirs()?;
        let outpath = PathBuf::from(format!(
            "./logs/{}/{}.log",
            &self.config().get_uuid(),
            &self.name()
        ));
        let errpath = PathBuf::from(format!(
            "./logs/{}/{}.err",
            &self.config().get_uuid(),
            &self.name()
        ));
        let outfile = File::options().append(true).create(true).open(&outpath)?;
        let errfile = File::options().append(true).create(true).open(&errpath)?;
        let cmd = &self.conf.command;
        let mut proc = Command::new(cmd);
        proc.stdin(Stdio::piped());
        proc.stdout(outfile);
        proc.stderr(errfile);
        if let Some(a) = &self.conf.args {
            proc.args(a.split_whitespace());
        };
        if let Some(c) = &self.conf.directory {
            proc.current_dir(c);
        };
        let child = proc.spawn()?;
        self.proc = Some(child);
        self.outpath = Some(outpath);
        self.errpath = Some(errpath);
        Ok(())
    }
    /// Returns true when process is started and false when process is stopped.
    pub fn started(&self) -> bool {
        self.proc.is_some()
    }
    /// Returns the process id
    pub fn id(&self) -> Result<u32, Box<dyn std::error::Error>> {
        if let Some(proc) = self.proc.as_ref() {
            Ok(proc.id())
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "process not started",
            )))
        }
    }
    /// Returns the state of the service
    pub fn state(&mut self) -> Result<ServiceState, Box<dyn std::error::Error>> {
        if let Some(proc) = self.proc.as_mut() {
            match proc.try_wait() {
                Err(_) | Ok(Some(_)) => Ok(ServiceState::Failed),
                Ok(None) => Ok(ServiceState::Running),
            }
        } else {
            Ok(ServiceState::Stopped)
        }
    }
    /// Invokes kill on the service process
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(proc) = self.proc.as_mut() {
            proc.kill()?;
            self.proc = None;
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No Process Associated with Service",
            )))
        }
    }
    /// Restarts service process
    #[inline]
    pub fn restart(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stop()?;
        self.start()?;
        Ok(())
    }
    /// Writes to the service process' stdin, if it exists.
    pub fn write_stdin(&mut self, buf: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(proc) = self.proc.as_mut() {
            let stdin = if let Some(stdin) = proc.stdin.as_mut() {
                stdin
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No stdin handle associated with process",
                )));
            };
            stdin.write(&buf.as_bytes())?;
            stdin.flush()?;
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No Process Associated with Service",
            )))
        }
    }
    /// Writes a line to the service process' stdin, if it exists.
    #[inline]
    pub fn writeln_stdin(&mut self, buf: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.write_stdin(&format!("{}\n", buf))
    }
}
