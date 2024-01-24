const fetch = require('node-fetch');
const { URL } = require('./const');
const _ = require('lodash');
let counter = 1;

function parseRustString(rustString) {
    let fields = ['message', 'error_code', 'vm_error_code'];
    const codes = {
        BadRequest: 400,
        Forbidden: 403,
        NotFound: 404,
        Gone: 410,
        Internal: 500,
        ServiceUnavailable: 503,
    };
    let result = {};
    let match = rustString.match(/(.*?)\(/)[1];
    result['code'] = codes[match] || 404;
    for (let field of fields) {
        let regex = new RegExp(field + ': (.*?)[,}]');
        let match = rustString.match(regex);
        result[field] = match ? match[1].trim() : null;
    }
    let someValues = rustString.match(/Some\((.*?)\)/g);
    if (someValues) {
        const headers = someValues.map(value => {
            return Number(value.replace('Some(', '').replace(')', ''));
        });
        result['header'] = {
            chain_id: headers[0],
            ledger_version: headers[1],
            ledger_oldest_version: headers[2],
            ledger_timestamp_usec: headers[3],
            epoch: headers[4],
            block_height: headers[5],
            oldest_block_height: headers[6],
            cursor: headers[7] || null,
        };
    }
    result['error_code'] = _.snakeCase(result['error_code']);
    result['vm_error_code'] = result['vm_error_code'] === 'None' ? null : result['vm_error_code'];
    return result;
}

function request(method, params) {
    counter++;
    const rpcData = {
        jsonrpc: '2.0',
        method: method,
        params: !!params ? [params] : [],
        id: counter,
    };
    let body = JSON.stringify(rpcData);
    console.log('-------rpcData-----', body);
    return fetch(URL, {
        method: 'POST',
        body,
        headers: { 'Content-Type': 'application/json' },
    })
        .then(response => response.json())
        .then(res => {
            // console.log('---rpc----res-----', body, res);
            let result = res.result;
            let data = result.data || '{}';
            let error = result.error;
            if (error) {
                error = parseRustString(error);
                result.header = error.header && JSON.stringify(error.header);
                delete error.header;
            } else {
                if (!params?.is_bcs_format) {
                    data = JSON.parse(result.data);
                }
            }
            let ret = {
                data,
                header: result.header && JSON.parse(result.header),
                error: error,
            };
            return ret;
        });
}
module.exports = { request };
