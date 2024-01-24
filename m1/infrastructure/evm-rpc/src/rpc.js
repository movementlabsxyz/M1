import { toHex } from './helper.js';
import { CHAIN_ID } from './const.js';
import {
    callContract,
    estimateGas,
    getBalance,
    getBlock,
    getBlockByHash,
    getBlockByNumber,
    getCode,
    getGasPrice,
    getNonce,
    getStorageAt,
    getTransactionByHash,
    getTransactionReceipt,
    sendRawTx,
    faucet,
    getLogs,
    eth_feeHistory,
} from './bridge.js';
import JsonRpc from 'json-rpc-2.0';
const { JSONRPCErrorException } = JsonRpc;
export const rpc = {
    eth_feeHistory: async function (args) {
        return eth_feeHistory();
    },
    eth_getLogs: async function (args) {
        return getLogs(args[0]);
    },
    web3_clientVersion: async function () {
        return 'Geth/v1.11.6-omnibus-f83e1598/linux-.mdx64/go1.20.3';
    },
    /**
     * Returns the chain ID in hexadecimal format.
     * @returns {Promise<string>} The chain ID.
     */
    eth_chainId: async function () {
        return toHex(CHAIN_ID);
    },

    /**
     * Returns the version number in hexadecimal format.
     * @returns {Promise<string>} The version number.
     */
    net_version: async function () {
        return toHex(CHAIN_ID);
    },

    /**
     * Retrieves the current gas price.
     * @returns {Promise<number>} The current gas price.
     */
    eth_gasPrice: async function () {
        return getGasPrice();
    },

    /**
     * Retrieves the latest block number.
     * @returns {Promise<number>} The latest block number.
     */
    eth_blockNumber: async function () {
        return getBlock();
    },

    /**
     * Sends a signed raw transaction.
     * @param {Array<string>} args - The arguments array, where the first element is the signed transaction data.
     * @returns {Promise<string>} The transaction hash.
     * @throws Will throw an error if the transaction fails.
     */
    eth_sendRawTransaction: async function (args) {
        try {
            return await sendRawTx(args[0]);
        } catch (error) {
            if (typeof error === 'string') {
                throw new JSONRPCErrorException(error, -32000);
            }
            throw new JSONRPCErrorException(error.message || 'execution reverted', -32000);
        }
    },

    /**
     * Invokes a method of a smart contract.
     * @param {Array<Object>} args - The arguments array, where the first element is an object containing the 'from', 'to', and 'data' properties.
     * @returns {Promise<string>} The result of the contract method invocation.
     * @throws Will throw an error if the contract method invocation fails.
     */
    eth_call: async function (args) {
        let { to, data: data_, from } = args[0];
        if (args[0].gasPrice) return {};
        try {
            return await callContract(from, to, data_);
        } catch (error) {
            throw new JSONRPCErrorException('execution reverted', -32000);
        }
    },

    /**
     * Get the transaction count for a given address
     * @param {Array} args - The arguments array, where args[0] is the address
     * @returns {Promise} - A promise that resolves to the transaction count
     */
    eth_getTransactionCount: async function (args) {
        return getNonce(args[0]);
    },

    /**
     * Get a transaction by its hash
     * @param {Array} args - The arguments array, where args[0] is the transaction hash
     * @returns {Promise} - A promise that resolves to the transaction object
     */
    eth_getTransactionByHash: async function (args) {
        return getTransactionByHash(args[0]);
    },

    /**
     * Get the receipt of a transaction by its hash
     * @param {Array} args - The arguments array, where args[0] is the transaction hash
     * @returns {Promise} - A promise that resolves to the transaction receipt object
     */
    eth_getTransactionReceipt: async function (args) {
        return getTransactionReceipt(args[0]);
    },

    /**
     * Estimate the gas required to execute a transaction
     * @param {Array} args - The arguments array, where args[0] is the transaction object
     * @returns {Promise} - A promise that resolves to the estimated gas
     */
    eth_estimateGas: async function (args) {
        let res = await estimateGas(args[0]);
        if (!res.success) {
            throw new JSONRPCErrorException(res.error, -32000);
        }
        return toHex(res.show_gas);
    },

    /**
     * Get a block by its number
     * @param {Array} args - The arguments array, where args[0] is the block number
     * @returns {Promise} - A promise that resolves to the block object
     */
    eth_getBlockByNumber: async function (args) {
        return getBlockByNumber(args[0]);
    },

    /**
     * Get a block by its hash
     * @param {Array} args - The arguments array, where args[0] is the block hash
     * @returns {Promise} - A promise that resolves to the block object
     */
    eth_getBlockByHash: async function (args) {
        return getBlockByHash(args[0]);
    },

    /**
     * Get the balance of an address
     * @param {Array} args - The arguments array, where args[0] is the address
     * @param {Object} ctx - The context object
     * @returns {Promise} - A promise that resolves to the balance
     */
    eth_getBalance: async function (args) {
        return getBalance(args[0]);
    },

    /**
     * Get the code at a specific address
     * @param {Array} args - The arguments array, where args[0] is the address
     * @returns {Promise} - A promise that resolves to the code
     */
    eth_getCode: async function (args) {
        return getCode(args[0]);
    },

    /**
     * Get the storage at a specific position in a specific address
     * @param {Array} args - The arguments array, where args[0] is the address and args[1] is the storage position
     * @returns {Promise} - A promise that resolves to the storage value
     */
    eth_getStorageAt: async function (args) {
        return getStorageAt(args[0], args[1]);
    },

    eth_faucet: async function (args) {
        return faucet(args[0]);
    },
};
