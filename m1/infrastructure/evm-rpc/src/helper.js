import { ZeroAddress, Transaction } from 'ethers';
import { BigNumber } from 'bignumber.js';
import { randomBytes } from 'node:crypto';
import { TransactionFactory } from '@ethereumjs/tx';
export function parseRawTx(tx) {
    const tx_ = Transaction.from(tx);
    const txJson = tx_.toJSON();
    const tx2 = TransactionFactory.fromSerializedData(Buffer.from(tx.slice(2), 'hex'));
    const from = tx_.from.toLowerCase();
    let gasPrice = toHex(1500 * 10 ** 9);
    if (txJson.type === 2) {
        gasPrice = toHex(txJson.maxFeePerGas);
    } else if (txJson.type === 0) {
        gasPrice = toHex(txJson.gasPrice);
    }
    return {
        hash: tx_.hash,
        nonce: txJson.nonce,
        from: from,
        type: toHex(txJson.type || 0),
        messageHash: tx_.unsignedHash,
        gasPrice: gasPrice,
        limit: toHex(txJson.gasLimit),
        to: txJson.to?.toLowerCase() || ZeroAddress,
        value: toHex(txJson.value),
        data: txJson.data || '0x',
        v: +tx2.v?.toString() ?? 27,
        r: (tx2.r && toHex(tx2.r)) || '0x',
        s: (tx2.s && toHex(tx2.s)) || '0x',
        chainId: +txJson.chainId,
    };
}

export function toHex(number) {
    let ret = BigNumber(number).toString(16);
    return '0x' + ret;
}

export function toNumber(number) {
    return BigNumber(number).toNumber();
}

export function toNumberStr(number) {
    return BigNumber(number).decimalPlaces(0).toFixed();
}

export function toU256Hex(a, includePrefix = true) {
    let it = toHex(a).slice(2).padStart(64, '0');
    if (includePrefix) return '0x' + it;
    return it;
}

export function sleep(s) {
    return new Promise(r => {
        setTimeout(r, s * 1000);
    });
}

export function randomHex(bytes = 32) {
    return '0x' + Buffer.from(randomBytes(bytes)).toString('hex');
}

let x =
    '0x02f8d58201500486015d3ef7980086015d3ef79800825208946a9a394cb23b2c5b2e4290f75f80a8e049f3347e80b864c47f00270000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000668656c6c6f320000000000000000000000000000000000000000000000000000c001a038787b861c38d1ff1efaa187cba5f4939228d103e732eae0173d7078389e0af9a079d70b4d9453f35688d3930bbfd87827a274eec1a7ffd8034898d5a600c14811';
parseRawTx(x);
