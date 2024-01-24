import { getTransactionReceipt } from './bridge.js';
import { GLobalState, RawTx, TxEvents } from './db.js';
import { sleep } from './helper.js';
import { Op } from 'sequelize';
async function saveEvents(tx) {
    const receipt = await getTransactionReceipt(tx);
    let logs = receipt.logs;
    if (logs.length > 0) {
        logs = logs.map(log => {
            const {
                address,
                topics,
                data,
                blockNumber,
                transactionHash,
                transactionIndex,
                blockHash,
                logIndex,
            } = log;
            return {
                logIndex,
                blockNumber: parseInt(blockNumber.slice(2), 16),
                blockHash,
                transactionHash,
                transactionIndex,
                address,
                data: data || '0x',
                topics: JSON.stringify(topics),
                topic0: topics[0] || '',
                topic1: topics[1] || '',
                topic2: topics[2] || '',
                topic3: topics[3] || '',
            };
        });
        await TxEvents.bulkCreate(logs);
    }
}
let latest_sync_event_tx_id = -1;
async function syncTxEvents() {
    const KEY = 'latestSyncEventTx';
    if (latest_sync_event_tx_id === -1) {
        const latestTx = await GLobalState.findOne({
            where: {
                key: KEY,
            },
        });
        if (!latestTx) {
            await GLobalState.create({
                key: KEY,
                value: '0',
            });
            return;
        }
        latest_sync_event_tx_id = parseInt(latestTx.value);
    }
    const nextTx = await RawTx.findOne({
        attributes: ['id', 'hash'],
        where: {
            id: {
                [Op.gt]: latest_sync_event_tx_id,
            },
        },
    });
    if (nextTx) {
        await saveEvents(nextTx.hash);
        await GLobalState.update(
            {
                value: nextTx.id,
            },
            {
                where: {
                    key: KEY,
                },
            },
        );
        latest_sync_event_tx_id = parseInt(nextTx.id);
    } else {
        await sleep(1);
    }
}
async function startSyncEventsTask() {
    while (true) {
        try {
            await syncTxEvents();
        } catch (e) {
            console.log(e);
        }
        await sleep(0.1);
    }
}

startSyncEventsTask();
