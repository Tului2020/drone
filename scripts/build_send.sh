# brew install --cask docker         # Desktop edition on macOS
# rustup component add rust-src      # cross needs it
# cargo install cross --git https://github.com/cross-rs/cross --locked

# build for pi os 32-bit
cross build --target armv7-unknown-linux-gnueabihf --release --no-default-features --features raspi --features heartbeat && scp ./target/armv7-unknown-linux-gnueabihf/release/drone drone@drone.local:/home/drone

# build for pi os 64-bit
# cross build --target=aarch64-unknown-linux-gnu --release --no-default-features --features raspi --features heartbeat && sudo scp ./target/aarch64-unknown-linux-gnu/release/drone