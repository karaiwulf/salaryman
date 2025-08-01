use serde::{Serialize, Deserialize};
use uuid::Uuid;
use super::service::ServiceConf;

#[derive(Serialize, Deserialize)]
pub enum SalarymanPacket {
    Ping,
    Pong,
    Okay,
    Create(ServiceConf),
    Delete(Uuid),
    Start(Uuid),
    Stop(Uuid),
    Restart(Uuid),

    Quit,
    Invalid,
}
impl SalarymanPacket {
    /// Used to encode the enum and any nested structs for network transmission
    pub fn serialize(&self) -> Result<Vec<u8>, bincode::error::EncodeError>{
        bincode::serde::encode_to_vec(&self, bincode::config::standard())
    }
    /// Used to decode the enum and any nested struct from a network transmission
    pub fn deserialize(data: &[u8]) -> Result<SalarymanPacket, bincode::error::DecodeError> {
        let (ret, _size) = bincode::serde::decode_from_slice(data, bincode::config::standard())?;
        Ok(ret)
    }
    /// Generates a response from a given SalarymanPacket invariant
    pub fn response(&self) -> Self {
        match self {
            Self::Ping => Self::Pong,
            Self::Create(_) => Self::Okay,
            Self::Delete(_) => Self::Okay,
            Self::Start(_) => Self::Okay,
            Self::Stop(_) => Self::Okay,
            Self::Restart(_) => Self::Okay,
            Self::Quit => Self::Okay,
            _ => Self::Invalid,
        }
    }
}
