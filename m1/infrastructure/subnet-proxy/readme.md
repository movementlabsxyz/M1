
## Introduction
This repository contains the code for the Movement Subnet JSON-RPC middleware. The purpose of this middleware is to ensure compatibility with the Aptos SDK. 

## Prerequisites
Before starting, ensure that you have the following dependencies installed:
- Node.js(^18.0)
## Installation
1. Clone the repository to your local machine.
2. Navigate to the project directory.
3. Open the `.env` file and replace the `url` with the RPC endpoint for the desired subnet. You can find the RPC endpoint at [movement-v2](https://github.com/movemntdev/movement-v2).
4. Save the `.env` file.
5. Run the following command to install the required dependencies:
```bash
npm i
```

## Usage
To start the middleware, run the following command:
```bash
npm start
```

## Test
```javascript
const aptos = require("aptos");
const NODE_URL = "https://127.0.0.1:3001/v1";
const client = new aptos.AptosClient(NODE_URL);
```