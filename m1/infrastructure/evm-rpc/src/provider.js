import { URL } from './const';
import { fetch } from 'node-fetch';
export function request(method, ...params) {
    const rpcData = {
        jsonrpc: '2.0',
        method: method,
        params: [...params],
        id: 1,
    };
    const body = JSON.stringify(rpcData);
    return fetch(URL, {
        method: 'POST',
        body,
        headers: { 'Content-Type': 'application/json' },
    }).then(response => response.json());
}
