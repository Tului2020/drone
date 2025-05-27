//! Control server. This module talks to a frontend application and sends messages to
//! the UDP server.
use actix_files as fs;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use futures::future::join_all;
use tokio::net::UdpSocket;
use tracing::{debug, error, info};

use crate::{
    app_data::DroneAppData, fc_comms::RcControls, logger::init_logger, messages::Message,
    DroneResult,
};

/// Control server module.
pub struct ControlServer {
    /// UDP server address.
    udp_server_addr: String,
    /// Address of the control server.
    addr: String,
    /// Heartbeat interval in milliseconds (optional, only if feature is enabled).
    #[cfg(feature = "heartbeat")]
    heartbeat_interval_ms: u64,
}

impl ControlServer {
    /// Creates a new instance of the control server.
    ///
    /// # Arguments
    ///
    /// * `app_data_file_path` - Path to the application data file.
    /// * `running` - An `Arc<AtomicBool>` indicating whether the server is running.
    ///
    /// # Returns
    ///
    /// A new instance of `ControlServer`.
    pub fn new(app_data_file_path: &str) -> DroneResult<Self> {
        // Load configuration
        let app_data = DroneAppData::load_from_file(app_data_file_path);

        init_logger(&app_data.log_level().clone().into())?;
        info!("Starting control server...");

        Ok(ControlServer {
            udp_server_addr: app_data.udp_server_addr().to_string(),
            addr: app_data.control_server_address().to_string(),
            #[cfg(feature = "heartbeat")]
            heartbeat_interval_ms: app_data.heartbeat_interval_ms() as u64,
        })
    }

    /// Starts the control server.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    pub async fn start(&self) -> DroneResult<()> {
        let udp_client = web::Data::new(UdpClient::new(self.udp_server_addr.clone()).await?);

        let mut tasks = vec![];

        #[cfg(feature = "heartbeat")]
        // Sends a heartbeat message at regular intervals to the UDP server.
        {
            let heartbeat_interval_ms = self.heartbeat_interval_ms;
            let udp_client_clone = udp_client.clone();
            // Spawn the loop before server starts
            let heartbeat_task = tokio::spawn(async move {
                loop {
                    // Send message (handle error as needed)
                    if let Err(e) = udp_client_clone.send_heartbeat().await {
                        error!("UDP send failed: {e:?}");
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(heartbeat_interval_ms))
                        .await;
                }
            });

            tasks.push(heartbeat_task);
        };

        // Spins up a web server that listens for incoming HTTP requests and serves static files.
        let server_task = {
            let addr = self.addr.clone();
            tokio::spawn(async move {
                HttpServer::new(move || {
                    App::new()
                        .route("/set-rc", web::post().to(Self::set_rc))
                        .service(fs::Files::new("/", "./static").index_file("index.html"))
                        .app_data(udp_client.clone())
                })
                .bind(&addr)
                .unwrap()
                .workers(1)
                .run()
                .await
            })
        };
        tasks.push(server_task);

        let _s = join_all(tasks).await;

        Ok(())
    }

    async fn set_rc(
        rc_controls: web::Json<RcControls>,
        udp_client: web::Data<UdpClient>,
    ) -> impl Responder {
        let rc_controls = rc_controls.into_inner();
        debug!("Received RC controls: {rc_controls}");

        match udp_client.send_rc(rc_controls).await {
            Ok(_) => HttpResponse::Ok(),
            Err(e) => {
                error!("{e}");
                HttpResponse::InternalServerError()
            }
        }
    }
}

struct UdpClient {
    socket: UdpSocket,
    server_addr: String,
}

impl UdpClient {
    /// Creates a new instance of the UDP client.
    ///
    /// # Arguments
    ///
    /// * `server_addr` - The address of the UDP server.
    ///
    /// # Returns
    ///
    /// A new instance of `UdpClient`.
    pub async fn new(server_addr: String) -> DroneResult<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;

        Ok(UdpClient {
            socket,
            server_addr,
        })
    }

    /// Sends a message to the UDP server.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to send.
    async fn send(&self, msg: &[u8]) -> DroneResult<()> {
        self.socket.send_to(msg, &self.server_addr).await?;
        Ok(())
    }

    /// Sends RC controls to the UDP server.
    ///
    /// # Arguments
    ///
    /// * `rc_controls` - The RC controls to send.
    pub async fn send_rc(&self, rc_controls: RcControls) -> DroneResult<()> {
        let msg = serde_json::to_string(&Message::SetRc(rc_controls))?;

        self.send(msg.as_bytes()).await
    }

    /// Sends a heartbeat message to the UDP server.
    #[cfg(feature = "heartbeat")]
    pub async fn send_heartbeat(&self) -> DroneResult<()> {
        let msg = serde_json::to_string(&Message::Heartbeat)?;

        self.send(msg.as_bytes()).await
    }
}
