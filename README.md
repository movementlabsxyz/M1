<div align="center">
    <img src="./rsc/movement_logo.png" alt="Project Logo" width="200" height="200">

# M1

[![License](https://img.shields.io/badge/license-BSD-blue.svg)](https://opensource.org/license/bsd-3-clause/)
[![Tests](https://img.shields.io/badge/tests-Passing-brightgreen)](#)
[![Build Status](https://img.shields.io/badge/build-Passing-brightgreen)](#)
[![Coverage](https://img.shields.io/codecov/c/github/username/project.svg)](https://codecov.io/gh/username/project)
[![Windows](https://img.shields.io/badge/Windows-Download-blue)](https://github.com/movemntdev/m1/releases)
[![macOS](https://img.shields.io/badge/macOS-Download-blue)](https://github.com/movemntdev/m1/releases)
[![Linux](https://img.shields.io/badge/Linux-Download-blue)](https://github.com/movemntdev/m1/releases)

**An L1 for Move VM built on Avalanche.**

</div>


## Table of Contents

- [Introduction](#introduction)
- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)

---

## Introduction

The Move programming language poses numerous benefits to builders including direct interaction with digital assets through custom resource types, flexibility with transaction script declaration, on-chain verification, and bytecode safety privileges.

Movement M1 is designed for the Avalanche subnet, allowing users to seamlessly interact with and build with the Move language on a on a high-performance, modular, scalable and ineroperable Layer 1.

- Movement will be able to hit 160,000+ theoretical TPS as the project scales to provide much needed performance to protocols.
- Move bytecode verifiers and interpreters provide native solvency for the reentrancy attacks and security woes that have plagued Solidity developers for years, resulting in $3 billion lost last year.

This repository contains the code and contributor documentation for M1. If you would like to learn how to use and develop for the platform, please visit [docs.movementlabs.xyx](docs.movementlabs.xyz).

## Features

Currently, M1 consists of...
- A testnet with bootstrap nodes at [https://seed1-node.movementlabs.xyz](https://seed1-node.movementlabs.xyz).
- An Aptos-compatible cient called `movement`.
- A fork of Aptos framework.

M1 also has its own DEX, with a web client currently available at [https://movemnt-dex-client.vercel.app/](https://movemnt-dex-client.vercel.app/).

## Installation

See [docs.movementlabs.xyx](docs.movementlabs.xyz) for a more complete installation guide. We recommend working with our Docker containers or using our installer.

## Usage

Once you've installed our platform, the easiest way to get started developing is to use the CLI to test code locally and publish to our testnet.

```bash
# test
movement move test

# compile and publish
movement move compile && movement move publish
```

## Contributing

Please submit and review/comment on issues before contributing. Review [CONTRIBUTING.md](./CONTRIBUTING.md).

## License

This project is licensed under the BSD-3-Clause License - see the [LICENSE](LICENSE) file for details.

