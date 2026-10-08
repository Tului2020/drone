//! Control server. This module talks to a frontend application, sends messages to
//! the UDP server and receives telemetry from the drone.
mod camera;
mod telemetry_hub;

use std::{net::ToSocketAddrs, path::PathBuf};

use actix_files as fs;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use futures::{future::join_all, stream, Stream, StreamExt};
use serde::Serialize;
use serde_json::json;
use tokio::{net::UdpSocket, sync::watch};
use tracing::{debug, error, info};

pub use camera::CameraHub;
pub use telemetry_hub::{ReceivedTelemetry, TelemetryHub};

#[cfg(feature = "dualsense")]
use crate::dualsense_controller::{settings::ControllerSettings, ControllerHub};

/// File (next to the config file) where controller sensitivity settings are saved
const CONTROLLER_SETTINGS_FILE: &str = "controller_settings.json";

use crate::{
    app_data::DroneAppData, fc_comms::RcControls, get_time_ms, logger::init_logger,
    messages::Message, DroneResult, HEARTBEAT_INTERVAL_MS,
};

/// Control server module.
pub struct ControlServer {
    /// UDP server address.
    udp_server_addr: String,
    /// Address of the control server.
    addr: String,
    /// UDP port to receive telemetry from the drone on.
    telemetry_port: u16,
    /// The drone's camera stream address.
    camera_stream_addr: String,
    /// Where controller sensitivity settings are saved.
    #[cfg_attr(not(feature = "dualsense"), allow(dead_code))]
    controller_settings_path: PathBuf,
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
            telemetry_port: app_data.telemetry_port(),
            camera_stream_addr: app_data.camera_stream_addr(),
            controller_settings_path: Self::controller_settings_path(app_data_file_path),
        })
    }

    /// Path of the controller settings file, next to the config file
    pub fn controller_settings_path(app_data_file_path: &str) -> PathBuf {
        PathBuf::from(app_data_file_path).with_file_name(CONTROLLER_SETTINGS_FILE)
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
                        HEARTBEAT_INTERVAL_MS * backoff_multiplier,
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

        // Relays the drone's camera stream
        let camera_hub = web::Data::new(CameraHub::new(self.camera_stream_addr.clone()));
        let camera_task: tokio::task::JoinHandle<DroneResult> = {
            let camera_hub = camera_hub.clone();
            tokio::spawn(async move {
                camera_hub.run().await;
                Ok(())
            })
        };

        #[cfg(feature = "dualsense")]
        let controller_hub =
            web::Data::new(ControllerHub::new(self.controller_settings_path.clone()));

        // Spins up a web server that listens for incoming HTTP requests and serves static files.
        let server_task: tokio::task::JoinHandle<DroneResult> = {
            let udp_client_clone = udp_client.clone();
            let telemetry_hub = telemetry_hub.clone();
            let camera_hub = camera_hub.clone();
            #[cfg(feature = "dualsense")]
            let controller_hub = controller_hub.clone();
            let addr = self.addr.clone();
            tokio::spawn(async move {
                HttpServer::new(move || {
                    let app = App::new()
                        // Make browsers revalidate the UI files so an updated dashboard is
                        // never shadowed by a stale cached copy
                        .wrap(middleware::DefaultHeaders::new().add(("Cache-Control", "no-cache")))
                        .route("/set-rc", web::post().to(Self::set_rc))
                        .route("/reboot-fc", web::post().to(Self::reboot_fc))
                        .route("/telemetry", web::get().to(Self::get_telemetry))
                        .route("/telemetry/stream", web::get().to(Self::event_stream))
                        .route("/camera/status", web::get().to(Self::camera_status))
                        .route("/camera/stream", web::get().to(Self::camera_stream))
                        .app_data(camera_hub.clone())
                        .app_data(udp_client_clone.clone())
                        .app_data(telemetry_hub.clone());

                    #[cfg(feature = "dualsense")]
                    let app = app
                        .route(
                            "/controller-settings",
                            web::get().to(Self::get_controller_settings),
                        )
                        .route(
                            "/controller-settings",
                            web::put().to(Self::put_controller_settings),
                        )
                        .app_data(controller_hub.clone());

                    app.service(fs::Files::new("/", "./static").index_file("index.html"))
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
            tokio::spawn(DualsenseController::new(udp_client, controller_hub))
        };

        let _s = join_all(vec![
            server_task,
            heartbeat_task,
            telemetry_task,
            camera_task,
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

    /// Server-Sent Events stream: a `telemetry` event for every drone update and, with a
    /// DualSense attached, a `controller` event for every controller tick
    async fn event_stream(
        telemetry_hub: web::Data<TelemetryHub>,
        req: HttpRequest,
    ) -> impl Responder {
        let events = sse_events("telemetry", telemetry_hub.subscribe()).boxed();

        #[cfg(feature = "dualsense")]
        let events = match req.app_data::<web::Data<ControllerHub>>() {
            Some(hub) => stream::select(events, sse_events("controller", hub.subscribe())).boxed(),
            None => events,
        };
        #[cfg(not(feature = "dualsense"))]
        let _ = req;

        HttpResponse::Ok()
            .content_type("text/event-stream")
            .insert_header(("Cache-Control", "no-cache"))
            .streaming(events)
    }

    /// Whether the control server is connected to the drone's camera
    async fn camera_status(camera_hub: web::Data<CameraHub>) -> impl Responder {
        HttpResponse::Ok().json(json!({
            "connected": camera_hub.is_connected(),
            "addr": camera_hub.addr(),
        }))
    }

    /// Length-prefixed H.264 NAL units (`[u32 BE length][NAL]`), starting at the next keyframe.
    /// A client that falls behind is resynced at the following keyframe.
    async fn camera_stream(camera_hub: web::Data<CameraHub>) -> impl Responder {
        use tokio::sync::broadcast::error::RecvError;

        let nals = stream::unfold(
            (camera_hub.subscribe(), false),
            |(mut rx, mut synced)| async move {
                loop {
                    match rx.recv().await {
                        Ok(nal) => {
                            synced = synced || camera::nal_type(&nal) == Some(camera::NAL_TYPE_SPS);
                            if synced {
                                let framed = camera::frame_for_browser(&nal);
                                return Some((Ok::<_, actix_web::Error>(framed), (rx, synced)));
                            }
                        }
                        Err(RecvError::Lagged(skipped)) => {
                            debug!("Camera client lagged by {skipped} NAL units, resyncing");
                            synced = false;
                        }
                        Err(RecvError::Closed) => return None,
                    }
                }
            },
        );

        HttpResponse::Ok()
            .content_type("application/octet-stream")
            .streaming(nals)
    }

    #[cfg(feature = "dualsense")]
    async fn get_controller_settings(hub: web::Data<ControllerHub>) -> impl Responder {
        HttpResponse::Ok().json(hub.settings())
    }

    #[cfg(feature = "dualsense")]
    async fn put_controller_settings(
        hub: web::Data<ControllerHub>,
        settings: web::Json<ControllerSettings>,
    ) -> impl Responder {
        match hub.update_settings(settings.into_inner()) {
            Ok(()) => HttpResponse::Ok().json(hub.settings()),
            Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
        }
    }

    /// Asks the drone to reboot the FC. The drone only does it if the FC reports disarmed;
    /// the outcome comes back as `fc_reboot` in the telemetry.
    async fn reboot_fc(udp_client: web::Data<UdpClient>) -> impl Responder {
        info!("Requesting FC reboot");
        match udp_client.send_reboot_fc().await {
            Ok(_) => HttpResponse::Accepted().finish(),
            Err(e) => {
                error!("{e}");
                HttpResponse::InternalServerError().finish()
            }
        }
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

    /// Asks the drone to reboot the flight controller.
    pub async fn send_reboot_fc(&self) -> DroneResult<()> {
        let msg = serde_json::to_string(&Message::RebootFc)?;

        self.send(msg.as_bytes()).await
    }

    /// Sends a heartbeat message to the UDP server.
    pub async fn send_heartbeat(&self) -> DroneResult<()> {
        let msg = serde_json::to_string(&Message::Heartbeat)?;

        self.send(msg.as_bytes()).await
    }
}

/// Turns a watch channel into a stream of Server-Sent Events named `event`, starting with the current value
fn sse_events<T>(
    event: &'static str,
    mut rx: watch::Receiver<T>,
) -> impl Stream<Item = Result<web::Bytes, actix_web::Error>> + 'static
where
    T: Serialize + Send + Sync + 'static,
{
    rx.mark_changed();
    stream::unfold(rx, move |mut rx| async move {
        rx.changed().await.ok()?;
        let data =
            serde_json::to_string(&*rx.borrow_and_update()).unwrap_or_else(|_| "null".into());
        Some((
            Ok(web::Bytes::from(format!(
                "event: {event}\ndata: {data}\n\n"
            ))),
            rx,
        ))
    })
}
