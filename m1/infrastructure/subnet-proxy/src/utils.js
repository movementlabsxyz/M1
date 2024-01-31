const { HexString, TxnBuilderTypes } = require('aptos');

function getAddress(pubKey) {
    pubKey = pubKey.replace('0x', '');
    let key = HexString.ensure(pubKey).toUint8Array();

    pubKey = new TxnBuilderTypes.Ed25519PublicKey(key);

    const authKey = TxnBuilderTypes.AuthenticationKey.fromEd25519PublicKey(pubKey);
    let keys = authKey.derivedAddress();
    return keys.hexString.slice(2);
}

function sleep(s) {
    return new Promise(r => setTimeout(r, s * 1000));
}

module.exports = {
    sleep,
    getAddress,
};
