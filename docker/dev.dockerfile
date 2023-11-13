FROM ubuntu:latest

ARG VERSION=0.0.0

# Update package list and install curl
RUN apt-get update && apt-get install -y curl wget git libpq-dev

# Install curl and other dependencies required for Node.js installation
RUN apt-get update && apt-get install -y curl && apt-get clean

# Install Node.js and npm
RUN curl -fsSL https://deb.nodesource.com/setup_14.x | bash - \
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

CMD ["/bin/bash"]
