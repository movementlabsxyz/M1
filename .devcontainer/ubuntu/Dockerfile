# Use the official Ubuntu image as a base
FROM ubuntu:latest

# Set environment variables to non-interactive (this prevents some prompts)
ENV DEBIAN_FRONTEND=non-interactive

# Install necessary tools and libraries
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    golang

RUN apt-get install -y libclang-dev

# Install Rust using rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Add Rust to the PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Install postrgresql (for indexer service)
RUN apt install -y postgresql libpq-dev

# Install avalanchego

# Install ANR


# Install subnet-cli
ENV VERSION=0.0.4
RUN echo '#!/bin/bash\n\
GOARCH=$(go env GOARCH)\n\
DOWNLOAD_PATH=/tmp/subnet-cli.tar.gz\n\
DOWNLOAD_URL=https://github.com/ava-labs/subnet-cli/releases/download/v'${VERSION}'/subnet-cli_'${VERSION}'_linux_'${GOARCH}'.tar.gz\n\
curl -L ${DOWNLOAD_URL} -o ${DOWNLOAD_PATH}\n\
tar xzvf ${DOWNLOAD_PATH} -C /tmp\n\
mv /tmp/subnet-cli /usr/local/bin/subnet-cli\n\
rm -f ${DOWNLOAD_PATH}\n\
' > /root/download_subnet_cli.sh && chmod +x /root/download_subnet_cli.sh

# Execute the script
RUN /root/download_subnet_cli.sh

# Set the working directory in the container
WORKDIR /workspace

# Command to run on container start
CMD [ "bash" ]
