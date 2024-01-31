// User.js

import { DataTypes, Sequelize } from 'sequelize';

const sequelize = new Sequelize('database', null, null, {
    dialect: 'sqlite',
    storage: './db/database.db',
    logging: false,
});

export const RawTx = sequelize.define('RawTx', {
    id: {
        type: DataTypes.INTEGER,
        primaryKey: true,
        autoIncrement: true,
    },
    tx: {
        type: DataTypes.TEXT,
        allowNull: false,
    },
    hash: {
        type: DataTypes.STRING,
        allowNull: false,
    },
    info: {
        type: DataTypes.JSON,
        allowNull: false,
    },
});
await RawTx.sync();
export function saveTx(tx, hash, info) {
    RawTx.create({
        tx,
        hash,
        info,
    }).catch(err => {
        // ignore
    });
}

const MoveEvmTxHash = sequelize.define(
    'MoveEvmTxHash',
    {
        id: {
            type: DataTypes.INTEGER.UNSIGNED,
            primaryKey: true,
            autoIncrement: true,
        },
        move_hash: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        evm_hash: {
            type: DataTypes.STRING,
            allowNull: false,
        },
    },
    {
        indexes: [
            {
                fields: ['move_hash'],
            },
            {
                fields: ['evm_hash'],
            },
        ],
    },
);
await MoveEvmTxHash.sync();

export async function saveMoveEvmTxHash(move_hash, evm_hash) {
    return MoveEvmTxHash.create({
        move_hash,
        evm_hash,
    }).catch(err => {
        // ignore
    });
}

export async function getMoveHash(evm_hash) {
    let move_hash = await MoveEvmTxHash.findOne({
        where: {
            evm_hash,
        },
        order: [['id', 'desc']],
    }).then(res => res?.move_hash ?? null);
    return move_hash || evm_hash;
}

export const TxEvents = sequelize.define(
    'TxEvents',
    {
        id: {
            type: DataTypes.INTEGER.UNSIGNED,
            primaryKey: true,
            autoIncrement: true,
        },
        logIndex: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        blockNumber: {
            type: DataTypes.INTEGER.UNSIGNED,
            allowNull: false,
        },
        blockHash: {
            type: DataTypes.STRING(66),
            allowNull: false,
        },
        transactionHash: {
            type: DataTypes.STRING(66),
            allowNull: false,
        },
        transactionIndex: {
            type: DataTypes.STRING,
            allowNull: false,
        },
        address: {
            type: DataTypes.STRING(42),
            allowNull: false,
        },
        data: {
            type: DataTypes.TEXT('long'),
            allowNull: false,
        },
        topic0: {
            type: DataTypes.STRING(66),
            allowNull: false,
        },
        topic1: {
            type: DataTypes.STRING(66),
            allowNull: false,
        },
        topic2: {
            type: DataTypes.STRING(66),
            allowNull: false,
        },
        topic3: {
            type: DataTypes.STRING(66),
            allowNull: false,
        },
        topics: {
            type: DataTypes.TEXT('long'),
            allowNull: false,
            comment: 'topic json array',
        },
    },
    {
        indexes: [
            {
                fields: ['blockNumber', 'address', 'topic0'],
                fields: ['blockNumber', 'address', 'topic0', 'topic1'],
                fields: ['blockNumber', 'address', 'topic0', 'topic1', 'topic2'],
                fields: ['blockNumber', 'address', 'topic0', 'topic1', 'topic2', 'topic3'],
            },
        ],
    },
);
await TxEvents.sync();

export const GLobalState = sequelize.define('GLobalState', {
    id: {
        type: DataTypes.INTEGER.UNSIGNED,
        primaryKey: true,
        autoIncrement: true,
    },
    key: {
        type: DataTypes.STRING,
        allowNull: false,
        unique: true,
    },
    value: {
        type: DataTypes.TEXT,
        allowNull: false,
    },
});
await GLobalState.sync();
