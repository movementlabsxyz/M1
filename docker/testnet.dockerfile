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

ARG VERSION=0.0.0

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

# Update package list and install curl
RUN apt-get update && apt-get install -y curl wget git libpq-dev

# Install curl and other dependencies required for Node.js installation
RUN apt-get update && apt-get install -y curl && apt-get clean

# Install Node.js and npm
RUN curl -fsSL https://deb.nodesource.com/setup_18.x | bash - \
    && apt-get install -y nodejs

# Verify installation
RUN node --version
RUN npm --version

# Download the script
RUN curl -fSsL -o install.sh https://raw.githubusercontent.com/movemntdev/M1/main/scripts/install.sh

# Make the script executable
RUN chmod +x install.sh

# Execute the script with the desired arguments
RUN ./install.sh --version ${VERSION}

RUN source ~/.bashrc && movement manage install m1 testnet --ver ${VERSION}

CMD ["/bin/bash"]