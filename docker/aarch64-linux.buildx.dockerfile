# Use a base Rust image
FROM rust:1.69.0 as builder

# Install necessary packages for cross-compilation
RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y build-essential gcc-aarch64-linux-gnu g++-aarch64-linux-gnu libc6-dev-arm64-cross

# Set up Rust for Linux ARM64
RUN rustup target add aarch64-unknown-linux-gnu

WORKDIR /app

# Copy your Rust project into the builder
COPY . /app

# Set environment variables for cross compilation
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
ENV CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
ENV CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++

# Build Linux ARM64 binaries
RUN cd /app/m1 && \
    cargo clean && \
    RUSTFLAGS="--cfg tokio_unstable" cargo build --release --target aarch64-unknown-linux-gnu
