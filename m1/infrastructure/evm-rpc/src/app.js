import 'dotenv/config';
import express from 'express';
import cors from 'cors';
import JsonRpc from 'json-rpc-2.0';
import { rpc } from './rpc.js';
import { SERVER_PORT } from './const.js';
import { ethers } from 'ethers';
import { faucet } from './bridge.js';
import { getMoveHash } from './db.js';
const { JSONRPCServer, createJSONRPCErrorResponse } = JsonRpc;
const app = express();
app.use(cors());
app.use(express.json({ limit: '10mb' }));

const server = new JSONRPCServer();
for (const [key, value] of Object.entries(rpc)) {
    server.addMethod(key, value);
}
// error handler
server.applyMiddleware(async function (next, request, serverParams) {
    try {
        return await next(request, serverParams);
    } catch (error) {
        const message = typeof error === 'string' ? error : error?.message || 'Internal error';
        const err = createJSONRPCErrorResponse(request.id, error?.code || -32000, message, {
            message,
        });
        return err;
    }
});
app.get('/v1/eth_faucet', async function (req, res, next) {
    const address = req.query.address;
    if (!ethers.isAddress(address)) {
        res.status(400).json({
            error: 'invalid address',
        });
        return;
    }
    try {
        let hash = await faucet(address);
        res.json({
            data: hash,
        });
    } catch (error) {
        res.status(400).json({
            error: 'please try again after 10 minutes',
        });
    }
});

app.get('/v1/move_hash', async function (req, res, next) {
    const hash = req.query?.hash?.toLowerCase() ?? '0x1';
    const move_hash = await getMoveHash(hash);
    res.status(200).json({
        data: move_hash,
    });
});

app.use('/v1', async function (req, res, next) {
    const context = { ip: req.ip };
    console.log('>>> %s %s', context.ip, req.body.method);
    let str_req = `<<< ${JSON.stringify(req.body)}`;
    server.receive(req.body).then(jsonRPCResponse => {
        if (jsonRPCResponse.error) {
            console.error(str_req, jsonRPCResponse);
        } else {
            console.log(str_req, jsonRPCResponse);
        }
        if (Array.isArray(req.body) && req.body.length === 1) {
            res.json([jsonRPCResponse]);
        } else {
            res.json(jsonRPCResponse);
        }
    });
});

app.use('/', async function (req, res, next) {
    const context = { ip: req.ip };
    console.log('>>> %s %s', context.ip, req.body.method);
    let str_req = `<<< ${JSON.stringify(req.body)}`;
    server.receive(req.body).then(jsonRPCResponse => {
        if (jsonRPCResponse.error) {
            console.error(str_req, jsonRPCResponse);
        } else {
            console.log(str_req, jsonRPCResponse);
        }
        if (Array.isArray(req.body) && req.body.length === 1) {
            res.json([jsonRPCResponse]);
        } else {
            res.json(jsonRPCResponse);
        }
    });
});

app.set('trust proxy', true);
app.listen(SERVER_PORT, () => {
    console.log('server start at http://127.0.0.1:' + SERVER_PORT);
});
import('./task.js');
