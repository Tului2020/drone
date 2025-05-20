//! Control server. This module talks to a frontend application and sends messages to
//! the UDP server.
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use tokio::net::UdpSocket;
use tracing::{debug, error};

use crate::{app_data::DroneAppData, fc_comms::RcControls, messages::Message, DroneResult};

/// Control server module.
pub struct ControlServer {
    /// UDP server address.
    udp_server_addr: String,
    /// Address of the control server.
    addr: String,
}

impl ControlServer {
    /// Creates a new instance of the control server.
    ///
    /// # Arguments
    ///
    /// * `app_data` - Drone application data containing configuration information.
    ///
    /// # Returns
    ///
    /// A new instance of `ControlServer`.
    pub fn new(app_data: &DroneAppData) -> Self {
        ControlServer {
            udp_server_addr: app_data.udp_server_addr().to_string(),
            addr: format!("127.0.0.1:{}", app_data.control_server_port())
                .parse()
                .unwrap(),
        }
    }

    /// Starts the control server.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    pub async fn start(&self) -> DroneResult<()> {
        let addr = &self.addr;

        let udp_client = UdpClient::new(self.udp_server_addr.clone()).await?;
        let udp_client = web::Data::new(udp_client); // 👈 Not Arc

        HttpServer::new(move || {
            App::new()
                .route("/set-rc", web::post().to(Self::set_rc))
                .app_data(udp_client.clone())
        })
        .bind(addr)?
        .run()
        .await?;

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

    pub async fn send_rc(&self, rc_controls: RcControls) -> DroneResult<()> {
        let msg = serde_json::to_string(&Message::SetRc(rc_controls))?;

        self.send(msg.as_bytes()).await
    }
}
