//! Messages between UDP server and client
use serde::{Deserialize, Serialize};

use crate::fc_comms::RcControls;

/// Messages between UDP server and client
#[derive(Debug)]
pub enum Message {
    /// Set the RC controls
    SetRc(RcControls),
    /// Heartbeat message
    #[cfg(feature = "heartbeat")]
    Heartbeat,
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Message::SetRc(rc_controls) => {
                let rc_str = format!("set_rc;{rc_controls}");
                serializer.serialize_str(&rc_str)
            }
            #[cfg(feature = "heartbeat")]
            Message::Heartbeat => serializer.serialize_str("heartbeat"),
        }
    }
}

impl<'de> Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parts: Vec<&str> = s.split(';').collect();

        if parts.len() > 2 {
            return Err(serde::de::Error::custom("Invalid message format"));
        }
        match parts[0] {
            "set_rc" => {
                let rc_controls = RcControls::from_str(parts[1]);
                Ok(Message::SetRc(rc_controls))
            }
            #[cfg(feature = "heartbeat")]
            "heartbeat" => Ok(Message::Heartbeat),
            _ => Err(serde::de::Error::custom("Unknown message type")),
        }
    }
}
