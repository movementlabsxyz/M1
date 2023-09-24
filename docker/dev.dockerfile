FROM ubuntu:latest

ARG VERSION=0.0.0

# Update package list and install curl
RUN apt-get update && apt-get install -y curl wget git

# Download the script
RUN curl -fSsL -o install.sh https://raw.githubusercontent.com/movemntdev/M1/main/scripts/install.sh

# Make the script executable
RUN chmod +x install.sh

# Execute the script with the desired arguments
RUN ./install.sh --version ${VERSION} --dev

CMD ["/bin/bash"]