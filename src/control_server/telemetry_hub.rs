//! Receives telemetry from the drone over UDP and fans it out to HTTP clients
use std::time::Duration;

use serde::Serialize;
use tokio::{net::UdpSocket, sync::watch, time::sleep};
use tracing::{debug, info, warn};

use crate::{fc_comms::Telemetry, get_time_ms};

/// Telemetry as received by the control server
#[derive(Debug, Clone, Serialize)]
pub struct ReceivedTelemetry {
    /// When the control server received it (ms since Unix epoch)
    pub received_ms: u64,
    /// Address of the drone that sent it
    pub from: String,
    /// The telemetry itself
    pub telemetry: Telemetry,
}

/// Holds the latest telemetry and notifies subscribers when it changes
pub struct TelemetryHub {
    tx: watch::Sender<Option<ReceivedTelemetry>>,
}

impl Default for TelemetryHub {
    fn default() -> Self {
        Self {
            tx: watch::channel(None).0,
        }
    }
}

impl TelemetryHub {
    /// Latest telemetry, if any has been received
    pub fn latest(&self) -> Option<ReceivedTelemetry> {
        self.tx.borrow().clone()
    }

    /// Subscribes to telemetry updates
    pub fn subscribe(&self) -> watch::Receiver<Option<ReceivedTelemetry>> {
        self.tx.subscribe()
    }

    /// Listens for telemetry on `0.0.0.0:{port}` forever
    pub async fn listen(&self, port: u16) {
        let addr = format!("0.0.0.0:{port}");
        let socket = loop {
            match UdpSocket::bind(&addr).await {
                Ok(socket) => break socket,
                Err(e) => {
                    warn!("Failed to bind telemetry listener on {addr}: {e}. Retrying in 2 seconds...");
                    sleep(Duration::from_secs(2)).await;
                }
            }
        };
        info!("Telemetry listener on udp://{addr}");

        let mut buf = vec![0u8; 64 * 1024];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, from)) => match serde_json::from_slice::<Telemetry>(&buf[..len]) {
                    Ok(telemetry) => {
                        if self.tx.borrow().is_none() {
                            info!("Receiving telemetry from {from}");
                        }
                        self.tx.send_replace(Some(ReceivedTelemetry {
                            received_ms: get_time_ms() as u64,
                            from: from.to_string(),
                            telemetry,
                        }));
                    }
                    Err(e) => debug!("Invalid telemetry from {from}: {e}"),
                },
                Err(e) => {
                    warn!("Telemetry socket error: {e}");
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }
}
