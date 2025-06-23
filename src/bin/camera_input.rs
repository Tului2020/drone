use bytes::BytesMut;
// use image::GrayImage;
// use optical_flow_lk::{build_pyramid, calc_optical_flow, good_features_to_track};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpListener,
};

const TCP_PORT: u16 = 2222;

fn find_start_codes(buf: &[u8]) -> Vec<usize> {
    let mut indices = Vec::new();
    let len = buf.len();
    let mut i = 0;

    while i + 3 < len {
        if buf[i] == 0
            && buf[i + 1] == 0
            && ((buf[i + 2] == 1) || (buf[i + 2] == 0 && buf[i + 3] == 1))
        {
            indices.push(i);
            i += if buf[i + 2] == 1 { 3 } else { 4 };
        } else {
            i += 1;
        }
    }

    indices
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{TCP_PORT}")).await?;
    println!("Listening on port {TCP_PORT}...");

    // let mut tracker_initialized = false;
    // let mut prev_pyr = Vec::new();
    // let mut prev_points = Vec::new();

    loop {
        let (mut socket, addr) = listener.accept().await?;
        println!("Connection from {}", addr);

        tokio::spawn(async move {
            let file = File::create("video.h264").await.unwrap();
            let mut writer = BufWriter::new(file);
            let mut buffer = [0u8; 4096];
            let mut frame_buf = BytesMut::with_capacity(1024 * 1024);

            loop {
                match socket.read(&mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        frame_buf.extend_from_slice(&buffer[..n]);

                        // detect NAL unit boundaries
                        let indices = find_start_codes(&frame_buf);
                        if indices.len() > 1 {
                            for w in indices.windows(2) {
                                let start = w[0];
                                let end = w[1];

                                let frame = &frame_buf[start..end];
                                println!("Frame candidate: {} bytes", frame.len());

                                if writer.write_all(frame).await.is_err() {
                                    eprintln!("Failed to write to file");
                                    break;
                                }
                            }

                            // retain last incomplete chunk
                            let last_start = *indices.last().unwrap();
                            frame_buf = frame_buf.split_off(last_start);
                        }
                    }
                    Err(e) => {
                        eprintln!("Socket error: {:?}", e);
                        break;
                    }
                }
            }

            println!("Connection closed.");
        });
    }
}
