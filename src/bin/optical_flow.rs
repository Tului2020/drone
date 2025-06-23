use image::{open, GrayImage};
use optical_flow_lk::{build_pyramid, calc_optical_flow, good_features_to_track};

fn resize_to_gray(img: &GrayImage, width: u32, height: u32) -> GrayImage {
    image::imageops::resize(img, width, height, image::imageops::FilterType::Triangle)
}

fn main() -> anyhow::Result<()> {
    loop {
        let beginning_timestamp = std::time::Instant::now();

        // --- Step 1: Load two images ---
        println!("loading images");
        let img1 = open("./optical_flow/pic1.png")?.to_luma8();
        let img2 = open("./optical_flow/pic2.png")?.to_luma8();

        // --- Step 2: Resize for performance ---
        let gray1: GrayImage = resize_to_gray(&img1, 160, 120);
        let gray2: GrayImage = resize_to_gray(&img2, 160, 120);

        // --- Step 3: Build image pyramids ---
        let pyr1 = build_pyramid(&gray1, 2); // use fewer levels for speed
        let pyr2 = build_pyramid(&gray2, 2);

        // --- Step 4: Detect Shi-Tomasi points in frame1 ---
        let mut points = good_features_to_track(&gray1, 0.05, 5);
        points.truncate(50); // limit to 50 points
        let prev_pts: Vec<(f32, f32)> = points
            .iter()
            .map(|&(x, y, _)| (x as f32, y as f32))
            .collect();

        // --- Step 5: Compute LK optical flow ---
        let next_pts = calc_optical_flow(&pyr1, &pyr2, &prev_pts, 15, 15); // smaller window and fewer iters

        // --- Step 6: Display motion vectors ---
        for ((x0, y0), (x1, y1)) in prev_pts.iter().zip(next_pts.iter()) {
            println!(
                "Point moved: ({:.2},{:.2}) → ({:.2},{:.2}); Δ = ({:.2}, {:.2})",
                x0,
                y0,
                x1,
                y1,
                x1 - x0,
                y1 - y0
            );
        }

        let elapsed = beginning_timestamp.elapsed();
        println!("Elapsed time: {:.2?}", elapsed);
    }

    Ok(())
}
