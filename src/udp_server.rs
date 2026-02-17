//! Remote server using UDP
use std::{
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
use crate::{fc_comms::RcControls, messages::Message};

/// Remote server using UDP
pub struct UdpServer;

impl UdpServer {
    /// Create a new instance of the remote server
    pub fn new(
        rc_controls: Arc<Mutex<RcControls>>,
        running: Arc<AtomicBool>,
        heartbeat_interval_ms: u128,
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
