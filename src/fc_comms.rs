//! Module for FC communications
pub mod crsf;
pub mod msp;
mod rc_controls;
pub mod telemetry;

use std::sync::{atomic::AtomicBool, Arc, Mutex};
#[cfg(any(feature = "real", feature = "udp_server"))]
use std::thread::spawn as thread_spawn;
#[cfg(feature = "real")]
use std::{sync::atomic::Ordering, thread::sleep, time::Duration};

pub use rc_controls::RcControls;
pub use telemetry::Telemetry;
#[cfg(feature = "real")]
use serialport::SerialPort;
use tracing::debug;
#[cfg(feature = "real")]
use tracing::{error, info};

#[cfg(feature = "udp_server")]
use crate::udp_server::UdpServer;
use crate::{app_data::DroneAppData, DroneResult};

#[cfg(feature = "real")]
use crsf::{build_frame, frame_type, pack_rc, us_to_crsf};
#[cfg(feature = "real")]
use telemetry::TelemetryDecoder;

/// Poll one MSP command (motors / status / IMU) every N RC frames (N × 20ms)
#[cfg(feature = "real")]
const MSP_POLL_EVERY_N_FRAMES: u32 = 5;

/// FC communications
pub struct FcComms {
    /// RC controls
    rc_controls: Arc<Mutex<RcControls>>,
    /// Latest telemetry received from the FC
    telemetry: Arc<Mutex<Telemetry>>,
}

impl FcComms {
    /// Create a new instance of the FC communications
    pub fn new(app_data: &DroneAppData, running: Arc<AtomicBool>) -> DroneResult<Self> {
        debug!("Creating FC communications {app_data:?} {running:?}");

        let rc_controls = Arc::new(Mutex::new(RcControls::default()));
        let telemetry = Arc::new(Mutex::new(Telemetry::default()));

        #[cfg(feature = "udp_server")]
        // Create a UDP server that listens for RC data and sets the "rc_controls"
        {
            let heatbeat_interval_ms = app_data.heartbeat_interval_ms();
            let (rc_controls_clone, running_clone) = (rc_controls.clone(), running.clone());
            thread_spawn(move || {
                UdpServer::new(rc_controls_clone, running_clone, heatbeat_interval_ms)
            });
        }

        #[cfg(feature = "real")]
        // Create a serial port that connects to the FC and sends "rc_controls" every 20ms
        {
            // Thread that sends RC data to the FC every 20ms to prevent the FC from going into failsafe mode
            let rc_controls_clone = rc_controls.clone();
            let telemetry_clone = telemetry.clone();
            let port_name = app_data.fc_port_name().to_string();
            let baud_rate = app_data.fc_baud_rate();

            thread_spawn(move || {
                // Initial connection with retry and timeout
                let mut port = {
                    let start_time = std::time::Instant::now();
                    let timeout_duration = Duration::from_secs(30); // 30 second timeout

                    loop {
                        if !running.load(Ordering::SeqCst) {
                            debug!("Stopping RC data thread during initial connection");
                            return;
                        }

                        if start_time.elapsed() > timeout_duration {
                            error!(
                                "Initial connection timeout after {} seconds",
                                timeout_duration.as_secs()
                            );
                            return;
                        }

                        match Self::open_port(&port_name, baud_rate) {
                            Ok(p) => {
                                info!("Initial connection successful to {port_name}");
                                break p;
                            }
                            Err(e) => {
                                error!("Initial connection attempt failed: {e}. Retrying in 2 seconds...");
                                sleep(Duration::from_millis(2000));
                            }
                        }
                    }
                };

                let mut decoder = TelemetryDecoder::new(MSP_POLL_EVERY_N_FRAMES);
                let mut read_buf = [0u8; 256];

                loop {
                    // ---------- build RC frame ----------
                    let chans_us = { rc_controls_clone.lock().unwrap().chans_us() };
                    let chans: Vec<u16> = chans_us.iter().copied().map(us_to_crsf).collect();
                    let payload = pack_rc(&chans);
                    let mut frame = build_frame(frame_type::RC_CHANNELS_PACKED, &payload);
                    if let Some(msp_request) = decoder.next_msp_request() {
                        frame.extend_from_slice(&msp_request);
                    }

                    // ---------- try to write ----------
                    match port.write_all(&frame) {
                        Ok(_) => {
                            // normal path
                            Self::read_telemetry(
                                port.as_mut(),
                                &mut decoder,
                                &mut read_buf,
                                &telemetry_clone,
                            );

                            if !running.load(Ordering::SeqCst) {
                                debug!("Stopping RC data thread");
                                break;
                            }
                            sleep(Duration::from_millis(20));
                        }

                        Err(e) => {
                            error!("Serial write failed ({e}). Dropping handle …");
                            drop(port); // closes the FD immediately

                            // ---------- reconnect loop ----------
                            loop {
                                debug!("Attempting to reconnect to FC on {port_name} at {baud_rate} baud");

                                if !running.load(Ordering::SeqCst) {
                                    debug!("Stopping RC data thread while reconnecting");
                                    return;
                                }

                                match Self::open_port(&port_name, baud_rate) {
                                    Ok(p) => {
                                        info!("Re-connected to {port_name}");
                                        port = p;
                                        break; // resume main loop
                                    }
                                    Err(err) => {
                                        info!("Reconnect attempt failed: {err}");
                                        sleep(Duration::from_millis(2000));
                                    }
                                }
                            }
                        }
                    }
                }
            });
        };

        Ok(Self {
            rc_controls,
            telemetry,
        })
    }

    /// Reads whatever the FC has sent since the last call and decodes it into `telemetry`
    #[cfg(feature = "real")]
    fn read_telemetry(
        port: &mut dyn SerialPort,
        decoder: &mut TelemetryDecoder,
        read_buf: &mut [u8],
        telemetry: &Mutex<Telemetry>,
    ) {
        let available = match port.bytes_to_read() {
            Ok(n) if n > 0 => n as usize,
            Ok(_) => return,
            Err(e) => {
                debug!("Failed to query FC serial buffer: {e}");
                return;
            }
        };

        let to_read = available.min(read_buf.len());
        match port.read(&mut read_buf[..to_read]) {
            Ok(n) => decoder.feed(
                &read_buf[..n],
                &mut telemetry.lock().unwrap(),
                crate::get_time_ms() as u64,
            ),
            Err(e) => debug!("Failed to read FC telemetry: {e}"),
        }
    }

    /// Latest telemetry received from the FC
    pub fn telemetry(&self) -> Arc<Mutex<Telemetry>> {
        self.telemetry.clone()
    }

    #[cfg(feature = "real")]
    fn open_port(port_name: &str, baud_rate: u32) -> DroneResult<Box<dyn SerialPort>> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(1000))
            .open()?;
        info!("Serial port opened: {port_name} at {baud_rate} baud");
        Ok(port)
    }

    /// Send RC data
    pub fn set_rc_controls(
        &mut self,
        roll: Option<u16>,
        pitch: Option<u16>,
        thr: Option<u16>,
        yaw: Option<u16>,
        aux1: Option<u16>,
        aux2: Option<u16>,
        aux3: Option<u16>,
        aux4: Option<u16>,
    ) {
        let mut rc_controls = self.rc_controls.lock().unwrap();

        if let Some(roll) = roll {
            rc_controls.roll = roll;
        }
        if let Some(pitch) = pitch {
            rc_controls.pitch = pitch;
        }
        if let Some(yaw) = yaw {
            rc_controls.yaw = yaw;
        }
        if let Some(thr) = thr {
            rc_controls.thr = thr;
        }
        if let Some(aux1) = aux1 {
            rc_controls.aux1 = aux1;
        }
        if let Some(aux2) = aux2 {
            rc_controls.aux2 = aux2;
        }
        if let Some(aux3) = aux3 {
            rc_controls.aux3 = aux3;
        }
        if let Some(aux4) = aux4 {
            rc_controls.aux4 = aux4;
        }
    }
}
