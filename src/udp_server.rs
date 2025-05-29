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
use tracing::{debug, info};

#[cfg(feature = "heartbeat")]
use crate::get_time_ms;
use crate::{fc_comms::RcControls, messages::Message};

/// Remote server using UDP
pub struct UdpServer;

impl UdpServer {
    /// Create a new instance of the remote server
    pub fn new(
        rc_controls: Arc<Mutex<RcControls>>,
        running: Arc<AtomicBool>,
        #[cfg(feature = "heartbeat")] heartbeat_interval_ms: u128,
    ) -> Self {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();

        rt.block_on(async {
            // async code here
            let socket = UdpSocket::bind("0.0.0.0:8080").await.unwrap();
            info!("UDP server listening on 0.0.0.0:8080");

            let mut tasks = vec![];

            // Checks heartbeat every heartbeat_interval_ms milliseconds and resets the RC controls if no heartbeat is received
            #[cfg(feature = "heartbeat")]
            let last_heartbeat_timestamp = {
                let last_heartbeat_timestamp = Arc::new(Mutex::new(get_time_ms()));
                let last_heartbeat_timestamp_clone = last_heartbeat_timestamp.clone();
                let rc_controls_clone = rc_controls.clone();

                let heartbeat_checker_task = tokio::spawn(async move {
                    loop {
                        let temp_last_heartbeat_timestamp =
                            { *last_heartbeat_timestamp.lock().unwrap() };

                        if get_time_ms() - temp_last_heartbeat_timestamp > heartbeat_interval_ms {
                            let mut rc_controls = rc_controls_clone.lock().unwrap();
                            rc_controls.reset();
                        }

                        sleep(Duration::from_millis(heartbeat_interval_ms as u64)).await
                    }
                });

                tasks.push(heartbeat_checker_task);

                last_heartbeat_timestamp_clone
            };

            // Listens for incoming UDP packets and processes them
            let rc_controls_clone = rc_controls.clone();
            let listener_task = tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                loop {
                    let (len, _) = socket.recv_from(&mut buf).await.unwrap();
                    let s = std::str::from_utf8(&buf[..len]).unwrap();
                    match serde_json::from_str::<Message>(s).unwrap() {
                        Message::SetRc(incoming_rc_controls) => {
                            rc_controls_clone
                                .lock()
                                .unwrap()
                                .update(&incoming_rc_controls);
                        }
                        #[cfg(feature = "heartbeat")]
                        Message::Heartbeat => {
                            debug!("Received heartbeat");
                            let mut temp_last_heartbeat_timestamp =
                                last_heartbeat_timestamp.lock().unwrap();
                            *temp_last_heartbeat_timestamp = get_time_ms();
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
