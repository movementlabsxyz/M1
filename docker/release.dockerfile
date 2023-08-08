FROM ubuntu:latest

ARG VERSION=0.0.0

RUN curl -fSsL https://raw.githubusercontent.com/movemntdev/M1/main/scripts/install.sh | sh -s --version ${VERSION} --dev

CMD ["/bin/bash"]