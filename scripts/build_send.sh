cross build --target=aarch64-unknown-linux-gnu --release --no-default-features --features raspi && sudo scp ./target/aarch64-unknown-linux-gnu/release/drone drone@drone.local:/home/drone
