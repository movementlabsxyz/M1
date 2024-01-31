# Stage 1: Use a Debian image with Golang pre-installed
FROM golang:1.21-bullseye as golang-base

# Stage 2: Use the official Ubuntu image as your final base
FROM ubuntu:latest

# Set environment variables to non-interactive (this prevents some prompts)
ENV DEBIAN_FRONTEND=non-interactive

# Copy Golang from the golang-base image
COPY --from=golang-base /usr/local/go /usr/local/go

# Add Golang to the PATH
ENV PATH="/usr/local/go/bin:${PATH}"

# Set environment variables to non-interactive (this prevents some prompts)
ENV DEBIAN_FRONTEND=non-interactive

# Install necessary tools and libraries
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    binutils \
    pkg-config \
    libssl-dev \
    sudo \       
    git          

RUN apt-get install -y libclang-dev

# Install Rust using rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Add Rust to the PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Install PostgreSQL (for indexer service)
RUN apt install -y postgresql libpq-dev

# Set the working directory in the container
WORKDIR /workspace

# Command to run on container start
CMD [ "bash" ]
