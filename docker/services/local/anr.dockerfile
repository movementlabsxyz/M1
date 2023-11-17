# Use Ubuntu latest as the base image
FROM ubuntu:latest

# Set the working directory in the container
WORKDIR /usr/src/app

# Install necessary tools and dependencies
RUN apt-get update && apt-get install -y curl build-essential

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Go
RUN apt-get install -y golang

# Copy the contents of your working directory to the container
COPY . .

# Make the script executable
RUN chmod +x ./m1/scripts/run.debug.sh

EXPOSE 9650
EXPOSE 8070
EXPOSE 8090

# Run the script
CMD cd ./m1 && ./scripts/run.debug.sh
