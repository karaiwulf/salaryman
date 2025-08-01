pub mod service;
#[cfg(feature = "protocol")]
pub mod protocol;

// re-exports
pub use self::service::{Service, ServiceConf, ServiceState};
#[cfg(feature = "protocol")]
pub use self::protocol::SalarymanPacket;

