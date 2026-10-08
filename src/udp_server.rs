//! Remote server using UDP
use std::{
    net::IpAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use futures::future::join_all;
use tokio::{net::UdpSocket, runtime::Builder, time::sleep};
use tracing::{debug, info, warn};

use crate::get_time_ms;
use crate::{
    fc_comms::{RcControls, Telemetry},
    messages::Message,
};

/// Stop sending telemetry if the control server has been silent for this long
const CONTROL_SERVER_TIMEOUT_MS: u128 = 5_000;

/// Remote server using UDP
pub struct UdpServer;

/// Telemetry streaming settings
pub struct TelemetryStream {
    /// Latest telemetry from the FC
    pub telemetry: Arc<Mutex<Telemetry>>,
    /// UDP port the control server listens on for telemetry
    pub port: u16,
    /// Send interval in milliseconds
    pub interval_ms: u64,
}

impl UdpServer {
    /// Create a new instance of the remote server
    ///
    /// Telemetry is sent to the IP of whoever last sent us a valid message (the control server),
    /// so the drone does not need to know the ground station's address.
    pub fn new(
        rc_controls: Arc<Mutex<RcControls>>,
        running: Arc<AtomicBool>,
        heartbeat_interval_ms: u128,
        telemetry_stream: TelemetryStream,
    ) -> Self {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();

        rt.block_on(async {
            // Initial socket binding with retry logic
            let socket = {
                let start_time = std::time::Instant::now();
                let timeout_duration = Duration::from_secs(30);

                loop {
                    if !running.load(Ordering::SeqCst) {
                        debug!("Stopping UDP server during initial binding");
                        return;
                    }

                    if start_time.elapsed() > timeout_duration {
                        warn!(
                            "UDP socket binding timeout after {} seconds",
                            timeout_duration.as_secs()
                        );
                        return;
                    }

                    match UdpSocket::bind("0.0.0.0:8080").await {
                        Ok(s) => {
                            info!("UDP server listening on 0.0.0.0:8080");
                            break s;
                        }
                        Err(e) => {
                            warn!("UDP socket binding failed: {e}. Retrying in 2 seconds...");
                            sleep(Duration::from_millis(2000)).await;
                        }
                    }
                }
            };

            let mut tasks = vec![];

            // IP of the control server and when we last heard from it
            let control_server: Arc<Mutex<Option<(IpAddr, u128)>>> = Arc::new(Mutex::new(None));

            // Streams the latest telemetry to the control server
            {
                let control_server = control_server.clone();
                let rc_controls = rc_controls.clone();
                let running = running.clone();
                let TelemetryStream {
                    telemetry,
                    port,
                    interval_ms,
                } = telemetry_stream;

                tasks.push(tokio::spawn(async move {
                    let socket = match UdpSocket::bind("0.0.0.0:0").await {
                        Ok(s) => s,
                        Err(e) => {
                            warn!("Failed to bind telemetry socket: {e}");
                            return;
                        }
                    };

                    while running.load(Ordering::SeqCst) {
                        sleep(Duration::from_millis(interval_ms)).await;

                        let Some((ip, last_seen_ms)) = *control_server.lock().unwrap() else {
                            continue;
                        };
                        if get_time_ms() - last_seen_ms > CONTROL_SERVER_TIMEOUT_MS {
                            continue;
                        }

                        let snapshot = {
                            let mut snapshot = telemetry.lock().unwrap().clone();
                            snapshot.rc_sent = Some(*rc_controls.lock().unwrap());
                            snapshot
                        };
                        match serde_json::to_vec(&snapshot) {
                            Ok(bytes) => {
                                if let Err(e) = socket.send_to(&bytes, (ip, port)).await {
                                    debug!("Failed to send telemetry to {ip}:{port}: {e}");
                                }
                            }
                            Err(e) => warn!("Failed to serialize telemetry: {e}"),
                        }
                    }
                }));
            }

            // Checks heartbeat every heartbeat_interval_ms milliseconds and resets the RC controls if no heartbeat is received
            let last_heartbeat_timestamp = {
                let last_heartbeat_timestamp = Arc::new(Mutex::new(get_time_ms()));
                let last_heartbeat_timestamp_clone = last_heartbeat_timestamp.clone();
                let rc_controls_clone = rc_controls.clone();

                let heartbeat_checker_task = tokio::spawn(async move {
                    loop {
                        let temp_last_heartbeat_timestamp =
                            { *last_heartbeat_timestamp.lock().unwrap() };

                        if get_time_ms() - temp_last_heartbeat_timestamp > heartbeat_interval_ms * 3
                        {
                            let mut rc_controls = rc_controls_clone.lock().unwrap();
                            rc_controls.reset();
                        }

                        sleep(Duration::from_millis(heartbeat_interval_ms as u64)).await
                    }
                });

                tasks.push(heartbeat_checker_task);

                last_heartbeat_timestamp_clone
            };

            // Listens for incoming UDP packets and processes them with reconnect logic
            let rc_controls_clone = rc_controls.clone();
            let running_clone = running.clone();
            let listener_task = tokio::spawn(async move {
                let mut socket = socket;
                let mut buf = [0u8; 1024];

                loop {
                    if !running_clone.load(Ordering::SeqCst) {
                        debug!("Stopping UDP listener");
                        break;
                    }

                    match socket.recv_from(&mut buf).await {
                        Ok((len, addr)) => {
                            // Parse the received message
                            match std::str::from_utf8(&buf[..len]) {
                                Ok(raw_string) => {
                                    if let Ok(decoded_message) =
                                        serde_json::from_str::<Message>(raw_string)
                                    {
                                        *control_server.lock().unwrap() =
                                            Some((addr.ip(), get_time_ms()));

                                        // Decode the message and process it
                                        match decoded_message {
                                            Message::SetRc(incoming_rc_controls) => {
                                                rc_controls_clone
                                                    .lock()
                                                    .unwrap()
                                                    .update(&incoming_rc_controls);
                                            }
                                            Message::Heartbeat => {
                                                debug!("Received heartbeat");
                                                let mut temp_last_heartbeat_timestamp =
                                                    last_heartbeat_timestamp.lock().unwrap();
                                                *temp_last_heartbeat_timestamp = get_time_ms();
                                            }
                                        }

                                        // Send an ACK response
                                        if let Err(e) = socket.send_to(b"ACK", addr).await {
                                            warn!("Failed to send ACK: {e}");
                                        }
                                    } else {
                                        warn!("Received invalid message: {raw_string}");
                                    }
                                }
                                Err(e) => {
                                    warn!("Received invalid UTF-8 data: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            warn!("UDP socket error: {e}. Attempting to reconnect...");

                            // Reconnection loop
                            loop {
                                if !running_clone.load(Ordering::SeqCst) {
                                    debug!("Stopping UDP server during reconnection");
                                    return;
                                }

                                match UdpSocket::bind("0.0.0.0:8080").await {
                                    Ok(new_socket) => {
                                        info!("UDP socket reconnected successfully");
                                        socket = new_socket;
                                        break; // Resume main loop
                                    }
                                    Err(err) => {
                                        warn!("UDP reconnect attempt failed: {err}");
                                        sleep(Duration::from_millis(2000)).await;
                                    }
                                }
                            }
                        }
                    }
                }
            });
            tasks.push(listener_task);

            // Task to check if the server is still running
            let fail_check_task = tokio::spawn(async move {
                loop {
                    if !running.load(Ordering::SeqCst) {
                        debug!("Shutting down remote server");
                        break;
                    }
                    sleep(Duration::from_millis(500)).await;
                }
            });
            tasks.push(fail_check_task);

            let _ = join_all(tasks).await;
        });

        Self
    }
}
