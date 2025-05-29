use std::process::{Command, Stdio, Child};
use std::io::Read;

fn exec(image: &str, args: Vec<&str>) -> Result<Child, Box<dyn std::error::Error>> {
    let child = Command::new(image).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
    Ok(child)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut child = exec("java", vec!["-jar", "minecraft_server.jar"])?;
    std::thread::sleep(std::time::Duration::from_secs(60));
    let mut buf: [u8; 512] = [0; 512];
    child.stdout.as_mut().unwrap().read(&mut buf[..])?;
    println!("{}", String::from_utf8_lossy(&buf));
    child.kill()?;
    Ok(())
}
