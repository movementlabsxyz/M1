import { HexString } from 'aptos';
import { Op } from 'sequelize';
import {
    EVM_CONTRACT,
    SENDER_ACCOUNT,
    SENDER_ADDRESS,
    client,
    ZERO_HASH,
    LOG_BLOOM,
    FAUCET_SENDER_ADDRESS,
    FAUCET_SENDER_ACCOUNT,
} from './const.js';
import { parseRawTx, sleep, toHex, toNumber } from './helper.js';
import { TxEvents, getMoveHash, saveMoveEvmTxHash, saveTx } from './db.js';
import { ZeroAddress, ethers, isHexString, toBeHex, keccak256 } from 'ethers';
import BigNumber from 'bignumber.js';
import Lock from 'async-lock';
const LOCKER_MAX_PENDING = 20;
const locker = new Lock({
    maxExecutionTime: 30 * 1000,
});

const LOCKER_KEY_SEND_TX = 'sendTx';
let lastBlockTime = Date.now();
let lastBlock = '0x1';
await getBlock();

export async function faucet(addr) {
    const payload = {
        function: `${EVM_CONTRACT}::evm::deposit`,
        type_arguments: [],
        arguments: [toBuffer(addr), toBuffer(toBeHex((1e18).toString()))],
    };
    const txnRequest = await client.generateTransaction(FAUCET_SENDER_ADDRESS, payload);
    const signedTxn = await client.signTransaction(FAUCET_SENDER_ACCOUNT, txnRequest);
    const transactionRes = await client.submitTransaction(signedTxn);
    await client.waitForTransaction(transactionRes.hash);
    return transactionRes.hash;
}

export async function eth_feeHistory() {
    const block = await getBlock();
    const baseFeePerGas = toHex(1500 * 10 ** 9);
    return {
        oldestBlock: toHex(toNumber(block) - 4),
        reward: [
            ['0x5f5e100', '0xd3cdba48'],
            ['0x5f5e100', '0xb146453a'],
            ['0xb8c63f00', '0xb8c63f00'],
            ['0x5f5e100', '0x77359400'],
        ],
        baseFeePerGas: Array.from({ length: 4 }, () => baseFeePerGas),
        gasUsedRatio: [0.5329073333333333, 0.3723229, 0.9996228333333333, 0.5487537333333333],
    };
}

/**
 * Get the latest block. If the last block was fetched less than 2 seconds ago, return the cached block.
 * Otherwise, fetch the latest block from the client and update the cache.
 * @returns {Promise<string>} - A promise that resolves to the latest block
 */
export async function getBlock() {
    if (Date.now() - lastBlockTime >= 2000) {
        let info = await client.getLedgerInfo();
        lastBlockTime = Date.now();
        lastBlock = toHex(info.block_height);
        return lastBlock;
    }
    return lastBlock;
}
/**
 * Get a block by its number. If the block number is "latest", fetch the latest block from the client.
 * @param {string|number} block - The block number or "latest"
 * @returns {Promise<Object>} - A promise that resolves to the block object with the following properties:
 *   - difficulty: string - The difficulty level of the block
 *   - extraData: string - Extra data related to the block
 *   - gasLimit: string - The maximum gas that transactions in the block are allowed to consume
 *   - gasUsed: string - The total gas that transactions in the block have consumed
 *   - hash: string - The hash of the block
 *   - logsBloom: string - The bloom filter for the logs in the block
 *   - miner: string - The address of the miner who mined the block
 *   - mixHash: string - A hash that, combined with the nonce, proves that the block has gone through enough computation
 *   - nonce: string - A random value that, combined with the mixHash, proves that the block has gone through enough computation
 *   - number: string - The number of the block in the blockchain
 *   - parentHash: string - The hash of the parent block
 *   - receiptsRoot: string - The root of the receipts trie of the block
 *   - sha3Uncles: string - The SHA3 hash of the uncles data in the block
 *   - size: string - The size of the block in bytes
 *   - stateRoot: string - The root of the final state trie of the block
 *   - timestamp: string - The timestamp when the block was mined
 *   - totalDifficulty: string - The total difficulty of the chain up to this block
 *   - transactions: Array<string> - The transactions included in the block
 *   - transactionsRoot: string - The root of the transactions trie of the block
 *   - uncles: Array<string> - The uncle blocks of the block
 */
export async function getBlockByNumber(block) {
    if (block === 'latest') {
        let info = await client.getLedgerInfo();
        block = info.block_height;
    }
    block = BigNumber(block).toNumber();
    let info = await client.getBlockByHeight(block, true);
    let parentHash = ZERO_HASH;
    if (block > 2) {
        let info = await client.getBlockByHeight(block - 1);
        parentHash = info.block_hash;
    }
    let transactions = info.transactions || [];
    let evm_tx = [];
    transactions.forEach(it => {
        if (it.type === 'user_transaction' && it.payload.function.startsWith('0x1::evm::send_tx')) {
            evm_tx.push(parseMoveTxPayload(it).hash);
        }
    });
    const genHash = c => {
        const seed = info.block_hash;
        let hash = seed;
        while (c > 0) {
            c--;
            hash = keccak256(hash);
        }
        return hash;
    };

    return {
        baseFeePerGas: '0xc', // eip1559
        difficulty: '0x0',
        extraData: genHash(1),
        gasLimit: toHex(30_000_000),
        gasUsed: '0x0000000000000000',
        hash: info.block_hash,
        logsBloom: LOG_BLOOM,
        miner: ZeroAddress,
        mixHash: genHash(2),
        nonce: '0x0000000000000000',
        number: toHex(block),
        parentHash: parentHash,
        receiptsRoot: genHash(3),
        sha3Uncles: genHash(4),
        size: toHex(1000000),
        stateRoot: genHash(5),
        timestamp: toHex(Math.trunc(info.block_timestamp / 1e6)),
        totalDifficulty: '0x0000000000000000',
        transactions: evm_tx,
        transactionsRoot: genHash(6),
        uncles: [],
    };
}

export async function getBlockByHash(hash) {
    return {};
}
/**
 * Get the code at a specific address.
 * @param {string} addr - The address to get the code from.
 * @returns {Promise<string>} - A promise that resolves to the code at the given address.
 */
export async function getCode(addr) {
    let result = await getAccountInfo(addr);
    return result.code;
}

export async function getStorageAt(addr, pos) {
    let res = '0x';
    let payload = {
        function: EVM_CONTRACT + `::evm::get_storage_at`,
        type_arguments: [],
        arguments: [addr, toHex(pos)],
    };
    try {
        let result = await client.view(payload);
        res = result[0];
    } catch (error) {
        console.log('getStorageAt error', error);
    }
    return res;
}

// Forge will send multiple transactions at the same time
// and the order of nonces is not necessarily in ascending order,
// so we need to sort them again.
async function checkAddressNonce(info) {
    const startTs = Date.now();
    while (1) {
        try {
            const accInfo = await Promise.race([
                getAccountInfo(info.from),
                new Promise((_, reject) => setTimeout(() => reject(new Error('Timeout')), 10000)),
            ]);
            if (parseInt(accInfo.nonce) === parseInt(info.nonce)) {
                return true;
            }
        } catch (error) {
            continue;
        }
        if (Date.now() - startTs > 30 * 1000) {
            throw 'Timeout to Discard. Please send tx follow address nonce order';
        }
        await sleep(0.5);
    }
}

// Because only successful transactions in Move EVM will update the nonce,
// we don't need to check the nonce here.
// We can directly read it from the blockchain instead of relying on user input.
export async function sendRawTx(tx) {
    const info = parseRawTx(tx);
    console.log('raw tx info', info);
    // let v = info.v;
    // if (v === 27 || v === 28) {
    //     throw 'only replay-protected (EIP-155) transactions allowed over RPC';
    // }
    // if (v !== 707 && v !== 708) {
    //     throw 'chain id error';
    // }
    checkTxQueue();
    // this guarantee the nonce order for same from address
    await checkAddressNonce(info);
    // this guarantee the the sender address is order
    return locker.acquire(LOCKER_KEY_SEND_TX, async function (done) {
        let fee = '0x01';
        let gasPrice = toNumber(BigNumber(info.gasPrice).div(1e10).decimalPlaces(0));
        // we don't need too large gasPrice for send tx
        // now it enough to finish this tx
        // now it will cost the sender gas , so set it a limit
        if (gasPrice > 300) gasPrice = 300;
        if (gasPrice < 100) gasPrice = 100;
        const payload = {
            function: `${EVM_CONTRACT}::evm::send_tx`,
            type_arguments: [],
            arguments: [
                toBuffer(info.from), // // useless will remove at next version
                toBuffer(tx),
                toBuffer(fee),
                1, // useless will remove at next version
            ],
        };
        let gasInfo;
        try {
            const txnRequest = await client.generateTransaction(SENDER_ADDRESS, payload);
            let res = await client.simulateTransaction(SENDER_ACCOUNT, txnRequest, {
                estimatePrioritizedGasUnitPrice: true,
            });
            gasInfo = {
                success: res[0].success,
                gas_used: res[0].gas_used,
                error: res[0].vm_status,
            };
        } catch (error) {
            gasInfo = {
                success: false,
                gas_used: 0,
                error: error.message || 'sendTx error',
            };
        }
        if (!gasInfo.success) return done(gasInfo.error);
        fee = toBeHex(BigNumber(gasPrice).times(gasInfo.gas_used).decimalPlaces(0).toFixed(0));
        console.log('nonce %s,fee:%s', info.nonce, fee);
        payload.arguments[2] = toBuffer(fee);
        sendTx(payload, true, {
            gas_unit_price: gasPrice,
        })
            .then(hash => {
                saveTx(tx, hash, JSON.stringify(info));
                saveMoveEvmTxHash(hash, info.hash).then(() => {
                    done(null, info.hash);
                });
            })
            .catch(err => {
                done(err.message || 'sendTx error');
            });
    });
}

export function callContract(from, contract, calldata) {
    from = from || ZeroAddress;
    return view(from, contract, calldata);
}
/**
 * Estimate the gas needed for a transaction.
 * @param {Object} info - The transaction information, including from, to, data, and value.
 * @returns {Promise<Object>} - A promise that resolves to an object containing:
 *   - success: boolean - Whether the gas estimation was successful
 *   - gas_used: number - The amount of gas used
 *   - show_gas: number - The amount of gas to show,
 */
export async function estimateGas(info) {
    if (!info.data) info.data = '0x';
    const payload = {
        function: `${EVM_CONTRACT}::evm::estimate_tx_gas`,
        type_arguments: [],
        arguments: [
            // The arguments for the function.
            toBuffer(info.from), // The sender's address.
            toBuffer(info.to || ZeroAddress), // The receiver's address, or the zero address if no receiver is specified.
            toBuffer(info.data === '0x' ? '0x' : toBeHex(info.data)), // The data to send, or '0x' if no data is specified.
            toBuffer(toBeHex(info.value || '0x0')), // The value to send, or '0x0' if no value is specified.
            1, // The last argument is always 1.
        ],
    }; // Set a default `error_gas` value to `1e6` (1,000,000). This value is used if the gas estimation fails.
    const error_gas = 1e6;
    let res;
    try {
        const txnRequest = await client.generateTransaction(SENDER_ADDRESS, payload);
        res = await client.simulateTransaction(SENDER_ACCOUNT, txnRequest, {
            estimatePrioritizedGasUnitPrice: true,
        });
        if (res[0].success) {
            res[0].show_gas = res[0].gas_used; // Set `show_gas` to the gas used by the transaction.
            if (res[0].gas_used < 21000) {
                // If the gas used is less than 21,000 (the minimum gas required for a transaction in Ethereum)...
                res[0].show_gas = 21000; // Set `show_gas` to 21,000.
            }
        } else {
            res[0].show_gas = error_gas;
            res[0].gas_used = error_gas;
            res[0].error = res[0].vm_status;
        }
    } catch (error) {
        res = [
            {
                success: false,
                gas_used: error_gas,
                show_gas: error_gas,
                error: error.message || 'estimate gas error',
            },
        ];
    }
    const ret = {
        success: res[0].success,
        gas_used: res[0].gas_used,
        show_gas: res[0].show_gas,
        error: res[0].error,
    };
    return ret;
}
export async function getGasPrice() {
    const info = await client.estimateGasPrice();
    // for the move is 8 and eth is 18
    // enlarge this gas price to fit eth decimals
    return toHex(BigNumber(info.prioritized_gas_estimate).times(1e10));
}
/**
 * Get a transaction by its hash.
 * @param {string} tx - The hash of the transaction.
 * @returns {Promise<Object>} - A promise that resolves to an object containing:
 *   - blockHash: string - The hash of the block containing the transaction
 *   - blockNumber: string - The number of the block containing the transaction
 *   - from: string - The address from which the transaction was sent
 *   - gas: string - The amount of gas used by the transaction
 *   - gasPrice: string - The price of gas in the transaction
 *   - hash: string - The hash of the transaction
 *   - input: string - The input data of the transaction
 *   - nonce: string - The nonce of the transaction
 *   - to: string - The address to which the transaction was sent
 *   - transactionIndex: string - The index of the transaction in the block
 *   - value: string - The value transferred in the transaction
 *   - v: string - The v value of the transaction's signature
 *   - r: string - The r value of the transaction's signature
 *   - s: string - The s value of the transaction's signature
 */
export async function getTransactionByHash(evm_hash) {
    let tx = await getMoveHash(evm_hash);
    let info = await client.getTransactionByHash(tx);
    let block = await client.getBlockByVersion(info.version);
    const { to, from, data, nonce, value, v, r, s, hash, type } = parseMoveTxPayload(info);
    return {
        blockHash: ZERO_HASH,
        blockNumber: toHex(block.block_height),
        from: from,
        gas: toHex(info.gas_used),
        gasPrice: toHex(+info.gas_unit_price * 1e10),
        maxFeePerGas: toHex(info.max_fee_per_gas),
        maxPriorityFeePerGas: toHex(info.max_priority_fee_per_gas),
        hash: hash,
        input: data,
        type,
        nonce: toHex(nonce),
        to: to,
        transactionIndex: '0x0',
        value: toHex(value),
        v: toHex(v),
        r: r,
        s: s,
    };
}

export async function getTransactionReceipt(evm_hash) {
    let tx = await getMoveHash(evm_hash);
    let info = await client.getTransactionByHash(tx);
    let block = await client.getBlockByVersion(info.version);
    const { to, from, type } = parseMoveTxPayload(info);
    let contractAddress = await getDeployedContract(info);
    const logs = parseLogs(info, block.block_height, block.block_hash, evm_hash);
    let recept = {
        blockHash: block.block_hash,
        blockNumber: toHex(block.block_height),
        contractAddress,
        cumulativeGasUsed: toHex(info.gas_used),
        effectiveGasPrice: toHex(info.gas_unit_price * 1e10),
        from: from,
        gasUsed: toHex(info.gas_used),
        logs: logs,
        to: Boolean(contractAddress) ? null : to,
        logsBloom: LOG_BLOOM,
        status: info.success ? '0x1' : '0x0',
        transactionHash: evm_hash,
        transactionIndex: '0x0',
        type,
    };
    return recept;
}
/**
 * Retrieves the nonce for a given sender.
 * @param {string} sender - The sender's address.
 * @returns {Promise<string>} The nonce in hexadecimal format.
 * @throws Will throw an error if the account information cannot be retrieved.
 */
export async function getNonce(sender) {
    let info = await getAccountInfo(sender);
    return toHex(info.nonce);
}

/**
 * Retrieves the balance for a given sender.
 * @param {string} sender - The sender's address.
 * @returns {Promise<string>} The balance in hexadecimal format.
 * @throws Will throw an error if the account information cannot be retrieved.
 */
export async function getBalance(sender) {
    let info = await getAccountInfo(sender);
    return toHex(info.balance);
}

const CACHE_ETH_ADDRESS_TO_MOVE = {};

/**
 * Retrieves account information for a given Ethereum address.
 * @param {string} acc - The Ethereum address.
 * @returns {Promise<Object>} An object containing the account's balance, nonce, and code.
 * @throws Will not throw an error if the Ethereum address has not been deposited from Move.
 */
async function getAccountInfo(acc) {
    const ret = {
        balance: '0x0',
        nonce: 0,
        code: '0x',
    };
    acc = acc.toLowerCase();
    try {
        let moveAddress = CACHE_ETH_ADDRESS_TO_MOVE[acc];
        if (!moveAddress) {
            let payload = {
                function: `${EVM_CONTRACT}::evm::get_move_address`,
                type_arguments: [],
                arguments: [acc],
            };
            let result = await client.view(payload);
            moveAddress = result[0];
            CACHE_ETH_ADDRESS_TO_MOVE[acc] = moveAddress;
        }
        const resource = await client.getAccountResource(moveAddress, `${EVM_CONTRACT}::evm::Account`);
        ret.balance = resource.data.balance;
        ret.nonce = +resource.data.nonce;
        ret.code = resource.data.code;
    } catch (error) {
        // if this eth address not deposit from move ,it will error
    }

    return ret;
}

async function sendTx(payload, wait = false, option = {}) {
    try {
        const txnRequest = await client.generateTransaction(SENDER_ADDRESS, payload, {
            ...option,
            max_gas_amount: 2 * 1e6,
            expiration_timestamp_secs: Math.trunc(Date.now() / 1000) + 10,
        });
        const signedTxn = await client.signTransaction(SENDER_ACCOUNT, txnRequest);
        const transactionRes = await client.submitTransaction(signedTxn);
        console.log('sendTx', transactionRes.hash);
        if (wait) await client.waitForTransaction(transactionRes.hash);
        return transactionRes.hash;
    } catch (error) {
        throw new Error(error.message || 'sendTx error ');
    }
}
/**
 * Retrieves the address of the deployed contract.
 * @param {Object} info - The transaction information.
 * @returns {Promise<string|null>} The address of the deployed contract, or null if the transaction was not successful or did not deploy a contract.
 */
async function getDeployedContract(info) {
    if (!info.success) return null;
    const { nonce, to, from } = parseMoveTxPayload(info);
    if (to === ZeroAddress) {
        return ethers.getCreateAddress({ from: from, nonce }).toLowerCase();
    }
    return null;
}

async function view(from, contract, calldata) {
    let payload = {
        function: EVM_CONTRACT + `::evm::query`,
        type_arguments: [],
        arguments: [from, contract, calldata],
    };
    try {
        let result = await client.view(payload);
        return result[0];
    } catch (error) {
        throw error.message;
    }
}

function toBuffer(hex) {
    return new HexString(hex).toUint8Array();
}

function parseMoveTxPayload(info) {
    const args = info.payload.arguments;
    const tx = parseRawTx(args[1]);
    return {
        value: tx.value,
        from: tx.from,
        to: tx.to,
        type: tx.type,
        nonce: tx.nonce,
        data: tx.data,
        fee: args[2],
        r: tx.r,
        s: tx.s,
        v: tx.v,
        hash: tx.hash,
        limit: tx.limit,
        gasPrice: tx.gasPrice,
    };
}

function move2ethAddress(addr) {
    addr = addr.toLowerCase();
    return '0x' + addr.slice(-40);
}

export async function getLogs(obj) {
    const nowBlock = await getBlock();
    const fromBlock = isHexString(obj.fromBlock) ? toNumber(obj.fromBlock) : toNumber(nowBlock);
    const toBlock = isHexString(obj.toBlock) ? toNumber(obj.toBlock) : toNumber(nowBlock);
    const address = Array.isArray(obj.address) ? obj.address : [obj.address];
    const topics = obj.topics;
    const topicsWhere = {};
    if (topics && topics.length > 0 && topics.length <= 4) {
        for (let i = 0; i < topics.length; i++) {
            const item = Array.isArray(topics[i]) ? topics[i] : [topics[i]];
            topicsWhere[`topic${i}`] = {
                [Op.in]: item,
            };
        }
    }
    const ret = await TxEvents.findAll({
        attributes: [
            'address',
            'topics',
            'data',
            'blockNumber',
            'transactionHash',
            'transactionIndex',
            'blockHash',
            'logIndex',
        ],
        where: {
            blockNumber: {
                [Op.gte]: fromBlock,
                [Op.lte]: toBlock,
            },
            address: {
                [Op.in]: address,
            },
            ...topicsWhere,
        },
        limit: 10000,
    });
    return ret.map(it => {
        it = it.dataValues;
        return {
            ...it,
            topics: JSON.parse(it.topics),
            blockNumber: toHex(it.blockNumber),
            removed: false,
        };
    });
}

function parseLogs(info, blockNumber, blockHash, evm_hash) {
    let logs = [];
    let events = info.events || [];
    let evmLogs = [1, 2, 3, 4].map(it => `${EVM_CONTRACT}::evm::Log${it}Event`);
    events = events.filter(it => evmLogs.includes(it.type));
    for (let i = 0; i < events.length; i++) {
        const event = events[i];
        let topics = [];
        if (event.data.topic0) topics.push(event.data.topic0);
        if (event.data.topic1) topics.push(event.data.topic1);
        if (event.data.topic2) topics.push(event.data.topic2);
        if (event.data.topic3) topics.push(event.data.topic3);
        logs.push({
            address: move2ethAddress(event.data.contract),
            topics,
            data: event.data.data,
            blockNumber: toHex(blockNumber),
            transactionHash: evm_hash,
            transactionIndex: toHex(event.sequence_number),
            blockHash: blockHash,
            logIndex: toHex(i),
            removed: false,
        });
    }
    return logs;
}

function checkTxQueue() {
    if (
        locker['queues'][LOCKER_KEY_SEND_TX] &&
        locker['queues'][LOCKER_KEY_SEND_TX].length > LOCKER_MAX_PENDING
    ) {
        throw new Error('system busy');
    }
}
