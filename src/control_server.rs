//! Control server. This module talks to a frontend application, sends messages to
//! the UDP server and receives telemetry from the drone.
mod telemetry_hub;

use std::net::ToSocketAddrs;

use actix_files as fs;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use futures::{future::join_all, stream};
use serde_json::json;
use tokio::net::UdpSocket;
use tracing::{debug, error, info};

pub use telemetry_hub::{ReceivedTelemetry, TelemetryHub};

use crate::{
    app_data::DroneAppData, fc_comms::RcControls, get_time_ms, logger::init_logger,
    messages::Message, DroneResult,
};

/// Control server module.
pub struct ControlServer {
    /// UDP server address.
    udp_server_addr: String,
    /// Address of the control server.
    addr: String,
    /// Heartbeat interval in milliseconds (optional, only if feature is enabled).
    heartbeat_interval_ms: u64,
    /// UDP port to receive telemetry from the drone on.
    telemetry_port: u16,
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
            heartbeat_interval_ms: app_data.heartbeat_interval_ms() as u64,
            telemetry_port: app_data.telemetry_port(),
        })
    }

    /// Starts the control server.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    pub async fn start(&self) -> DroneResult<()> {
        let udp_client = web::Data::new(UdpClient::new(self.udp_server_addr.clone()).await?);

        // Spawn the loop before server starts
        let heartbeat_task: tokio::task::JoinHandle<DroneResult> = {
            let heartbeat_interval_ms = self.heartbeat_interval_ms;
            let udp_client_heartbeat_task = udp_client.clone();
            tokio::spawn(async move {
                let mut backoff_multiplier = 1;

                loop {
                    // Send message (handle error as needed)
                    if let Err(e) = udp_client_heartbeat_task.send_heartbeat().await {
                        error!("UDP send failed: {e:?}");
                        backoff_multiplier = (backoff_multiplier * 2).min(8)
                    } else {
                        backoff_multiplier = 1; // Reset exponential backoff on success
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(
                        heartbeat_interval_ms * backoff_multiplier,
                    ))
                    .await;
                }
            })
        };

        // Receives telemetry from the drone
        let telemetry_hub = web::Data::new(TelemetryHub::default());
        let telemetry_task: tokio::task::JoinHandle<DroneResult> = {
            let telemetry_hub = telemetry_hub.clone();
            let port = self.telemetry_port;
            tokio::spawn(async move {
                telemetry_hub.listen(port).await;
                Ok(())
            })
        };

        // Spins up a web server that listens for incoming HTTP requests and serves static files.
        let server_task: tokio::task::JoinHandle<DroneResult> = {
            let udp_client_clone = udp_client.clone();
            let telemetry_hub = telemetry_hub.clone();
            let addr = self.addr.clone();
            tokio::spawn(async move {
                HttpServer::new(move || {
                    App::new()
                        .route("/set-rc", web::post().to(Self::set_rc))
                        .route("/telemetry", web::get().to(Self::get_telemetry))
                        .route("/telemetry/stream", web::get().to(Self::telemetry_stream))
                        .service(fs::Files::new("/", "./static").index_file("index.html"))
                        .app_data(udp_client_clone.clone())
                        .app_data(telemetry_hub.clone())
                })
                .bind(&addr)
                .unwrap()
                .workers(1)
                .run()
                .await?;

                Ok(())
            })
        };

        #[cfg(feature = "dualsense")]
        let dualsense_controller_task = {
            use crate::dualsense_controller::DualsenseController;

            // Spins up a DualSense controller task that reads input from the controller.
            let udp_client = udp_client.clone();
            tokio::spawn(DualsenseController::new(udp_client))
        };

        let _s = join_all(vec![
            server_task,
            heartbeat_task,
            telemetry_task,
            #[cfg(feature = "dualsense")]
            dualsense_controller_task,
        ])
        .await;

        Ok(())
    }

    /// Latest telemetry with its age, or 204 if none has been received yet
    async fn get_telemetry(telemetry_hub: web::Data<TelemetryHub>) -> impl Responder {
        match telemetry_hub.latest() {
            Some(latest) => HttpResponse::Ok().json(json!({
                "age_ms": (get_time_ms() as u64).saturating_sub(latest.received_ms),
                "received_ms": latest.received_ms,
                "from": latest.from,
                "telemetry": latest.telemetry,
            })),
            None => HttpResponse::NoContent().finish(),
        }
    }

    /// Server-Sent Events stream with an event for every telemetry update
    async fn telemetry_stream(telemetry_hub: web::Data<TelemetryHub>) -> impl Responder {
        let mut rx = telemetry_hub.subscribe();
        rx.mark_changed(); // send the current value straight away

        let events = stream::unfold(rx, |mut rx| async move {
            rx.changed().await.ok()?;
            let latest = rx.borrow_and_update().clone();
            let data = serde_json::to_string(&latest).unwrap_or_else(|_| "null".into());
            let event = format!("event: telemetry\ndata: {data}\n\n");
            Some((Ok::<_, actix_web::Error>(web::Bytes::from(event)), rx))
        });

        HttpResponse::Ok()
            .content_type("text/event-stream")
            .insert_header(("Cache-Control", "no-cache"))
            .streaming(events)
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

/// UDP client for sending messages to the UDP server.
pub struct UdpClient {
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

        let (host, port_str) = server_addr
            .split_once(':')
            .expect("missing ':' in host:port");

        let port: u16 = port_str.parse().expect("port is not a valid number");

        Ok(UdpClient {
            socket,
            server_addr: (host, port)
                .to_socket_addrs()?
                .find(|a| a.is_ipv4())
                .expect(&format!("{server_addr} has no IPv4 address"))
                .to_string(),
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
    pub async fn send_heartbeat(&self) -> DroneResult<()> {
        let msg = serde_json::to_string(&Message::Heartbeat)?;

        self.send(msg.as_bytes()).await
    }
}
