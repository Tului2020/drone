//! Relays the drone's H.264 camera stream (raw Annex B over TCP, see
//! `raspi_services/live_camera.service`) to browsers.
//!
//! The stream is split into NAL units and each browser receives them length-prefixed
//! (`[u32 big-endian length][NAL incl. start code]`), starting at an SPS so the decoder
//! always begins at a keyframe.
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use bytes::{Bytes, BytesMut};
use tokio::{
    io::AsyncReadExt,
    net::{lookup_host, TcpStream},
    sync::broadcast,
    time::{sleep, timeout},
};
use tracing::{debug, info, warn};

/// H.264 NAL unit type of a sequence parameter set (sent before every keyframe with `--inline`)
pub const NAL_TYPE_SPS: u8 = 7;
/// Drop buffered data if no NAL boundary shows up within this many bytes
const MAX_BUFFERED_BYTES: usize = 4 * 1024 * 1024;
/// NAL units buffered per subscriber before it lags (and resyncs at the next keyframe)
const BROADCAST_CAPACITY: usize = 512;
const RECONNECT_DELAY: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Connects to the drone's camera and fans the stream out to subscribers
pub struct CameraHub {
    addr: String,
    tx: broadcast::Sender<Bytes>,
    connected: AtomicBool,
}

impl CameraHub {
    /// `addr` is the drone's camera stream, e.g. `drone.local:2222`
    pub fn new(addr: String) -> Self {
        Self {
            addr,
            tx: broadcast::channel(BROADCAST_CAPACITY).0,
            connected: AtomicBool::new(false),
        }
    }

    /// Address of the camera stream
    pub fn addr(&self) -> &str {
        &self.addr
    }

    /// Whether the control server is currently connected to the camera
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Subscribes to complete NAL units (each starting with its start code)
    pub fn subscribe(&self) -> broadcast::Receiver<Bytes> {
        self.tx.subscribe()
    }

    /// Connects to the camera and relays its stream forever, reconnecting on errors
    pub async fn run(&self) {
        loop {
            match self.connect().await {
                Ok(stream) => {
                    info!("Camera stream connected: {}", self.addr);
                    self.connected.store(true, Ordering::SeqCst);
                    if let Err(e) = self.relay(stream).await {
                        warn!("Camera stream error: {e}");
                    } else {
                        warn!("Camera stream closed by {}", self.addr);
                    }
                    self.connected.store(false, Ordering::SeqCst);
                }
                Err(e) => debug!("Camera {} not reachable: {e}", self.addr),
            }
            sleep(RECONNECT_DELAY).await;
        }
    }

    async fn connect(&self) -> std::io::Result<TcpStream> {
        // Prefer IPv4: mDNS names often also resolve to link-local IPv6 addresses that don't route
        let addrs: Vec<_> = lookup_host(&self.addr).await?.collect();
        let addr = addrs
            .iter()
            .find(|a| a.is_ipv4())
            .or(addrs.first())
            .copied()
            .ok_or_else(|| std::io::Error::other("no address"))?;
        let stream = timeout(CONNECT_TIMEOUT, TcpStream::connect(addr))
            .await
            .map_err(|_| std::io::Error::other("connect timed out"))??;
        stream.set_nodelay(true)?;
        Ok(stream)
    }

    async fn relay(&self, mut stream: TcpStream) -> std::io::Result<()> {
        let mut buf = BytesMut::with_capacity(256 * 1024);
        loop {
            if stream.read_buf(&mut buf).await? == 0 {
                return Ok(());
            }
            for nal in split_nal_units(&mut buf) {
                // No subscribers is fine
                let _ = self.tx.send(nal);
            }
            if buf.len() > MAX_BUFFERED_BYTES {
                warn!(
                    "Camera stream has no NAL boundaries, dropping {} bytes",
                    buf.len()
                );
                buf.clear();
            }
        }
    }
}

/// Positions of every `00 00 01` start code in `buf`
fn start_codes(buf: &[u8]) -> Vec<usize> {
    buf.windows(3)
        .enumerate()
        .filter(|(_, w)| *w == [0, 0, 1])
        .map(|(i, _)| i)
        .collect()
}

/// Removes every complete NAL unit from `buf` (each returned with its start code).
/// Bytes before the first start code are discarded, and the last (possibly incomplete)
/// NAL unit stays in `buf` until the next start code arrives.
pub fn split_nal_units(buf: &mut BytesMut) -> Vec<Bytes> {
    let starts = start_codes(buf);
    let Some(&first) = starts.first() else {
        // keep the tail in case a start code is split across reads
        let keep = buf.len().min(2);
        let _ = buf.split_to(buf.len() - keep);
        return vec![];
    };

    let _ = buf.split_to(first);
    starts
        .windows(2)
        .map(|w| buf.split_to(w[1] - w[0]).freeze())
        .collect()
}

/// NAL unit type (lower 5 bits of the header byte after the start code)
pub fn nal_type(nal: &[u8]) -> Option<u8> {
    let header = start_codes(&nal[..nal.len().min(5)]).first()? + 3;
    nal.get(header).map(|b| b & 0x1F)
}

/// Frames a NAL unit for the browser: `[u32 big-endian length][NAL]`
pub fn frame_for_browser(nal: &[u8]) -> Bytes {
    let mut framed = BytesMut::with_capacity(nal.len() + 4);
    framed.extend_from_slice(&(nal.len() as u32).to_be_bytes());
    framed.extend_from_slice(nal);
    framed.freeze()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_complete_nal_units_and_keeps_the_rest() {
        let mut buf = BytesMut::from(
            &[
                9, 9, // garbage before the first start code
                0, 0, 0, 1, 0x67, 1, 2, // SPS (4-byte start code)
                0, 0, 1, 0x68, 3, // PPS
                0, 0, 1, 0x65, 4, 5, // IDR slice, incomplete so far
            ][..],
        );
        let nals = split_nal_units(&mut buf);
        assert_eq!(nals.len(), 2);
        assert_eq!(&nals[0][..], &[0, 0, 1, 0x67, 1, 2]);
        assert_eq!(&nals[1][..], &[0, 0, 1, 0x68, 3]);
        assert_eq!(&buf[..], &[0, 0, 1, 0x65, 4, 5]);

        assert_eq!(nal_type(&nals[0]), Some(NAL_TYPE_SPS));
        assert_eq!(nal_type(&nals[1]), Some(8));
        assert_eq!(nal_type(&buf), Some(5));

        buf.extend_from_slice(&[6, 0, 0, 1, 0x41, 7]);
        let nals = split_nal_units(&mut buf);
        assert_eq!(&nals[..], &[Bytes::from_static(&[0, 0, 1, 0x65, 4, 5, 6])]);
        assert_eq!(&buf[..], &[0, 0, 1, 0x41, 7]);
    }

    #[test]
    fn keeps_a_start_code_split_across_reads() {
        let mut buf = BytesMut::from(&[5, 5, 5, 0, 0][..]);
        assert!(split_nal_units(&mut buf).is_empty());
        assert_eq!(&buf[..], &[0, 0]);
        buf.extend_from_slice(&[1, 0x67, 0, 0, 1, 0x68]);
        let nals = split_nal_units(&mut buf);
        assert_eq!(&nals[..], &[Bytes::from_static(&[0, 0, 1, 0x67])]);
    }

    #[test]
    fn frames_with_big_endian_length() {
        assert_eq!(
            &frame_for_browser(&[0, 0, 1, 0x65])[..],
            &[0, 0, 0, 4, 0, 0, 1, 0x65]
        );
    }
}
