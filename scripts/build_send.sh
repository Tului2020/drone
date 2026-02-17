#!/bin/bash

# Dependencies (install once):
# brew install --cask docker         # Desktop edition on macOS
# rustup component add rust-src      # cross needs it
# cargo install cross --git https://github.com/cross-rs/cross --locked

# Script to build and deploy drone binary to Raspberry Pi
# Usage: ./build_send.sh [OS_BIT] [USERNAME] [HOSTNAME] [DESTINATION]
# 
# Parameters:
#   OS_BIT      - Target OS bit architecture (32 or 64, default: 64)
#   USERNAME    - SSH username (default: pilot for 64-bit, drone for 32-bit)
#   HOSTNAME    - SSH hostname (default: tului-drone.local for 64-bit, drone.local for 32-bit)
#   DESTINATION - Destination path (default: /home/USERNAME)

# Parse command line arguments with defaults
OS_BIT=${1:-64}
USERNAME=${2:-}
HOSTNAME=${3:-}
DESTINATION=${4:-}

# Set defaults based on OS_BIT if not provided
if [ "$OS_BIT" = "32" ]; then
    TARGET="armv7-unknown-linux-gnueabihf"
    USERNAME=${USERNAME:-drone}
    HOSTNAME=${HOSTNAME:-drone.local}
else
    TARGET="aarch64-unknown-linux-gnu"
    USERNAME=${USERNAME:-pilot}
    HOSTNAME=${HOSTNAME:-tului-drone.local}
fi

# Set destination default if not provided
DESTINATION=${DESTINATION:-/home/$USERNAME}

echo "Building for ${OS_BIT}-bit Raspberry Pi OS..."
echo "Target: $TARGET"
echo "Deploying to: $USERNAME@$HOSTNAME:$DESTINATION"

# Build and deploy
cross build --target=$TARGET --release --no-default-features --features raspi && \
scp ./target/$TARGET/release/drone $USERNAME@$HOSTNAME:$DESTINATION