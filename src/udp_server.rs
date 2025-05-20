//! Remote server using UDP
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use tokio::{net::UdpSocket, runtime::Builder};
use tracing::{debug, info};

use crate::fc_comms::RcControls;

/// Remote server using UDP
pub struct UdpServer;

impl UdpServer {
    /// Create a new instance of the remote server
    pub fn new(_rc_controls: Arc<Mutex<RcControls>>, running: Arc<AtomicBool>) {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();

        rt.block_on(async {
            // async code here
            let socket = UdpSocket::bind("0.0.0.0:8080").await.unwrap();
            info!("UDP server listening on 0.0.0.0:8080");

            let mut buf = [0u8; 1024];
            loop {
                let _ = socket.recv_from(&mut buf).await.unwrap();
                let s = std::str::from_utf8(&buf).unwrap();
                info!("Received: {}", s);

                if !running.load(Ordering::SeqCst) {
                    debug!("Shutting down remote server");
                    break;
                }
            }
        });
    }
}
